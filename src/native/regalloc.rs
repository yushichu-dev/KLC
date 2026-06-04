//! KLC 寄存器分配器 — 阶段二
//!
//! 使用简单线性扫描策略管理 x86_64 通用寄存器的分配与释放。
//! 严格遵循 Microsoft x64 调用约定。
//!
//! ## Microsoft x64 调用约定关键规则
//!
//! ### 参数传递
//! | 参数位置 | 整数/指针 | 浮点数 |
//! |---------|----------|--------|
//! | 第 1 个 | RCX      | XMM0   |
//! | 第 2 个 | RDX      | XMM1   |
//! | 第 3 个 | R8       | XMM2   |
//! | 第 4 个 | R9       | XMM3   |
//! | 第 5+   | 栈 (从右到左压入) | 栈 |
//!
//! ### 寄存器保留规则
//! | 类别 | 寄存器 | 说明 |
//! |------|--------|------|
//! | **被调用者保存** | RBX, RBP, RSI, RDI, R12-R15, XMM6-XMM15 | 函数必须保存和恢复 |
//! | **调用者保存(易失)** | RAX, RCX, RDX, R8-R11, XMM0-XMM5 | 调用后可任意修改 |
//!
//! ### 栈规则
//! - 栈在 call 指令前必须 16 字节对齐
//! - 调用者为被调用者在栈上预留 32 字节 Shadow Space
//! - 函数序言: push rbp; mov rbp, rsp; sub rsp, N
//! - 函数尾声: mov rsp, rbp; pop rbp; ret
//!
//! ### 返回值
//! - 整数/指针: RAX
//! - 浮点数:   XMM0

use crate::native::x64::Register;
use std::collections::{HashMap, HashSet};

// ============================================================================
// 寄存器分配器
// ============================================================================

/// 线性扫描寄存器分配器
///
/// 管理 x86_64 通用寄存器的分配/释放，处理栈帧布局和寄存器溢出。
///
/// # 使用流程
///
/// 1. `begin_function()` — 开始编译一个函数
/// 2. `allocate()` / `allocate_specific()` — 为变量分配寄存器
/// 3. `free()` — 释放不再需要的寄存器
/// 4. `end_function()` — 生成序言/尾声信息
///
/// # 分配策略
///
/// 优先使用调用者保存寄存器 (RAX, RCX, RDX, R8-R11)，
/// 用完后使用被调用者保存寄存器 (RBX, RSI, RDI, R12-R15)，
/// 全部用完则溢出到栈。
pub struct RegisterAllocator {
    /// 可用的调用者保存寄存器（易失）
    scratch_pool: Vec<Register>,
    /// 可用的被调用者保存寄存器
    saved_pool: Vec<Register>,
    /// 当前已分配的寄存器集合
    in_use: HashSet<Register>,
    /// 本次函数实际使用的被调用者保存寄存器（需要在序言/尾声中保存/恢复）
    used_saved: HashSet<Register>,
    /// 栈溢出槽偏移 (相对于 RBP，负值 = 局部变量)
    spill_offset: i32,
    /// 当前函数是否需要栈帧 (RBP)
    needs_frame: bool,
    /// 变量名 → 寄存器映射
    allocations: HashMap<String, Register>,
    /// 局部变量大小 (不包括被调用保存寄存器)
    local_var_size: i32,
}

impl RegisterAllocator {
    #![allow(dead_code)]
    /// 创建新的寄存器分配器
    ///
    /// 可用寄存器按优先级排列（调用者保存优先）
    pub fn new() -> Self {
        RegisterAllocator {
            // 调用者保存寄存器 (优先使用，不需保存/恢复)
            scratch_pool: vec![
                Register::RAX, Register::RCX, Register::RDX,
                Register::R8,  Register::R9,  Register::R10, Register::R11,
            ],
            // 被调用者保存寄存器 (次优先，需在序言/尾声处理)
            saved_pool: vec![
                Register::RBX, Register::RSI, Register::RDI,
                Register::R12, Register::R13, Register::R14, Register::R15,
            ],
            in_use: HashSet::new(),
            used_saved: HashSet::new(),
            spill_offset: 0,
            needs_frame: false,
            allocations: HashMap::new(),
            local_var_size: 0,
        }
    }

    /// 开始编译新函数，重置分配器状态
    pub fn begin_function(&mut self) {
        self.in_use.clear();
        self.used_saved.clear();
        self.spill_offset = 0;
        self.needs_frame = false;
        self.allocations.clear();
        self.local_var_size = 0;
    }

    /// 分配一个寄存器
    ///
    /// 按优先级搜索:
    /// 1. 首选调用者保存寄存器 (RAX, RCX, RDX, R8-R11)
    /// 2. 次选被调用者保存寄存器 (RBX, RSI, RDI, R12-R15)
    ///
    /// # Returns
    /// 分配的寄存器
    ///
    /// # Panics
    /// 如果没有可用寄存器且溢出未实现
    pub fn allocate(&mut self) -> Register {
        // 先从调用者保存池找
        for &reg in &self.scratch_pool {
            if !self.in_use.contains(&reg) {
                self.in_use.insert(reg);
                return reg;
            }
        }
        // 再从被调用者保存池找
        for &reg in &self.saved_pool {
            if !self.in_use.contains(&reg) {
                self.in_use.insert(reg);
                self.used_saved.insert(reg);
                self.needs_frame = true;
                return reg;
            }
        }

        // 所有寄存器都在使用中 → 需要溢出
        // 阶段二简单实现：分配一个栈槽
        self.spill_register()
    }

