//! KLC 主程序 — 编译器 + VM + 工具链入口

mod token;
mod lexer;
mod ast;
mod parser;
mod bytecode;
mod codegen;
mod vm;
mod native;
mod native_codegen;
mod formatter;
mod module;
mod dwarf;
mod gui;
mod bytecode_optimize;

use std::env;
use std::fs;
use std::path::Path;
use std::process;

use lexer::Lexer;
use parser::Parser;
use codegen::Codegen;
use vm::VM;
use bytecode::BytecodeProgram;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        // 双击运行时默认启动 IDE
        gui::run_ide();
        return;
    }

    let subcmd = args[1].as_str();

    match subcmd {
        // ─── 子命令: klc fmt ───
        "fmt" => cmd_fmt(&args[2..]),
        // ─── 子命令: klc build ───
        "build" => cmd_build(&args[2..]),
        // ─── 子命令: klc run ───
        "run" => cmd_run(&args[2..]),
        // ─── 子命令: klc check ───
        "check" => cmd_check(&args[2..]),
        // ─── 子命令: klc --ide 启动图形界面 ───
        "--ide" => gui::run_ide(),
        // ─── 子命令: klc version ───
        "version" | "--version" | "-v" => {
            println!("KLC v0.8.4 — Kaleidoscope Language Compiler");
        }
        // ─── 子命令: klc help ───
        "help" | "--help" | "-h" => print_usage(),

        // ─── 兼容原有命令 ───
        "--native" => {
            let no_opt = args.get(2).map(|s| s.as_str()) == Some("--no-opt");
            let gen_dbg = args.iter().any(|s| s == "--debug-info" || s == "-g");
            let file_idx = if no_opt { 3 } else { 2 };
            if args.len() <= file_idx {
                eprintln!("Error: --native requires a source file");
                process::exit(1);
            }
            compile_native(&args[file_idx], no_opt, gen_dbg);
        }
        "--test-pe" => test_pe_generation(),
        "--test-x64" => test_x64_generation(),
        "--test-imports" => test_imports_generation(),
        "--debug" => {
            let file_path = args.get(2)
                .unwrap_or_else(|| { eprintln!("Error: No source file specified"); process::exit(1); });
            run_vm(file_path, true);
        }
        _ => {
            // 默认: VM 执行 <source.klc>
            run_vm(&args[1], false);
        }
    }
}

fn print_usage() {
    eprintln!("KLC v0.8.4 — Kaleidoscope Language Compiler");
    eprintln!();
    eprintln!("USAGE:");
    eprintln!("    klc <source.klc>                  VM 执行");
    eprintln!("    klc run <source.klc>             VM 执行");
    eprintln!("    klc build [OPTIONS] <source>     项目构建");
    eprintln!("    klc fmt [OPTIONS] <file>...      格式化代码");
    eprintln!("    klc check <source.klc>           语法检查");
    eprintln!();
    eprintln!("OPTIONS:");
    eprintln!("    --native          编译为原生 Windows EXE");
    eprintln!("    --no-opt          禁用优化");
    eprintln!("    -g, --debug-info  生成 DWARF 调试信息");
    eprintln!("    --debug           显示 Token + AST + 字节码");
    eprintln!("    --check           仅检查语法，不执行");
    eprintln!("    -o <output>       指定输出文件路径");
    eprintln!("    --ide             启动 KLC 图形界面 IDE");
    eprintln!();
    eprintln!("FORMAT OPTIONS (klc fmt):");
    eprintln!("    --check           仅检查格式，不写入文件");
    eprintln!("    --indent <N>      缩进宽度 (默认 4)");
}

// ============================================================================
// 子命令实现
// ============================================================================

