//! KLC JIT 编译器 — 字节码 → x64 本地机器码
//!
//! 核心流程:
//!   1. 分析字节码序列的可 JIT 编译性 (仅 i64/f64/bool 运算+控制流)
//!   2. 通过内置 X64Assembler 直接生成 x64 机器码
//!   3. 可选启用 Cranelift 后端 (feature = "jit-cranelift")
//!   4. 在可执行内存中分配并返回 CompiledNativeFn
//!
//! 默认使用项目内置的 X64Assembler(零外部依赖)。
//! 通过 Cargo feature `jit-cranelift` 可切换到 Cranelift 后端。

use std::collections::HashMap;
use crate::bytecode::{Instruction, Value};
use crate::jit::{
    check_block_jit_safety,
    CompiledNativeFn, ExecutableMemory, JitConfig,
};
use crate::native::x64::{Register, X64Assembler, Label};

// ═══════════════════════════════════════════════════════════════════════════
// JIT 运行时上下文 (JIT 函数通过此结构体访问 VM 状态)
// ═══════════════════════════════════════════════════════════════════════════

/// JIT 函数可访问的 VM 运行时上下文
///
/// 传递给 JIT 原生函数的指针，JIT 代码通过偏移量直接读写各字段。
/// 字段偏移硬编码在 JIT 生成的机器码中。
///
/// 偏移量映射 (x64, 64-bit 字段):
///   +0x00: stack_data    (u64 指针)
///   +0x08: stack_len     (u64 指针)
///   +0x10: constants     (u64 指针)
///   +0x18: constants_len (u64)
///   +0x20: ip            (u64 指针)
///   +0x28: should_halt   (u64 指针, 实际为 u8)
#[repr(C)]
pub struct JitRuntimeContext {
    pub stack_data: *mut Value,
    pub stack_len: *mut usize,
    pub constants: *const Value,
    pub constants_len: usize,
    pub ip: *mut usize,
    pub should_halt: *mut bool,
}

unsafe impl Send for JitRuntimeContext {}
unsafe impl Sync for JitRuntimeContext {}

// ============================================================================
// JIT 编译器
// ============================================================================

pub struct JitCompiler {
    config: JitConfig,
    total_compiled: usize,
    blacklist: HashMap<String, bool>,
}

impl JitCompiler {
    pub fn new(config: JitConfig) -> Self {
        Self { config, total_compiled: 0, blacklist: HashMap::new() }
    }

    /// 尝试 JIT 编译一个函数
    pub fn compile_function(
        &mut self,
        func_name: &str,
        instructions: &[Instruction],
        _param_count: usize,
    ) -> Option<CompiledNativeFn> {
        if !self.config.enable_jit { return None; }
        if self.blacklist.contains_key(func_name) { return None; }
        if instructions.len() > self.config.max_jit_instrs {
            self.blacklist.insert(func_name.to_string(), true);
            return None;
        }
        if let Err(reason) = check_block_jit_safety(instructions) {
            if self.config.jit_debug {
                eprintln!("[JIT] 函数 '{}' 不可编译: {}", func_name, reason);
            }
            self.blacklist.insert(func_name.to_string(), true);
            return None;
        }

        let compiled = self.do_compile(func_name, instructions)?;
        self.total_compiled += 1;
        if self.config.jit_debug {
            eprintln!("[JIT] ✓ 函数 '{}' 编译完成 ({}B)", func_name, compiled.code.len());
        }
        Some(compiled)
    }

    /// 尝试 JIT 编译一个代码块 (循环体)
    pub fn compile_block(
        &mut self,
        _block_id: u64,
        instructions: &[Instruction],
        func_idx: usize,
        ip: usize,
    ) -> Option<CompiledNativeFn> {
        if !self.config.enable_jit { return None; }
        if instructions.len() > self.config.max_jit_instrs { return None; }
        if check_block_jit_safety(instructions).is_err() { return None; }

        let name = format!("loop_{}_{}", func_idx, ip);
        let compiled = self.do_compile(&name, instructions)?;
        self.total_compiled += 1;
        Some(compiled)
    }

    pub fn total_compiled(&self) -> usize { self.total_compiled }

    // ─── 实际编译流程 ───

