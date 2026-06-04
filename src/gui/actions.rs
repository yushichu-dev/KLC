//! KLC IDE — 动作处理模块
//!
//! 实现菜单/按钮事件与 KLC 编译器、虚拟机、原生生成器的联动：
//! - 【运行】：源码 → Lexer → Parser → Codegen → VM → 输出捕获
//! - 【编译原生EXE】：源码 → Lexer → Parser → NativeCodegen → PE 输出
//! - 【检查语法】：源码 → Lexer → Parser → 错误报告
//! - 【格式化代码】：源码 → formatter → 编辑区更新
//! - 【打开文件】：Win32 文件选择对话框 → 加载到编辑区
//! - 【保存文件】：编辑区内容 → Win32 文件保存对话框 → 写入文件
//! - 【清空输出】：清空底部日志面板
//!
//! 所有编译错误通过 output 模块友好展示（中文提示 + 行号）。

#![allow(non_snake_case)]
#![allow(dead_code)]

use std::fs::File;
use std::io::{self, Read, Write};
use std::os::windows::io::FromRawHandle;

use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::codegen::Codegen;
use crate::vm::VM;
use crate::bytecode_optimize;



// ============================================================================
// Win32 类型定义（文件对话框需要）
// ============================================================================

type HANDLE = isize;
type HWND = HANDLE;
type WCHAR = u16;
type UINT = u32;
type DWORD = u32;
type BOOL = i32;
type LPARAM = isize;
type WPARAM = usize;

// ============================================================================
// Win32 常量（文件对话框）
// ============================================================================

const MAX_PATH: usize = 260;

/// OPENFILENAME 标志：必须指定
const OFN_HIDEREADONLY: DWORD = 0x00000004;
/// OPENFILENAME 标志：文件必须存在（打开时使用）
const OFN_FILEMUSTEXIST: DWORD = 0x00001000;
/// OPENFILENAME 标志：路径必须存在
const OFN_PATHMUSTEXIST: DWORD = 0x00000800;
/// OPENFILENAME 标志：覆盖确认（保存时使用）
const OFN_OVERWRITEPROMPT: DWORD = 0x00000002;
/// OPENFILENAME 标志：无改变目录
const OFN_NOCHANGEDIR: DWORD = 0x00000008;

// ============================================================================
// Win32 结构体（文件对话框）
// ============================================================================

/// Win32 OPENFILENAMEW 结构体 — 用于 GetOpenFileName / GetSaveFileName
#[repr(C)]
struct OPENFILENAMEW {
    lStructSize: DWORD,
    hwndOwner: HWND,
    hInstance: HANDLE,
    lpstrFilter: *const WCHAR,
    lpstrCustomFilter: *mut WCHAR,
    nMaxCustFilter: DWORD,
    nFilterIndex: DWORD,
    lpstrFile: *mut WCHAR,
    nMaxFile: DWORD,
    lpstrFileTitle: *mut WCHAR,
    nMaxFileTitle: DWORD,
    lpstrInitialDir: *const WCHAR,
    lpstrTitle: *const WCHAR,
    Flags: DWORD,
    nFileOffset: WORD,
    nFileExtension: WORD,
    lpstrDefExt: *const WCHAR,
    lCustData: LPARAM,
    lpfnHook: usize,
    lpTemplateName: *const WCHAR,
    // Windows 2000+ 扩展字段
    pvReserved: *mut std::ffi::c_void,
    dwReserved: DWORD,
    FlagsEx: DWORD,
}

type WORD = u16;

// ============================================================================
// Win32 API 声明（文件对话框专用）
// ============================================================================

#[link(name = "comdlg32")]
extern "system" {
    /// 显示「打开文件」系统对话框
    fn GetOpenFileNameW(lpofn: *mut OPENFILENAMEW) -> BOOL;
    /// 显示「保存文件」系统对话框
    fn GetSaveFileNameW(lpofn: *mut OPENFILENAMEW) -> BOOL;
}

// ============================================================================
// Win32 API 声明（主窗口句柄获取）
// ============================================================================

#[link(name = "user32")]
extern "system" {
    fn GetActiveWindow() -> HWND;
}

// ============================================================================
// 辅助函数
// ============================================================================

/// Rust &str → null 结尾 UTF-16（与 controls.rs 一致）
fn to_wide(s: &str) -> Vec<WCHAR> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

// ============================================================================
// 当前文件路径状态
// ============================================================================

