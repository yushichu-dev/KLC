//! KLC IDE 图形界面模块
//!
//! 本模块实现基于原生 Win32 API 的 GUI 界面，零第三方依赖。
//! 通过 `klc --ide` 命令启动。
//!
//! 模块结构：
//! - `window`:   主窗口创建、消息循环、布局管理、菜单处理
//! - `controls`: 通用 Win32 控件封装（编辑框、按钮、菜单栏）
//! - `editor`:   Rich Edit 代码编辑区（语法高亮、行号、自动缩进）
//! - `output`:   底部输出/日志面板封装（追加日志、清空）
//! - `actions`:  动作处理（编译运行联动、文件对话框、错误展示）
//! - `highlight`:语法高亮颜色方案与主题管理
//! - `hotkey`:   快捷键处理

mod window;
mod controls;
mod editor;
mod output;
mod actions;
mod highlight;
mod hotkey;

/// 启动 KLC IDE 图形界面
///
/// 调用后将进入 Windows 消息循环，阻塞直到窗口关闭。
/// 在 main.rs 中通过 `klc --ide` 参数调用。
pub fn run_ide() {
    window::run_ide();
}