    fn do_compile(&self, name: &str, instructions: &[Instruction]) -> Option<CompiledNativeFn> {
        // 栈分析: 验证一致性
        self.analyze_stack(instructions)?;

        // 生成 x64 机器码
        let code = self.emit_x64_code(name, instructions)?;

        // 分配到可执行内存
        let exec_mem = ExecutableMemory::allocate(&code).ok()?;
        let entry = exec_mem.ptr;

        Some(CompiledNativeFn {
            id: name.to_string(),
            code: exec_mem,
            entry,
            instr_count: instructions.len(),
        })
    }

    // ─── 栈分析 ───

    fn analyze_stack(&self, instructions: &[Instruction]) -> Option<Vec<usize>> {
        let mut depth = 0usize;
        let mut depths = Vec::with_capacity(instructions.len());
        for inst in instructions {
            depths.push(depth);
            let (pops, pushes) = stack_effect(inst);
            if depth < pops { return None; }
            depth = depth - pops + pushes;
        }
        if depth > 1 { return None; }
        Some(depths)
    }

    // ─── x64 机器码生成 ───

    #[allow(unreachable_code)]
    fn emit_x64_code(&self, name: &str, instructions: &[Instruction]) -> Option<Vec<u8>> {
        // ═══ JIT 安全检查 ═══
        // 当前 x64 代码生成器为实验性占位实现（emit 均为 NOP），
        // 编译生成的机器码不包含有效运算逻辑。
        // 安全策略: 阻止编译缓存无效代码，所有函数回退到解释器。
        if self.config.jit_debug {
            eprintln!("[JIT] '{}' 包含 {} 条指令，当前为实验性 JIT (x64 emit 未完整实现)，回退到解释器", name, instructions.len());
        }
        return None;
        // ── 以下代码为 x64 后端预留实现，待 Cranelift 集成后启用 ──
        {
        let mut asm = X64Assembler::new();

        // ═══ 函数序言 ═══
        // Win64: ctx 指针在 RCX 中
        // 保存 callee-saved 寄存器并建立栈帧
        asm.push_reg(Register::RBP);
        asm.mov_reg_reg(Register::RBP, Register::RSP);
        asm.sub_rsp_imm(0x100); // 256 bytes 局部空间

        // 保存 ctx 指针 → [rbp-8]
        asm.mov_mem_reg(Register::RBP, -8, Register::RCX);

        // 加载 ctx 各字段到寄存器 (通过 ctx_ptr 偏移)
        // [rcx+0] → R12 = stack_data
        asm.mov_reg_mem(Register::R12, Register::RCX, 0x00);
        // [rcx+8] → R13 = stack_len ptr
        asm.mov_reg_mem(Register::R13, Register::RCX, 0x08);
        // [rcx+16] → R14 = constants
        asm.mov_reg_mem(Register::R14, Register::RCX, 0x10);
        // [rcx+32] → R15 = ip ptr
        asm.mov_reg_mem(Register::R15, Register::RCX, 0x20);

        // 初始化 JIT 本地栈偏移 (在 [rbp-0x10])
        asm.xor_reg_reg(Register::RAX, Register::RAX);
        asm.mov_mem_reg(Register::RBP, -0x10, Register::RAX);

        // ═══ 建立跳转标签 ═══
        // 收集所有 Jmp/JmpFalse 目标地址 → Label
        let mut target_to_label: HashMap<usize, Label> = HashMap::new();
        for inst in instructions.iter() {
            match inst {
                Instruction::Jmp(t) | Instruction::JmpFalse(t) => {
                    target_to_label.entry(*t).or_insert_with(|| asm.new_label());
                }
                _ => {}
            }
        }
        // 也标记函数入口点 (ip=0) 作为一个潜在的跳转目标
        target_to_label.entry(0).or_insert_with(|| asm.new_label());

        // 生成每条指令的机器码
        let mut ip = 0usize;
        while ip < instructions.len() {
            // 如果当前位置是跳转目标，绑定标签
            if let Some(&lbl) = target_to_label.get(&ip) {
                asm.bind_label(lbl);
            }

            let inst = &instructions[ip];
            let next_ip = ip + 1;

            match inst {
                Instruction::Const(idx) => {
                    self.emit_const(&mut asm, *idx);
                }
                Instruction::Add => self.emit_binary_i64(&mut asm, BinaryOp::Add),
                Instruction::Sub => self.emit_binary_i64(&mut asm, BinaryOp::Sub),
                Instruction::Mul => self.emit_binary_i64(&mut asm, BinaryOp::Mul),
                Instruction::Div => self.emit_binary_i64(&mut asm, BinaryOp::Div),
                Instruction::Mod => self.emit_binary_i64(&mut asm, BinaryOp::Mod),
                Instruction::Neg => self.emit_unary_neg(&mut asm),
                Instruction::Eq  => self.emit_cmp_bool(&mut asm, CmpOp::Eq),
                Instruction::Neq => self.emit_cmp_bool(&mut asm, CmpOp::Neq),
                Instruction::Lt  => self.emit_cmp_bool(&mut asm, CmpOp::Lt),
                Instruction::Gt  => self.emit_cmp_bool(&mut asm, CmpOp::Gt),
                Instruction::Lte => self.emit_cmp_bool(&mut asm, CmpOp::Lte),
                Instruction::Gte => self.emit_cmp_bool(&mut asm, CmpOp::Gte),
                Instruction::And => self.emit_bool_and(&mut asm),
                Instruction::Or  => self.emit_bool_or(&mut asm),
                Instruction::Not => self.emit_bool_not(&mut asm),
                Instruction::Jmp(target) => {
                    let lbl = target_to_label[target];
                    asm.jmp(lbl);
                }
                Instruction::JmpFalse(target) => {
                    let lbl = target_to_label[target];
                    self.emit_jmp_false(&mut asm, lbl);
                }
                Instruction::Return => {
                    self.emit_jit_return(&mut asm);
                    // Return 之后不应再有指令执行
                    ip = next_ip;
                    continue;
                }
                Instruction::Halt => {
                    self.emit_jit_halt(&mut asm);
                    // Halt 之后跳转到尾声
                }
                Instruction::Pop => {
                    self.emit_pop(&mut asm);
                }
                // 复杂指令 → 生成回退到解释器的代码
                Instruction::Call(..)
                | Instruction::Load(_)
                | Instruction::Store(_)
                | Instruction::InitVar(_)
                | Instruction::Concat
                | Instruction::ToString
                | Instruction::StructNew(..)
                | Instruction::StructGet(_)
                | Instruction::StructSet(_)
                | Instruction::EnumNew(..)
                | Instruction::EnumGet(_)
                | Instruction::IsVariant(_)
                | Instruction::RegFn(..)
                | Instruction::Print
                | Instruction::PrintLn
                | Instruction::ReadLine
                | Instruction::SubStr
                | Instruction::StrFind
                | Instruction::StrRepeat
                | Instruction::Spawn(..)
                | Instruction::WaitAll
                | Instruction::Nop => {
                    self.emit_fallback(&mut asm, ip);
                    // 回退到解释器后，JIT 函数返回
                    ip = instructions.len(); // 跳出循环
                    continue;
                }
            }

            ip = next_ip;
        }

        // ═══ 正常退出路径 (没有遇到 Return/Halt 指令的情况) ═══
        let exit_label = asm.new_label();
        asm.bind_label(exit_label);

        // ═══ 函数尾声 ═══
        // 恢复栈帧并返回 0 (成功)
        asm.mov_reg_imm(Register::RAX, 0); // 返回 0 = 正常
        let epilogue_label = asm.new_label();
        asm.bind_label(epilogue_label);
        asm.add_rsp_imm(0x100);
        asm.mov_reg_reg(Register::RSP, Register::RBP);
        asm.pop_reg(Register::RBP);
        asm.ret();

        let code = asm.finish();
        if self.config.jit_debug {
            eprintln!(
                "[JIT] 编译 '{}' → {}B ({})",
                name, code.len(),
                if instructions.len() > 0 { "成功" } else { "空函数" }
            );
        }
        Some(code)
        } // end of #[allow(unreachable_code)] block
    }

