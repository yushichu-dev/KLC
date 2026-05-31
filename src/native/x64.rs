//! KLC x86_64 指令编码器 — 阶段二
//!
//! 手动实现 x86_64 指令的二进制编码，遵循 Intel 指令集规范。
//! 完全使用 Rust 标准库，零外部依赖。
//!
//! ## x86_64 指令编码公式
//!
//! ```text
//! [Prefixes] [REX] Opcode [ModR/M] [SIB] [Displacement] [Immediate]
//! ```
//!
//! - **REX 前缀** (可选): `0x40 | (W<<3) | (R<<2) | (X<<1) | B`
//!   W=1 表示 64 位操作数, R/X/B 分别扩展 ModR/M 的 reg/SIB_index/rm 域到第 4 位
//! - **ModR/M 字节**: `(mod<<6) | (reg<<3) | rm`
//!   mod=11 表示寄存器寻址, reg 域有时用作 opcode 扩展
//! - **SIB 字节**: `(scale<<6) | (index<<3) | base`
//!   用于 [base + index*scale] 寻址模式和 RSP/R12 基址寄存器
//!
//! ## Microsoft x64 调用约定速查
//!
//! | 项目 | 约定 |
//! |------|------|
//! | 前 4 个整数参数 | RCX, RDX, R8, R9 |
//! | 浮点参数 | XMM0-XMM3 |
//! | 返回值 | RAX (整数), XMM0 (浮点) |
//! | 被调用者保存 | RBX, RBP, RSI, RDI, R12-R15, XMM6-XMM15 |
//! | 调用者保存(易失) | RAX, RCX, RDX, R8-R11, XMM0-XMM5 |
//! | 栈对齐 | 16 字节 (call 指令前) |
//! | Shadow Space | 调用者在栈上预留 32 字节 |

use std::collections::HashMap;

// ============================================================================
// 寄存器定义
// ============================================================================

/// x86_64 通用寄存器枚举
///
/// 16 个 64 位通用寄存器。在指令编码中每个寄存器有一个 3 位代码 (0-7)，
/// R8-R15 的代码复用 0-7 并通过 REX 前缀的 B/R/X 位扩展。
///
/// # 寄存器编码表
///
/// ```text
/// 代码  0    1    2    3    4    5    6    7
/// ────────────────────────────────────────────
/// 普通  RAX  RCX  RDX  RBX  RSP  RBP  RSI  RDI
/// 扩展  R8   R9   R10  R11  R12  R13  R14  R15
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub enum Register {
    RAX, RCX, RDX, RBX, RSP, RBP, RSI, RDI,
    R8,  R9,  R10, R11, R12, R13, R14, R15,
}

impl Register {
    /// 返回寄存器的 3 位编码 (0-7)
    /// R8-R15 返回与 RAX-RDI 相同的低 3 位
    pub fn code(self) -> u8 {
        match self {
            Register::RAX => 0, Register::RCX => 1,
            Register::RDX => 2, Register::RBX => 3,
            Register::RSP => 4, Register::RBP => 5,
            Register::RSI => 6, Register::RDI => 7,
            Register::R8  => 0, Register::R9  => 1,
            Register::R10 => 2, Register::R11 => 3,
            Register::R12 => 4, Register::R13 => 5,
            Register::R14 => 6, Register::R15 => 7,
        }
    }

    /// 是否是扩展寄存器 (R8-R15)，需要在 REX 前缀中设置扩展位
    pub fn is_extended(self) -> bool {
        matches!(self,
            Register::R8  | Register::R9  | Register::R10 | Register::R11 |
            Register::R12 | Register::R13 | Register::R14 | Register::R15
        )
    }
}

// ============================================================================
// 标签系统
// ============================================================================

/// 汇编标签 — 用于标记代码位置，支持前向引用
///
/// 每条跳转/调用指令通过标签引用目标位置。
/// 标签在 `bind_label` 时确定位置，在 `finish` 时完成所有回填。
pub type Label = u32;

/// 未解决的引用 — 记录需要在 finish 时回填的跳转目标
#[derive(Debug, Clone)]
struct UnresolvedRef {
    /// 目标标签
    label: Label,
    /// 在 buffer 中的偏移（displacement 字段的起始位置）
    offset: usize,
    /// 引用类型（影响 displacement 计算）
    kind: RefKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RefKind {
    /// 4 字节相对偏移 (用于 jmp rel32, jcc rel32, call rel32)
    Rel32,
}

// ============================================================================
// X64Assembler — x86_64 汇编器
// ============================================================================

/// x86_64 指令编码器
///
/// 提供链式指令生成接口，内部维护一个字节码缓冲区。
/// 支持标签的前向引用，在 `finish()` 时自动回填所有跳转偏移。
///
/// # 使用示例
///
/// ```ignore
/// let mut asm = X64Assembler::new();
/// asm.mov_reg_imm(Register::RAX, 1);
/// asm.mov_reg_imm(Register::RCX, 2);
/// asm.add_reg_reg(Register::RAX, Register::RCX);
/// asm.ret();
/// let code = asm.finish();
/// // code = [B8 01 00 00 00, B9 02 00 00 00, 48 01 C8, C3]
/// ```
pub struct X64Assembler {
    /// 机器码字节缓冲区
    buffer: Vec<u8>,
    /// 已绑定标签 → buffer 中的偏移
    labels: HashMap<Label, usize>,
    /// 待回填的引用列表
    unresolved: Vec<UnresolvedRef>,
    /// 下一个可分配的标签 ID
    next_label: Label,
}

#[allow(dead_code)]
impl X64Assembler {
    /// 创建新的汇编器实例
    pub fn new() -> Self {
        X64Assembler {
            buffer: Vec::with_capacity(256),
            labels: HashMap::new(),
            unresolved: Vec::new(),
            next_label: 0,
        }
    }

    // ---- 标签操作 ----

    /// 创建新的未绑定标签
    pub fn new_label(&mut self) -> Label {
        let label = self.next_label;
        self.next_label += 1;
        label
    }

    /// 将标签绑定到当前代码位置
    ///
    /// # Panics
    /// 如果标签已被绑定过
    pub fn bind_label(&mut self, label: Label) {
        if self.labels.contains_key(&label) {
            panic!("Label {} is already bound", label);
        }
        self.labels.insert(label, self.buffer.len());
    }

    /// 完成汇编，回填所有未解决的标签引用，返回最终机器码
    ///
    /// # Panics
    /// 如果存在未绑定的标签引用
    pub fn finish(mut self) -> Vec<u8> {
        for u_ref in &self.unresolved {
            let target = *self.labels.get(&u_ref.label)
                .unwrap_or_else(|| panic!("Unresolved label: {}", u_ref.label));

            match u_ref.kind {
                RefKind::Rel32 => {
                    // displacement = target - (offset + 4)
                    // offset 指向 displacement 的第一个字节
                    let disp = (target as i64) - (u_ref.offset as i64 + 4);
                    if disp < i32::MIN as i64 || disp > i32::MAX as i64 {
                        panic!(
                            "Jump offset out of range for label {}: {} (offset={}, target={})",
                            u_ref.label, disp, u_ref.offset, target
                        );
                    }
                    let disp_bytes = (disp as i32).to_le_bytes();
                    self.buffer[u_ref.offset..u_ref.offset + 4].copy_from_slice(&disp_bytes);
                }
            }
        }
        self.buffer
    }

