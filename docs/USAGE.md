# KLC 使用说明（v1.4.6-β）

本文档面向想要构建、运行或在项目中使用 KLC 的开发者，包含安装、常见命令、示例与 CI 建议。

## 环境与依赖

- 操作系统：Windows / Linux / macOS（开发与运行均已测试，原生 PE 生成器仅针对 Windows）
- 需要安装 Rust 工具链（stable）：`rustc`、`cargo`。

安装 Rust（如尚未安装）:

    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

确保 `cargo` 在 PATH 中后，进入项目目录进行构建。

## 先明确：当前依赖的真实边界

下面通过一张简单的对照表，明确区分 **运行依赖** 与 **开发依赖**：

| 依赖类型 | 是否存在 | 说明 |
|----------|---------:|------|
| **运行依赖 (runtime)** | ❌ | 完全不存在 —— 你编译出来的 `klc.exe` 是纯原生 Windows PE 文件，**不需要任何运行库、不需要安装任何软件**，双击就能在任何 Windows 电脑上运行，甚至可以放在 U 盘里随身携带。 |
| **开发依赖 (dev)** | ⚠️ | 暂时存在 —— 只有**编译 KLC 编译器本身**的时候需要 Rust 工具链（`rustc` / `cargo`）；一旦编译完成，Rust 就可以完全卸载，不影响 KLC 的任何功能。 |

这与常见的解释型/VM 语言有本质区别：

- Python：运行 Python 程序必须安装 Python 解释器（运行时依赖存在）。
- Java：运行 Java 程序必须安装 JVM（运行时依赖存在）。
- KLC：编译后生成的可执行文件**运行时零依赖**；仅在构建编译器或重新编译时才需要开发依赖（Rust）。

该边界意味着：你可以在一台装有 Rust 的机器上构建 `klc.exe`，然后把生成的可执行文件分发给任意 Windows 用户，他们无需安装任何额外软件即可运行。

## 构建 KLC 可执行文件

在仓库的 `klc/` 子目录运行：

    cd klc
    # Debug 构建
    cargo build

    # Release 构建（推荐用于日常使用）
    cargo build --release

构建完成后，二进制文件位于：

- `target/debug/klc`
- `target/release/klc`

将 `target/release/klc` 拷贝到系统 PATH 可直接使用 `klc` 命令。

## CLI 常用子命令

基础命令与说明：

- `klc <file.klc>` 或 `klc run <file.klc>` — 在 VM 上执行脚本
- `klc build [OPTIONS] <source>` — 构建项目 / 生成可执行
- `klc fmt [OPTIONS] <file>...` — 格式化代码
- `klc check <file.klc>` — 语法检查
- `klc --debug <file.klc>` — 以 debug 模式运行并打印 Tokens/AST/Bytecode
- `klc --ide` — 启动自带图形 IDE
- `klc version` / `klc --version` — 显示版本信息

常用 `klc build` 选项：

- `--native`：生成原生 Windows PE 可执行文件（实验性）
- `--no-opt`：禁用编译器优化
- `-g`, `--debug-info`：生成 DWARF 调试信息
- `-o <output>`：指定输出路径

`klc fmt` 选项：

- `--check`：仅检查格式（适用于 CI），发现不合规时返回非零退出码
- `--indent <N>`：设置缩进宽度（默认 4）

## KLC IDE 图形界面操作手册

KLC 自带基于 Win32 原生 API 的轻量级图形 IDE，提供代码编辑、项目管理、编译运行一体化体验。

### 启动 IDE

```powershell
klc --ide
```

### 界面布局

```
┌──────────────────────────────────────────────┐
│  菜单栏: 文件 | 运行 | 编译 | 视图          │
├──────────┬───────────────────────────────────┤
│ Explorer │ Tab1 | Tab2 | Tab3                │
│ 项目树   ├──────┬────────────────────────────┤
│ (.klc)   │行号   │ 代码编辑区                  │
│          │      │                            │
│          ├──────┴────────────────────────────┤
│          │ 输出面板 (编译日志/运行结果)        │
├──────────┴───────────────────────────────────┤
│ 状态栏: 行 N 列 N | UTF-8 | 修改             │
└──────────────────────────────────────────────┘
```

