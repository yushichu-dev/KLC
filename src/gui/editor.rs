//! KLC IDE — 代码编辑区域封装
//!
//! 使用 Rich Edit 控件实现：
//! - 语法高亮（多颜色、加粗、斜体）
//! - 行号显示（通过自定义绘制）
//! - 自动缩进（Tab/Enter 智能缩进）
//! - 等宽字体（Consolas）
//!
//! 从标准 EDIT 控件升级到 RICHEDIT50W 以支持富文本格式。

#![allow(non_snake_case)]
#![allow(dead_code)]

use super::controls;
use super::highlight;

/// Win32 类型
type HWND = isize;
type LPARAM = isize;
type WPARAM = usize;
type UINT = u32;
type DWORD = u32;
type BOOL = i32;
type HDC = isize;
type LONG = i32;
type HBRUSH = isize;
type HANDLE = isize;
type HMODULE = isize;
type COLORREF = UINT;
type WCHAR = u16;

// ============================================================================
// Win32 消息常量
// ============================================================================

const WM_SIZE: UINT = 0x0005;
const WM_NOTIFY: UINT = 0x004E;
const WM_CTLCOLOREDIT: UINT = 0x0133;
const EM_GETSEL: UINT = 0x00B0;
const EM_SETSEL: UINT = 0x00B1;
const EM_SETMODIFY: UINT = 0x00B9;
const EM_GETMODIFY: UINT = 0x00B8;
const EM_GETLINECOUNT: UINT = 0x00BA;
const EM_GETFIRSTVISIBLELINE: UINT = 0x00CE;
const EM_LINEINDEX: UINT = 0x00BB;
const EM_LINELENGTH: UINT = 0x00C1;
const EM_EXGETSEL: UINT = 0x0434;
const EM_REPLACESEL: UINT = 0x00C2;
const EM_GETTEXT: UINT = 0x000D;
const EM_SETTEXT: UINT = 0x000C;
const EM_GETTEXTLENGTH: UINT = 0x000E;
const WM_SETTEXT: UINT = 0x000C;
const WM_GETTEXT: UINT = 0x000D;
const WM_GETTEXTLENGTH: UINT = 0x000E;
const EM_SETTEXTEX: UINT = 0x0461;
const WM_SETFONT: UINT = 0x0030;
const EM_SETREADONLY: UINT = 0x00CF;
const EM_GETCHARFORMAT: UINT = 0x043A;
const EM_SETCHARFORMAT: UINT = 0x0444;
const EM_SETBKGNDCOLOR: UINT = 0x0443;
const SCF_SELECTION: UINT = 0x0001;
/// SCF_ALL: 应用字符格式到全部文本（关键！重置所有格式）
const SCF_ALL: UINT = 0x0004;
const EM_SETTYPOGRAPHYOPTIONS: UINT = 0x04CA;
const EM_GETTEXTLENGTHEX: UINT = 0x0451;
const EM_GETLINE: UINT = 0x00C4;
const EM_LINESCROLL: UINT = 0x00B6;

/// ES_NOHIDESEL: 始终显示选择（失去焦点时不隐藏选区）
const ES_NOHIDESEL: DWORD = 0x00000100;
/// ES_SAVESEL: 失去焦点时保持选择位置
const ES_SAVESEL: DWORD = 0x00000800;

// Rich Edit 编辑选项（EM_SETOPTIONS）
const EM_SETOPTIONS: UINT = 0x044D;
const ECO_WORDBREAK: DWORD = 0x00000020;       // 启用自动换行
/// ECO_RTLREADING: 从右到左阅读模式（必须清除！否则文本从右往左排列）
const ECO_RTLREADING: DWORD = 0x00000080;      // RTL模式 — 必须关闭
const ECO_VERTICAL: DWORD = 0x0000040000;      // 垂直文本模式（必须清除！）
const ECO_AUTOVSCROLL: DWORD = 0x00000040;     // 垂直自动滚动
const ECO_NOHIDESEL: DWORD = 0x00000100;

/// ECOOP_SET: 设置选项（替换全部）
const ECOOP_SET: DWORD = 0x00000001;
/// ECOOP_OR: 或运算（开启指定选项）
const ECOOP_OR: DWORD = 0x00000002;
/// ECOOP_AND: 与运算（关闭指定选项）
const ECOOP_AND: DWORD = 0x00000003;

/// EM_SETWORDWRAPMODE: 设置换行模式
const EM_SETWORDWRAPMODE: UINT = 0x04D0;
/// WBF_WORDWRAP: 在单词边界换行
const WBF_WORDWRAP: DWORD = 0x00000010;
/// WBF_BREAKLINE: 允许断行
const WBF_BREAKLINE: DWORD = 0x00000020;

// Rich Edit 编辑样式（ES_ 前缀）
/// ES_WORDWRAP: 在控件右边缘自动折行（核心修复！）
const ES_WORDWRAP: DWORD = 0x000010;

/// WS_HSCROLL: 水平滚动条（必须移除，防止横向滚动干扰换行）
const WS_HSCROLL: DWORD = 0x00010000;
/// GWL_STYLE: 获取/设置窗口样式
const GWL_STYLE: i32 = -16;

// Rich Edit 排版选项
const TO_ADVANCEDTYPOGRAPHY: DWORD = 0x00000001;
const TO_DISABLECUSTOMTEXTOUT: DWORD = 0x00000008;

/// Rich Edit 扩展消息
const EM_SETEVENTMASK: UINT = 0x044B;
const ENM_CHANGE: UINT = 0x00000001;
const ENM_KEYEVENTS: UINT = 0x00010000;

const EM_SETEDITSTYLE: UINT = 0x04CC;
const SES_EMULATESYSEDIT: DWORD = 0x00000004;

/// WM_CONTEXTMENU: 右键点击弹出原生 EDIT 菜单（撤销/剪切/复制/粘贴/删除/全选）
const WM_CONTEXTMENU: UINT = 0x007B;

// ============================================================================
// Win32 GDI 常量
// ============================================================================

const TRANSPARENT: i32 = 1;
const OPAQUE: i32 = 2;
const COLOR_WINDOW: UINT = 5;

/// ST_DEFAULT: 纯文本，替换当前内容
const ST_DEFAULT: DWORD = 0;

/// GWLP_WNDPROC: 窗口过程指针索引（用于 Subclass）
const GWLP_WNDPROC: i32 = -4;

// ============================================================================
// 控件 ID
// ============================================================================

/// 编辑区控件 ID
pub const EDITOR_CTRL_ID: usize = 100;

// ============================================================================
// Rich Edit CHARFORMAT
// ============================================================================

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