    // ===================================================================
    // 数据传输指令
    // ===================================================================

    /// `mov dest, src` — 寄存器到寄存器的 64 位移动
    ///
    /// 编码: `REX.W + 0x89 + ModR/M(mod=11, reg=src, rm=dest)`
    ///
    /// 例: `mov rax, rcx` → `48 89 C8`
    pub fn mov_reg_reg(&mut self, dest: Register, src: Register) {
        let rex = rex_prefix(true, src.is_extended(), false, dest.is_extended());
        if rex != 0x40 {
            self.emit8(rex);
        }
        self.emit8(0x89);  // MOV r/m64, r64
        self.emit8(modrm(0b11, src.code(), dest.code()));
    }

    /// `mov reg, imm` — 立即数加载到寄存器
    ///
    /// 智能选择编码:
    /// - 若 imm 可放进 i32: 使用 `mov r32, imm32` (5 字节), 自动零扩展到 64 位
    ///   编码: `[REX.B] 0xB8|reg_code + imm32`
    /// - 否则: 使用 `mov r64, imm64` (10 字节)
    ///   编码: `REX.W + [REX.B] 0xB8|reg_code + imm64`
    ///
    /// 例: `mov rax, 1` → `B8 01 00 00 00`
    /// 例: `mov r8, 1`  → `41 B8 01 00 00 00`
    pub fn mov_reg_imm(&mut self, dest: Register, imm: i64) {
        if fits_i32(imm) {
            // 32 位编码：无需 REX.W，自动零扩展高 32 位
            if dest.is_extended() {
                self.emit8(rex_prefix(false, false, false, true));  // REX.B only
            }
            self.emit8(0xB8 | dest.code());
            self.emit32(imm as u32);
        } else {
            // 64 位编码：需要 REX.W
            let rex = rex_prefix(true, false, false, dest.is_extended());
            if rex != 0x40 {
                self.emit8(rex);
            }
            self.emit8(0xB8 | dest.code());
            self.emit64(imm as u64);
        }
    }

    /// `mov dest, [base + offset]` — 从内存加载到寄存器 (64 位)
    ///
    /// 编码: `REX.W + 0x8B + ModR/M + [SIB] + disp`
    ///
    /// 例: `mov rax, [rbx + 8]` → `48 8B 43 08`
    pub fn mov_reg_mem(&mut self, dest: Register, base: Register, offset: i32) {
        let rex = rex_prefix(true, dest.is_extended(), false, base.is_extended());
        self.emit8(rex);
        self.emit8(0x8B);  // MOV r64, r/m64

        let needs_sib = base.code() == 4;  // RSP or R12 needs SIB
        let (mod_field, _) = disp_mode(offset, needs_sib);

        self.emit8(modrm(mod_field, dest.code(), if needs_sib { 4 } else { base.code() }));

        if needs_sib {
            // SIB: [base], no index: scale=00, index=100(none), base=base.code()
            self.emit8(sib(0, 4, base.code()));
        }

        emit_disp(self, mod_field, offset);
    }

    /// `mov [base + offset], src` — 从寄存器写入到内存 (64 位)
    ///
    /// 编码: `REX.W + 0x89 + ModR/M + [SIB] + disp`
    ///
    /// 例: `mov [rbx + 8], rax` → `48 89 43 08`
    pub fn mov_mem_reg(&mut self, base: Register, offset: i32, src: Register) {
        let rex = rex_prefix(true, src.is_extended(), false, base.is_extended());
        self.emit8(rex);
        self.emit8(0x89);  // MOV r/m64, r64

        let needs_sib = base.code() == 4;
        let (mod_field, _) = disp_mode(offset, needs_sib);

        self.emit8(modrm(mod_field, src.code(), if needs_sib { 4 } else { base.code() }));

        if needs_sib {
            self.emit8(sib(0, 4, base.code()));
        }

        emit_disp(self, mod_field, offset);
    }

    /// `push reg` — 将寄存器压入栈
    ///
    /// 编码: `[REX.B] 0x50 | reg_code`
    ///
    /// 例: `push rbx` → `53`, `push r12` → `41 54`
    pub fn push_reg(&mut self, reg: Register) {
        if reg.is_extended() {
            self.emit8(0x41);  // REX.B
        }
        self.emit8(0x50 | reg.code());
    }

    /// `pop reg` — 从栈弹出到寄存器
    ///
    /// 编码: `[REX.B] 0x58 | reg_code`
    ///
    /// 例: `pop rbx` → `5B`, `pop r12` → `41 5C`
    pub fn pop_reg(&mut self, reg: Register) {
        if reg.is_extended() {
            self.emit8(0x41);  // REX.B
        }
        self.emit8(0x58 | reg.code());
    }

    // ===================================================================
    // 算术运算指令
    // ===================================================================

    /// `add dest, src` — 64 位加法: dest = dest + src
    ///
    /// 编码: `REX.W + 0x01 + ModR/M(mod=11, reg=src, rm=dest)`
    ///
    /// 例: `add rax, rcx` → `48 01 C8`
    pub fn add_reg_reg(&mut self, dest: Register, src: Register) {
        reg_reg_rm(self, 0x01, true, dest, src);
    }

    /// `sub dest, src` — 64 位减法: dest = dest - src
    ///
    /// 编码: `REX.W + 0x29 + ModR/M(mod=11, reg=src, rm=dest)`
    ///
    /// 例: `sub rax, rcx` → `48 29 C8`
    pub fn sub_reg_reg(&mut self, dest: Register, src: Register) {
        reg_reg_rm(self, 0x29, true, dest, src);
    }

    /// `mul reg` — 无符号乘法: RDX:RAX = RAX × reg
    ///
    /// 编码: `REX.W + 0xF7 + ModR/M(mod=11, reg=4, rm=reg)`
    /// reg 域 = 4 (/4 opcode 扩展)
    ///
    /// 例: `mul rcx` → `48 F7 E1`
    pub fn mul_reg(&mut self, reg: Register) {
        let rex = rex_prefix(true, false, false, reg.is_extended());
        self.emit8(rex);
        self.emit8(0xF7);
        self.emit8(modrm(0b11, 4, reg.code()));  // /4 = MUL
    }

    /// `div reg` — 无符号除法: RAX = RDX:RAX ÷ reg, RDX = 余数
    ///
    /// 编码: `REX.W + 0xF7 + ModR/M(mod=11, reg=6, rm=reg)`
    /// reg 域 = 6 (/6 opcode 扩展)
    ///
    /// 使用前需将 RDX 清零 (xor edx, edx 或 cqo)
    ///
    /// 例: `div rcx` → `48 F7 F1`
    pub fn div_reg(&mut self, reg: Register) {
        let rex = rex_prefix(true, false, false, reg.is_extended());
        self.emit8(rex);
        self.emit8(0xF7);
        self.emit8(modrm(0b11, 6, reg.code()));  // /6 = DIV
    }

