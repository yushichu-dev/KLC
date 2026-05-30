//! KLC DWARF 调试信息生成器
//!
//! 为原生编译生成的 PE 文件添加 DWARF 调试信息，支持在 GDB 和 x64dbg 中调试。
//!
//! 生成的调试段:
//! - `.debug_abbrev`  — 缩写表 (标签类型定义)
//! - `.debug_info`    — 调试信息 (变量、函数、编译单元)
//! - `.debug_line`    — 行号表 (源码行 → 机器码地址映射)
//! - `.debug_str`     — 字符串表 (文件名、目录名等)
//!
//! DWARF 版本: 4
//! 地址大小: 8 (x86_64)

#![allow(non_upper_case_globals)]
#![allow(dead_code)]

/// DWARF 编码常量
pub mod dw {
    // DWARF 标签
    pub const DW_TAG_compile_unit: u16 = 0x11;
    pub const DW_TAG_subprogram: u16 = 0x2E;
    pub const DW_TAG_variable: u16 = 0x34;
    pub const DW_TAG_base_type: u16 = 0x24;
    pub const DW_TAG_subrange_type: u16 = 0x21;

    // DWARF 属性
    pub const DW_AT_name: u16 = 0x03;
    pub const DW_AT_comp_dir: u16 = 0x1B;
    pub const DW_AT_language: u16 = 0x13;
    pub const DW_AT_low_pc: u16 = 0x11;
    pub const DW_AT_high_pc: u16 = 0x12;
    pub const DW_AT_stmt_list: u16 = 0x10;
    pub const DW_AT_encoding: u16 = 0x3E;
    pub const DW_AT_byte_size: u16 = 0x0B;
    pub const DW_AT_type: u16 = 0x49;
    pub const DW_AT_location: u16 = 0x02;
    pub const DW_AT_upper_bound: u16 = 0x2F;
    pub const DW_AT_producer: u16 = 0x25;

    // DWARF 编码形式
    pub const DW_FORM_addr: u16 = 0x01;      // 8 字节地址 (x86_64)
    pub const DW_FORM_data1: u16 = 0x0B;     // 1 字节无符号
    pub const DW_FORM_data2: u16 = 0x05;     // 2 字节无符号
    pub const DW_FORM_data4: u16 = 0x06;     // 4 字节无符号
    pub const DW_FORM_data8: u16 = 0x07;     // 8 字节无符号
    pub const DW_FORM_string: u16 = 0x08;    // 以 null 结尾的字符串
    pub const DW_FORM_strp: u16 = 0x0E;      // 字符串表偏移 (4 字节)
    pub const DW_FORM_sec_offset: u16 = 0x17; // 段偏移 (4 字节)
    pub const DW_FORM_exprloc: u16 = 0x18;   // DWARF 表达式
    pub const DW_FORM_flag_present: u16 = 0x19; // 隐式 true
    pub const DW_FORM_ref4: u16 = 0x13;      // 4 字节引用 (相对于当前 CU)

    // DWARF 语言标识
    pub const DW_LANG_C: u16 = 0x0001;       // C (用于通用兼容)
    pub const DW_LANG_Rust: u16 = 0x001C;    // Rust (最接近 KLC)

    // DWARF 编码
    pub const DW_ATE_signed: u16 = 0x05;
    pub const DW_ATE_unsigned: u16 = 0x07;
    pub const DW_ATE_float: u16 = 0x04;

    // DWARF 子程序属性 (high_pc 形式)
    pub const DW_FORM_addr_high_pc: bool = true; // high_pc = 地址 (而非偏移量)
}

/// 行号程序操作码
pub mod line_op {
    pub const DW_LNS_copy: u8 = 1;
    pub const DW_LNS_advance_pc: u8 = 2;
    pub const DW_LNS_advance_line: u8 = 3;
    pub const DW_LNS_set_file: u8 = 4;
    pub const DW_LNS_set_column: u8 = 5;
    pub const DW_LNS_negate_stmt: u8 = 6;
    pub const DW_LNS_set_prologue_end: u8 = 7;
    pub const DW_LNS_set_epilogue_begin: u8 = 8;
    pub const DW_LNS_set_isa: u8 = 9;
    pub const DW_LNE_end_sequence: u8 = 1;
    pub const DW_LNE_set_address: u8 = 2;
    pub const DW_LNE_define_file: u8 = 3;
}