const CFM_COLOR: DWORD = 0x40000000;
const CFM_FACE: DWORD = 0x20000000;
const CFM_SIZE: DWORD = 0x80000000;
const CFM_BOLD: DWORD = 0x00000001;
const CFM_ITALIC: DWORD = 0x00000002;
const CFE_BOLD: DWORD = 0x00000001;
const CFE_ITALIC: DWORD = 0x00000002;

// ============================================================================
// Rich Edit PARAFORMAT (段落格式 — 强制水平方向)
// ============================================================================

/// EM_SETPARAFORMAT: 设置段落格式
const EM_SETPARAFORMAT: UINT = 0x0047;

/// PFM_ALIGNMENT: 段落对齐
const PFM_ALIGNMENT: DWORD = 0x00000008;

/// PFM_DIRECTION: 段落文本方向（关键！防止竖排）
const PFM_DIRECTION: DWORD = 0x00000200;

/// 段落对齐方式
const PFA_LEFT: UINT = 0x0001;

/// wEffects 标志：PFE_VERTICAL — 垂直文本（必须清除！否则变竖排）
const PFE_VERTICAL: u16 = 0x0002;

#[repr(C)]
struct PARAFORMAT2 {
    cbSize: UINT,
    dwMask: DWORD,
    wNumbering: u16,
    wEffects: i16, // WORD signed = SHORT
    dxStartIndent: i32,
    dxRightIndent: i32,
    dxOffset: i32,
    wAlignment: u16,
    cTabCount: i16,
    rgxTabs: [i32; 32],
    dySpaceBefore: i32,
    dySpaceAfter: i32,
    dyLineSpacing: i32,
    sStyle: i16,
    bLineSpacingRule: u8,
    bOutlineLevel: u8,
    wShadingWeight: u16,
    wNumberingStyle: u16,
    wNumberingStart: u16,
    wBorderSpace: u16,
    wBorderWidth: u16,
    wBorders: u16,
}

#[repr(C)]
struct SETTEXTEX {
    codepage: UINT,
    flags: DWORD,
}

#[repr(C)]
struct GETTEXTLENGTHEX {
    flags: DWORD,
    codepage: UINT,
}
const GTL_DEFAULT: DWORD = 0;
const GTL_PRECISE: DWORD = 2;
const GTL_NUMCHARS: DWORD = 8;

// ============================================================================
// 行号相关
// ============================================================================

/// 行号区域宽度（像素）
pub const LINE_NUM_WIDTH: i32 = 48;

/// Win32 消息常量（行号窗口需要）
const WM_PAINT_LN: UINT = 0x000F;
const WM_ERASEBKGND_LN: UINT = 0x0014;
const WM_MOUSEWHEEL: UINT = 0x020A;
/// 自定义消息：延迟插入字符（修复大写字母输入）
const WM_USER_INSERT_CHAR: UINT = 0x0400 + 3;

/// 行号背景色
fn get_line_num_bg_color() -> COLORREF {
    unsafe {
        match highlight::get_theme() {
            highlight::Theme::Dark => 0x001E1E1E,
            highlight::Theme::Light => 0x00E8E8E8,
        }
    }
}

/// 行号文字颜色
fn get_line_num_fg_color() -> COLORREF {
    unsafe {
        match highlight::get_theme() {
            highlight::Theme::Dark => 0x00858585,
            highlight::Theme::Light => 0x00808080,
        }
    }
}

// ============================================================================
// Win32 API
// ============================================================================

#[link(name = "user32")]
#[link(name = "gdi32")]
#[link(name = "kernel32")]
extern "system" {
    fn CreateWindowExW(
        dwExStyle: UINT, lpClassName: *const u16, lpWindowName: *const u16,
        dwStyle: UINT, x: i32, y: i32, nWidth: i32, nHeight: i32,
        hWndParent: HWND, hMenu: isize, hInstance: isize,
        lpParam: *mut std::ffi::c_void,
    ) -> HWND;
    fn SendMessageW(hWnd: HWND, Msg: UINT, wParam: WPARAM, lParam: LPARAM) -> isize;
    fn MoveWindow(hWnd: HWND, X: i32, Y: i32, nWidth: i32, nHeight: i32, bRepaint: BOOL) -> BOOL;
    fn SetBkMode(hdc: HDC, mode: i32) -> i32;
    fn SetTextColor(hdc: HDC, color: UINT) -> UINT;
    fn SetBkColor(hdc: HDC, color: UINT) -> UINT;
    fn GetStockObject(iObject: i32) -> isize;
    fn GetModuleHandleW(lpModuleName: *const WCHAR) -> isize;
    fn LoadLibraryW(lpLibFileName: *const WCHAR) -> HMODULE;
    fn DefWindowProcW(hWnd: HWND, Msg: UINT, wParam: WPARAM, lParam: LPARAM) -> isize;
    fn GetClientRect(hWnd: HWND, lpRect: *mut RECT) -> BOOL;

    // GDI 绘制
    fn CreateFontW(
        nHeight: i32, nWidth: i32, nEscapement: i32, nOrientation: i32,
        fnWeight: i32, fdwItalic: DWORD, fdwUnderline: DWORD, fdwStrikeOut: DWORD,
        fdwCharSet: DWORD, fdwOutputPrecision: DWORD, fdwClipPrecision: DWORD,
        fdwQuality: DWORD, fdwPitchAndFamily: DWORD, lpszFace: *const WCHAR,
    ) -> isize;
    fn CreateSolidBrush(color: UINT) -> HBRUSH;
    fn DeleteObject(ho: isize) -> BOOL;
    fn SelectObject(hdc: HDC, h: isize) -> isize;
    fn SetTextAlign(hdc: HDC, align: UINT) -> UINT;
    fn TextOutW(hdc: HDC, x: i32, y: i32, lpString: *const u16, c: i32) -> BOOL;
    fn GetDeviceCaps(hdc: HDC, index: i32) -> i32;
    fn CreateCompatibleDC(hdc: HDC) -> HDC;
    fn DeleteDC(hdc: HDC) -> BOOL;

    // Subclass API
    fn SetWindowLongPtrW(hWnd: HWND, nIndex: i32, dwNewLong: isize) -> isize;
    fn GetWindowLongPtrW(hWnd: HWND, nIndex: i32) -> isize;
    fn CallWindowProcW(
        lpPrevWndFunc: Option<unsafe extern "system" fn(HWND, UINT, WPARAM, LPARAM) -> isize>,
        hWnd: HWND, Msg: UINT, wParam: WPARAM, lParam: LPARAM,
    ) -> isize;
    fn SetPropW(hWnd: HWND, lpString: *const WCHAR, hData: isize) -> BOOL;
    fn GetPropW(hWnd: HWND, lpString: *const WCHAR) -> isize;
    fn PostMessageW(hWnd: HWND, Msg: UINT, wParam: WPARAM, lParam: LPARAM) -> BOOL;

    // 窗口管理
    fn DestroyWindow(hWnd: HWND) -> BOOL;
    fn InvalidateRect(hWnd: HWND, lpRect: *const std::ffi::c_void, bErase: BOOL) -> BOOL;
    fn UpdateWindow(hWnd: HWND) -> BOOL;
    fn SetFocus(hWnd: HWND) -> HWND;
}