/// klc fmt — 代码格式化
fn cmd_fmt(args: &[String]) {
    let check_only = args.iter().any(|s| s == "--check");
    let indent = args.iter().position(|s| s == "--indent")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(4);

    let mut config = formatter::FormatConfig::default();
    config.indent_width = indent;

    let files: Vec<&str> = args.iter()
        .filter(|s| !s.starts_with('-'))
        .map(String::as_str)
        .collect();

    if files.is_empty() {
        eprintln!("Error: klc fmt requires at least one file");
        eprintln!("  Usage: klc fmt [OPTIONS] <file>...");
        process::exit(1);
    }

    let mut changed_count = 0;
    let mut error_count = 0;

    for file in &files {
        match formatter::format_file(file, &config) {
            Ok((changed, formatted)) => {
                if check_only {
                    if changed {
                        println!("{}: needs formatting", file);
                        changed_count += 1;
                    } else {
                        println!("{}: OK", file);
                    }
                } else {
                    if changed {
                        if let Err(e) = fs::write(file, &formatted) {
                            eprintln!("Error writing '{}': {}", file, e);
                            error_count += 1;
                        } else {
                            println!("Formatted {}", file);
                            changed_count += 1;
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Error formatting '{}': {}", file, e);
                error_count += 1;
            }
        }
    }

    if check_only && changed_count > 0 {
        process::exit(1);
    }
    if error_count > 0 {
        process::exit(1);
    }
}

/// klc build — 项目构建
fn cmd_build(args: &[String]) {
    let native = args.iter().any(|s| s == "--native");
    let no_opt = args.iter().any(|s| s == "--no-opt");
    let gen_dbg = args.iter().any(|s| s == "--debug-info" || s == "-g");

    // 查找输出参数
    let output = args.iter().position(|s| s == "-o")
        .and_then(|i| args.get(i + 1))
        .map(String::as_str);

    // 查找源文件（非 -- 开头的参数）
    let source = args.iter()
        .find(|s| !s.starts_with('-'))
        .map(String::as_str);

    let source = source.unwrap_or("main.klc");

    let source_path = Path::new(source);
    if !source_path.exists() {
        eprintln!("Error: source file '{}' not found", source);
        process::exit(1);
    }

    // 使用模块系统解析
    let root_dir = source_path.parent().unwrap_or(Path::new(".")).to_path_buf();
    let mut builder = module::ProjectBuilder::new(&root_dir)
        .entry_file(source_path.file_name().unwrap_or_default().to_str().unwrap_or("main.klc"))
        .native(native)
        .no_opt(no_opt)
        .debug_info(gen_dbg);

    if let Some(out) = output {
        builder = builder.output(Path::new(out));
    }

    match builder.build() {
        Ok(result) => {
            println!("Compiling {} ({} modules, {} statements)",
                source, result.modules_count, result.stmt_count);

            if result.native {
                // 原生编译
                let mut gen = native_codegen::NativeCodeGenerator::new();
                gen.no_opt = no_opt;
                let output_str = result.output_path.display().to_string();
                match gen.compile(&result.program, &output_str) {
                    Ok(()) => {
                        let meta = fs::metadata(&result.output_path).unwrap();
                        println!("  Generated: {} ({} bytes, {:.0} KB)",
                            result.output_path.display(),
                            meta.len(), meta.len() as f64 / 1024.0);
                        if result.debug_info {
                            println!("  DWARF debug info: enabled");
                        }
                    }
                    Err(e) => {
                        eprintln!("Compile error: {}", e);
                        process::exit(1);
                    }
                }
            } else {
                // VM 执行 — 启用 AST 优化
                let mut program = result.program;
                bytecode_optimize::optimize_program(&mut program);
                let bytecode = match Codegen::compile(&program) {
                    Ok(bc) => bc,
                    Err(e) => {
                        eprintln!("Codegen error: {}", e);
                        process::exit(1);
                    }
                };
                let mut vm = VM::new(bytecode);
                if let Err(e) = vm.run() {
                    eprintln!("Runtime error: {}", e);
                    process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("Build error: {}", e);
            process::exit(1);
        }
    }
}

/// klc run — VM 执行
fn cmd_run(args: &[String]) {
    let debug = args.iter().any(|s| s == "--debug");
    let source = args.iter()
        .find(|s| !s.starts_with('-'))
        .map(String::as_str)
        .unwrap_or_else(|| {
            eprintln!("Error: klc run requires a source file");
            process::exit(1);
        });
    run_vm(source, debug);
}

/// klc check — 语法检查
fn cmd_check(args: &[String]) {
    let source = args.iter()
        .find(|s| !s.starts_with('-'))
        .map(String::as_str)
        .unwrap_or_else(|| {
            eprintln!("Error: klc check requires a source file");
            process::exit(1);
        });

    let source_text = match fs::read_to_string(source) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading '{}': {}", source, e);
            process::exit(1);
        }
    };

    let mut lexer = Lexer::new(&source_text);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    match parser.parse_program() {
        Ok(program) => {
            println!("{}: OK ({} statements)", source, program.statements.len());
        }
        Err(e) => {
            eprintln!("{}: error: {}", source, e);
            process::exit(1);
        }
    }
}

/// VM 执行（共用逻辑）
fn run_vm(file_path: &str, debug: bool) {
    let source = match fs::read_to_string(file_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", file_path, e);
            process::exit(1);
        }
    };

    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize();

    if debug {
        println!("=== Tokens ===");
        for t in &tokens {
            if t.kind != token::TokenKind::Eof {
                println!("  [{:?}] '{}' @ {}:{}", t.kind, t.lexeme, t.line, t.col);
            }
        }
        println!();
    }

    let mut parser = Parser::new(tokens);
    let program = match parser.parse_program() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Parse error: {}", e);
            process::exit(1);
        }
    };

    if debug {
        println!("=== AST ===");
        print_ast(&program, 0);
        println!();
    }

    // ─── AST 优化（常量折叠 + 死代码消除 + math 内联 + 循环简化） ───
    let mut program = program;
    bytecode_optimize::optimize_program(&mut program);

    let bytecode = match Codegen::compile(&program) {
        Ok(bc) => bc,
        Err(e) => {
            eprintln!("Codegen error: {}", e);
            process::exit(1);
        }
    };

    if debug {
        print_bytecode(&bytecode);
        println!();
    }

    let mut vm = VM::new(bytecode);
    if let Err(e) = vm.run() {
        eprintln!("Runtime error: {}", e);
        process::exit(1);
    }
}