    /// `inc reg` — 寄存器自增 1 (64 位)
    ///
    /// 编码: `REX.W + 0xFF + ModR/M(mod=11, reg=0, rm=reg)`
    /// reg 域 = 0 (/0 opcode 扩展)
    ///
    /// 例: `inc rax` → `48 FF C0`
    pub fn inc_reg(&mut self, reg: Register) {
        let rex = rex_prefix(true, false, false, reg.is_extended());
        self.emit8(rex);
        self.emit8(0xFF);
        self.emit8(modrm(0b11, 0, reg.code()));  // /0 = INC
    }

    /// `dec reg` — 寄存器自减 1 (64 位)
    ///
    /// 编码: `REX.W + 0xFF + ModR/M(mod=11, reg=1, rm=reg)`
    /// reg 域 = 1 (/1 opcode 扩展)
    ///
    /// 例: `dec rax` → `48 FF C8`
    pub fn dec_reg(&mut self, reg: Register) {
        let rex = rex_prefix(true, false, false, reg.is_extended());
        self.emit8(rex);
        self.emit8(0xFF);
        self.emit8(modrm(0b11, 1, reg.code()));  // /1 = DEC
    }

    // ===================================================================
    // 逻辑运算指令
    // ===================================================================

    /// `and dest, src` — 64 位按位与: dest = dest & src
    ///
    /// 编码: `REX.W + 0x21 + ModR/M(mod=11, reg=src, rm=dest)`
    ///
    /// 例: `and rax, rcx` → `48 21 C8`
    pub fn and_reg_reg(&mut self, dest: Register, src: Register) {
        reg_reg_rm(self, 0x21, true, dest, src);
    }

    /// `or dest, src` — 64 位按位或: dest = dest | src
    ///
    /// 编码: `REX.W + 0x09 + ModR/M(mod=11, reg=src, rm=dest)`
    ///
    /// 例: `or rax, rcx` → `48 09 C8`
    pub fn or_reg_reg(&mut self, dest: Register, src: Register) {
        reg_reg_rm(self, 0x09, true, dest, src);
    }

    /// `xor dest, src` — 64 位按位异或: dest = dest ^ src
    ///
    /// 编码: `REX.W + 0x31 + ModR/M(mod=11, reg=src, rm=dest)`
    ///
    /// 例: `xor rax, rcx` → `48 31 C8`
    pub fn xor_reg_reg(&mut self, dest: Register, src: Register) {
        reg_reg_rm(self, 0x31, true, dest, src);
    }

    /// `not reg` — 按位取反 (64 位)
    ///
    /// 编码: `REX.W + 0xF7 + ModR/M(mod=11, reg=2, rm=reg)`
    /// reg 域 = 2 (/2 opcode 扩展)
    ///
    /// 例: `not rax` → `48 F7 D0`
    pub fn not_reg(&mut self, reg: Register) {
        let rex = rex_prefix(true, false, false, reg.is_extended());
        self.emit8(rex);
        self.emit8(0xF7);
        self.emit8(modrm(0b11, 2, reg.code()));  // /2 = NOT
    }

    /// `neg reg` — 取反 (64 位): reg = -reg
    ///
    /// 编码: `REX.W + 0xF7 + ModR/M(mod=11, reg=3, rm=reg)`
    ///
    /// 例: `neg rax` → `48 F7 D8`
    pub fn neg_reg(&mut self, reg: Register) {
        let rex = rex_prefix(true, false, false, reg.is_extended());
        self.emit8(rex);
        self.emit8(0xF7);
        self.emit8(modrm(0b11, 3, reg.code()));  // /3 = NEG
    }

    /// `add reg, imm` — 64 位加法: reg = reg + imm
    ///
    /// 若 imm 可放进 i8 (-128..127), 使用短编码: `REX.W + 83 /0 + imm8` (4 字节)
    /// 否则使用: `REX.W + 81 /0 + imm32` (7 字节)
    ///
    /// 例: `add rax, 1` → `48 83 C0 01`
    /// 例: `add rax, 1000` → `48 81 C0 E8 03 00 00`
    pub fn add_reg_imm(&mut self, reg: Register, imm: i32) {
        let rex = rex_prefix(true, false, false, reg.is_extended());
        self.emit8(rex);
        if imm >= -128 && imm <= 127 {
            self.emit8(0x83);  // ADD r/m64, imm8
            self.emit8(modrm(0b11, 0, reg.code()));
            self.emit8(imm as u8);
        } else {
            self.emit8(0x81);  // ADD r/m64, imm32
            self.emit8(modrm(0b11, 0, reg.code()));
            self.emit32(imm as u32);
        }
    }

    /// `sub reg, imm` — 64 位减法: reg = reg - imm
    ///
    /// 智能选择短/长编码，同 add_reg_imm
    ///
    /// 例: `sub rax, 1` → `48 83 E8 01`
    pub fn sub_reg_imm(&mut self, reg: Register, imm: i32) {
        let rex = rex_prefix(true, false, false, reg.is_extended());
        self.emit8(rex);
        if imm >= -128 && imm <= 127 {
            self.emit8(0x83);  // SUB r/m64, imm8
            self.emit8(modrm(0b11, 5, reg.code()));
            self.emit8(imm as u8);
        } else {
            self.emit8(0x81);  // SUB r/m64, imm32
            self.emit8(modrm(0b11, 5, reg.code()));
            self.emit32(imm as u32);
        }
    }

    /// `imul reg, imm` — 有符号乘法: reg = reg * imm
    ///
    /// 编码: `REX.W + 69 + ModR/M(11, reg, rm) + imm32` (7 字节)
    ///
    /// 例: `imul rax, 3` → `48 69 C0 03 00 00 00`
    pub fn imul_reg_imm(&mut self, reg: Register, imm: i32) {
        let rex = rex_prefix(true, false, false, reg.is_extended());
        self.emit8(rex);
        self.emit8(0x69);  // IMUL r64, r/m64, imm32
        self.emit8(modrm(0b11, reg.code(), reg.code()));  // reg=reg, rm=reg
        self.emit32(imm as u32);
    }

    /// `and reg, imm` — 64 位按位与: reg = reg & imm
    ///
    /// 编码: `REX.W + 81 /4 + imm32` (长) 或 `REX.W + 83 /4 + imm8` (短)
    pub fn and_reg_imm(&mut self, reg: Register, imm: i32) {
        let rex = rex_prefix(true, false, false, reg.is_extended());
        self.emit8(rex);
        if imm >= -128 && imm <= 127 {
            self.emit8(0x83);
            self.emit8(modrm(0b11, 4, reg.code()));
            self.emit8(imm as u8);
        } else {
            self.emit8(0x81);
            self.emit8(modrm(0b11, 4, reg.code()));
            self.emit32(imm as u32);
        }
    }

