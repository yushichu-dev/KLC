//! KLC 标准 IO 库 — 控制台 IO / 文件 IO / 格式化 IO / 工具函数
//!
//! ## 功能清单
//!
//! ### 一、基础控制台 IO
//! | 函数 | 说明 | 用法 |
//! |------|------|------|
//! | `print(...)` | 打印不换行 | `print("Hello")` |
//! | `println(...)` | 打印换行 | `println("Hello")` |
//! | `input()` | 阻塞读取一行字符串 | `let s = input()` |
//! | `input(prompt)` | 显示提示后读取 | `let s = input("Name: ")` |
//! | `input_num()` | 读取数字输入(带类型校验) | `let n = input_num()` |
//! | `input_num(prompt)` | 提示后读取数字 | `let n = input_num("Age: ")` |
//! | `eprint(...)` | 标准错误输出(不换行) | `eprint("error!")` |
//! | `eprintln(...)` | 标准错误输出(换行) | `eprintln("error!")` |
//!
//! ### 二、文件 IO
//! | 函数 | 说明 | 用法 |
//! |------|------|------|
//! | `file_read(path)` | 读取文本文件全部内容 | `let s = file_read("a.txt")` |
//! | `file_write(path, content)` | 覆盖写入 | `file_write("a.txt", "hi")` |
//! | `file_append(path, content)` | 追加写入 | `file_append("a.txt", "more")` |
//! | `file_exists(path)` | 判断文件是否存在 | `if file_exists("a.txt") { ... }` |
//! | `file_delete(path)` | 删除文件 | `file_delete("a.txt")` |
//!
//! ### 三、格式化 IO
//! | 函数 | 说明 | 用法 |
//! |------|------|------|
//! | `fmt_printf(fmt, ...)` | 格式化占位符输出 | `fmt_printf("{} + {} = {}", 1, 2, 3)` |
//! | `print_table(headers, rows)` | 控制台表格打印 | `print_table(["Name","Age"], rows)` |
//! | `print_debug(val)` | 调试打印(类型+值) | `print_debug(x)` |
//!
//! ### 四、工具函数
//! | 函数 | 说明 | 用法 |
//! |------|------|------|
//! | `flush()` | 刷新输出缓冲区 | `flush()` |
//! | `stdin_is_empty()` | 标准输入是否为空 | `if stdin_is_empty() { ... }` |

use std::io::{self, Write};

// ============================================================================
// 一、基础控制台 IO — Rust 实现
// ============================================================================

/// print() — 打印文本/数字/变量，不换行
///
/// ## KLC 用法
/// ```klc
/// print("Hello ");    // 字符串
/// print(42);           // 数字
/// print(true);         // 布尔值
/// ```
pub fn print_value(val: &str) {
    print!("{}", val);
    // IDE GUI 输出捕获
    crate::vm::capture_output(val);
}

/// println() — 打印并自动换行
///
/// ## KLC 用法
/// ```klc
/// println("Hello, KLC!");   // 换行输出
/// println(3.14);             // 浮点数
/// ```
pub fn println_value(val: &str) {
    let output = format!("{}\n", val);
    print!("{}", &output);
    crate::vm::capture_output(&output);
}

/// input() — 阻塞获取控制台用户输入，返回字符串
///
/// 可选参数 `prompt`: 显示在输入前的提示文字
///
/// ## KLC 用法
/// ```klc
/// let name = input("Name: ");   // 显示 "Name: " 后等待输入
/// let line = input();            // 不显示提示，等待输入
/// ```
pub fn input(prompt: Option<&str>) -> String {
    if let Some(p) = prompt {
        print!("{}", p);
        io::stdout().flush().ok();
    }
    let mut buf = String::new();
    match io::stdin().read_line(&mut buf) {
        Ok(_) => buf.trim_end_matches('\n').trim_end_matches('\r').to_string(),
        Err(_) => String::new(),
    }
}

