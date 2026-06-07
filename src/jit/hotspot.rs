//! KLC JIT 热点代码探测模块
//!
//! 统计循环回边、高频函数调用的字节码执行次数，
//! 达到阈值后自动触发 JIT 编译。

use std::collections::{HashMap, HashSet};
use crate::jit::JitConfig;

// ============================================================================
// 热点探测器
// ============================================================================

/// 热点探测器 — 跟踪代码块执行频率
pub struct HotSpotDetector {
    /// 配置
    config: JitConfig,
    /// 函数调用计数: (函数索引, ip位置) → 执行次数
    /// 用于跟踪哪个函数的哪个调用点被频繁执行
    func_counts: HashMap<usize, u64>,
    /// 循环回边计数: (函数索引, ip位置) → 迭代次数
    /// key = (func_index << 32) | ip , 支持主程序 (func_index = u32::MAX)
    loop_counts: HashMap<u64, u64>,
    /// 已触发 JIT 编译的集合 (避免重复编译) — HashSet 比 HashMap<u64, bool> 更高效
    jit_triggered: HashSet<u64>,
}

impl HotSpotDetector {
    /// 创建新的热点探测器
    pub fn new(config: JitConfig) -> Self {
        Self {
            config,
            func_counts: HashMap::new(),
            loop_counts: HashMap::new(),
            jit_triggered: HashSet::new(),
        }
    }

    /// 判断是否启用
    #[inline(always)]
    pub fn enabled(&self) -> bool {
        self.config.enable_jit
    }

    /// 记录函数调用
    /// 
    /// `func_idx`: 被调用函数索引 (VM 的 func_names 中位置)
    /// 返回 true 表示达到热点阈值，应触发 JIT 编译
    pub fn record_function_call(&mut self, func_idx: usize) -> bool {
        if !self.config.enable_jit {
            return false;
        }

        let count = self.func_counts.entry(func_idx).or_insert(0);
        *count += 1;

        if *count >= self.config.hot_threshold {
            let key = func_idx as u64;
            // HashSet::insert 返回 false 表示已存在
            if !self.jit_triggered.insert(key) {
                return false; // 已触发过
            }
            if self.config.jit_debug {
                eprintln!(
                    "[JIT HotSpot] 函数 #{} 达到热点阈值 ({} 次调用)，触发 JIT 编译",
                    func_idx, count
                );
            }
            return true;
        }
        false
    }

    /// 记录循环回边 (后向跳转)
    ///
    /// `func_idx`: 当前函数索引 (usize::MAX = 主程序)
    /// `jmp_ip`: 跳转指令的 IP 地址
    /// `target_ip`: 跳转目标 IP (< jmp_ip 时判定为回边)
    /// 返回 true 表示该循环达到热点阈值
    pub fn record_loop_backedge(
        &mut self,
        func_idx: usize,
        jmp_ip: usize,
        target_ip: usize,
    ) -> bool {
        if !self.config.enable_jit {
            return false;
        }

        // 只有后向跳转才是循环回边
        if target_ip >= jmp_ip {
            return false;
        }

        // 生成唯一键: (func_idx 的低 32 位, target_ip)
        let func_part = if func_idx == usize::MAX {
            u32::MAX as u64
        } else {
            (func_idx as u64) & 0xFFFF_FFFF
        };
        let key = (func_part << 32) | (target_ip as u64);

        let count = self.loop_counts.entry(key).or_insert(0);
        *count += 1;

        if *count >= self.config.hot_threshold {
            // HashSet::insert 返回 false 表示已存在
            if !self.jit_triggered.insert(key) {
                return false; // 已触发过
            }
            if self.config.jit_debug {
                let func_name = if func_idx == usize::MAX {
                    "main".to_string()
                } else {
                    format!("#{}", func_idx)
                };
                eprintln!(
                    "[JIT HotSpot] 循环 @ {}:ip={}→{} 达到热点阈值 ({} 次迭代)，触发 JIT 编译",
                    func_name, jmp_ip, target_ip, count
                );
            }
            return true;
        }
        false
    }

    /// 记录通用热点 (外部可指定任意 key)
    pub fn record_custom(&mut self, key: u64) -> bool {
        if !self.config.enable_jit {
            return false;
        }
        let count = self.loop_counts.entry(key).or_insert(0);
        *count += 1;
        if *count >= self.config.hot_threshold {
            // HashSet::insert 返回 false 表示已存在
            if !self.jit_triggered.insert(key) {
                return false;
            }
            if self.config.jit_debug {
                eprintln!(
                    "[JIT HotSpot] 自定义热点 key={} 触发 ({} 次)",
                    key, count
                );
            }
            return true;
        }
        false
    }