/// 当前打开的文件路径（None 表示新文件/未保存）
/// 保存文件时用作默认路径，窗口标题显示用
static mut G_CURRENT_FILE: Option<String> = None;

/// 设置当前文件路径
pub unsafe fn set_current_file(path: Option<String>) {
    G_CURRENT_FILE = path;
}

/// 获取当前文件路径
pub unsafe fn get_current_file() -> Option<String> {
    let ptr = &raw const G_CURRENT_FILE;
    (*ptr).clone()
}

// ============================================================================
// 核心：源码 → 运行（VM 执行，输出重定向到输出面板）
// ============================================================================

/// 从编辑区读取源码，编译并在 VM 中执行。
///
/// 执行流程：
/// 1. 获取编辑区源码
/// 2. 词法分析 (Lexer)
/// 3. 语法分析 (Parser)
/// 4. 字节码生成 (Codegen)
/// 5. VM 执行（通过 stdout 重定向捕获 print/println 输出）
///
/// 所有阶段的错误信息输出到输出面板。
pub unsafe fn action_run() {
    use super::editor;
    use super::output;

    output::append_line("───────────────────────");
    output::log_ok("开始运行...");

    // ─── 步骤 1: 获取编辑区源码 ───
    let source = editor::get_source_code();
    if source.trim().is_empty() {
        output::log_warn("编辑区为空，无法运行");
        output::append_line("───────────────────────");
        return;
    }

    output::append_line(&format!("[信息] 源码大小: {} 字节", source.len()));

    // ─── 步骤 2: 词法分析 ───
    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize();
    output::append_line(&format!("[信息] 词法分析完成: {} 个 Token", tokens.len()));

    // ─── 步骤 3: 语法分析 ───
    let mut parser = Parser::new(tokens);
    let program = match parser.parse_program() {
        Ok(p) => p,
        Err(e) => {
            output::log_error("语法分析失败");
            output::append_line(&format!("  {}", e));
            // 尝试从错误信息中提取行号，展示上下文
            show_error_context(&source, &e);
            output::append_line("───────────────────────");
            return;
        }
    };
    output::append_line(&format!("[信息] 语法分析完成: {} 条语句", program.statements.len()));

    // ─── 步骤 3.5: AST 优化（常量折叠 + 死代码消除）───
    let mut program = program;
    bytecode_optimize::optimize_program(&mut program);
    output::append_line("[信息] AST 优化完成（常量折叠 + 死代码消除）");

    // ─── 步骤 4: 字节码生成 ───
    let bytecode = match Codegen::compile(&program) {
        Ok(bc) => bc,
        Err(e) => {
            output::log_error(&format!("字节码生成失败: {}", e));
            output::append_line("───────────────────────");
            return;
        }
    };
    output::log_ok(&format!("字节码生成完成: {} 条指令, {} 个常量",
        bytecode.main.len(), bytecode.constants.len()));

    // ─── 步骤 5: VM 执行（通过 VM 内置捕获机制获取 print 输出）───
    output::log_ok("程序输出:");

    // 使用 VM 的输出捕获缓冲区（GUI 程序中 stdout 不可用，管道重定向无效）
    crate::vm::start_output_capture();
    let run_result = {
        let mut vm = VM::new(bytecode);
        vm.run()
    };
    let captured = crate::vm::end_output_capture();

    // 检查运行错误
    if let Err(e) = run_result {
        output::log_error(&format!("运行时错误: {}", e));
    }

    // 将捕获的输出追加到面板
    if !captured.is_empty() {
        for line in captured.lines() {
            output::append_raw(line);
            output::append_raw("\r\n");
        }
    } else {
        output::append_line("（无输出）");
    }

    output::append_line("───────────────────────");
}

// ============================================================================
// 核心：源码 → 编译原生 EXE
// ============================================================================

