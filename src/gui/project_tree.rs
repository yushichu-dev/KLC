//! KLC IDE Beta — 项目文件树 (Explorer 侧边栏)
//!
//! 使用 SysTreeView32 展示 .klc 文件树。

#![allow(non_snake_case)]

use std::path::Path;
use std::fs;

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
const WS_CLIPSIBLINGS: DWORD = 0x04000000;
const WS_EX_CLIENTEDGE: DWORD = 0x00000200;
const WM_NOTIFY: UINT = 0x004E;
const TVS_HASBUTTONS: DWORD = 0x0001;
const TVS_HASLINES: DWORD = 0x0002;
const TVS_LINESATROOT: DWORD = 0x0004;
const TVM_INSERTITEMW: UINT = 0x1132;
const TVM_EXPAND: UINT = 0x1102;
const TVM_SELECTITEM: UINT = 0x110B;
const TVM_GETNEXTITEM: UINT = 0x110A;
const NM_DBLCLK: i32 = -3;
const TVE_EXPAND: DWORD = 2;
const TVGN_CARET: DWORD = 9;
const TVIF_TEXT: UINT = 0x0001;

const TREE_WIDTH: i32 = 200;

#[link(name = "user32")]
extern "system" {
    fn CreateWindowExW(dwExStyle: DWORD, lpClassName: *const WCHAR, lpWindowName: *const WCHAR,
        dwStyle: DWORD, x: i32, y: i32, nWidth: i32, nHeight: i32,
        hWndParent: HWND, hMenu: isize, hInstance: HINSTANCE, lpParam: *mut std::ffi::c_void) -> HWND;
    fn SendMessageW(hWnd: HWND, Msg: UINT, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
    fn GetModuleHandleW(lpModuleName: *const WCHAR) -> HINSTANCE;
    fn MoveWindow(hWnd: HWND, X: i32, Y: i32, nWidth: i32, nHeight: i32, bRepaint: BOOL) -> BOOL;
    fn SetWindowPos(hWnd: HWND, hWndInsertAfter: HWND, X: i32, Y: i32, cx: i32, cy: i32, uFlags: UINT) -> BOOL;
}

#[repr(C)] struct TVINSERTSTRUCTW {
    hParent: isize, hInsertAfter: isize, item: TVITEMEXW,
}
#[repr(C)] struct TVITEMEXW {
    mask: UINT, hItem: isize, state: UINT, stateMask: UINT,
    pszText: *mut WCHAR, cchTextMax: i32, iImage: i32, iSelectedImage: i32,
    cChildren: i32, lParam: LPARAM,
}
#[repr(C)] struct NMHDR { hwndFrom: HWND, idFrom: usize, code: UINT }
#[repr(C)] struct NMTREEVIEWW { hdr: NMHDR, action: UINT, itemOld: TVITEMEXW, itemNew: TVITEMEXW, ptDrag: POINT }
#[repr(C)] struct POINT { x: i32, y: i32 }

fn to_wide(s: &str) -> Vec<WCHAR> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

static mut G_TREE_HWND: HWND = 0;
static mut G_TREE_ROOT_ITEMS: Vec<(isize, String)> = Vec::new();

pub unsafe fn get_tree_hwnd() -> HWND { G_TREE_HWND }
pub fn tree_width() -> i32 { TREE_WIDTH }

pub unsafe fn create_tree(parent: HWND, x: i32, y: i32, w: i32, h: i32) -> HWND {
    let class = to_wide("SysTreeView32");
    let h_inst = GetModuleHandleW(std::ptr::null());
    let style = WS_CHILD | WS_VISIBLE | WS_BORDER | WS_CLIPSIBLINGS
        | TVS_HASBUTTONS | TVS_HASLINES | TVS_LINESATROOT;

    let hwnd = CreateWindowExW(
        WS_EX_CLIENTEDGE, class.as_ptr(), std::ptr::null(),
        style, x, y, w, h,
        parent, 2100, h_inst, std::ptr::null_mut(),
    );
    G_TREE_HWND = hwnd;
    hwnd
}