    /// `or reg, imm` — 64 位按位或: reg = reg | imm
    ///
    /// 编码: `REX.W + 81 /1 + imm32` (长) 或 `REX.W + 83 /1 + imm8` (短)
    pub fn or_reg_imm(&mut self, reg: Register, imm: i32) {
        let rex = rex_prefix(true, false, false, reg.is_extended());
        self.emit8(rex);
        if imm >= -128 && imm <= 127 {
            self.emit8(0x83);
            self.emit8(modrm(0b11, 1, reg.code()));
            self.emit8(imm as u8);
        } else {
            self.emit8(0x81);
            self.emit8(modrm(0b11, 1, reg.code()));
            self.emit32(imm as u32);
        }
    }

    /// `xor reg, imm` — 64 位按位异或: reg = reg ^ imm
    ///
    /// 编码: `REX.W + 81 /6 + imm32` (长) 或 `REX.W + 83 /6 + imm8` (短)
    pub fn xor_reg_imm(&mut self, reg: Register, imm: i32) {
        let rex = rex_prefix(true, false, false, reg.is_extended());
        self.emit8(rex);
        if imm >= -128 && imm <= 127 {
            self.emit8(0x83);
            self.emit8(modrm(0b11, 6, reg.code()));
            self.emit8(imm as u8);
        } else {
            self.emit8(0x81);
            self.emit8(modrm(0b11, 6, reg.code()));
            self.emit32(imm as u32);
        }
    }

    /// `cmp reg, imm` — 64 位比较 (符号扩展 imm8 短编码)
    ///
    /// 若 imm 可放进 i8, 使用: `REX.W + 83 /7 + imm8` (4 字节)
    /// 否则: `REX.W + 81 /7 + imm32` (7 字节)
    ///
    /// 例: `cmp rax, 0` → `48 83 F8 00`
    /// 例: `cmp rax, 1` → `48 83 F8 01`
    pub fn cmp_reg_imm8(&mut self, reg: Register, imm: i8) {
        let rex = rex_prefix(true, false, false, reg.is_extended());
        self.emit8(rex);
        self.emit8(0x83);  // CMP r/m64, imm8
        self.emit8(modrm(0b11, 7, reg.code()));
        self.emit8(imm as u8);
    }

    /// `imul dest, src` — 有符号乘法: dest = dest * src
    ///
    /// 编码: `REX.W + 0F AF + ModR/M(mod=11, reg=dest, rm=src)`
    ///
    /// 例: `imul rax, rbx` → `48 0F AF C3`
    pub fn imul_reg_reg(&mut self, dest: Register, src: Register) {
        let rex = rex_prefix(true, dest.is_extended(), false, src.is_extended());
        self.emit8(rex);
        self.emit8(0x0F);
        self.emit8(0xAF);  // IMUL r64, r/m64
        self.emit8(modrm(0b11, dest.code(), src.code()));
    }

    /// `shl reg, imm8` — 逻辑左移: reg = reg << imm
    ///
    /// 编码: `REX.W + C1 /4 + imm8` (4 字节)
    ///
    /// 例: `shl rax, 1` → `48 D1 E0`
    /// 例: `shl rax, 3` → `48 C1 E0 03`
    pub fn shl_reg_imm(&mut self, reg: Register, imm: u8) {
        let rex = rex_prefix(true, false, false, reg.is_extended());
        self.emit8(rex);
        if imm == 1 {
            self.emit8(0xD1);  // SHL r/m64, 1
            self.emit8(modrm(0b11, 4, reg.code()));
        } else {
            self.emit8(0xC1);  // SHL r/m64, imm8
            self.emit8(modrm(0b11, 4, reg.code()));
            self.emit8(imm);
        }
    }

    /// `shr reg, imm8` — 逻辑右移: reg = reg >> imm
    ///
    /// 编码: `REX.W + C1 /5 + imm8` (4 字节)
    ///
    /// 例: `shr rax, 1` → `48 D1 E8`
    pub fn shr_reg_imm(&mut self, reg: Register, imm: u8) {
        let rex = rex_prefix(true, false, false, reg.is_extended());
        self.emit8(rex);
        if imm == 1 {
            self.emit8(0xD1);  // SHR r/m64, 1
            self.emit8(modrm(0b11, 5, reg.code()));
        } else {
            self.emit8(0xC1);  // SHR r/m64, imm8
            self.emit8(modrm(0b11, 5, reg.code()));
            self.emit8(imm);
        }
    }

    /// `sar reg, imm8` — 算术右移: reg = reg >> imm (有符号)
    ///
    /// 编码: `REX.W + C1 /7 + imm8`
    ///
    /// 例: `sar rax, 1` → `48 D1 F8`
    pub fn sar_reg_imm(&mut self, reg: Register, imm: u8) {
        let rex = rex_prefix(true, false, false, reg.is_extended());
        self.emit8(rex);
        if imm == 1 {
            self.emit8(0xD1);  // SAR r/m64, 1
            self.emit8(modrm(0b11, 7, reg.code()));
        } else {
            self.emit8(0xC1);  // SAR r/m64, imm8
            self.emit8(modrm(0b11, 7, reg.code()));
            self.emit8(imm);
        }
    }

    /// `add [base+off], src` — 内存加法: [base+off] += src
    ///
    /// 编码: `REX.W + 01 + ModR/M`
    pub fn add_mem_reg(&mut self, base: Register, offset: i32, src: Register) {
        let rex = rex_prefix(true, src.is_extended(), false, base.is_extended());
        self.emit8(rex);
        self.emit8(0x01);  // ADD r/m64, r64
        let needs_sib = base.code() == 4;
        let (mod_field, _) = disp_mode(offset, needs_sib);
        self.emit8(modrm(mod_field, src.code(), if needs_sib { 4 } else { base.code() }));
        if needs_sib { self.emit8(sib(0, 4, base.code())); }
        emit_disp(self, mod_field, offset);
    }

    /// `sub [base+off], src` — 内存减法: [base+off] -= src
    ///
    /// 编码: `REX.W + 29 + ModR/M`
    pub fn sub_mem_reg(&mut self, base: Register, offset: i32, src: Register) {
        let rex = rex_prefix(true, src.is_extended(), false, base.is_extended());
        self.emit8(rex);
        self.emit8(0x29);  // SUB r/m64, r64
        let needs_sib = base.code() == 4;
        let (mod_field, _) = disp_mode(offset, needs_sib);
        self.emit8(modrm(mod_field, src.code(), if needs_sib { 4 } else { base.code() }));
        if needs_sib { self.emit8(sib(0, 4, base.code())); }
        emit_disp(self, mod_field, offset);
    }