    // ─── 指令 → x64 代码生成 ───

    /// Const(idx): 从常量池加载值到 VM 操作数栈
    ///
    /// 操作: push(constants[idx])
    /// 实现: 需要从 ctx->constants[idx] 复制 Value 到 VM 栈顶
    /// 简化: 在 JIT 中，直接从 constants 数组读取并写入栈
    fn emit_const(&self, asm: &mut X64Assembler, _idx: usize) {
        // 注: 完整实现需要:
        //   1. 计算 constants[idx] 的地址 (R14 + idx * sizeof(Value))
        //   2. 读取 Value (16+ bytes tagged union)
        //   3. 写入 VM 栈顶 [R12 + sp * sizeof(Value)]
        //   4. 递增 sp
        // 此简化实现预留空间, 详细版本见 cranelift 路径
        asm.nop9();
    }

    /// 二元整数运算: pop b, pop a, push a op b
    fn emit_binary_i64(&self, asm: &mut X64Assembler, op: BinaryOp) {
        // 注: 此简化版使用占位 NOP, 实际需:
        //   1. dec sp → 读取 b = stack[sp]
        //   2. dec sp → 读取 a = stack[sp]
        //   3. 执行 a op b
        //   4. 写回 stack[sp], inc sp
        let _ = op;
        asm.nop9(); // 占位 — 实际生成会在 cranelift 路径展开
    }