fn print_ast(program: &ast::Program, indent: usize) {
    for stmt in &program.statements {
        print_stmt(stmt, indent);
    }
}

fn print_stmt(stmt: &ast::Stmt, indent: usize) {
    let pad = "  ".repeat(indent);
    match stmt {
        ast::Stmt::Let { name: _, mutable, value, type_ann } => {
            let mut_str = if *mutable { " mut" } else { "" };
            let type_str = type_ann.as_ref().map(|t| format!(": {}", t)).unwrap_or_default();
            println!("{}Let{}{} = {:?}", pad, mut_str, type_str, value);
        }
        ast::Stmt::Assign { name, value } => {
            println!("{}Assign {} = {:?}", pad, name, value);
        }
        ast::Stmt::FieldAssign { obj, field, value } => {
            println!("{}FieldAssign {}.{} = {:?}", pad, obj, field, value);
        }
        ast::Stmt::Expr(expr) => {
            println!("{}Expr({:?})", pad, expr);
        }
        ast::Stmt::Return(expr) => {
            println!("{}Return {:?}", pad, expr);
        }
        ast::Stmt::While(cond, body) => {
            println!("{}While {:?}", pad, cond);
            for s in body { print_stmt(s, indent + 1); }
            println!("{}EndWhile", pad);
        }
        ast::Stmt::For { var, iterable, body } => {
            println!("{}For {} in {:?}", pad, var, iterable);
            for s in body { print_stmt(s, indent + 1); }
            println!("{}EndFor", pad);
        }
        ast::Stmt::If { cond, then_block, else_block } => {
            println!("{}If {:?}", pad, cond);
            println!("{}Then:", pad);
            for s in then_block { print_stmt(s, indent + 1); }
            if let Some(else_b) = else_block {
                println!("{}Else:", pad);
                for s in else_b { print_stmt(s, indent + 1); }
            }
            println!("{}EndIf", pad);
        }
        ast::Stmt::Block(stmts) => {
            for s in stmts { print_stmt(s, indent); }
        }
        ast::Stmt::PrintLn(expr) => {
            println!("{}PrintLn({:?})", pad, expr);
        }
        ast::Stmt::Print(expr) => {
            println!("{}Print({:?})", pad, expr);
        }
        ast::Stmt::Exit(expr) => {
            println!("{}Exit({:?})", pad, expr);
        }
        ast::Stmt::FnDef { name, params, return_type, body } => {
            println!("{}FnDef {} ({:?}) -> {:?}", pad, name, params, return_type);
            for s in body { print_stmt(s, indent + 1); }
            println!("{}EndFnDef", pad);
        }
        ast::Stmt::EnumDef { name, variants } => {
            println!("{}EnumDef {} {{", pad, name);
            for v in variants {
                print!("{}  {}", pad, v.name);
                if !v.fields.is_empty() {
                    let fstr: Vec<String> = v.fields.iter().map(|f| f.type_ann.clone()).collect();
                    print!("({})", fstr.join(", "));
                }
                println!();
            }
            println!("{}}}", pad);
        }
        ast::Stmt::TypeDef { name, fields } => {
            println!("{}TypeDef {} {{", pad, name);
            for f in fields {
                let default_str = f.default.as_ref().map(|d| format!(" = {:?}", d)).unwrap_or_default();
                println!("{}  {}: {}{}", pad, f.name, f.type_ann, default_str);
            }
            println!("{}}}", pad);
        }
        ast::Stmt::ImplBlock { type_name, methods } => {
            println!("{}Impl {} {{", pad, type_name);
            for m in methods { print_stmt(m, indent + 1); }
            println!("{}}}", pad);
        }
        ast::Stmt::Break => println!("{}Break", pad),
        ast::Stmt::Continue => println!("{}Continue", pad),
    }
}

