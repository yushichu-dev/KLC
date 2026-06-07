# KLC (v1.4.3)

KLC（Kaleidoscope Language Compiler）是一门从零实现的现代编程语言，包含编译器、字节码生成器、轻量级栈式虚拟机与图形化 IDE。KLC 以性能、内存安全与可用性为主要目标，适合学习编译器实现、嵌入式脚本和实验性语言特性。

主要特点：
- 现代语法（函数、类型、枚举、模式匹配、协程等）
- 所有权与借用语义（借鉴 Rust 思想）
- 内置 VM 与可选 JIT 即时编译
- 丰富的标准库：IO、数学、字符串、数组、Map 等
- 内置格式化工具 `klc fmt`
- 图形化 IDE 支持（括号配对、自动缩进、注释切换等）

本项目使用 Rust 编写，零运行时依赖。

---

## 快速开始

```bash
# 安装 Rust（如尚未安装）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 编译
cd klc
cargo build --release

# 运行示例
./target/release/klc examples/hello.klc

# 启动 IDE
./target/release/klc
```

---

## 常用命令

| 命令 | 说明 |
|------|------|
| `klc <file.klc>` | VM 执行脚本 |
| `klc run <file.klc>` | VM 执行（同直接运行） |
| `klc --jit <file.klc>` | 启用 JIT 即时编译 |
| `klc --debug <file.klc>` | 显示 Token/AST/字节码 |
| `klc check <file.klc>` | 语法检查 |
| `klc fmt <file>...` | 格式化代码 |
| `klc build --native <file>` | 编译为原生 Windows EXE |
| `klc --version` | 显示版本信息 |
| `klc --help` | 显示帮助 |
| 双击 `klc.exe` | 启动图形化 IDE |

---

## CLI 示例

```klc
-- hello.klc
println("Hello, KLC!")
```

运行：
```bash
klc hello.klc
# 输出: Hello, KLC!
```

---

## KLC IDE 使用说明

双击 `klc.exe` 或在命令行不带参数执行，即可启动 KLC 图形化编辑器。

### 快捷键

| 快捷键 | 功能 |
|--------|------|
| `Ctrl+N` | 新建文件 |
| `Ctrl+O` | 打开文件 |
| `Ctrl+S` | 保存文件 |
| `Ctrl+B` | 编译 |
| `Ctrl+Shift+B` | 编译并运行 |
| `F5` | 运行 |
| `Ctrl+/` | 切换行注释（`--`） |

### 编辑功能

| 功能 | 说明 |
|------|------|
| **括号自动配对** | 输入 `(`/`{`/`[`/`"`/`'` 自动补全配对，光标居中 |
| **智能跳过** | 输入右括号/引号时若下一字符相同，自动跳过 |
| **配对删除** | Backspace 删除 `()` 时同时删除两个 |
| **自动缩进** | Enter 换行自动继承上一行缩进 |
| **Tab 缩进** | Tab 插入 4 个空格 |
| **行号** | 左侧显示行号 |
| **自动折行** | 超长行自动折行显示 |

---

## 标准库

### 数学函数
`abs`, `sqrt`, `pow`, `round`, `floor`, `ceil`, `max`, `min`, `math_pi`, `math_e`

```klc
let result = abs(-42)          -- 42
let root = sqrt(16.0)          -- 4.0
let power = pow(2, 3)          -- 8.0
```

带命名空间前缀也兼容：`math::abs(-10)`

### 字符串函数
`str_upper`, `str_lower`, `str_trim`, `str_contains`, `str_replace`, `str_len`

```klc
let upper = str_upper("hello")               -- "HELLO"
let found = str_contains("abc", "b")         -- true
let replaced = str_replace("aaa", "a", "b")  -- "bbb"
```

方法形式也兼容：`s.trim()`、`s.to_upper()`、`s.len()`

### 数组函数
`arr_len`, `arr_push`, `arr_pop`, `arr_slice`

```klc
let a = __array(1, 2, 3)
arr_push(a, 4)
let last = arr_pop(a)
let sub = arr_slice(a, 0, 2)
```

方法形式也兼容：`a.push(4)`、`a.pop()`、`a.len()`

### IO 函数
`print`, `println`, `input`, `input_num`, `eprint`, `eprintln`
`file_read`, `file_write`, `file_append`, `file_exists`, `file_delete`
`fmt_printf`, `print_table`, `print_debug`, `flush`, `stdin_is_empty`

```klc
println("Hello, World!")
let name = input("Name: ")
file_write("test.txt", "Hello")
let content = file_read("test.txt")
```

### 工具函数
| 函数 | 说明 |
|------|------|
| `assert(cond, msg?)` | 断言校验，失败报错终止 |
| `env_get(key)` | 读取系统环境变量 |
| `type_of(val)` | 返回值的类型名 |
| `sleep(ms)` | 毫秒级休眠 |
| `is_null(val)` | 判断是否为 Null |
| `to_string(val)` | 转换为字符串 |
| `int_of(val)` / `float_of(val)` / `str_of(val)` | 类型转换 |
| `parse_int(s)` / `parse_float(s)` | 字符串解析 |

---

## 语言特性

- `let` / `let mut` 变量声明
- `fn` 函数定义
- `if` / `else` / `while` / `for` / `loop` 控制流
- `struct` 结构体、`enum` 枚举
- `match` 模式匹配
- `impl` 方法实现
- 数组 `[]`、Map `{}`
- 管道操作符 `|>`
- 字符串连接 `++`
- 逻辑 `and` / `or` / `not`
- 注释 `-- 单行` / `--| 多行 |--`

---

## 项目结构

```
klc/
├── src/           # 编译器 + VM + IDE（Rust）
│   ├── gui/       # IDE 图形界面
│   ├── stdlib/    # 标准库（IO/数学/字符串/数组/工具）
│   └── jit/       # JIT 即时编译
├── test/          # KLC 测试脚本
├── examples/      # 示例程序
├── docs/          # 文档
├── benchmarks/    # 性能基准
└── Cargo.toml
```

---

## 测试

```bash
cargo test                          # 运行全部单元测试（162 项）
klc test/std_all_new.klc           # 运行新增功能全量测试
klc test/std_io_quick.klc          # 运行 IO 标准库测试
```
