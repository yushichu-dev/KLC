//! KLC IDE — 快捷键处理模块
//!
//! 处理全局快捷键绑定：
//! - F5: 运行
//! - Ctrl+B: 编译
//! - Ctrl+S: 保存
//! - Ctrl+O: 打开
//! - Ctrl+N: 新建
//! - Ctrl+Shift+B: 编译并运行
//! - Tab: 插入缩进（4空格）
//! - Enter: 自动缩进
//! - Ctrl+Z / Ctrl+Y: 撤销/重做（Rich Edit 内置）
//!
//! 快捷键在 window_proc 的 WM_KEYDOWN 中处理。

#![allow(non_snake_case)]
#![allow(dead_code)]

/// Win32 类型
type HWND = isize;
type WPARAM = usize;
type LPARAM = isize;
type UINT = u32;
type BOOL = i32;

// ============================================================================
// 虚拟键码
// ============================================================================

/// VK_RETURN
const VK_RETURN: u32 = 0x0D;
/// VK_TAB
const VK_TAB: u32 = 0x09;
/// VK_F5
const VK_F5: u32 = 0x74;
/// VK_S
const VK_S: u32 = 0x53;
/// VK_O: u32 = 0x4F;
const VK_O: u32 = 0x4F;
/// VK_N
const VK_N: u32 = 0x4E;
/// VK_B
const VK_B: u32 = 0x42;
/// VK_Z
const VK_Z: u32 = 0x5A;

/// Ctrl 修饰键
const MOD_CTRL: u32 = 0x8000;
/// Shift 修饰键
const MOD_SHIFT: u32 = 0x4000;

// ============================================================================
// Win32 消息
// ============================================================================

/// WM_KEYDOWN
const WM_KEYDOWN: UINT = 0x0100;
/// WM_CHAR
const WM_CHAR: UINT = 0x0102;
/// WM_SYSKEYDOWN
const WM_SYSKEYDOWN: UINT = 0x0104;

/// EM_REPLACESEL
const EM_REPLACESEL: UINT = 0x00C2;

// ============================================================================
// 全局状态
// ============================================================================

/// Tab 宽度（空格数）
const TAB_WIDTH: usize = 4;

/// Tab 替换字符串
const TAB_SPACES: &str = "    ";

// ============================================================================
// Win32 API
// ============================================================================

#[link(name = "user32")]
extern "system" {
    fn SendMessageW(hWnd: HWND, Msg: UINT, wParam: WPARAM, lParam: LPARAM) -> isize;
    fn GetKeyState(nVirtKey: i32) -> i16;
}

// ============================================================================
// 辅助
// ============================================================================

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// 检查 Ctrl 键是否按下
unsafe fn is_ctrl_pressed() -> bool {
    (GetKeyState(0x11) as i32 & 0x8000) != 0 // VK_CONTROL = 0x11
}

/// 检查 Shift 键是否按下
unsafe fn is_shift_pressed() -> bool {
    (GetKeyState(0x10) as i32 & 0x8000) != 0 // VK_SHIFT = 0x10
}

// ============================================================================
// 公共 API
// ============================================================================

/// 处理键盘按下事件
///
/// 在 `window_proc` 的 `WM_KEYDOWN` 中调用。
/// 返回 `true` 表示事件已被处理（阻止默认行为），`false` 表示放行。
///
/// # 参数
/// - `editor_hwnd`: 编辑区窗口句柄
/// - `w_param`: 虚拟键码
/// - `l_param`: 键盘消息参数
pub unsafe fn handle_keydown(editor_hwnd: HWND, w_param: WPARAM, _l_param: LPARAM) -> bool {
    let vk = (w_param & 0xFFFF) as u32;

    match vk {
        // ─── F5: 运行 ───
        VK_F5 => {
            if !is_shift_pressed() {
                super::actions::action_run();
                return true;
            }
        }

        // ─── Ctrl+B: 编译 ───
        VK_B if is_ctrl_pressed() && !is_shift_pressed() => {
            super::actions::action_compile_native();
            return true;
        }

        // ─── Ctrl+Shift+B: 编译并运行 ───
        VK_B if is_ctrl_pressed() && is_shift_pressed() => {
            super::actions::action_build_and_run();
            return true;
        }

        // ─── Ctrl+S: 保存 ───
        VK_S if is_ctrl_pressed() => {
            super::actions::action_save_file();
            return true;
        }

        // ─── Ctrl+O: 打开 ───
        VK_O if is_ctrl_pressed() => {
            super::actions::action_open_file();
            return true;
        }

        // ─── Ctrl+N: 新建 ───
        VK_N if is_ctrl_pressed() => {
            super::actions::action_new_file();
            return true;
        }

        // ─── Tab: 插入 4 空格 ───
        VK_TAB if editor_hwnd != 0 => {
            // 替换 Tab 为 4 个空格
            let spaces = to_wide(TAB_SPACES);
            SendMessageW(editor_hwnd, EM_REPLACESEL, 0, spaces.as_ptr() as LPARAM);
            return true;
        }

        // ─── Enter: 自动缩进 ───
        VK_RETURN if editor_hwnd != 0 => {
            super::editor::handle_enter_indent();
            return true;
        }

        _ => {}
    }

    false
}

/// 处理 WM_CHAR 消息（Tab 的二次拦截，因为 WM_KEYDOWN 后还会有 WM_CHAR）
pub unsafe fn handle_char(editor_hwnd: HWND, w_param: WPARAM, _l_param: LPARAM) -> bool {
    let ch = (w_param & 0xFFFF) as u32;

    // 拦截 Tab 字符（已在 WM_KEYDOWN 中处理为空格）
    if ch == 9 && editor_hwnd != 0 { // VK_TAB = 9
        return true; // 已处理，阻止默认
    }

    false
}
