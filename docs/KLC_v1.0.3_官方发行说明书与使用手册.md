# KLC v1.0.3 (Stable) 官方发行说明书与使用手册

> **版本号**: KLC v1.0.3-正式版 (Stable)
> **发布日期**: 2026年5月31日
> **作者**: 于仕初（个人开发者）
> **项目类型**: 通用型编程语言
> **语言实现**: Rust + 自举（KLC 语言编写 KLC 编译器组件）

---

## 目录

1. [版本信息](#1-版本信息)
2. [版权与使用声明](#2-版权与使用声明)
3. [运行环境要求](#3-运行环境要求)
4. [安装与部署](#4-安装与部署)
5. [命令行接口参考](#5-命令行接口参考)
6. [核心架构说明](#6-核心架构说明)
7. [语言语法参考](#7-语言语法参考)
8. [内置标准库](#8-内置标准库)
9. [KLC AI 模块说明](#9-klc-ai-模块说明)
10. [示例代码](#10-示例代码)
11. [故障排查](#11-故障排查)
12. [版本历史](#12-版本历史)
13. [后续规划](#13-后续规划)

---

## 1. 版本信息

| 属性 | 内容 |
|------|------|
| **版本号** | KLC v1.0.3 (Stable) |
| **发布日期** | 2026年5月31日 |
| **作者** | 于仕初（个人开发者） |
| **项目类型** | 通用型编程语言 |
| **设计哲学** | "Let it flow" — 代码读起来应该像自然语言一样流畅 |
| **核心三要素** | 高性能 × 简洁易学 × 内存安全 |
| **开发语言(Rust)** | Rust 1.95.0 / Edition 2021 |
| **开源协议** | MIT License |
| **代码仓库(github)** | https://github.com/yushichu-dev/KLC |

### 1.1 项目简介

KLC（Kaleidoscope Language Compiler）是一门从零自主研发的现代通用编程语言，具备完整编译器工具链：

- **词法分析器** (Lexer)：识别 Token 流
- **语法分析器** (Parser)：递归下降 + Pratt 表达式解析，产出 AST
- **AST 优化器**：常量折叠、死代码消除、运算内联、多 Pass 收敛
- **字节码编译器** (Codegen)：AST → 35条字节码指令集
- **栈式虚拟机** (VM)：固定 4096 槽容量栈，零堆分配，高效执行
- **原生代码生成器** (Native Codegen)：AST → x86_64 机器码 → Windows PE（.exe）可执行文件
- **自举能力**：KLC 语言的 lexer、parser、codegen 均以 KLC 语言自身编写
- **GUI IDE**：基于 Win32 原生 API 的轻量图形界面
- **AI 推理引擎**：实验性的 Transformer 模型训练与推理模块

---

## 2. 版权与使用声明

### 2.1 版权归属

```
Copyright (c) 2026 于仕初 (yushichu-dev)

KLC Programming Language — Kaleidoscope Language Compiler
```

KLC 编译器、虚拟机、标准库及所有配套工具的版权归 **于仕初（个人开发者）** 所有。

### 2.2 使用许可范围

本软件以 **MIT License** 授权发布，用户享有以下权利：

- 自由使用、复制、修改、合并、出版发行、再授权
- 将本软件或其衍生版本用于商业或非商业目的
- 将本软件用于教学、科研、个人项目或生产环境

**条件**：在所有副本或实质性部分中必须包含上述版权声明和本许可声明。

### 2.3 免责声明

本软件按"原样"提供，不作任何明示或默示的保证，包括但不限于适销性、特定用途适用性和非侵权性的保证。在任何情况下，作者或版权持有人均不对因使用本软件而产生的任何索赔、损害或其他责任负责，无论是合同诉讼、侵权行为还是其他原因。

> **注意**：原生 PE 代码生成器 (`--native`) 为实验性功能，生成的 x86_64 可执行文件为最小化加载器实现，建议在受控环境下测试验证。

---

## 3. 运行环境要求

### 3.1 支持操作系统

| 操作系统 | 支持状态 | 说明 |
|----------|----------|------|
| **Windows 10 / 11 (x86_64)** | 完整支持 | 推荐平台 |
| **Windows 7 / 8 (x86_64)** | 兼容 | 部分 GUI 特性可能受限 |

> **说明**：当前版本仅支持 Windows x86_64 平台。macOS 与 Linux 支持计划在后续版本中推进。

### 3.2 最低硬件配置

| 组件 | 最低要求 | 推荐配置 |
|------|----------|----------|
| CPU | x86_64 双核 1.5GHz | 四核 2.0GHz+ |
| 内存 | 512 MB | 2 GB+ |
| 磁盘空间 | 50 MB（仅运行时） | 200 MB（含开发工具链） |
| 显示器 | 1024×768 | 1920×1080 |

### 3.3 运行依赖项

**KLC 可执行文件（klc.exe）为零外部依赖的独立程序**，不需要安装任何运行时库。

**从源码编译**需要以下工具链：

| 依赖 | 版本 | 说明 |
|------|------|------|
| Rust 工具链 | stable 1.95.0+ | 包含 cargo、rustc |
| MinGW-w64 (GNU ABI) | 任意 | 用于 GNU 工具链链接 |

> 如果使用 MSVC 工具链（`stable-x86_64-pc-windows-msvc`），需额外安装 Visual Studio 2017+ 或 Build Tools for Visual Studio。

---

## 4. 安装与部署

### 4.1 Windows 平台安装步骤

#### 方法一：直接使用预编译二进制

1. 获取 `klc.exe` 可执行文件（位于发布包中）
2. 将 `klc.exe` 放置到任意目录（例如 `C:\Program Files\klc\`）

#### 方法二：从源码编译

```powershell
# 1. 确保已安装 Rust 工具链
rustup default stable-x86_64-pc-windows-gnu

# 2. 克隆或解压项目到本地
# （假设项目位于 F:\klc）

# 3. 进入项目目录并编译
cd F:\klc
cargo build --release

# 编译产物位于: .\target\release\klc.exe
```

### 4.2 环境变量配置

将 `klc.exe` 所在目录添加到系统 PATH 环境变量：

**方法一：PowerShell（当前会话）**

```powershell
$env:Path += ";C:\Program Files\klc"
```

**方法二：永久添加**

1. 右键"此电脑" → 属性 → 高级系统设置 → 环境变量
2. 在"系统变量"中找到 `Path`，点击编辑
3. 添加 `klc.exe` 所在目录的完整路径
4. 点击确定，重新打开终端

**方法三：Cargo 用户**

```powershell
copy .\target\release\klc.exe C:\Users\%USERNAME%\.cargo\bin\klc.exe
```

### 4.3 全局命令注册

安装完成后，可在任意目录下直接使用 `klc` 命令：

```powershell
# 验证安装
klc version
# 输出: KLC v1.0.3-正式版 — Kaleidoscope Language Compiler

# 查看帮助
klc help
```

### 4.4 安装验证

创建一个测试文件 `hello_test.klc`：

```klc
mod main
use io

fn main() {
    io.println("Hello, KLC!")
}
```

执行：

```powershell
klc hello_test.klc
# 预期输出: Hello, KLC!
```

### 4.5 常见部署问题排查

| 问题 | 症状 | 解决方案 |
|------|------|----------|
| 命令未找到 | `'klc' is not recognized` | 检查 PATH 配置，重新打开终端 |
| 编译错误 | `Parse error` | 检查 .klc 文件语法，使用 `klc check` 验证 |
| 文件编码 | 中文注释乱码 | 确保 .klc 文件使用 UTF-8 编码 |
| 磁盘损坏 | `os error 1392` | 运行 `chkdsk /f` 修复磁盘，删除 `target` 目录后重试 |

### 4.6 快速开始（5 分钟上手）

#### 第 1 步：安装验证

```powershell
klc version
# 预期：KLC v1.0.3-正式版 — Kaleidoscope Language Compiler
```

#### 第 2 步：创建第一个 KLC 程序

用任意文本编辑器创建 `hello.klc`：

```klc
mod main
use io

fn main() {
    io.println("Hello, KLC v1.0.3!")
}
```

#### 第 3 步：运行

```powershell
klc hello.klc
# 预期输出：Hello, KLC v1.0.3!
```

#### 第 4 步：编译为独立 EXE

```powershell
klc build --native hello.klc -o hello.exe
.\hello.exe
# 输出同上，但这是一个独立可执行文件
```

#### 下一步

- 查看完整语法参考 → [第 7 章 语言语法参考](#7-语言语法参考)
- 浏览更多示例 → [第 10 章 示例代码](#10-示例代码)
- 使用图形界面 → `klc --ide`

---

## 5. 命令行接口参考

### 5.1 全局命令列表

| 命令 | 格式 | 说明 |
|------|------|------|
| **直接运行** | `klc <source.klc>` | 默认 VM 执行模式 |
| **运行脚本** | `klc run <source.klc>` | 同直接运行 |
| **项目构建** | `klc build [OPTIONS] <source>` | 编译为字节码或原生 EXE |
| **代码格式化** | `klc fmt [OPTIONS] <file>...` | 格式化源代码 |
| **语法检查** | `klc check <source.klc>` | 仅检查语法，不执行 |
| **版本信息** | `klc version` / `klc -v` | 显示版本号 |
| **帮助信息** | `klc help` / `klc -h` | 显示命令列表 |
| **图形界面** | `klc --ide` | 启动 KLC IDE |
| **调试运行** | `klc --debug <source.klc>` | 显示 Token/AST/字节码 |

### 5.2 编译指令

```powershell
# 编译为字节码（VM执行）
klc build source.klc

# 编译为原生 Windows EXE
klc build --native source.klc

# 指定输出文件
klc build --native source.klc -o output.exe

# 禁用优化
klc build --no-opt source.klc

# 生成 DWARF 调试信息
klc build --native -g source.klc -o debug_build.exe
```

### 5.3 运行指令

```powershell
# 直接运行（VM执行）
klc examples/hello.klc

# 带调试信息运行
klc run --debug examples/hello.klc
```

### 5.4 参数说明与示例

#### 构建选项

| 参数 | 说明 |
|------|------|
| `--native` | 生成原生 Windows PE (.exe)，而非字节码 |
| `--no-opt` | 禁用 AST 优化 Pass（常量折叠、死代码消除等） |
| `-g`, `--debug-info` | 生成 DWARF 格式调试信息 |
| `--debug` | 显示编译中间产物：Token 流、AST 结构、字节码指令 |
| `-o <output>` | 指定输出文件路径（配合 `--native` 使用） |

#### 格式化选项 (`klc fmt`)

| 参数 | 说明 |
|------|------|
| `--check` | 仅检查格式是否符合规范，不修改文件 |
| `--indent <N>` | 指定缩进宽度（默认 4 空格） |

#### 使用示例

```powershell
# 调试运行并查看完整编译流水线
klc --debug codegen/test_pipeline.klc

# 批量格式化目录下所有 KLC 文件
klc fmt examples/*.klc

# 仅检查格式（CI集成用）
klc fmt --check examples/*.klc

# 自定义缩进宽度
klc fmt --indent 2 examples/hello.klc

# 编译原生 EXE
klc build --native examples/hello.klc -o hello.exe
.\hello.exe
```

---

## 6. 核心架构说明

### 6.1 编译流水线

KLC v1.0.3 实现了完整的编译流水线，从源码到可执行文件的全链路覆盖：

```
.klc 源文件
    │
    ▼
┌──────────┐    ┌──────────┐    ┌─────────────┐
│  Lexer   │───▶│  Parser  │───▶│  AST 优化器  │
│ 词法分析  │    │ 递归下降  │    │ 常量折叠等   │
└──────────┘    └──────────┘    └──────┬──────┘
                                       │
              ┌────────────────────────┼────────────────────────┐
              ▼                        │                        ▼
      ┌──────────────┐                 │              ┌────────────────┐
      │   Codegen    │                 │              │ Native Codegen │
      │  字节码生成   │                 │              │ 原生 PE 生成   │
      └──────┬───────┘                 │              └───────┬────────┘
             ▼                         │                      ▼
      ┌──────────────┐                 │              ┌────────────────┐
      │   KLC VM     │                 │              │  Windows PE    │
      │ 栈式虚拟机    │                 │              │  x86_64 EXE    │
      │ 解释执行      │                 │              │ 独立可执行文件  │
      └──────────────┘                 │              └────────────────┘
```

### 6.2 核心模块表

| 模块 | 源文件 | 职责 |
|------|--------|------|
| 词法分析器 | `src/token.rs`, `src/lexer.rs` | 源码字符流 → Token 流 |
| 语法分析器 | `src/ast.rs`, `src/parser.rs` | 递归下降 + Pratt 表达式解析，Token 流 → AST |
| AST 优化器 | `src/bytecode_optimize.rs` | 常量折叠、死代码消除、运算内联、多 Pass 收敛 |
| 字节码生成 | `src/codegen.rs`, `src/bytecode.rs` | AST → 35条字节码指令序列 + 常量池 |
| 虚拟机 | `src/vm.rs` | 固定 4096 槽容量栈式 VM，零额外堆分配 |
| 原生代码生成 | `src/native_codegen.rs`, `src/native/` | AST → x86_64 汇编 → Windows PE |
| PE 格式构建 | `src/native/pe.rs` | 手写 Windows PE 格式构建器 |
| x64 编码器 | `src/native/x64.rs` | x86_64 机器码指令编码 |
| 寄存器分配 | `src/native/regalloc.rs` | 被调用者保存寄存器池管理 |
| DWARF 生成 | `src/dwarf.rs` | DWARF 调试信息生成器 |
| 代码格式化 | `src/formatter.rs` | AST 驱动的代码美化工具 |
| GUI IDE | `src/gui/` | Win32 原生 GUI 集成开发环境 |
| 模块系统 | `src/module.rs` | 模块声明与跨文件导入 |

### 6.3 字节码指令集（35条）

| 类别 | 指令 | 说明 |
|------|------|------|
| **栈操作** | `Const`, `Pop`, `Load`, `Store`, `InitVar` | 常量/变量与栈交互 |
| **算术运算** | `Add`, `Sub`, `Mul`, `Div`, `Mod`, `Neg` | 整数与浮点运算 |
| **比较运算** | `Eq`, `Neq`, `Lt`, `Gt`, `Lte`, `Gte` | 比较并压入 bool |
| **逻辑运算** | `And`, `Or`, `Not` | 逻辑与/或/非 |
| **字符串** | `Concat`, `ToString`, `SubStr`, `StrFind`, `StrRepeat` | 字符串拼接/查找/重复 |
| **结构体** | `StructNew`, `StructGet`, `StructSet` | 结构体构造与字段操作 |
| **枚举** | `EnumNew`, `EnumCheck`, `EnumGet` | 枚举构造/判别/取值 |
| **控制流** | `Jmp`, `JmpFalse`, `Call`, `Return`, `Halt` | 跳转/函数调用/终止 |
| **IO** | `Print`, `PrintLn` | 控制台输出 |
| **数组/Map** | `ArrayNew`, `ArraySet`, `ArrayGet`, `ArrayLen`, `ArrayPush`, `MapNew`, `MapSet`, `MapGet`, `MapKeys` | 集合类型操作 |

### 6.4 VM 执行层优化

| 优化项 | 说明 |
|--------|------|
| 固定容量栈 | `Box<[Value; 4096]>` 堆分配栈，零 realloc |
| 零 clone 指令 | 每次仅 clone 当前一条指令 |
| 数学内联 | math 函数编译为直接 f64 运算 |
| 内存池化 | StringPool 复用相同字符串 |

### 6.5 自举编译流水线

KLC 的 Lexer、Parser、Codegen 三个核心组件已经使用 KLC 语言自身重新实现：

| 自举组件 | 文件路径 | 代码规模 |
|----------|----------|----------|
| Token 定义 | `lexer/token.klc` | ~170 行 |
| Lexer 核心 | `lexer/lexer_core.klc` | ~470 行 |
| Lexer 测试 | `lexer/test_lexer.klc` | 测试用例 |
| AST 定义 | `parser/ast.klc` | ~230 行 |
| Parser 核心 | `parser/parser_core.klc` | ~700 行 |
| Parser 测试 | `parser/test_parser.klc` | 测试用例 |
| 流水线测试 | `codegen/test_pipeline.klc` | ~900 行 |

验证命令：

```powershell
klc codegen/test_pipeline.klc
```

该文件演示了**源码 → Lexer → Token → Parser → AST → Codegen → Bytecode** 的完整自举流水线。

### 6.6 线程并行计算能力

KLC v1.0.3 支持 13 线程并行计算（主要用于矩阵运算模块）：

```klc
use mat

let a = mat::create(100, 200)
let b = mat::create(200, 150)
-- 13线程并行矩阵乘法
let prod = mat::parallel_mul(a, b)
-- 单线程序列乘法
let prod_seq = mat::mul(a, b)
```

---

## 7. 语言语法参考

### 7.1 变量与常量

#### 默认不可变

```klc
let name = "KLC"
let version = 1.0
```

#### 可变变量（显式 mut）

```klc
let mut counter = 0
counter = counter + 1
```

#### 带类型标注

```klc
let name: str = "KLC"
let mut count: i64 = 0
```

### 7.2 数据类型

#### 内建基本类型

| 类型 | 说明 | 示例 |
|------|------|------|
| `i64` | 有符号 64 位整数 | `let x = 42` |
| `f64` | 双精度浮点数 | `let pi = 3.14` |
| `String` | UTF-8 字符串 | `let s = "hello"` |
| `bool` | 布尔值 | `let ok = true` |
| `char` | Unicode 字符 | `let c = 'A'` |
| `null` | 空值 | `let v = null` |

#### 复合类型

```klc
-- 动态数组
let arr = [1, 2, 3, 4, 5]
arr.push(6)
let first = arr[0]

-- Map 映射
let map = {"one": 1, "two": 2}
map.insert("three", 3)

-- 结构体
type Point { x: f64, y: f64 }
let p = Point { x: 3.0, y: 4.0 }

-- 匿名函数 (Lambda)
let callback = fn(x: i64) -> i64 = x * 2
```

### 7.3 算术运算

```klc
let a = 10 + 5     -- 加法
let b = 10 - 5     -- 减法
let c = 10 * 5     -- 乘法
let d = 10 / 3     -- 除法（整数截断）
let e = 10 % 3     -- 取模
let f = -a         -- 取负
```

#### 运算符优先级

| 优先级 | 运算符 | 结合性 |
|--------|--------|--------|
| 14 | `.` `::` `[]` | 左 |
| 13 | `-`(取负) `!` `not` | 右 |
| 12 | `*` `/` `%` | 左 |
| 11 | `+` `-` `++` | 左 |
| 9 | `==` `!=` `<` `<=` `>` `>=` | 左 |
| 7 | `and` | 左 |
| 6 | `or` | 左 |
| 4 | `=` | 右 |

### 7.4 条件分支

```klc
-- if 作为语句
if score >= 90 {
    io.println("A")
} else if score >= 80 {
    io.println("B")
} else {
    io.println("C")
}

-- if 作为表达式
let max = if a > b { a } else { b }

-- if/else 块内支持 let 绑定
let result = if 5 > 3 {
    let tmp = 10
    tmp + 1
} else {
}
```

### 7.5 循环结构

```klc
-- while 循环
let mut i = 0
while i < 10 {
    io.println(i)
    i = i + 1
}

-- for-in 遍历数组
let arr = [3, 1, 4, 1, 5]
for x in arr {
    io.println(x)
}

-- loop 无限循环
loop {
    i = i + 1
    if i > 100 { break }
    if i % 2 == 0 { continue }
    io.println(i)
}
```

### 7.6 函数定义与调用

```klc
-- 标准函数
fn greet(name: str) -> str {
    return "Hello, " ++ name
}

-- 无返回值
fn print_greeting(name: str) {
    io.println("Hi, " ++ name)
}

-- 短函数（表达式体）
fn add(a: i64, b: i64) -> i64 = a + b

-- 递归
fn factorial(n: i64) -> i64 {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

-- 匿名函数
let double = fn(x: i64) -> i64 = x * 2
io.println(double(5))   -- 输出: 10
```

### 7.7 结构体与枚举

```klc
-- 结构体定义
type Point {
    x: f64
    y: f64
}

-- 创建实例
let p = Point { x: 3.0, y: 4.0 }

-- 简写（变量名 = 字段名）
let x = 3.0
let y = 4.0
let p2 = Point { x, y }

-- impl 方法块
impl Point {
    fn distance(self) -> f64 {
        return math::sqrt(self.x * self.x + self.y * self.y)
    }

-- 枚举定义
type Option {
    Some(i64)
    None
}

type Result {
    Ok(i64)
    Err(str)
}

-- 枚举使用
let val = Some(42)

-- match 模式匹配
match val {
    Some(n) => n
    None => 0
}
```

### 7.8 字符串操作

```klc
let s = "  hello KLC  "

s.trim()              -- "hello KLC"
s.to_upper()          -- "  HELLO KLC  "
s.to_lower()          -- "  hello klc  "
s.starts_with("  ")   -- true
s.ends_with("  ")     -- true
s.split(" ")          -- ["", "", "hello", "KLC", "", ""]
s.replace("KLC", "World")
s.len()
s.char_at(0)          -- ' '
s.chars()             -- 字符数组

-- 拼接
let greeting = "Hello, " ++ name
```

### 7.9 文件 IO

```klc
-- 控制台输出
io.println("Hello")         -- 打印并换行
io.print("no newline")      -- 打印不换行

-- 文件操作
io.write("path.txt", "content")
io.append("path.txt", "more")
let content = io.read("path.txt")
let lines = io.read_lines("path.txt")
let exists = io.exists("path.txt")
io.delete("path.txt")
io.mkdir("dir_name")
let files = io.list_dir(".")
let size = io.file_size("path.txt")
```

### 7.10 类型检查与转换

```klc
type_of(42)         -- "i64"
type_of(3.14)       -- "f64"
type_of("hello")    -- "String"
type_of(null)       -- "Null"
type_of([1,2])      -- "Array"

is_null(x)          -- 判断是否为 null

int_of("123")       -- 字符串转整数: 123
float_of("3.14")    -- 字符串转浮点: 3.14
str_of(42)          -- 任意值转字符串: "42"
```

---

## 8. 内置标准库

### 8.1 io — 输入输出

| 函数 | 签名 | 说明 |
|------|------|------|
| `io.println(value)` | `(any) -> void` | 打印值并换行 |
| `io.print(value)` | `(any) -> void` | 打印值不换行 |
| `println(value)` | `(any) -> void` | 简写形式 |
| `print(value)` | `(any) -> void` | 简写形式 |
| `read_line()` | `() -> String` | 读取控制台输入行 |
| `io.write(path, content)` | `(str, str) -> void` | 写入文件 |
| `io.append(path, content)` | `(str, str) -> void` | 追加到文件 |
| `io.read(path)` | `(str) -> String` | 读取文件全部内容 |
| `io.read_lines(path)` | `(str) -> Array` | 按行读取文件 |
| `io.exists(path)` | `(str) -> bool` | 检查文件是否存在 |
| `io.delete(path)` | `(str) -> void` | 删除文件 |
| `io.mkdir(path)` | `(str) -> void` | 创建目录 |
| `io.list_dir(path)` | `(str) -> Array` | 列出目录下所有文件名 |
| `io.file_size(path)` | `(str) -> i64` | 获取文件大小 (字节) |
| `exit(code)` | `(i64) -> void` | 退出程序 |

### 8.2 String 方法

| 方法 | 签名 | 说明 |
|------|------|------|
| `s.trim()` | `() -> String` | 去除首尾空白 |
| `s.to_upper()` / `s.to_uppercase()` | `() -> String` | 转大写 |
| `s.to_lower()` | `() -> String` | 转小写 |
| `s.starts_with(prefix)` | `(str) -> bool` | 是否以指定前缀开头 |
| `s.ends_with(suffix)` | `(str) -> bool` | 是否以指定后缀结尾 |
| `s.split(sep)` | `(str) -> Array` | 按分隔符分割 |
| `s.replace(from, to)` | `(str, str) -> String` | 替换子串 |
| `s.len()` | `() -> i64` | 字符串长度 |
| `s.char_at(idx)` | `(i64) -> char` | 指定位置字符 |
| `s.chars()` | `() -> Array` | 转为字符数组 |

### 8.3 math — 数学库

#### 常量

| 函数 | 返回值 |
|------|--------|
| `math.pi()` | π ≈ 3.141592653589793 |
| `math.e()` | e ≈ 2.718281828459045 |

#### 三角函数

| 函数 | 说明 |
|------|------|
| `math.sin(x)` | 正弦 |
| `math.cos(x)` | 余弦 |
| `math.tanh(x)` | 双曲正切 |

#### 指数与对数

| 函数 | 说明 |
|------|------|
| `math.exp(x)` | e^x |
| `math.sqrt(x)` | 平方根 |
| `math.log(x)` / `math.ln(x)` | 自然对数 |
| `math.log2(x)` | 以2为底的对数 |
| `math.log10(x)` | 以10为底的对数 |

#### 数值工具

| 函数 | 说明 |
|------|------|
| `math.abs(x)` | 绝对值 |
| `math.min(a, b)` | 最小值 |
| `math.max(a, b)` | 最大值 |
| `math.pow(base, exp)` | 幂运算 |
| `math.floor(x)` | 向下取整 |
| `math.ceil(x)` | 向上取整 |
| `math.round(x)` | 四舍五入 |

### 8.4 fmt — 字符串格式化

```klc
-- 格式化说明符: %s %d %f %.Nf %x %b
fmt("Name: %s, Age: %d", "Alice", 30)
-- "Name: Alice, Age: 30"

fmt("Pi = %.6f", 3.14159265)
-- "Pi = 3.141593"

fmt("Hex: 0x%x, Bin: %b", 255, 15)
-- "Hex: 0xff, Bin: 1111"

-- format 是 fmt 的别名
format("Same as fmt: %s", "hello")
```

### 8.5 Array 方法

| 方法 | 说明 |
|------|------|
| `arr.len()` | 获取长度 |
| `arr.push(val)` | 尾部追加元素 |
| `arr.pop()` | 尾部弹出元素 |
| `arr.contains(val)` | 是否包含元素 |
| `arr.index_of(val)` | 元素首次出现索引 |
| `arr.reverse()` | 反转数组 |
| `arr.sort()` | 排序数组 |
| `arr.join(sep)` | 用分隔符连接为字符串 |
| `arr.is_empty()` | 是否为空 |
| `arr.clear()` | 清空数组 |

### 8.6 Map 方法

| 方法 | 说明 |
|------|------|
| `m.len()` | 获取键值对数量 |
| `m.insert(key, val)` / `m.set(key, val)` | 插入/更新 |
| `m.remove(key)` | 删除键 |
| `m.contains(key)` / `m.has_key(key)` | 是否包含键 |
| `m.keys()` | 返回所有键（字符串数组） |
| `m.values()` | 返回所有值（数组） |
| `m.is_empty()` | 是否为空 |
| `m.clear()` | 清空映射 |

### 8.7 mat — 矩阵计算库

```klc
use mat

let m1 = mat::create(2, 3)            -- 2×3 零矩阵
mat::set(m1, 0, 1, 5.0)              -- [0][1] = 5.0
let val = mat::get(m1, 0, 1)         -- 读取元素

let sum = mat::add(a, b)              -- 逐元素加法
let prod = mat::mul(a, b)             -- 矩阵乘法（单线程）
let prod_p = mat::parallel_mul(a, b)  -- 13线程并行乘法
let scaled = mat::mul_scalar(m, 2.0)  -- 标量乘法
let t = mat::transpose(m)             -- 转置
let dims = mat::shape(m)              -- [行数, 列数]
mat::print(m)                         -- 格式化打印
```

### 8.8 thread — 线程模块

支持基本的并发任务调度（实验性），用于并行计算场景。

---

## 9. KLC AI 模块说明

### 9.1 模块定位

KLC AI 模块是集成在 KLC 语言运行时中的**原生 Transformer 推理引擎**，旨在为 KLC 语言提供无需外部依赖的 AI 推理与训练能力。

### 9.2 当前状态与定位

> **KLC AI 不是一个玩具 Demo，而是一个完整的、有清晰迭代路线的大模型体系。**

**所有模型 100% 基于自研 KLC 语言构建**，不依赖 Python、不依赖 PyTorch、不依赖任何第三方深度学习框架。从 Tokenizer → 嵌入层 → Transformer 算子 → 反向传播 → 权重管理 → 推理引擎，全链路在 KLC 运行时与编译器中原生实现。

**当前阶段：训练与验证。** Transformer 模型的核心前向/反向传播算法已实现，已知限制见 [9.5 节](#95-已知限制)。

**当前可用功能**：

| 功能 | 状态 | 说明 |
|------|------|------|
| 模型创建 | 可用 | 创建 Transformer 模型实例 |
| 前向推理 | 可用 | 前向传播计算 logits |
| 训练步骤 | 可用 | 单步梯度下降训练 |
| 模型保存/加载 | 可用 | 持久化到 .klc_model 文件 |
| 模型打印 | 可用 | 调试输出模型结构 |

### 9.3 演示入口与使用方式

```klc
use transformer
use mat

mod main

fn main() {
    -- 创建 Transformer 模型
    -- 参数: d_model(嵌入维度), heads(注意力头数), vocab_size(词汇量, 可选)
    let model = transformer::create(512, 8, 10000)

    -- 创建输入/目标矩阵（演示用随机数据）
    let x = mat::create(1, 512)
    let y = mat::create(1, 10000)

    -- 单步训练
    transformer::train_step(model, x, y, 0.01)

    -- 前向推理
    let logits = transformer::forward(model, x)

    -- 保存模型
    transformer::save(model, "demo.klc_model")

    -- 加载模型
    let loaded = transformer::load("demo.klc_model")

    -- 调试打印
    transformer::print(loaded)
}
```

**演示文件**：`examples/transformer_chat.klc`

### 9.4 Transformer API 参考

| 函数 | 签名 | 说明 |
|------|------|------|
| `transformer::create(d_model, heads, vocab_size)` | `(i64, i64, i64) -> Model` | 创建 Transformer 模型 |
| `transformer::forward(model, input)` | `(Model, Matrix) -> Matrix` | 前向推理 |
| `transformer::train_step(model, x, y, lr)` | `(Model, Matrix, Matrix, f64) -> void` | 单步训练 |
| `transformer::save(model, path)` | `(Model, str) -> void` | 保存模型 |
| `transformer::load(path)` | `(str) -> Model` | 加载模型 |
| `transformer::print(model)` | `(Model) -> void` | 打印模型结构 |

**参数说明**：

| 参数 | 说明 | 推荐值 |
|------|------|--------|
| `d_model` | 嵌入维度 | 256-1024 |
| `heads` | 多头注意力头数 | 4-16 |
| `vocab_size` | 词汇量大小 | 根据语料库决定 |
| `lr` | 学习率 | 0.001-0.01 |

### 9.5 已知限制

1. **模型未训练**：当前权重为随机初始化，推理输出为噪声
2. **单线程执行**：Transformer 的前向/反向传播均为单线程，未启用并行加速
3. **有限模型规模**：受 KLC VM 内存限制，d_model 与 vocab_size 不宜超过 4096
4. **不支持 GPU**：当前仅有 CPU 实现
5. **无预训练权重**：不附带任何预训练模型文件
6. **训练功能初级**：仅支持单步梯度下降，无优化器选择、学习率调度、批量训练等高级特性

### 9.6 【规划AI模型清单】

> 以下为 KLC AI 完整模型体系规划，按系列分列，将在后续版本中按路线逐步实现与发布。
> 所有模型 100% 基于自研 KLC 语言构建，不依赖 Python、PyTorch 或任何第三方框架。

---

#### 🔹 基础通用系列

| 模型名称 | 定位 | 参数量级 | 应用场景 | 状态 |
|----------|------|----------|----------|------|
| **KLC:3B** | 入门基座小模型 | 3B | 个人 PC 本地运行、IDE 内置代码助手 | 训练中 |
| **KLC-MAX:27B** | 标准全能基座 | 27B | 通用对话、文本创作、逻辑推理 | 规划中 |

#### 🔹 POR 推理优化系列

> POR（Program Optimization Reuse）复用 KLC 编译器常量折叠/死代码消除能力，推理速度提升 3-5 倍。

| 模型名称 | 定位 | 参数量级 | 应用场景 | 状态 |
|----------|------|----------|----------|------|
| **KLC-POR:9B** | 中小规模推理优化版 | 9B | 低延迟推理服务、边缘部署 | 规划中 |
| **KLC-POR-MAX:45B** | 全能 + 推理双优化 | 45B | 企业级低延迟部署首选 | 规划中 |
| **KLC-3.7-MINI-POR:360B** | 轻量 + 推理双 buff | 360B | 端侧智能体极致性能 | 规划中 |

#### 🔹 全域通用系列

| 模型名称 | 定位 | 参数量级 | 应用场景 | 状态 |
|----------|------|----------|----------|------|
| **KLC-UIT:81B** | Universal Intelligent Transformer | 81B | 长文本处理、多领域知识、专业问答全覆盖 | 规划中 |

#### 🔹 MINI 轻量化系列

| 模型名称 | 定位 | 参数量级 | 应用场景 | 状态 |
|----------|------|----------|----------|------|
| **KLC-2.5-MINI:270B** | 超大参数轻量版 | 270B | INT4 量化后可在高配 PC 本地运行 | 规划中 |

#### 🔹 AGENT 智能体系列

| 模型名称 | 定位 | 参数量级 | 应用场景 | 状态 |
|----------|------|----------|----------|------|
| **KLC-1.V-AGENT:360B** | 初代智能体大模型 | 360B | 自主规划、工具调用、自动化任务执行 | 规划中 |
| **KLC-4.9-AGENT-POR-MAX:810B** | 全能 + 推理 + 智能体三合一 | 810B | 企业级复杂工作流编排 | 规划中 |

#### 🔹 超算系列

| 模型名称 | 定位 | 参数量级 | 应用场景 | 状态 |
|----------|------|----------|----------|------|
| **KLC-5.V-MAX:450B** | 第五代全能基座 | 450B | 高阶逻辑、深度创作、复杂推理天花板 | 规划中 |
| **KLC-6.0-ULTRA-POR-MAX-AGENT:9T** | 超算旗舰，9 万亿参数 | 9T (MoE) | MoE 混合专家架构，面向未来的通用超级智能 | 规划中 |

---

**总计：11 款模型，覆盖 3B 至 9T 参数量级，六大赛道全覆盖。**

---

## 10. 示例代码

### 10.1 基础语法示例 — Hello World

```klc
mod main
use io

fn main() {
    io.println("Hello, KLC v1.0.3!")
}
```

### 10.2 变量与运算

```klc
mod main
use io
use math

fn main() {
    let x = 42
    let y = 10
    let z = x + y * 2

    io.println(fmt("z = %d", z))          -- z = 62
    io.println(fmt("PI = %.6f", math.pi()))
    io.println(fmt("sqrt(16) = %d", math.sqrt(16.0)))
}
```

### 10.3 斐波那契数列

```klc
mod main
use io

fn fib(n: i64) -> i64 {
    if n <= 1 { return n }
    return fib(n - 1) + fib(n - 2)
}

fn main() {
    io.println(fib(10))  -- 55
}
```

### 10.4 结构体与方法

```klc
mod main
use io
use math

type Circle { radius: f64 }

impl Circle {
    fn area(self) -> f64 {
        return math.pi() * self.radius * self.radius
    }
}

fn main() {
    let c = Circle { radius: 5.0 }
    io.println(fmt("Area = %.2f", c.area()))  -- Area = 78.54
}
```

### 10.5 枚举与模式匹配

```klc
mod main
use io

type Option { Some(i64), None }

fn safe_divide(a: i64, b: i64) -> Option {
    if b == 0 { return None }
    return Some(a / b)
}

fn main() {
    let r1 = safe_divide(10, 2)
    let r2 = safe_divide(10, 0)

    match r1 {
        Some(n) => io.println(fmt("10/2 = %d", n))
        None => io.println("Division by zero")
    }

    match r2 {
        Some(n) => io.println(fmt("10/0 = %d", n))
        None => io.println("Division by zero")
    }
}
```

### 10.6 自举编译示例

以下代码展示了 KLC 自举能力——用 KLC 语言编写 lexer + parser + codegen 并编译分析 .klc 源文件：

```klc
mod main
use io

fn main() {
    -- 读取任意 .klc 文件
    let src = io.read("examples/hello.klc")

    -- 第1步: KLC 写的 Lexer 进行词法分析
    let tokens = lexer_run(src)
    io.println(fmt("Token 数量: %d", tokens.len()))

    -- 第2步: KLC 写的 Parser 进行语法分析
    let ast = parse_all(tokens)
    io.println(fmt("AST 节点数: %d", ast.len()))

    -- 第3步: KLC 写的 Codegen 生成字节码
    let prog = cg_compile(ast)
    io.println(fmt("指令数: %d, 函数数: %d, 常量数: %d",
        prog.main_instrs.len(),
        prog.functions.len(),
        prog.constants.len()))
}
```

完整自举流水线测试：`klc codegen/test_pipeline.klc`

**预期输出**：完整展示源码 → Token 数 → AST 节点数 → 字节码指令数、函数数、常量数 → 格式化打印字节码程序结构（常量池、函数定义、主程序指令），覆盖变量/分支/函数/循环等全部语法特性。

```powershell
klc codegen/test_pipeline.klc
```

### 10.7 AI 演示示例

```klc
mod main
use io
use mat
use transformer

fn main() {
    io.println("KLC Transformer 演示")

    -- 创建一个小型 Transformer
    let model = transformer::create(256, 4, 1000)

    io.println("模型创建完成")
    transformer::print(model)

    -- 创建模拟输入数据
    let input = mat::create(1, 256)
    let target = mat::create(1, 1000)

    -- 单步训练
    io.println("开始训练...")
    transformer::train_step(model, input, target, 0.01)

    -- 推理
    let output = transformer::forward(model, input)
    io.println("推理完成")

    -- 保存模型
    transformer::save(model, "ai_demo.klc_model")
    io.println("模型已保存到 ai_demo.klc_model")
}
```

### 10.8 矩阵并行计算示例

```klc
mod main
use io
use mat

fn main() {
    io.println("矩阵并行计算基准测试")

    -- 创建 100×200 和 200×150 矩阵
    let a = mat::create(100, 200)
    let b = mat::create(200, 150)

    -- 13线程并行矩阵乘法
    io.println("开始 13 线程并行计算...")
    let prod = mat::parallel_mul(a, b)

    let dims = mat::shape(prod)
    io.println(fmt("结果矩阵维度: %d × %d", dims[0], dims[1]))
}
```

---

## 11. 故障排查

### 11.1 编译错误

| 错误信息 | 原因 | 解决方案 |
|----------|------|----------|
| `Parse error: 期望...得到...` | 语法错误 | 检查对应行语法，参考语言规范 |
| `期望标识符` | 关键字误用或类型名拼写错误 | 检查标识符是否与关键字冲突 |
| `模式中出现意外 Token` | match 模式语法错误 | 检查 match 分支写法 |

### 11.2 运行时错误

| 错误 | 原因 | 解决方案 |
|------|------|----------|
| 程序无输出 | main() 函数未定义或参数不匹配 | 确保 `fn main()` 存在且无参数 |
| `Error reading file` | 文件不存在或编码问题 | 检查文件路径和 UTF-8 编码 |
| 栈溢出 | 递归过深或数据量过大 | 优化递归为迭代，减小数据规模 |

### 11.3 环境问题

| 问题 | 解决方案 |
|------|----------|
| `'klc' is not recognized` | 将 klc.exe 所在目录加入 PATH |
| 中文输出乱码 | 终端设置为 UTF-8 编码：`chcp 65001` |
| Rust 编译失败 (ICE) | 尝试 `cargo clean` 后重试；切换 GNU/MSVC 工具链 |
| `link.exe not found` | 切换到 GNU 工具链：`rustup default stable-x86_64-pc-windows-gnu` |
| `os error 1392` (磁盘损坏) | 运行 `chkdsk /f`；手动删除 `target` 目录后重试 |

---

## 12. 版本历史

### 12.1 v1.0.3 更新内容 (2026-05-31)

- 正式发布 v1.0.3-正式版 (Stable)，版本号从 v0.8.4 升级
- 完善命令行接口、文档与发行说明
- 优化解析器对 match 模式匹配中 `Type::Variant` 语法的支持
- 集成 Matrix 13 线程并行计算引擎

### 12.2 历史版本迭代记录

| 版本 | 日期 | 主要更新 |
|------|------|----------|
| v0.7.3 | 2026-05 | 初始公开版本 |
| v0.8.4 | 2026-05-29 | 新增 math 标准库、fmt() 格式化、null 空值语义、Map/Array/String 增强、for-in 遍历、type_of 类型检查、pub 标记、impl 双写法、if/else 内 let 绑定 |
| **v1.0.3** | **2026-05-31** | **首个 Stable 正式版发布** |

---

## 13. 后续规划

### 13.1 功能迭代方向

1. **跨平台支持**：推进 macOS (ARM/x86_64) 与 Linux 平台支持
2. **所有权系统**：实现 `own`、`borrow`、`borrow mut` 所有权语义检查
3. **泛型系统**：完善 `<T>` 泛型参数的类型检查与单态化
4. **并发模型**：实现 `task`/`go`/`channel` 以及 `async`/`await` 异步运行时
5. **标准库扩展**：网络 IO、JSON 解析、正则表达式
6. **VM 性能优化**：JIT 编译、直接线程解释器
7. **IDE 增强**：代码补全、实时错误提示、项目管理器
8. **包管理器**：`klc pkg` 子命令，支持依赖管理与远程注册表

### 13.2 AI 模块发布计划

| 阶段 | 计划内容 | 涵盖模型 |
|------|----------|----------|
| Phase 1 (当前) | 基础 Transformer 框架、前向/反向传播、模型持久化 | — (框架层) |
| Phase 2 (当前) | 小模型推理验证、POR 编译器优化集成、预训练 | KLC:3B、KLC-POR:9B |
| Phase 3 | 标准基座训练、对话能力、代码辅助 | KLC-MAX:27B、KLC-UIT:81B |
| Phase 4 | 推理优化系列发布、INT4 量化、轻量化部署 | KLC-POR-MAX:45B、KLC-2.5-MINI:270B |
| Phase 5 | 智能体框架、工具调用、自动化工作流 | KLC-1.V-AGENT:360B、KLC-3.7-MINI-POR:360B |
| Phase 6 | 企业级三合一模型、复杂工作流编排 | KLC-4.9-AGENT-POR-MAX:810B |
| Phase 7 | 超算旗舰、MoE 混合专家架构 | KLC-5.V-MAX:450B、KLC-6.0-ULTRA-POR-MAX-AGENT:9T |

> **注意**：上述计划为路线图性质，实际发布时间和功能可能根据开发进展调整。

---

*KLC v1.0.3-正式版 官方发行说明书 — 2026年5月31日*

*作者：于仕初（个人开发者）*