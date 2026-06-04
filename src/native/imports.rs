//! KLC PE 导入表生成器 — 阶段三
//!
//! 从字节层面手动构造 Windows PE32+ 导入表，支持调用外部 DLL 函数。
//! 严格遵循 PE 规范，零外部依赖。
//!
//! ## PE 导入表结构 (x64)
//!
//! ```text
//! 导入目录表 (IDT)        — IMAGE_IMPORT_DESCRIPTOR 数组，以全零条目结尾
//!   ├── OriginalFirstThunk → 导入查找表 (ILT)
//!   ├── Name              → DLL 名称字符串 (RVA)
//!   └── FirstThunk        → 导入地址表 (IAT)
//!
//! IAT / ILT (x64)         — IMAGE_THUNK_DATA64 数组，以 0 结尾
//!   每个条目 8 字节:
//!     bit 63 = 0 → RVA 指向 IMAGE_IMPORT_BY_NAME (Hint + Name)
//!     bit 63 = 1 → 序号导入 (低 16 位为序号)
//!
//! Hint/Name 条目:
//!   Hint: u16  (导出序号提示，可设为 0)
//!   Name: 以 '\0' 结尾的函数名
//! ```

// 阶段三: 不使用 HashMap

// ============================================================================
// 导入表结构体
// ============================================================================

/// 导入目录表条目 — IMAGE_IMPORT_DESCRIPTOR (20 字节)
///
/// 描述从一个 DLL 导入的一组函数。数组以全零条目终止。
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct ImportDirectoryEntry {
    /// OriginalFirstThunk — ILT (Import Lookup Table) 的 RVA
    /// 32 位 PE: IMAGE_THUNK_DATA32 数组, 64 位 PE: IMAGE_THUNK_DATA64 数组
    /// 可为 0 (某些链接器不生成 ILT), 此时使用 IAT 的值
    pub original_first_thunk: u32,
    /// 时间戳 — 绑定导入时使用, 通常为 0
    pub time_date_stamp: u32,
    /// 转发链 — 首个转发项的索引, 通常为 0
    pub forwarder_chain: u32,
    /// DLL 名称字符串的 RVA (例如 "kernel32.dll\0")
    pub name: u32,
    /// FirstThunk — IAT (Import Address Table) 的 RVA
    /// 加载器将实际函数地址写入此表
    pub first_thunk: u32,
}

impl ImportDirectoryEntry {
    /// 创建导入目录条目
    pub const fn new(ilt_rva: u32, name_rva: u32, iat_rva: u32) -> Self {
        ImportDirectoryEntry {
            original_first_thunk: ilt_rva,
            time_date_stamp: 0,
            forwarder_chain: 0,
            name: name_rva,
            first_thunk: iat_rva,
        }
    }

    /// 创建终止条目 (全零)
    pub const fn terminator() -> Self {
        ImportDirectoryEntry {
            original_first_thunk: 0,
            time_date_stamp: 0,
            forwarder_chain: 0,
            name: 0,
            first_thunk: 0,
        }
    }
}

// ============================================================================
// 导入表构建器
// ============================================================================

/// 导入表构建器 — 逐步构造 PE 导入表的所有组件
///
/// 管理一个 DLL 的导入表，自动计算所有 RVA 偏移。
///
/// # 使用示例
///
/// ```ignore
/// let mut builder = ImportTableBuilder::new();
/// let getstd_rva = builder.add_import("kernel32.dll", "GetStdHandle");
/// let exit_rva = builder.add_import("kernel32.dll", "ExitProcess");
/// let (data, import_rva, import_size) = builder.build();
/// // getstd_rva 和 exit_rva 是 IAT 条目的绝对 RVA
/// ```
#[derive(Debug, Clone)]
pub struct ImportTableBuilder {
    /// DLL 名称 → (导入函数名列表, 函数 IAT RVA 列表)
    dlls: Vec<DllImport>,
    /// 导入表数据区的基础 RVA (外部设定)
    base_rva: u32,
}

#[derive(Debug, Clone)]
struct DllImport {
    /// DLL 名称 (含 \0)
    dll_name: String,
    /// 函数名列表 (含 \0)
    functions: Vec<String>,
    /// 每个函数对应的 IAT 条目 RVA (相对于 base_rva)
    iat_entries: Vec<u32>,
}

impl ImportTableBuilder {
    /// 创建新的导入表构建器
    pub fn new() -> Self {
        ImportTableBuilder {
            dlls: Vec::new(),
            base_rva: 0x2000,  // .idata 默认 RVA
        }
    }

    /// 设置导入表数据区的基础 RVA (即 .idata 节的 RVA)
    pub fn set_base_rva(&mut self, rva: u32) {
        self.base_rva = rva;
    }

