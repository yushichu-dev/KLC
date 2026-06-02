//! KLC 原生 PE 文件生成器 — 最小 PE 文件生成
//! v1.0.5: 精简版，修复 Git 冲突残留

use std::mem;

// ── 常量 ──
pub const CODE_RVA: u32 = 0x1000;
pub const IDATA_RVA: u32 = 0x2000;
const FILE_ALIGNMENT: u32 = 0x200;
const PE_SIGNATURE_OFFSET: u32 = 0x80;
const SECTION_DATA_OFFSET: u32 = FILE_ALIGNMENT;
const PE_SIGNATURE: u32 = 0x0000_4550;

const DOS_STUB: &[u8] = &[
    0x0E, 0x1F, 0xBA, 0x0E, 0x00, 0xB4, 0x09, 0xCD,
    0x21, 0xB8, 0x01, 0x4C, 0xCD, 0x21,
    0x54, 0x68, 0x69, 0x73, 0x20, 0x70, 0x72, 0x6F,
    0x67, 0x72, 0x61, 0x6D, 0x20, 0x63, 0x61, 0x6E,
    0x6E, 0x6F, 0x74, 0x20, 0x62, 0x65, 0x20, 0x72,
    0x75, 0x6E, 0x20, 0x69, 0x6E, 0x20, 0x44, 0x4F,
    0x53, 0x20, 0x6D, 0x6F, 0x64, 0x65, 0x2E, 0x0D,
    0x0D, 0x0A, 0x24, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00,
];

#[repr(C, packed)]
#[derive(Debug, Clone)]
pub struct DosHeader {
    pub e_magic: u16,
    pub e_cblp: u16,
    pub e_cp: u16,
    pub e_crlc: u16,
    pub e_cparhdr: u16,
    pub e_minalloc: u16,
    pub e_maxalloc: u16,
    pub e_ss: u16,
    pub e_sp: u16,
    pub e_csum: u16,
    pub e_ip: u16,
    pub e_cs: u16,
    pub e_lfarlc: u16,
    pub e_ovno: u16,
    pub e_res: [u16; 4],
    pub e_oemid: u16,
    pub e_oeminfo: u16,
    pub e_res2: [u16; 10],
    pub e_lfanew: i32,
}

impl DosHeader {
    pub fn standard() -> Self {
        DosHeader {
            e_magic: 0x5A4D, e_cblp: 0x0090, e_cp: 0x0003, e_crlc: 0x0000,
            e_cparhdr: 0x0004, e_minalloc: 0x0000, e_maxalloc: 0xFFFF,
            e_ss: 0x0000, e_sp: 0x00B8, e_csum: 0x0000, e_ip: 0x0000,
            e_cs: 0x0000, e_lfarlc: 0x0040, e_ovno: 0x0000,
            e_res: [0; 4], e_oemid: 0x0000, e_oeminfo: 0x0000,
            e_res2: [0; 10], e_lfanew: 0x0000_0080,
        }
    }
}

#[repr(C, packed)]
#[derive(Debug, Clone)]
pub struct CoffHeader {
    pub machine: u16,
    pub number_of_sections: u16,
    pub time_date_stamp: u32,
    pub pointer_to_symbol_table: u32,
    pub number_of_symbols: u32,
    pub size_of_optional_header: u16,
    pub characteristics: u16,
}

impl CoffHeader {
    pub fn standard_x64() -> Self {
        CoffHeader {
            machine: 0x8664, number_of_sections: 1, time_date_stamp: 0,
            pointer_to_symbol_table: 0, number_of_symbols: 0,
            size_of_optional_header: 0x00F0, characteristics: 0x0022,
        }
    }
}