/// TA_RIGHT: 文本右对齐
const TA_RIGHT: UINT = 2;
/// LOGPIXELSY: 屏幕垂直分辨率
const LOGPIXELSY: i32 = 90;

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct RECT { left: LONG, top: LONG, right: LONG, bottom: LONG }

// ============================================================================
// 全局状态
// ============================================================================

/// 全局编辑区句柄
static mut G_EDITOR_HWND: HWND = 0;

/// 行号窗口句柄
static mut G_LINENUM_HWND: HWND = 0;

/// Rich Edit 模块句柄
static mut G_RICHEDIT_MODULE: HMODULE = 0;

/// 行号专用字体
static mut G_LINENUM_FONT: isize = 0;

/// 上次高亮的文本长度（用于增量高亮）
static mut G_LAST_TEXT_LEN: usize = 0;

/// 高亮防抖计时器
static mut G_HIGHLIGHT_PENDING: bool = false;

/// Rich Edit 原始窗口过程（Subclass 保存）
static mut G_ORIGINAL_EDIT_PROC: Option<unsafe extern "system" fn(HWND, UINT, WPARAM, LPARAM) -> isize> = None;

// ============================================================================
// 辅助函数
// ============================================================================

fn to_wide(s: &str) -> Vec<WCHAR> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// 加载 Rich Edit DLL
unsafe fn load_richedit() -> bool {
    if G_RICHEDIT_MODULE != 0 { return true; }

    // 尝试加载 Rich Edit 5.0（Windows 8+），回退到 4.1
    let dll_name = to_wide("Msftedit.dll");
    G_RICHEDIT_MODULE = LoadLibraryW(dll_name.as_ptr());
    if G_RICHEDIT_MODULE == 0 {
        let dll41 = to_wide("Riched20.dll");
        G_RICHEDIT_MODULE = LoadLibraryW(dll41.as_ptr());
    }
    G_RICHEDIT_MODULE != 0
}

// ============================================================================
// 行号窗口过程
// ============================================================================

/// 行号窗口类名
const LINENUM_CLASS: &str = "KLC_LineNum";

/// 注册行号窗口类
unsafe fn register_linenum_class() -> bool {
    use std::mem;

    #[repr(C)]
    struct WNDCLASSEXW {
        cbSize: UINT,
        style: UINT,
        lpfnWndProc: Option<unsafe extern "system" fn(HWND, UINT, WPARAM, LPARAM) -> isize>,
        cbClsExtra: i32,
        cbWndExtra: i32,
        hInstance: isize,
        hIcon: isize,
        hCursor: isize,
        hbrBackground: isize,
        lpszMenuName: *const WCHAR,
        lpszClassName: *const WCHAR,
        hIconSm: isize,
    }

    let class_name = to_wide(LINENUM_CLASS);
    let h_instance = GetModuleHandleW(std::ptr::null());
    let h_bg = CreateSolidBrush(get_line_num_bg_color());

    let wc = WNDCLASSEXW {
        cbSize: mem::size_of::<WNDCLASSEXW>() as UINT,
        style: 0, // CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(linenum_wndproc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: h_instance,
        hIcon: 0,
        hCursor: 0, // LoadCursorW(0, IDC_ARROW)
        hbrBackground: h_bg,
        lpszMenuName: std::ptr::null(),
        lpszClassName: class_name.as_ptr(),
        hIconSm: 0,
    };

    extern "system" {
        fn RegisterClassExW(lpWndClass: *const WNDCLASSEXW) -> u16;
    }
    RegisterClassExW(&wc) != 0
}

/// 行号窗口过程
unsafe extern "system" fn linenum_wndproc(
    hwnd: HWND,
    msg: UINT,
    w_param: WPARAM,
    l_param: LPARAM,
) -> isize {
    match msg {
        m if m == WM_PAINT_LN => {
            paint_line_numbers(hwnd);
            0
        }
        m if m == WM_ERASEBKGND_LN => {
            // 填充背景
            let hdc = w_param as HDC;
            if hdc != 0 {
                SetBkMode(hdc, OPAQUE);
                SetBkColor(hdc, get_line_num_bg_color());

                let mut rect: RECT = std::mem::zeroed();
                GetClientRect(hwnd, &mut rect);
                let bg_brush = CreateSolidBrush(get_line_num_bg_color());
                extern "system" {
                    fn FillRect(hDC: HDC, lprc: *const RECT, hbr: HBRUSH) -> i32;
                }
                FillRect(hdc, &rect, bg_brush);
                DeleteObject(bg_brush);
            }
            1
        }

        // ─── 鼠标滚轮：硬编码 EM_LINESCROLL 滚动编辑区 ───
        m if m == WM_MOUSEWHEEL => {
            if G_EDITOR_HWND != 0 {
                let delta = ((w_param >> 16) as u16 as i16) as i32;
                let lines = if delta > 0 { -3 } else { 3 };
                SendMessageW(G_EDITOR_HWND, EM_LINESCROLL, 0, lines as LPARAM);
                update_line_numbers();
            }
            0
        }
        _ => DefWindowProcW(hwnd, msg, w_param, l_param),
    }
}

