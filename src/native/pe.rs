//! KLC 原生 PE 文件生成器 — 阶段一：最小 PE 文件生成
//!
//! 本模块从字节层面手动构造 Windows x86_64 PE (Portable Executable) 格式的 EXE 文件。
//! 完全使用 Rust 标准库，不依赖任何第三方 crate 或外部工具链。
//!
//! ## PE 文件布局（阶段一）
//!
//! ```text
//! 文件偏移    大小      内容
//! ─────────────────────────────────────────────
//! 0x0000      64       DOS Header (IMAGE_DOS_HEADER)
//! 0x0040      64       DOS Stub (标准 "This program cannot be run in DOS mode." 消息)
//! 0x0080       4       PE 签名 "PE\0\0"
//! 0x0084      20       COFF File Header (IMAGE_FILE_HEADER)
//! 0x0098     112       Optional Header PE32+ (IMAGE_OPTIONAL_HEADER64)
//! 0x0108     128       数据目录表 (16 个 IMAGE_DATA_DIRECTORY, 全零占位)
//! 0x0188      40       .text 节表头 (IMAGE_SECTION_HEADER)
//! 0x01B0      80       对齐填充 (填充到 FileAlignment = 0x200)
//! 0x0200     N+pad    .text 节数据 (机器码 + 对齐填充)
//! ─────────────────────────────────────────────
//! 典型总大小: ~1024 字节 (0x400)
//! ```
//!
//! ## 默认值
//! - 映像基地址 (ImageBase):    0x00000000_00400000
//! - 代码节 RVA:                0x1000
//! - 文件对齐 (FileAlignment):  0x200 (512 字节)
//! - 节对齐 (SectionAlignment): 0x1000 (4096 字节)
//! - 子系统:                    3 (Windows Console)
//! - 机器类型:                  0x8664 (x86_64 / AMD64)

use std::mem;

// ============================================================================
// PE 格式结构体定义
// ============================================================================
// 所有结构体使用 #[repr(C, packed)] 确保内存布局与 PE 规范完全一致
// 无任何填充字节，字段顺序和大小严格匹配 Windows PE 定义

/// DOS 头 — PE 文件的前 64 字节，以 "MZ" 魔数开头
///
/// 此结构体是历史遗留（兼容 DOS MZ 可执行文件格式），
/// 在现代 Windows 中只用到两个关键字段：
/// - `e_magic`: 必须为 0x5A4D（"MZ" 小端序）
/// - `e_lfanew`: 指向 PE 签名（"PE\0\0"）的文件偏移
#[repr(C, packed)]
#[derive(Debug, Clone)]
pub struct DosHeader {
    /// 魔数 — 固定为 0x5A4D ("MZ")，标识这是一个 MZ 格式的可执行文件
    pub e_magic: u16,
    /// 文件最后一页的字节数（历史字段，我们的生成器填入 0x0090）
    pub e_cblp: u16,
    /// 文件中的页数（历史字段）
    pub e_cp: u16,
    /// 重定位表项数（历史字段）
    pub e_crlc: u16,
    /// 以段落(16字节)计的文件头大小（历史字段）
    pub e_cparhdr: u16,
    /// 最小额外内存需求（段落数，历史字段）
    pub e_minalloc: u16,
    /// 最大额外内存需求（段落数，历史字段）
    pub e_maxalloc: u16,
    /// 初始 SS 寄存器值（历史字段）
    pub e_ss: u16,
    /// 初始 SP 寄存器值（历史字段）
    pub e_sp: u16,
    /// 校验和（历史字段）
    pub e_csum: u16,
    /// 初始 IP 寄存器值（历史字段）
    pub e_ip: u16,
    /// 初始 CS 寄存器值（历史字段）
    pub e_cs: u16,
    /// 重定位表文件偏移（历史字段）
    pub e_lfarlc: u16,
    /// 覆盖号（历史字段）
    pub e_ovno: u16,
    /// 保留字 [4]
    pub e_res: [u16; 4],
    /// OEM 标识符（历史字段）
    pub e_oemid: u16,
    /// OEM 信息（历史字段）
    pub e_oeminfo: u16,
    /// 保留字 [10]
    pub e_res2: [u16; 10],
    /// ★ 关键字段 ★ PE 签名 "PE\0\0" 的文件偏移地址
    /// 我们的生成器固定设为 0x80 (128)
    pub e_lfanew: i32,
}