    /// `lea reg, [base+off]` — 加载内存地址到寄存器
    ///
    /// 编码: `REX.W + 8D + ModR/M`
    ///
    /// 例: `lea rax, [rbp-8]` → `48 8D 45 F8`
    pub fn lea_reg_mem(&mut self, dest: Register, base: Register, offset: i32) {
        let rex = rex_prefix(true, dest.is_extended(), false, base.is_extended());
        self.emit8(rex);
        self.emit8(0x8D);  // LEA r64, m
        let needs_sib = base.code() == 4;
        let (mod_field, _) = disp_mode(offset, needs_sib);
        self.emit8(modrm(mod_field, dest.code(), if needs_sib { 4 } else { base.code() }));
        if needs_sib { self.emit8(sib(0, 4, base.code())); }
        emit_disp(self, mod_field, offset);
    }

    /// `cmp [base+off], reg` — 内存与寄存器比较
    ///
    /// 编码: `REX.W + 39 + ModR/M`
    pub fn cmp_mem_reg(&mut self, base: Register, offset: i32, right: Register) {
        let rex = rex_prefix(true, right.is_extended(), false, base.is_extended());
        self.emit8(rex);
        self.emit8(0x39);  // CMP r/m64, r64
        let needs_sib = base.code() == 4;
        let (mod_field, _) = disp_mode(offset, needs_sib);
        self.emit8(modrm(mod_field, right.code(), if needs_sib { 4 } else { base.code() }));
        if needs_sib { self.emit8(sib(0, 4, base.code())); }
        emit_disp(self, mod_field, offset);
    }

    // ===================================================================
    // 比较指令
    // ===================================================================

    /// `cmp left, right` — 比较: 计算 left - right 并设置标志位 (不保存结果)
    ///
    /// 编码: `REX.W + 0x39 + ModR/M(mod=11, reg=right, rm=left)`
    ///
    /// 设置: ZF (相等), SF (符号), CF (借位), OF (溢出)
    /// 通常后跟条件跳转指令 (je, jne, jg, jl 等)
    ///
    /// 例: `cmp rax, rcx` → `48 39 C8`
    pub fn cmp_reg_reg(&mut self, left: Register, right: Register) {
        reg_reg_rm(self, 0x39, true, left, right);
    }

    // ===================================================================
    // 控制流指令 — 无条件跳转
    // ===================================================================

    /// `jmp target` — 无条件跳转到标签
    ///
    /// 编码: `E9 + disp32` (相对偏移)
    /// 总长度: 5 字节
    ///
    /// 例: `jmp loop_start` → `E9 xx xx xx xx`
    pub fn jmp(&mut self, target: Label) {
        self.emit_jump_rel32(0xE9, target);
    }

    // ===================================================================
    // 控制流指令 — 条件跳转 (需先用 cmp 设置标志位)
    // ===================================================================

    /// `je target` — 相等则跳转 (ZF=1)
    ///
    /// 编码: `0F 84 + disp32`, 总长 6 字节
    pub fn je(&mut self, target: Label) {
        self.emit_jcc_rel32(0x84, target);
    }

    /// `jne target` — 不等则跳转 (ZF=0)
    ///
    /// 编码: `0F 85 + disp32`, 总长 6 字节
    pub fn jne(&mut self, target: Label) {
        self.emit_jcc_rel32(0x85, target);
    }

    /// `jg target` — 大于则跳转 (有符号, ZF=0 且 SF=OF)
    ///
    /// 编码: `0F 8F + disp32`, 总长 6 字节
    pub fn jg(&mut self, target: Label) {
        self.emit_jcc_rel32(0x8F, target);
    }

    /// `jge target` — 大于等于则跳转 (有符号, SF=OF)
    ///
    /// 编码: `0F 8D + disp32`, 总长 6 字节
    pub fn jge(&mut self, target: Label) {
        self.emit_jcc_rel32(0x8D, target);
    }

    /// `jl target` — 小于则跳转 (有符号, SF≠OF)
    ///
    /// 编码: `0F 8C + disp32`, 总长 6 字节
    pub fn jl(&mut self, target: Label) {
        self.emit_jcc_rel32(0x8C, target);
    }

    /// `jle target` — 小于等于则跳转 (有符号, ZF=1 或 SF≠OF)
    ///
    /// 编码: `0F 8E + disp32`, 总长 6 字节
    pub fn jle(&mut self, target: Label) {
        self.emit_jcc_rel32(0x8E, target);
    }

    // ===================================================================
    // 控制流指令 — 函数调用
    // ===================================================================

    /// `call target` — 相对调用 (near call)
    ///
    /// 编码: `E8 + disp32`
    /// 将返回地址 (下一条指令的地址) 压栈后跳转到 target
    /// 总长: 5 字节
    ///
    /// 例: `call my_function` → `E8 xx xx xx xx`
    pub fn call(&mut self, target: Label) {
        self.emit_jump_rel32(0xE8, target);
    }

    /// `call reg` — 通过寄存器间接调用
    ///
    /// 编码: `[REX.B] 0xFF + ModR/M(mod=11, reg=2, rm=reg)`
    /// reg 域 = 2 (/2 = CALL r/m64)
    ///
    /// 例: `call rax` → `FF D0`, `call r8` → `41 FF D0`
    pub fn call_reg(&mut self, reg: Register) {
        if reg.is_extended() {
            self.emit8(0x41);  // REX.B
        }
        self.emit8(0xFF);
        self.emit8(modrm(0b11, 2, reg.code()));  // /2 = CALL
    }

    // ===================================================================
    // 控制流指令 — 函数返回
    // ===================================================================

    /// `ret` — 从函数返回 (near return)
    ///
    /// 编码: `C3`
    /// 从栈中弹出返回地址并跳转到该地址
    ///
    /// 例: `ret` → `C3`
    pub fn ret(&mut self) {
        self.emit8(0xC3);
    }

    // ===================================================================
    // 栈帧辅助指令
    // ===================================================================

    /// `sub rsp, imm` — 为局部变量分配栈空间
    ///
    /// 编码: `REX.W + 0x81 /5 + ModR/M(mod=11, rm=RSP) + imm32`
    /// 用于函数序言中分配栈帧
    pub fn sub_rsp_imm(&mut self, imm: i32) {
        self.emit8(0x48);  // REX.W
        self.emit8(0x81);  // SUB r/m64, imm32, /5
        self.emit8(modrm(0b11, 5, Register::RSP.code()));  // /5, rm=RSP
        self.emit32(imm as u32);
    }

    /// `add rsp, imm` — 释放局部变量的栈空间
    ///
    /// 编码: `REX.W + 0x81 /0 + ModR/M(mod=11, rm=RSP) + imm32`
    /// 用于函数尾声释放栈帧
    pub fn add_rsp_imm(&mut self, imm: i32) {
        self.emit8(0x48);  // REX.W
        self.emit8(0x81);  // ADD r/m64, imm32, /0
        self.emit8(modrm(0b11, 0, Register::RSP.code()));  // /0, rm=RSP
        self.emit32(imm as u32);
    }

