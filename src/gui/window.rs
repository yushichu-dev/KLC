//! KLC IDE — 主窗口模块（第三阶段：Rich Edit + 行号 + 快捷键 + 主题）
//!
//! 本模块实现 IDE 主窗口的核心逻辑：
//! - 注册窗口类、创建顶层主窗口
//! - 顶部菜单栏（文件、运行、编译、视图）
//! - Rich Edit 代码编辑区（语法高亮、行号显示）
//! - 底部输出面板（只读，深色/浅色主题）
//! - 窗口缩放时所有子控件布局自适应
//! - 快捷键处理（F5/Ctrl+S/Ctrl+B/Ctrl+O/Tab/Enter）
//! - 主题切换（暗色/亮色）
//!
//! 窗口布局（自上而下）：
//! ┌────────────────────────────────────┐
//! │ 菜单栏: 文件 | 运行 | 编译 | 视图  │
//! ├────┬───────────────────────────────┤
//! │行号│                               │
//! │    │        代码编辑区 (Rich Edit)  │
//! │    │                               │
//! ├────┴───────────────────────────────┤
//! │ 输出面板 (Rich Edit, 只读)         │
//! │     固定高度 180px                  │
//! └────────────────────────────────────┘

// Win32 结构体使用 Windows API 原始命名风格（匈牙利表示法），非 Rust snake_case
#![allow(non_snake_case)]
// 预留常量为后续阶段使用
#![allow(dead_code)]

use std::mem;

// 导入子模块
use super::controls;
use super::editor;
use super::output;
use super::actions;
use super::highlight;

// ============================================================================
// Win32 类型定义
// ============================================================================

type HANDLE = isize;
type HWND = HANDLE;
type HINSTANCE = HANDLE;
type HCURSOR = HANDLE;
type HICON = HANDLE;
type HBRUSH = HANDLE;
type HDC = HANDLE;
type LONG = i32;
type LPARAM = isize;
type WPARAM = usize;
type LRESULT = isize;
type WCHAR = u16;
type UINT = u32;
type DWORD = u32;
type BOOL = i32;

const TRUE: BOOL = 1;
const FALSE: BOOL = 0;

// ============================================================================
// Win32 消息常量
// ============================================================================

const WM_DESTROY: UINT = 0x0002;
const WM_PAINT: UINT = 0x000F;
const WM_CLOSE: UINT = 0x0010;
const WM_SIZE: UINT = 0x0005;
const WM_QUIT: UINT = 0x0012;
const WM_CREATE: UINT = 0x0001;
const WM_SETFOCUS: UINT = 0x0007;
const WM_ERASEBKGND: UINT = 0x0014;

/// 命令消息（菜单点击、按钮点击、控件通知等）
const WM_COMMAND: UINT = 0x0111;
/// 通知消息
const WM_NOTIFY: UINT = 0x004E;

/// 编辑框颜色消息
const WM_CTLCOLOREDIT: UINT = 0x0133;

/// 键盘消息
const WM_KEYDOWN: UINT = 0x0100;
const WM_CHAR: UINT = 0x0102;
const WM_SYSKEYDOWN: UINT = 0x0104;

/// 鼠标滚轮消息
const WM_MOUSEWHEEL: UINT = 0x020A;

/// Rich Edit 通知
const EN_CHANGE: UINT = 0x0000300;

/// 自定义消息：请求语法高亮
const WM_USER_HIGHLIGHT: UINT = 0x0400 + 1;
/// 自定义消息：更新行号
const WM_USER_UPDATE_LINENUM: UINT = 0x0400 + 2;

/// SetBkMode: OPAQUE
const OPAQUE: i32 = 2;

// ============================================================================
// Win32 窗口样式常量
// ============================================================================

const WS_OVERLAPPEDWINDOW: DWORD = 0x00CF0000;
const WS_VISIBLE: DWORD = 0x10000000;
const WS_CHILD: DWORD = 0x40000000;
const WS_CLIPSIBLINGS: DWORD = 0x04000000;

