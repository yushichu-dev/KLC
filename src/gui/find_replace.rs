//! KLC IDE Beta — 查找/替换对话框 (Ctrl+F)
//!
//! 纯原生 EDIT + Win32 API 实现，零依赖。
//! - Ctrl+F: 弹出查找对话框
//! - F3: 查找下一个
//! - Shift+F3: 查找上一个
//! - 替换功能: 在对话框中提供替换按钮

#![allow(non_snake_case)]
#![allow(dead_code)]

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
type HMENU = isize;
type HDC = isize;
type HFONT = isize;

// ============================================================================
// Win32 常量
// ============================================================================

const WS_CHILD: DWORD = 0x40000000;
const WS_VISIBLE: DWORD = 0x10000000;
const WS_BORDER: DWORD = 0x00800000;
const WS_CLIPSIBLINGS: DWORD = 0x04000000;
const WS_EX_CLIENTEDGE: DWORD = 0x00000200;
const WS_OVERLAPPED: DWORD = 0x00000000;
const WS_CAPTION: DWORD = 0x00C00000;
const WS_SYSMENU: DWORD = 0x00080000;
const WS_POPUP: DWORD = 0x80000000;
const WS_EX_DLGMODALFRAME: DWORD = 0x00000001;
const WS_EX_TOPMOST: DWORD = 0x00000008;

const BS_PUSHBUTTON: DWORD = 0x00000000;
const ES_AUTOHSCROLL: DWORD = 0x00000080;
const ES_LEFT: DWORD = 0x00000000;
const WM_SETFONT: UINT = 0x0030;
const WM_COMMAND: UINT = 0x0111;
const WM_CLOSE: UINT = 0x0010;
const WM_DESTROY: UINT = 0x0002;

/// EM_FINDTEXT: 在Edit控件中查找文本
const EM_FINDTEXT: UINT = 0x047C;
/// EM_SETSEL
const EM_SETSEL: UINT = 0x00B1;
const EM_GETSEL: UINT = 0x00B0;
const EM_REPLACESEL: UINT = 0x00C2;
const EM_GETTEXTLENGTH: UINT = 0x000E;
const WM_SETTEXT: UINT = 0x000C;
const WM_GETTEXTLENGTH: UINT = 0x000E;

/// EM_EXGETSEL
const EM_EXGETSEL: UINT = 0x0434;

// ============================================================================
// Win32 API
// ============================================================================