/// 向项目树添加目录/文件
pub unsafe fn populate_tree(root_path: &str) {
    if G_TREE_HWND == 0 { return; }
    // 清空
    for (_h, _) in &G_TREE_ROOT_ITEMS {
        // 简单删除 (TVM_DELETEITEM = 0x1101)
        // SendMessageW(G_TREE_HWND, 0x1101, 0, *h);
    }
    G_TREE_ROOT_ITEMS.clear();

    let rp = Path::new(root_path);
    if !rp.exists() { return; }

    add_directory(rp, 0);
}

unsafe fn add_directory(dir: &Path, parent: isize) {
    let name = dir.file_name().and_then(|n| n.to_str()).unwrap_or(".");
    let w_name = to_wide(name);

    let mut tvis = TVINSERTSTRUCTW {
        hParent: parent,
        hInsertAfter: 0xFFFF0001isize, // TVI_LAST
        item: TVITEMEXW {
            mask: TVIF_TEXT, hItem: 0, state: 0, stateMask: 0,
            pszText: w_name.as_ptr() as *mut WCHAR, cchTextMax: w_name.len() as i32,
            iImage: 0, iSelectedImage: 0, cChildren: 1, lParam: 0,
        },
    };

    let h_item = SendMessageW(G_TREE_HWND, TVM_INSERTITEMW, 0, &mut tvis as *mut _ as LPARAM);
    let dir_path = dir.to_string_lossy().to_string();
    G_TREE_ROOT_ITEMS.push((h_item, dir_path.clone()));

    // 读取目录
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                add_directory(&p, h_item);
            } else if let Some(ext) = p.extension() {
                if ext == "klc" {
                    let fname = p.file_name().and_then(|n| n.to_str()).unwrap_or("?.klc");
                    let w_fname = to_wide(fname);
                    let mut tvis_f = TVINSERTSTRUCTW {
                        hParent: h_item,
                        hInsertAfter: 0xFFFF0001isize,
                        item: TVITEMEXW {
                            mask: TVIF_TEXT, hItem: 0, state: 0, stateMask: 0,
                            pszText: w_fname.as_ptr() as *mut WCHAR, cchTextMax: w_fname.len() as i32,
                            iImage: 0, iSelectedImage: 0, cChildren: 0, lParam: 0,
                        },
                    };
                    let fh = SendMessageW(G_TREE_HWND, TVM_INSERTITEMW, 0, &mut tvis_f as *mut _ as LPARAM);
                    G_TREE_ROOT_ITEMS.push((fh, p.to_string_lossy().to_string()));
                }
            }
        }
    }

    // 展开目录节点
    if parent == 0 {
        SendMessageW(G_TREE_HWND, TVM_EXPAND, TVE_EXPAND as WPARAM, h_item as LPARAM);
    }
}

/// 处理项目树双击通知 → 打开文件
pub unsafe fn handle_notify(hwnd_from: HWND, code: UINT) -> Option<String> {
    if hwnd_from != G_TREE_HWND { return None; }
    if code != NM_DBLCLK as u32 { return None; }

    // 获取选中项
    let h_item = SendMessageW(G_TREE_HWND, TVM_GETNEXTITEM, TVGN_CARET as WPARAM, 0) as isize;
    for (h, path) in &G_TREE_ROOT_ITEMS {
        if *h == h_item && path.ends_with(".klc") {
            return Some(path.clone());
        }
    }
    None
}

pub unsafe fn resize_tree(x: i32, y: i32, w: i32, h: i32) {
    if G_TREE_HWND != 0 { MoveWindow(G_TREE_HWND, x, y, w, h, 1); }
}

pub unsafe fn release() {
    G_TREE_HWND = 0;
    G_TREE_ROOT_ITEMS.clear();
}