impl DosHeader {
    /// 创建标准 DOS 头，所有历史字段填入典型值，e_lfanew = 0x80
    pub fn standard() -> Self {
        DosHeader {
            e_magic:    0x5A4D,         // "MZ"
            e_cblp:     0x0090,         // 典型值
            e_cp:       0x0003,         // 典型值
            e_crlc:     0x0000,
            e_cparhdr:  0x0004,         // 典型值
            e_minalloc: 0x0000,
            e_maxalloc: 0xFFFF,
            e_ss:       0x0000,
            e_sp:       0x00B8,         // 典型值
            e_csum:     0x0000,
            e_ip:       0x0000,
            e_cs:       0x0000,
            e_lfarlc:   0x0040,
            e_ovno:     0x0000,
            e_res:      [0; 4],
            e_oemid:    0x0000,
            e_oeminfo:  0x0000,
            e_res2:     [0; 10],
            e_lfanew:   0x0000_0080,    // PE 签名在文件偏移 128 处
        }
    }
}

/// PE 签名 — 固定 4 字节 "PE\0\0"
pub const PE_SIGNATURE: u32 = 0x0000_4550;  // "PE\0\0" 小端序

/// COFF 文件头 — PE 签名之后，描述目标机器、节数量等信息
///
/// 大小: 20 字节
#[repr(C, packed)]
#[derive(Debug, Clone)]
pub struct CoffHeader {
    /// 目标机器类型 — 0x8664 = AMD64 / x86_64
    pub machine: u16,
    /// 节（Section）的数量 — 阶段一只有 .text 节，所以为 1
    pub number_of_sections: u16,
    /// 时间戳 — 设为 0 表示不使用
    pub time_date_stamp: u32,
    /// 符号表指针 — 我们不使用符号表，设为 0
    pub pointer_to_symbol_table: u32,
    /// 符号数量 — 设为 0
    pub number_of_symbols: u32,
    /// ★ 可选头大小 — PE32+ 可选头 + 数据目录 = 112 + 16×8 = 240 (0xF0)
    pub size_of_optional_header: u16,
    /// 文件特征标志:
    ///   0x0002 = IMAGE_FILE_EXECUTABLE_IMAGE (可执行)
    ///   0x0020 = IMAGE_FILE_LARGE_ADDRESS_AWARE (支持 >2GB 地址空间)
    pub characteristics: u16,
}

impl CoffHeader {
    /// 创建 x86_64 可执行文件的 COFF 头
    pub fn standard_x64() -> Self {
        CoffHeader {
            machine:                   0x8664,   // AMD64
            number_of_sections:        1,         // 只有 .text
            time_date_stamp:           0,
            pointer_to_symbol_table:   0,
            number_of_symbols:         0,
            size_of_optional_header:   0x00F0,   // 240 = 112 + 128
            characteristics:           0x0022,   // EXECUTABLE | LARGE_ADDRESS_AWARE
        }
    }
}