    /// 预注册一个 DLL 条目（不添加任何函数）
    /// 必须在 add_import 之前调用，以稳定 IDT 大小，确保 IAT 偏移计算正确
    pub fn register_dll(&mut self, dll_name: &str) {
        let dll_owned = format!("{}\0", dll_name);
        if !self.dlls.iter().any(|d| d.dll_name == dll_owned) {
            self.dlls.push(DllImport {
                dll_name: dll_owned,
                functions: Vec::new(),
                iat_entries: Vec::new(),
            });
        }
    }

    /// 添加一个导入函数
    ///
    /// # 参数
    /// * `dll_name` - DLL 名称 (如 "kernel32.dll")
    /// * `function_name` - 函数名 (如 "GetStdHandle")
    ///
    /// # 返回值
    /// 该函数在 IAT 中的条目 RVA (绝对地址), 供代码生成器计算 RIP-relative 偏移
    pub fn add_import(&mut self, dll_name: &str, function_name: &str) -> u32 {
        let dll_owned = format!("{}\0", dll_name);
        let fn_owned = format!("{}\0", function_name);

        // 检查是否已存在
        for dll in &self.dlls {
            if dll.dll_name == dll_owned {
                if let Some(pos) = dll.functions.iter().position(|f| *f == fn_owned) {
                    return self.base_rva + dll.iat_entries[pos];
                }
            }
        }

        // 预计算 IAT 偏移
        let iat_offset = self.calc_next_iat_offset(&dll_owned);

        // 查找或创建 DLL 条目 (用索引避免借用冲突)
        let idx = self.dlls.iter().position(|d| d.dll_name == dll_owned);
        match idx {
            Some(i) => {
                self.dlls[i].functions.push(fn_owned);
                self.dlls[i].iat_entries.push(iat_offset);
            }
            None => {
                self.dlls.push(DllImport {
                    dll_name: format!("{}\0", dll_name),
                    functions: vec![fn_owned],
                    iat_entries: vec![iat_offset],
                });
            }
        }

        self.base_rva + iat_offset
    }

    /// 计算新函数在当前布局中的 IAT 偏移 (假设函数/DLL 已添加)
    fn calc_next_iat_offset(&self, target_dll_name: &str) -> u32 {
        // 判断目标 DLL 是否已存在
        let dll_exists = self.dlls.iter().any(|d| d.dll_name == target_dll_name);
        // IDT 条目数 = 已有 DLL + 新 DLL(如果不存在) + 1 终止符
        let dll_count = if dll_exists { self.dlls.len() } else { self.dlls.len() + 1 };
        let idt_size = (dll_count + 1) as u32 * mem_size_of::<ImportDirectoryEntry>() as u32;
        let mut offset = idt_size;

        for dll in &self.dlls {
            let iat_size = (dll.functions.len() + 1) as u32 * 8;
            if dll.dll_name == target_dll_name {
                // 已有 DLL — 新函数追加到该 DLL 的 IAT 末尾 (在终止符前)
                return offset + dll.functions.len() as u32 * 8;
            }
            offset += iat_size;
        }
        // 新 DLL — IAT 数组在已有所有 DLL 的 IAT 之后
        // 第一个函数 = 第 0 个条目
        offset
    }