    /// 尝试分配指定的寄存器
    ///
    /// 调用约定要求特定参数放在特定寄存器中时使用。
    ///
    /// # Returns
    /// `Ok(())` 如果成功，`Err` 如果寄存器已被占用
    pub fn allocate_specific(&mut self, reg: Register) -> Result<(), String> {
        if self.in_use.contains(&reg) {
            return Err(format!("Register {:?} is already in use", reg));
        }
        self.in_use.insert(reg);
        // 检查是否是被调用者保存寄存器
        if self.saved_pool.contains(&reg) {
            self.used_saved.insert(reg);
            self.needs_frame = true;
        }
        Ok(())
    }

    /// 释放寄存器
    pub fn free(&mut self, reg: Register) {
        self.in_use.remove(&reg);
    }

    /// 为变量命名分配寄存器并记录映射
    pub fn alloc_var(&mut self, name: &str) -> Register {
        let reg = self.allocate();
        self.allocations.insert(name.to_string(), reg);
        reg
    }

    /// 获取变量对应的寄存器
    pub fn get_var_reg(&self, name: &str) -> Option<Register> {
        self.allocations.get(name).copied()
    }

    /// 释放变量及其寄存器
    pub fn free_var(&mut self, name: &str) {
        if let Some(reg) = self.allocations.remove(name) {
            self.free(reg);
        }
    }

    /// 返回当前函数实际使用的被调用者保存寄存器集合
    ///
    /// 用于生成函数序言 (push) 和尾声 (pop)
    pub fn used_saved_regs(&self) -> Vec<Register> {
        let mut regs: Vec<Register> = self.used_saved.iter().copied().collect();
        // 按寄存器编号排序以保证确定性
        regs.sort_by_key(|r| r.code());
        regs
    }

    /// 是否需要建立栈帧
    ///
    /// 当使用了被调用者保存寄存器或局部变量时需要
    pub fn needs_frame(&self) -> bool {
        self.needs_frame
    }

    /// 预留栈空间（用于局部变量溢出），返回栈偏移
    ///
    /// 偏移为负值 (相对于 RBP)，8 字节对齐。
    pub fn reserve_stack(&mut self, size: i32) -> i32 {
        // 向上对齐到 8 字节
        let aligned = (size + 7) & !7;
        self.spill_offset -= aligned;
        self.needs_frame = true;
        self.local_var_size += aligned;
        self.spill_offset
    }

    /// 获取当前栈帧总大小（不包括返回地址和保存的 RBP）
    pub fn frame_size(&self) -> i32 {
        let saved_regs_size = self.used_saved.len() as i32 * 8;
        let var_size = self.local_var_size;
        saved_regs_size + var_size
    }

    /// 获取参数寄存器列表 (按 Microsoft x64 约定)
    ///
    /// 返回用于传递前 4 个整数参数的寄存器
    pub fn param_registers() -> [Register; 4] {
        [Register::RCX, Register::RDX, Register::R8, Register::R9]
    }

    /// 获取返回值寄存器
    pub fn return_register() -> Register {
        Register::RAX
    }

    /// 溢出：强行分配一个寄存器（简单实现：用栈槽代替）
    fn spill_register(&mut self) -> Register {
        // 简单策略：溢出一个调用者保存寄存器 (让出给新变量)
        // 实际完整的溢出机制将在阶段五实现
        // 这里使用 R11 作为应急溢出寄存器
        let spill_reg = Register::R11;
        // 如果 R11 正好可用就用它
        if !self.in_use.contains(&spill_reg) {
            self.in_use.insert(spill_reg);
            return spill_reg;
        }
        // 最坏情况：使用 R10
        let spill_reg = Register::R10;
        if !self.in_use.contains(&spill_reg) {
            self.in_use.insert(spill_reg);
            return spill_reg;
        }
        panic!("Register allocator: all registers exhausted, spill not yet fully implemented");
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试：基本分配和释放
    #[test]
    fn test_allocate_free() {
        let mut ra = RegisterAllocator::new();
        ra.begin_function();

        let r1 = ra.allocate();
        assert!(ra.in_use.contains(&r1));

        ra.free(r1);
        assert!(!ra.in_use.contains(&r1));
    }

    /// 测试：耗尽调用者保存寄存器后使用被调用者保存
    #[test]
    fn test_saved_reg_usage() {
        let mut ra = RegisterAllocator::new();
        ra.begin_function();

        // 分配所有 7 个调用者保存寄存器
        let mut regs = Vec::new();
        for _ in 0..7 {
            regs.push(ra.allocate());
        }

        // 下一个分配应使用被调用者保存寄存器
        let saved_reg = ra.allocate();
        assert!(ra.used_saved.contains(&saved_reg));
        assert!(ra.needs_frame());
    }

    /// 测试：参数寄存器列表
    #[test]
    fn test_param_regs() {
        let params = RegisterAllocator::param_registers();
        assert_eq!(params[0], Register::RCX);
        assert_eq!(params[1], Register::RDX);
        assert_eq!(params[2], Register::R8);
        assert_eq!(params[3], Register::R9);
    }

    /// 测试：栈预留
    #[test]
    fn test_stack_reserve() {
        let mut ra = RegisterAllocator::new();
        ra.begin_function();

        let offset = ra.reserve_stack(12);  // 12 字节 → 对齐到 16 字节
        assert_eq!(offset, -16);
        assert_eq!(ra.frame_size(), 16);
    }
}
