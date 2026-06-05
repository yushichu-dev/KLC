# KLC:AI 基础张量计算库

> **基于 KLC 原生语法的 AI 基础库 | 支持 KLC:3B/9B/27B 模型系列**

## 📋 项目概述

本套件为 KLC 编程语言提供完整的 AI 基础计算能力，包括：

- **张量运算引擎**：支持 N 维张量、广播机制、矩阵乘法
- **激活函数**：ReLU, GELU (精确/近似), SiLU, Softmax 等
- **归一化层**：LayerNorm, RMSNorm（LLaMA 风格）, BatchNorm
- **内存池管理**：Windows 优化、预分配、权重加载、KV 缓存
- **Transformer 内核**：完整的多头注意力、SwiGLU FFN、RoPE

### 设计目标

1. ✅ **纯 KLC 实现** - 无外部依赖，直接在 KLC VM 上运行
2. ✅ **编译期优化友好** - 纯函数设计，支持常量折叠、死代码消除
3. ✅ **生产级架构** - 预留扩展接口，支持 3B → 9B → 27B 迭代
4. ✅ **详细注释** - 每个函数包含数学公式、算法说明、使用示例

---

## 📁 文件结构

```
ai_lib/
├── tensor.klc          # 核心张量数据结构与运算
├── activation.klc      # 激活函数库 (GELU/SiLU/ReLU)
├── normalization.klc   # 归一化层 (LayerNorm/RMSNorm)
├── memory_pool.klc     # 内存池 + 权重加载 + KV缓存
├── transformer.klc     # Transformer 最小内核
└── demo.klc            # 完整示例与测试套件
```

---

## 🚀 快速开始

### 方式 1: 运行演示程序

```bash
# 使用 KLC VM 执行（推荐用于调试）
klc run examples/ai_lib/demo.klc

# 或原生编译以获得最佳性能
klc build --native examples/ai_lib/demo.klc -o demo.exe
./demo.exe
```

### 方式 2: 在你的 KLC 代码中导入

```klc
-- 导入所需模块
use tensor::{Tensor, Shape}
use activation::{gelu, silu, softmax}
use normalization::{RMSNorm}
use transformer::{ModelConfig, KLCTransformerModel}

fn main() {
    -- 1. 创建配置
    let config = ModelConfig::klc_3b()

    -- 2. 初始化模型（需配合权重文件）
    let model = KLCTransformerModel::new(config, pool)

    -- 3. 推理
    let output = model.generate([15496, 4435], max_new_tokens=128)
}
```

---

## 📖 API 参考

### 1. 张量操作 (`tensor.klc`)

```klc
-- 创建张量
let s = Tensor::scalar(42.0)                    -- 标量
let v = Tensor::vector([1.0, 2.0, 3.0])        -- 向量
let m = Tensor::matrix(2, 3, [1..6])            -- 2×3矩阵
let t = Tensor::zeros(Shape{dims:[3,4,5]})      -- 3D零张量
let r = Tensor::random(Shape{dims:[10]}, seed=42)

-- 基础运算（支持广播）
let c = a.add(b)         -- 加法
let d = a.mul(b)          -- 逐元素乘法
let e = a.matmul(b)       -- 矩阵乘法
let f = e.transpose()     -- 转置

-- 归约操作
v.sum()                   -- 所有元素求和
v.mean()                  -- 均值
v.norm()                  -- L2范数

-- 形状变换
t.reshape(Shape{dims:[15,4]})
t.unsqueeze(axis=1)      -- 插入维度
```

### 2. 激活函数 (`activation.klc`)

```klc
-- ReLU 系列
relu(x)                          -- max(0, x)
leaky_relu(x, alpha=0.01)        -- Leaky ReLU

-- GELU（推荐用于 Transformer）
gelu(x)                         -- 精确版（erf）
gelu_tanh_approx(x)              -- Tanh 近似版（更快）
gelu_sigmoid_approx(x)           -- Sigmoid 近似版（最快）

-- SiLU / Swish（LLaMA/Qwen2 默认）
silu(x)                          -- x * σ(x)
swish(x, beta=1.0)               -- 可调参数版本

-- 其他
softmax(x)                       -- 注意力机制必需
log_softmax(x)                   -- 数值稳定版本
mish(x)                          -- 新兴激活函数
```

### 3. 归一化层 (`normalization.klc`)

```klc
-- LayerNorm（带可学习参数）
let ln = LayerNorm::new(normalized_shape=768, eps=1e-5)
let output = ln.forward(input_tensor)

-- RMSNorm（LLaMA/Mistral 标准，更高效）
let rn = RMSNorm::new(normalized_shape=4096, eps=1e-6)
let output = rn.forward(input_tensor)

-- 快速接口（无参数）
layer_norm_quick(x)
rms_norm_quick(x)
```

### 4. 内存池 (`memory_pool.klc`)

```klc
-- 初始化内存池（预分配 512MB）
let pool = MemoryPool::new(initial_size=512*1024*1024)

-- 分配张量
let weight = pool.alloc_zeros(Shape{dims:[2560, 2560]})

-- KV Cache 管理（推理必需）
let kv_cache = KVCacheManager::new(
    num_layers=28,
    num_heads=32,
    head_dim=80,
    max_seq_len=4096,
    max_batch_size=1,
    pool=pool
)

-- 更新缓存
kv_cache.update(layer_idx=0, batch_idx=0, new_key=k_tensor, new_value=v_tensor)
```

### 5. Transformer (`transformer.klc`)

```klc
-- 选择模型规格
let config = ModelConfig::klc_3b()   -- 入门款 (3B参数)
let config = ModelConfig::klc_9b()   -- 进阶款 (9B参数)
let config = ModelConfig::klc_27b()  -- 专业款 (27B参数)

-- 查看配置详情
config.validate()
io.println(config.model_name)
io.println("GQA: " ++ config.uses_gqa())
```