fn print_bytecode(program: &BytecodeProgram) {
    println!("=== Constants ===");
    for (i, c) in program.constants.iter().enumerate() {
        println!("  [{}] {:?}", i, c);
    }

    println!("\n=== Main ===");
    for (i, inst) in program.main.iter().enumerate() {
        println!("  {:04} {:?}", i, inst);
    }

    for func in &program.functions {
        println!("\n=== Function: {} ({} params) ===", func.name, func.param_count);
        for (i, inst) in func.instructions.iter().enumerate() {
            println!("  {:04} {:?}", i, inst);
        }
    }
}

// ============================================================================
// 原生 PE 文件生成器测试（阶段一）
// ============================================================================

fn test_pe_generation() {
    println!("╔══════════════════════════════════════════════╗");
    println!("║   KLC 原生 PE 生成器 — 阶段一测试              ║");
    println!("║   最小 Windows x86_64 EXE 文件生成             ║");
    println!("╚══════════════════════════════════════════════╝");
    println!();

    let machine_code: Vec<u8> = vec![
        0x31, 0xC0,  // xor eax, eax  → 退出码 = 0
        0xC3,        // ret           → 返回加载器，进程结束
    ];

    println!("→ 机器码:");
    println!("    31 C0    xor eax, eax    ; 设置退出码为 0");
    println!("    C3       ret              ; 返回加载器，进程终止");
    println!();

    let output_path = "minimal.exe";

    match native::compile_to_exe(output_path, &machine_code) {
        Ok(()) => {
            let metadata = match fs::metadata(output_path) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("✗ 无法读取生成文件: {}", e);
                    process::exit(1);
                }
            };

            let size = metadata.len();
            println!("═══════════════════════════════════════════════");
            println!("  ✓ 成功生成: {}", output_path);
            println!("  ✓ 文件大小: {} 字节 ({:.1} KB)", size, size as f64 / 1024.0);
            println!();
            println!("  PE 结构:");
            println!("    - 机器类型:        x86_64 (AMD64)");
            println!("    - 子系统:          Windows Console (CUI)");
            println!("    - 映像基地址:     0x00000000_00400000");
            println!("    - 入口点 RVA:     0x1000");
            println!("    - 代码节:          .text");
            println!("    - 节对齐:          4096 字节");
            println!("    - 文件对齐:        512 字节");
            println!();
            println!("  验证步骤:");
            println!("    1. 双击 minimal.exe → 应立即退出，无错误弹窗");
            println!("    2. cmd 中运行: echo %errorlevel% → 应显示 0");
            println!("    3. 用 x64dbg 打开 → 可看到 xor eax, eax; ret");
            println!("    4. 文件大小应约 1KB (1024 字节)");
            println!("═══════════════════════════════════════════════");

            if size == 1024 {
                println!("\n  ✓ 文件大小验证通过 (恰好 1024 字节)");
            } else {
                println!("\n  ⚠ 文件大小: {} 字节 (预期 1024 字节)", size);
            }
        }
        Err(e) => {
            eprintln!("✗ PE 文件生成失败: {}", e);
            process::exit(1);
        }
    }
}