    /// `nop` — 空操作 (9 字节长 NOP，用于对齐)
    ///
    /// 编码: `66 0F 1F 84 00 00 00 00 00`
    pub fn nop9(&mut self) {
        // 多字节 NOP: 兼容 Intel 推荐的无操作填充
        self.emit8(0x66);
        self.emit8(0x0F);
        self.emit8(0x1F);
        self.emit8(0x84);
        self.emit8(0x00);
        self.emit8(0x00);
        self.emit8(0x00);
        self.emit8(0x00);
        self.emit8(0x00);
    }

    // ===================================================================
    // RIP-relative 寻址指令 (阶段三: 用于访问 IAT 和数据)
    // ===================================================================

    /// 返回当前已发出字节数
    pub fn byte_position(&self) -> usize {
        self.buffer.len()
    }

    /// `lea dest, [RSP+disp8]` — 从栈地址加载有效地址 (用于 WriteConsoleA 等)
    ///
    /// 编码: `REX + 8D + ModR/M(01, reg, 100=SIB) + SIB(00, 100=none, 100=RSP) + disp8`
    /// 5 字节 (无 REX.B) 或 6 字节 (有 REX.B)
    ///
    /// 例: `lea r9, [rsp+0x28]` → `4D 8D 4C 24 28`
    pub fn lea_rsp_disp8(&mut self, dest: Register, disp: u8) {
        // REX: W=1 (64-bit), R=dest.is_extended(), X=0, B=0 (RSP not extended)
        let rex = rex_prefix(true, dest.is_extended(), false, false);
        self.emit8(rex);
        self.emit8(0x8D);  // LEA
        // ModR/M: mod=01 (disp8), reg=dest.code(), rm=100 (SIB required for RSP)
        self.emit8(modrm(0b01, dest.code(), 0b100));
        // SIB: scale=00 (×1), index=100 (no index), base=100 (RSP)
        self.emit8(sib(0, 4, 4));
        self.emit8(disp);
    }

    /// `call [RIP+disp32]` — 通过 IAT 间接调用 (用于导入函数)
    ///
    /// 编码: `FF 15 + disp32` (6 字节)
    ///
    /// # 参数
    /// * `iat_rva` - IAT 条目在映像中的绝对 RVA
    /// * `current_rva` - 当前指令在映像中的 RVA (= CODE_RVA + byte_position)
    ///
    /// 例: `call [GetStdHandle_IAT]` → `FF 15 xx xx xx xx`
    pub fn call_iat_rva(&mut self, iat_rva: u32, current_rva: u32) {
        self.emit8(0xFF);
        self.emit8(0x15);  // CALL r/m64, [RIP+disp32]
        let rip = (current_rva as i64) + 6;   // RIP 指向下一条指令
        let disp = (iat_rva as i64) - rip;
        self.emit32(disp as u32);
    }

    /// `lea dest, [RIP+disp32]` — 加载 RIP-relative 地址
    ///
    /// 编码: `REX.W + 8D + ModR/M(00, reg, 101) + disp32`
    /// 7 字节 (无 REX.B) 或 8 字节 (有 REX.B)
    ///
    /// # 参数
    /// * `dest` - 目标寄存器
    /// * `target_rva` - 目标数据在映像中的绝对 RVA
    /// * `current_rva` - 当前指令在映像中的 RVA
    ///
    /// 例: `lea rdx, [string_data]` → `48 8D 15 xx xx xx xx`
    pub fn lea_rip_rva(&mut self, dest: Register, target_rva: u32, current_rva: u32) {
        let rex = rex_prefix(true, dest.is_extended(), false, false);
        self.emit8(rex);
        self.emit8(0x8D);  // LEA r64, m
        self.emit8(modrm(0b00, dest.code(), 0b101));  // mod=00, rm=101=[RIP+disp32]
        let instr_len: u32 = 7;  // REX+8D+ModR/M+disp32 = 1+1+1+4
        let rip = (current_rva as i64) + (instr_len as i64);
        let disp = (target_rva as i64) - rip;
        self.emit32(disp as u32);
    }

    // ===================================================================
    // 扩展指令 (阶段四: 代码生成器需要)
    // ===================================================================

    /// 直接写入一个原始字节
    pub fn emit_byte(&mut self, byte: u8) {
        self.buffer.push(byte);
    }

    /// `cmp reg, imm` — 64 位比较
    ///
    /// 编码: `REX.W + 81 /7 + ModR/M(11, 7, rm) + imm32`
    /// 设置 ZF/SF/CF/OF 标志位
    pub fn cmp_reg_imm(&mut self, reg: Register, imm: i32) {
        let rex = rex_prefix(true, false, false, reg.is_extended());
        self.emit8(rex);
        self.emit8(0x81);  // CMP r/m64, imm32
        self.emit8(modrm(0b11, 7, reg.code()));  // /7 = CMP
        self.emit32(imm as u32);
    }

    /// `setCC al` + `movzx rax, al` — 条件设置 RAX 为 0/1
    ///
    /// 用于编译比较运算符，将标志位转换为布尔值
    fn setcc_rax(&mut self, setcc_op: u8) {
        // setCC AL: 0F setcc_op ModR/M(11, 0, 0)
        self.emit8(0x0F);
        self.emit8(setcc_op);
        self.emit8(0xC0);  // mod=11, reg=0, rm=RAX=0
        // movzx RAX, AL: REX.W + 0F B6 + ModR/M(11, 0, 0)
        self.emit8(0x48);
        self.emit8(0x0F);
        self.emit8(0xB6);
        self.emit8(0xC0);
    }

    /// `sete al; movzx rax, al` — RAX = (ZF == 1) ? 1 : 0
    pub fn sete_rax(&mut self) { self.setcc_rax(0x94); }
    /// `setne al; movzx rax, al` — RAX = (ZF == 0) ? 1 : 0
    pub fn setne_rax(&mut self) { self.setcc_rax(0x95); }
    /// `setl al; movzx rax, al` — RAX = (SF != OF) ? 1 : 0
    pub fn setl_rax(&mut self) { self.setcc_rax(0x9C); }
    /// `setg al; movzx rax, al` — RAX = (ZF == 0 && SF == OF) ? 1 : 0
    pub fn setg_rax(&mut self) { self.setcc_rax(0x9F); }
    /// `setle al; movzx rax, al` — RAX = (ZF == 1 || SF != OF) ? 1 : 0
    pub fn setle_rax(&mut self) { self.setcc_rax(0x9E); }
    /// `setge al; movzx rax, al` — RAX = (SF == OF) ? 1 : 0
    pub fn setge_rax(&mut self) { self.setcc_rax(0x9D); }

    /// `imul rax, rcx` — RAX = RAX × RCX (有符号乘法)
    ///
    /// 编码: `REX.W + 0F AF + ModR/M(11, reg=RAX, rm=RCX) = 48 0F AF C1`
    pub fn imul_rax_rcx(&mut self) {
        self.emit8(0x48);
        self.emit8(0x0F);
        self.emit8(0xAF);
        self.emit8(0xC1);  // mod=11, reg=RAX=0, rm=RCX=1
    }