/// input_num() — 读取数字输入，自带类型校验
///
/// 非数字输入返回 0，并打印错误提示到 stderr。
///
/// ## KLC 用法
/// ```klc
/// let age = input_num("Age: ");  // 输入数字
/// let n = input_num();            // 不带提示
/// ```
pub fn input_num(prompt: Option<&str>) -> f64 {
    if let Some(p) = prompt {
        print!("{}", p);
        io::stdout().flush().ok();
    }
    let mut buf = String::new();
    match io::stdin().read_line(&mut buf) {
        Ok(_) => {
            let trimmed = buf.trim();
            match trimmed.parse::<f64>() {
                Ok(n) => n,
                Err(_) => {
                    eprintln!("[KLC] 输入不是有效数字: '{}', 默认返回 0", trimmed);
                    0.0
                }
            }
        }
        Err(_) => 0.0,
    }
}

/// eprint() — 标准错误输出，不换行
///
/// ## KLC 用法
/// ```klc
/// eprint("Warning: file not found");
/// ```
pub fn eprint_value(val: &str) {
    eprint!("{}", val);
    io::stderr().flush().ok();
}

/// eprintln() — 标准错误输出，自动换行
///
/// ## KLC 用法
/// ```klc
/// eprintln("Error: division by zero");
/// ```
pub fn eprintln_value(val: &str) {
    eprintln!("{}", val);
}

// ============================================================================
// 二、文件 IO — Rust 实现
// ============================================================================

/// file_read() — 读取文本文件全部内容
///
/// 文件不存在时返回空字符串。
///
/// ## KLC 用法
/// ```klc
/// let content = file_read("input.txt");
/// println(content);
/// ```
pub fn file_read(path: &str) -> Result<String, String> {
    std::fs::read_to_string(path)
        .map_err(|e| format!("读取文件 '{}' 失败: {}", path, e))
}

/// file_write() — 覆盖写入文本文件
///
/// 文件不存在时自动创建，存在时覆盖。
///
/// ## KLC 用法
/// ```klc
/// file_write("output.txt", "Hello, World!");
/// ```
pub fn file_write(path: &str, content: &str) -> Result<(), String> {
    std::fs::write(path, content)
        .map_err(|e| format!("写入文件 '{}' 失败: {}", path, e))
}

/// file_append() — 追加写入文本文件
///
/// 文件不存在时自动创建。
///
/// ## KLC 用法
/// ```klc
/// file_append("log.txt", "New log entry\n");
/// ```
pub fn file_append(path: &str, content: &str) -> Result<(), String> {
    use std::fs::OpenOptions;
    use std::io::Write as IoWrite;
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .map_err(|e| format!("打开文件 '{}' 失败: {}", path, e))?;
    file.write_all(content.as_bytes())
        .map_err(|e| format!("追加写入 '{}' 失败: {}", path, e))
}

/// file_exists() — 判断文件是否存在
///
/// ## KLC 用法
/// ```klc
/// if file_exists("config.json") {
///     let data = file_read("config.json");
/// }
/// ```
pub fn file_exists(path: &str) -> bool {
    std::path::Path::new(path).exists()
}

/// file_delete() — 删除文件
///
/// ## KLC 用法
/// ```klc
/// file_delete("temp.txt");
/// ```
pub fn file_delete(path: &str) -> Result<(), String> {
    std::fs::remove_file(path)
        .map_err(|e| format!("删除文件 '{}' 失败: {}", path, e))
}

// ============================================================================
// 三、格式化 IO — Rust 实现
// ============================================================================