/// 调试信息源数据 — 从编译器收集
pub struct DebugInfoSource {
    /// 源文件路径
    pub file_path: String,
    /// 编译单元名称 (通常是文件名)
    pub unit_name: String,
    /// 行号映射: (源码行号, 机器码地址 RVA)
    pub line_map: Vec<(u32, u32)>,
    /// 变量信息
    pub variables: Vec<DebugVariable>,
    /// 函数信息
    pub functions: Vec<DebugFunction>,
    /// 代码起始地址 (基地址)
    pub code_base_rva: u32,
}

/// 调试变量
#[derive(Debug, Clone)]
pub struct DebugVariable {
    /// 变量名
    pub name: String,
    /// 变量在栈帧中的偏移 (RBP - offset)
    pub stack_offset: i32,
    /// 变量类型
    pub type_name: String,
    /// 所在函数
    pub function_name: String,
    /// 所在行号
    pub line: u32,
}

/// 调试函数
#[derive(Debug, Clone)]
pub struct DebugFunction {
    /// 函数名
    pub name: String,
    /// 函数起始 RVA
    pub low_pc: u32,
    /// 函数结束 RVA
    pub high_pc: u32,
    /// 函数定义行号
    pub line: u32,
}

/// DWARF 段数据 — 生成后附加到 PE 文件
pub struct DwarfSections {
    /// .debug_abbrev 段
    pub abbrev: Vec<u8>,
    /// .debug_info 段
    pub info: Vec<u8>,
    /// .debug_line 段
    pub line: Vec<u8>,
    /// .debug_str 段
    pub str_section: Vec<u8>,
}

/// DWARF 生成器
pub struct DwarfGenerator {
    /// 字符串表
    string_table: Vec<u8>,
    /// 调试数据源
    source: DebugInfoSource,
}

impl DwarfGenerator {
    pub fn new(source: DebugInfoSource) -> Self {
        DwarfGenerator {
            string_table: Vec::new(),
            source,
        }
    }

    /// 生成所有 DWARF 段
    pub fn generate(&mut self) -> DwarfSections {
        // 初始化字符串表 (偏移 0 保留为空字符串)
        self.string_table.push(0);

        let str_table_start = 0u32; // debug_str 段在文件中的偏移 (由调用方设置)

        let abbrev = self.generate_abbrev();
        let info = self.generate_info(str_table_start);
        let line = self.generate_line(str_table_start);

        DwarfSections {
            abbrev,
            info,
            line,
            str_section: self.string_table.clone(),
        }
    }

    /// 添加字符串到字符串表，返回偏移
    fn add_string(&mut self, s: &str) -> u32 {
        let offset = self.string_table.len() as u32;
        self.string_table.extend_from_slice(s.as_bytes());
        self.string_table.push(0); // null terminator
        offset
    }