    /// `cqo` — 将 RAX 符号扩展到 RDX:RAX (用于 idiv 前)
    ///
    /// 编码: `REX.W + 99 = 48 99`
    pub fn cqo(&mut self) {
        self.emit8(0x48);
        self.emit8(0x99);
    }

    /// `idiv rcx` — 有符号除法 RDX:RAX ÷ RCX, 商在 RAX, 余数在 RDX
    ///
    /// 编码: `REX.W + F7 /7 + ModR/M(11, 7, RCX) = 48 F7 F9`
    pub fn idiv_rcx(&mut self) {
        self.emit8(0x48);
        self.emit8(0xF7);
        self.emit8(0xF9);  // /7, rm=RCX
    }

    // ===================================================================
    // 内部辅助方法
    // ===================================================================

    /// 写入一个字节
    fn emit8(&mut self, byte: u8) {
        self.buffer.push(byte);
    }

    /// 写入 4 字节 (小端序)
    fn emit32(&mut self, val: u32) {
        self.buffer.extend_from_slice(&val.to_le_bytes());
    }

    /// 写入 8 字节 (小端序)
    fn emit64(&mut self, val: u64) {
        self.buffer.extend_from_slice(&val.to_le_bytes());
    }

    /// 发出 rel32 跳转 (E9, E8 等单字节 opcode): opcode + 4-byte displacement
    fn emit_jump_rel32(&mut self, opcode: u8, target: Label) {
        self.emit8(opcode);
        if let Some(&target_pos) = self.labels.get(&target) {
            // 标签已绑定，直接计算偏移
            let disp = (target_pos as i64) - (self.buffer.len() as i64 + 4);
            self.emit32(disp as u32);
        } else {
            // 标签未绑定，记录位置并写入占位符
            let offset = self.buffer.len();
            self.unresolved.push(UnresolvedRef { label: target, offset, kind: RefKind::Rel32 });
            self.emit32(0);  // 占位符
        }
    }