/// 绘制行号
unsafe fn paint_line_numbers(linenum_hwnd: HWND) {
    extern "system" {
        fn BeginPaint(hWnd: HWND, lpPaint: *mut PAINTSTRUCT) -> HDC;
        fn EndPaint(hWnd: HWND, lpPaint: *const PAINTSTRUCT) -> BOOL;
    }
    #[repr(C)]
    struct PAINTSTRUCT {
        hdc: HDC, fErase: BOOL, rcPaint: RECT,
        fRestore: BOOL, fIncUpdate: BOOL, rgbReserved: [u8; 32],
    }

    if G_EDITOR_HWND == 0 { return; }

    let mut ps: PAINTSTRUCT = std::mem::zeroed();
    let hdc = BeginPaint(linenum_hwnd, &mut ps);

    // 背景
    let mut rect: RECT = std::mem::zeroed();
    GetClientRect(linenum_hwnd, &mut rect);
    SetBkMode(hdc, OPAQUE);
    SetBkColor(hdc, get_line_num_bg_color());

    let bg_brush = CreateSolidBrush(get_line_num_bg_color());
    extern "system" {
        fn FillRect(hDC: HDC, lprc: *const RECT, hbr: HBRUSH) -> i32;
    }
    FillRect(hdc, &rect, bg_brush);

    // 获取编辑区信息
    let total_lines = SendMessageW(G_EDITOR_HWND, EM_GETLINECOUNT, 0, 0) as i32;
    let first_visible = SendMessageW(G_EDITOR_HWND, EM_GETFIRSTVISIBLELINE, 0, 0) as i32;

    // 获取行高
    let line_height = get_line_height(hdc);

    // 设置文字颜色和对齐
    SetTextColor(hdc, get_line_num_fg_color());
    SetTextAlign(hdc, TA_RIGHT);

    // 创建字体
    if G_LINENUM_FONT == 0 {
        G_LINENUM_FONT = CreateFontW(
            -13, 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 0, 0,
            to_wide("Consolas").as_ptr(),
        );
    }
    let old_font = SelectObject(hdc, G_LINENUM_FONT);

    // 计算可见行数
    let visible_count = ((rect.bottom - rect.top) as f64 / line_height as f64).ceil() as i32 + 1;

    let mut y = rect.top;
    for i in first_visible..=(first_visible + visible_count).min(total_lines - 1) {
        let line_num_str = to_wide(&format!("{}", i + 1));
        TextOutW(hdc, rect.right - 6, y, line_num_str.as_ptr(), (i + 1).to_string().len() as i32);
        y += line_height;
    }

    SelectObject(hdc, old_font);
    DeleteObject(bg_brush);
    EndPaint(linenum_hwnd, &ps);
}

/// 获取编辑区行高
unsafe fn get_line_height(hdc: HDC) -> i32 {
    extern "system" {
        fn GetTextMetricsW(hdc: HDC, lptm: *mut TEXTMETRICW) -> BOOL;
    }
    #[repr(C)]
    struct TEXTMETRICW {
        tmHeight: i32, tmAscent: i32, tmDescent: i32, tmInternalLeading: i32,
        tmExternalLeading: i32, tmAveCharWidth: i32, tmMaxCharWidth: i32,
        tmWeight: i32, tmOverhang: i32, tmDigitizedAspectX: i32, tmDigitizedAspectY: i32,
        tmFirstChar: u16, tmLastChar: u16, tmDefaultChar: u16, tmBreakChar: u16,
        tmItalic: u8, tmUnderlined: u8, tmStruckOut: u8, tmPitchAndFamily: u8,
        tmCharSet: u8, _padding: [u8; 1],
    }
    let mut tm: TEXTMETRICW = std::mem::zeroed();
    GetTextMetricsW(hdc, &mut tm);
    tm.tmHeight + tm.tmExternalLeading
}

// ============================================================================
// ============================================================================
// 全局快捷键处理（内联于 editor_subclass_proc 以避免模块路径问题）
// ============================================================================

unsafe fn is_ctrl_down() -> bool {
    extern "system" { fn GetKeyState(nVirtKey: i32) -> i16; }
    (GetKeyState(0x11) as i32 & 0x8000) != 0
}

/// 直接在子类过程中处理所有全局快捷键，返回 true 表示已处理
unsafe fn handle_global_hotkey(hwnd: HWND, vk: u32) -> bool {
    match vk {
        0x74 => { // F5: 运行
            super::actions::action_run(); return true;
        }
        0x72 => { // F3: 查找下一个
            if is_ctrl_down() { return false; }
            super::find_replace::find_next(); return true;
        }
        0x46 if is_ctrl_down() => { // Ctrl+F: 查找
            super::find_replace::show_find(hwnd); return true;
        }
        0x53 if is_ctrl_down() => { // Ctrl+S: 保存
            super::actions::action_save_file(); return true;
        }
        0x42 if is_ctrl_down() => { // Ctrl+B: 编译
            super::actions::action_compile_native(); return true;
        }
        0x4F if is_ctrl_down() => { // Ctrl+O: 打开
            super::actions::action_open_file(); return true;
        }
        0x4E if is_ctrl_down() => { // Ctrl+N: 新建标签
            super::tabs::add_tab("untitled.klc", None);
            return true;
        }
        0x57 if is_ctrl_down() => { // Ctrl+W: 关闭标签
            super::tabs::close_current();
            return true;
        }
        0x41 if is_ctrl_down() => { // Ctrl+A: 全选
            SendMessageW(hwnd, EM_SETSEL, 0, (-1isize) as LPARAM); return true;
        }
        0x4D if is_ctrl_down() => { // Ctrl+M: 折叠
            let src = get_source_code();
            let line = get_current_line();
            let mut blocks = super::code_folding::analyze_folds(&src);
            if super::code_folding::toggle_fold(&mut blocks, line as usize) {
                let folded = super::code_folding::fold_source(&src, &blocks);
                set_source_code(&folded);
            }
            return true;
        }
        0x20 if is_ctrl_down() => { // Ctrl+Space: 提示
            let src = get_source_code();
            let line = get_current_line() as usize;
            let lines: Vec<&str> = src.lines().collect();
            if line < lines.len() {
                let prefix = lines[line].trim_end()
                    .split(|c: char| !c.is_alphanumeric() && c != '_')
                    .last().unwrap_or("");
                if prefix.len() >= 1 {
                    super::intellisense::show_hints(hwnd, prefix);
                }
            }
            return true;
        }
        _ => {}
    }
    false
}

// Rich Edit Subclass（拦截 WM_MOUSEWHEEL）
// ============================================================================