// ============================================================================
// 原生 x86_64 指令生成器测试（阶段二）
// ============================================================================

fn test_x64_generation() {
    use native::x64::{Register, X64Assembler};

    println!("╔══════════════════════════════════════════════╗");
    println!("║   KLC x86_64 指令生成器 — 阶段二测试          ║");
    println!("║   手动指令编码 + 寄存器分配                     ║");
    println!("╚══════════════════════════════════════════════╝");
    println!();

    // 测试 1: 计算 1+2=3
    println!("══════ 测试 1: 计算 1+2=3 ══════");
    println!();
    println!("  汇编代码:");
    println!("    mov eax, 1       ; EAX = 1");
    println!("    mov ecx, 2       ; ECX = 2");
    println!("    add rax, rcx     ; RAX = RAX + RCX = 3");
    println!("    ret              ; 返回，EAX=3=退出码");
    println!();

    let mut asm = X64Assembler::new();
    asm.mov_reg_imm(Register::RAX, 1);
    asm.mov_reg_imm(Register::RCX, 2);
    asm.add_reg_reg(Register::RAX, Register::RCX);
    asm.ret();

    let calc_code = asm.finish();

    println!("  生成的机器码:");
    print_hex_dump(&calc_code);
    println!();
    println!("  x64dbg 反汇编验证:");
    println!("    {:02X} {:02X} {:02X} {:02X} {:02X}       mov eax, 1", calc_code[0], calc_code[1], calc_code[2], calc_code[3], calc_code[4]);
    println!("    {:02X} {:02X} {:02X} {:02X} {:02X}       mov ecx, 2", calc_code[5], calc_code[6], calc_code[7], calc_code[8], calc_code[9]);
    println!("    {:02X} {:02X} {:02X}                   add rax, rcx", calc_code[10], calc_code[11], calc_code[12]);
    println!("    {:02X}                               ret", calc_code[13]);
    println!();

    match native::compile_to_exe("test_x64_calc.exe", &calc_code) {
        Ok(()) => {
            let size = fs::metadata("test_x64_calc.exe").unwrap().len();
            println!("  ✓ 已生成: test_x64_calc.exe ({} 字节)", size);
            println!("  → 运行退出码应为 3 (1+2)");
        }
        Err(e) => eprintln!("  ✗ 生成失败: {}", e),
    }
    println!();

    // 测试 2: 指令编码演示
    println!("══════ 测试 2: 指令编码演示 ══════");
    println!();
    println!("  --- 算术指令 ---");

    let code = compile(|asm| {
        asm.inc_reg(Register::RAX);
        asm.dec_reg(Register::RBX);
    });
    print_instr_demo(&code, "inc rax; dec rbx");

    let code = compile(|asm| {
        asm.mul_reg(Register::RCX);
        asm.xor_reg_reg(Register::RDX, Register::RDX);
        asm.div_reg(Register::R8);
    });
    print_instr_demo(&code, "mul rcx; xor rdx, rdx; div r8");

    let code = compile(|asm| {
        asm.push_reg(Register::RBX);
        asm.push_reg(Register::R12);
        asm.pop_reg(Register::R12);
        asm.pop_reg(Register::RBX);
    });
    print_instr_demo(&code, "push rbx; push r12; pop r12; pop rbx");

    let code = compile(|asm| {
        asm.mov_reg_mem(Register::RAX, Register::RBX, 8);
        asm.mov_mem_reg(Register::RBX, 16, Register::RCX);
    });
    print_instr_demo(&code, "mov rax, [rbx+8]; mov [rbx+16], rcx");

    println!();

    // 测试 3: 条件跳转
    println!("══════ 测试 3: 条件跳转 ══════");
    println!();

    let mut asm = X64Assembler::new();
    asm.mov_reg_imm(Register::RAX, 10);
    asm.mov_reg_imm(Register::RCX, 0);
    let loop_lbl = asm.new_label();
    asm.bind_label(loop_lbl);
    asm.dec_reg(Register::RAX);
    asm.cmp_reg_reg(Register::RAX, Register::RCX);
    asm.jg(loop_lbl);
    asm.ret();

    let jump_code = asm.finish();

    println!("  汇编代码:");
    println!("    mov eax, 10");
    println!("    mov ecx, 0");
    println!("  loop:");
    println!("    dec rax");
    println!("    cmp rax, rcx");
    println!("    jg loop          ; 循环直到 RAX=0");
    println!("    ret               ; 退出码=0");
    println!();
    println!("  生成的机器码 ({} 字节):", jump_code.len());
    print_hex_dump(&jump_code);
    println!();

    match native::compile_to_exe("test_x64_jump.exe", &jump_code) {
        Ok(()) => {
            let size = fs::metadata("test_x64_jump.exe").unwrap().len();
            println!("  ✓ 已生成: test_x64_jump.exe ({} 字节)", size);
            println!("  → 运行退出码应为 0");
        }
        Err(e) => eprintln!("  ✗ 生成失败: {}", e),
    }
    println!();

    // 测试 4: ExitProcess 调用模式（不可运行，供审查）
    println!("══════ 测试 4: ExitProcess(0) 调用模式 ══════");
    println!();
    println!("  汇编代码 (阶段三实现导入表后可运行):");
    println!("    xor ecx, ecx     ; RCX = 0 (退出码)");
    println!("    call <ExitProcess>");
    println!();

    let mut asm2 = X64Assembler::new();
    asm2.xor_reg_reg(Register::RCX, Register::RCX);
    let ep_code = asm2.finish();

    println!("  xor ecx, ecx 编码:");
    print_hex_dump(&ep_code);
    println!("  (call ExitProcess 的完整编码将在阶段三实现)");
    println!();

    // 测试 5: 寄存器分配器演示
    println!("══════ 测试 5: 寄存器分配器 ══════");
    println!();

    use native::regalloc::RegisterAllocator;
    let mut ra = RegisterAllocator::new();
    ra.begin_function();

    let vars = ["x", "y", "z", "sum", "temp"];
    for var in &vars {
        let reg = ra.alloc_var(var);
        println!("  变量 {} → {:?}", var, reg);
    }

    ra.free_var("x");
    ra.free_var("y");
    let reg = ra.alloc_var("result");
    println!("  释放 x, y 后: result → {:?}", reg);

    let saved = ra.used_saved_regs();
    if !saved.is_empty() {
        println!("  使用的被调用者保存寄存器: {:?}", saved);
    }

    let params = RegisterAllocator::param_registers();
    println!("  Microsoft x64 参数寄存器: {:?} {:?} {:?} {:?}",
        params[0], params[1], params[2], params[3]);
    let ret_reg = RegisterAllocator::return_register();
    println!("  返回值寄存器: {:?}", ret_reg);

    println!();
    println!("═══════════════════════════════════════════════");
    println!("  阶段二测试完成!");
    println!("  - test_x64_calc.exe → 退出码应为 3");
    println!("  - test_x64_jump.exe → 退出码应为 0");
    println!("  - 可用 x64dbg 打开验证所有指令编码");
    println!("═══════════════════════════════════════════════");
}

