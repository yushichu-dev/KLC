//! KLC 原生代码生成器 — 模块入口
//!
//! 本模块将 KLC 的中间表示生成为 Windows x86_64 原生 PE 可执行文件。
//! 不依赖任何外部编译器（MSVC、LLVM 等）或链接器，直接从字节层面构造完整的 EXE 文件。
//!
//! ## 模块结构
//!
//! - `pe` — PE 格式定义与文件生成器 (阶段一)
//! - `x64` — x86_64 指令编码器 (阶段二)
//! - `regalloc` — 寄存器分配器 (阶段二)
//! - (后续阶段将添加: 导入表 等)
//!
//! ## 阶段规划
//!
//! 1. **最小 PE 文件生成** (已完成) — 构造结构正确的 PE 文件
//! 2. **x86_64 指令生成器** (已完成) — 实现机器码编码和寄存器分配
//! 3. **导入表实现** — 调用 Windows API（如 ExitProcess, WriteConsole 等）
//! 4. **编译器集成** — 将原生代码生成器接入 KLC 编译管道
//! 5. **优化与完善** — 更多语言特性、性能优化

pub mod pe;
pub mod x64;
pub mod regalloc;
pub mod imports;
pub mod optimize;

/// 将给定的 x86_64 机器码编译为独立的 Windows PE 可执行文件
///
/// 接收原始机器码字节，生成完整 EXE 文件。
///
/// # 参数
///
/// * `output_path` - 输出 EXE 文件的路径（如 `"minimal.exe"`）
/// * `code` - x86_64 机器码字节
///
/// # 返回值
///
/// 成功时返回 `Ok(())`，失败时返回 I/O 错误。
///
/// # 示例
///
/// ```ignore
/// use native::compile_to_exe;
///
/// // 生成一个立即退出的最小 EXE
/// let code = vec![0x31, 0xC0, 0xC3];  // xor eax, eax; ret
/// compile_to_exe("minimal.exe", &code).expect("生成 EXE 失败");
/// ```
pub fn compile_to_exe(output_path: &str, code: &[u8]) -> std::io::Result<()> {
    let mut builder = pe::PeBuilder::new();
    builder.add_code(code);
    builder.set_entry_point(pe::CODE_RVA);

    let exe_data = builder.build();
    std::fs::write(output_path, &exe_data)
}

/// 使用 x86_64 指令编码器编译一个简单函数
///
/// 便捷函数：创建 X64Assembler，调用闭包生成指令，返回最终机器码。
///
/// # 参数
///
/// * `f` - 接受 `&mut X64Assembler` 的闭包，在其中生成指令
///
/// # 示例
///
/// ```ignore
/// let code = native::compile_function(|asm| {
///     asm.mov_reg_imm(Register::RAX, 42);
///     asm.ret();
/// });
/// ```
#[allow(dead_code)]
pub fn compile_function<F>(f: F) -> Vec<u8>
where
    F: FnOnce(&mut x64::X64Assembler),
{
    let mut asm = x64::X64Assembler::new();
    f(&mut asm);
    asm.finish()
}