/// fmt_printf() — 格式化占位符输出
///
/// 支持 `{}` 占位符，参数按顺序替换。
/// 不支持宽度/精度等高级格式。
///
/// ## KLC 用法
/// ```klc
/// fmt_printf("Hello, {}! You are {} years old.", name, age);
/// // 输出: Hello, Alice! You are 25 years old.
/// ```
pub fn fmt_printf(fmt_str: &str, args: &[String]) -> String {
    let mut result = String::new();
    let mut arg_idx = 0;
    let chars: Vec<char> = fmt_str.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        // 转义: {{ → {, }} → }
        if i + 1 < chars.len() && chars[i] == '{' && chars[i + 1] == '{' {
            result.push('{');
            i += 2;
        } else if i + 1 < chars.len() && chars[i] == '}' && chars[i + 1] == '}' {
            result.push('}');
            i += 2;
        } else if i + 1 < chars.len() && chars[i] == '{' && chars[i + 1] == '}' {
            if let Some(arg) = args.get(arg_idx) {
                result.push_str(arg);
            } else {
                result.push_str("{}");
            }
            arg_idx += 1;
            i += 2;
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }
    result
}

/// print_table() — 控制台表格打印
///
/// 延续 v1.3.2 KLC 项目风格，使用 Unicode 制表符。
///
/// ## KLC 用法
/// ```klc
/// let headers = ["Name", "Age", "City"];
/// let rows = [
///     ["Alice", "25", "Beijing"],
///     ["Bob", "30", "Shanghai"],
/// ];
/// print_table(headers, rows);
/// // ┌───────┬─────┬──────────┐
/// // │ Name  │ Age │ City     │
/// // ├───────┼─────┼──────────┤
/// // │ Alice │ 25  │ Beijing  │
/// // │ Bob   │ 30  │ Shanghai │
/// // └───────┴─────┴──────────┘
/// ```
pub fn print_table(headers: &[String], rows: &[Vec<String>]) -> String {
    if headers.is_empty() {
        return String::new();
    }

    // 计算每列最大宽度
    let mut col_widths: Vec<usize> = headers.iter().map(|h| h.chars().count()).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < col_widths.len() {
                let w = cell.chars().count();
                if w > col_widths[i] {
                    col_widths[i] = w;
                }
            }
        }
    }

    // 每列最小宽度 3
    for w in &mut col_widths {
        if *w < 3 { *w = 3; }
    }

    let mut output = String::new();
    let _total_width: usize = col_widths.iter().map(|w| w + 3).sum::<usize>() + 1;

    // 顶框
    output.push('┌');
    for (i, w) in col_widths.iter().enumerate() {
        output.push_str(&"─".repeat(w + 2));
        if i < col_widths.len() - 1 { output.push('┬'); }
    }
    output.push_str("┐\n");

    // 表头
    output.push('│');
    for (i, h) in headers.iter().enumerate() {
        let hw = h.chars().count();
        let pad = col_widths[i].saturating_sub(hw);
        output.push(' ');
        output.push_str(h);
        output.push_str(&" ".repeat(pad));
        output.push_str(" │");
    }
    output.push('\n');

    // 分隔线
    output.push('├');
    for (i, w) in col_widths.iter().enumerate() {
        output.push_str(&"─".repeat(w + 2));
        if i < col_widths.len() - 1 { output.push('┼'); }
    }
    output.push_str("┤\n");

    // 数据行
    for row in rows {
        output.push('│');
        for (i, cell) in row.iter().enumerate() {
            if i < col_widths.len() {
                let cw = cell.chars().count();
                let pad = col_widths[i].saturating_sub(cw);
                output.push(' ');
                output.push_str(cell);
                output.push_str(&" ".repeat(pad));
                output.push_str(" │");
            }
        }
        output.push('\n');
    }

    // 底框
    output.push('└');
    for (i, w) in col_widths.iter().enumerate() {
        output.push_str(&"─".repeat(w + 2));
        if i < col_widths.len() - 1 { output.push('┴'); }
    }
    output.push_str("┘\n");

    output
}

/// print_debug() — 调试打印变量的类型 + 值
///
/// ## KLC 用法
/// ```klc
/// let x = 42;
/// print_debug(x);
/// // 输出: [DEBUG] i64 = 42
///
/// let s = "hello";
/// print_debug(s);
/// // 输出: [DEBUG] String = "hello"
/// ```
pub fn debug_format(type_name: &str, value_str: &str) -> String {
    format!("[DEBUG] {} = {}", type_name, value_str)
}