#[repr(C, packed)]
#[derive(Debug, Clone)]
pub struct OptionalHeader64 {
    pub magic: u16,
    pub major_linker_version: u8,
    pub minor_linker_version: u8,
    pub size_of_code: u32,
    pub size_of_initialized_data: u32,
    pub size_of_uninitialized_data: u32,
    pub address_of_entry_point: u32,
    pub base_of_code: u32,
    pub image_base: u64,
    pub section_alignment: u32,
    pub file_alignment: u32,
    pub major_os_version: u16,
    pub minor_os_version: u16,
    pub major_image_version: u16,
    pub minor_image_version: u16,
    pub major_subsystem_version: u16,
    pub minor_subsystem_version: u16,
    pub win32_version_value: u32,
    pub size_of_image: u32,
    pub size_of_headers: u32,
    pub checksum: u32,
    pub subsystem: u16,
    pub dll_characteristics: u16,
    pub size_of_stack_reserve: u64,
    pub size_of_stack_commit: u64,
    pub size_of_heap_reserve: u64,
    pub size_of_heap_commit: u64,
    pub loader_flags: u32,
    pub number_of_rva_and_sizes: u32,
}

impl OptionalHeader64 {
    pub fn standard_console(entry_point: u32, code_size: u32, image_base: u64) -> Self {
        OptionalHeader64 {
            magic: 0x020B,
            major_linker_version: 1, minor_linker_version: 0,
            size_of_code: code_size,
            size_of_initialized_data: 0,
            size_of_uninitialized_data: 0,
            address_of_entry_point: entry_point,
            base_of_code: 0x1000,
            image_base,
            section_alignment: 0x1000,
            file_alignment: 0x0200,
            major_os_version: 6, minor_os_version: 1,
            major_image_version: 0, minor_image_version: 0,
            major_subsystem_version: 6, minor_subsystem_version: 1,
            win32_version_value: 0,
            size_of_image: (0x2000 + ((code_size.max(1) + 0xFFF) & !0xFFF)),
            size_of_headers: 0x0200,
            checksum: 0,
            subsystem: 0x0003,
            dll_characteristics: 0x8160,
            size_of_stack_reserve: 0x0010_0000,
            size_of_stack_commit: 0x0000_1000,
            size_of_heap_reserve: 0x0010_0000,
            size_of_heap_commit: 0x0000_1000,
            loader_flags: 0,
            number_of_rva_and_sizes: 16,
        }
    }
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct DataDirectory { pub virtual_address: u32, pub size: u32 }

impl DataDirectory {
    pub const fn empty() -> Self { DataDirectory { virtual_address: 0, size: 0 } }
}

#[repr(C, packed)]
#[derive(Debug, Clone)]
pub struct SectionHeader {
    pub name: [u8; 8],
    pub virtual_size: u32,
    pub virtual_address: u32,
    pub size_of_raw_data: u32,
    pub pointer_to_raw_data: u32,
    pub pointer_to_relocations: u32,
    pub pointer_to_linenumbers: u32,
    pub number_of_relocations: u16,
    pub number_of_linenumbers: u16,
    pub characteristics: u32,
}

impl SectionHeader {
    pub fn text_section(code_bytes_len: u32, code_rva: u32, file_alignment: u32, raw_offset: u32) -> Self {
        let raw_size = (code_bytes_len + file_alignment - 1) & !(file_alignment - 1);
        SectionHeader {
            name: *b".text\0\0\0", virtual_size: code_bytes_len, virtual_address: code_rva,
            size_of_raw_data: raw_size, pointer_to_raw_data: raw_offset,
            pointer_to_relocations: 0, pointer_to_linenumbers: 0,
            number_of_relocations: 0, number_of_linenumbers: 0,
            characteristics: 0x6000_0020,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PeBuilder {
    code: Vec<u8>,
    entry_point: u32,
    image_base: u64,
    code_rva: u32,
}

impl PeBuilder {
    pub fn new() -> Self {
        PeBuilder {
            code: Vec::new(), entry_point: CODE_RVA,
            image_base: 0x00000000_00400000, code_rva: CODE_RVA,
        }
    }

    pub fn add_code(&mut self, code: &[u8]) { self.code.extend_from_slice(code); }
    pub fn set_entry_point(&mut self, entry_point: u32) { self.entry_point = entry_point; }

    pub fn build(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(4096);
        let code_size = self.code.len() as u32;
        let file_aligned_code_size = (code_size + FILE_ALIGNMENT - 1) & !(FILE_ALIGNMENT - 1);
        let size_of_image = 0x2000u32;

        // DOS Header
        self.write_struct(&mut buf, &DosHeader::standard());
        // DOS Stub
        let _stub_size = PE_SIGNATURE_OFFSET as usize - mem::size_of::<DosHeader>();
        buf.extend_from_slice(DOS_STUB);
        while buf.len() < PE_SIGNATURE_OFFSET as usize { buf.push(0); }
        // PE Signature
        buf.extend_from_slice(&PE_SIGNATURE.to_le_bytes());
        // COFF Header
        let mut coff = CoffHeader::standard_x64();
        coff.number_of_sections = 1;
        self.write_struct(&mut buf, &coff);
        // Optional Header
        let mut opt = OptionalHeader64::standard_console(self.entry_point, file_aligned_code_size, self.image_base);
        opt.size_of_image = size_of_image;
        self.write_struct(&mut buf, &opt);
        // Data Directories (16 × 8 = 128 bytes, all zero)
        for _ in 0..16 { self.write_struct(&mut buf, &DataDirectory::empty()); }
        // Section Header (.text)
        let text_sect = SectionHeader::text_section(code_size, self.code_rva, FILE_ALIGNMENT, SECTION_DATA_OFFSET);
        self.write_struct(&mut buf, &text_sect);
        // Align to FILE_ALIGNMENT
        while (buf.len() as u32) < SECTION_DATA_OFFSET { buf.push(0); }
        // Code data
        buf.extend_from_slice(&self.code);
        while (buf.len() as u32) < SECTION_DATA_OFFSET + file_aligned_code_size { buf.push(0); }
        // Checksum
        let checksum = Self::compute_checksum(&buf);
        let ck_off = PE_SIGNATURE_OFFSET as usize + 4 + mem::size_of::<CoffHeader>() + 64;
        buf[ck_off..ck_off + 4].copy_from_slice(&checksum.to_le_bytes());
        buf
    }

    fn compute_checksum(data: &[u8]) -> u32 {
        let mut sum: u64 = 0;
        let len = data.len();
        let mut i = 0;
        while i + 1 < len { sum += u16::from_le_bytes([data[i], data[i+1]]) as u64; i += 2; }
        if i < len { sum += data[i] as u64; }
        sum += len as u64;
        (sum & 0xFFFF_FFFF) as u32
    }

    fn write_struct<T>(&self, buf: &mut Vec<u8>, val: &T) {
        let size = mem::size_of::<T>();
        let ptr = val as *const T as *const u8;
        buf.extend_from_slice(unsafe { std::slice::from_raw_parts(ptr, size) });
    }
}

#[allow(dead_code)]
pub fn compile_to_exe(output_path: &str, machine_code: &[u8]) -> Result<(), String> {
    let mut builder = PeBuilder::new();
    builder.add_code(machine_code);
    builder.set_entry_point(CODE_RVA);
    let pe = builder.build();
    std::fs::write(output_path, &pe).map_err(|e| format!("无法写入 {}: {}", output_path, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_pe() {
        let builder = PeBuilder::new();
        let pe = builder.build();
        assert!(pe.len() >= 512);
        assert_eq!(&pe[0..2], b"MZ");
        let e_lfanew = i32::from_le_bytes([pe[0x3C], pe[0x3D], pe[0x3E], pe[0x3F]]);
        assert_eq!(&pe[e_lfanew as usize..e_lfanew as usize + 4], b"PE\0\0");
    }

    #[test]
    fn test_pe_with_code() {
        let mut builder = PeBuilder::new();
        builder.add_code(&[0x31, 0xC0, 0xC3]);
        builder.set_entry_point(0x1000);
        let pe = builder.build();
        assert_eq!(pe.len(), 1024);
        assert_eq!(&pe[0x200..0x203], &[0x31, 0xC0, 0xC3]);
    }
}
