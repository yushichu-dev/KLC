//! KLC IDE — 主窗口模块 (完整 IDE 布局)
//!
//! 布局:
//! ┌─────────────────────────────────────────────┐
//! │ 菜单栏: 文件 | 运行 | 编译 | 视图           │
//! ├──────────┬──────────────────────────────────┤
//! │ Explorer │ Tab1 | Tab2 | Tab3               │
//! │ 项目树   ├──────┬───────────────────────────┤
//! │          │行号   │ 代码编辑区                 │
//! │          │      │                           │
//! │          ├──────┴───────────────────────────┤
//! │          │ 输出面板                          │
//! ├──────────┴──────────────────────────────────┤
//! │ 状态栏: 行 N 列 N | UTF-8 | 修改             │
//! └─────────────────────────────────────────────┘

#![allow(non_snake_case)]
#![allow(dead_code)]

use std::mem;
use super::{controls, editor, output, actions, highlight, find_replace, status_bar,
            tabs, project_tree};

type HANDLE = isize;
type HWND = HANDLE; type HINSTANCE = HANDLE; type HCURSOR = HANDLE; type HICON = HANDLE;
type HBRUSH = HANDLE; type HDC = HANDLE;
type LONG = i32; type LPARAM = isize; type WPARAM = usize;
type LRESULT = isize; type WCHAR = u16;
type UINT = u32; type DWORD = u32; type BOOL = i32;
const TRUE: BOOL = 1; const FALSE: BOOL = 0;

const WM_DESTROY: UINT = 0x0002; const WM_PAINT: UINT = 0x000F;
const WM_CLOSE: UINT = 0x0010; const WM_SIZE: UINT = 0x0005;
const WM_CREATE: UINT = 0x0001; const WM_SETFOCUS: UINT = 0x0007;
const WM_COMMAND: UINT = 0x0111; const WM_CTLCOLOREDIT: UINT = 0x0133;
const WM_KEYDOWN: UINT = 0x0100; const WM_CHAR: UINT = 0x0102;
const WM_MOUSEWHEEL: UINT = 0x020A; const WM_NOTIFY: UINT = 0x004E;
const EN_CHANGE: UINT = 0x00000300;
const OPAQUE: i32 = 2;

const WS_OVERLAPPEDWINDOW: DWORD = 0x00CF0000;
const WS_VISIBLE: DWORD = 0x10000000;
const CS_HREDRAW: UINT = 0x0002; const CS_VREDRAW: UINT = 0x0001;
const SW_SHOW: i32 = 5; const COLOR_BTNFACE: UINT = 15;
const IDC_ARROW: usize = 32512; const EXIT_CODE: i32 = 0;

// ── 布局常量 ──
const TREE_WIDTH: i32 = 200;
const TAB_BAR_HEIGHT: i32 = 28;
const OUTPUT_PANEL_HEIGHT: i32 = 160;
const PANEL_GAP: i32 = 2;

#[repr(C)]
struct WNDCLASSEXW {
    cbSize: UINT, style: UINT, lpfnWndProc: WNDPROC,
    cbClsExtra: i32, cbWndExtra: i32,
    hInstance: HINSTANCE, hIcon: HICON, hCursor: HCURSOR,
    hbrBackground: HBRUSH, lpszMenuName: *const WCHAR,
    lpszClassName: *const WCHAR, hIconSm: HICON,
}
#[repr(C)]
struct MSG { hwnd: HWND, message: UINT, wParam: WPARAM, lParam: LPARAM, time: DWORD, pt: POINT }
#[repr(C)]
struct POINT { x: LONG, y: LONG }
#[repr(C)]
struct RECT { left: LONG, top: LONG, right: LONG, bottom: LONG }
#[repr(C)]
struct PAINTSTRUCT { hdc: HDC, fErase: BOOL, rcPaint: RECT, fRestore: BOOL, fIncUpdate: BOOL, rgbReserved: [u8; 32] }
#[repr(C)]
struct NMHDR { hwndFrom: HWND, idFrom: isize, code: UINT }
type WNDPROC = Option<unsafe extern "system" fn(HWND, UINT, WPARAM, LPARAM) -> LRESULT>;