### 菜单栏

| 菜单 | 选项 | 快捷键 | 说明 |
|------|------|--------|------|
| **文件** | 新建 | Ctrl+N | 创建新标签页 |
| | 打开 | Ctrl+O | 打开 .klc 文件 |
| | 保存 | Ctrl+S | 保存当前文件 |
| | 退出 | | 关闭 IDE |
| **运行** | 运行 | F5 | 在 VM 上执行当前脚本 |
| | 编译 | Ctrl+B | 编译为字节码 |
| | 构建并运行 | | 编译后自动运行 |
| **编译** | 语法检查 | | 仅检查语法 |
| | 格式化 | | 格式化代码 |
| **视图** | 暗色主题 | | 切换暗色主题 |
| | 亮色主题 | | 切换亮色主题 |

### 编辑器核心操作

#### 多标签编辑

- **新建标签** `Ctrl+N` — 在编辑器中打开一个新的空白标签页，标签独立保存文本内容
- **关闭标签** `Ctrl+W` — 关闭当前标签页（至少保留一个标签页）
- **切换标签** — 点击标签栏任意标签标题切换，编辑器保留各自的内容和修改状态

每个标签页携带独立的：
- 编辑器文本内容
- 文件路径（打开/保存时绑定）
- 修改状态（关闭时提示保存）

#### 代码编辑

- **语法高亮**：关键字、字符串、注释、数字、函数调用等不同颜色区分
- **括号自动配对**：输入 `(` `{` `[` 自动补全配对，输入 `)` `}` `]` `"` `'` 时若光标后字符匹配则跳过
- **自动缩进**：回车自动继承当前行缩进
- **Tab 输入 4 空格**：Tab 键转换为 4 个空格
- **注释切换**：`Ctrl+/` — 切换当前行为 `--` 注释/取消注释

#### 查找替换

- **`Ctrl+F`** — 打开查找对话框，支持：
  - 查找匹配文本并高亮
  - 显示匹配计数
  - 回车查找下一个

#### 代码折叠

- **`Ctrl+M`** — 切换当前光标所在函数/代码块的折叠状态（基于缩进分析）

#### 智能提示 (IntelliSense)

- **`Ctrl+Space`** — 在光标位置触发代码补全提示，基于当前单词前缀匹配

### 项目树

左侧 Explorer 面板显示当前工作目录下所有 `.klc` 文件：

- **双击 .klc 文件** — 在当前标签页打开文件内容
- 文件以树形列表呈现，自动刷新

### 输出面板

底部输出面板显示：

| 消息类型 | 前缀 | 说明 |
|----------|------|------|
| 文件操作 | `[文件]` | 新建、打开、保存确认 |
| 运行结果 | `[运行]` | 脚本执行输出（stdout） |
| 编译日志 | `[编译]` | 编译进度和统计 |
| 错误信息 | `[错误]` | 编译/运行时的错误消息 |

- **清空输出** — 菜单"编译 → 清空输出"或右键菜单

### 状态栏

底栏从左到右显示：

- 当前光标位置：`行 N 列 N`
- 文件编码：`UTF-8`
- 修改标记：当文件有未保存更改时显示 `已修改`

### 主题切换

支持两种编辑主题，通过菜单 `视图 → 暗色主题/亮色主题` 切换：

| 主题 | 背景色 | 适用场景 |
|------|--------|----------|
| **暗色 (Dark)** | 深灰底色 | 夜间编程、沉浸式编码 |
| **亮色 (Light)** | 浅灰底色 | 日间编程、高亮环境 |

切换即时生效，编辑器和输出面板同步更新。

### 快捷键速查表