/// 从编辑区读取源码，编译为 Windows PE 原生可执行文件。
///
/// 执行流程：
/// 1. 获取编辑区源码
/// 2. 词法分析 → 语法分析 → AST
/// 3. 调用 NativeCodeGenerator 生成 PE 文件
///
/// 输出文件路径：如果当前有打开文件，则保存为同名 .exe；
/// 否则使用 "output.exe" 作为默认名。
pub unsafe fn action_compile_native() {
    use super::editor;
    use super::output;
    use crate::native_codegen::NativeCodeGenerator;

    output::append_line("───────────────────────");
    output::log_ok("开始编译原生 EXE...");

    // ─── 步骤 1: 获取编辑区源码 ───
    let source = editor::get_source_code();
    if source.trim().is_empty() {
        output::log_warn("编辑区为空，无法编译");
        output::append_line("───────────────────────");
        return;
    }

    // ─── 步骤 2: 词法分析 ───
    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize();

    // ─── 步骤 3: 语法分析 ───
    let mut parser = Parser::new(tokens);
    let program = match parser.parse_program() {
        Ok(p) => p,
        Err(e) => {
            output::log_error("语法分析失败");
            output::append_line(&format!("  {}", e));
            show_error_context(&source, &e);
            output::append_line("───────────────────────");
            return;
        }
    };

    // ─── 步骤 3.5: AST 优化（常量折叠 + 死代码消除）───
    let mut program = program;
    bytecode_optimize::optimize_program(&mut program);
    output::append_line("[信息] AST 优化完成");

    // ─── 步骤 4: 确定输出路径 ───
    let output_path = if let Some(ref file) = G_CURRENT_FILE {
        // 如果有当前文件路径，替换扩展名为 .exe
        file.replace(".klc", ".exe")
    } else {
        "output.exe".to_string()
    };

    output::append_line(&format!("[信息] 输出路径: {}", output_path));

    // ─── 步骤 5: 原生编译 ───
    let gen = NativeCodeGenerator::new();
    match gen.compile(&program, &output_path) {
        Ok(()) => {
            match std::fs::metadata(&output_path) {
                Ok(meta) => {
                    output::log_ok(&format!("编译成功!"));
                    output::append_line(&format!("  文件: {}", output_path));
                    output::append_line(&format!("  大小: {} 字节 ({:.0} KB)",
                        meta.len(), meta.len() as f64 / 1024.0));
                }
                Err(e) => {
                    output::log_error(&format!("编译完成但无法读取输出文件: {}", e));
                }
            }
        }
        Err(e) => {
            output::log_error(&format!("编译失败: {}", e));
        }
    }

    output::append_line("───────────────────────");
}

// ============================================================================
// 语法检查
// ============================================================================

/// 从编辑区读取源码，进行词法+语法分析检查。
/// 仅检查不执行，将结果输出到输出面板。
pub unsafe fn action_check_syntax() {
    use super::editor;
    use super::output;

    output::append_line("───────────────────────");
    output::log_ok("检查语法...");

    let source = editor::get_source_code();
    if source.trim().is_empty() {
        output::log_warn("编辑区为空");
        output::append_line("───────────────────────");
        return;
    }

    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize();
    output::append_line(&format!("[信息] 词法分析完成: {} 个 Token", tokens.len()));

    let mut parser = Parser::new(tokens);
    match parser.parse_program() {
        Ok(program) => {
            output::log_ok(&format!("语法检查通过! ({} 条语句)", program.statements.len()));
        }
        Err(e) => {
            output::log_error("语法错误");
            output::append_line(&format!("  {}", e));
            show_error_context(&source, &e);
        }
    }

    output::append_line("───────────────────────");
}

// ============================================================================
// 编译并运行（VM 字节码执行）
// ============================================================================

/// 从编辑区读取源码，编译字节码并在 VM 中执行。
/// 与 action_run 相同（当前 VM 就是字节码执行路径），
/// 预留给未来可能的「编译为字节码文件 + 执行」扩展。
pub unsafe fn action_build_and_run() {
    // 当前实现与 action_run 相同
    action_run();
}

// ============================================================================
// 格式化代码
// ============================================================================

/// 格式化编辑区中的代码并更新编辑区内容。
pub unsafe fn action_format() {
    use super::editor;
    use super::output;

    output::append_line("───────────────────────");
    output::log_ok("格式化代码...");

    let source = editor::get_source_code();
    if source.trim().is_empty() {
        output::log_warn("编辑区为空");
        output::append_line("───────────────────────");
        return;
    }

    // 使用 formatter 模块格式化
    match crate::formatter::format_source(&source, &crate::formatter::FormatConfig::default()) {
        Ok(formatted) => {
            editor::set_source_code(&formatted);
            output::log_ok("格式化完成");
        }
        Err(e) => {
            output::log_error(&format!("格式化失败: {}", e));
        }
    }

    output::append_line("───────────────────────");
}

// ============================================================================
// 清空输出面板
// ============================================================================