/// PE32+ 可选头 — x86_64 PE 文件的核心配置
///
/// 大小: 112 字节（不含可变数据目录数组，数据目录跟随在可选头之后）
#[repr(C, packed)]
#[derive(Debug, Clone)]
pub struct OptionalHeader64 {
    /// 魔数 — 0x020B 表示 PE32+ (64位)
    pub magic: u16,
    /// 主链接器版本号
    pub major_linker_version: u8,
    /// 次链接器版本号
    pub minor_linker_version: u8,
    /// 所有代码节的总大小（文件中，已按文件对齐补齐）
    pub size_of_code: u32,
    /// 所有已初始化数据节的总大小
    pub size_of_initialized_data: u32,
    /// 所有未初始化数据节的总大小
    pub size_of_uninitialized_data: u32,
    /// ★ 程序入口点 RVA — 相对于映像基地址的虚拟地址
    pub address_of_entry_point: u32,
    /// 代码起始 RVA
    pub base_of_code: u32,
    /// ★ 映像基地址 — 程序加载的首选虚拟地址 (Windows x64 默认 0x400000)
    /// PE32 中此字段为 u32，PE32+ 中为 u64
    pub image_base: u64,
    /// ★ 节在内存中的对齐粒度 — 4096 字节 (0x1000)
    pub section_alignment: u32,
    /// ★ 节在文件中的对齐粒度 — 512 字节 (0x200)
    pub file_alignment: u32,
    /// 主操作系统版本号
    pub major_os_version: u16,
    /// 次操作系统版本号
    pub minor_os_version: u16,
    /// 主映像版本号
    pub major_image_version: u16,
    /// 次映像版本号
    pub minor_image_version: u16,
    /// 主子系统版本号 — 设为 6 (Windows Vista+)
    pub major_subsystem_version: u16,
    /// 次子系统版本号
    pub minor_subsystem_version: u16,
    /// Win32 版本值 — 保留，设为 0
    pub win32_version_value: u32,
    /// ★ 映像总大小 — 所有节在内存中的总大小（含头，按节对齐补齐）
    pub size_of_image: u32,
    /// ★ 头总大小 — DOS头 + PE签名 + COFF头 + 可选头 + 数据目录 + 节表头，按文件对齐补齐
    pub size_of_headers: u32,
    /// 映像校验和 — 大多数情况下可为 0
    pub checksum: u32,
    /// ★ 子系统 — 3 = IMAGE_SUBSYSTEM_WINDOWS_CUI (控制台应用)
    pub subsystem: u16,
    /// DLL 特征标志
    pub dll_characteristics: u16,
    /// 栈保留大小 — 1 MB (0x100000)
    pub size_of_stack_reserve: u64,
    /// 栈初始提交大小 — 4 KB (0x1000)
    pub size_of_stack_commit: u64,
    /// 堆保留大小 — 1 MB (0x100000)
    pub size_of_heap_reserve: u64,
    /// 堆初始提交大小 — 4 KB (0x1000)
    pub size_of_heap_commit: u64,
    /// 加载器标志 — 保留，设为 0
    pub loader_flags: u32,
    /// ★ 数据目录项数量 — 固定为 16 (IMAGE_NUMBEROF_DIRECTORY_ENTRIES)
    pub number_of_rva_and_sizes: u32,
}

impl OptionalHeader64 {
    /// 创建阶段一的控制台 x86_64 PE 可选头
    ///
    /// # 参数
    /// * `entry_point` - 程序入口点 RVA (默认 0x1000)
    /// * `code_size` - 代码节在文件中的对齐后大小
    /// * `image_base` - 映像基地址 (默认 0x400000)
    pub fn standard_console(entry_point: u32, code_size: u32, image_base: u64) -> Self {
        OptionalHeader64 {
            magic:                      0x020B,      // PE32+
            major_linker_version:       1,
            minor_linker_version:       0,
            size_of_code:               code_size,   // 代码节文件大小
            size_of_initialized_data:   0,
            size_of_uninitialized_data: 0,
            address_of_entry_point:     entry_point, // 入口 RVA
            base_of_code:               0x1000,      // 代码节 RVA
            image_base:                 image_base,
            section_alignment:          0x1000,      // 4KB
            file_alignment:             0x0200,      // 512B
            major_os_version:           6,           // Windows Vista+
            minor_os_version:           0,
            major_image_version:        0,
            minor_image_version:        0,
            major_subsystem_version:    6,
            minor_subsystem_version:    0,
            win32_version_value:        0,
            size_of_image:              (0x1000 + ((code_size.max(1) + 0xFFF) & !0xFFF)), // 头 + 代码节（内存对齐）
            size_of_headers:            0x0200,      // 512B（文件对齐）
            checksum:                   0,
            subsystem:                  0x0003,      // WINDOWS_CUI
            dll_characteristics:        0x0100,      // NX_COMPAT only (no ASLR for pointer simplicity)
            size_of_stack_reserve:      0x0010_0000, // 1MB
            size_of_stack_commit:       0x0000_1000, // 4KB
            size_of_heap_reserve:       0x0010_0000, // 1MB
            size_of_heap_commit:        0x0000_1000, // 4KB
            loader_flags:               0,
            number_of_rva_and_sizes:    16,
        }
    }
}