#[link(name = "user32")]
#[link(name = "gdi32")]
#[link(name = "kernel32")]
extern "system" {
    fn GetModuleHandleW(lpModuleName: *const WCHAR) -> HINSTANCE;
    fn RegisterClassExW(lpWndClass: *const WNDCLASSEXW) -> u16;
    fn CreateWindowExW(dwExStyle: DWORD, lpClassName: *const WCHAR, lpWindowName: *const WCHAR,
        dwStyle: DWORD, x: i32, y: i32, nWidth: i32, nHeight: i32,
        hWndParent: HWND, hMenu: HANDLE, hInstance: HINSTANCE, lpParam: *mut std::ffi::c_void) -> HWND;
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
    fn GetClientRect(hWnd: HWND, lpRect: *mut RECT) -> BOOL;
    fn SetBkMode(hdc: HDC, mode: i32) -> i32;
    fn SetTextColor(hdc: HDC, color: UINT) -> UINT;
    fn SetBkColor(hdc: HDC, color: UINT) -> UINT;
    fn CreateSolidBrush(color: UINT) -> HBRUSH;
    fn DeleteObject(ho: isize) -> BOOL;
    fn SetFocus(hWnd: HWND) -> HWND;
    fn SetWindowTextW(hWnd: HWND, lpString: *const WCHAR) -> BOOL;
    fn SendMessageW(hWnd: HWND, Msg: UINT, wParam: WPARAM, lParam: LPARAM) -> isize;
}

fn to_wide(s: &str) -> Vec<WCHAR> { s.encode_utf16().chain(std::iter::once(0)).collect() }

static mut G_CLIENT_WIDTH: i32 = 0;
static mut G_CLIENT_HEIGHT: i32 = 0;
static mut G_IS_MINIMIZED: bool = false;
static mut G_OUTPUT_BRUSH: HBRUSH = 0;

const CLASS_NAME: &str = "KLC_IDE_WINDOW";
const WINDOW_TITLE: &str = "KLC IDE Beta";
const WINDOW_WIDTH: i32 = 1100;
const WINDOW_HEIGHT: i32 = 750;

// ──────────────────────────────────────────────
// 入口
// ──────────────────────────────────────────────

pub fn run_ide() {
    unsafe {
        let h_instance = GetModuleHandleW(std::ptr::null());
        let class_name = to_wide(CLASS_NAME);
        let h_cursor = LoadCursorW(0, IDC_ARROW as *const WCHAR);
        let h_background = GetStockObject(COLOR_BTNFACE as i32 + 1);

        let wc = WNDCLASSEXW {
            cbSize: mem::size_of::<WNDCLASSEXW>() as UINT,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_proc), cbClsExtra: 0, cbWndExtra: 0,
            hInstance: h_instance, hIcon: 0, hCursor: h_cursor,
            hbrBackground: h_background,
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: 0,
        };
        if RegisterClassExW(&wc) == 0 { eprintln!("注册窗口类失败"); return; }

        let window_title = to_wide(WINDOW_TITLE);
        let hwnd = CreateWindowExW(0, class_name.as_ptr(), window_title.as_ptr(),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE, -1, -1, WINDOW_WIDTH, WINDOW_HEIGHT,
            0, 0, h_instance, std::ptr::null_mut());
        if hwnd == 0 { eprintln!("创建窗口失败"); return; }

        ShowWindow(hwnd, SW_SHOW); UpdateWindow(hwnd);

        let mut msg: MSG = mem::zeroed();
        while GetMessageW(&mut msg, 0, 0, 0) > 0 {
            TranslateMessage(&msg); DispatchMessageW(&msg);
        }
    }
}

// ──────────────────────────────────────────────
// 布局计算
// ──────────────────────────────────────────────

unsafe fn update_layout() {
    if G_IS_MINIMIZED { return; }
    let w = G_CLIENT_WIDTH;
    let h = G_CLIENT_HEIGHT;
    if w <= 0 || h <= 0 { return; }

    let sb_h = status_bar::statusbar_height();
    let content_h = h - sb_h;

    let right_x = TREE_WIDTH;
    let right_w = w - TREE_WIDTH;
    if right_w < 100 { return; }

    let editor_area_y = TAB_BAR_HEIGHT;
    let editor_area_h = content_h - TAB_BAR_HEIGHT - OUTPUT_PANEL_HEIGHT - PANEL_GAP;
    if editor_area_h < 50 { return; }

    let output_y = content_h - OUTPUT_PANEL_HEIGHT;

    // 1. 项目树：左列，全高
    project_tree::resize_tree(0, 0, TREE_WIDTH, content_h);

    // 2. Tab 标签栏：右列顶部
    tabs::resize_tab_control(right_x, 0, right_w, TAB_BAR_HEIGHT);

    // 3. 编辑器区域：Tab 栏下方到输出面板上方
    tabs::set_editor_area(right_x, editor_area_y, right_w, editor_area_h);

    // 4. 输出面板
    output::resize_output(right_x, output_y, right_w, OUTPUT_PANEL_HEIGHT);

    // 5. 状态栏
    status_bar::resize(0, content_h, w, sb_h);

    // 6. 更新状态栏文本
    status_bar::update(0, 0, false, "UTF-8");
}