/// 清空底部输出/日志面板的全部内容
pub unsafe fn action_clear_output() {
    super::output::clear_output();
}

// ============================================================================
// 文件操作：打开文件
// ============================================================================

/// 弹出系统「打开文件」对话框，加载 .klc 源码到编辑区。
///
/// 使用 Win32 API GetOpenFileNameW 实现标准文件选择框。
/// 过滤器只显示 .klc 文件和所有文件。
pub unsafe fn action_open_file() {
    use super::editor;
    use super::output;

    let hwnd = GetActiveWindow();

    // 准备文件名缓冲区（预填空字符串，Windows 会写入选中的路径）
    let mut file_buf: Vec<WCHAR> = vec![0; MAX_PATH];

    // 准备文件过滤器：
    // "KLC 源文件 (*.klc)\0*.klc\0所有文件 (*.*)\0*.*\0\0"
    let filter_str: Vec<WCHAR> = "KLC 源文件 (*.klc)\0*.klc\0所有文件 (*.*)\0*.*\0\0"
        .encode_utf16().collect();

    let title_str = to_wide("打开 KLC 源文件");
    let def_ext = to_wide("klc");

    let mut ofn: OPENFILENAMEW = std::mem::zeroed();
    ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as DWORD;
    ofn.hwndOwner = hwnd;
    ofn.lpstrFilter = filter_str.as_ptr();
    ofn.nFilterIndex = 1;                               // 默认选中第一个过滤器
    ofn.lpstrFile = file_buf.as_mut_ptr();
    ofn.nMaxFile = MAX_PATH as DWORD;
    ofn.lpstrTitle = title_str.as_ptr();
    ofn.lpstrDefExt = def_ext.as_ptr();
    ofn.Flags = OFN_HIDEREADONLY | OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR;

    if GetOpenFileNameW(&mut ofn) != 0 {
        // 用户选中了文件 — 从缓冲区提取路径
        let actual_len = file_buf.iter().position(|&c| c == 0).unwrap_or(file_buf.len());
        let file_path = String::from_utf16_lossy(&file_buf[..actual_len]);

        // 读取文件内容
        match std::fs::read_to_string(&file_path) {
            Ok(content) => {
                editor::set_source_code(&content);
                editor::set_modified(false);
                G_CURRENT_FILE = Some(file_path.clone());

                let lines = content.lines().count();
                output::append_line(&format!("[文件] 已打开: {}", file_path));
                output::append_line(&format!("[文件] {} 行, {} 字节", lines, content.len()));
            }
            Err(e) => {
                output::log_error(&format!("读取文件失败: {}", e));
            }
        }
    } else {
        // 用户取消了对话框（正常行为，不需要输出提示）
    }
}

// ============================================================================
// 文件操作：保存文件
// ============================================================================

/// 弹出系统「保存文件」对话框，将编辑区代码保存为 .klc 文件。
///
/// 如果当前已有文件路径，直接覆盖保存（不弹对话框）。
/// 否则弹出保存对话框让用户选择路径。
pub unsafe fn action_save_file() {
    use super::editor;
    use super::output;

    let source = editor::get_source_code();
    if source.is_empty() {
        output::log_warn("编辑区为空，无需保存");
        return;
    }

    let file_path = if let Some(ref path) = G_CURRENT_FILE {
        // 已有路径 — 直接保存（但仍弹出确认对话框，防止意外覆盖）
        path.clone()
    } else {
        // 新文件 — 弹出保存对话框
        let hwnd = GetActiveWindow();

        let mut file_buf: Vec<WCHAR> = vec![0; MAX_PATH];
        // 默认文件名
        let default_name: Vec<WCHAR> = "untitled.klc\0".encode_utf16().collect();
        file_buf[..default_name.len()].copy_from_slice(&default_name);

        let filter_str: Vec<WCHAR> = "KLC 源文件 (*.klc)\0*.klc\0所有文件 (*.*)\0*.*\0\0"
            .encode_utf16().collect();

        let title_str = to_wide("保存 KLC 源文件");
        let def_ext = to_wide("klc");

        let mut ofn: OPENFILENAMEW = std::mem::zeroed();
        ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as DWORD;
        ofn.hwndOwner = hwnd;
        ofn.lpstrFilter = filter_str.as_ptr();
        ofn.nFilterIndex = 1;
        ofn.lpstrFile = file_buf.as_mut_ptr();
        ofn.nMaxFile = MAX_PATH as DWORD;
        ofn.lpstrTitle = title_str.as_ptr();
        ofn.lpstrDefExt = def_ext.as_ptr();
        ofn.Flags = OFN_HIDEREADONLY | OFN_OVERWRITEPROMPT | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR;

        if GetSaveFileNameW(&mut ofn) != 0 {
            let actual_len = file_buf.iter().position(|&c| c == 0).unwrap_or(file_buf.len());
            String::from_utf16_lossy(&file_buf[..actual_len])
        } else {
            return; // 用户取消了保存对话框
        }
    };

    // 写入文件
    match std::fs::write(&file_path, &source) {
        Ok(()) => {
            editor::set_modified(false);
            G_CURRENT_FILE = Some(file_path.clone());
            output::append_line(&format!("[文件] 已保存: {} ({} 字节)", file_path, source.len()));
        }
        Err(e) => {
            output::log_error(&format!("保存失败: {}", e));
        }
    }
}

