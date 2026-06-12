//! KLC 原生代码生成器 — 阶段五
//!
//! 支持: let, 算术, 比较, if/else, while, for, break, continue,
//!       println, exit, MessageBoxA (Win32 auto-import)

use crate::ast::{BinOp, Expr, Program, Stmt};
use crate::native::x64::{Register, X64Assembler};
use crate::native::imports::ImportTableBuilder;
use crate::native::pe::{PeBuilder, CODE_RVA, IDATA_RVA};
use crate::native::optimize;
use std::collections::{HashMap, HashSet};

pub struct NativeCodeGenerator {
    asm: X64Assembler,
    imports: ImportTableBuilder,
    vars: HashMap<String, i32>,
    stack_size: i32, next_slot: i32,
    ext_imports: HashMap<String, u32>,      // fn_name → IAT RVA
    string_map: HashMap<String, (u32, usize)>,
    getstd_rva: Option<u32>, write_rva: Option<u32>, exit_rva: Option<u32>,
    stdout_in_r12: bool,
    label_cnt: u32,
    break_lbl: Vec<u32>,  // 循环 break 标签栈
    cont_lbl: Vec<u32>,   // 循环 continue 标签栈
    /// 未使用的变量集合 — 跳过栈分配
    unused_vars: HashSet<String>,
    /// 已知常量值（常量传播） — 在代码生成时内联
    const_values: HashMap<String, i64>,
    /// 变量 → 被调用者保存寄存器映射 (寄存器提升)
    /// 循环变量等高频访问变量直接放在 callee-saved 寄存器中，避免内存访问
    var_regs: HashMap<String, Register>,
    /// 已使用的被调用者保存寄存器列表（需要在序言中 push、尾声中 pop）
    used_callee_saved: Vec<Register>,
    /// 可用的被调用者保存寄存器池
    /// R12 已被 stdout handle 占用, RBP 是栈帧指针
    /// 可用: RBX, RSI, RDI, R13, R14, R15
    avail_callee_saved: Vec<Register>,
    /// 对齐后的栈帧总大小（含局部变量 + 对齐填充，不含 push regs）
    aligned_stack_size: i32,
    /// 用户自定义函数 → 标签映射（用于 call/jmp 目标）
    fn_labels: HashMap<String, u32>,
    /// 函数结束标签（用于 return 跳转）
    fn_end_label: Option<u32>,
    /// 禁用 AST 级优化（用于基准测试对比）
    pub no_opt: bool,
}

impl NativeCodeGenerator {
    pub fn new() -> Self { NativeCodeGenerator {
        asm: X64Assembler::new(), imports: ImportTableBuilder::new(),
        vars: HashMap::new(), stack_size: 0, next_slot: -8,
        ext_imports: HashMap::new(), string_map: HashMap::new(),
        getstd_rva: None, write_rva: None, exit_rva: None,
        stdout_in_r12: false, label_cnt: 0,
        break_lbl: Vec::new(), cont_lbl: Vec::new(),
        unused_vars: HashSet::new(), const_values: HashMap::new(),
        var_regs: HashMap::new(), used_callee_saved: Vec::new(),
        // 可用的 callee-saved 寄存器 (排除 RBP=栈帧, R12=stdout)
        avail_callee_saved: vec![Register::RBX, Register::RSI, Register::RDI,
                                  Register::R13, Register::R14, Register::R15],
        aligned_stack_size: 0,
        fn_labels: HashMap::new(),
        fn_end_label: None,
        no_opt: false,
    }}

    fn ensure_imports(&mut self) {
        self.imports.set_base_rva(IDATA_RVA);
        if self.exit_rva.is_none() {
            self.exit_rva = Some(self.imports.add_import("kernel32.dll", "ExitProcess"));
            self.getstd_rva = Some(self.imports.add_import("kernel32.dll", "GetStdHandle"));
            self.write_rva = Some(self.imports.add_import("kernel32.dll", "WriteConsoleA"));
        }
    }
    fn ensure_stdout(&mut self) {
        if !self.stdout_in_r12 {
            self.ensure_imports();
            let shadow: i32 = if self.aligned_stack_size % 16 == 0 { 0x20 } else { 0x28 };
            self.asm.sub_rsp_imm(shadow); self.asm.mov_reg_imm(Register::RCX, -11);
            let cur = CODE_RVA + self.asm.byte_position() as u32;
            self.asm.call_iat_rva(self.getstd_rva.unwrap(), cur);
            self.asm.add_rsp_imm(shadow); self.asm.mov_reg_reg(Register::R12, Register::RAX);
            self.stdout_in_r12 = true;
        }
    }
    fn nlbl(&mut self) -> u32 { let l = self.label_cnt; self.label_cnt += 1; l }

    fn place_string(&mut self, s: &str) -> u32 {
        let rva = CODE_RVA + self.asm.byte_position() as u32;
        for &b in format!("{}\0", s).as_bytes() { self.asm.emit_byte(b); }
        rva
    }
    fn alloc_var(&mut self, name: &str) -> i32 {
        if let Some(&off) = self.vars.get(name) { return off; }
        // 跳过未使用的变量 — 不分配栈空间
        if self.unused_vars.contains(name) {
            self.vars.insert(name.to_string(), 0xDEAD);
            return 0xDEAD;
        }
        let off = self.next_slot; self.next_slot -= 8;
        self.stack_size = self.stack_size.max(-off);
        self.vars.insert(name.to_string(), off); off
    }

    /// 尝试将变量提升到 callee-saved 寄存器（仅用于循环变量等高频变量）
    fn promote_to_reg(&mut self, name: &str) {
        if self.var_regs.contains_key(name) { return; }
        if self.unused_vars.contains(name) { return; }
        if let Some(reg) = self.avail_callee_saved.pop() {
            self.var_regs.insert(name.to_string(), reg);
            self.used_callee_saved.push(reg);
        }
    }

    /// 检查变量是否有被提升到寄存器
    fn get_var_reg(&self, name: &str) -> Option<Register> {
        self.var_regs.get(name).copied()
    }