// ──────────────────────────────────────────────
// 菜单处理
// ──────────────────────────────────────────────

unsafe fn handle_menu(id: UINT) {
    match id {
        controls::MENU_FILE_NEW => { tabs::add_tab("untitled.klc", None); }
        controls::MENU_FILE_OPEN => { actions::action_open_file(); }
        controls::MENU_FILE_SAVE => { actions::action_save_file(); }
        controls::MENU_FILE_EXIT => { PostQuitMessage(EXIT_CODE); }
        controls::MENU_RUN_RUN => { actions::action_run(); }
        controls::MENU_RUN_COMPILE => { actions::action_compile_native(); }
        controls::MENU_RUN_BUILD_RUN => { actions::action_build_and_run(); }
        controls::MENU_COMPILE_CHECK => { actions::action_check_syntax(); }
        controls::MENU_COMPILE_FORMAT => { actions::action_format(); }
        controls::MENU_CLEAR_OUTPUT => { actions::action_clear_output(); }
        controls::MENU_THEME_DARK => {
            highlight::set_theme(highlight::Theme::Dark);
            editor::apply_theme(); update_output_brush();
            output::clear_output(); output::append_line("[视图] 暗色主题");
        }
        controls::MENU_THEME_LIGHT => {
            highlight::set_theme(highlight::Theme::Light);
            editor::apply_theme(); update_output_brush();
            output::clear_output(); output::append_line("[视图] 亮色主题");
        }
        _ => {}
    }
}

// ──────────────────────────────────────────────
// 输出面板画刷
// ──────────────────────────────────────────────

unsafe fn get_output_brush() -> HBRUSH {
    if G_OUTPUT_BRUSH == 0 {
        G_OUTPUT_BRUSH = CreateSolidBrush(highlight::get_output_bg_color());
    }
    G_OUTPUT_BRUSH
}

unsafe fn update_output_brush() {
    if G_OUTPUT_BRUSH != 0 { DeleteObject(G_OUTPUT_BRUSH); }
    G_OUTPUT_BRUSH = CreateSolidBrush(highlight::get_output_bg_color());
}

// ──────────────────────────────────────────────
// 窗口过程
// ──────────────────────────────────────────────