/// 数据目录项 — 指向特定 PE 数据结构的 RVA 和大小
///
/// 每个条目 8 字节，共 16 个条目，总共 128 字节。
/// 索引含义：
///   0: 导出表  1: 导入表  2: 资源表  3: 异常表
///   4: 安全表  5: 重定位表 6: 调试表 7: 架构表
///   8: 全局指针 9: TLS表  10: 加载配置 11: 绑定导入
///  12: IAT    13: 延迟导入 14: COM描述符 15: 保留
///
/// 阶段一所有目录项填充为 0（无导入/导出等）。
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct DataDirectory {
    /// 目录数据的起始 RVA
    pub virtual_address: u32,
    /// 目录数据的大小（字节）
    pub size: u32,
}

impl DataDirectory {
    /// 创建空的（全零）数据目录项
    pub const fn empty() -> Self {
        DataDirectory {
            virtual_address: 0,
            size: 0,
        }
    }
}

/// 节表头 (Section Header) — 描述 PE 文件中每个节的内存布局
///
/// 大小: 40 字节。
/// 阶段一只有一个 .text（代码）节。
#[repr(C, packed)]
#[derive(Debug, Clone)]
pub struct SectionHeader {
    /// 节名 — 8 字节 (UTF-8, 不足 8 字节用 \0 填充)
    pub name: [u8; 8],
    /// 节在内存中的大小（未按节对齐的原始大小）
    pub virtual_size: u32,
    /// 节的起始 RVA（按节对齐）
    pub virtual_address: u32,
    /// 节在文件中的原始数据大小（按文件对齐补齐）
    pub size_of_raw_data: u32,
    /// 节原始数据在文件中的偏移（按文件对齐）
    pub pointer_to_raw_data: u32,
    /// 重定位项指针（不使用，设为 0）
    pub pointer_to_relocations: u32,
    /// 行号表指针（不使用，设为 0）
    pub pointer_to_linenumbers: u32,
    /// 重定位项数量（设为 0）
    pub number_of_relocations: u16,
    /// 行号表项数量（设为 0）
    pub number_of_linenumbers: u16,
    /// ★ 节特征标志:
    ///   0x60000020 = IMAGE_SCN_CNT_CODE (含代码)
    ///              | IMAGE_SCN_MEM_EXECUTE (可执行)
    ///              | IMAGE_SCN_MEM_READ (可读)
    pub characteristics: u32,
}

impl SectionHeader {
    /// 创建 .text 代码节的节表头
    ///
    /// # 参数
    /// * `code_bytes_len` - 实际机器码字节数
    /// * `code_rva` - 代码节起始 RVA (默认 0x1000)
    /// * `file_alignment` - 文件对齐粒度
    /// * `raw_offset` - 代码节数据在文件中的起始偏移
    pub fn text_section(code_bytes_len: u32, code_rva: u32, file_alignment: u32, raw_offset: u32) -> Self {
        // 将原始数据大小向上取整到文件对齐
        let raw_size = (code_bytes_len + file_alignment - 1) & !(file_alignment - 1);

        SectionHeader {
            name:                       *b".text\0\0\0",   // ".text" + 3 个空字节
            virtual_size:               code_bytes_len,     // 实际代码大小
            virtual_address:            code_rva,           // RVA 起始地址
            size_of_raw_data:           raw_size,           // 文件中对齐后大小
            pointer_to_raw_data:        raw_offset,         // 文件中的偏移
            pointer_to_relocations:     0,
            pointer_to_linenumbers:     0,
            number_of_relocations:      0,
            number_of_linenumbers:      0,
            characteristics:            0x6000_0020,       // CODE | EXECUTE | READ
        }
    }

    /// 创建 .idata 导入数据节表头
    ///
    /// 节特性: INITIALIZED_DATA | MEM_READ | MEM_WRITE = 0xC0000040
    /// 必须可写 — Windows 加载器会将已解析的函数地址写入 IAT。
    pub fn idata_section(data_len: u32, idata_rva: u32, file_alignment: u32, raw_offset: u32) -> Self {
        let raw_size = (data_len + file_alignment - 1) & !(file_alignment - 1);

        SectionHeader {
            name:                       *b".idata\0\0",    // ".idata" + 2 个空字节
            virtual_size:               data_len,
            virtual_address:            idata_rva,
            size_of_raw_data:           raw_size,
            pointer_to_raw_data:        raw_offset,
            pointer_to_relocations:     0,
            pointer_to_linenumbers:     0,
            number_of_relocations:      0,
            number_of_linenumbers:      0,
            characteristics:            0xC000_0040,       // INITIALIZED_DATA | READ | WRITE
        }
    }
}

