//! KLC IDE — 语法高亮模块
//!
//! 使用 KLC Lexer 对编辑区源码进行词法分析，然后通过 Rich Edit 控件的
//! 字符格式化 API (CHARFORMAT2W) 对不同 Token 类型应用不同颜色。
//!
//! 高亮规则：
//! - 关键字: 蓝色加粗
//! - 字符串: 棕色/橙色
//! - 数字: 绿色
//! - 注释: 灰色斜体
//! - 布尔值: 紫色
//! - 运算符: 浅灰色

#![allow(non_snake_case)]
#![allow(dead_code)]

use crate::token::TokenKind;

/// Win32 类型
type HWND = isize;
type WPARAM = usize;
type LPARAM = isize;
type UINT = u32;
type DWORD = u32;
type BOOL = i32;

// ============================================================================
// 主题颜色定义
// ============================================================================

/// IDE 主题
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Theme {
    /// 暗色主题
    Dark,
    /// 亮色主题
    Light,
}

/// 语法高亮颜色方案
#[derive(Debug, Clone, Copy)]
pub struct HighlightColors {
    /// 关键字颜色 (蓝色)
    pub keyword: COLORREF,
    /// 字符串颜色 (橙色)
    pub string: COLORREF,
    /// 数字颜色 (绿色)
    pub number: COLORREF,
    /// 注释颜色 (灰色)
    pub comment: COLORREF,
    /// 布尔值颜色 (紫色)
    pub boolean: COLORREF,
    /// 运算符颜色
    pub operator: COLORREF,
    /// 标识符颜色（Type 名称等）
    pub type_name: COLORREF,
}

/// COLORREF: 0x00BBGGRR
type COLORREF = UINT;

/// 暗色主题颜色方案（VS Code Dark+ 风格）
const DARK_COLORS: HighlightColors = HighlightColors {
    keyword:   0x0056CDFF,  // #569CD6 亮蓝
    string:    0x00CE9178,  // #CE9178 橙棕
    number:    0x00B5CEA8,  // #B5CEA8 浅绿
    comment:   0x006A9955,  // #6A9955 深绿
    boolean:   0x00C586C0,  // #C586C0 紫
    operator:  0x00D4D4D4,  // #D4D4D4 浅灰
    type_name: 0x004EC9B0,  // #4EC9B0 青绿
};

/// 亮色主题颜色方案
const LIGHT_COLORS: HighlightColors = HighlightColors {
    keyword:   0x000000FF,  // 纯蓝
    string:    0x00A31515,  // 暗红
    number:    0x00098050,  // 绿
    comment:   0x00008000,  // 深绿
    boolean:   0x00800080,  // 紫
    operator:  0x00000000,  // 黑
    type_name: 0x00267499,  // 深青
};

// ============================================================================
// Win32 消息常量
// ============================================================================

/// 设置选择区域
const EM_SETSEL: UINT = 0x00B1;
/// 获取选择区域
const EM_GETSEL: UINT = 0x00B0;
/// 获取字符数
const EM_GETTEXTLENGTH: UINT = 0x000E;
/// 设置字符格式
const EM_SETCHARFORMAT: UINT = 0x0444;
/// 字符格式：仅影响选择
const SCF_SELECTION: UINT = 0x0001;

// ============================================================================
// Win32 结构体
// ============================================================================

/// CHARFORMAT2W — Rich Edit 字符格式
#[repr(C)]
#[derive(Debug, Clone)]
struct CHARFORMAT2W {
    cbSize: UINT,
    dwMask: DWORD,
    dwEffects: DWORD,
    yHeight: i32,
    yOffset: i32,
    crTextColor: COLORREF,
    bCharSet: u8,
    bPitchAndFamily: u8,
    szFaceName: [u16; 32],
    // CHARFORMAT2 扩展字段
    wWeight: u16,
    sSpacing: i16,
    crBackColor: COLORREF,
    lcid: UINT,
    dwReserved: DWORD,
    sStyle: i16,
    wKerning: u16,
    bUnderlineType: u8,
    bAnimation: u8,
    bRevAuthor: u8,
    bReserved1: u8,
}

/// CFM 标志位
const CFM_COLOR: DWORD = 0x40000000;
const CFM_BOLD: DWORD = 0x00000001;
const CFM_ITALIC: DWORD = 0x00000002;
const CFE_BOLD: DWORD = 0x00000001;
const CFE_ITALIC: DWORD = 0x00000002;

// ============================================================================
// 全局状态
// ============================================================================

/// 当前主题
static mut G_THEME: Theme = Theme::Dark;

/// 当前颜色方案
static mut G_COLORS: HighlightColors = DARK_COLORS;

// ============================================================================
// Win32 API
// ============================================================================

#[link(name = "user32")]
extern "system" {
    fn SendMessageW(hWnd: HWND, Msg: UINT, wParam: WPARAM, lParam: LPARAM) -> isize;
}

// ============================================================================
// 公共 API
// ============================================================================