    fn var_off(&self, n: &str) -> i32 {
        let off = *self.vars.get(n).unwrap();
        if off == 0xDEAD { panic!("bug: using unused var {}", n); }
        off
    }

    /// 检查变量是否是已知常量
    fn is_var_const(&self, name: &str) -> Option<i64> {
        self.const_values.get(name).copied()
    }

    // ---- extern 导入 ----
    fn import_extern(&mut self, fn_name: &str, dll: &str) -> u32 {
        if let Some(&rva) = self.ext_imports.get(fn_name) { return rva; }
        self.imports.set_base_rva(IDATA_RVA);
        let rva = self.imports.add_import(dll, fn_name);
        self.ext_imports.insert(fn_name.to_string(), rva);
        rva
    }

    // 已知 Win32 API 的 DLL
    fn known_dll(fn_name: &str) -> Option<&'static str> {
        match fn_name {
            "MessageBoxA" | "CreateWindowExA" | "ShowWindow" | "GetMessageA" | "DispatchMessageA" | "DefWindowProcA" | "RegisterClassA" | "PostQuitMessage" | "UpdateWindow" | "GetDC" | "ReleaseDC" | "BeginPaint" | "EndPaint" | "LoadCursorA" | "LoadIconA" => Some("user32.dll"),
            "Beep" | "Sleep" | "GetTickCount" | "GetLastError" | "SetLastError" | "GetCommandLineA" | "GetModuleHandleA" | "CreateFileA" | "WriteFile" | "ReadFile" | "CloseHandle" | "GetStdHandle" | "WriteConsoleA" | "ExitProcess" | "GetProcessHeap" | "HeapAlloc" | "HeapFree" | "VirtualAlloc" | "VirtualFree" | "LoadLibraryA" | "GetProcAddress" | "FreeLibrary" => Some("kernel32.dll"),
            _ => None,
        }
    }

    fn is_extern(&self, fn_name: &str) -> bool {
        self.ext_imports.contains_key(fn_name) || Self::known_dll(fn_name).is_some()
    }

    /// 递归扫描 AST，收集所有 extern 调用所需的 (DLL, 函数名) 对
    /// 必须在 add_import / ensure_imports 之前调用，以稳定 IAT 布局
    fn scan_extern_imports(stmts: &[Stmt]) -> Vec<(&'static str, String)> {
        fn walk(e: &Expr, out: &mut Vec<(&'static str, String)>) {
            if let Expr::Call(name, args) = e {
                if let Some(d) = NativeCodeGenerator::known_dll(name) {
                    if !out.iter().any(|(dn, fn_)| *dn == d && fn_ == name) {
                        out.push((d, name.clone()));
                    }
                }
                for a in args { walk(a, out); }
            } else if let Expr::Binary(l, _, r) = e {
                walk(l, out); walk(r, out);
            }
        }
        let mut imports = Vec::new();
        for s in stmts {
            match s {
                Stmt::Expr(e) => walk(e, &mut imports),
                Stmt::Let { value, .. } => walk(value, &mut imports),
                Stmt::If { cond, then_block, else_block } => {
                    walk(cond, &mut imports);
                    imports.extend(Self::scan_extern_imports(then_block));
                    if let Some(b) = else_block { imports.extend(Self::scan_extern_imports(b)); }
                }
                Stmt::While(_, body) | Stmt::For { body, .. } => {
                    imports.extend(Self::scan_extern_imports(body));
                }
                Stmt::Block(stmts) => imports.extend(Self::scan_extern_imports(stmts)),
                _ => {}
            }
        }
        imports
    }

    /// 预注册一组 (DLL, 函数名) 导入（不生成代码）
    /// 按 DLL 分组，逐个 DLL 添加所有函数，确保 IAT 布局稳定
    fn pre_register_all_imports(&mut self, stmts: &[Stmt]) {
        self.imports.set_base_rva(IDATA_RVA);

        // 收集用户代码中的所有 extern 调用
        let user_imports = Self::scan_extern_imports(stmts);

        // 按 DLL 分组: (dll_name → [fn_names])
        let mut groups: std::collections::HashMap<&str, Vec<String>> = std::collections::HashMap::new();
        groups.insert("kernel32.dll", Vec::new());  // kernel32 始终在第一个
        for (dll, fn_name) in &user_imports {
            groups.entry(dll).or_default().push(fn_name.clone());
        }

        // 内置的 kernel32 函数（ensure_imports 会用到）
        groups.get_mut("kernel32.dll").unwrap()
            .extend(["ExitProcess", "GetStdHandle", "WriteConsoleA"].map(String::from));

        // 第一步：注册所有 DLL（空），稳定 IDT 大小
        let mut dll_order: Vec<&str> = groups.keys().map(|k| *k).collect();
        dll_order.sort_by(|a, b| {
            // kernel32 始终排第一
            if *a == "kernel32.dll" { return std::cmp::Ordering::Less; }
            if *b == "kernel32.dll" { return std::cmp::Ordering::Greater; }
            std::cmp::Ordering::Equal
        });
        for dll in &dll_order {
            self.imports.register_dll(dll);
        }

        // 第二步：逐 DLL 添加所有函数（每个 DLL 的 IAT 在处理下一个 DLL 前已固定）
        for dll in &dll_order {
            for fn_name in &groups[dll] {
                self.imports.add_import(dll, fn_name);
            }
        }

        // 预填充 ext_imports 缓存，让后续 import_extern 直接命中
        for (dll, fns) in &groups {
            for fn_name in fns {
                let rva = self.imports.add_import(dll, fn_name);  // 重复添加会返回已有值
                self.ext_imports.insert(fn_name.clone(), rva);
            }
        }
        // 设置内置函数的 RVA（供 println/exit 使用）
        if let Some(&r) = self.ext_imports.get("ExitProcess") { self.exit_rva = Some(r); }
        if let Some(&r) = self.ext_imports.get("GetStdHandle") { self.getstd_rva = Some(r); }
        if let Some(&r) = self.ext_imports.get("WriteConsoleA") { self.write_rva = Some(r); }
    }

    fn call_extern(&mut self, fn_name: &str, args: &[Expr]) {
        let dll = Self::known_dll(fn_name).unwrap_or("kernel32.dll");
        let iat_rva = self.import_extern(fn_name, dll);
        let reg_count = args.len().min(4);
        let stack_param_count = if args.len() > 4 { args.len() - 4 } else { 0 };
        // 栈参数 (5th+)
        for i in (reg_count..args.len()).rev() {
            self.expr_arg(&args[i]);
            self.asm.push_reg(Register::RAX);
        }
        // 寄存器参数: 字符串用 RIP-relative LEA, 整数用 mov_reg_imm
        for i in 0..reg_count {
            let dest = [Register::RCX, Register::RDX, Register::R8, Register::R9][i];
            // 字符串参数: 用 RIP-relative LEA (不受 ASLR 影响!)
            if let Expr::String(s) = &args[i] {
                if let Some(&(rva, _)) = self.string_map.get(s) {
                    let cur = CODE_RVA + self.asm.byte_position() as u32;
                    self.asm.lea_rip_rva(dest, rva, cur);
                    continue;
                }
            }
            self.expr_arg(&args[i]);
            self.asm.mov_reg_reg(dest, Register::RAX);
        }
        // 计算 shadow space：Windows x64 要求 CALL 指令前 RSP 必须 16 字节对齐
        // 进入函数时 RSP ≡ 8 mod 16 (CALL 压了返回地址)
        // push rbp 后 RSP ≡ 0 mod 16
        // sub rsp, stack_size 后 RSP ≡ -stack_size mod 16
        // push N 个栈参数后 RSP ≡ -(stack_size + N*8) mod 16
        // sub rsp, shadow 后需要 RSP ≡ 0 mod 16
        // → shadow ≡ -(aligned_stack_size + N*8) mod 16, 最小 0x20 (32)
        let total_push = self.aligned_stack_size + (stack_param_count as i32) * 8;
        let shadow: i32 = if total_push % 16 == 0 { 0x20 } else { 0x28 };
        self.asm.sub_rsp_imm(shadow);
        let cur = CODE_RVA + self.asm.byte_position() as u32;
        self.asm.call_iat_rva(iat_rva, cur);
        self.asm.add_rsp_imm(shadow);
    }

    // 为 extern 调用编译参数 (含字符串 — 传实际地址 ImageBase+RVA)
    fn expr_arg(&mut self, e: &Expr) {
        match e {
            Expr::String(s) => {
                if let Some(&(rva, _)) = self.string_map.get(s) {
                    let cur = CODE_RVA + self.asm.byte_position() as u32;
                    self.asm.lea_rip_rva(Register::RAX, rva, cur);
                }
            }
            Expr::Integer(n) => self.asm.mov_reg_imm(Register::RAX, *n),
            _ => self.expr(e),
        }
    }

    fn expr(&mut self, e: &Expr) {
        match e {
            Expr::Integer(n) => self.asm.mov_reg_imm(Register::RAX, *n),
            Expr::Bool(b) => self.asm.mov_reg_imm(Register::RAX, if *b {1} else {0}),
            Expr::String(_) => self.asm.mov_reg_imm(Register::RAX, 0),
            Expr::Ident(name) => {
                // 常量传播: 变量有已知常量值 → 直接加载常量
                if let Some(v) = self.is_var_const(name) {
                    self.asm.mov_reg_imm(Register::RAX, v);
                    return;
                }
                // 寄存器提升: 变量在 callee-saved 寄存器中
                if let Some(reg) = self.get_var_reg(name) {
                    self.asm.mov_reg_reg(Register::RAX, reg);
                    return;
                }
                let off = self.var_off(name);
                self.asm.mov_reg_mem(Register::RAX, Register::RBP, off);
            }
            Expr::Binary(l, op, r) => self.binary(l, op, r),
            Expr::Call(name, args) => {
                // 在表达式上下文中调用函数，返回值在 RAX
                if self.is_extern(name) { self.call_extern(name, args); return; }
                if let Some(&_label) = self.fn_labels.get(name) {
                    self.emit_user_call(name, args);
                    return;
                }
                // 内置函数 (println 等) 在表达式上下文中无有意义的返回值
            }
            Expr::TailCall(name, args) => {
                // 尾调用: 计算参数 → 释放栈帧 → jmp 目标函数
                self.emit_tail_call(name, args);
            }
            _ => {}
        }
    }

    /// 优化的二元运算 — 利用 reg+imm 指令，减少 push/pop
    fn binary(&mut self, l: &Expr, op: &BinOp, r: &Expr) {
        let cmp = matches!(op, BinOp::Eq|BinOp::Neq|BinOp::Lt|BinOp::Gt|BinOp::Lte|BinOp::Gte);

        // 尝试编译为 reg+imm 形式 (更高效，避免 push/pop)
        if self.try_binary_imm(l, op, r, cmp) {
            return;
        }

        // 回退到通用 push/pop 模式
        if cmp {
            self.expr(r); self.asm.push_reg(Register::RAX);
            self.expr(l); self.asm.pop_reg(Register::RCX);
            self.asm.cmp_reg_reg(Register::RAX, Register::RCX);
            match op { BinOp::Eq => self.asm.sete_rax(), BinOp::Neq => self.asm.setne_rax(), BinOp::Lt => self.asm.setl_rax(), BinOp::Gt => self.asm.setg_rax(), BinOp::Lte => self.asm.setle_rax(), BinOp::Gte => self.asm.setge_rax(), _ => {} }
        } else {
            self.expr(r); self.asm.push_reg(Register::RAX);
            self.expr(l); self.asm.pop_reg(Register::RCX);
            match op { BinOp::Add => self.asm.add_reg_reg(Register::RAX, Register::RCX), BinOp::Sub => self.asm.sub_reg_reg(Register::RAX, Register::RCX), BinOp::Mul => self.asm.imul_rax_rcx(), BinOp::Div => { self.asm.cqo(); self.asm.idiv_rcx(); }, BinOp::Mod => { self.asm.cqo(); self.asm.idiv_rcx(); self.asm.mov_reg_reg(Register::RAX, Register::RDX); }, _ => {} }
        }
    }

    /// 尝试将二元运算编译为 reg+imm 形式
    /// 返回 true 表示已生成代码，false 表示需要回退到通用模式
    fn try_binary_imm(&mut self, l: &Expr, op: &BinOp, r: &Expr, cmp: bool) -> bool {
        // 模式 1: 右操作数是常量 → reg + imm 指令
        // 例: x + 5 → load x into RAX; add rax, 5
        if let Some(rv) = self.eval_imm(r) {
            if rv >= i32::MIN as i64 && rv <= i32::MAX as i64 {
                self.expr(l);
                let imm = rv as i32;
                if cmp {
                    self.asm.cmp_reg_imm(Register::RAX, imm);
                } else {
                    match op {
                        BinOp::Add => self.asm.add_reg_imm(Register::RAX, imm),
                        BinOp::Sub => self.asm.sub_reg_imm(Register::RAX, imm),
                        BinOp::Mul => {
                            // 乘以 2 的幂次用移位更快
                            if imm > 0 && imm.count_ones() == 1 {
                                self.asm.shl_reg_imm(Register::RAX, imm.trailing_zeros() as u8);
                            } else {
                                self.asm.imul_reg_imm(Register::RAX, imm);
                            }
                        }
                        BinOp::And => self.asm.and_reg_imm(Register::RAX, imm),
                        BinOp::Or => self.asm.or_reg_imm(Register::RAX, imm),
                        _ => return false, // 其他操作回退
                    }
                }
                // 比较运算需要 setCC
                if cmp {
                    match op {
                        BinOp::Eq => self.asm.sete_rax(),
                        BinOp::Neq => self.asm.setne_rax(),
                        BinOp::Lt => self.asm.setl_rax(),
                        BinOp::Gt => self.asm.setg_rax(),
                        BinOp::Lte => self.asm.setle_rax(),
                        BinOp::Gte => self.asm.setge_rax(),
                        _ => {}
                    }
                }
                return true;
            }
        }

        // 模式 2: 左操作数是常量 → 调整指令顺序
        // 例: 5 + x → load x into RAX; add rax, 5
        // 例: 5 - x → load x into RAX; mov rcx, 5; sub rcx, rax; mov rax, rcx (或 neg + add)
        if let Some(lv) = self.eval_imm(l) {
            if lv >= i32::MIN as i64 && lv <= i32::MAX as i64 {
                let imm = lv as i32;
                match op {
                    BinOp::Add => {
                        self.expr(r);
                        self.asm.add_reg_imm(Register::RAX, imm);
                        return true;
                    }
                    BinOp::Mul => {
                        self.expr(r);
                        if imm > 0 && imm.count_ones() == 1 {
                            self.asm.shl_reg_imm(Register::RAX, imm.trailing_zeros() as u8);
                        } else {
                            self.asm.imul_reg_imm(Register::RAX, imm);
                        }
                        return true;
                    }
                    BinOp::Sub => {
                        // c - x: load x → neg → add c
                        self.expr(r);
                        self.asm.neg_reg(Register::RAX);
                        self.asm.add_reg_imm(Register::RAX, imm);
                        return true;
                    }
                    _ => {}
                }
            }
        }

        // 模式 3: 两个变量比较 → 直接内存比较
        // 例: x < y → cmp [rbp+x], [rbp+y] (需要先加载一个到寄存器)
        if cmp {
            if let (Expr::Ident(ln), Expr::Ident(rn)) = (l, r) {
                if self.is_var_const(ln).is_none() && self.is_var_const(rn).is_none() {
                    // 两个非常量变量比较
                    if ln != rn {
                        self.expr(r); // RAX = y
                        self.asm.push_reg(Register::RAX);
                        self.expr(l); // RAX = x
                        self.asm.pop_reg(Register::RCX); // RCX = y
                        self.asm.cmp_reg_reg(Register::RAX, Register::RCX);
                        match op {
                            BinOp::Eq => self.asm.sete_rax(),
                            BinOp::Neq => self.asm.setne_rax(),
                            BinOp::Lt => self.asm.setl_rax(),
                            BinOp::Gt => self.asm.setg_rax(),
                            BinOp::Lte => self.asm.setle_rax(),
                            BinOp::Gte => self.asm.setge_rax(),
                            _ => {}
                        }
                        return true;
                    }
                }
            }
        }

        false
    }

    /// 清除表达式中引用的所有变量的编译期常量标记
    /// 用于循环条件编译前：条件被重复求值，编译期常量值只对首次迭代有效，
    /// 必须清除以防止常量值被内联到循环头的机器码中
    fn invalidate_const_in_expr(&mut self, e: &Expr) {
        match e {
            Expr::Ident(name) => { self.const_values.remove(name); }
            Expr::Binary(l, _, r) => {
                self.invalidate_const_in_expr(l);
                self.invalidate_const_in_expr(r);
            }
            Expr::Unary(_, inner) => self.invalidate_const_in_expr(inner),
            Expr::If(cond, then_val, else_val) => {
                self.invalidate_const_in_expr(cond);
                self.invalidate_const_in_expr(then_val);
                if let Some(e) = else_val { self.invalidate_const_in_expr(e); }
            }
            Expr::Call(_, args) | Expr::TailCall(_, args) => {
                for a in args { self.invalidate_const_in_expr(a); }
            }
            _ => {}
        }
    }

    /// 评估表达式的立即数值（用于 reg+imm 优化）
    fn eval_imm(&self, e: &Expr) -> Option<i64> {
        match e {
            Expr::Integer(n) => Some(*n),
            Expr::Bool(b) => Some(if *b { 1 } else { 0 }),
            Expr::Char(c) => Some(*c as i64),
            Expr::Ident(name) => self.is_var_const(name),
            _ => optimize::try_eval_const(e),
        }
    }

    fn println_with_rva(&mut self, str_rva: u32, len: usize) {
        self.ensure_stdout();
        // 0x38 = 32(shadow) + 8(5th param) + 8(local var for lpNumberOfCharsWritten)
        // 需要对齐: stack_size%16==0 时用 0x40, 否则用 0x38
        let shadow: i32 = if self.aligned_stack_size % 16 == 0 { 0x40 } else { 0x38 };
        self.asm.sub_rsp_imm(shadow); self.asm.xor_reg_reg(Register::RCX, Register::RCX);
        self.asm.mov_mem_reg(Register::RSP, 0x28, Register::RCX);
        self.asm.mov_mem_reg(Register::RSP, 0x20, Register::RCX);
        self.asm.lea_rsp_disp8(Register::R9, 0x28);
        self.asm.mov_reg_imm(Register::R8, len as i64);
        let cur = CODE_RVA + self.asm.byte_position() as u32;
        self.asm.lea_rip_rva(Register::RDX, str_rva, cur);
        self.asm.mov_reg_reg(Register::RCX, Register::R12);
        let cur = CODE_RVA + self.asm.byte_position() as u32;
        self.asm.call_iat_rva(self.write_rva.unwrap(), cur);
        self.asm.add_rsp_imm(shadow);
    }

    fn stmt(&mut self, s: &Stmt) {
        match s {
            Stmt::Let { name, value, .. } => {
                let off = self.alloc_var(name);
                // 跟踪常量值
                if let Some(v) = self.eval_imm(value) {
                    self.const_values.insert(name.clone(), v);
                } else {
                    self.const_values.remove(name);
                }
                self.expr(value);
                // 跳过未使用变量的存储
                if off != 0xDEAD {
                    self.asm.mov_mem_reg(Register::RBP, off, Register::RAX);
                }
            }
            Stmt::Assign { name, value } => {
                // 跟踪常量值
                if let Some(v) = self.eval_imm(value) {
                    self.const_values.insert(name.clone(), v);
                } else {
                    self.const_values.remove(name);
                }
                let off = self.var_off(name);
                self.expr(value);
                // 写回栈内存
                self.asm.mov_mem_reg(Register::RBP, off, Register::RAX);
                // 如果变量被提升到寄存器，同步更新寄存器（关键！否则循环变量死循环）
                if let Some(reg) = self.get_var_reg(name) {
                    self.asm.mov_reg_reg(reg, Register::RAX);
                }
            }
            Stmt::Expr(e) => {
                // 检查 TailCall（尾调用: jmp 替代 call）
                if let Expr::TailCall(name, args) = e {
                    self.emit_tail_call(name, args);
                    return;
                }
                if let Expr::Call(name, args) = e {
                    match name.as_str() {
                        "println" | "print" => {
                            if let Some(Expr::String(s)) = args.first() {
                                if let Some(&(rva, len)) = self.string_map.get(s) { self.println_with_rva(rva, len); }
                            }
                            return;
                        }
                        "exit" => {
                            if let Some(a) = args.first() { self.expr(a); }
                            self.asm.mov_reg_reg(Register::RCX, Register::RAX);
                            self.asm.sub_rsp_imm(0x28);
                            let cur = CODE_RVA + self.asm.byte_position() as u32;
                            self.asm.call_iat_rva(self.exit_rva.unwrap(), cur);
                            return;
                        }
                        _ => {
                            if self.is_extern(name) { self.call_extern(name, args); return; }
                            // 用户自定义函数调用
                            if self.fn_labels.contains_key(name) {
                                self.emit_user_call(name, args);
                                return;
                            }
                        }
                    }
                }
                self.expr(e);
            }
            Stmt::PrintLn(e) | Stmt::Print(e) => {
                if let Expr::String(s) = e {
                    if let Some(&(rva, len)) = self.string_map.get(s) { self.println_with_rva(rva, len); }
                }
            }
            Stmt::Exit(_) => {
                // exit(code) — 调用 ExitProcess
                self.asm.mov_reg_imm(Register::RCX, 0); // exit code
                self.call_extern("ExitProcess", &[]);
            }
            Stmt::If { cond, then_block, else_block } => {
                let el = self.nlbl(); let en = self.nlbl();
                self.expr(cond); self.asm.cmp_reg_imm8(Register::RAX, 0); self.asm.je(el);
                for s in then_block { self.stmt(s); } self.asm.jmp(en);
                self.asm.bind_label(el);
                if let Some(b) = else_block { for s in b { self.stmt(s); } }
                self.asm.bind_label(en);
            }
            Stmt::While(cond, body) => {
                let cnt = self.nlbl(); let en = self.nlbl();
                self.break_lbl.push(en); self.cont_lbl.push(cnt);
                // 关键修复：循环条件在运行时被重复求值，编译期常量值只对首次迭代有效。
                // 必须在生成条件机器码之前清除条件中引用变量的常量标记，
                // 否则 mov rax, <initial_const> 会被固化到循环头，导致死循环。
                self.invalidate_const_in_expr(cond);
                self.asm.bind_label(cnt);
                self.expr(cond); self.asm.cmp_reg_imm8(Register::RAX, 0); self.asm.je(en);
                for s in body { self.stmt(s); }
                self.asm.jmp(cnt);
                self.asm.bind_label(en);
                self.break_lbl.pop(); self.cont_lbl.pop();
            }
            Stmt::For { var, iterable, body } => {
                if let Expr::Binary(_start, BinOp::Range, end_expr) = iterable {
                    let cnt = self.nlbl(); let en = self.nlbl();
                    self.break_lbl.push(en); self.cont_lbl.push(cnt);
                    // 尝试将循环变量提升到寄存器
                    self.promote_to_reg(var);
                    let loop_reg = self.get_var_reg(var);
                    let off = self.alloc_var(var);
                    // 清除循环变量的编译期常量标记（变量会在循环中被修改）
                    self.const_values.remove(var);
                    // 初始化: var = 0
                    self.asm.mov_reg_imm(Register::RAX, 0);
                    if let Some(reg) = loop_reg {
                        self.asm.mov_reg_reg(reg, Register::RAX);
                    } else if off != 0xDEAD {
                        self.asm.mov_mem_reg(Register::RBP, off, Register::RAX);
                    }
                    self.asm.bind_label(cnt);
                    // 加载循环变量到 RAX
                    if let Some(reg) = loop_reg {
                        self.asm.mov_reg_reg(Register::RAX, reg);
                    } else if off != 0xDEAD {
                        self.asm.mov_reg_mem(Register::RAX, Register::RBP, off);
                    } else {
                        self.asm.mov_reg_imm(Register::RAX, 0);
                    }
                    // 比较: var < end
                    if let Some(end_val) = self.eval_imm(end_expr.as_ref()) {
                        if end_val >= i32::MIN as i64 && end_val <= i32::MAX as i64 {
                            self.asm.cmp_reg_imm(Register::RAX, end_val as i32);
                        } else {
                            self.expr(end_expr.as_ref());
                            self.asm.mov_reg_reg(Register::RCX, Register::RAX);
                            if let Some(reg) = loop_reg {
                                self.asm.mov_reg_reg(Register::RAX, reg);
                            } else if off != 0xDEAD {
                                self.asm.mov_reg_mem(Register::RAX, Register::RBP, off);
                            }
                            self.asm.cmp_reg_reg(Register::RAX, Register::RCX);
                        }
                    } else {
                        self.expr(end_expr.as_ref());
                        self.asm.mov_reg_reg(Register::RCX, Register::RAX);
                        if let Some(reg) = loop_reg {
                            self.asm.mov_reg_reg(Register::RAX, reg);
                        } else if off != 0xDEAD {
                            self.asm.mov_reg_mem(Register::RAX, Register::RBP, off);
                        }
                        self.asm.cmp_reg_reg(Register::RAX, Register::RCX);
                    }
                    self.asm.jge(en);
                    // body
                    for s in body { self.stmt(s); }
                    // var += 1 — 寄存器直接操作
                    if let Some(reg) = loop_reg {
                        self.asm.add_reg_imm(reg, 1);
                    } else if off != 0xDEAD {
                        self.asm.mov_reg_mem(Register::RAX, Register::RBP, off);
                        self.asm.add_reg_imm(Register::RAX, 1);
                        self.asm.mov_mem_reg(Register::RBP, off, Register::RAX);
                    }
                    self.asm.jmp(cnt);
                    self.asm.bind_label(en);
                    self.break_lbl.pop(); self.cont_lbl.pop();
                }
            }
            Stmt::Block(stmts) => { for s in stmts { self.stmt(s); } }
            Stmt::Break if !self.break_lbl.is_empty() => {
                self.asm.jmp(*self.break_lbl.last().unwrap());
            }
            Stmt::Continue if !self.cont_lbl.is_empty() => {
                self.asm.jmp(*self.cont_lbl.last().unwrap());
            }
            Stmt::Return(e) => {
                // 检查尾调用
                if let Some(Expr::TailCall(name, args)) = e {
                    self.emit_tail_call(name, args);
                    return;
                }
                if let Some(v) = e { self.expr(v); }
                // 跳转到函数尾声（统一出口）
                if let Some(end_lbl) = self.fn_end_label {
                    self.asm.jmp(end_lbl);
                }
            }
            _ => {}
        }
    }

    // ---- 尾调用 / 函数调用 / 函数编译辅助 ----

    /// 发出函数尾声: 恢复 callee-saved 寄存器, 释放栈帧, pop rbp
    fn emit_epilogue(&mut self) {
        for &reg in self.used_callee_saved.iter().rev() {
            self.asm.pop_reg(reg);
        }
        if self.aligned_stack_size > 0 {
            self.asm.add_rsp_imm(self.aligned_stack_size);
        }
        self.asm.pop_reg(Register::RBP);
    }

    /// 发出尾调用: 计算参数到寄存器 → 释放当前栈帧 → jmp 目标函数
    /// 仅支持 ≤ 4 个参数的尾调用（Win64 调用约定的前 4 个寄存器参数）
    fn emit_tail_call(&mut self, name: &str, args: &[Expr]) {
        let reg_count = args.len().min(4);
        // 1. 从最后一个参数开始 push，这样第一个参数会在栈顶
        //    (Win64: arg0→RCX, arg1→RDX, arg2→R8, arg3→R9)
        for i in (0..reg_count).rev() {
            self.expr(&args[i]);
            self.asm.push_reg(Register::RAX);
        }
        // 2. 按正序 pop 到正确的参数寄存器
        let param_regs: [Register; 4] = [Register::RCX, Register::RDX, Register::R8, Register::R9];
        for i in 0..reg_count {
            let dest = param_regs[i];
            self.asm.pop_reg(dest);
        }
        // 3. 释放当前栈帧: pop callee-saved → add rsp → pop rbp
        for &reg in self.used_callee_saved.iter().rev() {
            self.asm.pop_reg(reg);
        }
        if self.aligned_stack_size > 0 {
            self.asm.add_rsp_imm(self.aligned_stack_size);
        }
        self.asm.pop_reg(Register::RBP);
        // 4. jmp 到目标函数（此时 RSP 指向原始返回地址，栈对齐正确）
        if let Some(&label) = self.fn_labels.get(name) {
            self.asm.jmp(label);
        }
    }

    /// 发出用户自定义函数的普通调用（call + shadow space）
    fn emit_user_call(&mut self, name: &str, args: &[Expr]) {
        let reg_count = args.len().min(4);
        let stack_param_count = if args.len() > 4 { args.len() - 4 } else { 0 };

        // 栈参数 (5th+)
        for i in (reg_count..args.len()).rev() {
            self.expr(&args[i]);
            self.asm.push_reg(Register::RAX);
        }
        // 寄存器参数
        for i in 0..reg_count {
            let dest = [Register::RCX, Register::RDX, Register::R8, Register::R9][i];
            self.expr(&args[i]);
            self.asm.mov_reg_reg(dest, Register::RAX);
        }
        // Shadow space: 保持 Win64 栈对齐
        let total_push = self.aligned_stack_size + (stack_param_count as i32) * 8;
        let shadow: i32 = if total_push % 16 == 0 { 0x20 } else { 0x28 };
        self.asm.sub_rsp_imm(shadow);
        // call 到函数标签
        if let Some(&label) = self.fn_labels.get(name) {
            self.asm.call(label);
        }
        self.asm.add_rsp_imm(shadow);
    }

    /// 编译用户自定义函数: 序言 → 参数存储 → 函数体 → 尾声
    fn emit_user_fn(&mut self, name: &str, params: &[crate::ast::Param], body: &[Stmt]) {
        let fn_label = *self.fn_labels.get(name).unwrap();
        let end_label = self.nlbl();
        let saved_end_label = self.fn_end_label;

        // 绑定函数入口标签
        self.asm.bind_label(fn_label);

        // 保存/重置编译状态（每个函数独立的栈帧）
        let saved_vars = std::mem::take(&mut self.vars);
        let saved_stack_size = self.stack_size;
        let saved_next_slot = self.next_slot;
        let saved_aligned = self.aligned_stack_size;
        let saved_callee = std::mem::take(&mut self.used_callee_saved);
        let saved_var_regs = std::mem::take(&mut self.var_regs);
        let saved_avail: Vec<Register> = self.avail_callee_saved.drain(..).collect();
        let saved_consts = std::mem::take(&mut self.const_values);
        let saved_unused = std::mem::take(&mut self.unused_vars);
        self.stack_size = 0;
        self.next_slot = -8;
        self.var_regs.clear();

        // 参数映射: Win64 前 4 个参数在 RCX, RDX, R8, R9
        let param_regs = [Register::RCX, Register::RDX, Register::R8, Register::R9];
        let mut param_stores: Vec<(String, Register)> = Vec::new();
        for (i, p) in params.iter().enumerate() {
            if i < 4 {
                param_stores.push((p.name.clone(), param_regs[i]));
            }
        }

        // 预分配参数的栈槽
        for p in params {
            self.alloc_var(&p.name);
        }

        // 序言: push rbp; mov rbp, rsp
        self.asm.push_reg(Register::RBP);
        self.asm.mov_reg_reg(Register::RBP, Register::RSP);

        // 存储参数到栈
        for (pname, reg) in &param_stores {
            if let Some(&off) = self.vars.get(pname) {
                if off != 0xDEAD {
                    self.asm.mov_mem_reg(Register::RBP, off, *reg);
                }
            }
        }

        // 保存 callee-saved 寄存器（暂不启用寄存器提升）
        // for &reg in &self.used_callee_saved { self.asm.push_reg(reg); }

        // 栈对齐: push rbp (8) + sub rsp, total → RSP ≡ 0 mod 16
        let aligned_stack = if self.stack_size % 16 != 0 {
            self.stack_size + 8
        } else {
            self.stack_size
        };
        self.aligned_stack_size = aligned_stack;
        if aligned_stack > 0 { self.asm.sub_rsp_imm(aligned_stack); }

        // 设置函数结束标签（return 跳转目标）
        self.fn_end_label = Some(end_label);

        // 函数体
        for s in body { self.stmt(s); }

        // 如果最后一条语句不是跳转/尾调用，补充返回值 0
        // (确保函数有明确的退出路径)

        // 尾声标签
        self.asm.bind_label(end_label);
        self.fn_end_label = None;

        // 尾声
        self.emit_epilogue();
        self.asm.ret();

        // 恢复编译状态
        self.vars = saved_vars;
        self.stack_size = saved_stack_size;
        self.next_slot = saved_next_slot;
        self.aligned_stack_size = saved_aligned;
        self.used_callee_saved = saved_callee;
        self.var_regs = saved_var_regs;
        self.avail_callee_saved = saved_avail;
        self.const_values = saved_consts;
        self.unused_vars = saved_unused;
        self.fn_end_label = saved_end_label;
    }

    pub fn compile(mut self, program: &Program, output_path: &str) -> std::io::Result<()> {
        // Phase 0: AST 级优化（可通过 no_opt 标志跳过）
        let optimized_stmts = if self.no_opt {
            program.statements.clone()
        } else {
            optimize::optimize_program(program.statements.clone())
        };
        let used_vars = if self.no_opt {
            // 无优化时：所有变量都视为已使用
            let mut all = HashSet::new();
            fn collect_var_names(stmts: &[Stmt], vars: &mut HashSet<String>) {
                for s in stmts { match s {
                    Stmt::Let { name, .. } | Stmt::For { var: name, .. }
                    | Stmt::Assign { name, .. } => { vars.insert(name.clone()); }
                    Stmt::FnDef { params, body, .. } => {
                        for p in params { vars.insert(p.name.clone()); }
                        collect_var_names(body, vars);
                    }
                    Stmt::If { then_block, else_block, .. } => {
                        collect_var_names(then_block, vars);
                        if let Some(b) = else_block { collect_var_names(b, vars); }
                    }
                    Stmt::While(_, body) => collect_var_names(body, vars),
                    Stmt::Block(stmts) => collect_var_names(stmts, vars),
                    Stmt::Expr(e) | Stmt::Return(Some(e)) => {
                        fn expr_vars(e: &Expr, vars: &mut HashSet<String>) {
                            match e {
                                Expr::Ident(n) => { vars.insert(n.clone()); }
                                Expr::Binary(l, _, r) => { expr_vars(l, vars); expr_vars(r, vars); }
                                Expr::If(cond, _, _) => { expr_vars(cond, vars); }
                                Expr::Call(_, args) | Expr::TailCall(_, args) => {
                                    for a in args { expr_vars(a, vars); }
                                }
                                _ => {}
                            }
                        }
                        expr_vars(e, vars);
                    }
                    _ => {}
                }}
            }
            collect_var_names(&optimized_stmts, &mut all);
            all
        } else {
            optimize::get_used_variables(&optimized_stmts)
        };

        // 收集未使用变量
        for s in &optimized_stmts {
            if let Stmt::Let { name, .. } = s {
                // For 循环变量由循环基础设施管理，始终视为"已使用"
                if !used_vars.contains(name) {
                    self.unused_vars.insert(name.clone());
                }
            }
            // Stmt::For 的 var 由循环基础设施使用，不标记为 unused
        }

        // Phase 1: 预注册所有 DLL 和所有导入函数（逐 DLL 分批添加）
        // 关键: 每个 DLL 的函数必须一次性全部添加完，再处理下一个 DLL
        // 否则跨 DLL 添加新函数会导致后续 DLL 的 IAT 偏移失效
        self.pre_register_all_imports(&optimized_stmts);

        // Phase 1.5: 收集用户自定义函数名，分配标签
        for s in &optimized_stmts {
            if let Stmt::FnDef { name, .. } = s {
                let label = self.nlbl();
                self.fn_labels.insert(name.clone(), label);
            }
        }

        // 收集字符串 (包括 extern 调用中的字符串参数)
        fn collect_strings(e: &Expr, map: &mut HashMap<String, (u32, usize)>) {
            match e { Expr::String(s) => { map.entry(s.clone()).or_insert((0, s.len())); } Expr::Binary(l, _, r) => { collect_strings(l, map); collect_strings(r, map); } _ => {} }
        }
        fn collect_stmts(stmts: &[Stmt], map: &mut HashMap<String, (u32, usize)>) {
            for s in stmts { match s {
                Stmt::PrintLn(e) | Stmt::Print(e) | Stmt::Exit(e) => collect_strings(e, map),
                Stmt::If { cond, then_block, else_block } => { collect_strings(cond, map);
                    for s in then_block { collect_stmts(&[s.clone()], map); }
                    if let Some(b) = else_block { for s in b { collect_stmts(&[s.clone()], map); } }
                }
                Stmt::While(_, body) | Stmt::For { body, .. } => { for s in body { collect_stmts(&[s.clone()], map); } }
                Stmt::Let { value, .. } => collect_strings(value, map),
                Stmt::Expr(e) => {
                    if let Expr::Call(_, args) = e { for a in args { collect_strings(a, map); } }
                    collect_strings(e, map);
                }
                _ => {}
            }}
        }
        for s in &optimized_stmts {
            // 从所有语句收集字符串（包括 FnDef 内部）
            fn collect_all_stmts(s: &Stmt, map: &mut HashMap<String, (u32, usize)>) {
                match s {
                    Stmt::FnDef { body, .. } => {
                        for bs in body { collect_all_stmts(bs, map); }
                    }
                    Stmt::ImplBlock { methods, .. } => {
                        for m in methods { collect_all_stmts(m, map); }
                    }
                    _ => collect_stmts(&[s.clone()], map),
                }
            }
            collect_all_stmts(s, &mut self.string_map);
        }

        // 预分配变量（在 promote_to_reg 之前）
        for s in &optimized_stmts {
            match s {
                Stmt::Let { name, .. } | Stmt::For { var: name, .. } => { self.alloc_var(name); }
                _ => {}
            }
        }

        // 字符串放在开头
        let code_start = self.nlbl();
        self.asm.jmp(code_start);
        let strs: Vec<(String, usize)> = self.string_map.iter().map(|(s, (_, l))| (s.clone(), *l)).collect();
        let mut new_map = HashMap::new();
        for (s, len) in &strs {
            let rva = self.place_string(s);
            new_map.insert(s.clone(), (rva, *len));
        }
        self.string_map = new_map;
        self.asm.bind_label(code_start);

        // Phase 2: 编译用户自定义函数（在主代码之前，字符串之后）
        for s in &optimized_stmts {
            if let Stmt::FnDef { name, params, body, .. } = s {
                self.emit_user_fn(name, params, body);
            }
        }

        // 序言
        self.asm.push_reg(Register::RBP);
        self.asm.mov_reg_reg(Register::RBP, Register::RSP);
        // 保存被使用的 callee-saved 寄存器
        for &reg in &self.used_callee_saved {
            self.asm.push_reg(reg);
        }
        // 注意: callee-saved push 了 N 个 8 字节, stack_size 需要加上 N*8 来对齐
        let callee_saved_size = self.used_callee_saved.len() as i32 * 8;
        let total_stack = self.stack_size;
        // 对齐: push rbp (8) + push N callee-saved (N*8) + sub rsp, total
        // 进函数时 RSP ≡ 8 mod 16, push rbp → 0, push N regs → (-N*8), sub rsp → need 0 mod 16
        // 即 (N*8 + total_stack) % 16 == 0
        let aligned_stack = if (callee_saved_size + total_stack) % 16 != 0 {
            total_stack + 8  // 多 sub 8 字节来对齐
        } else {
            total_stack
        };
        self.aligned_stack_size = aligned_stack;
        if aligned_stack > 0 { self.asm.sub_rsp_imm(aligned_stack); }

        // 代码生成
        for s in &optimized_stmts { if !matches!(s, Stmt::FnDef{..}) { self.stmt(s); } }

        // ExitProcess(0) — 无返回值时默认为 0
        self.asm.xor_reg_reg(Register::RCX, Register::RCX);
        let exit_shadow: i32 = if self.aligned_stack_size % 16 == 0 { 0x20 } else { 0x28 };
        self.asm.sub_rsp_imm(exit_shadow);
        let cur = CODE_RVA + self.asm.byte_position() as u32;
        self.asm.call_iat_rva(self.exit_rva.unwrap(), cur);

        let code = self.asm.finish();
        let (import_data, _import_rva, _import_size) = self.imports.build();
        let mut builder = PeBuilder::new();
        builder.add_code(&code);
        builder.set_entry_point(CODE_RVA);
        // v1.0.5: 导入表暂不在简化 PE 构建器中链接
        let _ = (&import_data, _import_rva, _import_size);
        let exe_data = builder.build();
        std::fs::write(output_path, &exe_data)
    }
}