/// 编辑器子类窗口过程——滚轮同步行号 + 括号自动配对
unsafe extern "system" fn editor_subclass_proc(
    hwnd: HWND, msg: UINT, w_param: WPARAM, l_param: LPARAM,
) -> isize {
    // ★ WM_CONTEXTMENU: 不拦截，直接返回原窗口过程
    // 标准 EDIT 控件收到此消息会自动弹出原生右键菜单：
    //   撤销 / 剪切 / 复制 / 粘贴 / 删除 / 全选
    // (Ctrl+Z / Ctrl+X / Ctrl+C / Ctrl+V / Ctrl+A 均由 EDIT 内置处理)
    if msg == WM_CONTEXTMENU {
        return call_original_edit_proc(hwnd, msg, w_param, l_param);
    }

    if msg == WM_MOUSEWHEEL {
        let delta = ((w_param >> 16) as u16 as i16) as i32;
        let lines = if delta > 0 { -3 } else { 3 };
        SendMessageW(hwnd, EM_LINESCROLL, 0, lines as LPARAM);
        update_line_numbers();
        return 0;
    }

    // ─── WM_KEYDOWN: 全局快捷键 + 编辑键 ───
    if msg == 0x0100 {
        let vk = (w_param & 0xFFFF) as u32;
        // ★ 直接处理所有全局快捷键
        if handle_global_hotkey(hwnd, vk) { return 0; }
        // 静默拦截功能键(F1-F24) + Page Up/Down + Home/End + ScrollLock/Pause/CtrlBreak
        // 方向键(0x25..0x28)、Delete(0x2E)、Insert(0x2D) 不拦截，让 EDIT 控件处理
        match vk {
            0x70..=0x87 | 0x21..=0x24 | 0x90 | 0x91 | 0x13 => return 0,
            _ => {}
        }
        match vk {
            0x08 => { // VK_BACK: 配对删除
                let (start, end) = get_sel_raw(hwnd);
                if start == end && end >= 1 {
                    let text = get_text_wide(hwnd);
                    if (end as usize) <= text.len() && end >= 2 {
                        let prev = text[end as usize - 1];
                        let next = if (end as usize) < text.len() { text[end as usize] } else { 0 };
                        let is_pair = (prev == b'(' as u16 && next == b')' as u16)
                            || (prev == b'{' as u16 && next == b'}' as u16)
                            || (prev == b'[' as u16 && next == b']' as u16)
                            || (prev == b'"' as u16 && next == b'"' as u16)
                            || (prev == b'\'' as u16 && next == b'\'' as u16);
                        if is_pair {
                            let new_text = [&text[..end as usize - 1], &text[end as usize + 1..]].concat();
                            let mut nt = new_text.clone(); nt.push(0);
                            SendMessageW(hwnd, 0x000C, 0, nt.as_ptr() as LPARAM);
                            SendMessageW(hwnd, EM_SETSEL, (end - 1) as WPARAM, (end - 1) as LPARAM);
                            return 0;
                        }
                    }
                }
            }
            0x09 => { // VK_TAB: 插入4空格
                let spaces = to_wide("    ");
                SendMessageW(hwnd, EM_REPLACESEL, 1, spaces.as_ptr() as LPARAM);
                return 0;
            }
            0x0D => { // VK_RETURN: 自动缩进
                let indent = get_current_line_indent();
                let ins = format!("\r\n{}", indent);
                let wide = to_wide(&ins);
                SendMessageW(hwnd, EM_REPLACESEL, 1, wide.as_ptr() as LPARAM);
                return 0;
            }
            0xBF => { // VK_OEM_2: '/' 键
                if is_ctrl_key_down() { toggle_comment_line(hwnd); return 0; }
            }
            _ => {}
        }
    }

    // WM_CHAR: 括号/引号自动配对 + 拦截 Ctrl+字母控制字符
    if msg == 0x0102 {
        let ch = (w_param & 0xFFFF) as u32;
        // Ctrl+字母产生的 ASCII 控制字符 (1-26) → 静默丢弃，防止蜂鸣
        // 但排除 0x08(Backspace)，EDIT 控件需要它来删除字符
        if ch >= 1 && ch <= 26 && ch != 0x08 { return 0; }
        // Tab/Enter 的 WM_CHAR 已在 WM_KEYDOWN 中处理，拦截防止重复
        if ch == 9 || ch == 13 { return 0; }
        let (sel_start, sel_end) = get_sel_raw(hwnd);
        let has_selection = sel_start != sel_end;

        match ch {
            // 左括号 ( { [ → 无选区时自动补全配对
            _p @ (40 | 123 | 91) if !has_selection => {
                let close: u16 = match ch { 40 => 41, 123 => 125, _ => 93 };
                let pair = [ch as u16, close, 0u16];
                SendMessageW(hwnd, EM_REPLACESEL, 1, pair.as_ptr() as LPARAM);
                let (_, cur) = get_sel_raw(hwnd);
                SendMessageW(hwnd, EM_SETSEL, (cur - 1) as WPARAM, (cur - 1) as LPARAM);
                return 0;
            }
            // 引号 " ' → 无选区时自动补全配对
            _p @ (34 | 39) if !has_selection => {
                let pair = [ch as u16, ch as u16, 0u16];
                SendMessageW(hwnd, EM_REPLACESEL, 1, pair.as_ptr() as LPARAM);
                let (_, cur) = get_sel_raw(hwnd);
                SendMessageW(hwnd, EM_SETSEL, (cur - 1) as WPARAM, (cur - 1) as LPARAM);
                return 0;
            }
            // 右括号/引号 ) } ] " ' → 光标后字符相同则跳过，否则正常输入
            _c @ (41 | 125 | 93 | 34 | 39) if !has_selection => {
                let all = get_text_wide(hwnd);
                if (sel_end as usize) < all.len() && all[sel_end as usize] == ch as u16 {
                    SendMessageW(hwnd, EM_SETSEL, (sel_end + 1) as WPARAM, (sel_end + 1) as LPARAM);
                    return 0;
                }
            }
            _ => {}
        }
    }

    call_original_edit_proc(hwnd, msg, w_param, l_param)
}

/// 从窗口属性中取出原始 EDIT proc，调用 CallWindowProcW
unsafe fn call_original_edit_proc(hwnd: HWND, msg: UINT, w_param: WPARAM, l_param: LPARAM) -> isize {
    let prop_name: Vec<u16> = "ORIG_EDIT_PROC\0".encode_utf16().collect();
    let orig_val = GetPropW(hwnd, prop_name.as_ptr());
    if orig_val != 0 {
        let orig: Option<unsafe extern "system" fn(HWND, UINT, WPARAM, LPARAM) -> isize> 
            = std::mem::transmute(orig_val);
        CallWindowProcW(orig, hwnd, msg, w_param, l_param)
    } else {
        // 回退：标准 EDIT 默认处理
        DefWindowProcW(hwnd, msg, w_param, l_param)
    }
}

unsafe fn get_sel_raw(hwnd: HWND) -> (u32, u32) {
    let mut start: usize = 0;
    let mut end: usize = 0;
    SendMessageW(hwnd, 0x00B0, &mut start as *mut usize as WPARAM, &mut end as *mut usize as LPARAM);
    (start as u32, end as u32)
}

unsafe fn get_text_wide(hwnd: HWND) -> Vec<u16> {
    let len = SendMessageW(hwnd, 0x000E, 0, 0) as usize;
    let mut buf: Vec<u16> = vec![0; len + 2];
    SendMessageW(hwnd, 0x000D, (len + 1) as WPARAM, buf.as_mut_ptr() as LPARAM);
    buf.truncate(len);
    buf
}

unsafe fn is_ctrl_key_down() -> bool {
    extern "system" { fn GetKeyState(nVirtKey: i32) -> i16; }
    (GetKeyState(0x11) as i32 & 0x8000) != 0
}