// 公开 .idata 节默认 RVA
pub const IDATA_RVA: u32 = 0x2000;

// ============================================================================
// 常量定义
// ============================================================================

/// 文件对齐粒度 — 文件中的各区块起始偏移必须是此值的倍数
const FILE_ALIGNMENT: u32 = 0x200;

/// 代码节默认 RVA
pub const CODE_RVA: u32 = 0x1000;

/// PE 签名在文件中的偏移（DOS头 64字节 + DOS Stub 64字节 = 128 = 0x80）
const PE_SIGNATURE_OFFSET: u32 = 0x80;

/// 节数据在文件中的起始偏移（头部填充到 FILE_ALIGNMENT 后）
const SECTION_DATA_OFFSET: u32 = FILE_ALIGNMENT;

/// DOS Stub 内容 — 在 DOS 环境下运行时会显示此消息
const DOS_STUB: &[u8] = &[
    // 16位 DOS 程序: 打印消息并退出
    0x0E, 0x1F, 0xBA, 0x0E, 0x00, 0xB4, 0x09, 0xCD,
    0x21, 0xB8, 0x01, 0x4C, 0xCD, 0x21,
    // 消息文本: "This program cannot be run in DOS mode.\r\r\n$"
    0x54, 0x68, 0x69, 0x73, 0x20, 0x70, 0x72, 0x6F,
    0x67, 0x72, 0x61, 0x6D, 0x20, 0x63, 0x61, 0x6E,
    0x6E, 0x6F, 0x74, 0x20, 0x62, 0x65, 0x20, 0x72,
    0x75, 0x6E, 0x20, 0x69, 0x6E, 0x20, 0x44, 0x4F,
    0x53, 0x20, 0x6D, 0x6F, 0x64, 0x65, 0x2E, 0x0D,
    0x0D, 0x0A, 0x24, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00,  // 末尾填充到 64 字节
];

// ============================================================================
// PE 构建器
// ============================================================================

/// PE 文件构建器 — 逐步组装 PE 文件各组件，最终生成完整的二进制 PE 文件
///
/// # 使用示例
/// ```ignore
/// let mut builder = PeBuilder::new();
/// builder.add_code(&[0x31, 0xC0, 0xC3]);  // xor eax, eax; ret
/// builder.set_entry_point(0x1000);
/// let pe_bytes = builder.build();
/// std::fs::write("output.exe", &pe_bytes).unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct PeBuilder {
    /// 要放入 .text 节的机器码字节
    code: Vec<u8>,
    /// 程序入口点 RVA（可选，默认 0x1000，即 .text 节起始处）
    entry_point: u32,
    /// 映像基地址（默认 0x400000）
    image_base: u64,
    /// 代码节 RVA（默认 0x1000）
    code_rva: u32,
    /// 导入表数据 (阶段三): (二进制数据, IDT_RVA, IDT_大小)
    import_data: Option<Vec<u8>>,
    import_rva: u32,
    import_size: u32,
    /// 导入节 RVA（默认 .text 之后: 0x2000）
    idata_rva: u32,
}

impl PeBuilder {
    /// 创建新的 PE 构建器，所有参数使用标准默认值
    ///
    /// 默认值:
    /// - 入口点 RVA: 0x1000
    /// - 映像基地址: 0x400000
    /// - 代码节 RVA: 0x1000
    pub fn new() -> Self {
        PeBuilder {
            code: Vec::new(),
            entry_point: CODE_RVA,
            image_base: 0x00000000_00400000,
            code_rva: CODE_RVA,
            import_data: None,
            import_rva: 0,
            import_size: 0,
            idata_rva: 0x2000,
        }
    }