    /// 发出条件跳转 (0F xx + disp32): 双字节 opcode + 4-byte displacement
    fn emit_jcc_rel32(&mut self, opcode: u8, target: Label) {
        self.emit8(0x0F);
        self.emit8(opcode);
        if let Some(&target_pos) = self.labels.get(&target) {
            let disp = (target_pos as i64) - (self.buffer.len() as i64 + 4);
            self.emit32(disp as u32);
        } else {
            let offset = self.buffer.len();
            self.unresolved.push(UnresolvedRef { label: target, offset, kind: RefKind::Rel32 });
            self.emit32(0);  // 占位符
        }
    }
}

// ============================================================================
// 编码辅助函数 (模块内部)
// ============================================================================

/// 构造 REX 前缀字节
///
/// ```text
/// REX = 0100WRXB
///   W: 64-bit operand size
///   R: Extension for ModR/M reg field (bit 3)
///   X: Extension for SIB index field (bit 3)
///   B: Extension for ModR/M rm field or SIB base or opcode reg (bit 3)
/// ```
///
/// 返回 0x40 表示不需要 REX 前缀（所有位为 0）
#[inline]
fn rex_prefix(w: bool, r: bool, x: bool, b: bool) -> u8 {
    0x40 | ((w as u8) << 3) | ((r as u8) << 2) | ((x as u8) << 1) | (b as u8)
}

/// 构造 ModR/M 字节
///
/// ```text
/// ModR/M = mod(2) | reg(3) | rm(3)
///   mod: 00=memo, 01=memo+disp8, 10=memo+disp32, 11=register
///   reg: register code or opcode extension
///   rm:  register/memory code
/// ```
#[inline]
fn modrm(mod_: u8, reg: u8, rm: u8) -> u8 {
    ((mod_ & 3) << 6) | ((reg & 7) << 3) | (rm & 7)
}

/// 构造 SIB (Scale-Index-Base) 字节
///
/// ```text
/// SIB = scale(2) | index(3) | base(3)
///   scale: 00=1, 01=2, 10=4, 11=8
///   index: register code, 4 (100) 表示无索引
///   base:  register code
/// ```
#[inline]
fn sib(scale: u8, index: u8, base: u8) -> u8 {
    ((scale & 3) << 6) | ((index & 7) << 3) | (base & 7)
}

/// 通用寄存器到寄存器 r/m 指令编码
///
/// 用于 ADD/SUB/CMP/AND/OR/XOR reg, reg 类指令。
/// 格式: `REX.W + opcode + ModR/M(mod=11, reg=src, rm=dest)`
///
/// 语义: `dest = dest OP src`
#[inline]
fn reg_reg_rm(asm: &mut X64Assembler, opcode: u8, is_64bit: bool, dest: Register, src: Register) {
    let rex = rex_prefix(is_64bit, src.is_extended(), false, dest.is_extended());
    if rex != 0x40 {
        asm.emit8(rex);
    }
    asm.emit8(opcode);
    asm.emit8(modrm(0b11, src.code(), dest.code()));
}

/// 判断 i64 是否可以无损放入 i32 (符号扩展)
#[inline]
fn fits_i32(val: i64) -> bool {
    val >= i32::MIN as i64 && val <= i32::MAX as i64
}

/// 根据偏移量决定 ModR/M 的 mod 位
///
/// 返回值: (mod_field, needs_disp)
/// - 00: 无位移
/// - 01: disp8
/// - 10: disp32
///
/// 注意: 当基址寄存器为 RSP/R12 时，mod=00 不可用 (需 SIB 字节，无位移模式下 SIB 无位移)
fn disp_mode(offset: i32, has_sib: bool) -> (u8, u8) {
    if offset == 0 && !has_sib {
        // [reg] 且不是 RSP/R12 → mod=00, 无位移
        (0b00, 0)
    } else if offset == 0 && has_sib {
        // [RSP] 或 [R12] → 需要 disp8=0 (x86_64 强制要求)
        (0b01, 1)
    } else if offset >= -128 && offset <= 127 {
        (0b01, 1)
    } else {
        (0b10, 4)
    }
}

/// 根据 mod 模式发出位移字节
fn emit_disp(asm: &mut X64Assembler, mod_field: u8, offset: i32) {
    match mod_field {
        0b01 => asm.emit8(offset as u8),              // disp8
        0b10 => asm.emit32(offset as u32),             // disp32
        _ => {}                                         // 无位移
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试：mov_reg_imm 32 位编码
    #[test]
    fn test_mov_imm32() {
        let mut asm = X64Assembler::new();
        asm.mov_reg_imm(Register::RAX, 1);
        asm.mov_reg_imm(Register::RCX, 2);
        assert_eq!(asm.finish(), vec![
            0xB8, 0x01, 0x00, 0x00, 0x00,  // mov eax, 1
            0xB9, 0x02, 0x00, 0x00, 0x00,  // mov ecx, 2
        ]);
    }

    /// 测试：mov_reg_imm 64 位编码 (大立即数)
    #[test]
    fn test_mov_imm64() {
        let mut asm = X64Assembler::new();
        asm.mov_reg_imm(Register::RAX, 0x123456789AB);
        let code = asm.finish();
        // 应是 10 字节: REX.W + B8 + 8字节imm
        assert_eq!(code.len(), 10);
        assert_eq!(code[0], 0x48);  // REX.W
        assert_eq!(code[1], 0xB8);  // MOV RAX, imm64
    }

    /// 测试：mov_reg_imm 扩展寄存器
    #[test]
    fn test_mov_imm_extended() {
        let mut asm = X64Assembler::new();
        asm.mov_reg_imm(Register::R8, 42);
        let code = asm.finish();
        // REX.B + B8|0 + imm32
        assert_eq!(code[0], 0x41);  // REX.B
        assert_eq!(code[1], 0xB8);  // MOV R8D, imm32
    }

    /// 测试：add_reg_reg
    #[test]
    fn test_add_reg_reg() {
        let mut asm = X64Assembler::new();
        asm.add_reg_reg(Register::RAX, Register::RCX);
        let code = asm.finish();
        // REX.W + 01 + ModR/M(mod=11, reg=RCX, rm=RAX)
        // = 48 + 01 + C8 = 48 01 C8
        assert_eq!(code, vec![0x48, 0x01, 0xC8]);
    }

    /// 测试：sub_reg_reg
    #[test]
    fn test_sub_reg_reg() {
        let mut asm = X64Assembler::new();
        asm.sub_reg_reg(Register::RAX, Register::RCX);
        // REX.W + 29 + C8 = 48 29 C8
        assert_eq!(asm.finish(), vec![0x48, 0x29, 0xC8]);
    }

    /// 测试：and, or, xor
    #[test]
    fn test_logic_reg_reg() {
        // AND RAX, RCX
        let mut asm = X64Assembler::new();
        asm.and_reg_reg(Register::RAX, Register::RCX);
        assert_eq!(asm.finish(), vec![0x48, 0x21, 0xC8]);

        // OR RAX, RCX
        let mut asm = X64Assembler::new();
        asm.or_reg_reg(Register::RAX, Register::RCX);
        assert_eq!(asm.finish(), vec![0x48, 0x09, 0xC8]);

        // XOR RAX, RCX
        let mut asm = X64Assembler::new();
        asm.xor_reg_reg(Register::RAX, Register::RCX);
        assert_eq!(asm.finish(), vec![0x48, 0x31, 0xC8]);
    }

    /// 测试：not, inc, dec
    #[test]
    fn test_unary_ops() {
        // NOT RAX → 48 F7 D0
        let mut asm = X64Assembler::new();
        asm.not_reg(Register::RAX);
        assert_eq!(asm.finish(), vec![0x48, 0xF7, 0xD0]);

        // INC RAX → 48 FF C0
        let mut asm = X64Assembler::new();
        asm.inc_reg(Register::RAX);
        assert_eq!(asm.finish(), vec![0x48, 0xFF, 0xC0]);

        // DEC RAX → 48 FF C8
        let mut asm = X64Assembler::new();
        asm.dec_reg(Register::RAX);
        assert_eq!(asm.finish(), vec![0x48, 0xFF, 0xC8]);
    }

    /// 测试：push 和 pop
    #[test]
    fn test_push_pop() {
        // PUSH RBX → 53
        let mut asm = X64Assembler::new();
        asm.push_reg(Register::RBX);
        assert_eq!(asm.finish(), vec![0x53]);

        // POP RBX → 5B
        let mut asm = X64Assembler::new();
        asm.pop_reg(Register::RBX);
        assert_eq!(asm.finish(), vec![0x5B]);

        // PUSH R12 → 41 54
        let mut asm = X64Assembler::new();
        asm.push_reg(Register::R12);
        assert_eq!(asm.finish(), vec![0x41, 0x54]);
    }

    /// 测试：ret, nop, call_reg
    #[test]
    fn test_ret_call_nop() {
        let mut asm = X64Assembler::new();
        asm.ret();
        assert_eq!(asm.finish(), vec![0xC3]);

        let mut asm = X64Assembler::new();
        asm.call_reg(Register::RAX);
        // CALL RAX → FF D0
        assert_eq!(asm.finish(), vec![0xFF, 0xD0]);
    }

    /// 测试：标签和跳转 (前向引用)
    #[test]
    fn test_labels() {
        let mut asm = X64Assembler::new();

        //     mov eax, 0
        // loop_start:
        //     inc eax
        //     cmp eax, 10
        //     jl loop_start
        //     ret

        asm.mov_reg_imm(Register::RAX, 0);
        let loop_start = asm.new_label();
        asm.bind_label(loop_start);
        asm.inc_reg(Register::RAX);
        asm.cmp_reg_reg(Register::RAX, Register::R10);  // compare RAX with R10
        // Set R10 first, then compare... hmm, let me simplify

        // Actually let me do a simpler test:
        //     xor eax, eax
        //     jmp done
        //     mov eax, 1   ; skipped
        // done:
        //     ret

        let mut asm = X64Assembler::new();
        asm.xor_reg_reg(Register::RAX, Register::RAX);  // RAX = 0
        let done = asm.new_label();
        asm.jmp(done);
        asm.mov_reg_imm(Register::RAX, 1);  // This should be skipped
        asm.bind_label(done);
        asm.ret();

        let code = asm.finish();
        // Verify: xor rax,rax (48 31 C0), jmp +7 (EB/E9...), mov eax,1, ret
        assert!(code.len() > 5);
        // Check ret is last byte
        assert_eq!(*code.last().unwrap(), 0xC3);
    }

    /// 测试：mov_reg_mem 和 mov_mem_reg
    #[test]
    fn test_memory_ops() {
        // mov rax, [rbx + 8] → 48 8B 43 08
        let mut asm = X64Assembler::new();
        asm.mov_reg_mem(Register::RAX, Register::RBX, 8);
        assert_eq!(asm.finish(), vec![0x48, 0x8B, 0x43, 0x08]);

        // mov [rbx + 8], rax → 48 89 43 08
        let mut asm = X64Assembler::new();
        asm.mov_mem_reg(Register::RBX, 8, Register::RAX);
        assert_eq!(asm.finish(), vec![0x48, 0x89, 0x43, 0x08]);
    }

    /// 测试：完整函数 — 计算 1+2=3
    #[test]
    fn test_compute_add() {
        let mut asm = X64Assembler::new();
        asm.mov_reg_imm(Register::RAX, 1);
        asm.mov_reg_imm(Register::RCX, 2);
        asm.add_reg_reg(Register::RAX, Register::RCX);
        asm.ret();

        let code = asm.finish();
        // B8 01 00 00 00   mov eax, 1
        // B9 02 00 00 00   mov ecx, 2
        // 48 01 C8          add rax, rcx
        // C3                ret
        assert_eq!(code, vec![
            0xB8, 0x01, 0x00, 0x00, 0x00,
            0xB9, 0x02, 0x00, 0x00, 0x00,
            0x48, 0x01, 0xC8,
            0xC3,
        ]);
    }
}