fn compile<F>(f: F) -> Vec<u8>
where
    F: FnOnce(&mut native::x64::X64Assembler),
{
    let mut asm = native::x64::X64Assembler::new();
    f(&mut asm);
    asm.finish()
}

fn print_instr_demo(code: &[u8], desc: &str) {
    print!("    ");
    print_hex_dump(code);
    let pad = " ".repeat(40usize.saturating_sub(code.len() * 3));
    println!("{}; {}", pad, desc);
}

fn print_hex_dump(bytes: &[u8]) {
    for b in bytes {
        print!("{:02X} ", b);
    }
    println!();
}

// ============================================================================
// 导入表生成器测试（阶段三）
// ============================================================================

fn test_imports_generation() {
    use native::x64::{Register, X64Assembler};
    use native::imports::ImportTableBuilder;
    use native::pe::{self, PeBuilder};

    println!("╔══════════════════════════════════════════════╗");
    println!("║   KLC 导入表生成器 — 阶段三测试                ║");
    println!("║   调用 Windows API 输出 Hello, KLC!            ║");
    println!("╚══════════════════════════════════════════════╝");
    println!();

    let code_rva: u32 = pe::CODE_RVA;     // 0x1000
    let idata_rva: u32 = pe::IDATA_RVA;   // 0x2000

    // === 步骤 1: 构建导入表 ===
    println!("── 步骤 1: 构建导入表 ──");
    let mut imports = ImportTableBuilder::new();
    imports.set_base_rva(idata_rva);

    let getstd_rva: u32 = imports.add_import("kernel32.dll", "GetStdHandle");
    let write_rva: u32 = imports.add_import("kernel32.dll", "WriteConsoleA");
    let exit_rva: u32 = imports.add_import("kernel32.dll", "ExitProcess");

    let (import_data, import_table_rva, import_table_size) = imports.build();

    println!("  GetStdHandle  IAT: 0x{:08X}", getstd_rva);
    println!("  WriteConsoleA IAT: 0x{:08X}", write_rva);
    println!("  ExitProcess   IAT: 0x{:08X}", exit_rva);
    println!("  导入数据: {} 字节", import_data.len());
    println!();

    // === 步骤 2: 生成机器码 ===
    println!("── 步骤 2: 生成机器码 ──");

    let mut asm = X64Assembler::new();
    let rva = |a: &X64Assembler| code_rva + a.byte_position() as u32;

    // --- GetStdHandle(-11) ---
    asm.sub_rsp_imm(0x28);
    asm.mov_reg_imm(Register::RCX, -11);
    asm.call_iat_rva(getstd_rva, rva(&asm));
    asm.add_rsp_imm(0x28);
    asm.mov_reg_reg(Register::R12, Register::RAX);  // 保存句柄

    // --- WriteConsoleA ---
    asm.sub_rsp_imm(0x38);
    asm.xor_reg_reg(Register::RCX, Register::RCX);
    asm.mov_mem_reg(Register::RSP, 0x28, Register::RCX);  // *written = 0
    asm.mov_mem_reg(Register::RSP, 0x20, Register::RCX);  // arg5 = NULL
    asm.lea_rsp_disp8(Register::R9, 0x28);                 // R9 = &written
    asm.mov_reg_imm(Register::R8, 14);                     // len = 14 (6字节, R8需REX.B)

    // 消息 RVA = 当前位置 + 剩余指令 (lea7 + mov3 + call6 + xor3 + call6 = 25)
    let msg_rva = code_rva + asm.byte_position() as u32 + 25;
    asm.lea_rip_rva(Register::RDX, msg_rva, rva(&asm));

    asm.mov_reg_reg(Register::RCX, Register::R12);          // handle
    asm.call_iat_rva(write_rva, rva(&asm));
    // 不恢复栈 — 复用已分配的 shadow space 给 ExitProcess

    // --- ExitProcess(0) ---
    asm.xor_reg_reg(Register::RCX, Register::RCX);
    asm.call_iat_rva(exit_rva, rva(&asm));

    // --- 消息 ---
    let msg: &[u8] = b"Hello, KLC!\r\n\0";
    let mut code = asm.finish();
    code.extend_from_slice(msg);

    println!("  机器码 ({} 字节):", code.len());
    print_hex_dump(&code);
    println!();

    // === 步骤 3: 生成 PE ===
    println!("── 步骤 3: 生成 PE ──");

    let mut builder = PeBuilder::new();
    builder.add_code(&code);
    builder.set_entry_point(pe::CODE_RVA);
    builder.add_import_table(&import_data, import_table_rva, import_table_size);

    let exe_data = builder.build();
    let output_path = "hello_klc.exe";
    std::fs::write(output_path, &exe_data).unwrap();

    let size = std::fs::metadata(output_path).unwrap().len();
    println!("  ✓ 已生成: {} ({} 字节, {:.1} KB)", output_path, size, size as f64 / 1024.0);
    println!();

    println!("═══ 导入表信息 ═══");
    println!("  DLL: kernel32.dll");
    println!("  函数: GetStdHandle, WriteConsoleA, ExitProcess");
    println!("  IAT ExitProcess: 0x{:08X}", exit_rva);
    println!("  IDT RVA: 0x{:08X}", import_table_rva);
    println!("  IDT 大小: {} 字节", import_table_size);
    println!();
    println!("═══════════════════════════════════════════════");
    println!("  验证: .\\hello_klc.exe → 应输出 'Hello, KLC!'");
    println!("═══════════════════════════════════════════════");
}