// ============================================================================
// 四、工具函数 — Rust 实现
// ============================================================================

/// flush() — 刷新标准输出缓冲区
///
/// ## KLC 用法
/// ```klc
/// print("Loading...");
/// flush();  // 确保立即显示
/// ```
pub fn flush_stdout() {
    io::stdout().flush().ok();
}

/// stdin_is_empty() — 检查标准输入是否为空（无待读取数据）
///
/// 在 Windows 上通过检测控制台输入缓冲区实现，
/// Unix 上通过非阻塞读取检测。
///
/// ## KLC 用法
/// ```klc
/// if stdin_is_empty() {
///     println("No input available");
/// }
/// ```
pub fn stdin_is_empty() -> bool {
    #[cfg(windows)]
    {
        // Windows: FFI 调用 GetNumberOfConsoleInputEvents 检测控制台输入
        // 使用 isize (Windows HANDLE) 与 gui/actions.rs 声明保持一致
        extern "system" {
            fn GetStdHandle(nStdHandle: u32) -> isize;
            fn GetNumberOfConsoleInputEvents(
                hConsoleInput: isize,
                lpNumberOfEvents: *mut u32,
            ) -> i32;
        }
        const STD_INPUT_HANDLE: u32 = 0xFFFFFFF6;
        unsafe {
            let handle = GetStdHandle(STD_INPUT_HANDLE);
            let mut events: u32 = 0;
            let ret = GetNumberOfConsoleInputEvents(handle, &mut events);
            if ret == 0 { return false; } // 非控制台 stdin，保守返回 false
            events == 0
        }
    }
    #[cfg(not(windows))]
    {
        // Unix: 使用非阻塞 read 检测
        use std::os::unix::io::AsRawFd;
        let fd = io::stdin().as_raw_fd();
        let mut fds = libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        };
        unsafe { libc::poll(&mut fds, 1, 0) == 0 }
    }
}

// ============================================================================
// 五、VM 内置函数注册 — 字符串匹配分发
// ============================================================================

use std::rc::Rc;
use std::cell::RefCell;
use crate::bytecode::Value as BValue;

/// IO 函数名常量（用于 VM 中的字符串匹配）
pub mod func_names {
    // —— 旧版兼容 (io:: 前缀) ——
    pub const IO_READ: &str = "io::read";
    pub const IO_READ_LINES: &str = "io::read_lines";
    pub const IO_WRITE: &str = "io::write";
    pub const IO_APPEND: &str = "io::append";
    pub const IO_EXISTS: &str = "io::exists";
    pub const IO_DELETE: &str = "io::delete";
    pub const IO_MKDIR: &str = "io::mkdir";
    pub const IO_LIST_DIR: &str = "io::list_dir";
    pub const IO_FILE_SIZE: &str = "io::file_size";

    // —— 新版 IO 函数 (无前缀, 直接调用) ——
    pub const PRINT: &str = "print";
    pub const PRINTLN: &str = "println";
    pub const INPUT: &str = "input";
    pub const INPUT_NUM: &str = "input_num";
    pub const EPRINT: &str = "eprint";
    pub const EPRINTLN: &str = "eprintln";
    pub const FILE_READ: &str = "file_read";
    pub const FILE_WRITE: &str = "file_write";
    pub const FILE_APPEND: &str = "file_append";
    pub const FILE_EXISTS: &str = "file_exists";
    pub const FILE_DELETE: &str = "file_delete";
    pub const FMT_PRINTF: &str = "fmt_printf";
    pub const PRINT_TABLE: &str = "print_table";
    pub const PRINT_DEBUG: &str = "print_debug";
    pub const FLUSH: &str = "flush";
    pub const STDIN_IS_EMPTY: &str = "stdin_is_empty";
}