unsafe fn toggle_comment_line(hwnd: HWND) {
    let source = get_source_code();
    let (start, _end) = get_sel_raw(hwnd);
    let pos = start as usize;

    let line_start = source[..pos].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let line_end = source[pos..].find('\n').map(|i| pos + i).unwrap_or(source.len());
    let line = &source[line_start..line_end];

    let text = get_text_wide(hwnd);
    let trimmed = line.trim_start();
    let indent_len = line.len() - trimmed.len();

    let mut new_text: Vec<u16>;
    let cursor_delta: i32;

    if trimmed.starts_with("--") {
        let after = &trimmed[2..];
        new_text = Vec::with_capacity(text.len());
        new_text.extend_from_slice(&text[..line_start]);
        for _ in 0..indent_len { new_text.push(b' ' as u16); }
        new_text.extend(after.encode_utf16());
        new_text.extend_from_slice(&text[line_end..]);
        cursor_delta = -2;
    } else {
        new_text = Vec::with_capacity(text.len() + 4);
        new_text.extend_from_slice(&text[..line_start]);
        for _ in 0..indent_len { new_text.push(b' ' as u16); }
        new_text.push(b'-' as u16);
        new_text.push(b'-' as u16);
        new_text.extend(trimmed.encode_utf16());
        new_text.extend_from_slice(&text[line_end..]);
        cursor_delta = 2;
    }

    new_text.push(0);
    SendMessageW(hwnd, 0x000C, 0, new_text.as_ptr() as LPARAM);
    let new_pos = (pos as i32 + cursor_delta).max(0) as u32;
    SendMessageW(hwnd, EM_SETSEL, new_pos as WPARAM, new_pos as LPARAM);
}

// ============================================================================
// 编辑区管理
// ============================================================================

/// 创建标准 EDIT 编辑区（替代 Rich Edit，修复大写字母 BUG）  
pub unsafe fn create_editor(parent: HWND, x: i32, y: i32, w: i32, h: i32) -> HWND {
    // 销毁旧控件
    if G_EDITOR_HWND != 0 { DestroyWindow(G_EDITOR_HWND); G_EDITOR_HWND = 0; }
    if G_LINENUM_HWND != 0 { DestroyWindow(G_LINENUM_HWND); G_LINENUM_HWND = 0; }

    register_linenum_class();

    let class_name = to_wide("EDIT");
    let style: DWORD = 0x40000000   // WS_CHILD
        | 0x10000000                // WS_VISIBLE
        | 0x00200000                // WS_VSCROLL
        | 0x04000000                // WS_CLIPSIBLINGS
        | 0x00000004                // ES_MULTILINE
        | 0x00000040                // ES_AUTOVSCROLL
        | 0x00001000                // ES_WANTRETURN
        | ES_NOHIDESEL;             // 始终显示选择

    let hwnd = CreateWindowExW(
        0, class_name.as_ptr(), std::ptr::null(), style,
        x + LINE_NUM_WIDTH, y, w - LINE_NUM_WIDTH, h,
        parent, EDITOR_CTRL_ID as isize,
        GetModuleHandleW(std::ptr::null()),
        std::ptr::null_mut(),
    );
    if hwnd == 0 { eprintln!("Error: EDIT 控件创建失败"); return 0; }

    G_EDITOR_HWND = hwnd;

    // 设置等宽字体
    let font = controls::get_mono_font();
    if font != 0 { SendMessageW(hwnd, WM_SETFONT, font as WPARAM, 1); }

    // 创建行号窗口
    let linenum_class = to_wide(LINENUM_CLASS);
    G_LINENUM_HWND = CreateWindowExW(
        0, linenum_class.as_ptr(), std::ptr::null(),
        0x40000000 | 0x10000000, x, y, LINE_NUM_WIDTH, h, // WS_CHILD | WS_VISIBLE
        parent, 0, GetModuleHandleW(std::ptr::null()), std::ptr::null_mut(),
    );

    // Subclass: 用 SetProp 保存原始 proc
    subclass_editor(hwnd);

    hwnd
}

/// 应用默认字符格式到编辑区（使用 SCF_ALL 重置全部文本格式）
/// ★ 仅重置颜色、字体、字号 —— 绝不碰 dwEffects/wWeight/粗体，
/// ★ 避免与后续语法高亮的 CFE_BOLD 冲突导致颜色丢失
unsafe fn apply_default_format(hwnd: HWND) {
    let mut cf: CHARFORMAT2W = std::mem::zeroed();
    cf.cbSize = std::mem::size_of::<CHARFORMAT2W>() as UINT;
    cf.dwMask = CFM_COLOR | CFM_FACE | CFM_SIZE; // ★ 仅这三项，不包含 CFM_BOLD
    cf.crTextColor = highlight::get_editor_fg_color();
    cf.yHeight = 14 * 20; // 14pt in twips (1pt = 20 twips)
    let face = to_wide("Consolas");
    let face_len = face.len().min(32);
    cf.szFaceName[..face_len].copy_from_slice(&face[..face_len]);

    SendMessageW(hwnd, EM_SETCHARFORMAT, SCF_ALL as WPARAM, &mut cf as *mut CHARFORMAT2W as LPARAM);
}

/// 强制设置编辑区段落为水平方向（防止竖排/RTL文字）
unsafe fn force_horizontal_layout(hwnd: HWND) {
    // 通过 EM_SETOPTIONS 清除 RTL 模式 + 设置正确选项
    SendMessageW(hwnd, EM_SETOPTIONS,
        ECOOP_SET as WPARAM, 0 as LPARAM); // 先全部重置
    SendMessageW(hwnd, EM_SETOPTIONS,
        ECOOP_OR as WPARAM,
        (ECO_WORDBREAK | ECO_NOHIDESEL) as LPARAM);
    SendMessageW(hwnd, EM_SETOPTIONS,
        ECOOP_AND as WPARAM,
        (!ECO_RTLREADING) as LPARAM); // 强制关闭RTL

    // 禁用高级排版（防止竖排变体字）
    SendMessageW(hwnd, EM_SETTYPOGRAPHYOPTIONS,
        TO_ADVANCEDTYPOGRAPHY as WPARAM, 0);

    // 通过 PARAFORMAT2 设置段落方向为左对齐
    let mut pf: PARAFORMAT2 = std::mem::zeroed();
    pf.cbSize = std::mem::size_of::<PARAFORMAT2>() as UINT;
    pf.dwMask = PFM_ALIGNMENT | PFM_DIRECTION;
    pf.wAlignment = PFA_LEFT as u16;      // 左对齐
    pf.wEffects = 0;                      // 不设置 PFE_VERTICAL

    SendMessageW(hwnd, EM_SETPARAFORMAT, 0, &mut pf as *mut PARAFORMAT2 as LPARAM);
}

/// 获取编辑区窗口句柄
pub unsafe fn get_editor_hwnd() -> HWND {
    G_EDITOR_HWND
}

