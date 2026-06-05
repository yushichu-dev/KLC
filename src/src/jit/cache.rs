//! KLC JIT 代码缓存管理模块
//!
//! 存储已编译的原生函数代码，提供快速查找和淘汰机制。

use std::collections::{HashMap, VecDeque};
use crate::jit::{CompiledNativeFn, JitConfig};

// ============================================================================
// JIT 代码缓存
// ============================================================================

/// JIT 编译代码缓存
///
/// 使用 LRU 风格的简单淘汰策略:
/// - 超过 max_entries 时淘汰最旧条目
pub struct JitCodeCache {
    /// 配置
    config: JitConfig,
    /// 编译后的函数缓存: 函数名 → 编译结果
    functions: HashMap<String, CompiledNativeFn>,
    /// 编译后的循环/代码块缓存: key → 编译结果
    blocks: HashMap<u64, CompiledNativeFn>,
    /// 插入顺序 (用于简单 LRU 淘汰，VecDeque 使 pop_front O(1))
    insert_order: VecDeque<String>,
    /// 块插入顺序
    block_order: VecDeque<u64>,
}

impl JitCodeCache {
    /// 创建新的代码缓存
    pub fn new(config: JitConfig) -> Self {
        Self {
            config,
            functions: HashMap::new(),
            blocks: HashMap::new(),
            insert_order: VecDeque::new(),
            block_order: VecDeque::new(),
        }
    }

    /// 缓存总量
    pub fn total_entries(&self) -> usize {
        self.functions.len() + self.blocks.len()
    }

    /// 查找函数缓存
    pub fn find_function(&self, name: &str) -> Option<&CompiledNativeFn> {
        self.functions.get(name)
    }

    /// 查找代码块缓存 (按循环地址)
    pub fn find_block(&self, func_idx: usize, ip: usize) -> Option<&CompiledNativeFn> {
        let key = Self::block_key(func_idx, ip);
        self.blocks.get(&key)
    }

    /// 插入函数缓存
    pub fn insert_function(&mut self, compiled: CompiledNativeFn) {
        let name = compiled.id.clone();

        // 同名函数: 先清理旧条目和顺序记录，防止 insert_order 重复
        if self.functions.contains_key(&name) {
            self.functions.remove(&name);
            self.insert_order.retain(|k| k != &name);
        }

        // LRU 淘汰 — pop_front O(1) 替代 Vec::remove(0) 的 O(n)
        if self.functions.len() >= self.config.max_cache_entries {
            if let Some(old_key) = self.insert_order.pop_front() {
                self.functions.remove(&old_key);
                if self.config.jit_debug {
                    eprintln!("[JIT Cache] LRU 淘汰函数: {}", old_key);
                }
            }
        }

        self.functions.insert(name.clone(), compiled);
        self.insert_order.push_back(name);
    }

    /// 插入代码块缓存
    pub fn insert_block(&mut self, compiled: CompiledNativeFn, func_idx: usize, ip: usize) {
        let key = Self::block_key(func_idx, ip);

        // LRU 淘汰 — pop_front O(1)
        if self.blocks.len() >= self.config.max_cache_entries {
            if let Some(old_key) = self.block_order.pop_front() {
                self.blocks.remove(&old_key);
                if self.config.jit_debug {
                    eprintln!("[JIT Cache] LRU 淘汰代码块: 0x{:016X}", old_key);
                }
            }
        }

        self.blocks.insert(key, compiled);
        self.block_order.push_back(key);
    }

    /// 清除所有缓存
    pub fn clear(&mut self) {
        self.functions.clear();
        self.blocks.clear();
        self.insert_order.clear();
        self.block_order.clear();
        if self.config.jit_debug {
            eprintln!("[JIT Cache] 已清空全部缓存");
        }
    }

    /// 获取缓存统计
    pub fn stats(&self) -> CacheStats {
        let total_code_size: usize = self.functions.values()
            .map(|f| f.code.len())
            .chain(self.blocks.values().map(|b| b.code.len()))
            .sum();

        CacheStats {
            function_entries: self.functions.len(),
            block_entries: self.blocks.len(),
            total_code_size,
            max_entries: self.config.max_cache_entries,
        }
    }

    /// 生成块缓存键
    #[inline(always)]
    fn block_key(func_idx: usize, ip: usize) -> u64 {
        let func_part = if func_idx == usize::MAX {
            u32::MAX as u64
        } else {
            (func_idx as u64) & 0xFFFF_FFFF
        };
        (func_part << 32) | (ip as u64 & 0xFFFF_FFFF)
    }
}

// ============================================================================
// 缓存统计
// ============================================================================

/// 缓存统计信息
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub function_entries: usize,
    pub block_entries: usize,
    pub total_code_size: usize,
    pub max_entries: usize,
}

impl std::fmt::Display for CacheStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[JIT Cache] 函数: {}, 代码块: {}, 代码总大小: {}B, 上限: {}",
            self.function_entries,
            self.block_entries,
            self.total_code_size,
            self.max_entries
        )
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jit::ExecutableMemory;

    fn test_config() -> JitConfig {
        JitConfig {
            enable_jit: true,
            hot_threshold: 50,
            max_jit_instrs: 500,
            jit_debug: false,
            max_cache_entries: 4,
        }
    }

    fn make_dummy_compiled(id: &str) -> CompiledNativeFn {
        let code = vec![0xC3u8]; // ret
        let mem = ExecutableMemory::allocate(&code).unwrap();
        let entry = mem.ptr;
        CompiledNativeFn {
            id: id.to_string(),
            code: mem,
            entry,
            instr_count: 1,
        }
    }

    #[test]
    fn test_cache_insert_and_find() {
        let mut cache = JitCodeCache::new(test_config());

        let compiled = make_dummy_compiled("factorial");
        cache.insert_function(compiled);

        assert!(cache.find_function("factorial").is_some());
        assert!(cache.find_function("nonexistent").is_none());
    }

    #[test]
    fn test_cache_lru_eviction() {
        let mut cache = JitCodeCache::new(test_config());

        // 插入 4 个 (达到上限)
        for i in 0..4 {
            cache.insert_function(make_dummy_compiled(&format!("fn_{}", i)));
        }
        assert_eq!(cache.functions.len(), 4);

        // 第 5 个触发淘汰
        cache.insert_function(make_dummy_compiled("fn_4"));
        assert_eq!(cache.functions.len(), 4);
        // 最旧的 fn_0 被淘汰
        assert!(cache.find_function("fn_0").is_none());
        assert!(cache.find_function("fn_4").is_some());
    }

    #[test]
    fn test_block_cache() {
        let mut cache = JitCodeCache::new(test_config());

        let compiled = make_dummy_compiled("block_0_5");
        cache.insert_block(compiled, 0, 5);

        assert!(cache.find_block(0, 5).is_some());
        assert!(cache.find_block(0, 10).is_none());
    }

    #[test]
    fn test_clear() {
        let mut cache = JitCodeCache::new(test_config());

        cache.insert_function(make_dummy_compiled("test"));
        cache.insert_block(make_dummy_compiled("block"), 0, 0);
        assert_eq!(cache.total_entries(), 2);

        cache.clear();
        assert_eq!(cache.total_entries(), 0);
    }
}
