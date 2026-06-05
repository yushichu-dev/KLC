<<<<<<< HEAD
﻿# KLC (v1.0.3-正式版)

KLC（Kaleidoscope Language Compiler）是一门从零实现的现代编程语言，包含编译器、字节码生成器与轻量级栈式虚拟机。KLC 以性能、内存安全与可用性为主要目标，适合学习编译器实现、嵌入式脚本和实验性语言特性。

主要特点：

- 现代语法（函数、类型、枚举、模式匹配、协程等）
- 所有权与借用语义（借鉴 Rust 思想）
- 内置 VM 与可选原生 PE 生成（实验性）
- 丰富的标准库：字符串、数组、Map、math、fmt 等
- 内置格式化工具与代码风格检查（`klc fmt`）

本仓库包含编译器源码（Rust）、示例程序、文档与测试基准。

快速链接：

- 发行说明： [RELEASE_v084.md](RELEASE_v084.md)
- 使用指南（详细）： [docs/USAGE.md](docs/USAGE.md)
- 语言规范： [docs/lang_spec.md](docs/lang_spec.md)
- 示例目录： [examples/](examples/)

---

## 快速开始

先保证已安装 Rust 工具链（stable）：

    # 安装 rustup（如尚未安装）
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

在仓库根目录构建 KLC 可执行文件：

    cd klc
    cargo build --release

运行示例：

    target/release/klc run examples/hello.klc

---

## 运行依赖与开发依赖（边界说明）

下面通过一张简要表格明确区分两类依赖：

| 依赖类型 | 是否存在 | 说明 |
|----------|---------:|------|
| **运行依赖 (runtime)** | ❌ | 完全不存在 —— 编译得到的 `klc.exe` 为纯原生 Windows PE，可在任意 Windows 机器上直接运行，无需安装任何运行时或额外库。 |
| **开发依赖 (dev)** | ⚠️ | 仅在**编译 KLC 编译器本身**时需要 Rust 工具链（`rustc` / `cargo`）。一旦编译完成，可以卸载 Rust，已生成的 `klc.exe` 不受影响。 |

与解释型语言的对比：

- Python/Java 等需在运行时安装解释器/JVM；而 KLC 的目标是：构建时使用 Rust，运行时无任何依赖。

---

## 常用命令摘要

- 运行脚本（VM）: `klc <source.klc>` 或 `klc run <source.klc>`
- 构建/打包: `klc build [OPTIONS] <source>`（支持 `--native` 生成 Windows EXE）
- 格式化: `klc fmt [OPTIONS] <file>...`（`--check` 用于 CI）
- 语法检查: `klc check <source.klc>`
- 调试输出: `klc --debug <source.klc>`（显示 Tokens/AST/Bytecode）
- 启动 IDE: `klc --ide`

完整用法与示例请参阅 [docs/USAGE.md](docs/USAGE.md)。

---

## 项目结构（简要）

```
klc/
├─ src/          # 编译器实现（Rust）
├─ examples/     # 示例程序
├─ docs/         # 文档与使用说明
├─ RELEASE_v084.md
└─ Cargo.toml
=======
# KLC Programming Language

> **K**aleidoscope **L**anguage **C**ompiler  
> 高性能 · 内存安全 · 简洁易学

---

## 设计哲学

KLC 融合三种理念：

| 理念 | 来源 | 实现 |
|------|------|------|
| **高性能** | 系统级编程 | 编译为字节码 + 优化 JIT，零成本抽象，所有权无 GC |
| **内存安全** | Rust 所有权 | 精炼的 own/borrow 模型，编译期保证，无数据竞争 |
| **简洁易学** | Python 体验 | 强大类型推断，管道操作符，默认不可变，无分号 |

---

## 项目结构

```
klc/
├── docs/                    # 文档
│   └── lang_spec.md        # 语言规范 v0.3.1-beta
├── examples/                # 示例程序 (10 个)
│   ├── hello.klc           # 入门示例
│   ├── fibonacci.klc       # 递归/迭代/闭包
│   ├── ownership.klc       # 所有权系统
│   ├── concurrency.klc     # 协程/通道
│   ├── generics.klc        # 泛型/Trait
│   ├── pattern_match.klc   # 模式匹配/ADT
│   ├── test_fn.klc         # 函数调用测试
│   ├── test_match.klc      # match 表达式测试
│   ├── test_map.klc        # Map/索引测试
│   └── test_lambda.klc     # 匿名函数/闭包测试
├── src/                     # KLC 编译器源码
│   ├── main.rs             # 入口: Lexer → Parser → Codegen → VM
│   ├── token.rs + lexer.rs # 词法分析
│   ├── ast.rs + parser.rs  # 语法分析 (递归下降)
│   ├── bytecode.rs         # 字节码定义 + 运行时值类型
│   ├── codegen.rs          # AST → 字节码生成
│   └── vm.rs               # 栈式虚拟机引擎
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20
```

---

<<<<<<< HEAD
如需贡献、提交 bug 或请求新特性，请在仓库中打开 Issue 或提交 Pull Request。
=======
## 开发路线图

- [x] **阶段 0**: 语言设计（规范 + 示例）
- [x] **阶段 1**: MVP — 最小可用原型（Lexer + Parser + Codegen + VM）✅ `v0.3.1-beta.1`
- [ ] **阶段 2**: 核心特性（所有权语义 + 协程运行时 + 泛型单态化）
- [ ] **阶段 3**: 自举准备（KLC 具备编写编译器能力）
- [ ] **阶段 4**: 自举实现（KLC 编译 KLC 自身）
- [ ] **阶段 5**: 生态建设（包管理 + LSP + 工具链）

---

## 快速预览

```klc
-- hello.klc
mod main
use io

fn main() {
    let greeting = "Hello"
    let target = "KLC"
    io.println(greeting ++ ", " ++ target ++ "!")
}
```

更多示例见 [`examples/`](examples/) 目录。

---

## KLC 核心语法速览

| 场景 | 语法 |
|------|------|
| 变量 | `let x = 42` / `let mut x = 42` |
| 函数 | `fn add(a: i32, b: i32) -> i32 = a + b` |
| 结构体 | `type Point { x: f64, y: f64 }` |
| 所有权 | `own T` / `borrow T` / `borrow mut T` |
| 泛型 | `fn id<T>(x: T) -> T = x` |
| 协程 | `task w() { ... }` / `go w()` |
| 模式匹配 | `match x { 0 => "zero" ... }` |
| 错误处理 | `Result[T, E]` / `?` 操作符 |

---

*KLC v0.3.1-beta.1 — MVP 阶段*
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20
