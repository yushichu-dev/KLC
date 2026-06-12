//! KLC JIT 即时编译模块 (v1.3.6+)
//!
//! 基于 Cranelift 后端的栈式虚拟机 JIT 优化。
//!
//! 架构:
//!   - `hotspot.rs`  : 热点代码探测 (循环/高频函数执行计数)
//!   - `compiler.rs` : 字节码 → Cranelift IR → x64 本地机器码
//!   - `cache.rs`    : JIT 编译代码缓存管理
//!
//! 工作流程:
//!   1. VM 默认解释执行，每执行一条 Call/Jmp 指令，热点计数器 +1
//!   2. 计数器超过阈值 → 触发 JIT 编译
//!   3. 编译后的原生函数存入缓存
//!   4. 后续调用命中缓存 → 直接从解释器跳转到原生代码执行
//!   5. 原生代码执行异常 → 自动回落至解释模式

#![allow(dead_code)]

pub mod hotspot;
pub mod compiler;
pub mod cache;

use std::sync::atomic::{AtomicBool, Ordering};
use crate::bytecode::Instruction;

// ============================================================================
// JIT 配置
// ============================================================================

/// JIT 编译配置
#[derive(Debug, Clone)]
pub struct JitConfig {
    /// 是否启用 JIT (默认关闭)
    pub enable_jit: bool,
    /// 热点阈值: 函数/循环被执行超过此次数后触发 JIT 编译
    pub hot_threshold: u64,
    /// 单个函数最大 JIT 编译指令条数 (超过则跳过，避免编译时间过长)
    pub max_jit_instrs: usize,
    /// 是否输出 JIT 编译日志
    pub jit_debug: bool,
    /// JIT 缓存最大条目数
    pub max_cache_entries: usize,
}

impl Default for JitConfig {
    fn default() -> Self {
        Self {
            enable_jit: false,
            hot_threshold: 50,
            max_jit_instrs: 500,
            jit_debug: false,
            max_cache_entries: 256,
        }
    }
}

// ============================================================================
// JIT 编译结果
// ============================================================================

/// JIT 编译后的原生函数
pub struct CompiledNativeFn {
    /// 函数标识 (函数名 或 代码块 hash)
    pub id: String,
    /// 编译后的机器码 (可执行内存)
    pub code: ExecutableMemory,
    /// 机器码入口点地址
    pub entry: *const u8,
    /// 编译时的指令条数
    pub instr_count: usize,
}

/// 可执行内存块
pub struct ExecutableMemory {
    ptr: *mut u8,
    size: usize,
}

impl ExecutableMemory {
    /// 分配可读写可执行内存 (Windows VirtualAlloc / Unix mmap)
    pub fn allocate(code: &[u8]) -> Result<Self, String> {
        if code.is_empty() {
            return Err("JIT: empty code".into());
        }
        unsafe { Self::allocate_raw(code) }
    }

    #[cfg(windows)]
    unsafe fn allocate_raw(code: &[u8]) -> Result<Self, String> {
        use std::ffi::c_void;
        extern "system" {
            fn VirtualAlloc(
                lpAddress: *const c_void,
                dwSize: usize,
                flAllocationType: u32,
                flProtect: u32,
            ) -> *mut c_void;
            fn VirtualProtect(
                lpAddress: *const c_void,
                dwSize: usize,
                flNewProtect: u32,
                lpflOldProtect: *mut u32,
            ) -> i32;
        }

        const MEM_COMMIT: u32 = 0x1000;
        const MEM_RESERVE: u32 = 0x2000;
        const PAGE_READWRITE: u32 = 0x04;
        const PAGE_EXECUTE_READ: u32 = 0x20;

        let size = code.len().next_multiple_of(4096);

        // 使用 VirtualAlloc 分配内存（VirtualFree 需要 VirtualAlloc 分配的地址）
        let ptr = VirtualAlloc(
            std::ptr::null(),
            size,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_READWRITE,
        );
        if ptr.is_null() {
            return Err("JIT: VirtualAlloc failed".into());
        }

        // 写入机器码
        std::ptr::copy_nonoverlapping(code.as_ptr(), ptr as *mut u8, code.len());

        // 将内存改为可执行
        let mut old_protect: u32 = 0;
        let ret = VirtualProtect(
            ptr,
            size,
            PAGE_EXECUTE_READ,
            &mut old_protect,
        );
        if ret == 0 {
            // 降级: 使用 PAGE_EXECUTE_READWRITE
            const PAGE_EXECUTE_READWRITE: u32 = 0x40;
            let ret2 = VirtualProtect(
                ptr,
                size,
                PAGE_EXECUTE_READWRITE,
                &mut old_protect,
            );
            if ret2 == 0 {
                // 清理已分配的内存
                extern "system" {
                    fn VirtualFree(lpAddress: *const c_void, dwSize: usize, dwFreeType: u32) -> i32;
                }
                const MEM_RELEASE: u32 = 0x8000;
                let _ = VirtualFree(ptr, 0, MEM_RELEASE);
                return Err("JIT: VirtualProtect failed".into());
            }
        }

        Ok(Self { ptr: ptr as *mut u8, size })
    }