const CS_HREDRAW: UINT = 0x0002;
const CS_VREDRAW: UINT = 0x0001;

const SW_SHOW: i32 = 5;

const COLOR_BTNFACE: UINT = 15;
const IDC_ARROW: usize = 32512;

const EXIT_CODE: i32 = 0;

// ============================================================================
// 布局常量
// ============================================================================

/// 输出面板固定高度（像素）
const OUTPUT_PANEL_HEIGHT: i32 = 180;

/// 编辑区和输出面板之间的间距
const PANEL_GAP: i32 = 2;

// ============================================================================
// Win32 结构体定义
// ============================================================================

#[repr(C)]
#[derive(Debug, Clone)]
struct WNDCLASSEXW {
    cbSize: UINT,
    style: UINT,
    lpfnWndProc: WNDPROC,
    cbClsExtra: i32,
    cbWndExtra: i32,
    hInstance: HINSTANCE,
    hIcon: HICON,
    hCursor: HCURSOR,
    hbrBackground: HBRUSH,
    lpszMenuName: *const WCHAR,
    lpszClassName: *const WCHAR,
    hIconSm: HICON,
}

#[repr(C)]
#[derive(Debug, Clone)]
struct MSG {
    hwnd: HWND,
    message: UINT,
    wParam: WPARAM,
    lParam: LPARAM,
    time: DWORD,
    pt: POINT,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct POINT { x: LONG, y: LONG }

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct RECT { left: LONG, top: LONG, right: LONG, bottom: LONG }

#[repr(C)]
struct CREATESTRUCTW {
    lpCreateParams: LPARAM,
    hInstance: HINSTANCE,
    hMenu: HANDLE,
    hwndParent: HWND,
    cy: i32, cx: i32,
    y: i32, x: i32,
    style: LONG,
    lpszName: *const WCHAR,
    lpszClass: *const WCHAR,
    dwExStyle: DWORD,
}

/// 绘图结构体
#[repr(C)]
#[derive(Debug, Clone)]
struct PAINTSTRUCT {
    hdc: HDC,
    fErase: BOOL,
    rcPaint: RECT,
    fRestore: BOOL,
    fIncUpdate: BOOL,
    rgbReserved: [u8; 32],
}

type WNDPROC = Option<unsafe extern "system" fn(HWND, UINT, WPARAM, LPARAM) -> LRESULT>;

// ============================================================================
// Win32 API 函数声明
// ============================================================================

#[link(name = "user32")]
#[link(name = "gdi32")]
#[link(name = "kernel32")]
extern "system" {
    fn GetModuleHandleW(lpModuleName: *const WCHAR) -> HINSTANCE;
    fn RegisterClassExW(lpWndClass: *const WNDCLASSEXW) -> u16;
    fn CreateWindowExW(
        dwExStyle: DWORD, lpClassName: *const WCHAR, lpWindowName: *const WCHAR,
        dwStyle: DWORD, x: i32, y: i32, nWidth: i32, nHeight: i32,
        hWndParent: HWND, hMenu: HANDLE, hInstance: HINSTANCE,
        lpParam: *mut std::ffi::c_void,
    ) -> HWND;
    fn ShowWindow(hWnd: HWND, nCmdShow: i32) -> BOOL;
    fn UpdateWindow(hWnd: HWND) -> BOOL;
    fn GetMessageW(lpMsg: *mut MSG, hWnd: HWND, wMsgFilterMin: UINT, wMsgFilterMax: UINT) -> BOOL;
    fn TranslateMessage(lpMsg: *const MSG) -> BOOL;
    fn DispatchMessageW(lpMsg: *const MSG) -> LRESULT;
    fn PostQuitMessage(nExitCode: i32);
    fn DefWindowProcW(hWnd: HWND, Msg: UINT, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
    fn LoadCursorW(hInstance: HINSTANCE, lpCursorName: *const WCHAR) -> HCURSOR;
    fn GetStockObject(iObject: i32) -> HANDLE;
    fn DestroyWindow(hWnd: HWND) -> BOOL;
    fn BeginPaint(hWnd: HWND, lpPaint: *mut PAINTSTRUCT) -> HDC;
    fn EndPaint(hWnd: HWND, lpPaint: *const PAINTSTRUCT) -> BOOL;
    fn FillRect(hDC: HDC, lprc: *const RECT, hbr: HBRUSH) -> i32;
    fn GetClientRect(hWnd: HWND, lpRect: *mut RECT) -> BOOL;

    /// 设置设备上下文背景模式
    fn SetBkMode(hdc: HDC, mode: i32) -> i32;
    /// 设置设备上下文文本颜色
    fn SetTextColor(hdc: HDC, color: UINT) -> UINT;
    /// 设置设备上下文背景颜色
    fn SetBkColor(hdc: HDC, color: UINT) -> UINT;
    /// 创建纯色画刷
    fn CreateSolidBrush(color: UINT) -> HBRUSH;
    /// 删除 GDI 对象
    fn DeleteObject(ho: isize) -> BOOL;
    /// 将键盘焦点设置到指定窗口
    fn SetFocus(hWnd: HWND) -> HWND;
    /// 获取窗口文本
    fn SetWindowTextW(hWnd: HWND, lpString: *const WCHAR) -> BOOL;
    /// PostMessage
    fn PostMessageW(hWnd: HWND, Msg: UINT, wParam: WPARAM, lParam: LPARAM) -> BOOL;
    /// SendMessage
    fn SendMessageW(hWnd: HWND, Msg: UINT, wParam: WPARAM, lParam: LPARAM) -> isize;
    /// 获取鼠标光标位置
    fn GetCursorPos(lpPoint: *mut POINT) -> BOOL;
    /// 从屏幕坐标获取窗口句柄
    fn WindowFromPoint(Point: POINT) -> HWND;
}

// ============================================================================
// 辅助函数
// ============================================================================

fn to_wide(s: &str) -> Vec<WCHAR> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

// ============================================================================
// 布局状态
// ============================================================================

/// 保存当前窗口客户区尺寸，用于布局计算
static mut G_CLIENT_WIDTH: i32 = 0;
static mut G_CLIENT_HEIGHT: i32 = 0;

/// 记录窗口是否最小化（最小化时不更新布局）
static mut G_IS_MINIMIZED: bool = false;

/// 高亮防抖标记
static mut G_HIGHLIGHT_PENDING: bool = false;

// ============================================================================
// 窗口常量
// ============================================================================

const CLASS_NAME: &str = "KLC_IDE_WINDOW_CLASS";
const WINDOW_TITLE: &str = "KLC IDE";
const WINDOW_WIDTH: i32 = 1000;
const WINDOW_HEIGHT: i32 = 700;

// ============================================================================
// 公共 API
// ============================================================================

/// 启动 KLC IDE 图形界面
pub fn run_ide() {
    unsafe {
        // ─── 步骤 1: 获取实例句柄 ───
        let h_instance = GetModuleHandleW(std::ptr::null());

        // ─── 步骤 2: 注册窗口类 ───
        let class_name = to_wide(CLASS_NAME);
        let h_cursor = LoadCursorW(0, IDC_ARROW as *const WCHAR);
        let h_background = GetStockObject(COLOR_BTNFACE as i32 + 1);

        let wc = WNDCLASSEXW {
            cbSize: mem::size_of::<WNDCLASSEXW>() as UINT,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: h_instance,
            hIcon: 0,
            hCursor: h_cursor,
            hbrBackground: h_background,
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: 0,
        };

        if RegisterClassExW(&wc) == 0 {
            eprintln!("Error: 窗口类注册失败");
            return;
        }

        // ─── 步骤 3: 创建主窗口 ───
        let window_title = to_wide(WINDOW_TITLE);

        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            window_title.as_ptr(),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            -1, -1,
            WINDOW_WIDTH, WINDOW_HEIGHT,
            0, 0,
            h_instance,
            std::ptr::null_mut(),
        );

        if hwnd == 0 {
            eprintln!("Error: 窗口创建失败");
            return;
        }

        // ─── 步骤 4: 显示并刷新窗口 ───
        ShowWindow(hwnd, SW_SHOW);
        UpdateWindow(hwnd);

        // ─── 步骤 5: 消息循环 ───
        let mut msg: MSG = mem::zeroed();

        while GetMessageW(&mut msg, 0, 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

/// 请求语法高亮（通过 PostMessage 实现延迟高亮）
pub unsafe fn request_highlight() {
    // 通过自定义消息异步触发高亮
    PostMessageW(
        editor::get_editor_hwnd(),
        WM_USER_HIGHLIGHT,
        0,
        0,
    );
}

/// 请求行号更新
pub unsafe fn request_linenum_update() {
    PostMessageW(
        editor::get_linenum_hwnd(),
        WM_USER_UPDATE_LINENUM,
        0,
        0,
    );
}

/// 更新窗口标题（显示文件名）
pub unsafe fn update_window_title(filename: Option<&str>) {
    let main_hwnd = editor::get_editor_hwnd();
    // 找到主窗口
    extern "system" {
        fn GetParent(hWnd: HWND) -> HWND;
    }
    let top_hwnd = GetParent(main_hwnd);
    if top_hwnd != 0 {
        let title = match filename {
            Some(name) => format!("{} - KLC IDE", name),
            None => "KLC IDE".to_string(),
        };
        SetWindowTextW(top_hwnd, to_wide(&title).as_ptr());
    }
}

// ============================================================================
// 布局计算与更新
// ============================================================================

unsafe fn update_layout() {
    if G_IS_MINIMIZED {
        return;
    }

    let w = G_CLIENT_WIDTH;
    let h = G_CLIENT_HEIGHT;

    if w <= 0 || h <= 0 {
        return;
    }

    let editor_y = 0;
    let editor_h = h - OUTPUT_PANEL_HEIGHT - PANEL_GAP;
    let output_y = h - OUTPUT_PANEL_HEIGHT;

    editor::resize_editor(0, editor_y, w, editor_h);
    output::resize_output(0, output_y, w, OUTPUT_PANEL_HEIGHT);

    // 更新行号
    editor::update_line_numbers();
}

// ============================================================================
// 菜单命令处理
// ============================================================================

unsafe fn handle_menu_command(menu_id: UINT) {
    match menu_id {
        // ─── 文件菜单 ───
        controls::MENU_FILE_NEW => {
            actions::action_new_file();
            update_window_title(None);
        }
        controls::MENU_FILE_OPEN => {
            actions::action_open_file();
            // 更新标题
            if let Some(file) = actions::get_current_file() {
                let name = std::path::Path::new(&file)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("untitled.klc");
                update_window_title(Some(name));
            }
        }
        controls::MENU_FILE_SAVE => {
            actions::action_save_file();
            if let Some(file) = actions::get_current_file() {
                let name = std::path::Path::new(&file)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("untitled.klc");
                update_window_title(Some(name));
            }
        }
        controls::MENU_FILE_EXIT => {
            PostQuitMessage(EXIT_CODE);
        }

        // ─── 运行菜单 ───
        controls::MENU_RUN_RUN => {
            actions::action_run();
        }
        controls::MENU_RUN_COMPILE => {
            actions::action_compile_native();
        }
        controls::MENU_RUN_BUILD_RUN => {
            actions::action_build_and_run();
        }

        // ─── 编译菜单 ───
        controls::MENU_COMPILE_CHECK => {
            actions::action_check_syntax();
        }
        controls::MENU_COMPILE_FORMAT => {
            actions::action_format();
        }

        // ─── 视图菜单 ───
        controls::MENU_CLEAR_OUTPUT => {
            actions::action_clear_output();
        }
        controls::MENU_THEME_DARK => {
            highlight::set_theme(highlight::Theme::Dark);
            editor::apply_theme();
            update_output_brush();
            output::clear_output();
            output::append_line("[视图] 已切换到暗色主题");
        }
        controls::MENU_THEME_LIGHT => {
            highlight::set_theme(highlight::Theme::Light);
            editor::apply_theme();
            update_output_brush();
            output::clear_output();
            output::append_line("[视图] 已切换到亮色主题");
        }

        _ => {}
    }
}

// ============================================================================
// 全局深色画刷（用于输出面板背景）
// ============================================================================

static mut G_OUTPUT_BRUSH: HBRUSH = 0;

unsafe fn get_output_brush() -> HBRUSH {
    if G_OUTPUT_BRUSH == 0 {
        G_OUTPUT_BRUSH = CreateSolidBrush(highlight::get_output_bg_color());
    }
    G_OUTPUT_BRUSH
}

/// 主题切换时更新输出面板画刷
unsafe fn update_output_brush() {
    if G_OUTPUT_BRUSH != 0 {
        DeleteObject(G_OUTPUT_BRUSH);
    }
    G_OUTPUT_BRUSH = CreateSolidBrush(highlight::get_output_bg_color());
}

// ============================================================================
// 窗口过程
// ============================================================================

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: UINT,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    match msg {
        // ─── WM_CREATE: 窗口创建 ───
        WM_CREATE => {
            // 1. 创建菜单栏
            controls::create_menu_bar(hwnd);

            // 2. 获取客户区初始尺寸
            let mut rect: RECT = mem::zeroed();
            GetClientRect(hwnd, &mut rect);
            G_CLIENT_WIDTH = rect.right - rect.left;
            G_CLIENT_HEIGHT = rect.bottom - rect.top;

            // 3. 计算初始布局
            let editor_h = G_CLIENT_HEIGHT - OUTPUT_PANEL_HEIGHT - PANEL_GAP;
            let output_y = G_CLIENT_HEIGHT - OUTPUT_PANEL_HEIGHT;

            // 4. 创建代码编辑区（Rich Edit）
            editor::create_editor(hwnd, 0, 0, G_CLIENT_WIDTH, editor_h);

            // 5. 创建输出面板
            output::create_output_panel(hwnd, 0, output_y, G_CLIENT_WIDTH, OUTPUT_PANEL_HEIGHT);

            // 6. 输出欢迎信息
            output::append_line("═══════════════════════════════════════");
            output::append_line("  KLC IDE v0.5.0");
            output::append_line("  零依赖原生 Windows 图形界面");
            output::append_line("═══════════════════════════════════════");
            output::append_line("");
            output::append_line("  菜单: 文件 | 运行 | 编译 | 视图");
            output::append_line("  快捷键:");
            output::append_line("    F5        运行");
            output::append_line("    Ctrl+B    编译原生 EXE");
            output::append_line("    Ctrl+S    保存文件");
            output::append_line("    Ctrl+O    打开文件");
            output::append_line("    Ctrl+N    新建文件");
            output::append_line("    Ctrl+D    暗色主题");
            output::append_line("    Tab       插入 4 空格");
            output::append_line("    Enter     自动缩进");
            output::append_line("");
            output::append_line("  在编辑区输入 KLC 代码后按 F5 运行");
            output::append_line("");

            0
        }

        // ─── WM_SIZE: 窗口大小改变 ───
        WM_SIZE => {
            let size_type = (w_param & 0xFFFF) as UINT;

            if size_type == 1 {
                G_IS_MINIMIZED = true;
            } else {
                G_IS_MINIMIZED = false;
                G_CLIENT_WIDTH = (l_param as u32 & 0xFFFF) as i32;
                G_CLIENT_HEIGHT = ((l_param as u32) >> 16) as i32;
                update_layout();
            }
            0
        }

        // ─── WM_COMMAND: 命令消息 ───
        WM_COMMAND => {
            let notify_code = (w_param >> 16) as UINT;
            let ctrl_id = (w_param & 0xFFFF) as UINT;

            if notify_code == 0 && l_param == 0 {
                // 菜单命令
                handle_menu_command(ctrl_id);
            } else if notify_code == EN_CHANGE {
                // 编辑区内容改变 → 触发语法高亮
                // 检查是否是编辑区
                if l_param as HWND == editor::get_editor_hwnd() {
                    if !G_HIGHLIGHT_PENDING {
                        G_HIGHLIGHT_PENDING = true;
                        PostMessageW(hwnd, WM_USER_HIGHLIGHT, 0, 0);
                    }
                }
            }
            DefWindowProcW(hwnd, msg, w_param, l_param)
        }

        // ─── WM_KEYDOWN: 键盘按下 ───
        WM_KEYDOWN => {
            let editor_hwnd = editor::get_editor_hwnd();
            if super::hotkey::handle_keydown(editor_hwnd, w_param, l_param) {
                0 // 已处理
            } else {
                DefWindowProcW(hwnd, msg, w_param, l_param)
            }
        }

        // ─── WM_CHAR: 字符输入 ───
        WM_CHAR => {
            let editor_hwnd = editor::get_editor_hwnd();
            if super::hotkey::handle_char(editor_hwnd, w_param, l_param) {
                0
            } else {
                DefWindowProcW(hwnd, msg, w_param, l_param)
            }
        }

        // ─── WM_MOUSEWHEEL: 鼠标滚轮 ───
        // 直接调用 EM_LINESCROLL 滚动编辑区（最可靠的方式）
        WM_MOUSEWHEEL => {
            // wParam 高16位 = 滚轮增量 (正=向上，负=向下)
            let wheel_delta = ((w_param >> 16) as u16 as i16) as i32;
            if editor::handle_mouse_wheel(wheel_delta) {
                0
            } else {
                DefWindowProcW(hwnd, msg, w_param, l_param)
            }
        }

        // ─── 自定义消息：语法高亮 ───
        WM_USER_HIGHLIGHT => {
            G_HIGHLIGHT_PENDING = false;
            let editor_hwnd = editor::get_editor_hwnd();
            if editor_hwnd != 0 {
                let source = editor::get_source_code();
                highlight::highlight_editor(editor_hwnd, &source);
                editor::update_line_numbers();
            }
            0
        }

        // ─── 自定义消息：更新行号 ───
        WM_USER_UPDATE_LINENUM => {
            editor::update_line_numbers();
            0
        }

        // ─── WM_CTLCOLOREDIT: 编辑框颜色处理 ───
        WM_CTLCOLOREDIT => {
            let edit_hwnd = w_param as HWND;
            let edit_hdc = l_param as HDC;

            if edit_hwnd == output::get_output_hwnd() {
                // 输出面板：根据主题设置颜色
                SetBkMode(edit_hdc, OPAQUE);
                SetTextColor(edit_hdc, highlight::get_output_fg_color());
                SetBkColor(edit_hdc, highlight::get_output_bg_color());
                get_output_brush() as LRESULT
            } else {
                // 编辑区：Rich Edit 自行管理颜色
                DefWindowProcW(hwnd, msg, w_param, l_param)
            }
        }

        // ─── WM_SETFOCUS: 主窗口获得焦点 ───
        WM_SETFOCUS => {
            let editor_hwnd = editor::get_editor_hwnd();
            if editor_hwnd != 0 {
                SetFocus(editor_hwnd);
            }
            0
        }

        // ─── WM_PAINT ───
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = mem::zeroed();
            BeginPaint(hwnd, &mut ps);
            EndPaint(hwnd, &ps);
            0
        }

        // ─── WM_CLOSE ───
        WM_CLOSE => {
            DestroyWindow(hwnd);
            0
        }

        // ─── WM_DESTROY ───
        WM_DESTROY => {
            controls::release_mono_font();
            editor::release_editor_resources();

            if G_OUTPUT_BRUSH != 0 {
                DeleteObject(G_OUTPUT_BRUSH);
                G_OUTPUT_BRUSH = 0;
            }

            PostQuitMessage(EXIT_CODE);
            0
        }

        _ => DefWindowProcW(hwnd, msg, w_param, l_param),
    }
}