// ============================================================================
// 新建文件
// ============================================================================

/// 新建文件：清空编辑区，重置当前文件路径
pub unsafe fn action_new_file() {
    use super::editor;
    use super::output;

    editor::set_source_code("");
    editor::set_modified(false);
    G_CURRENT_FILE = None;
    output::append_line("[文件] 新建文件");
}

// ============================================================================
// 错误上下文展示
// ============================================================================

/// 从错误信息中尝试提取行号，并展示出错行及其上下文
///
/// KLC 的 parser 错误格式通常包含 "at line N" 或 "at line N col M"，
/// 我们解析出错误行号后展示该行代码和上下文行。
fn show_error_context(source: &str, error_msg: &str) {
    // 调用 unsafe 的 output 函数
    // SAFETY: 仅在 GUI 线程中调用，output 面板句柄在窗口创建时已初始化
    unsafe { show_error_context_impl(source, error_msg); }
}

unsafe fn show_error_context_impl(source: &str, error_msg: &str) {
    use super::output;

    // 尝试从错误信息中提取行号
    // 常见格式: "... at line 5 col 10" 或 "... at line 5"
    let line_num = extract_line_number(error_msg);

    if let Some(num) = line_num {
        let lines: Vec<&str> = source.lines().collect();
        output::append_line("");
        output::append_line(&format!("  ┌─ 第 {} 行", num));

        // 展示前一行（上下文）
        if num > 1 && num - 2 < lines.len() {
            output::append_line(&format!("  │ {} | {}", num - 1, lines[num - 2]));
        }

        // 展示出错行
        if num >= 1 && num - 1 < lines.len() {
            let err_line = lines[num - 1];
            output::append_line(&format!("  ▶ {} | {}", num, err_line));

            // 尝试提取列号来放置 ~ 指示符
            let col_num = extract_col_number(error_msg);
            if let Some(col) = col_num {
                let prefix = format!("  │     ");
                let padding = " ".repeat((col as usize).saturating_sub(1));
                output::append_line(&format!("{}{}^", prefix, padding));
            } else {
                // 没有列号，用 ~ 标记整行
                let line_prefix_len = format!("  ▶ {} | ", num).len();
                output::append_line(&format!("{}{}", " ".repeat(line_prefix_len),
                    "~".repeat(lines[num - 1].len().min(60))));
            }
        }

        // 展示后一行（上下文）
        if num < lines.len() {
            output::append_line(&format!("  │ {} | {}", num + 1, lines[num]));
        }

        output::append_line("  └─────────────────────");
    }
}

/// 从错误信息字符串中提取行号
///
/// 匹配模式: "at line N" 或 "line N" (N 为数字)
fn extract_line_number(error_msg: &str) -> Option<usize> {
    // 查找 "line X" 模式
    let lower = error_msg.to_lowercase();
    if let Some(pos) = lower.find("line ") {
        let after = &error_msg[pos + 5..];
        // 提取行号（数字序列）
        let num_str: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(num) = num_str.parse::<usize>() {
            if num > 0 {
                return Some(num);
            }
        }
    }
    None
}

/// 从错误信息字符串中提取列号
///
/// 匹配模式: "col N" (N 为数字)
fn extract_col_number(error_msg: &str) -> Option<usize> {
    let lower = error_msg.to_lowercase();
    if let Some(pos) = lower.find("col ") {
        let after = &error_msg[pos + 4..];
        let num_str: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(num) = num_str.parse::<usize>() {
            if num > 0 {
                return Some(num);
            }
        }
    }
    None
}