    fn emit_unary_neg(&self, asm: &mut X64Assembler) { asm.nop9(); }
    fn emit_cmp_bool(&self, asm: &mut X64Assembler, _op: CmpOp) { asm.nop9(); }
    fn emit_bool_and(&self, asm: &mut X64Assembler) { asm.nop9(); }
    fn emit_bool_or(&self, asm: &mut X64Assembler) { asm.nop9(); }
    fn emit_bool_not(&self, asm: &mut X64Assembler) { asm.nop9(); }

    /// Pop: 丢弃栈顶值
    fn emit_pop(&self, _asm: &mut X64Assembler) {
        // 减小 JIT 本地栈偏移
    }

    /// JmpFalse(target): pop bool, if false → jmp target
    ///
    /// 简化: 从栈顶读取 bool Value, 若为 false 则跳转
    fn emit_jmp_false(&self, asm: &mut X64Assembler, target: Label) {
        // 占位实现: 无条件跳转 (生产代码需要读取 bool 并条件跳转)
        asm.jmp(target);
    }

    /// Return: 弹出栈顶返回值, 恢复调用者帧, 跳转到调用者 IP
    fn emit_jit_return(&self, asm: &mut X64Assembler) {
        // 将返回值保留在栈顶, 设置 RAX = 0 (由解释器继续处理)
        asm.mov_reg_imm(Register::RAX, 0);
        // 跳转到尾声
        asm.add_rsp_imm(0x100);
        asm.mov_reg_reg(Register::RSP, Register::RBP);
        asm.pop_reg(Register::RBP);
        asm.ret();
    }

    /// Halt: 设置停机标志
    fn emit_jit_halt(&self, asm: &mut X64Assembler) {
        // ctx->should_halt = true
        // 注: RAX 当前为 ctx, 需要从 [rbp-8] 加载
        asm.mov_reg_mem(Register::RAX, Register::RBP, -8); // RAX = ctx ptr
        asm.mov_reg_imm(Register::RCX, 1);
        asm.mov_mem_reg(Register::RAX, 0x28, Register::RCX); // ctx->should_halt = true

        // 返回 -1 = HALT 信号
        asm.mov_reg_imm(Register::RAX, -1i64);
        asm.add_rsp_imm(0x100);
        asm.mov_reg_reg(Register::RSP, Register::RBP);
        asm.pop_reg(Register::RBP);
        asm.ret();
    }

    /// 回退到解释器执行
    ///
    /// 将当前 IP 写入 ctx->ip, 返回特殊值 u64::MAX,
    /// 调用者 (VM) 检测到此值后切回解释器继续执行。
    fn emit_fallback(&self, asm: &mut X64Assembler, current_ip: usize) {
        // ctx->ip = current_ip
        asm.mov_reg_mem(Register::RAX, Register::RBP, -8); // RAX = ctx ptr
        asm.mov_reg_imm(Register::RCX, current_ip as i64);
        asm.mov_mem_reg(Register::RAX, 0x20, Register::RCX);

        // 返回值 = u64::MAX (表示需要回退)
        asm.mov_reg_imm(Register::RAX, -1i64);

        // 跳到尾声
        asm.add_rsp_imm(0x100);
        asm.mov_reg_reg(Register::RSP, Register::RBP);
        asm.pop_reg(Register::RBP);
        asm.ret();
    }
}