    /// 添加要放入 .text 代码节的机器码
    ///
    /// 可以多次调用，每次追加到已有代码之后。
    ///
    /// # 参数
    /// * `code` - 机器码字节切片（如 `xor eax, eax` → `[0x31, 0xC0]`）
    pub fn add_code(&mut self, code: &[u8]) {
        self.code.extend_from_slice(code);
    }

    /// 设置程序入口点 RVA
    ///
    /// # 参数
    /// * `entry_point` - 入口点相对于映像基地址的虚拟地址（如 0x1000）
    pub fn set_entry_point(&mut self, entry_point: u32) {
        self.entry_point = entry_point;
    }

    /// 添加导入表数据 (阶段三)
    ///
    /// # 参数
    /// * `import_data` - 导入表二进制数据 (由 ImportTableBuilder::build 生成)
    /// * `import_rva` - 导入目录表 (IDT) 的 RVA
    /// * `import_size` - 导入目录表大小
    pub fn add_import_table(&mut self, import_data: &[u8], import_rva: u32, import_size: u32) {
        self.import_data = Some(import_data.to_vec());
        self.import_rva = import_rva;
        self.import_size = import_size;
    }

    /// 生成完整的 PE 文件二进制数据
    ///
    /// 按 PE 规范组装所有头部和节数据，返回可直接写入磁盘的字节数组。
    ///
    /// # 返回值
    /// 完整的 PE 文件二进制数据 (`Vec<u8>`)
    pub fn build(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(2048);
        let has_imports = self.import_data.is_some();
        let num_sections = if has_imports { 2u16 } else { 1u16 };

        // ---- 1. DOS Header (64 字节) ----
        self.write_struct(&mut buf, &DosHeader::standard());

        // ---- 2. DOS Stub (64 字节) ----
        let stub_size = PE_SIGNATURE_OFFSET as usize - mem::size_of::<DosHeader>();
        assert!(DOS_STUB.len() <= stub_size, "DOS Stub 长度超过预留空间");
        buf.extend_from_slice(DOS_STUB);
        while buf.len() < PE_SIGNATURE_OFFSET as usize {
            buf.push(0);
        }

        // ---- 3. PE 签名 "PE\0\0" (4 字节) ----
        buf.extend_from_slice(&PE_SIGNATURE.to_le_bytes());

        // ---- 4. COFF 文件头 (20 字节) ----
        let mut coff = CoffHeader::standard_x64();
        coff.number_of_sections = num_sections;
        self.write_struct(&mut buf, &coff);

        // ---- 5. 可选头 PE32+ (112 字节) ----
        let code_size = self.code.len() as u32;
        let file_aligned_code_size = (code_size + FILE_ALIGNMENT - 1) & !(FILE_ALIGNMENT - 1);
        let import_file_size = if has_imports {
            let isize = self.import_data.as_ref().unwrap().len() as u32;
            (isize + FILE_ALIGNMENT - 1) & !(FILE_ALIGNMENT - 1)
        } else {
            0
        };
        // SizeOfImage = headers(0x1000) + .text(0x1000) + [.idata(0x1000)]
        let size_of_image = if has_imports { 0x3000u32 } else { 0x2000u32 };
        let mut opt_header = OptionalHeader64::standard_console(self.entry_point, file_aligned_code_size, self.image_base);
        opt_header.size_of_image = size_of_image;
        self.write_struct(&mut buf, &opt_header);

        // ---- 6. 数据目录表 (16 × 8 = 128 字节) ----
        // 目录索引 1 = 导入表
        for i in 0..16 {
            if i == 1 && has_imports {
                // IMAGE_DIRECTORY_ENTRY_IMPORT = 1
                let dd = DataDirectory {
                    virtual_address: self.import_rva,
                    size: self.import_size,
                };
                self.write_struct(&mut buf, &dd);
            } else {
                self.write_struct(&mut buf, &DataDirectory::empty());
            }
        }

        // ---- 7. 节表头 ----
        // .text 节
        let text_section = SectionHeader::text_section(
            code_size,
            self.code_rva,
            FILE_ALIGNMENT,
            SECTION_DATA_OFFSET,
        );
        self.write_struct(&mut buf, &text_section);

        if has_imports {
            // .idata 节: INITIALIZED_DATA | MEM_READ | MEM_WRITE
            // IAT 必须可写 (加载器需要写入已解析的函数地址)
            let idata_raw_offset = SECTION_DATA_OFFSET + file_aligned_code_size;
            let idata_section = SectionHeader::idata_section(
                self.import_data.as_ref().unwrap().len() as u32,
                self.idata_rva,
                FILE_ALIGNMENT,
                idata_raw_offset,
            );
            self.write_struct(&mut buf, &idata_section);
        }

        // ---- 8. 头部对齐填充 ----
        while (buf.len() as u32) < SECTION_DATA_OFFSET {
            buf.push(0);
        }

        // ---- 9. .text 节数据 ----
        buf.extend_from_slice(&self.code);
        while (buf.len() as u32) < SECTION_DATA_OFFSET + file_aligned_code_size {
            buf.push(0);
        }

        // ---- 10. .idata 节数据 (如果有) ----
        if has_imports {
            buf.extend_from_slice(self.import_data.as_ref().unwrap());
            let idata_end = SECTION_DATA_OFFSET + file_aligned_code_size + import_file_size;
            while (buf.len() as u32) < idata_end {
                buf.push(0);
            }
        }

        buf
    }