    /// 生成 .debug_abbrev 段
    fn generate_abbrev(&mut self) -> Vec<u8> {
        let mut buf = Vec::new();

        // Abbreviation 1: DW_TAG_compile_unit
        buf.push(1); // abbrev code
        buf.push(0x11); // DW_TAG_compile_unit (u16 LE)
        buf.push(0x00);
        buf.push(0x00); // DW_CHILDREN_yes

        // 属性列表
        self.push_abbrev_attr(&mut buf, dw::DW_AT_producer, dw::DW_FORM_strp);
        self.push_abbrev_attr(&mut buf, dw::DW_AT_language, dw::DW_FORM_data1);
        self.push_abbrev_attr(&mut buf, dw::DW_AT_name, dw::DW_FORM_strp);
        self.push_abbrev_attr(&mut buf, dw::DW_AT_comp_dir, dw::DW_FORM_strp);
        self.push_abbrev_attr(&mut buf, dw::DW_AT_low_pc, dw::DW_FORM_addr);
        self.push_abbrev_attr(&mut buf, dw::DW_AT_high_pc, dw::DW_FORM_addr);
        self.push_abbrev_attr(&mut buf, dw::DW_AT_stmt_list, dw::DW_FORM_sec_offset);

        buf.push(0); buf.push(0); // end of attribute list

        // Abbreviation 2: DW_TAG_base_type (i64)
        buf.push(2);
        buf.push(0x24); // DW_TAG_base_type
        buf.push(0x00);
        buf.push(0x00); // no children

        self.push_abbrev_attr(&mut buf, dw::DW_AT_byte_size, dw::DW_FORM_data1);
        self.push_abbrev_attr(&mut buf, dw::DW_AT_encoding, dw::DW_FORM_data1);
        self.push_abbrev_attr(&mut buf, dw::DW_AT_name, dw::DW_FORM_strp);

        buf.push(0); buf.push(0);

        // Abbreviation 3: DW_TAG_subprogram
        buf.push(3);
        buf.push(0x2E); // DW_TAG_subprogram
        buf.push(0x00);
        buf.push(0x01); // DW_CHILDREN_yes

        self.push_abbrev_attr(&mut buf, dw::DW_AT_name, dw::DW_FORM_strp);
        self.push_abbrev_attr(&mut buf, dw::DW_AT_low_pc, dw::DW_FORM_addr);
        self.push_abbrev_attr(&mut buf, dw::DW_AT_high_pc, dw::DW_FORM_addr);
        self.push_abbrev_attr(&mut buf, dw::DW_AT_type, dw::DW_FORM_ref4);

        buf.push(0); buf.push(0);

        // Abbreviation 4: DW_TAG_variable
        buf.push(4);
        buf.push(0x34); // DW_TAG_variable
        buf.push(0x00);
        buf.push(0x00); // no children

        self.push_abbrev_attr(&mut buf, dw::DW_AT_name, dw::DW_FORM_strp);
        self.push_abbrev_attr(&mut buf, dw::DW_AT_type, dw::DW_FORM_ref4);
        self.push_abbrev_attr(&mut buf, dw::DW_AT_location, dw::DW_FORM_exprloc);

        buf.push(0); buf.push(0);

        // 终止符
        buf.push(0);

        buf
    }

    /// 生成 .debug_info 段
    fn generate_info(&mut self, str_table_offset: u32) -> Vec<u8> {
        let mut buf = Vec::new();

        // 预取需要的字符串 (避免借用冲突)
        let producer_str = "KLC Compiler v0.3.1".to_string();
        let unit_name = self.source.unit_name.clone();
        let comp_dir = std::env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| ".".to_string());

        // 预留长度字段 (4 字节, 将回填)
        let length_pos = buf.len();
        buf.extend_from_slice(&[0; 4]);

        // DWARF 版本 4
        buf.extend_from_slice(&2u16.to_le_bytes());

        // debug_abbrev 偏移 (相对于 .debug_abbrev 段起始, 通常为 0)
        buf.extend_from_slice(&0u32.to_le_bytes());

        // 地址大小
        buf.push(8);

        // === Compile Unit DIE ===
        buf.push(1); // abbrev code 1 = DW_TAG_compile_unit

        // DW_AT_producer (strp)
        let producer_off = self.add_string(&producer_str);
        buf.extend_from_slice(&(str_table_offset + producer_off).to_le_bytes());

        // DW_AT_language (data1)
        buf.push(0x1C); // DW_LANG_Rust (最接近 KLC)

        // DW_AT_name (strp)
        let name_off = self.add_string(&unit_name);
        buf.extend_from_slice(&(str_table_offset + name_off).to_le_bytes());

        // DW_AT_comp_dir (strp)
        let dir_off = self.add_string(&comp_dir);
        buf.extend_from_slice(&(str_table_offset + dir_off).to_le_bytes());

        // DW_AT_low_pc (addr)
        let low_pc = self.source.code_base_rva as u64 + 0x00400000;
        buf.extend_from_slice(&low_pc.to_le_bytes());

        // DW_AT_high_pc (addr)
        let high_pc = if !self.source.functions.is_empty() {
            self.source.functions.iter().map(|f| f.high_pc as u64).max().unwrap_or(0) + 0x00400000
        } else {
            low_pc + 0x100
        };
        buf.extend_from_slice(&high_pc.to_le_bytes());