    /// 生成完整的导入表二进制数据
    ///
    /// # 返回值
    /// `(data, import_rva, import_size)`
    /// - `data`: 导入表完整二进制数据 (按节对齐补齐)
    /// - `import_rva`: 导入目录表 (IDT) 的 RVA
    /// - `import_size`: 导入目录表的大小
    ///
    /// # 布局
    ///
    /// ```text
    /// [IDT] → [IAT_for_dll1] → [IAT_for_dll2] → ... → [strings]
    /// ```
    /// ILT 和 IAT 共用同一数组 (OriginalFirstThunk == FirstThunk)
    pub fn build(&self) -> (Vec<u8>, u32, u32) {
        let mut data = Vec::with_capacity(256);
        let mut rva_offset: u32 = 0;  // 相对于 base_rva 的偏移

        // ---- 计算 IDT 大小 ----
        let idt_size = (self.dlls.len() + 1) as u32 * mem_size_of::<ImportDirectoryEntry>() as u32;
        let _idt_rva = rva_offset;  // IDT 在开头
        rva_offset += idt_size;

        // ---- 计算每个 DLL 的 IAT/ILT 位置 ----
        let mut dll_iat_rvas: Vec<u32> = Vec::new();   // 每个 DLL 的 IAT RVA (相对)
        for dll in &self.dlls {
            dll_iat_rvas.push(rva_offset);
            let iat_size = (dll.functions.len() + 1) as u32 * 8;  // +1 terminator
            rva_offset += iat_size;
        }

        // ---- 字符串区域 (Hint/Name 条目 + DLL 名 + 函数名) ----
        let mut hint_name_rvas: Vec<(u32, u32)> = Vec::new(); // (hint_offset, name_offset) per function
        for (_di, dll) in self.dlls.iter().enumerate() {
            for fi in 0..dll.functions.len() {
                let hint_offset = rva_offset;
                rva_offset += 2;  // Hint: u16
                let name_offset = rva_offset;
                rva_offset += dll.functions[fi].len() as u32;  // name + \0
                hint_name_rvas.push((hint_offset, name_offset));
            }
        }
        // DLL 名称字符串
        let mut dll_name_rvas: Vec<u32> = Vec::new();
        for dll in &self.dlls {
            dll_name_rvas.push(rva_offset);
            rva_offset += dll.dll_name.len() as u32;
        }

        // ---- 写入 IDT 条目 (RVA 必须为绝对地址, 加 base_rva) ----
        let mut _fn_idx = 0usize;
        for (di, dll) in self.dlls.iter().enumerate() {
            let ilt_rva = self.base_rva + dll_iat_rvas[di];  // 绝对 RVA
            let iat_rva = self.base_rva + dll_iat_rvas[di];  // 绝对 RVA
            let name_rva = self.base_rva + dll_name_rvas[di]; // 绝对 RVA
            let entry = ImportDirectoryEntry::new(ilt_rva, name_rva, iat_rva);
            push_struct(&mut data, &entry);
            _fn_idx += dll.functions.len();
        }
        // IDT 终止符
        push_struct(&mut data, &ImportDirectoryEntry::terminator());

        // ---- 写入 IAT/ILT 数组 ----
        let mut fn_counter = 0usize;
        for dll in &self.dlls {
            for _ in 0..dll.functions.len() {
                let (hint_offset, _) = hint_name_rvas[fn_counter];
                // IAT 条目: 绝对 RVA 指向 Hint/Name 条目 (bit 63 = 0 表示按名导入)
                // Windows 加载器用此 RVA 找到函数名, 解析后用实际地址覆盖此条目
                let thunk: u64 = (self.base_rva + hint_offset) as u64;
                data.extend_from_slice(&thunk.to_le_bytes());
                fn_counter += 1;
            }
            // IAT 终止符 (0)
            data.extend_from_slice(&0u64.to_le_bytes());
        }

        // ---- 写入 Hint/Name 条目 ----
        fn_counter = 0;
        for dll in &self.dlls {
            for fi in 0..dll.functions.len() {
                let (_hint_off, _name_off) = hint_name_rvas[fn_counter];
                // Hint: u16 = 0
                data.extend_from_slice(&0u16.to_le_bytes());
                // Name: ASCII 字符串
                data.extend_from_slice(dll.functions[fi].as_bytes());
                fn_counter += 1;
            }
        }

        // ---- 写入 DLL 名称字符串 ----
        for dll in &self.dlls {
            data.extend_from_slice(dll.dll_name.as_bytes());
        }

        // 返回: (二进制数据, IDT 的绝对 RVA, IDT 大小)
        let import_rva = self.base_rva;
        (data, import_rva, idt_size)
    }
}

/// 计算结构体大小 (无需实例)
fn mem_size_of<T>() -> usize {
    std::mem::size_of::<T>()
}

/// 将 #[repr(C, packed)] 结构体写入 Vec<u8>
fn push_struct<T>(buf: &mut Vec<u8>, val: &T) {
    let size = std::mem::size_of::<T>();
    let ptr = val as *const T as *const u8;
    buf.extend_from_slice(unsafe { std::slice::from_raw_parts(ptr, size) });
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试：导入表结构正确性
    #[test]
    fn test_import_table_structure() {
        assert_eq!(std::mem::size_of::<ImportDirectoryEntry>(), 20);
    }

    /// 测试：基本导入表生成
    #[test]
    fn test_build_imports() {
        let mut builder = ImportTableBuilder::new();
        builder.set_base_rva(0x2000);

        let _exit_rva = builder.add_import("kernel32.dll", "ExitProcess");
        let _getstd_rva = builder.add_import("kernel32.dll", "GetStdHandle");

        let (data, import_rva, import_size) = builder.build();

        // import_rva 应该是 base_rva
        assert_eq!(import_rva, 0x2000);

        // 有数据生成
        assert!(!data.is_empty());
        // import_size > 0
        assert!(import_size > 0);

        // 数据中应包含 "kernel32.dll" 字符串 (12 字节)
        let kernel32_pos = data.windows(12).position(|w| w == b"kernel32.dll");
        assert!(kernel32_pos.is_some(), "data中应包含 'kernel32.dll' 字符串");

        // 数据中应包含 "ExitProcess" 字符串
        let exit_pos = data.windows(12).position(|w| w == b"ExitProcess\0");
        assert!(exit_pos.is_some(), "应包含 'ExitProcess' 字符串");
    }
}