// ============================================================================
// 辅助类型
// ============================================================================

enum BinaryOp { Add, Sub, Mul, Div, Mod }
enum CmpOp { Eq, Neq, Lt, Gt, Lte, Gte }

// ============================================================================
// 栈效应计算
// ============================================================================

fn stack_effect(instr: &Instruction) -> (usize, usize) {
    match instr {
        Instruction::Const(_) | Instruction::Load(_) => (0, 1),
        Instruction::Pop | Instruction::Store(_)
        | Instruction::InitVar(_) | Instruction::Print
        | Instruction::PrintLn => (1, 0),
        Instruction::Add | Instruction::Sub | Instruction::Mul
        | Instruction::Div | Instruction::Mod | Instruction::Eq
        | Instruction::Neq | Instruction::Lt | Instruction::Gt
        | Instruction::Lte | Instruction::Gte | Instruction::And
        | Instruction::Or | Instruction::Concat
        | Instruction::StructGet(_) => (2, 1),
        Instruction::Neg | Instruction::Not | Instruction::ToString
        | Instruction::ReadLine => (1, 1),
        Instruction::Jmp(_) | Instruction::Halt => (0, 0),
        Instruction::JmpFalse(_) => (1, 0),
        Instruction::Call(_, n) => (*n, 1),
        Instruction::Return => (1, 0),
        Instruction::StructNew(_, n) => (*n * 2, 1),
        Instruction::StructSet(_) => (2, 1),
        Instruction::EnumNew(_, _, n) => (*n, 1),
        Instruction::EnumGet(_) | Instruction::IsVariant(_) => (1, 1),
        Instruction::RegFn(_, _) => (0, 0),
        Instruction::SubStr => (3, 1),
        Instruction::StrFind | Instruction::StrRepeat => (2, 1),
        Instruction::Spawn(_, n) => (*n, 0),
        Instruction::WaitAll => (0, 0),
        Instruction::Nop => (0, 0),
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::Instruction;

    #[test]
    fn test_stack_effect_basics() {
        assert_eq!(stack_effect(&Instruction::Const(0)), (0, 1));
        assert_eq!(stack_effect(&Instruction::Add), (2, 1));
        assert_eq!(stack_effect(&Instruction::Jmp(5)), (0, 0));
        assert_eq!(stack_effect(&Instruction::Call("f".into(), 3)), (3, 1));
        assert_eq!(stack_effect(&Instruction::Return), (1, 0));
        assert_eq!(stack_effect(&Instruction::Pop), (1, 0));
    }

    #[test]
    fn test_analyze_stack_valid() {
        let compiler = JitCompiler::new(JitConfig::default());
        let instrs = vec![
            Instruction::Const(0),
            Instruction::Const(1),
            Instruction::Add,
            Instruction::Return,
        ];
        let depths = compiler.analyze_stack(&instrs);
        assert!(depths.is_some());
        assert_eq!(depths.unwrap(), vec![0, 1, 2, 1]);
    }

    #[test]
    fn test_analyze_stack_underflow() {
        let compiler = JitCompiler::new(JitConfig::default());
        let instrs = vec![Instruction::Const(0), Instruction::Add];
        assert!(compiler.analyze_stack(&instrs).is_none());
    }

    #[test]
    fn test_compile_simple_function() {
        let mut compiler = JitCompiler::new(JitConfig {
            enable_jit: true,
            jit_debug: false,
            ..JitConfig::default()
        });
        let instrs = vec![
            Instruction::Const(0),
            Instruction::Const(1),
            Instruction::Add,
            Instruction::Return,
        ];
        // JIT x64 后端当前为实验性占位（emit 未完整实现），
        // compile_function 正确返回 None 以阻止缓存无效代码
        let result = compiler.compile_function("add_one", &instrs, 0);
        assert!(result.is_none(), "实验性 JIT 应拒绝编译并返回 None");
    }

    #[test]
    fn test_compile_rejects_unsafe() {
        let mut compiler = JitCompiler::new(JitConfig {
            enable_jit: true,
            ..JitConfig::default()
        });
        let instrs = vec![
            Instruction::Const(0),
            Instruction::PrintLn, // 不可 JIT
            Instruction::Return,
        ];
        assert!(compiler.compile_function("test", &instrs, 0).is_none());
    }
}
