//! KLC IDE Beta — 智能提示/自动补全 (IntelliSense)
//!
//! 当用户输入关键字前缀时，弹出候选列表。
//! 使用 ListBox 弹出窗口实现。

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

const WS_CHILD: DWORD = 0x40000000;
const WS_VISIBLE: DWORD = 0x10000000;
const WS_BORDER: DWORD = 0x00800000;
const WS_POPUP: DWORD = 0x80000000;
const WS_CLIPSIBLINGS: DWORD = 0x04000000;
const WS_EX_TOPMOST: DWORD = 0x00000008;
const WS_EX_TOOLWINDOW: DWORD = 0x00000080;

const LBS_NOTIFY: DWORD = 0x00000001;
const LB_ADDSTRING: UINT = 0x0180;
const LB_RESETCONTENT: UINT = 0x0184;
const LB_GETCURSEL: UINT = 0x0188;
const LB_GETTEXT: UINT = 0x0189;
const LB_SETSEL: UINT = 0x0185;
const LB_SETCURSEL: UINT = 0x0186;

const WM_COMMAND: UINT = 0x0111;
const WM_ACTIVATE: UINT = 0x0006;
const WM_KILLFOCUS: UINT = 0x0008;
const WM_KEYDOWN: UINT = 0x0100;
const LBN_DBLCLK: UINT = 2;

#[link(name = "user32")]
extern "system" {
    fn CreateWindowExW(dwExStyle: DWORD, lpClassName: *const WCHAR, lpWindowName: *const WCHAR,
        dwStyle: DWORD, x: i32, y: i32, nWidth: i32, nHeight: i32,
        hWndParent: HWND, hMenu: isize, hInstance: HINSTANCE, lpParam: *mut std::ffi::c_void) -> HWND;
    fn SendMessageW(hWnd: HWND, Msg: UINT, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
    fn GetModuleHandleW(lpModuleName: *const WCHAR) -> HINSTANCE;
    fn DestroyWindow(hWnd: HWND) -> BOOL;
    fn SetFocus(hWnd: HWND) -> HWND;
    fn MoveWindow(hWnd: HWND, X: i32, Y: i32, nWidth: i32, nHeight: i32, bRepaint: BOOL) -> BOOL;
}

fn to_wide(s: &str) -> Vec<WCHAR> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// KLC 关键字/内置函数列表
const KEYWORDS: &[&str] = &[
    "let", "mut", "fn", "return", "if", "else", "while", "loop", "for", "in",
    "break", "continue", "type", "impl", "mod", "use", "pub", "own", "borrow",
    "task", "go", "match", "trait", "async", "await", "true", "false",
    "and", "or", "not", "enum", "const", "yield", "as", "self", "any", "null",
    "println", "print", "readln", "len", "push", "pop", "exit",
];

static mut G_HINT_HWND: HWND = 0;
static mut G_HINT_EDITOR: HWND = 0;
static mut G_HINT_MATCHES: Vec<String> = Vec::new();

pub unsafe fn is_hint_open() -> bool { G_HINT_HWND != 0 }

/// 触发智能提示
pub unsafe fn show_hints(editor_hwnd: HWND, prefix: &str) {
    close_hints();

    // 查找匹配的关键字
    let lower = prefix.to_lowercase();
    G_HINT_MATCHES.clear();
    for kw in KEYWORDS {
        if kw.to_lowercase().starts_with(&lower) && kw.to_lowercase() != lower {
            G_HINT_MATCHES.push(kw.to_string());
        }
    }

    if G_HINT_MATCHES.is_empty() { return; }

    G_HINT_EDITOR = editor_hwnd;

    let class = to_wide("LISTBOX");
    let hwnd = CreateWindowExW(
        WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
        class.as_ptr(), std::ptr::null(),
        WS_POPUP | WS_BORDER | WS_VISIBLE | LBS_NOTIFY,
        0, 0, 200, (G_HINT_MATCHES.len() as i32 * 18).min(200),
        0, 0, GetModuleHandleW(std::ptr::null()), std::ptr::null_mut(),
    );

    if hwnd == 0 { return; }
    G_HINT_HWND = hwnd;

    // 填入选项
    for kw in &G_HINT_MATCHES {
        let w = to_wide(kw);
        SendMessageW(hwnd, LB_ADDSTRING, 0, w.as_ptr() as LPARAM);
    }

    // 定位到编辑器光标下方
    unsafe {
        extern "system" {
            fn GetCaretPos(lpPoint: *mut POINT) -> BOOL;
            fn ClientToScreen(hWnd: HWND, lpPoint: *mut POINT) -> BOOL;
        }
        #[repr(C)]
        struct POINT { x: i32, y: i32 }
        let mut pt: POINT = mem::zeroed();
        GetCaretPos(&mut pt);
        ClientToScreen(editor_hwnd, &mut pt);
        MoveWindow(hwnd, pt.x, pt.y + 20, 200, (G_HINT_MATCHES.len() as i32 * 18).min(200), 1);
    }
}

/// 选择当前高亮的提示项，插入到编辑器
pub unsafe fn accept_hint() -> bool {
    if G_HINT_HWND == 0 { return false; }

    let sel = SendMessageW(G_HINT_HWND, LB_GETCURSEL, 0, 0);
    if sel < 0 { return false; }

    let mut buf: [u16; 64] = [0; 64];
    SendMessageW(G_HINT_HWND, LB_GETTEXT, sel as WPARAM, buf.as_mut_ptr() as LPARAM);
    let kw = String::from_utf16_lossy(&buf);

    // 获取编辑器当前光标前的前缀，替换
    // 简单：插入剩余字符
    let remaining = &kw[if kw.len() > 0 { 1 } else { 0 }..]; // 去掉已输入的首字母
    let w = to_wide(remaining);

    extern "system" {
        fn SendMessageW2(hWnd: HWND, Msg: UINT, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
    }
    // EM_REPLACESEL
    SendMessageW(G_HINT_EDITOR, 0x00C2, 1, w.as_ptr() as LPARAM);

    close_hints();
    true
}

pub unsafe fn close_hints() {
    if G_HINT_HWND != 0 {
        DestroyWindow(G_HINT_HWND);
        G_HINT_HWND = 0;
    }
    G_HINT_MATCHES.clear();
}

/// 键盘导航：↑ ↓
pub unsafe fn hint_key_down(vk: u32) -> bool {
    if G_HINT_HWND == 0 { return false; }
    match vk {
        0x28 | 0x27 => { // Down / Right
            let sel = SendMessageW(G_HINT_HWND, LB_GETCURSEL, 0, 0);
            if sel < (G_HINT_MATCHES.len() as isize - 1) as isize {
                SendMessageW(G_HINT_HWND, LB_SETCURSEL, (sel + 1) as WPARAM, 0);
            }
            return true;
        }
        0x26 | 0x25 => { // Up / Left
            let sel = SendMessageW(G_HINT_HWND, LB_GETCURSEL, 0, 0);
            if sel > 0 {
                SendMessageW(G_HINT_HWND, LB_SETCURSEL, (sel - 1) as WPARAM, 0);
            }
            return true;
        }
        0x0D | 0x09 => { // Enter / Tab: 接受
            accept_hint();
            return true;
        }
        0x1B => { // Esc: 关闭
            close_hints();
            return true;
        }
        _ => {}
    }
    false
}