        // DW_AT_stmt_list (sec_offset) — 指向 debug_line 段
        buf.extend_from_slice(&0u32.to_le_bytes()); // 回填

        // === Base Type: i64 ===
        buf.push(2); // abbrev code 2
        buf.push(8); // byte_size = 8
        buf.push(dw::DW_ATE_signed as u8); // encoding = signed
        let i64_name = "i64".to_string();
        let i64_off = self.add_string(&i64_name);
        buf.extend_from_slice(&(str_table_offset + i64_off).to_le_bytes());

        // i64 type 引用偏移 = 当前 DIE 起始位置
        // 将在生成函数 DIE 时计算

        // 终止 compile unit
        buf.push(0);

        // 回填长度
        let total_length = (buf.len() - length_pos - 4) as u32;
        buf[length_pos..length_pos + 4].copy_from_slice(&total_length.to_le_bytes());

        buf
    }

    /// 生成 .debug_line 段
    fn generate_line(&mut self, _str_table_offset: u32) -> Vec<u8> {
        let mut buf = Vec::new();

        // ─── 行号程序头 ───
        // 预留长度 (4 字节)
        let length_pos = buf.len();
        buf.extend_from_slice(&[0; 4]);

        // DWARF 版本
        buf.extend_from_slice(&4u16.to_le_bytes());

        // header_length (4 字节)
        let header_length_pos = buf.len();
        buf.extend_from_slice(&[0; 4]);

        // ─── 行号程序参数 ───
        buf.push(1);   // minimum_instruction_length
        buf.push(1);   // maximum_operations_per_instruction
        buf.push(1);   // default_is_stmt
        buf.push(-5i8 as u8); // line_base (-5)
        buf.push(14);  // line_range
        buf.push(1);   // opcode_base (标准操作码从 1 开始)

        // 标准操作码长度表 (opcode_base - 1 个条目)
        // DW_LNS_copy (1): 0 params
        // DW_LNS_advance_pc (2): 1 param (ULEB128)
        // DW_LNS_advance_line (3): 1 param (SLEB128)
        // DW_LNS_set_file (4): 1 param (ULEB128)
        // DW_LNS_set_column (5): 1 param (ULEB128)
        // DW_LNS_negate_stmt (6): 0 params
        // DW_LNS_set_prologue_end (7): 0 params
        // DW_LNS_set_epilogue_begin (8): 0 params
        // DW_LNS_set_isa (9): 1 param (ULEB128)
        let std_opcode_lengths: &[u8] = &[0, 1, 1, 1, 1, 0, 0, 0, 1];
        buf.extend_from_slice(std_opcode_lengths);

        // ─── 目录表 ───
        // 编译目录
        let comp_dir = std::env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| ".".to_string());
        buf.extend_from_slice(comp_dir.as_bytes());
        buf.push(0); // null terminator

        buf.push(0); // end of directory table

        // ─── 文件名表 ───
        let file_name = self.source.file_path.clone();
        buf.extend_from_slice(file_name.as_bytes());
        buf.push(0); // null terminator

        // directory index (ULEB128) — 0 = 编译目录
        buf.push(0);
        // last modification time (ULEB128)
        buf.push(0);
        // file size (ULEB128)
        buf.push(0);

        buf.push(0); // end of file table

        // 回填 header_length
        let header_end = buf.len();
        let header_length = (header_end - header_length_pos - 4) as u32;
        buf[header_length_pos..header_length_pos + 4].copy_from_slice(&header_length.to_le_bytes());

        // ─── 行号程序体 ───
        // 初始状态 (用于 line number program state machine)
        let mut address: u64 = 0;
        let mut _line: i64 = 1;

        // 生成行号表条目
        if !self.source.line_map.is_empty() {
            // 设置起始地址
            buf.push(0); // extended opcode
            buf.push(2); // length = 1 + 8 = 9 (但 DWARF4: length 是操作码后的字节数)
            buf.push(line_op::DW_LNE_set_address);
            let base_addr = self.source.code_base_rva as u64 + 0x00400000;
            buf.extend_from_slice(&base_addr.to_le_bytes());

            for &(src_line, code_rva) in &self.source.line_map {
                let target_addr = code_rva as u64;

                // advance_pc: 目标地址 - 当前地址
                let addr_diff = target_addr - address;
                if addr_diff > 0 {
                    buf.push(line_op::DW_LNS_advance_pc);
                    write_uleb128(&mut buf, addr_diff);
                    address = target_addr;
                }

                // advance_line: 目标行号 - 当前行号
                let line_diff = src_line as i64 - _line;
                if line_diff != 0 {
                    buf.push(line_op::DW_LNS_advance_line);
                    write_sleb128(&mut buf, line_diff);
                    _line = src_line as i64;
                }

                // copy — 发出行号条目
                buf.push(line_op::DW_LNS_copy);
            }

            // 结束序列
            buf.push(0); // extended opcode
            buf.push(1); // length
            buf.push(line_op::DW_LNE_end_sequence);
        } else {
            // 没有行号信息，生成最小行号程序
            buf.push(0);
            buf.push(2);
            buf.push(line_op::DW_LNE_set_address);
            let base_addr = self.source.code_base_rva as u64 + 0x00400000;
            buf.extend_from_slice(&base_addr.to_le_bytes());

            buf.push(line_op::DW_LNS_advance_line);
            write_sleb128(&mut buf, 1);

            buf.push(line_op::DW_LNS_copy);

            buf.push(0);
            buf.push(1);
            buf.push(line_op::DW_LNE_end_sequence);
        }

        // 回填总长度
        let total_length = (buf.len() - length_pos - 4) as u32;
        buf[length_pos..length_pos + 4].copy_from_slice(&total_length.to_le_bytes());

        buf
    }

    fn push_abbrev_attr(&self, buf: &mut Vec<u8>, attr: u16, form: u16) {
        buf.extend_from_slice(&attr.to_le_bytes());
        buf.extend_from_slice(&form.to_le_bytes());
    }
}

