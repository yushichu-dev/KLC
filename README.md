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
```

---

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
