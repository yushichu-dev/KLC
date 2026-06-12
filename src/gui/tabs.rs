//! KLC IDE — 多文档 Tab 管理
//!
//! 只有一个全局编辑器对（editor.rs 的 G_EDITOR_HWND），
//! 每个 Tab 在 TabInfo.saved_text 中保存编辑器文本。
//! 切换 Tab 时：从全局编辑器读回旧 Tab 文本 → 写入新 Tab 文本到全局编辑器。
//! 这避免了多 EDIT 控件显隐/剪裁导致的文本丢失。

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

const WS_CHILD: DWORD = 0x40000000;
const WS_VISIBLE: DWORD = 0x10000000;
const WS_CLIPSIBLINGS: DWORD = 0x04000000;

const TCS_FIXEDWIDTH: DWORD = 0x0400;
const TCM_INSERTITEMW: UINT = 0x133E;
const TCM_GETCURSEL: UINT = 0x130B;
const TCM_SETCURSEL: UINT = 0x130C;
const TCM_SETITEMSIZE: UINT = 0x1341;
const TCM_DELETEITEM: UINT = 0x1308;
const TCN_SELCHANGE: i32 = -551;

const MAX_TABS: usize = 20;

#[link(name = "user32")]
#[link(name = "comctl32")]
extern "system" {
    fn CreateWindowExW(dwExStyle: DWORD, lpClassName: *const WCHAR, lpWindowName: *const WCHAR,
        dwStyle: DWORD, x: i32, y: i32, nWidth: i32, nHeight: i32,
        hWndParent: HWND, hMenu: isize, hInstance: HINSTANCE, lpParam: *mut std::ffi::c_void) -> HWND;
    fn SendMessageW(hWnd: HWND, Msg: UINT, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
    fn GetModuleHandleW(lpModuleName: *const WCHAR) -> HINSTANCE;
    fn MoveWindow(hWnd: HWND, X: i32, Y: i32, nWidth: i32, nHeight: i32, bRepaint: BOOL) -> BOOL;
    fn SetFocus(hWnd: HWND) -> HWND;
    fn DestroyWindow(hWnd: HWND) -> BOOL;
    fn GetClientRect(hWnd: HWND, lpRect: *mut RECT) -> BOOL;
}
extern "system" { fn InitCommonControls(); }

#[repr(C)]
struct TCITEMW {
    mask: UINT, dwState: DWORD, dwStateMask: DWORD,
    pszText: *mut WCHAR, cchTextMax: i32, iImage: i32, lParam: LPARAM,
}
const TCIF_TEXT: UINT = 0x0001;

#[repr(C)]
struct RECT { left: i32, top: i32, right: i32, bottom: i32 }

fn to_wide(s: &str) -> Vec<WCHAR> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Tab 信息——不存储编辑器句柄，只存储文本（由全局单编辑器承载）
struct TabInfo {
    saved_text: String,
    file_path: Option<String>,
    title: String,
    modified: bool,
}

static mut G_TAB_HWND: HWND = 0;
static mut G_TABS: [Option<TabInfo>; MAX_TABS] = [const { None }; MAX_TABS];
static mut G_TAB_COUNT: usize = 0;
static mut G_ACTIVE_TAB: usize = 0;
static mut G_PARENT: HWND = 0;

/// 编辑器区域的坐标（相对于主窗口客户区，由 window.rs 的 update_layout 传入）
static mut G_EDITOR_X: i32 = 0;
static mut G_EDITOR_Y: i32 = 0;
static mut G_EDITOR_W: i32 = 0;
static mut G_EDITOR_H: i32 = 0;

// ──────────────────────────────────────────────
// 公共查询接口
// ──────────────────────────────────────────────

pub unsafe fn get_tab_hwnd() -> HWND { G_TAB_HWND }

pub unsafe fn get_active_editor() -> HWND {
    super::editor::get_editor_hwnd()
}

pub unsafe fn get_active_file() -> Option<String> {
    G_TABS[G_ACTIVE_TAB].as_ref().and_then(|t| t.file_path.clone())
}

pub unsafe fn tab_count() -> usize { G_TAB_COUNT }
pub unsafe fn active_tab_index() -> usize { G_ACTIVE_TAB }

// ──────────────────────────────────────────────
// 文本保存/恢复（核心！）
// ──────────────────────────────────────────────

/// 将当前全局编辑器的文本和修改状态保存到指定 Tab
unsafe fn save_tab_text(tab_idx: usize) {
    if tab_idx >= G_TAB_COUNT { return; }
    if let Some(ref mut tab) = G_TABS[tab_idx] {
        tab.saved_text = super::editor::get_source_code();
        tab.modified = super::editor::is_modified();
    }
}

/// 将指定 Tab 保存的文本和修改状态恢复到全局编辑器
unsafe fn restore_tab_text(tab_idx: usize) {
    if tab_idx >= G_TAB_COUNT { return; }
    if let Some(ref tab) = G_TABS[tab_idx] {
        super::editor::set_source_code(&tab.saved_text);
        super::editor::set_modified(tab.modified);
    }
}

/// 立即把当前编辑器文本保存到当前激活 Tab
pub unsafe fn save_current() {
    save_tab_text(G_ACTIVE_TAB);
}

// ──────────────────────────────────────────────
// 创建 Tab 控件（仅标签栏）
// ──────────────────────────────────────────────

pub unsafe fn create_tab_control(parent: HWND, x: i32, y: i32, w: i32, h: i32) -> HWND {
    InitCommonControls();
    G_PARENT = parent;

    let class = to_wide("SysTabControl32");
    let h_inst = GetModuleHandleW(std::ptr::null());

    let hwnd = CreateWindowExW(
        0, class.as_ptr(), std::ptr::null(),
        WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS | TCS_FIXEDWIDTH,
        x, y, w, h,
        parent, 2000, h_inst, std::ptr::null_mut(),
    );

    G_TAB_HWND = hwnd;
    SendMessageW(hwnd, TCM_SETITEMSIZE, 0, ((120 << 16) | 26) as LPARAM);
    hwnd
}

// ──────────────────────────────────────────────
// 新增标签页
// ──────────────────────────────────────────────

pub unsafe fn add_tab(title: &str, file_path: Option<String>) -> usize {
    if G_TAB_COUNT >= MAX_TABS { return G_TAB_COUNT; }

    let idx = G_TAB_COUNT;

    // 将当前编辑器文本保存到当前激活的 Tab
    save_tab_text(G_ACTIVE_TAB);

    // 加入 Tab 标签条
    let w_title = to_wide(title);
    let mut tc: TCITEMW = mem::zeroed();
    tc.mask = TCIF_TEXT;
    tc.pszText = w_title.as_ptr() as *mut WCHAR;
    tc.cchTextMax = w_title.len() as i32;
    SendMessageW(G_TAB_HWND, TCM_INSERTITEMW, idx as WPARAM, &mut tc as *mut _ as LPARAM);

    G_TABS[idx] = Some(TabInfo {
        saved_text: String::new(),
        file_path,
        title: title.to_string(),
        modified: false,
    });
    G_TAB_COUNT += 1;

    // 切换到新标签（清空编辑器）
    switch_to(idx);

    idx
}

// ──────────────────────────────────────────────
// 切换活动标签
// ──────────────────────────────────────────────

pub unsafe fn switch_to(idx: usize) {
    if idx >= G_TAB_COUNT { return; }

    let old_idx = G_ACTIVE_TAB;

    // 步骤1：保存旧 Tab 的文本 + 修改状态
    if old_idx != idx {
        save_tab_text(old_idx);
    }

    G_ACTIVE_TAB = idx;
    SendMessageW(G_TAB_HWND, TCM_SETCURSEL, idx as WPARAM, 0);

    // 步骤2：恢复新 Tab 的文本 + 修改状态到全局编辑器
    restore_tab_text(idx);

    // 确保全局 editor 句柄正确（create_editor 已经设好了，但确认一下）
    super::find_replace::set_target_editor(super::editor::get_editor_hwnd());
}

// ──────────────────────────────────────────────
// 布局编辑器（窗口 resize 时由 window.rs 调用）
// ──────────────────────────────────────────────

pub unsafe fn set_editor_area(x: i32, y: i32, w: i32, h: i32) {
    G_EDITOR_X = x; G_EDITOR_Y = y; G_EDITOR_W = w; G_EDITOR_H = h;
    layout_editors();
}

pub unsafe fn resize_tab_control(x: i32, y: i32, w: i32, h: i32) {
    if G_TAB_HWND != 0 {
        MoveWindow(G_TAB_HWND, x, y, w, h, 1);
    }
}

pub unsafe fn layout_editors() {
    // 只有一个全局编辑器，直接 resize
    super::editor::resize_editor(G_EDITOR_X, G_EDITOR_Y, G_EDITOR_W, G_EDITOR_H);
}

// ──────────────────────────────────────────────
// WM_NOTIFY 处理
// ──────────────────────────────────────────────

pub unsafe fn handle_notify(hwnd_from: HWND, code: UINT) -> bool {
    if hwnd_from == G_TAB_HWND && code == TCN_SELCHANGE as u32 {
        let sel = SendMessageW(G_TAB_HWND, TCM_GETCURSEL, 0, 0) as usize;
        switch_to(sel);
        let ed = super::editor::get_editor_hwnd();
        if ed != 0 { SetFocus(ed); }
        return true;
    }
    false
}

// ──────────────────────────────────────────────
// 关闭标签
// ──────────────────────────────────────────────

pub unsafe fn close_current() {
    if G_TAB_COUNT <= 1 { return; }
    let old_idx = G_ACTIVE_TAB;

    // 保存当前 Tab 文本（虽然要关闭，但为了稳妥先保存）
    save_tab_text(old_idx);

    // 从 Tab 控件移除
    SendMessageW(G_TAB_HWND, TCM_DELETEITEM, old_idx as WPARAM, 0);

    // 数组左移
    for i in old_idx..G_TAB_COUNT - 1 {
        G_TABS[i] = G_TABS[i + 1].take();
    }
    G_TAB_COUNT -= 1;
    G_TABS[G_TAB_COUNT] = None;
    if G_ACTIVE_TAB >= G_TAB_COUNT {
        G_ACTIVE_TAB = G_TAB_COUNT - 1;
    }
    // 切换到另一个 Tab
    switch_to(G_ACTIVE_TAB);
}

// ──────────────────────────────────────────────
// 标签信息更新
// ──────────────────────────────────────────────

pub unsafe fn set_active_title(title: &str) {
    if let Some(ref mut tab) = G_TABS[G_ACTIVE_TAB] {
        tab.title = title.to_string();
        let w_title = to_wide(title);
        let mut tc: TCITEMW = mem::zeroed();
        tc.mask = TCIF_TEXT;
        tc.pszText = w_title.as_ptr() as *mut WCHAR;
        tc.cchTextMax = w_title.len() as i32;
        SendMessageW(G_TAB_HWND, 0x133F, G_ACTIVE_TAB as WPARAM, &mut tc as *mut _ as LPARAM);
    }
}

pub unsafe fn set_active_path(path: Option<String>) {
    if let Some(ref mut tab) = G_TABS[G_ACTIVE_TAB] {
        tab.file_path = path;
    }
}

pub unsafe fn is_modified() -> bool {
    if let Some(ref tab) = G_TABS[G_ACTIVE_TAB] {
        tab.modified
    } else { false }
}

pub unsafe fn set_modified(mod_val: bool) {
    if let Some(ref mut tab) = G_TABS[G_ACTIVE_TAB] {
        tab.modified = mod_val;
    }
    // 也同步到全局编辑器
    super::editor::set_modified(mod_val);
}

// ──────────────────────────────────────────────
// 释放资源
// ──────────────────────────────────────────────

pub unsafe fn release() {
    // 编辑器由 editor::release_editor_resources 释放，这里只清理 Tab 数据
    for i in 0..G_TAB_COUNT {
        G_TABS[i] = None;
    }
    G_TAB_HWND = 0;
    G_TAB_COUNT = 0;
    G_ACTIVE_TAB = 0;
}
