//! KLC IDE Beta — 快捷键处理模块
//!
//! 处理全局快捷键绑定：
//! - F5: 运行 / F3: 查找下一个 / Shift+F3: 查找上一个
//! - Ctrl+B: 编译 / Ctrl+Shift+B: 编译并运行
//! - Ctrl+S: 保存 / Ctrl+O: 打开 / Ctrl+N: 新建
//! - Ctrl+F: 查找 / Ctrl+A: 全选 / Ctrl+W: 关闭标签
//! - Ctrl+Tab: 切换标签 / Ctrl+M: 折叠代码块
//! - Ctrl+Space: 智能提示

#![allow(non_snake_case)]
#![allow(dead_code)]

/// Win32 类型
type HWND = isize;
type WPARAM = usize;
type LPARAM = isize;
type UINT = u32;
type BOOL = i32;

const VK_RETURN: u32 = 0x0D;
const VK_TAB: u32 = 0x09;
const VK_F3: u32 = 0x72;
const VK_F5: u32 = 0x74;
const VK_S: u32 = 0x53;
const VK_O: u32 = 0x4F;
const VK_N: u32 = 0x4E;
const VK_B: u32 = 0x42;
const VK_A: u32 = 0x41;
const VK_F: u32 = 0x46;
const VK_W: u32 = 0x57;
const VK_M: u32 = 0x4D;
const VK_Z: u32 = 0x5A;
const VK_SPACE: u32 = 0x20;

const EM_SETSEL: UINT = 0x00B1;
const EM_REPLACESEL: UINT = 0x00C2;

const TAB_SPACES: &str = "    ";

#[link(name = "user32")]
extern "system" {
    fn SendMessageW(hWnd: HWND, Msg: UINT, wParam: WPARAM, lParam: LPARAM) -> isize;
    fn GetKeyState(nVirtKey: i32) -> i16;
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

unsafe fn is_ctrl_pressed() -> bool {
    (GetKeyState(0x11) as i32 & 0x8000) != 0
}
unsafe fn is_shift_pressed() -> bool {
    (GetKeyState(0x10) as i32 & 0x8000) != 0
}

pub unsafe fn handle_keydown(editor_hwnd: HWND, w_param: WPARAM, _l_param: LPARAM) -> bool {
    let vk = (w_param & 0xFFFF) as u32;

    // 先处理智能提示导航
    if super::intellisense::is_hint_open() {
        return super::intellisense::hint_key_down(vk);
    }

    match vk {
        // ─── F5: 运行 ───
        VK_F5 if !is_shift_pressed() => {
            super::actions::action_run();
            return true;
        }

        // ─── F3: 查找下一个 ───
        VK_F3 if !is_shift_pressed() => {
            super::find_replace::find_next();
            return true;
        }
        // ─── Shift+F3: 查找上一个 ───
        VK_F3 if is_shift_pressed() => {
            super::find_replace::find_prev();
            return true;
        }

        // ─── Ctrl+F: 查找 ───
        VK_F if is_ctrl_pressed() => {
            super::find_replace::set_target_editor(editor_hwnd);
            super::find_replace::show_find(editor_hwnd);
            return true;
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

        // ─── Ctrl+A: 全选 ───
        VK_A if is_ctrl_pressed() && editor_hwnd != 0 => {
            SendMessageW(editor_hwnd, EM_SETSEL, 0, (-1isize) as LPARAM);
            return true;
        }

        // ─── Ctrl+O: 打开 ───
        VK_O if is_ctrl_pressed() => {
            super::actions::action_open_file();
            return true;
        }

        // ─── Ctrl+N: 新建标签 ───
        VK_N if is_ctrl_pressed() => {
            super::tabs::add_tab("untitled.klc", None);
            return true;
        }

        // ─── Ctrl+W: 关闭当前标签 ───
        VK_W if is_ctrl_pressed() => {
            super::tabs::close_current();
            return true;
        }

        // ─── Ctrl+M: 代码折叠切换 ───
        VK_M if is_ctrl_pressed() => {
            toggle_fold_at_cursor(editor_hwnd);
            return true;
        }

        // ─── Ctrl+Space: 智能提示 ───
        VK_SPACE if is_ctrl_pressed() => {
            trigger_intellisense(editor_hwnd);
            return true;
        }

        // ─── Tab: 插入 4 空格 ───
        VK_TAB if editor_hwnd != 0 => {
            if super::intellisense::is_hint_open() {
                super::intellisense::accept_hint();
                return true;
            }
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

pub unsafe fn handle_char(editor_hwnd: HWND, w_param: WPARAM, _l_param: LPARAM) -> bool {
    let ch = (w_param & 0xFFFF) as u32;
    if ch == 9 && editor_hwnd != 0 { return true; }
    false
}

/// 分析光标位置并切换折叠
unsafe fn toggle_fold_at_cursor(_editor_hwnd: HWND) {
    let source = super::editor::get_source_code();
    let line = super::editor::get_current_line();
    let mut blocks = super::code_folding::analyze_folds(&source);
    if super::code_folding::toggle_fold(&mut blocks, line as usize) {
        let folded = super::code_folding::fold_source(&source, &blocks);
        super::editor::set_source_code(&folded);
    }
}

/// 触发智能提示
unsafe fn trigger_intellisense(editor_hwnd: HWND) {
    let source = super::editor::get_source_code();
    let line = super::editor::get_current_line() as usize;
    let lines: Vec<&str> = source.lines().collect();
    if line < lines.len() {
        let cur_line = lines[line];
        // 提取行首到光标之间的最后一个单词
        let prefix = cur_line.trim_end().split(|c: char| !c.is_alphanumeric() && c != '_').last().unwrap_or("");
        if prefix.len() >= 1 {
            super::intellisense::show_hints(editor_hwnd, prefix);
        }
    }
}