/// 获取行号窗口句柄
pub unsafe fn get_linenum_hwnd() -> HWND {
    G_LINENUM_HWND
}

/// 更新 editor 模块全局句柄（切换 Tab 时由 tabs.rs 调用）
pub unsafe fn set_active_editor(edit: HWND, linenum: HWND) {
    G_EDITOR_HWND = edit;
    G_LINENUM_HWND = linenum;
}

/// 创建编辑器+行号对（不存储到全局，用于多标签）
/// ★ 创建时带 WS_VISIBLE，Z 序由 tabs::switch_to 管理（HWND_TOP/HWND_BOTTOM）
pub unsafe fn create_editor_pair(
    parent: HWND, x: i32, y: i32, w: i32, h: i32,
) -> (HWND, HWND) {
    register_linenum_class();

    let class_name = to_wide("EDIT");
    let style: DWORD = 0x40000000   // WS_CHILD
        | 0x10000000                // WS_VISIBLE
        | 0x00200000                // WS_VSCROLL
        | 0x04000000                // WS_CLIPSIBLINGS
        | 0x00000004                // ES_MULTILINE
        | 0x00000040                // ES_AUTOVSCROLL
        | 0x00001000                // ES_WANTRETURN
        | ES_NOHIDESEL;

    let hwnd = CreateWindowExW(
        0, class_name.as_ptr(), std::ptr::null(), style,
        x + LINE_NUM_WIDTH, y, w.saturating_sub(LINE_NUM_WIDTH), h,
        parent, 0,
        GetModuleHandleW(std::ptr::null()),
        std::ptr::null_mut(),
    );
    if hwnd == 0 { return (0, 0); }

    let linenum_class = to_wide(LINENUM_CLASS);
    let linenum = CreateWindowExW(
        0, linenum_class.as_ptr(), std::ptr::null(),
        0x40000000 | 0x10000000, // WS_CHILD | WS_VISIBLE
        x, y, LINE_NUM_WIDTH, h,
        parent, 0, GetModuleHandleW(std::ptr::null()), std::ptr::null_mut(),
    );

    let font = controls::get_mono_font();
    if font != 0 { SendMessageW(hwnd, WM_SETFONT, font as WPARAM, 1); }

    subclass_editor(hwnd);

    (hwnd, linenum)
}

/// 销毁编辑器+行号对
pub unsafe fn destroy_editor_pair(edit: HWND, linenum: HWND) {
    if edit != 0 {
        // 清除窗口属性
        let prop_name: Vec<u16> = "ORIG_EDIT_PROC\0".encode_utf16().collect();
        extern "system" { fn RemovePropW(hWnd: HWND, lpString: *const WCHAR) -> isize; }
        RemovePropW(edit, prop_name.as_ptr());
        DestroyWindow(edit);
    }
    if linenum != 0 { DestroyWindow(linenum); }
}

/// 调整编辑器+行号对的位置和大小
pub unsafe fn resize_editor_pair(edit: HWND, linenum: HWND, x: i32, y: i32, w: i32, h: i32) {
    if edit != 0 {
        MoveWindow(edit, x + LINE_NUM_WIDTH, y, w.saturating_sub(LINE_NUM_WIDTH), h, 1);
        InvalidateRect(edit, std::ptr::null(), 1);
        UpdateWindow(edit);
    }
    if linenum != 0 {
        MoveWindow(linenum, x, y, LINE_NUM_WIDTH, h, 1);
        InvalidateRect(linenum, std::ptr::null(), 1);
        UpdateWindow(linenum);
    }
}

/// 获取编辑区全部源码文本
pub unsafe fn get_source_code() -> String {
    if G_EDITOR_HWND == 0 {
        return String::new();
    }
    // 使用标准 GetText 方式
    let len = SendMessageW(G_EDITOR_HWND, WM_GETTEXTLENGTH, 0, 0) as usize;
    if len == 0 { return String::new(); }

    let mut buf: Vec<u16> = vec![0; len + 2];
    SendMessageW(G_EDITOR_HWND, WM_GETTEXT, (len + 1) as WPARAM, buf.as_mut_ptr() as LPARAM);
    let actual = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..actual])
}

/// ★ 步骤3：过滤文本中的 Unicode 控制字符（RLM/LRM 等）
/// 仅保留 ASCII 32-126 + 换行符(\r\n) + Unicode，清除 RTL 控制字符
/// ★ 必须保留空终止符 \0，否则传给 Win32 API 的字符串无终止符 → 崩溃
fn filter_control_characters(text_wide: &[u16]) -> Vec<u16> {
    let mut result: Vec<u16> = text_wide.iter()
        .filter(|&&c| c == 0 || c == 10 || c == 13 || (c >= 32 && c <= 126) || c > 127)
        .copied()
        .collect();
    // 确保以 \0 结尾（防止过滤后丢失终止符导致访问冲突）
    if !result.is_empty() && *result.last().unwrap() != 0 {
        result.push(0);
    }
    result
}

/// 设置编辑区内容
pub unsafe fn set_source_code(text: &str) {
    if G_EDITOR_HWND == 0 { return; }

    extern "system" { fn SetFocus(hWnd: HWND) -> HWND; }
    SetFocus(G_EDITOR_HWND);

    // 过滤控制字符后通过 WM_SETTEXT 设置
    let text_wide = to_wide(text);
    let filtered = filter_control_characters(&text_wide);
    SendMessageW(G_EDITOR_HWND, WM_SETTEXT, 0, filtered.as_ptr() as LPARAM);
    SendMessageW(G_EDITOR_HWND, EM_SETSEL, 0, 0);
}

/// 调整编辑区和行号区域的位置和大小
/// ★ 步骤5：修复 WM_SIZE 布局 + 强制重绘
pub unsafe fn resize_editor(x: i32, y: i32, w: i32, h: i32) {
    if G_EDITOR_HWND == 0 { return; }

    controls::resize_control(G_EDITOR_HWND, x + LINE_NUM_WIDTH, y, w - LINE_NUM_WIDTH, h, true);
    if G_LINENUM_HWND != 0 {
        controls::resize_control(G_LINENUM_HWND, x, y, LINE_NUM_WIDTH, h, true);
    }

    // ★ 强制重绘编辑区和行号
    InvalidateRect(G_EDITOR_HWND, std::ptr::null(), 1);
    UpdateWindow(G_EDITOR_HWND);
    InvalidateRect(G_LINENUM_HWND, std::ptr::null(), 1);
    UpdateWindow(G_LINENUM_HWND);
}

/// 检查编辑区是否有未保存修改
pub unsafe fn is_modified() -> bool {
    if G_EDITOR_HWND == 0 { return false; }
    SendMessageW(G_EDITOR_HWND, EM_GETMODIFY, 0, 0) != 0
}

