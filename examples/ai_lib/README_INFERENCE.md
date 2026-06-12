# KLC:3B 端到端推理引擎 — 完整使用指南

## 📋 目录

- [项目概述](#项目概述)
- [架构设计](#架构设计)
- [环境要求](#环境要求)
- [快速开始](#快速开始)
- [详细安装步骤](#详细安装步骤)
- [模块说明](#模块说明)
- [权重转换指南](#权重转换指南)
- [IDE 集成指南](#ide-集成指南)
- [API 参考](#api-参考)
- [性能优化](#性能优化)
- [常见问题](#常见问题)
- [开发路线图](#开发路线图)

---

## 项目概述

### 什么是 KLC:3B 推理引擎？

KLC:3B 是一套基于自研 KLC 编程语言构建的**完整 AI 推理解决方案**，包含：

| 模块 | 文件 | 功能 |
|------|------|------|
| **权重转换器** | `weight_converter.klc` | HuggingFace safetensors → KLC .klcw 格式 |
| **BPE 分词器** | `tokenizer.klc` | 文本 ↔ Token 编解码 |
| **推理引擎** | `klc_3b_infer.klc` | 自回归文本生成（核心） |
| **IDE 集成** | `ide_integration.klc` | 对话、补全、Lint、解释 |

### 核心特性

✅ **100% KLC 原生实现** — 无外部依赖  
✅ **Windows 深度优化** — 内存对齐、低显存占用、纯 CPU 推理  
✅ **完整采样策略** — 温度/Top-K/Top-P/重复惩罚  
✅ **KV Cache 加速** — 自回归推理优化  
✅ **流式输出支持** — 实时生成体验  
✅ **IDE 友好接口** — DLL 导出 + 回调机制  

### 模型规格

```
模型名称：     KLC:3B-v1
隐藏维度：     2560
层数：         28
注意力头数：   32 (MHA, 非GQA)
头维度：       80 (2560/32)
FFN 中间层：   6912
最大序列长：   4096
词表大小：     64,000
参数量：       ~3.09 Billion
数据类型：     FP32 (可量化为 FP16/INT8)

内存需求：
  - FP32: ~12 GB (完整精度)
  - FP16: ~6 GB  (推荐平衡)
  - INT8: ~3 GB  (极致压缩)
```

---

## 架构设计

```
┌──────────────────────────────────────────────────────────────┐
│                     用户应用层                                │
│    (CLI 工具 / KLC IDE / Web 服务 / Python 绑定)            │
└───────────────────────────┬──────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────┐
│                  ide_integration.klc                         │
│         IDE 集成层（对话 / 补全 / Lint / 解释）               │
│                                                              │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐       │
│  │ Chat API │ │Complete  │ │ Lint     │ │ Explain  │       │
│  └────┬─────┘ └────┬─────┘ └────┬─────┘ └────┬─────┘       │
│       └────────────┼────────────┼────────────┘              │
└─────────────────────┼────────────┼───────────────────────────┘
                      ▼            ▼
┌──────────────────────────────────────────────────────────────┐
│                   klc_3b_infer.klc                           │
│                 核心推理引擎                                  │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │ Sampling     │  │ KV Cache     │  │ Memory Pool  │      │
│  │ (温度/K/P)   │  │ Manager      │  │ (Windows优化)│      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │ Transformer  │  │ Tokenizer    │  │ Weight Loader│      │
│  │ (28 Layers)  │  │ (BPE)        │  │ (.klcw)      │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
└──────────────────────────────────────────────────────────────┘
                      │
                      ▼
┌──────────────────────────────────────────────────────────────┐
│                    AI 基础库 (已完成)                          │
│                                                              │
│  tensor.klc  │ activation.klc  │ normalization.klc          │
│  memory_pool.klc             │ transformer.klc              │
└──────────────────────────────────────────────────────────────┘
```

---

## 环境要求

### 硬件要求

| 配置级别 | CPU | 内存 | 硬盘 | GPU |
|---------|-----|------|------|-----|
| **最低配置** (INT8) | 4 核 2.0 GHz+ | 8 GB | 10 GB 可用 | 不需要 |
| **推荐配置** (FP16) | 8 核 3.0 GHz+ | 16 GB | 15 GB 可用 | 可选加速 |
| **最佳配置** (FP32) | 16 核 3.5 GHz+ | 32 GB | 25 GB 可用 | 可选加速 |

> 💡 **提示**：所有功能均可在纯 CPU 上运行。GPU 为可选加速。

### 软件要求

- **操作系统**: Windows 10 / Windows 11 (64位)
- **KLC 运行时**: v1.0+
- **Python 3.8+** (仅用于下载预训练模型，可选)

---

## 快速开始

### 一行代码体验 AI 对话

```klc
-- main.klc
use ide_integration::{KLCIDEIntegration}

fn main() {
    -- 初始化（自动加载默认模型）
    let ide = KLCIDEIntegration::new("./models")
    ide.initialize()

    -- 发起对话
    let response = ide.chat("你好！请介绍一下KLC编程语言")
    
    -- 输出回复
    io.println(response.data["reply"])
    
    -- 清理资源
    ide.shutdown()
}
```

运行：

```bash
klc main.klc
```

### 完整示例：代码补全

```klc
use ide_integration::{KLCIDEIntegration}

fn main() {
    let ide = KLCIDEIntegration::new("./models")
    ide.initialize()
    
    let code = "fn main() {\n    io.pr"
    let result = ide.code_complete(code, cursor_position=22)
    
    for item in result.data.items {
        io.println(item.text ++ " (" ++ item.kind ++ ")")
    }
}
```

---

## 详细安装步骤

### Step 1: 获取源码

确保你已有完整的 AI 库文件：

```bash
klc/examples/ai_lib/
├── tensor.klc           # 张量计算基础库 ✅ 已有
├── activation.klc       # 激活函数库 ✅ 已有
├── normalization.klc    # 归一化层 ✅ 已有
├── memory_pool.klc      # 内存池管理 ✅ 已有
├── transformer.klc      # Transformer 内核 ✅ 已有
├── demo.klc             # 测试用例 ✅ 已有
├── weight_converter.klc # 权重转换器 🆕 新增
├── tokenizer.klc        # BPE 分词器 🆕 新增
├── klc_3b_infer.klc     # 推理引擎 🆕 新增
├── ide_integration.klc  # IDE 集成 🆕 新增
└── README.md            # 本文档 🆕 新增
```

### Step 2: 下载预训练模型权重

#### 方式 A: 使用开源 3B 模型（推荐）

我们推荐以下兼容模型：

| 模型名称 | 参数量 | 来源 | 推荐格式 |
|---------|--------|------|----------|
| TinyLlama-1.1B | 1.1B | HuggingFace | safetensors |
| Qwen2-1.5B | 1.5B | HuggingFace | safetensors |
| Phi-2 | 2.7B | HuggingFace | safetensors |
| StableLM-2-1.6B | 1.6B | HuggingFace | safetensors |

#### 下载命令（使用 HuggingFace CLI）

```bash
# 安装 huggingface_hub (需要 Python)
pip install huggingface_hub

# 下载 TinyLlama-1.1B（轻量级，适合测试）
huggingface-cli download TinyLlama/TinyLlama-1.1B-Chat-v1.0 \
    --local-dir ./models/TinyLlama-1.1B

# 或下载 Qwen2-1.5B（中文能力更强）
huggingface-cli download Qwen/Qwen2-1.5B-Instruct \
    --local-dir ./models/Qwen2-1.5B
```

### Step 3: 转换权重为 .klcw 格式

创建转换脚本：

```klc
-- convert_weights.klc
use weight_converter::{convert_model}

fn main() {
    -- 转换 TinyLlama-1.1B
    convert_model(
        input_file="models/TinyLlama-1.1B/model.safetensors",
        output_file="models/klc_3b.klcw",
        model_name="TinyLlama-1.1B-KLC",
        architecture="llama",      -- LLaMA 架构
        target_dtype="fp16"       -- 使用 FP16 减少内存占用
    )
}
```

运行转换：

```bash
klc convert_weights.klc
```

输出示例：

```
╔══════════════════════════════════╗
║   KLC 权重转换器 v1.0            ║
║   HuggingFace → KLC Format      ║
╚══════════════════════════════════╝

输入: models/TinyLlama-1.1B/model.safetensors
输出: models/klc_3b.klcw
架构: llama
目标: FP16

━━━ Phase 1: 解析输入文件 ━━━
════ Safetensors 文件信息 ═════
  张量总数: 187
  总参数量: 1.10B
  文件大小: ~2.20 GB

━━━ Phase 4: 量化与转换 ━━━
  进度: 50/187 张量
  进度: 100/187 张量
  INT8 量化: range=[...], scale=...

══════ 转换完成 ════════
  张量数: 187
  总参数: 1.10B
  ✓ KLCW 文件写入完成
```

### Step 4: （可选）获取分词器文件

大多数 HuggingFace 模型已包含 `tokenizer.json`，直接复制即可：

```bash
# 复制分词器文件到模型目录
copy models\TinyLlama-1.1B\tokenizer.json models\tokenizer.json

# 如果没有 tokenizer.json，可能还有其他格式：
copy models\TinyLlama-1.1B\tokenizer.model models\
copy models\TinyLlama-1.1B\tokenizer_config.json models\
copy models\TinyLlama-1.1B\special_tokens_map.json models\
```

### Step 5: 验证安装

运行测试程序：

```bash
klc demo.klc
```

预期输出：

```
╔══════════════════════════════════════════════╗
║  KLC:AI 基础张量计算库 — 完整示例           ║
║  Version 1.0.0 | KLC:3B Model Foundation    ║
╚══════════════════════════════════════════════╝

━━━ 测试 1: 张量基础运算 ━━━
✓ 张量运算测试通过

━━━ 测试 2: 激活函数 ━━━
✓ 激活函数测试通过

... (更多测试)

═══════════════════════════════════════════
  ✓ 所有测试通过！KLC:AI 基础库就绪
═══════════════════════════════════════════
```

---

## 模块说明

### 1. weight_converter.klc — 权重转换器

**功能**：将各种格式的模型权重转换为 KLC 专用的 `.klcw` 二进制格式。

**主要接口**：

```klc
-- 最简 API：一行转换
convert_model(
    input_file: str,        -- 输入 safetensors 文件
    output_file: str,       -- 输出 .klcw 文件
    model_name: str = "",   -- 模型名称
    architecture: str = "llama",  -- 架构类型
    target_dtype: str = "fp32"    -- 目标精度
) -> bool

-- 批量转换分片文件
convert_sharded_model(
    shard_files: [str],    -- 分片文件列表
    output_file: str,
    model_name: str,
    architecture: str,
    target_dtype: str
) -> bool

-- 交互式向导
interactive_wizard()
```

**支持的量化选项**：

| 类型 | 内存占用 | 精度损失 | 适用场景 |
|------|----------|----------|----------|
| FP32 | 100% | 无 | 开发调试 |
| FP16 | ~50% | 极小 | **推荐生产环境** |
| INT8 | ~25% | 小 | 低内存设备 |
| INT4 | ~12.5% | 中 | 极限压缩 |

---

### 2. tokenizer.klc — BPE 分词器

**功能**：实现完整的 Byte-Pair Encoding 编解码，将文本与 Token ID 相互转换。

**主要接口**：

```klc
-- 创建分词器
let tok = KLCTokenizer::new("tokenizer.json")

-- 编码：文本 → Token IDs
let ids = tok.encode("Hello, world!")
-- 结果: [2, 15496, 4435, 393, ...]  (2=BOS)

-- 解码：Token IDs → 文本
let text = tok.decode(ids)
-- 结果: "Hello, world!"

-- 带信息的编码
let result = tok.encode_full("Hello")
-- result.ids → [2, 15496]
-- result.tokens → ["", "Hello"]
-- result.special_tokens_mask → [1, 0]

-- 批量处理
let batch_ids = tok.encode_batch(["Hi", "Hello", "World"])

-- Token 数估算（无需完整编码）
let estimated = estimate_token_count(tok, long_text)
```

**特殊 Token**：

| Token | ID | 用途 |
|-------|-----|------|
| PAD | 0 | 填充符 |
| UNK | 1 | 未知字符 |
| BOS | 2 | 序列开始 |
| EOS | 3 | 序列结束 |
| MASK | 4 | 掩码（训练用） |

---

### 3. klc_3b_infer.klc — 推理引擎

**功能**：基于 Transformer 的自回归文本生成，支持多种采样策略。

**主要接口**：

```klc
-- 创建并加载引擎
let engine = KLC3BInferenceEngine::new(
    weight_path="models/klc_3b.klcw",
    tokenizer_path="models/tokenizer.json"
)
engine.load_model()

-- 生成文本（最常用 API）
let result = engine.generate(prompt="你好，请介绍人工智能")

-- 访问结果
io.println(result.text)              -- 生成的文本
io.println(result.num_generated_tokens)  -- token 数
io.println(result.tokens_per_second)     -- 吞吐量
io.println(result.finish_reason)         -- 完成原因

-- 流式生成
let stream_fn = engine.generate_stream("写一首诗")
loop {
    let chunk = stream_fn()
    if chunk.is_finished { break }
    print(chunk.text)  -- 逐块输出
}
```

**采样策略配置**：

```klc
-- 贪心模式（确定性最强）
let greedy_config = GenerationConfig {
    sampling: SamplingConfig::greedy(),
    max_new_tokens: 256
}

-- 默认模式（平衡质量与多样性）
let default_config = GenerationConfig {
    sampling: SamplingConfig::default(),  -- temp=0.8, top_k=50, top_p=0.9
    max_new_tokens: 512
}

-- 创意模式（更随机、更有创意）
let creative_config = GenerationConfig {
    sampling: SamplingConfig::creative(),  -- temp=1.2, top_k=100
    max_new_tokens: 1024
}

-- 精确模式（适合技术内容）
let precise_config = GenerationConfig {
    sampling: SamplingConfig::precise(),  -- temp=0.5, top_k=10
    max_new_tokens: 256
}

-- 使用自定义配置
let custom_sampling = SamplingConfig {
    temperature: 0.7,
    top_k: 40,
    top_p: 0.85,
    repetition_penalty: 1.2,  -- 抑制重复
    presence_penalty: 0.3,   -- 鼓励新话题
    frequency_penalty: 0.2,  -- 降低高频词
    do_sample: true
}
```

**便捷函数**：

```klc
-- 一行加载模型
let engine = quick_load("models/klc_3b.klcw")

-- 一行提问
let answer = ask_ai(engine, "什么是量子计算？")

-- 多轮对话
let history = ConversationHistory::new(max_rounds=10)
history.add_message("system", "你是物理学家")
chat(engine, history, "解释薛定谔的猫")
chat(engine, history, "那观察者效应呢？")  -- 有上下文
```

---

### 4. ide_integration.klc — IDE 集成

**功能**：提供面向 IDE 的完整 API，包括 AI 对话、代码补全、语法检查等。

**主要接口**：

```klc
-- 初始化
let ide = KLCIDEIntegration::new(model_dir="./models")
let init_result = ide.initialize(FEATURE_ALL)

-- AI 对话（Chat 模式）
let chat_response = ide.chat("帮我写一个快速排序")
io.println(chat_response.data["reply"])

-- 多会话管理
let session_id = ide.new_chat_session(system_prompt="你是KLC专家")
ide.switch_session(session_id)
ide.chat("如何定义结构体？")

-- 代码补全
let code = "fn fibonacci(n: i64) -> i64 { if n <= "
let comp_result = ide.code_complete(code, cursor_position=38)
for item in comp_result.data.items {
    io.println(item.display_text ++ " [" ++ item.kind ++ "]")
}

-- 语法检查 (Lint)
let lint_result = ide.lint_code(code_text)
for issue in lint_result.data.issues:
    io.println("[{severity}] Line {line}: {message}")

-- 自动修复
let fix_result = ide.auto_fix(buggy_code)
io.println(fix_result.data["fixed_code"])

-- 代码解释
let explain_result = ide.explain_code(complex_code, detail_level="detailed")
io.println(explain_result.data["summary"])
io.println(explain_result.data["detailed"])

-- 单元测试生成
let tests = ide.generate_tests(source_code)
io.println(tests.data["test_code"])

-- 代码翻译
let translation = ide.translate_code(klc_code, "KLC", "Python")
io.println(translation.data["translated_code"])

-- 清理
ide.shutdown()
```

**DLL 导出接口（C 兼容）**：

```c
// klcaide.h - C 语言头文件
#ifdef __cplusplus
extern "C" {
#endif

// 初始化引擎
__declspec(dllexport) int KLCA_Init(const char* model_dir);

// AI 对话
__declspec(dllexport) int KLCA_Chat(
    const char* session_id,
    const char* message,
    char* output_buffer,
    int buffer_size
);

// 代码补全
__declspec(dllexport) int KLCA_Complete(
    const char* code,
    int cursor_pos,
    char* output_buffer,
    int buffer_size
);

// 语法检查
__declspec(dllexport) int KLCA_Lint(
    const char* code,
    char* output_buffer,
    int buffer_size
);

// 清理资源
__declspec(dllexport) void KLCA_Shutdown(void);

#ifdef __cplusplus
}
#endif
```

**在 KLC IDE 中集成**：

```klc
-- ide_plugin.klc - IDE 插件示例
use ide_integration::{KLCIDEIntegration, FEATURE_ALL}

// 全局实例
let g_ai_engine: KLCIDEIntegration? = null

// 插件初始化
pub fn plugin_init(model_dir: str) -> i64 {
    g_ai_engine = KLCIDEIntegration::new(model_dir)
    return g_ai_engine.initialize(FEATURE_ALL).code
}

// 编辑器快捷键：Ctrl+Enter → AI 补全
pub fn on_ai_complete(current_code: str, cursor_pos: i64) -> str {
    if g_ai_engine == null { return "" }
    return g_ai_engine.complete_best_match(current_code, cursor_pos)
}

// 侧边栏：AI 助手面板
pub fn on_chat_send(user_message: str) -> str {
    if g_ai_engine == null { return "" }
    let response = g_ai_engine.chat(user_message)
    return response.data["reply"]
}

// 保存时自动检查
pub fn on_file_save(file_content: str) -> [LintIssue] {
    if g_ai_engine == null { return [] }
    let result = g_ai_engine.lint_code(file_content)
    return result.data.issues
}
```

---

## 权重转换指南

### 支持的源格式

| 格式 | 扩展名 | 来源 | 说明 |
|------|--------|------|------|
| SafeTensors | `.safetensors` | HuggingFace | **推荐**，安全高效 |
| PyTorch | `.bin`, `.pt` | PyTorch | 需先转 safetensors |
| GGUF | `.gguf` | llama.cpp | 量化模型 |

### 支持的目标架构

| 架构 | identifier | 兼容模型 |
|------|------------|----------|
| LLaMA | `"llama"` | LLaMA 1/2, Qwen2, TinyLlama, Yi, DeepSeek |
| Mistral | `"mistral"` | Mistral 7B, Mixtral |
| GPT-2 | `"gpt2"` | GPT-2, OPT, Bloom |
| Falcon | `"falcon"` | Falcon 7B/40B |

### 转换示例

**示例 1：转换 Qwen2-1.5B（中文模型）**

```klc
use weight_converter::{convert_model, ConversionPipeline, QuantizationConfig}

fn main() {
    -- 基础转换
    convert_model(
        "models/Qwen2-1.5B/model-00001-of-00002.safetensors",
        "models/qwen2_1.5b_fp16.klcw",
        "Qwen2-1.5B-KLC",
        "llama",
        "fp16"
    )

    -- 高级：INT8 量化
    let pipeline = ConversionPipeline {
        input_path: "models/Qwen2-1.5B/model.safetensors",
        output_path: "models/qwen2_1.5b_int8.klcw",
        model_name: "Qwen2-1.5B-KLC-INT8",
        architecture: "llama",
        target_dtype: "int8",
        quant_config: QuantizationConfig {
            method: "symmetric",
            target_dtype: "int8",
            group_size: -1,
            calibration_data: null
        },
        use_mmap: true,
        verify_checksum: true,
        verbose: true
    }

    run_conversion(pipeline)
}
```

**示例 2：批量转换多分片**

```klc
use weight_converter::{convert_sharded_model}

fn main() {
    -- Qwen2-7B 通常分为多个分片
    let shards = [
        "models/Qwen2-7B/model-00001-of-00004.safetensors",
        "models/Qwen2-7B/model-00002-of-00004.safetensors",
        "models/Qwen2-7B/model-00003-of-00004.safetensors",
        "models/Qwen2-7B/model-00004-of-00004.safetensors"
    ]

    convert_sharded_model(shards, "output/qwen2_7b.klcw", "Qwen2-7B-KLC", "llama", "fp16")
}
```

---

## IDE 集成指南

### 在 KLC IDE 中启用 AI 功能

#### 步骤 1：编译 DLL

```bash
klc --compile ide_integration.klc --output klcaide.dll --target windows-x64
```

#### 步骤 2：配置 IDE

在 IDE 的设置文件中添加：

```json
{
  "ai": {
    "enabled": true,
    "dll_path": "./plugins/klcaide.dll",
    "model_dir": "./models",
    "features": {
      "chat": true,
      "completion": true,
      "lint": true,
      "explain": true
    },
    "shortcuts": {
      "ai_chat": "Ctrl+Shift+A",
      "ai_complete": "Tab",
      "ai_explain": "Ctrl+Shift+E",
      "ai_fix": "Ctrl+Shift+F"
    },
    "ui": {
      "chat_panel_width": 400,
      "show_inline_completions": true,
      "lint_on_save": true
    }
  }
}
```

#### 步骤 3：使用

启动 IDE 后，你可以：

1. **AI 对话面板**：`Ctrl+Shift+A`
2. **智能补全**：输入代码后按 `Tab`
3. **代码检查**：保存时自动运行
4. **代码解释**：选中代码后 `Ctrl+Shift+E`
5. **自动修复**：`Ctrl+Shift+F`

### 性能建议

| 场景 | 建议配置 |
|------|----------|
| 实时补全 | `temperature=0.2`, `max_tokens=128`, 贪心模式 |
| 对话助手 | `temperature=0.8`, `max_tokens=2048`, Top-P=0.9 |
| 代码解释 | `temperature=0.3`, `max_tokens=1536`, 确定性优先 |
| 内容创作 | `temperature=1.2`, `max_tokens=4096`, 创意模式 |

---

## API 参考

### 核心类型索引

#### 数据类型

| 类型 | 定义于 | 说明 |
|------|--------|------|
| `Tensor` | tensor.klc | N 维张量 |
| `Shape` | tensor.klc | 张量形状 |
| `MemoryPool` | memory_pool.klc | 内存池管理器 |
| `KVCacheManager` | memory_pool.klc | KV 缓存管理器 |
| `ModelConfig` | transformer.klc | 模型配置 |
| `KLCTransformerModel` | transformer.klc | Transformer 模型 |
| `KLCTokenizer` | tokenizer.klc | BPE 分词器 |
| `SamplingConfig` | klc_3b_infer.klc | 采样配置 |
| `GenerationConfig` | klc_3b_infer.klc | 生成配置 |
| `GenerationResult` | klc_3b_infer.klc | 生成结果 |
| `KLC3BInferenceEngine` | klc_3b_infer.klc | 推理引擎 |
| `CompletionResult` | ide_integration.klc | 补全结果 |
| `LintResult` | ide_integration.klc | Lint 结果 |
| `APIResponse` | ide_integration.klc | API 响应 |

#### 常量

| 常量 | 值 | 说明 |
|------|-----|------|
| `DEFAULT_HIDDEN_SIZE` | 2560 | 默认隐藏维度 |
| `DEFAULT_NUM_LAYERS` | 28 | 默认层数 |
| `DEFAULT_NUM_HEADS` | 32 | 默认注意力头数 |
| `DEFAULT_VOCAB_SIZE` | 64000 | 默认词表大小 |
| `DEFAULT_MAX_POSITION` | 4096 | 最大序列长度 |
| `BOS_TOKEN_ID` | 2 | 开始标记 |
| `EOS_TOKEN_ID` | 3 | 结束标记 |

---

## 性能优化

### 内存优化

```klc
-- 1. 使用 INT8 量化（减少 75% 内存）
let engine = KLC3BInferenceEngine::new(
    weight_path="models/klc_3b_int8.klcw",
    pool_size=4 * 1024 * 1024 * 1024  -- 仅需 4GB
)

-- 2. 手动设置内存池大小
let pool = MemoryPool::new(initial_size=8 * 1024 * 1024 * 1024)  -- 8GB

-- 3. 卸载未使用的张量
engine.unload_unused_weights(["lm_head"])  -- 不需要时可卸载
```

### 推理速度优化

```klc
-- 1. 减少 KV Cache 最大长度
let config = ModelConfig {
    max_position_embeddings: 2048,  -- 从 4096 减半
    ...
}

-- 2. 使用贪心解码（避免采样开销）
let fast_config = GenerationConfig {
    sampling: SamplingConfig::greedy(),
    ...
}

-- 3. 批量推理（如果有多个请求）
let batch_result = engine.generate_batch(prompts, batch_size=4)
```

### 预期性能参考

| 配置 | 首次 Token (ms) | 后续 Token (ms) | 吞吐 (tokens/s) |
|------|-----------------|-----------------|-----------------|
| FP32, CPU (8核) | 2000-5000 | 200-500 | 2-5 |
| FP16, CPU (8核) | 1500-3000 | 150-300 | 3-7 |
| INT8, CPU (8核) | 800-1500 | 80-150 | 7-12 |
| FP16, GPU | 50-100 | 10-30 | 30-100 |

> ⚠️ 以上为估计值，实际性能取决于硬件和具体实现优化程度。

---

## 常见问题

### Q1: 内存不足怎么办？

**A:** 尝试以下方法：

1. 使用 INT8 量化：内存从 12GB 降至 3GB
2. 减少 `max_position_embeddings`（如设为 2048）
3. 关闭其他应用程序释放内存
4. 使用更小的模型（如 1.1B 而非 3B）

### Q2: 生成的文本质量不好？

**A:** 可能的原因和解决方法：

1. **未加载真实权重**：当前演示模式使用零权重，必须加载训练好的模型
2. **温度过高**：尝试降低 temperature（如 0.5-0.7）
3. **上下文不足**：增加 prompt 的详细信息
4. **模型不匹配**：确保使用的是经过良好训练的模型

### Q3: 如何添加新的特殊 Token？

**A:**

```klc
let tok = KLCTokenizer::new()

tok.add_special_tokens({
    "": 32000,
    "": 32001,
    "": 32002
})

io.println(tok.vocab_size())  -- 应该增加 3
```

### Q4: 如何实现流式输出的实时显示？

**A:**

```klc
let engine = KLC3BInferenceEngine::new(...)
engine.load_model()

-- 设置回调
engine.set_stream_callback(fn(chunk: StreamChunk) {
    if not chunk.is_finished {
        print(chunk.text)  -- 实时打印
    }
})

-- 流式生成
let _ = engine.generate("讲一个故事", stream=true)
```

### Q5: 如何对接其他编程语言的分词器？

**A:**

`tokenizer.klc` 支持 HuggingFace 标准格式。如果需要使用其他来源：

1. 将分词器的词汇表导出为 JSON 格式
2. 确保 JSON 包含 `model.vocab` 和 `merges` 字段
3. 调用 `KLCTokenizer::from_precomputed(vocab, merges)` 创建实例

---

## 开发路线图

### v1.1 (规划中)

- [ ] GPU 加速（CUDA/OpenCL 后端）
- [ ] Flash Attention 2 实现
- [ ] Speculative Decoding（推测解码加速 2-3x）
- [ ] LoRA 微调支持
- [ ] 多模态扩展（图像理解）

### v1.2 (远期)

- [ ] 分布式推理（多机并行）
- [ ] 模型蒸馏工具链
- [ ] 自定义算子编译器
- [ ] 移动端部署支持

---

## 许可证

本项目基于 MIT 许可证开源。

---

## 贡献指南

欢迎提交 Issue 和 Pull Request！

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add amazing feature'`)
4. 推送分支 (`git push origin feature/amazing-feature`)
5. 提交 Pull Request

---

## 致谢

感谢以下开源项目的启发和支持：

- [LLaMA](https://ai.meta.com/llama/) — Meta 的 LLaMA 架构
- [Qwen](https://qwenlm.github.io/) — 阿里的 Qwen 系列
- [HuggingFace Transformers](https://huggingface.co/docs/transformers) — 模型和工具
- [llama.cpp](https://github.com/ggerganov/llama.cpp) — CPU 推理优化思路

---

*KLC:3B 推理引擎 — 让每个开发者都能在自己的电脑上运行 AI*
