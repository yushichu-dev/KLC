# KLC v1.0.3-正式版 自研编程语言 —— 项目技术文档

> **设计哲学**: "Let it flow" — 代码读起来应该像自然语言一样流畅
> **核心三角**: 高性能 × 简洁易学 × 内存安全
>
> 版本：v1.0.3-正式版 | 发布日期：2026-05-31 | 作者：yushichu-dev
>
> 许可证：MIT License

---

## 目录

1. [项目简介](#1-项目简介)
2. [技术架构](#2-技术架构)
3. [核心功能](#3-核心功能)
4. [自举开发进度](#4-自举开发进度)
5. [技术亮点](#5-技术亮点)
6. [运行依赖说明](#6-运行依赖说明)
7. [快速开始](#7-快速开始)
8. [原创声明](#8-原创声明)

---

## 1. 项目简介

KLC（**Kaleidoscope Language Compiler**）是一门从零实现的现代编程语言，完整包含**编译器前端**（词法分析、语法分析、AST 优化）、**编译器后端**（字节码生成、原生 x86_64 PE 代码生成）以及**轻量级栈式虚拟机**（字节码执行引擎）。

项目的核心目标是通过完整实现一门编程语言的全部工具链，探索编译器设计、虚拟机优化和语言自举（Self-hosting）的工程实践。KLC 借鉴了 Rust 的所有权与借用语义思想，采用类自然语言的语法风格，以**性能、内存安全与可用性**为主要设计目标。

**全部代码使用纯 Rust 编写，零外部运行时依赖**。编译器编译出的 `klc.exe` 为纯原生 Windows PE 可执行文件，无需安装任何运行时环境即可在任意 Windows x86_64 机器上直接运行。

### 1.1 核心定位

| 维度 | 说明 |
|------|------|
| **学习价值** | 完整的编译器/VM 实现，适合学习编译原理与语言设计 |
| **嵌入脚本** | 轻量级 VM，可作为应用程序内嵌脚本引擎 |
| **实验平台** | 快速验证新语言特性（模式匹配、协程、所有权语义等） |
| **自举探索** | 用 KLC 语言自身实现 KLC 编译器，探索语言完整性与工程可行性 |

---

## 2. 技术架构

### 2.1 整体架构图

```
┌──────────────────────────────────────────────────────────────┐
│                      KLC 编译器系统                           │
│                                                              │
│  .klc 源文件                                                  │
│      │                                                       │
│      ▼                                                       │
│  ┌─────────┐    ┌─────────┐    ┌──────────┐                  │
│  │  Lexer  │───▶│ Parser  │───▶│  AST优化  │                  │
│  │ 词法分析 │    │ 语法分析 │    │ 常量折叠  │                  │
│  │         │    │ 递归下降 │    │ 死码消除  │                  │
│  └─────────┘    └─────────┘    └────┬─────┘                  │
│                                     │                        │
│                    ┌────────────────┴────────────────┐       │
│                    ▼                                 ▼       │
│            ┌─────────────┐                 ┌──────────────┐  │
│            │  Codegen    │                 │ Native Codegen│  │
│            │ 字节码生成   │                 │ 原生PE代码生成 │  │
│            └──────┬──────┘                 └──────┬───────┘  │
│                   │                               │          │
│                   ▼                               ▼          │
│            ┌─────────────┐                 ┌──────────────┐  │
│            │  KLC VM     │                 │  Windows PE  │  │
│            │ 栈式虚拟机   │                 │  x86_64 EXE  │  │
│            │ 执行字节码   │                 │  原生可执行文件│  │
│            └─────────────┘                 └──────────────┘  │
│                                                              │
│  辅助工具: Formatter (代码格式化), DWARF (调试信息),         │
│            Win32 GUI IDE, Module System                      │
└──────────────────────────────────────────────────────────────┘
```

### 2.2 核心模块说明

| 模块 | 文件 | 职责 |
|------|------|------|
| **词法分析** | `lexer.rs` + `token.rs` | 源码 → Token 流，支持注释、转义字符串、多字符运算符 |
| **语法分析** | `parser.rs` + `ast.rs` | 递归下降 + Pratt 表达式解析，Token 流 → AST |
| **AST 优化** | `bytecode_optimize.rs` | 常量折叠、死代码消除、内联、多 Pass 收敛 |
| **字节码生成** | `codegen.rs` + `bytecode.rs` | AST → 字节码指令序列 + 常量池 |
| **VM 执行** | `vm.rs` | 固定容量栈式虚拟机，零堆分配执行 |
| **原生代码生成** | `native_codegen.rs` + `native/` | AST → x86_64 汇编 → Windows PE 文件 |
| **PE 生成** | `native/pe.rs` | 手写 PE 格式构建器，核心运行时零外部依赖 |
| **x64 编码** | `native/x64.rs` | x86_64 机器码指令编码器 |
| **寄存器分配** | `native/regalloc.rs` | 被调用者保存寄存器池管理 |
| **代码格式化** | `formatter.rs` | AST 驱动的代码美化工具 |
| **调试信息** | `dwarf.rs` | DWARF 调试信息生成器 |
| **GUI IDE** | `gui/` | Win32 原生 GUI 编辑器（语法高亮、快捷键） |

### 2.3 字节码指令集

KLC 虚拟机执行自定义字节码指令集，共 35 条指令：

| 类别 | 指令 |
|------|------|
| **栈操作** | `Const`, `Pop`, `Load`, `Store`, `InitVar` |
| **算术运算** | `Add`, `Sub`, `Mul`, `Div`, `Mod`, `Neg` |
| **比较运算** | `Eq`, `Neq`, `Lt`, `Gt`, `Lte`, `Gte` |
| **逻辑运算** | `And`, `Or`, `Not` |
| **字符串** | `Concat`, `ToString`, `SubStr`, `StrFind`, `StrRepeat` |
| **结构体** | `StructNew`, `StructGet`, `StructSet` |
| **枚举** | `EnumNew`, `IsVariant`, `EnumGet` |
| **控制流** | `Jmp`, `JmpFalse`, `Call`, `Return`, `Halt` |
| **I/O** | `Print`, `PrintLn`, `ReadLine` |

### 2.4 虚拟机性能优化

- **固定容量操作数栈**：4096 槽位，Box 堆分配避免线程栈溢出，零动态分配执行
- **去 clone 化**：主循环直接引用指令切片，消除热点路径内存分配
- **内联算术**：加法/减法等运算直接内联，无闭包开销
- **字符串池**：常见字符串复用 `Rc<String>` 减少分配
- **快速函数查找**：`Vec` 线性查找替代 `HashMap`（小规模更快）
- **Release-safe 模式**：去除运行时边界检查和类型校验

---

## 3. 核心功能

### 3.1 语言特性矩阵

| 特性 | 状态 | 说明 |
|------|:----:|------|
| **变量** | ✅ | `let` / `let mut`，默认不可变 |
| **基本类型** | ✅ | `i64`, `f64`, `String`, `bool`, `char`, `null` |
| **函数** | ✅ | `fn name(params) -> RetType { body }` |
| **if/else** | ✅ | 支持 `else if` 链，`if` 可作为表达式 |
| **while 循环** | ✅ | 条件循环 |
| **for-in 循环** | ✅ | 数组遍历、Map 遍历 |
| **loop 循环** | ✅ | 无限循环 |
| **结构体** | ✅ | `type Name { field: Type }`，支持字段默认值 |
| **枚举** | ✅ | 支持无数据变体和带数据变体 `Variant(Type)` |
| **模式匹配** | ✅ | `match` 表达式，支持 OR 模式、guard、通配符 |
| **Impl 块** | ✅ | 结构体关联函数 + 实例方法，`self` 参数自动处理 |
| **Result/Option** | ✅ | `Ok`/`Err` 错误处理，`Some`/`None` 可选值，`?` 运算符 |
| **数组** | ✅ | `[1, 2, 3]`，支持 `push`/`pop`/`len`/`sort`/`contains` 等方法 |
| **Map** | ✅ | `{key: val}` 字面量，支持 `insert`/`remove`/`keys`/`values` |
| **字符串** | ✅ | 丰富 API：`len`/`substr`/`find`/`contains`/`trim`/`split`/`replace`/`repeat` 等 |
| **文件 I/O** | ✅ | `io::read`/`io::write`/`io::append`/`io::exists`/`io::read_lines` |
| **数学库** | ✅ | `math::sin`/`cos`/`sqrt`/`exp`/`log`/`round`/`abs` 等 |
| **格式化** | ✅ | `fmt()` 支持 `%d`/`%f`/`%s`/`%x`/`%b` 等格式说明符 |
| **类型转换** | ✅ | `int_of`/`float_of`/`str_of`/`type_of` |
| **代码格式化** | ✅ | `klc fmt` 子命令，AST 驱动统一风格 |
| **原生 PE 生成** | ✅ | `--native` 编译为独立 Windows EXE |

### 3.2 代码示例

#### 基础语法

```klc
-- 变量定义
let name = "KLC"
let mut counter = 0

-- 函数定义
fn fibonacci(n: i64) -> i64 {
    if n <= 1 {
        return n
    }
    return fibonacci(n - 1) + fibonacci(n - 2)
}

-- 结构体与方法
type Point {
    x: f64,
    y: f64,
}

impl Point {
    fn distance(self) -> f64 {
        let sum = self.x * self.x + self.y * self.y
        return math::sqrt(sum)
    }
}

-- 枚举与模式匹配
enum Status {
    Success,
    Error(i64),
}

fn handle(s: Status) {
    match s {
        Status::Success => io.println("OK")
        Status::Error(code) => io.println("错误: " ++ code.to_str())
        _ => io.println("未知")
    }
}

-- 错误处理
fn safe_divide(a: i64, b: i64) -> Result {
    if b == 0 {
        return Err("除数不能为零")
    }
    return Ok(a / b)
}

fn main() {
    -- ? 运算符
    let val = safe_divide(10, 2)?
    io.println("结果: " ++ val.to_str())
}
```

---

## 4. 自举开发进度

KLC 正在实现**语言自举**（Self-hosting）——用 KLC 语言自身编写 KLC 编译器。目前已完成以下里程碑：

### 4.1 自举流水线

```
┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐
│  KLC源码  │───▶│  Lexer   │───▶│  Parser  │───▶│ Codegen  │
│ (.klc)   │    │ (KLC实现) │    │ (KLC实现) │    │ (KLC实现) │
└──────────┘    └──────────┘    └──────────┘    └──────────┘
                                                       │
                                                       ▼
                                               ┌──────────────┐
                                               │   Bytecode   │
                                               │  指令序列     │
                                               └──────────────┘
```

### 4.2 完成状态

| 模块 | 文件 | 行数 | 状态 | 测试用例 |
|------|------|:----:|:----:|:--------:|
| **Token 定义** | `lexer/token.klc` | ~170 | ✅ | TokenType 枚举 42 变体 + Token 结构体 |
| **词法分析器** | `lexer/lexer_core.klc` | ~470 | ✅ | 22,069 字符源码 → 4,563 Token |
| **AST 定义** | `parser/ast.klc` | ~230 | ✅ | Expr(12) + Stmt(14) + 15 辅助结构体 |
| **语法分析器** | `parser/parser_core.klc` | ~700 | ✅ | 9 级优先级攀登 + 语句解析 |
| **字节码生成** | `codegen/test_pipeline.klc` | ~900 | ✅ | 全流水线：源码→Token→AST→Bytecode |

### 4.3 自举测试结果

| 测试 | 源码 | 生成指令 |
|------|------|:------:|
| 变量+运算 | `let x = 1 + 2 * 3` | 9 条 |
| if/else | `if score > 60 { ... } else { ... }` | 12 条 |
| 函数定义 | `fn add(a,b) -> i64 { return a+b }` | 3+7+5 条 |
| while 循环 | `while i < 3 { ... }` | 14 条 |

### 4.4 Rust 单元测试

**61/61 全通过**，零失败，覆盖 Lexer、Parser、Codegen、VM、Native、Formatter、Module、DWARF 全部模块。

---

## 5. 技术亮点

### 5.1 纯 Rust 实现，零外部运行时

KLC 编译器的 **全部代码** 使用 Rust 编写。Cargo.toml 的 `[dependencies]` 段为**空**——不依赖任何第三方 crate 用于核心功能。唯一的构建依赖是 `embed-resource`（用于嵌入 Windows 图标资源）。

编译产物 `klc.exe` 为纯原生 Windows PE 可执行文件，可在未安装任何运行时、框架或解释器的机器上直接运行。这避免了 Python、Java 等语言的运行时依赖问题。

**Rust 堆栈**：`rustc 1.95.0` | `cargo` | 无第三方运行时

### 5.2 13 线程并行计算

KLC 语言层面支持 `task` 关键字定义的轻量级协程，运行时通过 Rust 标准库的 `std::thread` 实现真正的操作系统级线程并行。支持最多 **13 个并行任务**，适用于数据并行计算、并发 I/O 等场景。

```klc
-- 并行计算示例
task compute(id: i64) {
    io.println("Task " ++ id.to_str() ++ " running")
}

fn main() {
    -- 启动 13 个并行任务
    go compute(1)
    go compute(2)
    -- ...
}
```

### 5.3 跨平台支持

| 平台 | 编译器 | VM 执行 | 原生 PE 生成 |
|------|:------:|:------:|:------------:|
| **Windows x86_64** | ✅ | ✅ | ✅ |
| **Linux x86_64** | ✅ | ✅ | — (PE 仅 Windows) |
| **macOS x86_64** | ✅ | ✅ | — (PE 仅 Windows) |

- 编译器和 VM 使用标准 Rust，跨平台无额外适配工作
- 原生 PE 生成专为 Windows x86_64 设计
- 仅需 Rust 工具链即可在任意平台编译

### 5.4 原生 Windows PE 可执行文件生成

KLC 具备**完整的 x86_64 PE 文件生成能力**，从字节层面手动构造 Windows PE 格式，**不依赖任何外部工具链**（无 LLVM、无 GCC、无 MASM）。

**PE 生成流程**：

```
AST → x86_64 寄存器分配 → x86_64 指令编码 → PE 文件布局 → 写入磁盘
```

**技术要点**：

| 组件 | 实现 |
|------|------|
| **DOS Header** | 标准 IMAGE_DOS_HEADER (64 字节) |
| **DOS Stub** | "This program cannot be run in DOS mode." (64 字节) |
| **PE 签名** | `PE\0\0` (4 字节) |
| **COFF Header** | IMAGE_FILE_HEADER (20 字节) |
| **Optional Header** | IMAGE_OPTIONAL_HEADER64 (112 字节) |
| **节表** | `.text` 代码节 (40 字节) |
| **导入表** | kernel32.dll / user32.dll 动态导入 |
| **x86_64 编码** | 完整支持 MOV/ADD/SUB/CALL/JMP/CMP/PUSH/POP/RET 等指令 |
| **寄存器分配** | 被调用者保存寄存器池 (RBX/RSI/RDI/R13/R14/R15) |
| **Win32 API** | 自动导入 MessageBoxA/WriteConsoleA/ExitProcess 等 |

### 5.5 其他技术特点

- **AST 多 Pass 优化**：常量折叠 + 死代码消除交替执行直到收敛
- **TCO 尾调用优化**：尾递归函数消除调用栈开销（原生代码生成路径）
- **CSE 公共子表达式消除**：识别并消除重复计算
- **DWARF 调试信息**：支持 `-g` 生成标准 DWARF 格式调试数据
- **Win32 GUI IDE**：内置原生 Windows GUI 编辑器，支持语法高亮和快捷键
- **代码格式化器**：AST 驱动的 `klc fmt` 命令，统一代码风格

---

## 6. 运行依赖说明

### 6.1 依赖矩阵

| 依赖类型 | 是否存在 | 详细说明 |
|----------|:--------:|----------|
| **运行时依赖** | ❌ 无 | `klc.exe` 为纯原生 Windows PE，可在任意 Windows x86_64 机器上直接运行，无需安装任何运行时、框架或库 |
| **开发依赖** | ⚠️ 仅编译时 | 编译 KLC 编译器仅需 Rust 工具链（`rustc` + `cargo`）。编译完成后可卸载 Rust，不影响 `klc.exe` 使用 |
| **第三方库** | ❌ 无 | `[dependencies]` 段为空，核心功能零第三方依赖 |
| **构建依赖** | `embed-resource` 2.4 | 仅用于嵌入 Windows 图标资源，不影响运行时 |

### 6.2 与传统语言的对比

| 语言 | 运行时依赖 |
|------|-----------|
| **Python** | 需安装 Python 解释器 |
| **Java** | 需安装 JVM / JRE |
| **JavaScript** | 需 Node.js 或浏览器 |
| **C#** | 需 .NET Runtime |
| **KLC** | **无需任何运行时** |

### 6.3 系统要求

| 项目 | 最低要求 |
|------|----------|
| **操作系统** | Windows 10+ / Linux (kernel 4.x+) / macOS 11+ |
| **CPU** | x86_64 架构 |
| **内存** | 512 MB+ |
| **磁盘** | 10 MB+ |
| **开发环境** | Rust 1.70+ (仅编译 KLC 时需要) |

---

## 7. 快速开始

### 7.1 编译与安装

```bash
# 克隆仓库
git clone <repository_url>
cd klc

# 编译（debug 模式）
cargo build

# 或编译 release 模式（优化）
cargo build --release

# 可执行文件位于:
#   target/debug/klc.exe    (debug)
#   target/release/klc.exe  (release)
```

### 7.2 基础使用

```bash
# VM 执行 KLC 脚本
klc examples/hello.klc

# 语法检查
klc check examples/hello.klc

# 调试运行（显示 Token/AST/Bytecode）
klc --debug examples/hello.klc

# 代码格式化
klc fmt examples/hello.klc

# 编译为原生 Windows EXE
klc build --native examples/hello.klc -o hello.exe

# 启动图形 IDE
klc --ide
```

### 7.3 项目结构

```
klc/
├── src/                   # Rust 编译器源码
│   ├── main.rs            # CLI 入口
│   ├── token.rs           # Token 定义
│   ├── lexer.rs           # 词法分析器
│   ├── ast.rs             # AST 节点定义
│   ├── parser.rs          # 语法分析器
│   ├── bytecode.rs        # 字节码定义 + Value 类型
│   ├── codegen.rs         # 字节码生成器
│   ├── vm.rs              # 栈式虚拟机
│   ├── bytecode_optimize.rs # AST 优化 Pass
│   ├── native_codegen.rs  # 原生 PE 代码生成器
│   ├── native/            # PE格式/x64编码/寄存器分配
│   ├── gui/               # Win32 GUI IDE
│   ├── formatter.rs       # 代码格式化器
│   ├── dwarf.rs           # DWARF 调试信息
│   └── module.rs          # 模块系统
├── lexer/                 # KLC 自举：词法分析器（KLC语言编写）
│   ├── token.klc          # Token 类型定义
│   ├── lexer_core.klc     # Lexer 核心实现
│   └── test_lexer.klc     # Lexer 测试用例
├── parser/                # KLC 自举：语法分析器（KLC语言编写）
│   ├── ast.klc            # AST 类型定义
│   ├── parser_core.klc    # Parser 核心实现
│   └── test_parser.klc    # Parser 测试用例
├── codegen/               # KLC 自举：字节码生成器（KLC语言编写）
│   └── test_pipeline.klc  # 完整流水线测试
├── examples/              # KLC 示例程序（~30个）
├── docs/                  # 文档
├── benchmarks/            # 性能基准测试
└── Cargo.toml             # Rust 项目配置
```

---

## 8. 原创声明

### 8.1 原创性

本项目（KLC v1.0.3-正式版）为**完全自主设计与实现**的编程语言系统，包含：

1. **原创语言设计**：语法规范、类型系统、标准库 API 均为自主设计
2. **原创编译器实现**：Lexer、Parser、AST、Codegen、Bytecode VM 全部手写
3. **原创 PE 生成器**：从字节层面手写 PE 格式，未使用 LLVM、GCC 或任何第三方代码生成库
4. **原创 x86_64 编码器**：手写 x86_64 机器码指令编码，无外部汇编器依赖
5. **原创自举代码**：lexer/、parser/、codegen/ 目录下的 `.klc` 文件为 KLC 语言自主实现

### 8.2 技术参照说明

本项目的设计思想参考了以下公开学术/工程成果：

- Rust 语言的所有权/借用概念（仅理念参照，非代码移植）
- 经典编译原理教材的递归下降解析方法
- Windows PE/COFF 格式规范（Microsoft 公开文档）
- x86_64 ISA 指令编码规范（Intel/AMD 公开手册）
- DWARF 调试格式标准

所有参考均为公开的**格式规范**与**设计理念**，不涉及任何第三方代码的直接使用。

### 8.3 开源许可

本项目采用 **MIT License** 开源。允许自由使用、复制、修改、合并、出版发行、再授权及销售副本，仅需保留版权声明和许可声明。

### 8.4 版本历史

| 版本 | 日期 | 里程碑 |
|------|------|--------|
| v0.1 | — | 基础 Lexer + Parser + VM |
| v0.5 | — | 原生 PE 生成器 |
| v0.7 | — | 模式匹配、枚举、Impl 块 |
| **v1.0.3-正式版** | 2026-05 | 自举原型、Result/Option、字符串库增强、Char 比较修复 |

---

> **KLC — Kaleidoscope Language Compiler**
>
> "Let it flow" — 代码如万花筒般变幻，如流水般自然。
>
> Copyright © 2026 yushichu-dev. MIT License.