unsafe extern "system" fn window_proc(
    hwnd: HWND, msg: UINT, w_param: WPARAM, l_param: LPARAM,
) -> LRESULT {
    match msg {
        // ── 窗口创建 ──
        WM_CREATE => {
            controls::create_menu_bar(hwnd);

            let mut rect: RECT = mem::zeroed();
            GetClientRect(hwnd, &mut rect);
            G_CLIENT_WIDTH = rect.right - rect.left;
            G_CLIENT_HEIGHT = rect.bottom - rect.top;

            let w = G_CLIENT_WIDTH;
            let h = G_CLIENT_HEIGHT;
            let sb_h = status_bar::statusbar_height();
            let content_h = h - sb_h;
            let right_x = TREE_WIDTH;
            let right_w = w - TREE_WIDTH;
            let editor_area_y = TAB_BAR_HEIGHT;
            let editor_area_h = content_h - TAB_BAR_HEIGHT - OUTPUT_PANEL_HEIGHT - PANEL_GAP;
            let output_y = content_h - OUTPUT_PANEL_HEIGHT;

            // 1. 项目树
            project_tree::create_tree(hwnd, 0, 0, TREE_WIDTH, content_h);
            // 填充当前目录的 .klc 文件
            project_tree::populate_tree(".");

            // 2. Tab 标签栏
            tabs::create_tab_control(hwnd, right_x, 0, right_w, TAB_BAR_HEIGHT);

            // 3. 创建全局单编辑器（先于 add_tab）
            editor::create_editor(hwnd, right_x, editor_area_y, right_w, editor_area_h.max(200));
            tabs::set_editor_area(right_x, editor_area_y, right_w, editor_area_h.max(200));
            // 第一个编辑器标签
            tabs::add_tab("untitled.klc", None);

            // 4. 输出面板
            output::create_output_panel(hwnd, right_x, output_y, right_w, OUTPUT_PANEL_HEIGHT);

            // 5. 状态栏
            status_bar::create_statusbar(hwnd, 0, content_h, w, sb_h);

            update_layout();

            output::append_line("══════════════════════════════");
            output::append_line("  KLC IDE Beta");
            output::append_line("  F5 运行 | Ctrl+F 查找 | Ctrl+S 保存");
            output::append_line("  Ctrl+B 编译 | Ctrl+M 折叠 | Ctrl+A 全选");
            output::append_line("  Ctrl+N 新建标签 | Ctrl+W 关闭标签");
            output::append_line("══════════════════════════════");
            0
        }

        // ── 窗口大小变化 ──
        WM_SIZE => {
            let st = (w_param & 0xFFFF) as UINT;
            if st == 1 {
                G_IS_MINIMIZED = true;
            } else {
                G_IS_MINIMIZED = false;
                G_CLIENT_WIDTH = (l_param as u32 & 0xFFFF) as i32;
                G_CLIENT_HEIGHT = ((l_param as u32) >> 16) as i32;
                update_layout();
            }
            0
        }

        // ── 菜单命令 ──
        WM_COMMAND => {
            let notify = (w_param >> 16) as UINT;
            let ctrl_id = (w_param & 0xFFFF) as UINT;
            if notify == 0 && l_param == 0 {
                handle_menu(ctrl_id);
            } else if notify == EN_CHANGE {
                // 编辑器内容变化 → 更新行号
                editor::update_line_numbers();
            }
            DefWindowProcW(hwnd, msg, w_param, l_param)
        }

        // ── 通知消息（Tab 切换、树双击） ──
        WM_NOTIFY => {
            let nm = &*(l_param as *const NMHDR);
            // Tab 切换
            if tabs::handle_notify(nm.hwndFrom, nm.code) {
                return 0;
            }
            // 项目树双击 → 打开文件
            if let Some(file_path) = project_tree::handle_notify(nm.hwndFrom, nm.code) {
                match std::fs::read_to_string(&file_path) {
                    Ok(content) => {
                        editor::set_source_code(&content);
                        editor::set_modified(false);
                        actions::set_current_file(Some(file_path.clone()));
                        let fname = std::path::Path::new(&file_path)
                            .file_name().and_then(|n| n.to_str()).unwrap_or("?.klc");
                        tabs::set_active_title(fname);
                        tabs::set_active_path(Some(file_path.clone()));
                        let lines = content.lines().count();
                        output::append_line(&format!("[文件] 已打开: {} ({} 行, {} 字节)",
                            file_path, lines, content.len()));
                    }
                    Err(e) => {
                        output::append_line(&format!("[错误] 打开文件失败: {}", e));
                    }
                }
                return 0;
            }
            DefWindowProcW(hwnd, msg, w_param, l_param)
        }

        // ── 键盘 → 转发给活动编辑器 ──
        WM_KEYDOWN | WM_CHAR => {
            let ed = editor::get_editor_hwnd();
            if ed != 0 { SendMessageW(ed, msg, w_param, l_param) }
            else { DefWindowProcW(hwnd, msg, w_param, l_param) }
        }

        // ── 鼠标滚轮 ──
        WM_MOUSEWHEEL => {
            let delta = ((w_param >> 16) as u16 as i16) as i32;
            if editor::handle_mouse_wheel(delta) { 0 }
            else { DefWindowProcW(hwnd, msg, w_param, l_param) }
        }

        // ── 编辑框颜色 ──
        WM_CTLCOLOREDIT => {
            let edit_hwnd = w_param as HWND;
            let edit_hdc = l_param as HDC;
            if edit_hwnd == output::get_output_hwnd() {
                SetBkMode(edit_hdc, OPAQUE);
                SetTextColor(edit_hdc, highlight::get_output_fg_color());
                SetBkColor(edit_hdc, highlight::get_output_bg_color());
                get_output_brush() as LRESULT
            } else {
                DefWindowProcW(hwnd, msg, w_param, l_param)
            }
        }

        // ── 焦点 → 活动编辑器 ──
        WM_SETFOCUS => {
            let ed = editor::get_editor_hwnd();
            if ed != 0 { SetFocus(ed); }
            0
        }

        // ── 绘制 ──
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = mem::zeroed();
            BeginPaint(hwnd, &mut ps); EndPaint(hwnd, &ps);
            0
        }

        // ── 关闭/销毁 ──
        WM_CLOSE => { DestroyWindow(hwnd); 0 }
        WM_DESTROY => {
            find_replace::close_find_dlg();
            controls::release_mono_font();
            editor::release_editor_resources();
            tabs::release();
            project_tree::release();
            status_bar::release();
            if G_OUTPUT_BRUSH != 0 { DeleteObject(G_OUTPUT_BRUSH); G_OUTPUT_BRUSH = 0; }
            PostQuitMessage(EXIT_CODE);
            0
        }

        _ => DefWindowProcW(hwnd, msg, w_param, l_param),
    }
}