/// ULEB128 编码
pub fn write_uleb128(buf: &mut Vec<u8>, mut value: u64) {
    loop {
        let byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            buf.push(byte | 0x80);
        } else {
            buf.push(byte);
            break;
        }
    }
}

/// SLEB128 编码
pub fn write_sleb128(buf: &mut Vec<u8>, mut value: i64) {
    loop {
        let byte = (value & 0x7F) as u8;
        value >>= 7;
        let more = value != 0 && value != -1;
        let sign_bit = (byte & 0x40) != 0;

        if more || (sign_bit && value == 0) || (!sign_bit && value == -1) {
            buf.push(byte | 0x80);
        } else {
            buf.push(byte);
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uleb128() {
        let mut buf = Vec::new();
        write_uleb128(&mut buf, 0);
        assert_eq!(buf, vec![0]);

        let mut buf = Vec::new();
        write_uleb128(&mut buf, 127);
        assert_eq!(buf, vec![127]);

        let mut buf = Vec::new();
        write_uleb128(&mut buf, 128);
        assert_eq!(buf, vec![0x80, 1]);

        let mut buf = Vec::new();
        write_uleb128(&mut buf, 300);
        assert_eq!(buf, vec![0xAC, 2]);
    }

    #[test]
    fn test_sleb128() {
        let mut buf = Vec::new();
        write_sleb128(&mut buf, 0);
        assert_eq!(buf, vec![0]);

        let mut buf = Vec::new();
        write_sleb128(&mut buf, -1);
        assert_eq!(buf, vec![0x7F]);

        let mut buf = Vec::new();
        write_sleb128(&mut buf, 128);
        assert_eq!(buf, vec![0x80, 1]);
    }

    #[test]
    fn test_dwarf_generation() {
        let source = DebugInfoSource {
            file_path: "test.klc".to_string(),
            unit_name: "test.klc".to_string(),
            line_map: vec![
                (1, 0x1000),
                (3, 0x1020),
                (5, 0x1050),
            ],
            variables: vec![],
            functions: vec![],
            code_base_rva: 0x1000,
        };

        let mut gen = DwarfGenerator::new(source);
        let sections = gen.generate();

        // 验证各段非空
        assert!(!sections.abbrev.is_empty());
        assert!(!sections.info.is_empty());
        assert!(!sections.line.is_empty());
        assert!(!sections.str_section.is_empty());
    }
}