---

## 🏗️ 架构设计

### KLC:3B 模型规格

| 参数                | 值            | 说明                        |
|---------------------|---------------|-----------------------------|
| 隐藏维度 (d_model)  | 2560          | 模型宽度                    |
| 层数 (n_layers)      | 28            | Transformer Block 数量       |
| 注意力头数           | 32            | Multi-Head Attention        |
| 头维度 (head_dim)    | 80            | 2560 ÷ 32 = 80              |
| 中间层大小           | 6912          | FFN 中间维度 (~2.7x hidden) |
| 词表大小             | 64,000        | Tokenizer 词汇表             |
| 最大序列长度         | 4,096         | 支持的最大上下文长度         |
| 总参数量             | ~3.09 Billion | FP32 约 12GB 显存/内存       |

### 单个 Transformer Block 结构

```
输入 x ──→ [RMSNorm] → [Multi-Head Attention (RoPE)] → (+) ──┐
                                                          │
输出 ←─── [RMSNorm] → [SwiGLU FFN] → (+) ←────────────────┘
```

关键特性：
- **Pre-Norm**: 归一化在子层之前（训练稳定）
- **RoPE**: 旋转位置编码（外推能力强）
- **SwiGLU**: 门控 FFN（性能优于标准 FFN）
- **GQA 可选**: 9B/27B 使用分组查询注意力（节省显存）

---

## ⚡ 性能优化建议

### 1. 使用原生编译

```bash
# 比 VM 解释执行快 10-100x
klc build --native your_model.klc -o model.exe
```

### 2. 启用编译器优化

KLC 编译器会自动应用：
- **常量折叠**: `gelu(0.0)` → `0.0` （编译期计算）
- **代数简化**: `x * 1.0` → `x`, `x + 0.0` → `x`
- **死代码消除**: 未使用的分支不生成指令

### 3. 内存池调优

根据模型大小调整初始分配：

```klc
-- 3B 模型（FP32 推理需要 ~12GB 工作内存）
let pool = MemoryPool::new(initial_size=1024*1024*1024)  -- 1GB 池

-- 9B/27B 模型建议使用更大的池或分批加载权重
```

### 4. KV Cache 优化

对于长文本生成：
- 设置合理的 `max_seq_len`
- 使用 GQA 减少缓存占用（9B/27B 已默认启用）

---

## 🔧 扩展开发指南

### 添加新的激活函数

编辑 `activation.klc`：

```klc
pub fn my_activation(x: Tensor) -> Tensor {
    let n = x.size()
    let mut result = []
    for i in 0..n {
        result.push(your_formula(x.data[i]))
    }
    return Tensor::from_shape(x.get_shape(), result)
}
```

### 添加新模型规格

编辑 `transformer.klc` 的 `ModelConfig`：

```klc
pub fn klc_custom() -> ModelConfig {
    return ModelConfig {
        vocab_size: 100000,
        hidden_size: 3072,
        num_layers: 32,
        num_heads: 48,
        num_kv_heads: 8,  -- 启用 GQA
        head_dim: 64,
        intermediate_size: 16384,
        ...
    }
}
```

### 自定义量化支持

编辑 `memory_pool.klc` 的数据类型转换逻辑，添加：
- INT4/INT8 量化格式支持
- 自定义量化表加载
- 动态量化/反量化接口

---

## 📊 测试覆盖

运行 `demo.klc` 会自动执行以下测试：

✅ 张量四则运算与广播
✅ 矩阵乘法正确性
✅ 形状变换与索引访问
✅ 激活函数数值精度验证（GELU 误差 < 0.005）
✅ LayerNorm/RMSNorm 归一化效果检验
✅ 内存池分配与释放
✅ KV Cache 更新流程
✅ Transformer 组件初始化
✅ 多规格配置对比（3B/9B/27B）

---

## 🛠️ 故障排除

### 问题 1: 内存不足

**错误**: `Error: 空间不足`  
**解决**: 增大内存池初始大小或分块处理大张量

### 问题 2: 维度不匹配

**错误**: `Error: 维度不兼容`  
**解决**: 检查 `.shape.dims` 输出，确保矩阵乘法满足 `[M,K] @ [K,N]`

### 问题 3: 权重加载失败

**错误**: `Error: 未找到权重: xxx`  
**解决**: 检查权重文件路径和张量名称是否正确

---

## 📈 版本历史

- **v1.0.0** (2026-05-28): 初始发布
  - 完整的张量运算引擎
  - 6 种激活函数（含 3 种 GELU 变体）
  - LayerNorm + RMSNorm + BatchNorm
  - Windows 优化内存池
  - LLaMA 风格 Transformer 内核
  - KLC:3B/9B/27B 配置预设
  - 端到端示例与测试套件

---

## 📝 许可证

本项目作为 KLC 语言生态的核心组件，
遵循 KLC 项目主许可证。

---

## 🤝 贡献指南

欢迎提交 Issue 和 PR！主要方向：

- 性能优化（SIMD 向量化、并行计算）
- 新算子支持（CrossAttention, Sparse Attention等）
- 更多模型架构（MoE, Mamba, RWKV 等）
- GPU/CUDA 后端适配

---

## 🙏 致谢

- LLaMA (Meta AI) - RMSNorm + SwiGLU + RoPE 架构参考
- Qwen (Alibaba Cloud) - 中文场景优化思路
- Mistral AI - GQA 分组查询注意力设计
- HuggingFace Transformers - API 设计灵感

---

**用 KLC 构建 AI 的未来 🚀**
