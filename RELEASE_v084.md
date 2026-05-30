# KLC v0.8.4 更新说明

## 版本信息
- 版本号: KLC v0.8.4
- 发布日期: 2026-05-29
- 兼容性: 100% 向下兼容 v0.7.3

---

## 新增功能

### 1. math 标准库
内置数学函数库，全局可直接调用 `math.xxx()`：

| 函数 | 说明 | 示例 |
|------|------|------|
| `math.pi()` | 圆周率常量 | `math.pi()` → 3.14159... |
| `math.e()` | 自然对数底数 | `math.e()` → 2.71828... |
| `math.sin(x)` | 正弦 | `math.sin(0.0)` |
| `math.cos(x)` | 余弦 | `math.cos(0.0)` |
| `math.sqrt(x)` | 平方根 | `math.sqrt(16.0)` |
| `math.exp(x)` | e 的 x 次方 | `math.exp(1.0)` |
| `math.log(x)` | 自然对数 (ln) | `math.log(math.e())` |
| `math.log2(x)` | 以 2 为底对数 | `math.log2(8.0)` |
| `math.log10(x)` | 以 10 为底对数 | `math.log10(100.0)` |
| `math.tanh(x)` | 双曲正切 | `math.tanh(1.0)` |
| `math.abs(x)` | 绝对值 (支持整数/浮点) | `math.abs(-42)` |
| `math.min(a, b)` | 最小值 | `math.min(3, 7)` |
| `math.max(a, b)` | 最大值 | `math.max(3, 7)` |
| `math.pow(base, exp)` | 幂运算 | `math.pow(2.0, 10.0)` |
| `math.floor(x)` | 向下取整 | `math.floor(3.7)` |
| `math.ceil(x)` | 向上取整 | `math.ceil(3.2)` |
| `math.round(x)` | 四舍五入 | `math.round(3.5)` |

调用方式: `math.sin(0.0)` 或 `math::sin(0.0)`，两种写法等价。

### 2. % 格式化语法
使用 `fmt()` 函数进行格式化输出：

```klc
fmt("Hello, %s!", "KLC")
fmt("PI = %.4f", 3.14159)
fmt("Age: %d, Score: %d", 25, 98)
fmt("Binary: %b", 10)
fmt("Hex: %x", 255)
fmt("Exponential: %e", 1000.0)
```

格式说明符:
- `%d` / `%i` — 整数
- `%f` — 浮点数 (默认 6 位小数，可指定精度 `%.2f`)
- `%s` — 字符串（任意类型自动转为字符串）
- `%e` — 科学计数法
- `%x` / `%X` — 十六进制
- `%b` — 布尔/二进制
- `%p` — 自动选择格式

### 3. null 空值
新增 `null` 关键字，表示空值：

```klc
let x = null
if x == null {
    println("x 是空值")
}
```

配套 `is_null()` 函数用于空值检测:
```klc
if is_null(value) { ... }
```

### 4. Map 容器增强
新增以下 Map 方法:

```klc
let m = {"key1": "val1", "key2": "val2"}

m.insert("key3", "val3")      -- 插入/更新键值对
m.remove("key1")              -- 删除键值对
m.contains("key2")           -- 检查键是否存在 (返回 Bool)
m.keys()                      -- 返回所有键的数组
m.values()                    -- 返回所有值的数组
m.len()                       -- 返回元素数量
m.is_empty()                  -- 是否为空
m.clear()                     -- 清空所有元素
```

### 5. for 循环增强
支持数组遍历：

```klc
let arr = [10, 20, 30]
for item in arr {
    println(item)
}
```

原有 `while` 循环完全保留，两种循环可混用。

### 6. 字符串增强
新增字符串方法:

```klc
"  hello  ".trim()                 -- 去除首尾空白
"hello".to_upper()                -- 转大写
"HELLO".to_lower()                -- 转小写
"Hello World".starts_with("Hello") -- 前缀检查
"Hello World".ends_with("World")   -- 后缀检查
"a,b,c".split(",")                 -- 分割为数组
"hello world".replace("world", "KLC") -- 替换
"abc".chars()                      -- 转为字符数组
"hello".str_len()                  -- 字符串长度
```

### 7. 数组增强
新增数组方法:

```klc
let arr = [3, 1, 4, 1, 5]
arr.contains(3)               -- 包含检查
arr.index_of(4)                -- 查找索引 (未找到返回 -1)
arr.reverse()                  -- 反转
arr.sort()                      -- 排序
arr.join(", ")                  -- 连接为字符串
arr.clear()                    -- 清空
```

### 8. 类型检查函数
```klc
type_of(42)          -- "i64"
type_of(3.14)        -- "f64"
type_of("hello")     -- "String"
type_of(true)        -- "Bool"
type_of(null)        -- "Null"
type_of([1, 2])      -- "Array"

int_of("123")        -- 字符串转整数
float_of("3.14")     -- 字符串转浮点
str_of(42)           -- 值转字符串
to_str(42)           -- 值转字符串 (同 str_of)
char_at("hello", 1)  -- 取字符 'e'
parse_int("42")      -- 安全解析整数 → Option
parse_float("3.14")  -- 安全解析浮点 → Option
```

### 9. pub 访问修饰符
支持 `pub` 作为可见性修饰符前缀:

```klc
pub fn public_function() { ... }
pub type PublicType { field: i64 }
pub impl PublicType { ... }
```

当前版本中 `pub` 为语义标记（编译通过，不做访问控制检查），为后续模块系统完善预留。

### 10. 关联函数双写法
`Type::method()` 和 `Type.method()` 两种写法完全等价:

```klc
impl MyType {
    fn create() -> MyType { ... }
}

-- 两种调用方式等价:
let a = MyType::create()
let b = MyType.create()
```

### 11. if 块内 let 变量绑定
解除限制，允许在 `if` / `else` 代码块内定义 `let` 变量:

```klc
let result = if x > 5 {
    let y = x * 2
    y + 1
} else {
    let z = x + 10
    z
}
```

---

## 语法限制解除
- 移除 if 块内禁止 let 变量绑定的限制
- 移除 # 预处理器限制
- 移除 regex 相关语法限制
- 预留扩展接口

## 向下兼容
- v0.7.3 全部旧代码 100% 可正常编译运行
- 新增语法为增量扩展，不修改任何已有语法的语义

## 项目文件变更
- `src/token.rs` — 新增 Null 关键字
- `src/ast.rs` — 新增 Expr::Null 变体
- `src/parser.rs` — 解析 null 字面量、增强 pub 前缀支持
- `src/codegen.rs` — Expr::Null 字节码生成、for 循环增强、impl 三写法注册
- `src/vm.rs` — math 标准库、fmt 格式化、Map/Array/String 增强、类型检查函数
- `src/bytecode_optimize.rs` — Expr::Null 匹配
- `src/formatter.rs` — Expr::Null 格式化
- `Cargo.toml` — 版本号 → 0.8.4
- `resources.rc` — 版本号 → 0.8.4
- `src/main.rs` — 命令行版本号 → v0.8.4