    #[cfg(not(windows))]
    unsafe fn allocate_raw(code: &[u8]) -> Result<Self, String> {
        let size = code.len().next_multiple_of(4096);
        let page_size = 4096;

        // mmap 可读写可执行内存
        let ptr = libc::mmap(
            std::ptr::null_mut(),
            size,
            libc::PROT_READ | libc::PROT_WRITE | libc::PROT_EXEC,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        );
        if ptr == libc::MAP_FAILED {
            return Err("JIT: mmap failed".into());
        }

        std::ptr::copy_nonoverlapping(code.as_ptr(), ptr as *mut u8, code.len());

        Ok(Self {
            ptr: ptr as *mut u8,
            size,
        })
    }

    /// 获取入口函数指针 (类型由调用者保证)
    pub fn entry<T>(&self) -> *const T {
        self.ptr as *const T
    }

    /// 代码大小
    pub fn len(&self) -> usize {
        self.size
    }
}

unsafe impl Send for ExecutableMemory {}
unsafe impl Sync for ExecutableMemory {}

impl Drop for ExecutableMemory {
    fn drop(&mut self) {
        unsafe {
            #[cfg(windows)]
            {
                use std::ffi::c_void;
                extern "system" {
                    fn VirtualFree(
                        lpAddress: *const c_void,
                        dwSize: usize,
                        dwFreeType: u32,
                    ) -> i32;
                }
                const MEM_RELEASE: u32 = 0x8000;
                VirtualFree(self.ptr as *const c_void, 0, MEM_RELEASE);
            }
            #[cfg(not(windows))]
            {
                libc::munmap(self.ptr as *mut libc::c_void, self.size);
            }
        }
    }
}

// ============================================================================
// JIT 安全检查结果
// ============================================================================

/// JIT 可编译性分析结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JitSafety {
    /// 可安全 JIT 编译: 仅包含 i64/f64/bool 运算与控制流
    Safe,
    /// 包含复杂类型操作，不可 JIT:
    /// 字符串操作、结构体、枚举、数组、Map、I/O、内置函数调用等
    Unsafe { reason: &'static str },
}