#[link(name = "user32")]
extern "system" {
    fn CreateWindowExW(
        dwExStyle: DWORD, lpClassName: *const WCHAR, lpWindowName: *const WCHAR,
        dwStyle: DWORD, x: i32, y: i32, nWidth: i32, nHeight: i32,
        hWndParent: HWND, hMenu: HMENU, hInstance: HINSTANCE,
        lpParam: *mut std::ffi::c_void,
    ) -> HWND;
    fn SendMessageW(hWnd: HWND, Msg: UINT, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
    fn GetModuleHandleW(lpModuleName: *const WCHAR) -> HINSTANCE;
    fn DestroyWindow(hWnd: HWND) -> BOOL;
    fn SetFocus(hWnd: HWND) -> HWND;
    fn MoveWindow(hWnd: HWND, X: i32, Y: i32, nWidth: i32, nHeight: i32, bRepaint: BOOL) -> BOOL;
    fn GetWindowRect(hWnd: HWND, lpRect: *mut RECT) -> BOOL;
    fn GetClientRect(hWnd: HWND, lpRect: *mut RECT) -> BOOL;
    fn SetWindowPos(hWnd: HWND, hWndInsertAfter: HWND, X: i32, Y: i32, cx: i32, cy: i32, uFlags: UINT) -> BOOL;
    fn DefWindowProcW(hWnd: HWND, Msg: UINT, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
    fn CreateFontW(nHeight: i32, nWidth: i32, nEscapement: i32, nOrientation: i32,
        fnWeight: i32, fdwItalic: DWORD, fdwUnderline: DWORD, fdwStrikeOut: DWORD,
        fdwCharSet: DWORD, fdwOutputPrecision: DWORD, fdwClipPrecision: DWORD,
        fdwQuality: DWORD, fdwPitchAndFamily: DWORD, lpszFace: *const WCHAR) -> HFONT;
    fn GetKeyState(nVirtKey: i32) -> i16;
}

#[repr(C)]
struct RECT { left: i32, top: i32, right: i32, bottom: i32 }

fn to_wide(s: &str) -> Vec<WCHAR> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

// ============================================================================
// 查找状态
// ============================================================================

static mut G_FIND_HWND: HWND = 0;
static mut G_FIND_EDIT_HWND: HWND = 0;
static mut G_REPLACE_EDIT_HWND: HWND = 0;
static mut G_TARGET_EDITOR: HWND = 0;
static mut G_LAST_FIND_TEXT: Option<Vec<WCHAR>> = None;
static mut G_LAST_FIND_POS: isize = -1;
static mut G_CASE_SENSITIVE: bool = false;

// ============================================================================
// 查找/替换功能
// ============================================================================

/// 在主编辑区中查找文本
unsafe fn do_find(editor_hwnd: HWND, text: &[WCHAR], down: bool) -> bool {
    let text_len = text.len();
    let total_len = SendMessageW(editor_hwnd, EM_GETTEXTLENGTH, 0, 0) as usize;

    if text_len == 0 || total_len == 0 { return false; }

    let (sel_start, sel_end) = get_sel(editor_hwnd);

    // 计算搜索起始位置
    let search_start = if down {
        sel_end as isize
    } else {
        if sel_start > 0 { (sel_start - 1) as isize } else { -1 }
    };

    // EM_FINDTEXT: wParam=flags, lParam=FINDTEXTEXW pointer
    let mut ft: FINDTEXTEXW = mem::zeroed();
    // chrg: 搜索范围
    if down {
        ft.chrg.cpMin = search_start;
        ft.chrg.cpMax = total_len as isize;
    } else {
        ft.chrg.cpMin = 0;
        ft.chrg.cpMax = search_start;
    }
    ft.lpstrText = text.as_ptr();

    let flags: LPARAM = if G_CASE_SENSITIVE { 4 } else { 0 }; // FR_MATCHCASE=4

    let found = SendMessageW(editor_hwnd, EM_FINDTEXT, flags as WPARAM, &mut ft as *mut _ as LPARAM);
    if found >= 0 {
        SendMessageW(editor_hwnd, EM_SETSEL, found as WPARAM, (found + text_len as isize) as LPARAM);
        // 滚动到可见
        SendMessageW(editor_hwnd, 0x00B7, 0, 0); // EM_SCROLLCARET
        true
    } else {
        false
    }
}

/// 替换当前选中文本
unsafe fn do_replace(editor_hwnd: HWND, text: &[WCHAR]) {
    let (start, end) = get_sel(editor_hwnd);
    if start != end {
        // 有选中文本 → 替换
        SendMessageW(editor_hwnd, EM_REPLACESEL, 1, text.as_ptr() as LPARAM);
    }
}

/// 替换全部匹配项
unsafe fn do_replace_all(editor_hwnd: HWND, find_text: &[WCHAR], replace_text: &[WCHAR]) -> usize {
    let mut count = 0;
    // 从开头开始搜索
    SendMessageW(editor_hwnd, EM_SETSEL, 0, 0);
    loop {
        let mut ft: FINDTEXTEXW = mem::zeroed();
        let (_, cur_end) = get_sel(editor_hwnd);
        ft.chrg.cpMin = cur_end as isize;
        ft.chrg.cpMax = -1; // 到末尾
        ft.lpstrText = find_text.as_ptr();
        let found = SendMessageW(editor_hwnd, EM_FINDTEXT, 0, &mut ft as *mut _ as LPARAM);
        if found < 0 { break; }
        SendMessageW(editor_hwnd, EM_SETSEL, found as WPARAM, (found + find_text.len() as isize) as LPARAM);
        SendMessageW(editor_hwnd, EM_REPLACESEL, 1, replace_text.as_ptr() as LPARAM);
        count += 1;
        if count > 10000 { break; } // 安全阀
    }
    count
}

unsafe fn get_sel(hwnd: HWND) -> (usize, usize) {
    let mut start: usize = 0;
    let mut end: usize = 0;
    SendMessageW(hwnd, EM_GETSEL, &mut start as *mut usize as WPARAM, &mut end as *mut usize as LPARAM);
    (start, end)
}

/// 获取对话框中的文本
unsafe fn get_dlg_text(hwnd: HWND) -> Vec<WCHAR> {
    let len = SendMessageW(hwnd, WM_GETTEXTLENGTH, 0, 0) as usize;
    if len == 0 { return vec![]; }
    let mut buf: Vec<WCHAR> = vec![0; len + 1];
    SendMessageW(hwnd, 0x000D, (len + 1) as WPARAM, buf.as_mut_ptr() as LPARAM); // WM_GETTEXT
    let actual = buf.iter().position(|&c| c == 0).unwrap_or(len);
    buf.truncate(actual);
    buf
}

// ============================================================================
// FINDTEXTEX 结构体
// ============================================================================

#[repr(C)]
struct CHARRANGE { cpMin: isize, cpMax: isize }

#[repr(C)]
struct FINDTEXTEXW {
    chrg: CHARRANGE,
    lpstrText: *const WCHAR,
    chrgText: CHARRANGE,
}

// ============================================================================
// 对话框创建
// ============================================================================

const DLG_WIDTH: i32 = 420;
const DLG_HEIGHT: i32 = 180;
const MARGIN: i32 = 10;
const BTN_W: i32 = 80;
const BTN_H: i32 = 26;
const EDIT_H: i32 = 24;
const LABEL_W: i32 = 50;

unsafe fn create_find_dlg(parent: HWND, h_instance: HINSTANCE) -> HWND {
    let class_name = to_wide("KLC_FIND_DLG");
    let title = to_wide("查找");

    let hwnd = CreateWindowExW(
        WS_EX_DLGMODALFRAME | WS_EX_TOPMOST,
        class_name.as_ptr(),
        title.as_ptr(),
        WS_POPUP | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
        -1, -1, DLG_WIDTH, DLG_HEIGHT,
        parent, 0, h_instance,
        std::ptr::null_mut(),
    );

    if hwnd == 0 {
        // 如果自定义类没注册，使用预定义的 #32770 (Dialog)
        let dialog_class = to_wide("#32770");
        CreateWindowExW(
            WS_EX_DLGMODALFRAME | WS_EX_TOPMOST,
            dialog_class.as_ptr(),
            title.as_ptr(),
            WS_POPUP | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            -1, -1, DLG_WIDTH, DLG_HEIGHT,
            parent, 0, h_instance,
            std::ptr::null_mut(),
        )
    } else {
        hwnd
    }
}

/// 显示查找对话框
pub unsafe fn show_find(editor_hwnd: HWND) {
    if G_FIND_HWND != 0 {
        SetFocus(G_FIND_HWND);
        return;
    }
    G_TARGET_EDITOR = editor_hwnd;
    let h_instance = GetModuleHandleW(std::ptr::null());

    // 注册对话框类
    register_find_class(h_instance);

    let hwnd = create_find_dlg(0, h_instance);
    if hwnd == 0 { return; }
    G_FIND_HWND = hwnd;

    // 居中到父窗口
    let ed = super::editor::get_editor_hwnd();
    if ed != 0 {
        center_window(hwnd, ed);
    }

    // 创建子控件
    let font = super::controls::get_mono_font();

    // "查找目标:" 标签
    let lbl1 = to_wide("STATIC");
    let lbl1_text = to_wide("查找目标:");
    let h_label = CreateWindowExW(0, lbl1.as_ptr(), lbl1_text.as_ptr(),
        WS_CHILD | WS_VISIBLE | ES_LEFT | 0x00000000,
        MARGIN, MARGIN + 4, LABEL_W, EDIT_H,
        hwnd, 101, h_instance, std::ptr::null_mut());
    if font != 0 { SendMessageW(h_label, WM_SETFONT, font as WPARAM, 1); }

    // 查找文本框
    let edit_class = to_wide("EDIT");
    let h_edit = CreateWindowExW(
        WS_EX_CLIENTEDGE, edit_class.as_ptr(), std::ptr::null(),
        WS_CHILD | WS_VISIBLE | WS_BORDER | ES_AUTOHSCROLL | ES_LEFT,
        MARGIN + LABEL_W, MARGIN,
        DLG_WIDTH - MARGIN * 2 - LABEL_W, EDIT_H,
        hwnd, 102, h_instance, std::ptr::null_mut());
    if font != 0 { SendMessageW(h_edit, WM_SETFONT, font as WPARAM, 1); }
    G_FIND_EDIT_HWND = h_edit;

    // 如果有上次查找的文本，填入文本框
    if let Some(ref last) = G_LAST_FIND_TEXT {
        if !last.is_empty() && last[0] != 0 {
            SendMessageW(h_edit, WM_SETTEXT, 0, last.as_ptr() as LPARAM);
        }
    }

    // 替换文本框
    let lbl2_text = to_wide("替换为:");
    let h_lbl2 = CreateWindowExW(0, lbl1.as_ptr(), lbl2_text.as_ptr(),
        WS_CHILD | WS_VISIBLE,
        MARGIN, MARGIN + EDIT_H + 8, LABEL_W, EDIT_H,
        hwnd, 105, h_instance, std::ptr::null_mut());
    if font != 0 { SendMessageW(h_lbl2, WM_SETFONT, font as WPARAM, 1); }

    let h_replace = CreateWindowExW(
        WS_EX_CLIENTEDGE, edit_class.as_ptr(), std::ptr::null(),
        WS_CHILD | WS_VISIBLE | WS_BORDER | ES_AUTOHSCROLL | ES_LEFT,
        MARGIN + LABEL_W, MARGIN + EDIT_H + 8,
        DLG_WIDTH - MARGIN * 2 - LABEL_W, EDIT_H,
        hwnd, 106, h_instance, std::ptr::null_mut());
    if font != 0 { SendMessageW(h_replace, WM_SETFONT, font as WPARAM, 1); }
    G_REPLACE_EDIT_HWND = h_replace;

    // 按钮
    let btn_class = to_wide("BUTTON");

    let y_btn = MARGIN + EDIT_H * 2 + 16;
    // "查找下一个" 按钮
    CreateWindowExW(0, btn_class.as_ptr(), to_wide("查找(&F)").as_ptr(),
        WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON,
        MARGIN, y_btn, BTN_W, BTN_H,
        hwnd, 201, h_instance, std::ptr::null_mut());

    // "替换" 按钮
    CreateWindowExW(0, btn_class.as_ptr(), to_wide("替换(&R)").as_ptr(),
        WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON,
        MARGIN + BTN_W + 8, y_btn, BTN_W, BTN_H,
        hwnd, 202, h_instance, std::ptr::null_mut());

    // "全部替换" 按钮
    CreateWindowExW(0, btn_class.as_ptr(), to_wide("全部替换(&A)").as_ptr(),
        WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON,
        MARGIN + (BTN_W + 8) * 2, y_btn, BTN_W + 12, BTN_H,
        hwnd, 203, h_instance, std::ptr::null_mut());

    // "关闭" 按钮
    CreateWindowExW(0, btn_class.as_ptr(), to_wide("关闭").as_ptr(),
        WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON,
        DLG_WIDTH - BTN_W - MARGIN, y_btn, BTN_W, BTN_H,
        hwnd, 204, h_instance, std::ptr::null_mut());

    SetFocus(h_edit);
}

unsafe fn center_window(hwnd: HWND, parent: HWND) {
    let mut pr: RECT = mem::zeroed();
    GetWindowRect(parent, &mut pr);
    let parent_w = pr.right - pr.left;
    let parent_h = pr.bottom - pr.top;
    let x = pr.left + (parent_w - DLG_WIDTH) / 2;
    let y = pr.top + (parent_h - DLG_HEIGHT) / 2;
    MoveWindow(hwnd, x, y, DLG_WIDTH, DLG_HEIGHT, 1);
}

/// 处理查找对话框的 WM_COMMAND
pub unsafe fn handle_find_command(ctrl_id: u32) -> bool {
    if G_TARGET_EDITOR == 0 { return false; }

    match ctrl_id {
        201 => { // 查找下一个
            let text = get_dlg_text(G_FIND_EDIT_HWND);
            if !text.is_empty() {
                G_LAST_FIND_TEXT = Some(text.clone());
                G_CASE_SENSITIVE = false;
                do_find(G_TARGET_EDITOR, &text, true);
            }
            return true;
        }
        202 => { // 替换
            let find_text = get_dlg_text(G_FIND_EDIT_HWND);
            let replace_text = get_dlg_text(G_REPLACE_EDIT_HWND);
            if !find_text.is_empty() {
                G_LAST_FIND_TEXT = Some(find_text.clone());
                do_replace(G_TARGET_EDITOR, &replace_text);
                do_find(G_TARGET_EDITOR, &find_text, true);
            }
            return true;
        }
        203 => { // 全部替换
            let find_text = get_dlg_text(G_FIND_EDIT_HWND);
            let replace_text = get_dlg_text(G_REPLACE_EDIT_HWND);
            if !find_text.is_empty() {
                let count = do_replace_all(G_TARGET_EDITOR, &find_text, &replace_text);
                super::output::append_line(&format!("[查找] 已替换 {} 处", count));
            }
            return true;
        }
        204 => { // 关闭
            close_find_dlg();
            return true;
        }
        _ => {}
    }
    false
}

/// 查找下一个（F3 按下）
pub unsafe fn find_next() -> bool {
    if let Some(ref text) = G_LAST_FIND_TEXT {
        if !text.is_empty() && text[0] != 0 {
            if G_TARGET_EDITOR != 0 {
                return do_find(G_TARGET_EDITOR, text, true);
            }
        }
    }
    false
}

/// 查找上一个（Shift+F3）
pub unsafe fn find_prev() -> bool {
    if let Some(ref text) = G_LAST_FIND_TEXT {
        if !text.is_empty() && text[0] != 0 {
            if G_TARGET_EDITOR != 0 {
                return do_find(G_TARGET_EDITOR, text, false);
            }
        }
    }
    false
}

pub unsafe fn close_find_dlg() {
    if G_FIND_HWND != 0 {
        DestroyWindow(G_FIND_HWND);
        G_FIND_HWND = 0;
        G_FIND_EDIT_HWND = 0;
        G_REPLACE_EDIT_HWND = 0;
    }
}

pub unsafe fn is_find_dlg_open() -> bool {
    G_FIND_HWND != 0
}

pub unsafe fn get_target_editor() -> HWND {
    G_TARGET_EDITOR
}

pub unsafe fn set_target_editor(hwnd: HWND) {
    G_TARGET_EDITOR = hwnd;
}

// ============================================================================
// 窗口类注册
// ============================================================================

unsafe fn register_find_class(h_instance: HINSTANCE) {
    #[repr(C)]
    struct WNDCLASSEXW {
        cbSize: UINT, style: UINT,
        lpfnWndProc: Option<unsafe extern "system" fn(HWND, UINT, WPARAM, LPARAM) -> LRESULT>,
        cbClsExtra: i32, cbWndExtra: i32,
        hInstance: HINSTANCE, hIcon: isize, hCursor: isize,
        hbrBackground: isize, lpszMenuName: *const WCHAR,
        lpszClassName: *const WCHAR, hIconSm: isize,
    }

    extern "system" {
        fn RegisterClassExW(lpWndClass: *const WNDCLASSEXW) -> u16;
        fn LoadCursorW(hInstance: HINSTANCE, lpCursorName: *const WCHAR) -> isize;
    }

    let class = to_wide("KLC_FIND_DLG");
    let mut wc: WNDCLASSEXW = mem::zeroed();
    wc.cbSize = mem::size_of::<WNDCLASSEXW>() as UINT;
    wc.style = 0x0002 | 0x0001; // CS_HREDRAW | CS_VREDRAW
    wc.lpfnWndProc = Some(find_dlg_proc);
    wc.hInstance = h_instance;
    wc.hCursor = LoadCursorW(0, 32512 as *const WCHAR); // IDC_ARROW
    wc.hbrBackground = 16; // COLOR_BTNFACE + 1 = 16
    wc.lpszClassName = class.as_ptr();
    RegisterClassExW(&wc);
}

unsafe extern "system" fn find_dlg_proc(
    hwnd: HWND, msg: UINT, w_param: WPARAM, l_param: LPARAM,
) -> LRESULT {
    match msg {
        WM_COMMAND => {
            let ctrl_id = (w_param & 0xFFFF) as u32;
            handle_find_command(ctrl_id);
            0
        }
        WM_CLOSE => {
            close_find_dlg();
            0
        }
        WM_DESTROY => {
            G_FIND_HWND = 0;
            0
        }
        _ => DefWindowProcW(hwnd, msg, w_param, l_param),
    }
}
