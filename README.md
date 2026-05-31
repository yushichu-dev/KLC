# KLC (v1.0.3-正式版)

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
```

---

如需贡献、提交 bug 或请求新特性，请在仓库中打开 Issue 或提交 Pull Request。