/// VM 分发: 处理 IO 标准库函数调用
///
/// 在 VM 的 `handle_call` 中调用此函数进行 IO 函数分发。
/// 返回 `Some(value)` 表示已处理并产生了返回值，
/// 返回 `None` 表示函数名不匹配，VM 应继续查找。
pub fn dispatch_io_func(
    name: &str,
    args: &[BValue],
) -> Option<BValue> {
    match name {
        // —— 新版 IO 函数 ——
        func_names::INPUT => {
            let prompt = args.first().and_then(|a| a.as_str()).map(|s| s.to_string());
            let result = input(prompt.as_deref());
            Some(BValue::String(result))
        }
        func_names::INPUT_NUM => {
            let prompt = args.first().and_then(|a| a.as_str()).map(|s| s.to_string());
            let result = input_num(prompt.as_deref());
            Some(BValue::Float(result))
        }
        func_names::EPRINT => {
            for arg in args {
                eprint!("{}", arg.to_string());
            }
            if !args.is_empty() { io::stderr().flush().ok(); }
            Some(BValue::Null)
        }
        func_names::EPRINTLN => {
            for arg in args {
                eprintln!("{}", arg.to_string());
            }
            Some(BValue::Null)
        }
        func_names::FILE_READ => {
            let path = args.first().and_then(|a| a.as_str()).unwrap_or("");
            match file_read(path) {
                Ok(content) => Some(BValue::String(content)),
                Err(e) => {
                    eprintln!("[KLC IO Error] {}", e);
                    Some(BValue::String(String::new()))
                }
            }
        }
        func_names::FILE_WRITE => {
            let path = args.first().and_then(|a| a.as_str()).unwrap_or("");
            let content = args.get(1).map(|a| a.to_string()).unwrap_or_default();
            let result = file_write(path, &content);
            if let Err(e) = &result {
                eprintln!("[KLC IO Error] file_write 失败: {}", e);
            }
            Some(BValue::Bool(result.is_ok()))
        }
        func_names::FILE_APPEND => {
            let path = args.first().and_then(|a| a.as_str()).unwrap_or("");
            let content = args.get(1).map(|a| a.to_string()).unwrap_or_default();
            let result = file_append(path, &content);
            if let Err(e) = &result {
                eprintln!("[KLC IO Error] file_append 失败: {}", e);
            }
            Some(BValue::Bool(result.is_ok()))
        }
        func_names::FILE_EXISTS => {
            let path = args.first().and_then(|a| a.as_str()).unwrap_or("");
            Some(BValue::Bool(file_exists(path)))
        }
        func_names::FILE_DELETE => {
            let path = args.first().and_then(|a| a.as_str()).unwrap_or("");
            let result = file_delete(path);
            if let Err(e) = &result {
                eprintln!("[KLC IO Error] file_delete 失败: {}", e);
            }
            Some(BValue::Bool(result.is_ok()))
        }
        func_names::FMT_PRINTF => {
            let fmt_str = args.first().and_then(|a| a.as_str()).unwrap_or("");
            let fmt_args: Vec<String> = args.iter().skip(1)
                .map(|a| a.to_string())
                .collect();
            let result = fmt_printf(fmt_str, &fmt_args);
            print!("{}", result);
            Some(BValue::String(result))
        }
        func_names::PRINT_TABLE => {
            // print_table(headers: Array, rows: Array<Array>)
            let headers: Vec<String> = args.first()
                .and_then(|a| a.as_str_array())
                .unwrap_or_default();
            let rows: Vec<Vec<String>> = if args.len() >= 2 {
                extract_table_rows(&args[1])
            } else {
                Vec::new()
            };
            let table = print_table(&headers, &rows);
            print!("{}", table);
            Some(BValue::String(table))
        }
        func_names::PRINT_DEBUG => {
            let val = args.first().unwrap_or(&BValue::Null);
            let type_name = type_name_of(val);
            let value_str = val.to_string();
            let debug_str = debug_format(type_name, &value_str);
            println!("{}", debug_str);
            Some(BValue::String(debug_str))
        }
        func_names::FLUSH => {
            flush_stdout();
            Some(BValue::Null)
        }
        func_names::STDIN_IS_EMPTY => {
            Some(BValue::Bool(stdin_is_empty()))
        }

        // —— 旧版 io:: 函数兼容 ——
        func_names::IO_READ => {
            let path = args.first().and_then(|a| a.as_str()).unwrap_or("");
            match file_read(path) {
                Ok(content) => Some(BValue::String(content)),
                Err(_) => Some(BValue::String(String::new())),
            }
        }
        func_names::IO_READ_LINES => {
            let path = args.first().and_then(|a| a.as_str()).unwrap_or("");
            match file_read(path) {
                Ok(content) => {
                    let lines: Vec<BValue> = content.lines()
                        .map(|l| BValue::String(l.to_string()))
                        .collect();
                    Some(BValue::Array(Rc::new(RefCell::new(lines))))
                }
                Err(_) => Some(BValue::Array(Rc::new(RefCell::new(Vec::new())))),
            }
        }
        func_names::IO_WRITE => {
            let path = args.first().and_then(|a| a.as_str()).unwrap_or("");
            let content = args.get(1).map(|a| a.to_string()).unwrap_or_default();
            let _ = file_write(path, &content);
            Some(BValue::Null)
        }
        func_names::IO_APPEND => {
            let path = args.first().and_then(|a| a.as_str()).unwrap_or("");
            let content = args.get(1).map(|a| a.to_string()).unwrap_or_default();
            let _ = file_append(path, &content);
            Some(BValue::Null)
        }
        func_names::IO_EXISTS => {
            let path = args.first().and_then(|a| a.as_str()).unwrap_or("");
            Some(BValue::Bool(file_exists(path)))
        }
        func_names::IO_DELETE => {
            let path = args.first().and_then(|a| a.as_str()).unwrap_or("");
            let _ = file_delete(path);
            Some(BValue::Null)
        }
        func_names::IO_MKDIR => {
            let path = args.first().and_then(|a| a.as_str()).unwrap_or("");
            let _ = std::fs::create_dir_all(path);
            Some(BValue::Null)
        }
        func_names::IO_LIST_DIR => {
            let path = args.first().and_then(|a| a.as_str()).unwrap_or(".");
            let entries: Vec<BValue> = match std::fs::read_dir(path) {
                Ok(rd) => rd.filter_map(|e| e.ok())
                    .filter_map(|e| e.file_name().into_string().ok())
                    .map(|n| BValue::String(n))
                    .collect(),
                Err(_) => Vec::new(),
            };
            Some(BValue::Array(Rc::new(RefCell::new(entries))))
        }
        func_names::IO_FILE_SIZE => {
            let path = args.first().and_then(|a| a.as_str()).unwrap_or("");
            let size = std::fs::metadata(path)
                .map(|m| m.len() as i64)
                .unwrap_or(-1);
            Some(BValue::Integer(size))
        }

        _ => None,
    }
}