| 快捷键 | 功能 |
|--------|------|
| `Ctrl+N` | 新建标签 |
| `Ctrl+W` | 关闭标签 |
| `Ctrl+O` | 打开文件 |
| `Ctrl+S` | 保存文件 |
| `F5` | 运行脚本 |
| `Ctrl+B` | 编译 |
| `Ctrl+F` | 查找 |
| `Ctrl+M` | 折叠/展开代码块 |
| `Ctrl+Space` | 智能提示 |
| `Ctrl+Shift+/` | 注释切换 |
| `Ctrl+A` | 全选 |

### 典型工作流

1. **新建项目**：启动 IDE → 默认打开空白标签 → 输入代码
2. **多文件编辑**：`Ctrl+N` 新建多个标签 → 在标签间切换编辑
3. **打开现有文件**：`Ctrl+O` 或双击项目树中的 `.klc` 文件
4. **保存**：`Ctrl+S` → 输出面板显示保存确认
5. **运行调试**：`F5` 运行 → 输出面板显示运行结果
6. **编译发布**：`Ctrl+B` 编译 → 输出面板显示编译统计

### 最新改动 (v1.4.3+)

| 改动 | 说明 |
|------|------|
| **多标签编辑** | 每个标签独立保存文本，切换时自动保存/恢复 |
| **标签切换修复** | 修复了 64 位系统上 `NMHDR` 结构体对齐导致的点击切换失效问题 |
| **统一编辑器** | 单全局编辑器 + 文本保存/恢复机制，避免 Win32 控件显隐导致的内容丢失 |

---

## 运行与示例

1) Hello World

保存为 `hello.klc`：

```klc
mod main
use io

fn main() {
    println("Hello, KLC!")
}
```

运行：

    klc run hello.klc

2) math 示例

```klc
mod main
use math

fn main() {
    println(fmt("PI = %.6f", math.pi()))
    println(fmt("sqrt(16) = %d", math.sqrt(16.0)))
}
```

3) 使用 fmt() 进行格式化输出

```klc
let s = fmt("Name: %s, Score: %d", "Bob", 88)
println(s)
```

4) Map/数组/字符串 常见操作

```klc
let m = {"x": 1, "y": 2}
m.insert("z", 3)
if m.contains("y") { println(fmt("y=%d", m["y"])) }

let arr = [5, 1, 3]
arr.sort()
for v in arr { println(v) }

let s = " hello "
println(s.trim())
```

## 调试与诊断

- 打印 Token/AST/Bytecode：`klc run --debug file.klc` 或 `klc --debug file.klc`
- 构建时输出详细信息：`klc build -g <source>`（生成 DWARF）
- 格式化检查（CI）：`klc fmt --check <file>`（返回码用于 CI 判断）

## 在 CI 中使用（示例 GitHub Actions）

简要策略：在 CI 中安装 Rust，构建 `klc` 并执行格式检查与测试。

示例步骤（伪代码）：

    - name: Install Rust
      run: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

    - name: Build KLC
      run: cd klc && cargo build --release

    - name: Format check
      run: cd klc && target/release/klc fmt --check examples/*.klc

    - name: Syntax check
      run: cd klc && target/release/klc check examples/*.klc || true

（注意：具体 CI 配置请根据 runner 平台调整）

## 原生可执行生成（实验性）

使用 `klc build --native -o out.exe <source>` 可以生成最小化 PE 可执行文件。该功能为实验性实现，可能在不同平台或权限环境下表现不一致。

操作建议：

- 在 Windows 环境使用 `target/release/klc build --native -o my.exe examples/hello.klc`。
- 检查输出文件大小与执行结果，谨慎分发（安全性审查）。

## 常见问题 (FAQ)

Q: 为什么 `klc run` 报找不到模块或文件？

A: 请检查当前工作目录与源文件路径是否正确。`klc run` 会按文件路径读取并解析模块引用，建议在项目根目录执行 `klc build`。

Q: 如何在项目中使用多个模块？

A: 使用 `module` / `use` 语法，并确保模块文件位于工程目录中，`klc build` 会通过 `module::ProjectBuilder` 解析并打包模块。