    /// 内部辅助方法：将结构体以原始字节写入缓冲区
    ///
    /// 利用 `#[repr(C, packed)]` 的内存布局，安全地将结构体序列化为字节，
    /// 保持 PE 规范要求的小端序字节序（与 x86_64 原生字节序一致）。
    fn write_struct<T>(&self, buf: &mut Vec<u8>, val: &T) {
        let size = mem::size_of::<T>();
        let ptr = val as *const T as *const u8;
        buf.extend_from_slice(unsafe {
            std::slice::from_raw_parts(ptr, size)
        });
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试：空代码 PE 文件的基本结构正确性
    #[test]
    fn test_empty_pe_structure() {
        let builder = PeBuilder::new();
        let pe = builder.build();

        // 文件至少包含头部
        assert!(pe.len() >= 512, "PE 文件应至少大于 512 字节");

        // 检查 DOS 魔数
        assert_eq!(&pe[0..2], b"MZ");

        // 检查 e_lfanew 指向 PE 签名
        let e_lfanew = i32::from_le_bytes([pe[0x3C], pe[0x3D], pe[0x3E], pe[0x3F]]);
        let pe_offset = e_lfanew as usize;
        assert_eq!(&pe[pe_offset..pe_offset + 4], b"PE\0\0");

        // 检查机器类型为 x86_64
        let machine = u16::from_le_bytes([pe[pe_offset + 4], pe[pe_offset + 5]]);
        assert_eq!(machine, 0x8664);
    }

    /// 测试：带代码的 PE 生成
    #[test]
    fn test_pe_with_code() {
        let mut builder = PeBuilder::new();
        let code: Vec<u8> = vec![0x31, 0xC0, 0xC3]; // xor eax, eax; ret
        builder.add_code(&code);
        builder.set_entry_point(0x1000);

        let pe = builder.build();

        // 总大小应为头部(512) + 对齐后代码(512) = 1024
        assert_eq!(pe.len(), 1024);

        // 检查 .text 节中是否包含我们的机器码
        let section_data_start = 0x200;
        assert_eq!(&pe[section_data_start..section_data_start + 3], &[0x31, 0xC0, 0xC3]);
    }

    /// 测试：DOS Stub 内容
    #[test]
    fn test_dos_stub() {
        let builder = PeBuilder::new();
        let pe = builder.build();

        // DOS Stub 从偏移 64 开始，应包含标准消息
        let stub_start = 64;
        let stub_bytes = &pe[stub_start..stub_start + DOS_STUB.len()];
        assert_eq!(stub_bytes, DOS_STUB);
    }

    /// 测试：节表头的 .text 名称
    #[test]
    fn test_section_name() {
        let builder = PeBuilder::new();
        let pe = builder.build();

        // 节表头在可选头 + 数据目录之后
        // DOS(64) + Stub(64) + PE(4) + COFF(20) + 可选头(112) + 数据目录(128) = 392
        let section_offset = 0x188; // 392
        assert_eq!(&pe[section_offset..section_offset + 5], b".text");
    }
}
