//! KLC IDE Beta — 状态栏模块
//!
//! 位于主窗口底部，显示：
//! - 光标位置（行:列）
//! - 编码格式（UTF-8）
//! - 文件修改状态

#![allow(non_snake_case)]

use std::mem;

type HWND = isize;
type UINT = u32;
type DWORD = u32;
type WPARAM = usize;
type LPARAM = isize;
type LRESULT = isize;
type BOOL = i32;
type WCHAR = u16;
type HINSTANCE = isize;
type HFONT = isize;
type HBRUSH = isize;
type HDC = isize;

const WS_CHILD: DWORD = 0x40000000;
const WS_VISIBLE: DWORD = 0x10000000;
const WS_CLIPSIBLINGS: DWORD = 0x04000000;
const WM_SIZE: UINT = 0x0005;
const WM_PAINT: UINT = 0x000F;
const WM_ERASEBKGND: UINT = 0x0014;
const WM_SETTEXT: UINT = 0x000C;
const WM_SETFONT: UINT = 0x0030;
const WM_SETREDRAW: UINT = 0x000B;

const STATUSBAR_HEIGHT: i32 = 24;

#[link(name = "user32")]
#[link(name = "gdi32")]
extern "system" {
    fn CreateWindowExW(dwExStyle: DWORD, lpClassName: *const WCHAR, lpWindowName: *const WCHAR,
        dwStyle: DWORD, x: i32, y: i32, nWidth: i32, nHeight: i32,
        hWndParent: HWND, hMenu: isize, hInstance: HINSTANCE, lpParam: *mut std::ffi::c_void) -> HWND;
    fn SendMessageW(hWnd: HWND, Msg: UINT, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
    fn GetModuleHandleW(lpModuleName: *const WCHAR) -> HINSTANCE;
    fn MoveWindow(hWnd: HWND, X: i32, Y: i32, nWidth: i32, nHeight: i32, bRepaint: BOOL) -> BOOL;
    fn SetWindowPos(hWnd: HWND, hWndInsertAfter: HWND, X: i32, Y: i32, cx: i32, cy: i32, uFlags: UINT) -> BOOL;
    fn DefWindowProcW(hWnd: HWND, Msg: UINT, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
    fn BeginPaint(hWnd: HWND, lpPaint: *mut PAINTSTRUCT) -> HDC;
    fn EndPaint(hWnd: HWND, lpPaint: *const PAINTSTRUCT) -> BOOL;
    fn GetClientRect(hWnd: HWND, lpRect: *mut RECT) -> BOOL;
    fn CreateSolidBrush(color: UINT) -> HBRUSH;
    fn DeleteObject(ho: isize) -> BOOL;
    fn SetBkMode(hdc: HDC, mode: i32) -> i32;
    fn SetTextColor(hdc: HDC, color: UINT) -> UINT;
    fn SetBkColor(hdc: HDC, color: UINT) -> UINT;
    fn TextOutW(hdc: HDC, x: i32, y: i32, lpString: *const u16, c: i32) -> BOOL;
    fn SelectObject(hdc: HDC, h: isize) -> isize;
    fn CreateFontW(nHeight: i32, nWidth: i32, nEscapement: i32, nOrientation: i32,
        fnWeight: i32, fdwItalic: DWORD, fdwUnderline: DWORD, fdwStrikeOut: DWORD,
        fdwCharSet: DWORD, fdwOutputPrecision: DWORD, fdwClipPrecision: DWORD,
        fdwQuality: DWORD, fdwPitchAndFamily: DWORD, lpszFace: *const WCHAR) -> HFONT;
    fn UpdateWindow(hWnd: HWND) -> BOOL;
    fn InvalidateRect(hWnd: HWND, lpRect: *const std::ffi::c_void, bErase: BOOL) -> BOOL;
}

#[repr(C)]
struct RECT { left: i32, top: i32, right: i32, bottom: i32 }

#[repr(C)]
struct PAINTSTRUCT { hdc: HDC, fErase: BOOL, rcPaint: RECT, fRestore: BOOL, fIncUpdate: BOOL, rgbReserved: [u8; 32] }

fn to_wide(s: &str) -> Vec<WCHAR> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

static mut G_STATUSBAR_HWND: HWND = 0;
static mut G_SB_FONT: HFONT = 0;
const SB_BG: UINT = 0x00F0F0F0; // 浅灰
const SB_FG: UINT = 0x00404040; // 深灰
const STATUSBAR_CLASS: &str = "KLC_StatusBar";

pub unsafe fn get_statusbar_hwnd() -> HWND { G_STATUSBAR_HWND }
pub fn statusbar_height() -> i32 { STATUSBAR_HEIGHT }

pub unsafe fn create_statusbar(parent: HWND, x: i32, y: i32, w: i32, h: i32) -> HWND {
    register_class();

    let class = to_wide(STATUSBAR_CLASS);
    let h_inst = GetModuleHandleW(std::ptr::null());

    let hwnd = CreateWindowExW(
        0, class.as_ptr(), std::ptr::null(),
        WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS,
        x, y, w, h,
        parent, 0, h_inst,
        std::ptr::null_mut(),
    );

    G_STATUSBAR_HWND = hwnd;

    // 创建状态栏专用小字体
    if G_SB_FONT == 0 {
        G_SB_FONT = CreateFontW(-12, 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 0, 0,
            to_wide("Segoe UI").as_ptr());
    }

    hwnd
}

unsafe fn register_class() {
    #[repr(C)]
    struct WNDCLASSEXW {
        cbSize: UINT, style: UINT,
        lpfnWndProc: Option<unsafe extern "system" fn(HWND, UINT, WPARAM, LPARAM) -> LRESULT>,
        cbClsExtra: i32, cbWndExtra: i32,
        hInstance: HINSTANCE, hIcon: isize, hCursor: isize,
        hbrBackground: isize, lpszMenuName: *const WCHAR,
        lpszClassName: *const WCHAR, hIconSm: isize,
    }
    extern "system" { fn RegisterClassExW(lpWndClass: *const WNDCLASSEXW) -> u16; }

    let class = to_wide(STATUSBAR_CLASS);
    let mut wc: WNDCLASSEXW = mem::zeroed();
    wc.cbSize = mem::size_of::<WNDCLASSEXW>() as UINT;
    wc.lpfnWndProc = Some(sb_proc);
    wc.hInstance = GetModuleHandleW(std::ptr::null());
    wc.hbrBackground = CreateSolidBrush(SB_BG);
    wc.lpszClassName = class.as_ptr();
    RegisterClassExW(&wc);
}

static mut G_SB_TEXT: [u16; 256] = [0; 256];

/// 更新状态栏显示
pub unsafe fn update(line: i32, col: i32, modified: bool, encoding: &str) {
    let mod_str = if modified { "* " } else { "" };
    let status = format!("{}  行 {}  列 {}  |  {}", mod_str, line + 1, col + 1, encoding);
    let wide = to_wide(&status);
    let len = wide.len().min(255);
    G_SB_TEXT[..len].copy_from_slice(&wide[..len]);
    G_SB_TEXT[len] = 0;

    if G_STATUSBAR_HWND != 0 {
        InvalidateRect(G_STATUSBAR_HWND, std::ptr::null(), 1);
        UpdateWindow(G_STATUSBAR_HWND);
    }
}

unsafe extern "system" fn sb_proc(hwnd: HWND, msg: UINT, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    match msg {
        m if m == WM_PAINT => {
            let mut ps: PAINTSTRUCT = mem::zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);

            // 背景
            SetBkMode(hdc, 2); // OPAQUE
            SetBkColor(hdc, SB_BG);
            SetTextColor(hdc, SB_FG);

            // 选择字体
            let old_font = if G_SB_FONT != 0 {
                SelectObject(hdc, G_SB_FONT)
            } else { 0 };

            // 绘制文本
            let len = G_SB_TEXT.iter().position(|&c| c == 0).unwrap_or(0);
            if len > 0 {
                TextOutW(hdc, 8, 4, G_SB_TEXT.as_ptr(), len as i32);
            }

            if old_font != 0 { SelectObject(hdc, old_font); }
            EndPaint(hwnd, &ps);
            0
        }
        m if m == WM_ERASEBKGND => 1, // 自己画背景
        _ => DefWindowProcW(hwnd, msg, w_param, l_param),
    }
}

pub unsafe fn resize(x: i32, y: i32, w: i32, h: i32) {
    if G_STATUSBAR_HWND != 0 {
        MoveWindow(G_STATUSBAR_HWND, x, y, w, h, 1);
    }
}

/// 释放状态栏资源
pub unsafe fn release() {
    G_STATUSBAR_HWND = 0;
    if G_SB_FONT != 0 {
        DeleteObject(G_SB_FONT);
        G_SB_FONT = 0;
    }
}
