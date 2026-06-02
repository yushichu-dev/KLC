//! KLC IDE — 底部输出/日志面板封装
//!
//! 封装底部只读多行文本区域，用于显示：
//! - 编译日志和错误信息
//! - 程序运行输出
//! - IDE 状态消息
//!
//! 特性：
//! - 黑色背景 + 白色文字（终端风格）
//! - 只读模式（用户不可编辑输出内容）
//! - 自动滚动到底部（新消息追加后）
//! - 清空功能

#![allow(non_snake_case)]
#![allow(dead_code)]

use super::controls;

/// Win32 类型
type HWND = isize;
type LPARAM = isize;
type WPARAM = usize;
type UINT = u32;
type BOOL = i32;
type HDC = isize;
type LONG = i32;
type HBRUSH = isize;

// ============================================================================
// Win32 消息常量
// ============================================================================

/// 编辑框颜色消息 — 请求父窗口提供编辑框的背景/前景色
const WM_CTLCOLOREDIT: UINT = 0x0133;

// ============================================================================
// Win32 GDI 常量
// ============================================================================

/// SetBkMode: OPAQUE — 不透明背景模式（用于输出面板黑色背景）
const OPAQUE: i32 = 2;

/// 系统颜色：窗口背景（浅灰色）
const COLOR_WINDOW: UINT = 5;
/// 系统颜色：窗口文本（黑色）
const COLOR_WINDOWTEXT: UINT = 8;

// ============================================================================
// 输出面板控件 ID
// ============================================================================

/// 输出面板控件 ID（用于 WM_COMMAND 中识别）
pub const OUTPUT_CTRL_ID: usize = 101;

// ============================================================================
// 输出面板颜色常量
// ============================================================================

/// 输出面板背景色 — 黑色 (RGB 30, 30, 30)
/// 使用深灰色而非纯黑，视觉更柔和
pub const OUTPUT_BG_COLOR: UINT = 0x001E1E1E;

/// 输出面板前景色 — 浅灰色 (RGB 212, 212, 212)
pub const OUTPUT_FG_COLOR: UINT = 0x00D4D4D4;

// ============================================================================
// Win32 API 函数声明
// ============================================================================

#[link(name = "user32")]
#[link(name = "gdi32")]
extern "system" {
    fn SetBkMode(hdc: HDC, mode: i32) -> i32;
    fn SetTextColor(hdc: HDC, color: UINT) -> UINT;
    fn SetBkColor(hdc: HDC, color: UINT) -> UINT;
    fn GetStockObject(iObject: i32) -> isize;
    fn SendMessageW(hWnd: HWND, Msg: UINT, wParam: WPARAM, lParam: LPARAM) -> isize;
    fn MoveWindow(hWnd: HWND, X: i32, Y: i32, nWidth: i32, nHeight: i32, bRepaint: BOOL) -> BOOL;
    fn CreateWindowExW(
        dwExStyle: UINT, lpClassName: *const u16, lpWindowName: *const u16,
        dwStyle: UINT, x: i32, y: i32, nWidth: i32, nHeight: i32,
        hWndParent: HWND, hMenu: isize, hInstance: isize,
        lpParam: *mut std::ffi::c_void,
    ) -> HWND;
    fn GetModuleHandleW(lpModuleName: *const u16) -> isize;
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct RECT { left: LONG, top: LONG, right: LONG, bottom: LONG }

// ============================================================================
// 输出面板管理
// ============================================================================

/// 全局输出面板句柄
static mut G_OUTPUT_HWND: HWND = 0;

/// 创建输出面板（底部只读多行文本框）
///
/// # 参数
/// - `parent`: 主窗口句柄
/// - `x, y, w, h`: 输出面板初始位置和尺寸
///
/// # 返回
/// 输出面板控件句柄
pub unsafe fn create_output_panel(parent: HWND, x: i32, y: i32, w: i32, h: i32) -> HWND {
    let hwnd = controls::create_edit_control(parent, x, y, w, h, OUTPUT_CTRL_ID, true);
    G_OUTPUT_HWND = hwnd;

    // 设置输出面板默认提示文本
    controls::set_text(hwnd, "KLC IDE — Ready.\r\n");

    hwnd
}

/// 获取输出面板窗口句柄
pub unsafe fn get_output_hwnd() -> HWND {
    G_OUTPUT_HWND
}

/// 向输出面板追加一行文本
///
/// 自动在文本末尾添加换行符（\r\n），并滚动到底部。
///
/// # 参数
/// - `text`: 要追加的文本行
pub unsafe fn append_line(text: &str) {
    if G_OUTPUT_HWND == 0 { return; }
    controls::append_text(G_OUTPUT_HWND, text, false);
    controls::append_text(G_OUTPUT_HWND, "\r\n", true); // 追加换行并滚动到底部
}

/// 向输出面板追加原始文本（不自动换行）
pub unsafe fn append_raw(text: &str) {
    if G_OUTPUT_HWND == 0 { return; }
    controls::append_text(G_OUTPUT_HWND, text, true);
}

/// 清空输出面板内容
pub unsafe fn clear_output() {
    if G_OUTPUT_HWND == 0 { return; }
    controls::set_text(G_OUTPUT_HWND, "");
}

/// 调整输出面板位置和大小（窗口缩放时调用）
pub unsafe fn resize_output(x: i32, y: i32, w: i32, h: i32) {
    if G_OUTPUT_HWND == 0 { return; }
    controls::resize_control(G_OUTPUT_HWND, x, y, w, h, true);
}

/// 获取输出面板中的全部文本
pub unsafe fn get_output_text() -> String {
    if G_OUTPUT_HWND == 0 { return String::new(); }
    controls::get_text(G_OUTPUT_HWND)
}

/// 打印带时间戳的日志行到输出面板
pub unsafe fn log(text: &str) {
    // 简单格式：[HH:MM:SS] text
    // 这里简化处理，不加时间戳（避免引入更多 Win32 API）
    append_line(text);
}

/// 打印成功信息（绿色前缀 — 但 Win32 Edit 控件不支持富文本，
/// 所以用文本标记代替）
pub unsafe fn log_ok(text: &str) {
    append_line(&format!("[OK] {}", text));
}

/// 打印错误信息
pub unsafe fn log_error(text: &str) {
    append_line(&format!("[ERROR] {}", text));
}

/// 打印警告信息
pub unsafe fn log_warn(text: &str) {
    append_line(&format!("[WARN] {}", text));
}