/// 设置编辑区未保存修改标志
pub unsafe fn set_modified(modified: bool) {
    if G_EDITOR_HWND == 0 { return; }
    SendMessageW(G_EDITOR_HWND, EM_SETMODIFY, if modified { 1 } else { 0 }, 0);
}

/// 获取编辑区当前光标位置的行号
pub unsafe fn get_current_line() -> i32 {
    if G_EDITOR_HWND == 0 { return 0; }
    let (start, _) = get_selection_range();
    // 找到 start 所在行
    let mut line = 0;
    let source = get_source_code();
    for (i, ch) in source.chars().enumerate() {
        if i == start { break; }
        if ch == '\n' { line += 1; }
    }
    line
}

/// 获取选区范围
unsafe fn get_selection_range() -> (usize, usize) {
    if G_EDITOR_HWND == 0 { return (0, 0); }
    let start = SendMessageW(G_EDITOR_HWND, EM_GETSEL, 0, 0);
    let end = SendMessageW(G_EDITOR_HWND, EM_GETSEL, 0, 0);
    // EM_GETSEL 返回 (start, end) 的高低位
    // 但实际 SendMessage 的 lParam 才是 end
    // 使用更精确的方式
    let sel = (start as u64) | ((end as u64) << 32);
    (sel as usize, (sel >> 32) as usize)
}

/// 获取当前行的缩进
pub unsafe fn get_current_line_indent() -> String {
    let source = get_source_code();
    if let Some(cursor_pos) = get_cursor_pos(&source) {
        // 找到当前行开头
        let line_start = source[..cursor_pos].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let line_end = source[cursor_pos..].find('\n').map(|i| cursor_pos + i).unwrap_or(source.len());
        let line = &source[line_start..line_end];

        // 提取前导空格
        let indent: String = line.chars().take_while(|&c| c == ' ' || c == '\t').collect();
        indent
    } else {
        String::new()
    }
}

/// 获取光标在字符中的位置
unsafe fn get_cursor_pos(source: &str) -> Option<usize> {
    let (start, _) = get_selection_range();
    // 验证 start 在范围内
    let mut pos = 0;
    for (i, _ch) in source.chars().enumerate() {
        if i == start { pos = i; break; }
    }
    Some(pos)
}

/// 获取当前行末尾位置
pub unsafe fn get_current_line_end() -> usize {
    let source = get_source_code();
    if let Some(cursor_pos) = get_cursor_pos(&source) {
        source[cursor_pos..].find('\n').map(|i| cursor_pos + i).unwrap_or(source.len())
    } else {
        source.len()
    }
}

/// 在光标处插入文本
pub unsafe fn insert_text_at_cursor(text: &str) {
    if G_EDITOR_HWND == 0 { return; }
    let text_wide = to_wide(text);
    // 确保替换选区为 0 长度（在光标位置插入）
    SendMessageW(G_EDITOR_HWND, EM_REPLACESEL, 0, text_wide.as_ptr() as LPARAM);
}

/// 请求语法高亮（带防抖）
pub unsafe fn request_highlight() {
    if G_EDITOR_HWND == 0 { return; }
    let source = get_source_code();
    highlight::highlight_editor(G_EDITOR_HWND, &source);
    update_line_numbers();
}
pub unsafe fn update_line_numbers() {
    if G_LINENUM_HWND != 0 {
        InvalidateRect(G_LINENUM_HWND, std::ptr::null(), 1);
    }
}

/// 应用主题（更新背景色和前景色）
pub unsafe fn apply_theme() {
    if G_EDITOR_HWND == 0 { return; }

    // 更新行号背景色
    if G_LINENUM_HWND != 0 {
        extern "system" {
            fn SetClassLongPtrW(hWnd: HWND, nIndex: i32, dwNewLong: isize) -> isize;
        }
        let h_brush = CreateSolidBrush(get_line_num_bg_color());
        SetClassLongPtrW(G_LINENUM_HWND, -10, h_brush as isize); // GCLP_HBRBACKGROUND = -10
        InvalidateRect(G_LINENUM_HWND, std::ptr::null(), 1);
    }

    // 重新高亮
    request_highlight();
}

/// 处理编辑区 Enter 键自动缩进
pub unsafe fn handle_enter_indent() {
    let indent = get_current_line_indent();
    insert_text_at_cursor(&format!("\r\n{}", indent));
}

/// 处理 } 键自动减少缩进
pub unsafe fn handle_close_brace() {
    insert_text_at_cursor("}");
}

/// 释放资源
pub unsafe fn release_editor_resources() {
    if G_LINENUM_FONT != 0 {
        DeleteObject(G_LINENUM_FONT);
        G_LINENUM_FONT = 0;
    }
    G_EDITOR_HWND = 0;
    G_LINENUM_HWND = 0;
}

// ============================================================================
// 用于 WM_CTLCOLOREDIT — 判断是否是编辑区
// ============================================================================

/// Rich Edit 不使用 WM_CTLCOLOREDIT，它自行管理颜色
/// 但我们需要在 window_proc 中处理编辑区的背景色
/// 所以提供一个判断函数
pub unsafe fn is_editor_hwnd(hwnd: HWND) -> bool {
    hwnd == G_EDITOR_HWND || hwnd == G_LINENUM_HWND
}

/// 对任意 EDIT 控件应用子类化（用 SetProp 为每个编辑器独立保存原始 proc）
pub unsafe fn subclass_editor(hwnd: HWND) {
    let orig = SetWindowLongPtrW(hwnd, GWLP_WNDPROC, editor_subclass_proc as *const () as isize);
    // 用窗口属性存储原始 proc，避免多编辑器间覆盖
    let prop_name: Vec<u16> = "ORIG_EDIT_PROC\0".encode_utf16().collect();
    SetPropW(hwnd, prop_name.as_ptr(), orig as isize);
}

/// 处理鼠标滚轮滚动编辑区
/// wheel_delta: 滚轮增量 (WPARAM 的高16位，正=向上/远离用户，负=向下/朝向用户)
pub unsafe fn handle_mouse_wheel(wheel_delta: i32) -> bool {
    if G_EDITOR_HWND == 0 { return false; }

    // WHEEL_DELTA = 120，每 120 单位滚动 3 行
    const WHEEL_DELTA: i32 = 120;
    let lines_to_scroll = -(wheel_delta / WHEEL_DELTA) * 3;
    if lines_to_scroll != 0 {
        // EM_LINESCROLL: wParam = 水平字符数, lParam = 垂直行数
        SendMessageW(G_EDITOR_HWND, EM_LINESCROLL, 0, lines_to_scroll as LPARAM);
        update_line_numbers();
        true
    } else {
        false
    }
}