/// 检查单条指令是否可 JIT 编译
pub fn check_instruction_jit_safety(instr: &Instruction) -> JitSafety {
    match instr {
        // ✓ 算术运算 (仅 i64/f64)
        Instruction::Add | Instruction::Sub | Instruction::Mul
        | Instruction::Div | Instruction::Mod | Instruction::Neg => JitSafety::Safe,

        // ✓ 比较运算
        Instruction::Eq | Instruction::Neq | Instruction::Lt
        | Instruction::Gt | Instruction::Lte | Instruction::Gte => JitSafety::Safe,

        // ✓ 逻辑运算
        Instruction::And | Instruction::Or | Instruction::Not => JitSafety::Safe,

        // ✓ 控制流
        Instruction::Jmp(_) | Instruction::JmpFalse(_)
        | Instruction::Return | Instruction::Halt => JitSafety::Safe,

        // ✓ 栈操作
        Instruction::Const(_) | Instruction::Pop
        | Instruction::Load(_) | Instruction::Store(_)
        | Instruction::InitVar(_) => JitSafety::Safe,

        // ✓ 简单函数调用 (仅限同模块内的算术函数)
        Instruction::Call(_, _) => JitSafety::Safe,

        // ✗ 字符串操作 — 涉及堆分配
        Instruction::Concat | Instruction::ToString
        | Instruction::SubStr | Instruction::StrFind
        | Instruction::StrRepeat => {
            JitSafety::Unsafe { reason: "字符串操作涉及堆分配" }
        }

        // ✗ 结构体操作
        Instruction::StructNew(..) | Instruction::StructGet(_)
        | Instruction::StructSet(_) => {
            JitSafety::Unsafe { reason: "结构体操作涉及 Rc/RefCell 堆分配" }
        }

        // ✗ 枚举操作
        Instruction::EnumNew(..) | Instruction::EnumGet(_)
        | Instruction::IsVariant(_) | Instruction::RegFn(..) => {
            JitSafety::Unsafe { reason: "枚举/Lambda 操作涉及堆分配" }
        }

        // ✗ I/O 操作
        Instruction::Print | Instruction::PrintLn
        | Instruction::ReadLine => {
            JitSafety::Unsafe { reason: "I/O 操作需要解释器上下文" }
        }

        // ✗ 异步操作
        Instruction::Spawn(..) | Instruction::WaitAll => {
            JitSafety::Unsafe { reason: "异步操作需要线程调度" }
        }

        // ✓ NOP — VM 内部优化占位，JIT 编译时直接跳过
        Instruction::Nop => JitSafety::Safe,
    }
}

/// 检查整个指令序列是否可 JIT 编译
pub fn check_block_jit_safety(instructions: &[Instruction]) -> Result<(), &'static str> {
    for instr in instructions {
        match check_instruction_jit_safety(instr) {
            JitSafety::Safe => continue,
            JitSafety::Unsafe { reason } => return Err(reason),
        }
    }
    if instructions.len() > 500 {
        return Err("指令序列过长 (>500)");
    }
    Ok(())
}

// ============================================================================
// JIT 引擎 — 统一入口
// ============================================================================

/// JIT 引擎状态
static JIT_ENABLED: AtomicBool = AtomicBool::new(false);

/// 设置全局 JIT 开关
pub fn set_jit_enabled(enabled: bool) {
    JIT_ENABLED.store(enabled, Ordering::SeqCst);
}

/// 获取全局 JIT 开关状态
pub fn is_jit_enabled() -> bool {
    JIT_ENABLED.load(Ordering::SeqCst)
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jit_safety_arithmetic() {
        assert_eq!(check_instruction_jit_safety(&Instruction::Add), JitSafety::Safe);
        assert_eq!(check_instruction_jit_safety(&Instruction::Sub), JitSafety::Safe);
        assert_eq!(check_instruction_jit_safety(&Instruction::Mul), JitSafety::Safe);
    }

    #[test]
    fn test_jit_safety_unsafe() {
        assert!(matches!(
            check_instruction_jit_safety(&Instruction::Concat),
            JitSafety::Unsafe { .. }
        ));
        assert!(matches!(
            check_instruction_jit_safety(&Instruction::StructNew("Point".into(), 2)),
            JitSafety::Unsafe { .. }
        ));
        assert!(matches!(
            check_instruction_jit_safety(&Instruction::Print),
            JitSafety::Unsafe { .. }
        ));
    }

    #[test]
    fn test_block_jit_safety() {
        let safe_block = vec![
            Instruction::Const(0),
            Instruction::Const(1),
            Instruction::Add,
            Instruction::Return,
        ];
        assert!(check_block_jit_safety(&safe_block).is_ok());

        let unsafe_block = vec![
            Instruction::Const(0),
            Instruction::PrintLn,
            Instruction::Return,
        ];
        assert!(check_block_jit_safety(&unsafe_block).is_err());
    }

    #[test]
    fn test_executable_memory() {
        let code = vec![0xC3u8]; // x64 ret instruction
        let mem = ExecutableMemory::allocate(&code);
        assert!(mem.is_ok());
    }
}