// ============================================================================
// stdout 重定向（捕获 VM 的 print 输出到字符串）
// ============================================================================

// Win32 管道 API 声明
#[link(name = "kernel32")]
extern "system" {
    /// 创建匿名管道
    fn CreatePipe(
        hReadPipe: *mut HANDLE,
        hWritePipe: *mut HANDLE,
        lpPipeAttributes: *mut std::ffi::c_void,
        nSize: DWORD,
    ) -> BOOL;
    /// 设置进程标准句柄
    fn SetStdHandle(nStdHandle: DWORD, hHandle: HANDLE) -> BOOL;
    /// 获取进程标准句柄
    fn GetStdHandle(nStdHandle: DWORD) -> HANDLE;
    /// 以指定方式打开/创建文件
    fn CreateFileW(
        lpFileName: *const WCHAR,
        dwDesiredAccess: DWORD,
        dwShareMode: DWORD,
        lpSecurityAttributes: *const std::ffi::c_void,
        dwCreationDisposition: DWORD,
        dwFlagsAndAttributes: DWORD,
        hTemplateFile: HANDLE,
    ) -> HANDLE;
}

/// STD_OUTPUT_HANDLE 常量
const STD_OUTPUT_HANDLE: DWORD = 0xFFFFFFF5u32; // DWORD(-11)
/// GENERIC_WRITE 访问权限
const GENERIC_WRITE: DWORD = 0x40000000;
/// FILE_SHARE_WRITE 共享模式
const FILE_SHARE_WRITE: DWORD = 0x00000002;
/// OPEN_EXISTING 创建方式
const OPEN_EXISTING: DWORD = 3;

/// 捕获闭包执行期间所有 stdout 输出
///
/// 通过创建管道替换 stdout，将闭包中的 print!/println!
/// 输出重定向到内存缓冲区。执行完成后恢复原始 stdout。
///
/// 这使得 GUI 中 VM 执行的 print 输出能被捕获并显示在输出面板中。
fn capture_stdout<F: FnOnce() -> R, R>(f: F) -> String {
    unsafe {
        // ─── 步骤 1: 创建匿名管道 ───
        let mut read_h: HANDLE = 0;
        let mut write_h: HANDLE = 0;

        if CreatePipe(&mut read_h, &mut write_h, std::ptr::null_mut(), 0) == 0 {
            // 管道创建失败，回退到不捕获模式
            let _ = f();
            return String::new();
        }

        // ─── 步骤 2: 保存原始 stdout 句柄 ───
        let original_stdout_handle = GetStdHandle(STD_OUTPUT_HANDLE);

        // ─── 步骤 3: 将 stdout 替换为管道写入端 ───
        SetStdHandle(STD_OUTPUT_HANDLE, write_h);

        // ─── 步骤 4: 执行闭包（VM 的 print 输出会进入管道）───
        let result = f();

        // ─── 步骤 5: 关闭管道写入端（使 read 端能读到 EOF）───
        // 先 flush stdout 以确保所有缓冲数据写入管道
        let _ = io::stdout().flush();
        extern "system" {
            fn CloseHandle(hHandle: HANDLE) -> BOOL;
        }
        CloseHandle(write_h);

        // ─── 步骤 6: 从管道读取端读取所有捕获的输出 ───
        let mut read_file = File::from_raw_handle(read_h as std::os::windows::io::RawHandle);
        let mut buf = String::new();
        let _ = read_file.read_to_string(&mut buf);

        // ─── 步骤 7: 恢复原始 stdout ───
        // 重新打开 CONOUT$ 作为控制台输出
        let conout = to_wide("CONOUT$\0");
        let con_handle = CreateFileW(
            conout.as_ptr(),
            GENERIC_WRITE,
            FILE_SHARE_WRITE,
            std::ptr::null(),
            OPEN_EXISTING,
            0,
            0,
        );
        if con_handle != !0isize as HANDLE { // INVALID_HANDLE_VALUE
            SetStdHandle(STD_OUTPUT_HANDLE, con_handle);
        } else {
            // CONOUT$ 打开失败，恢复原始句柄
            SetStdHandle(STD_OUTPUT_HANDLE, original_stdout_handle);
        }

        // result 被忽略（R 仅用于返回控制流）
        std::mem::forget(result);

        buf
    }
}