/// 设置当前主题
pub unsafe fn set_theme(theme: Theme) {
    G_THEME = theme;
    G_COLORS = match theme {
        Theme::Dark => DARK_COLORS,
        Theme::Light => LIGHT_COLORS,
    };
}

/// 获取当前主题
pub unsafe fn get_theme() -> Theme {
    G_THEME
}

/// 获取当前颜色方案
pub unsafe fn get_colors() -> HighlightColors {
    G_COLORS
}

// ============================================================================
// 语法高亮核心
// ============================================================================

/// 对编辑区应用语法高亮
///
/// 重新对整个文本进行词法分析，根据 Token 类型设置对应颜色。
pub unsafe fn highlight_editor(hwnd: HWND, source: &str) {
    use crate::lexer::Lexer;

    let colors = G_COLORS;
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();

    // 禁用重绘，批量设置颜色后一次性刷新
    // WM_SETREDRAW: 0x000B — 控制窗口是否响应 WM_PAINT 重绘
    const WM_SETREDRAW: UINT = 0x000B;
    SendMessageW(hwnd, WM_SETREDRAW, 0, 0);

    // 转换为 UTF-16 并计算每个字符的偏移
    let chars: Vec<char> = source.chars().collect();

    // 为每个 token 计算其在源码中的字节偏移 → 字符偏移
    let mut token_ranges: Vec<(usize, usize, TokenKind)> = Vec::new();

    // 通过重新扫描定位每个 token 的字符位置
    let mut char_pos = 0;
    for token in &tokens {
        let token_len = token.lexeme.chars().count();
        // 在源码中查找匹配位置（向前搜索）
        let found_pos = find_token_pos(&chars, &token.lexeme, char_pos);
        if let Some(pos) = found_pos {
            token_ranges.push((pos, pos + token_len, token.kind.clone()));
            char_pos = pos + token_len;
        }
    }

    // 对注释也做高亮（Lexer 跳过了注释，我们需要单独扫描）
    // 注释行以 // 开头
    let comment_ranges = find_comment_ranges(source);

    // 逐个设置 Token 颜色
    for (start, end, kind) in &token_ranges {
        let (color, bold, italic) = token_color(kind, &colors);
        if color != 0 {
            set_text_color(hwnd, *start, *end, color, bold, italic, &chars);
        }
    }

    // 设置注释颜色（覆盖其他高亮）
    for (start, end) in &comment_ranges {
        let start_ch = count_chars_before(source, *start);
        let end_ch = count_chars_before(source, *end);
        set_text_color(hwnd, start_ch, end_ch, colors.comment, false, true, &chars);
    }

    // 恢复重绘
    SendMessageW(hwnd, WM_SETREDRAW, 1, 0);

    // 强制重绘
    extern "system" {
        fn InvalidateRect(hWnd: HWND, lpRect: *const std::ffi::c_void, bErase: BOOL) -> BOOL;
    }
    InvalidateRect(hwnd, std::ptr::null(), 1);
}

/// 根据 Token 类型返回颜色、是否加粗、是否斜体
fn token_color(kind: &TokenKind, colors: &HighlightColors) -> (COLORREF, bool, bool) {
    match kind {
        // 关键字：蓝色加粗
        TokenKind::Let | TokenKind::Mut | TokenKind::Fn | TokenKind::Return |
        TokenKind::If | TokenKind::Else | TokenKind::While | TokenKind::Loop |
        TokenKind::For | TokenKind::In | TokenKind::Break | TokenKind::Continue |
        TokenKind::Type | TokenKind::Impl | TokenKind::Mod | TokenKind::Use |
        TokenKind::Pub | TokenKind::Own | TokenKind::Borrow | TokenKind::Task |
        TokenKind::Go | TokenKind::Match | TokenKind::Trait | TokenKind::Async |
        TokenKind::Await | TokenKind::And | TokenKind::Or | TokenKind::Not |
        TokenKind::Enum | TokenKind::Const | TokenKind::Yield | TokenKind::As |
        TokenKind::Self_ | TokenKind::Any => (colors.keyword, true, false),

        // 布尔值：紫色
        TokenKind::True | TokenKind::False => (colors.boolean, true, false),

        // 字符串和字符：橙色
        TokenKind::String(_) | TokenKind::Char(_) => (colors.string, false, false),

        // 数字：绿色
        TokenKind::Integer(_) | TokenKind::Float(_) => (colors.number, false, false),

        // 标识符（Type 名称大写开头）
        TokenKind::Ident(name) => {
            if name.len() > 0 && name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
               && name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                (colors.type_name, false, false)
            } else {
                (0, false, false) // 0 = 不设置颜色
            }
        }

        // 运算符
        TokenKind::Plus | TokenKind::Minus | TokenKind::Star | TokenKind::Slash |
        TokenKind::Percent | TokenKind::Eq | TokenKind::Neq | TokenKind::Lt |
        TokenKind::Gt | TokenKind::Lte | TokenKind::Gte | TokenKind::Assign |
        TokenKind::PlusEq | TokenKind::MinusEq | TokenKind::StarEq | TokenKind::SlashEq |
        TokenKind::Arrow | TokenKind::FatArrow | TokenKind::Colon | TokenKind::Colon2 |
        TokenKind::Dot | TokenKind::DotDot | TokenKind::DotDotEq | TokenKind::Comma |
        TokenKind::LParen | TokenKind::RParen | TokenKind::LBrace | TokenKind::RBrace |
        TokenKind::LBracket | TokenKind::RBracket | TokenKind::Pipe | TokenKind::Bar |
        TokenKind::Ampersand | TokenKind::Question | TokenKind::Question2 |
        TokenKind::Concat => (0, false, false), // 不单独高亮运算符

        _ => (0, false, false),
    }
}

