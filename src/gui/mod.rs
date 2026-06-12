//! KLC IDE Beta 图形界面模块
//!
//! 基于原生 Win32 API，零第三方依赖。
//! 通过 `klc --ide` 命令启动。
//!
//! 模块结构:
//! - `window`:       主窗口创建、消息循环、布局管理
//! - `editor`:       代码编辑区（括号配对、缩进、快捷键）
//! - `output`:       底部输出/日志面板
//! - `actions`:      动作处理（编译运行联动、文件对话框）
//! - `controls`:     通用 Win32 控件封装（菜单栏、字体）
//! - `highlight`:    颜色方案与主题
//! - `find_replace`: Ctrl+F 查找/替换对话框
//! - `status_bar`:   底部状态栏
//! - `code_folding`: Ctrl+M 代码折叠
//! - `intellisense`: Ctrl+Space 智能提示
//! - `hotkey`:       全局快捷键定义（保留，实际处理在 editor subclass 内）
//! - `tabs`:         多标签（预留）
//! - `project_tree`: Explorer 侧边栏（预留）

mod window;
mod controls;
mod editor;
mod output;
mod actions;
mod highlight;
mod hotkey;
pub mod find_replace;
pub mod status_bar;
pub mod code_folding;
pub mod intellisense;
pub mod tabs;
pub mod project_tree;

/// 启动 KLC IDE 图形界面
pub fn run_ide() {
    window::run_ide();
}