/// 获取 Value 的类型名（用于 print_debug）
fn type_name_of(val: &BValue) -> &'static str {
    match val {
        BValue::Integer(_) => "i64",
        BValue::Float(_) => "f64",
        BValue::String(_) => "String",
        BValue::Bool(_) => "Bool",
        BValue::Char(_) => "Char",
        BValue::Null => "Null",
        BValue::Array(_) => "Array",
        BValue::Struct(_) => "Struct",
        BValue::Enum(_) => "Enum",
        BValue::Map(_) => "Map",
        BValue::Matrix(_) => "Matrix",
        BValue::TransformerModel(_) => "TransformerModel",
        BValue::Function(_) => "Function",
    }
}

/// 从 Value::Array 提取 Vec<Vec<String>> (用于 print_table 的行数据)
fn extract_table_rows(val: &BValue) -> Vec<Vec<String>> {
    match val {
        BValue::Array(rows_rc) => {
            let rows_ref = rows_rc.borrow();
            rows_ref.iter().map(|row_val| {
                match row_val {
                    BValue::Array(cells_rc) => {
                        let cells_ref = cells_rc.borrow();
                        cells_ref.iter().map(|c| c.to_string()).collect()
                    }
                    _ => vec![row_val.to_string()],
                }
            }).collect()
        }
        _ => Vec::new(),
    }
}