/// 设置指定范围文本的颜色
/// 使用标准 CFM_BOLD/CFE_BOLD 控制粗细，只修改 crTextColor 和 bold/italic
unsafe fn set_text_color(
    hwnd: HWND,
    start_char: usize,
    end_char: usize,
    color: COLORREF,
    bold: bool,
    italic: bool,
    _chars: &[char],
) {
    // 选择范围
    SendMessageW(hwnd, EM_SETSEL, start_char, end_char as LPARAM);

    // 准备 CHARFORMAT2 — 仅设置颜色和粗体/斜体，不碰对齐/RTL相关字段
    let mut cf = std::mem::zeroed::<CHARFORMAT2W>();
    cf.cbSize = std::mem::size_of::<CHARFORMAT2W>() as UINT;
    cf.dwMask = CFM_COLOR;
    if bold { cf.dwMask |= CFM_BOLD; cf.dwEffects |= CFE_BOLD; }
    if italic { cf.dwMask |= CFM_ITALIC; cf.dwEffects |= CFE_ITALIC; }
    cf.crTextColor = color;

    SendMessageW(
        hwnd,
        EM_SETCHARFORMAT,
        SCF_SELECTION as WPARAM,
        &mut cf as *mut CHARFORMAT2W as LPARAM,
    );
}

/// 在字符数组中查找 token 的位置
fn find_token_pos(chars: &[char], lexeme: &str, start_search: usize) -> Option<usize> {
    let lexeme_chars: Vec<char> = lexeme.chars().collect();
    if lexeme_chars.is_empty() { return None; }
    if start_search >= chars.len() { return None; }

    for i in start_search..chars.len().saturating_sub(lexeme_chars.len() - 1) {
        if chars[i..].starts_with(&lexeme_chars) {
            return Some(i);
        }
    }
    // 向前搜索（容忍位置偏差）
    if start_search > 0 {
        for i in (0..start_search).rev() {
            if i + lexeme_chars.len() <= chars.len()
               && chars[i..i + lexeme_chars.len()] == lexeme_chars {
                return Some(i);
            }
        }
    }
    None
}

/// 查找所有 // 注释的范围（字节偏移）
fn find_comment_ranges(source: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let bytes = source.as_bytes();

    let mut i = 0;
    while i + 1 < bytes.len() {
        // 检查是否在字符串内
        if bytes[i] == b'"' {
            i += 1;
            while i < bytes.len() && bytes[i] != b'"' {
                if bytes[i] == b'\\' && i + 1 < bytes.len() { i += 2; continue; }
                i += 1;
            }
            i += 1;
            continue;
        }
        if bytes[i] == b'/' && bytes[i + 1] == b'/' {
            let start = i;
            // 找到行尾
            while i < bytes.len() && bytes[i] != b'\n' { i += 1; }
            ranges.push((start, i));
        } else {
            i += 1;
        }
    }
    ranges
}

/// 计算字节偏移前的字符数（近似 UTF-8 字符位置）
fn count_chars_before(source: &str, byte_offset: usize) -> usize {
    source[..byte_offset.min(source.len())].chars().count()
}

/// 获取编辑区背景色（根据主题）
pub unsafe fn get_editor_bg_color() -> COLORREF {
    match G_THEME {
        Theme::Dark => 0x001E1E1E,  // VS Code 暗色背景
        Theme::Light => 0x00FFFFFF,  // 白色
    }
}

/// 获取编辑区前景色（根据主题）
pub unsafe fn get_editor_fg_color() -> COLORREF {
    match G_THEME {
        Theme::Dark => 0x00D4D4D4,  // 浅灰
        Theme::Light => 0x00000000,  // 黑色
    }
}

/// 获取输出面板背景色（根据主题）
pub unsafe fn get_output_bg_color() -> COLORREF {
    match G_THEME {
        Theme::Dark => 0x001E1E1E,
        Theme::Light => 0x00F0F0F0,
    }
}

/// 获取输出面板前景色（根据主题）
pub unsafe fn get_output_fg_color() -> COLORREF {
    match G_THEME {
        Theme::Dark => 0x00D4D4D4,
        Theme::Light => 0x00000000,
    }
}
