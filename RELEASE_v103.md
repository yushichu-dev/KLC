# KLC v1.0.3-正式版 — 发行说明与使用说明

发布日期: 2026-05-29

概览: 本次小版本以可用性与标准库扩展为主，新增数学库、格式化函数、空值语义与若干容器/字符串/数组的便捷方法，同时放宽了部分语法限制（例如允许在 if/else 块内定义 `let` 变量）。本版本对 v0.7.3 保持向下兼容。

---

## 1. 版本要点（Highlights）

- math 标准库：一组常用数学函数（sin/cos/sqrt/exp/log/round/…），可通过 `math.xxx()` 或 `math::xxx()` 调用。
- 新增 `fmt()` 格式化函数，支持 `%` 风格的格式说明符（`%d`、`%f`、`%s`、`%x`、`%b` 等）。
- 引入 `null` 空值以及 `is_null()` 辅助函数。
- Map/Array/String 增强：新增 `insert`/`remove`/`keys`/`values`、`contains`、`index_of`、`trim`、`to_upper` 等常用方法。
- 支持 `for item in arr` 数组遍历语法。
- 新增 `type_of`、`int_of`、`float_of`、`str_of` 等类型检查与转换工具函数。
- 支持 `pub` 标记（语义标记，供后续模块系统使用）。
- 允许 `Type::method()` 與 `Type.method()` 两种调用写法等价。
- 允许在 `if` / `else` 内使用 `let` 局部绑定。

---

## 2. 兼容性与迁移指南

- 向下兼容: 本版本保持与 `v0.7.3` 的兼容性，已有代码应能直接编译运行。
- 新关键字: `null` 与 `pub` 为新增关键字。若你的代码中使用了同名标识符（例如变量名 `null`），请重命名以避免解析冲突。

迁移建议:

- 使用 `klc check` 对代码进行一次完整语法检查，确保未受新关键字影响。
- 若使用第三方工具或脚本解析 KLC 源代码，请更新它们以识别 `null` 作为字面量。

---

## 3. 快速上手（CLI 与常用命令）

在工程根目录编译并生成 `klc` 可执行文件（需要 Rust 工具链）:

    cargo build               # debug 构建，二进制位于 target/debug/klc
    cargo build --release     # release 构建，二进制位于 target/release/klc

基础用法:

- 运行脚本（VM 执行）:

    klc <source.klc>
    klc run <source.klc>

- 构建项目（可选原生 PE）:

    klc build [OPTIONS] <source>

    常用选项:
      --native           # 生成原生 Windows EXE（使用内部 PE 生成器）
      --no-opt           # 禁用编译器优化
      -g, --debug-info   # 生成 DWARF 调试信息
      -o <output>        # 指定输出路径

- 格式化代码:

    klc fmt [OPTIONS] <file>...

    格式化选项:
      --check            # 仅检查格式，不写入文件
      --indent <N>       # 缩进宽度（默认 4）

- 语法检查:

    klc check <source.klc>

- 调试运行（显示 Tokens/AST/Bytecode）:

    klc --debug <source.klc>
    klc run --debug <source.klc>

- 启动图形 IDE:

    klc --ide

- 查看版本/帮助:

    klc version
    klc --version
    klc help

注意: `klc build --native` 会在内部通过 `native_codegen` 生成 PE 文件，生成过程可能依赖平台工具链与写入权限。

---

## 4. 重要功能示例（代码片段与说明）

下面的示例覆盖了 v1.0.3-正式版 中新增或增强的常用功能，复制到文件后用 `klc run` 或 `klc check` 验证。

1) math 标准库

```klc
mod main
use math

fn main() {
    println(fmt("PI = %.6f", math.pi()))
    println("sin(0) = " ++ to_str(math.sin(0.0)))
}
```

2) fmt() 格式化（返回字符串，可与 println 配合）

```klc
mod main
use io

fn main() {
    let s = fmt("Name: %s, Age: %d", "Alice", 30)
    println(s)
    println(fmt("Hex: 0x%x", 255))
}
```

3) null 与 is_null

```klc
mod main

fn main() {
    let v = null
    if is_null(v) {
        println("v is null")
    }
}
```

4) Map 操作

```klc
let m = {"a": 1, "b": 2}
m.insert("c", 3)
if m.contains("b") {
    println(fmt("b => %d", m["b"]))
}
for k in m.keys() {
    println(fmt("key: %s", k))
}
```

5) for-in 数组遍历 & 数组方法

```klc
let arr = [3, 1, 4, 1, 5]
for x in arr {
    println(x)
}
arr.sort()
println(fmt("first: %d", arr[0]))
```

6) 字符串常用方法

```klc
let s = "  hello KLC  "
println(s.trim().to_upper())   -- 输出: HELLO KLC
println(s.replace("KLC", "World"))
```

7) 类型检查与转换

```klc
println(type_of(42))          -- i64
println(type_of(3.14))        -- f64
println(type_of(null))        -- Null
let n = int_of("123")
let f = float_of("3.14")
```

8) pub 与关联函数双写法

```klc
pub type Point { x: f64, y: f64 }
impl Point {
    fn origin() -> Point { Point { x: 0.0, y: 0.0 } }
}
let p1 = Point::origin()
let p2 = Point.origin()
```

9) if/else 块内的 let 绑定

```klc
let result = if 5 > 3 {
    let tmp = 10
    tmp + 1
} else {
    let tmp = 1
    tmp
}
```

---

## 5. 构建与发布（开发者）

依赖: Rust (stable) 工具链。

从源码构建:

    git clone <repo>
    cd klc
    cargo build --release

运行本地编译器:

    target/release/klc run examples/hello.klc

将 KLC 用作构建工具来生成原生可执行文件:

    target/release/klc build --native -o myprog.exe examples/hello.klc

注意: 原生生成器为实验性功能，生成的 PE 文件为最小化加载器，请在受控环境下测试。

---

## 6. 调试与诊断

- 打印 Token/AST/Bytecode: 使用 `--debug` 选项（如 `klc run --debug file.klc`）。
- 生成 DWARF 调试信息: 在 `klc build` 时添加 `-g` 或 `--debug-info`。
- 格式化检查（CI 场景）: `klc fmt --check <file>`，返回非零退出码表示需格式化。

---

## 7. 文档与示例代码

- 语言规范: [docs/lang_spec.md](docs/lang_spec.md)
- 示例程序: `examples/` 目录（包含 `hello.klc`, `fibonacci.klc`, `transformer_chat.klc` 等）
- 本次详细使用说明: [docs/USAGE.md](docs/USAGE.md)

---

## 8. 项目文件变更（简要）

- `src/token.rs` — 新增 `Null` 关键字
- `src/ast.rs` — 新增 `Expr::Null` 变体
- `src/parser.rs` — 支持 `null` 字面量与 `pub` 前缀
- `src/codegen.rs` — 为 `null` 生成字节码，同时增强 `for`、`impl` 写法支持
- `src/vm.rs` — math/ fmt / Map/Array/String 的实现
- `Cargo.toml`, `resources.rc`, `src/main.rs` — 版本号更新为 `1.0.3-正式版`