// ============================================================================
// Value 辅助扩展 trait
// ============================================================================

/// 扩展 Value 类型的辅助方法
trait ValueExt {
    fn as_str(&self) -> Option<&str>;
    fn as_str_array(&self) -> Option<Vec<String>>;
}

impl ValueExt for BValue {
    fn as_str(&self) -> Option<&str> {
        match self {
            BValue::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    fn as_str_array(&self) -> Option<Vec<String>> {
        match self {
            BValue::Array(rc) => {
                let arr = rc.borrow();
                let strings: Vec<String> = arr.iter()
                    .map(|v| v.to_string())
                    .collect();
                Some(strings)
            }
            _ => None,
        }
    }
}

// ============================================================================
// VM 注册宏 — 一行注册一个 IO 函数
// ============================================================================

/// 在 VM 的 handle_call 中注册所有 IO 标准库函数
///
/// 调用方式（在 VM 代码中）:
/// ```ignore
/// // 在 handle_call 的开头插入:
/// if let Some(result) = std::io::dispatch_io_func(name, &popped_args) {
///     self.stack.push(result);
///     return Ok(());
/// }
/// ```
#[macro_export]
macro_rules! register_io_funcs {
    () => {
        // 此宏留空 — 实际注册通过 dispatch_io_func 完成
        // 在构建时由 VM 初始化调用
    };
}

/// 返回所有已注册 IO 函数名列表（用于调试/文档）
pub fn list_io_functions() -> Vec<&'static str> {
    vec![
        // 新版
        func_names::INPUT,
        func_names::INPUT_NUM,
        func_names::EPRINT,
        func_names::EPRINTLN,
        func_names::FILE_READ,
        func_names::FILE_WRITE,
        func_names::FILE_APPEND,
        func_names::FILE_EXISTS,
        func_names::FILE_DELETE,
        func_names::FMT_PRINTF,
        func_names::PRINT_TABLE,
        func_names::PRINT_DEBUG,
        func_names::FLUSH,
        func_names::STDIN_IS_EMPTY,
        // 旧版兼容
        func_names::IO_READ,
        func_names::IO_READ_LINES,
        func_names::IO_WRITE,
        func_names::IO_APPEND,
        func_names::IO_EXISTS,
        func_names::IO_DELETE,
        func_names::IO_MKDIR,
        func_names::IO_LIST_DIR,
        func_names::IO_FILE_SIZE,
    ]
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ─── 格式化 IO 测试 ───

    #[test]
    fn test_fmt_printf_simple() {
        let result = fmt_printf("Hello, {}!", &["World".to_string()]);
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_fmt_printf_multiple() {
        let result = fmt_printf("{} + {} = {}", &[
            "1".to_string(),
            "2".to_string(),
            "3".to_string(),
        ]);
        assert_eq!(result, "1 + 2 = 3");
    }

    #[test]
    fn test_fmt_printf_no_args() {
        let result = fmt_printf("No placeholders", &[]);
        assert_eq!(result, "No placeholders");
    }

    #[test]
    fn test_fmt_printf_missing_args() {
        let result = fmt_printf("{} {} {}", &["only".to_string()]);
        assert_eq!(result, "only {} {}");
    }

    // ─── 表格打印测试 ───

    #[test]
    fn test_print_table_basic() {
        let headers = vec!["Name".to_string(), "Age".to_string()];
        let rows = vec![
            vec!["Alice".to_string(), "25".to_string()],
            vec!["Bob".to_string(), "30".to_string()],
        ];
        let result = print_table(&headers, &rows);
        assert!(result.contains("│ Name  │ Age │"));
        assert!(result.contains("│ Alice │ 25  │"));
        assert!(result.contains("│ Bob   │ 30  │"));
        assert!(result.starts_with('┌'));
        assert!(result.ends_with("┘\n"));
    }

    #[test]
    fn test_print_table_empty() {
        let headers: Vec<String> = vec![];
        let rows: Vec<Vec<String>> = vec![];
        let result = print_table(&headers, &rows);
        assert_eq!(result, "");
    }

    // ─── 调试打印测试 ───

    #[test]
    fn test_debug_format() {
        let result = debug_format("i64", "42");
        assert_eq!(result, "[DEBUG] i64 = 42");
    }

    #[test]
    fn test_debug_format_string() {
        let result = debug_format("String", "\"hello\"");
        assert_eq!(result, "[DEBUG] String = \"hello\"");
    }

    // ─── 文件 IO 测试 ───

    #[test]
    fn test_file_write_and_read() {
        let test_path = "_test_io_stdlib.txt";
        let content = "Hello, KLC IO Test!";

        // 写入
        assert!(file_write(test_path, content).is_ok());

        // 存在性检查
        assert!(file_exists(test_path));

        // 读取
        let read_back = file_read(test_path).unwrap();
        assert_eq!(read_back, content);

        // 删除
        assert!(file_delete(test_path).is_ok());
        assert!(!file_exists(test_path));
    }

    #[test]
    fn test_file_append() {
        let test_path = "_test_io_append.txt";
        let _ = file_delete(test_path); // 清理

        file_write(test_path, "line1\n").ok();
        file_append(test_path, "line2\n").ok();

        let content = file_read(test_path).unwrap();
        assert_eq!(content, "line1\nline2\n");

        let _ = file_delete(test_path);
    }

    #[test]
    fn test_file_not_exists() {
        assert!(!file_exists("_nonexistent_file_xyz_123.txt"));
    }

    // ─── 类型检测测试 ───

    #[test]
    fn test_type_name_of() {
        assert_eq!(type_name_of(&BValue::Integer(1)), "i64");
        assert_eq!(type_name_of(&BValue::Float(1.0)), "f64");
        assert_eq!(type_name_of(&BValue::String("hi".into())), "String");
        assert_eq!(type_name_of(&BValue::Bool(true)), "Bool");
        assert_eq!(type_name_of(&BValue::Null), "Null");
    }

    // ─── dispatch_io_func 集成测试 ───

    #[test]
    fn test_dispatch_file_exists() {
        // 创建临时文件, 确保路径正确
        let test_path = "_test_dispatch_exists_temp.txt";
        let _ = std::fs::write(test_path, "test");
        let result = dispatch_io_func("file_exists", &[
            BValue::String(test_path.to_string()),
        ]);
        assert!(matches!(result, Some(BValue::Bool(true))));
        let _ = std::fs::remove_file(test_path);
    }

    #[test]
    fn test_dispatch_file_not_exists() {
        let result = dispatch_io_func("file_exists", &[
            BValue::String("_this_file_does_not_exist_999.txt".to_string()),
        ]);
        assert!(matches!(result, Some(BValue::Bool(false))));
    }

    #[test]
    fn test_dispatch_unknown_func() {
        let result = dispatch_io_func("unknown_func_xyz", &[]);
        assert!(result.is_none());
    }

    #[test]
    fn test_dispatch_fmt_printf() {
        let result = dispatch_io_func("fmt_printf", &[
            BValue::String("Hello {}!".to_string()),
            BValue::String("KLC".to_string()),
        ]);
        assert!(matches!(result, Some(BValue::String(_))));
        if let Some(BValue::String(s)) = result {
            assert_eq!(s, "Hello KLC!");
        }
    }

    #[test]
    fn test_list_io_functions() {
        let funcs = list_io_functions();
        assert!(funcs.contains(&"input"));
        assert!(funcs.contains(&"file_read"));
        assert!(funcs.contains(&"file_write"));
        assert!(funcs.contains(&"print_table"));
        assert!(funcs.contains(&"print_debug"));
        assert!(funcs.contains(&"fmt_printf"));
        assert!(!funcs.is_empty());
    }
}