    /// 获取函数调用计数
    pub fn get_func_count(&self, func_idx: usize) -> u64 {
        self.func_counts.get(&func_idx).copied().unwrap_or(0)
    }

    /// 获取循环计数
    pub fn get_loop_count(&self, func_idx: usize, ip: usize) -> u64 {
        let func_part = if func_idx == usize::MAX {
            u32::MAX as u64
        } else {
            (func_idx as u64) & 0xFFFF_FFFF
        };
        let key = (func_part << 32) | (ip as u64);
        self.loop_counts.get(&key).copied().unwrap_or(0)
    }

    /// 重置所有计数器 (程序重新执行时调用)
    pub fn reset(&mut self) {
        self.func_counts.clear();
        self.loop_counts.clear();
        // 保留 jit_triggered 避免重复编译
    }

    /// 获取统计摘要
    pub fn stats(&self) -> HotSpotStats {
        HotSpotStats {
            total_funcs_tracked: self.func_counts.len(),
            total_loops_tracked: self.loop_counts.len(),
            jit_compiled: self.jit_triggered.len(),
            hot_threshold: self.config.hot_threshold,
        }
    }
}

// ============================================================================
// 热点统计摘要
// ============================================================================

/// 热点统计信息 (用于调试输出)
#[derive(Debug, Clone)]
pub struct HotSpotStats {
    pub total_funcs_tracked: usize,
    pub total_loops_tracked: usize,
    pub jit_compiled: usize,
    pub hot_threshold: u64,
}

impl std::fmt::Display for HotSpotStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[JIT Stats] 函数追踪: {}, 循环追踪: {}, JIT已编译: {}, 阈值: {}",
            self.total_funcs_tracked,
            self.total_loops_tracked,
            self.jit_compiled,
            self.hot_threshold
        )
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> JitConfig {
        JitConfig {
            enable_jit: true,
            hot_threshold: 3,
            max_jit_instrs: 500,
            jit_debug: false,
            max_cache_entries: 256,
        }
    }

    #[test]
    fn test_func_hotspot() {
        let mut detector = HotSpotDetector::new(test_config());

        // 前 2 次不应触发
        assert!(!detector.record_function_call(0));
        assert!(!detector.record_function_call(0));

        // 第 3 次触发
        assert!(detector.record_function_call(0));

        // 第 4 次不应重复触发
        assert!(!detector.record_function_call(0));
    }

    #[test]
    fn test_loop_hotspot() {
        let mut detector = HotSpotDetector::new(test_config());

        // 后向跳转: ip 10 → 5 (回边)
        assert!(!detector.record_loop_backedge(0, 10, 5));
        assert!(!detector.record_loop_backedge(0, 10, 5));
        assert!(detector.record_loop_backedge(0, 10, 5));
        // 已触发，不再重复
        assert!(!detector.record_loop_backedge(0, 10, 5));
    }

    #[test]
    fn test_forward_jump_not_loop() {
        let mut detector = HotSpotDetector::new(test_config());

        // 前向跳转不应计为循环回边
        assert!(!detector.record_loop_backedge(0, 5, 10));
        assert!(!detector.record_loop_backedge(0, 5, 10));
        assert!(!detector.record_loop_backedge(0, 5, 10));
    }

    #[test]
    fn test_disabled_detector() {
        let config = JitConfig {
            enable_jit: false,
            ..test_config()
        };
        let mut detector = HotSpotDetector::new(config);

        // 禁用时永远不触发
        for _ in 0..100 {
            assert!(!detector.record_function_call(0));
            assert!(!detector.record_loop_backedge(0, 10, 5));
        }
    }

    #[test]
    fn test_reset() {
        let mut detector = HotSpotDetector::new(test_config());

        assert!(!detector.record_function_call(0));
        assert!(!detector.record_function_call(0));

        detector.reset();

        // 重置后计数归零，需要重新计数到阈值
        // 但 jit_triggered 不重置，所以需要不同的 function
        // Actually, the triggered set is preserved, so func 0 still won't re-trigger
        // Let's test func 1 instead
        assert!(!detector.record_function_call(1));
        assert!(!detector.record_function_call(1));
        assert!(detector.record_function_call(1));
    }
}