#[allow(dead_code)]
fn print_hex_lines(bytes: &[u8], base_rva: u32) {
    for (i, chunk) in bytes.chunks(16).enumerate() {
        print!("    {:04X}: ", base_rva + (i * 16) as u32);
        for b in chunk {
            print!("{:02X} ", b);
        }
        println!();
    }
}

// ============================================================================
// 原生编译 (阶段四)
// ============================================================================

fn compile_native(file_path: &str, no_opt: bool, gen_debug: bool) {
    use native_codegen::NativeCodeGenerator;
    use dwarf::{DwarfGenerator, DebugInfoSource};

    let source = match fs::read_to_string(file_path) {
        Ok(s) => s,
        Err(e) => { eprintln!("error: can't read '{}': {}", file_path, e); process::exit(1); }
    };

    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let program = match parser.parse_program() {
        Ok(p) => p,
        Err(e) => {
            let lines: Vec<&str> = source.lines().collect();
            eprintln!("\n╔══ KLC Compile Error ══╗");
            eprintln!("║  {}", e);
            eprintln!("╚═══════════════════════╝");
            for line_num in 1..=lines.len() {
                if e.contains(&line_num.to_string()) {
                    eprintln!();
                    if line_num > 1 { eprintln!(" {} | {}", line_num - 1, lines[line_num - 2]); }
                    let err_line = lines[line_num - 1];
                    eprintln!(" {} | {}", line_num, err_line);
                    eprintln!(" {} | {}", " ".repeat(line_num.to_string().len()), "~".repeat(err_line.len()));
                    if line_num < lines.len() { eprintln!(" {} | {}", line_num + 1, lines[line_num]); }
                    break;
                }
            }
            process::exit(1);
        }
    };

    let output_path = file_path.replace(".klc", ".exe");
    let mut gen = NativeCodeGenerator::new();
    gen.no_opt = no_opt;

    println!("Compiling {} → {}", file_path, output_path);
    match gen.compile(&program, &output_path) {
        Ok(()) => {
            let meta = fs::metadata(&output_path).unwrap();

            // 生成 DWARF 调试信息
            if gen_debug {
                let debug_source = DebugInfoSource {
                    file_path: file_path.to_string(),
                    unit_name: file_path.to_string(),
                    line_map: collect_line_map(&source),
                    variables: vec![],
                    functions: vec![],
                    code_base_rva: 0x1000,
                };
                let mut dwarf_gen = DwarfGenerator::new(debug_source);
                let sections = dwarf_gen.generate();
                println!("  DWARF sections: .debug_abbrev ({}B) .debug_info ({}B) .debug_line ({}B) .debug_str ({}B)",
                    sections.abbrev.len(), sections.info.len(), sections.line.len(), sections.str_section.len());
            }

            println!("  Generated: {} ({} bytes, {:.0} KB)",
                output_path, meta.len(), meta.len() as f64 / 1024.0);
            println!("  Run: .\\{}", output_path);
        }
        Err(e) => {
            eprintln!("Native compile error: {}", e);
            process::exit(1);
        }
    }
}

/// 从源码收集行号映射 (简化版: 每个非空行对应一个地址)
fn collect_line_map(source: &str) -> Vec<(u32, u32)> {
    let mut line_map = Vec::new();
    let mut line_num: u32 = 1;
    let mut addr_offset: u32 = 0;

    for line in source.lines() {
        let trimmed = line.trim();
        // 跳过空行和注释
        if !trimmed.is_empty() && !trimmed.starts_with("--") {
            line_map.push((line_num, 0x1000 + addr_offset));
            addr_offset += 0x10; // 估算每行 16 字节
        }
        line_num += 1;
    }
    line_map
}
