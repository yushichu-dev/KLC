<<<<<<< HEAD
﻿# KLC 语言规范 v1.0.3-正式版

> **设计哲学**: "Let it flow" — 代码读起来应该像自然语言一样流畅
=======
# KLC 语言规范 v0.3.1-beta

> **设计哲学**: "Let it flow" — 代码读起来应该像自然语言一样流畅  
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20
> **核心三角**: 高性能 × 简洁易学 × 内存安全

---

## 0. 目录

1. [词汇约定](#1-词汇约定)
2. [变量与类型](#2-变量与类型)
<<<<<<< HEAD
3. [运算符](#3-运算符)
4. [函数](#4-函数)
5. [结构体与方法](#5-结构体与方法)
6. [枚举](#6-枚举)
7. [控制流](#7-控制流)
8. [模式匹配](#8-模式匹配)
9. [模块系统](#9-模块系统)
10. [标准库](#10-标准库)
11. [字节码优化](#11-字节码优化)
12. [CLI 工具](#12-cli-工具)
13. [附录](#附录)
=======
3. [函数](#3-函数)
4. [所有权系统](#4-所有权系统)
5. [结构体与方法](#5-结构体与方法)
6. [泛型](#6-泛型)
7. [控制流](#7-控制流)
8. [模式匹配](#8-模式匹配)
9. [并发模型](#9-并发模型)
10. [错误处理](#10-错误处理)
11. [模块系统](#11-模块系统)
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20

---

## 1. 词汇约定

### 1.1 注释
<<<<<<< HEAD
```klc
=======
```
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20
-- 单行注释
--- 文档注释（会生成文档）
--|
  多行注释
  支持嵌套
|--
```

### 1.2 标识符
<<<<<<< HEAD
- 字母或下划线开头，可含字母、数字、下划线
- 惯例: `snake_case` 用于变量/函数, `PascalCase` 用于类型

### 1.3 语句分隔
- **不使用分号**，以换行分隔语句
=======
- 字母开头，可含字母、数字、下划线
- 惯例: `snake_case` 用于变量/函数, `PascalCase` 用于类型
- 保留关键字见附录 A

### 1.3 语句分隔
- **不使用分号**，以换行分隔语句
- 允许尾随逗号
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20

---

## 2. 变量与类型

### 2.1 核心原则：默认不可变

```klc
-- 不可变变量（默认）
let name = "KLC"
<<<<<<< HEAD
let version = 0.8
=======
let version = 0.1
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20

-- 可变变量（显式标注 mut）
let mut counter = 0
counter = counter + 1

-- 带类型标注
let name: str = "KLC"
<<<<<<< HEAD
let mut count: i64 = 0
=======
let mut count: i32 = 0
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20
```

### 2.2 内建类型

| 类型     | 说明           | 示例                     |
|----------|----------------|-------------------------|
<<<<<<< HEAD
| `i64`    | 有符号整数（64位） | `let x = 42`          |
| `f64`    | 浮点数（64位）   | `let pi = 3.14`       |
| `String` | 字符串          | `let s = "hello"`      |
| `bool`    | 布尔值          | `let ok = true`        |
| `char`    | 字符(Unicode)   | `let c = 'A'`          |
| `null`    | 空值            | `let v = null`         |

> **注**: v1.0.3-正式版 中，整数默认为 `i64`，浮点默认为 `f64`。`i8`/`i16`/`i32`/`u*`/`f32` 关键字已保留但当前仅支持 `i64` 和 `f64`。
=======
| `i8` `i16` `i32` `i64` | 有符号整数 | `let x: i64 = 42`     |
| `u8` `u16` `u32` `u64` | 无符号整数 | `let x: u32 = 100`   |
| `f32` `f64`            | 浮点数      | `let pi: f64 = 3.14`  |
| `bool`                  | 布尔值      | `let ok = true`       |
| `char`                  | 字符(Unicode)| `let c = 'A'`        |
| `str`                   | 字符串(不可变) | `let s = "hello"`  |
| `any`                   | 动态类型    | `let x: any = ...`    |
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20

### 2.3 复合类型

```klc
<<<<<<< HEAD
-- 动态数组
let arr = [1, 2, 3, 4, 5]
arr.push(6)
let first = arr[0]
let len = arr.len()

-- 映射（Map）
let map = {"one": 1, "two": 2}
map.insert("three", 3)
let val = map["one"]

-- 结构体
type Point { x: f64, y: f64 }
let p = Point { x: 3.0, y: 4.0 }

-- 函数类型（lambda）
let callback = fn(x: i64) -> i64 = x * 2
=======
-- 数组（固定大小，栈分配）
let arr: [i32; 5] = [1, 2, 3, 4, 5]

-- 切片（动态大小，堆分配）
let list: [i32] = [1, 2, 3]
list.push(4)

-- 映射
let map: {str: i32} = {"one": 1, "two": 2}

-- 元组
let pair: (i32, str) = (42, "answer")
let (num, text) = pair  -- 解构

-- 函数类型
let callback: fn(i32, i32) -> i32 = add
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20
```

### 2.4 类型推断

<<<<<<< HEAD
KLC 拥有类型推断，大部分场景省略类型标注：

```klc
let x = 42          -- 推断为 i64
let y = 3.14        -- 推断为 f64
let s = "hello"     -- 推断为 String
let arr = [1, 2, 3] -- 推断为 Array
```

### 2.5 类型检查与转换

```klc
-- 类型检查
type_of(42)       -- 返回 "i64"
type_of(3.14)     -- 返回 "f64"
type_of("hello")  -- 返回 "String"
type_of(null)     -- 返回 "Null"
type_of([1,2])    -- 返回 "Array"

-- 空值检查
is_null(x)        -- 判断 x 是否为 null

-- 类型转换
int_of("123")     -- 字符串转整数
float_of("3.14")  -- 字符串转浮点
str_of(42)        -- 任意值转字符串
to_str(3.14)      -- 同 str_of
to_string(true)   -- 同 str_of
=======
KLC 拥有强大的类型推断，大部分场景省略类型标注：

```klc
let x = 42          -- 推断为 i32
let y = 3.14        -- 推断为 f64
let s = "hello"     -- 推断为 str
let list = [1, 2, 3]  -- 推断为 [i32]
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20
```

---

<<<<<<< HEAD
## 3. 运算符

### 3.1 算术运算

```klc
let a = 10 + 5    -- 加法
let b = 10 - 5    -- 减法
let c = 10 * 5    -- 乘法
let d = 10 / 3    -- 除法（整数除法 truncate）
let e = 10 % 3    -- 取模
let f = -a        -- 取负
```

### 3.2 比较运算

```klc
a == b            -- 相等
a != b            -- 不等
a < b             -- 小于
a > b             -- 大于
a <= b            -- 小于等于
a >= b            -- 大于等于
```

### 3.3 逻辑运算

```klc
a and b           -- 逻辑与
a or b            -- 逻辑或
not a             -- 逻辑非
!a                -- 逻辑非（等价写法）
```

### 3.4 字符串拼接

```klc
let greeting = "Hello, " ++ name
```

### 3.5 运算符优先级

| 优先级 | 运算符                        | 结合性 |
|--------|-------------------------------|--------|
| 14     | `.` `::`                      | 左     |
| 12     | `*` `/` `%`                   | 左     |
| 11     | `+` `-` `++`                  | 左     |
| 9      | `==` `!=` `<` `<=` `>` `>=`  | 左     |
| 7      | `and`                         | 左     |
| 6      | `or`                          | 左     |
| 4      | `=`                           | 右     |

---

## 4. 函数

### 4.1 函数定义
=======
## 3. 函数

### 3.1 函数定义
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20

```klc
-- 标准形式
fn greet(name: str) -> str {
    return "Hello, " ++ name
}

<<<<<<< HEAD
-- 无返回值（隐式返回 Null）
=======
-- 单表达式简写
fn add(a: i32, b: i32) -> i32 = a + b

-- 无返回值
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20
fn print_greeting(name: str) {
    io.println("Hi, " ++ name)
}

<<<<<<< HEAD
-- 短函数
fn add(a: i64, b: i64) -> i64 = a + b
```

### 4.2 main 入口

```klc
mod main

fn main() {
    io.println("Hello, KLC!")
}
```

### 4.3 匿名函数（Lambda / 闭包）

```klc
-- 赋值
let double = fn(x: i64) -> i64 = x * 2
io.println(double(5))   -- 输出: 10

-- 多行 lambda
let triple = fn(x: i64) -> i64 {
    return x * 3
}

-- 直接调用
let result = (fn(x: i64) -> i64 = x + 1)(10)  -- 11
```

### 4.4 递归

```klc
fn factorial(n: i64) -> i64 {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}
```

### 4.5 调用语法

```klc
-- 直接调用
let r = add(3, 4)

-- :: 命名空间风格（等价于 .）
let pi = math::pi()
let pi2 = math.pi()   -- 等价，均支持

-- 方法调用
let p = Point { x: 3.0, y: 4.0 }
let d = p.distance()
=======
-- 默认参数
fn power(base: i32, exp: i32 = 2) -> i32 {
    if exp == 0 { return 1 }
    return base * power(base, exp - 1)
}
```

### 3.2 函数即值

```klc
-- 函数赋值
let op: fn(i32, i32) -> i32 = add

-- 匿名函数（lambda）
let double = fn(x: i32) -> i32 = x * 2
let triple = fn(x: i32) -> i32 { return x * 3 }

-- 闭包
fn make_counter() -> fn() -> i32 {
    let mut count = 0
    return fn() -> i32 {
        count = count + 1
        return count
    }
}
```

### 3.3 管道操作符

```klc
-- |> 将左边结果传入右边函数作为第一个参数
let result = [1, 2, 3, 4, 5]
    |> filter(fn(x) -> bool = x % 2 == 0)
    |> map(fn(x) -> i32 = x * x)
    |> sum()
```

---

## 4. 所有权系统

KLC 的内存安全基于精炼的所有权模型，核心概念：

### 4.1 所有权转移 (`own`)

```klc
fn take_data(own value: Data) {
    -- value 获取了传入数据的所有权
    -- 函数结束时，value 被自动释放
    io.println("got: " ++ value.name)
}

fn main() {
    let data = Data { name: "important" }
    take_data(data)   -- 所有权转移给 take_data
    -- io.println(data.name)  -- 编译错误！data 已被移走
}
```

### 4.2 借用 (`borrow`)

```klc
fn read_data(borrow value: Data) {
    io.println(value.name)  -- 只读访问
    -- value.name = "new"   -- 编译错误！不可变借用
}

fn main() {
    let data = Data { name: "shared" }
    read_data(data)   -- 借用 data
    read_data(data)   -- 可以多次借用
    io.println(data.name)  -- 所有权还在，可以继续使用
}
```

### 4.3 可变借用 (`borrow mut`)

```klc
fn modify_data(borrow mut value: Data) {
    value.name = "modified"  -- 允许修改
}

fn main() {
    let mut data = Data { name: "original" }
    modify_data(data)  -- 可变借用
    -- 同一时间只能有一个可变借用
}
```

### 4.4 所有权规则汇总

| 操作        | 语法              | 特点                          |
|-------------|-------------------|-------------------------------|
| 移动所有权  | `own T`           | 转移后原变量失效               |
| 不可变借用  | `borrow T` / `&T` | 多个同时存在，只读              |
| 可变借用    | `borrow mut T`    | 独占，可修改，唯一的借用        |
| 拷贝        | `T`(不标 own)     | 对 `Copy` 类型自动复制         |

### 4.5 生命周期（Lifetime）自动推导

KLC 在绝大多数情况下自动推导生命周期，无需手动标注。极端复杂场景可使用 `'a` 语法：

```klc
-- 自动推导，无需标注
fn longest(borrow a: str, borrow b: str) -> borrow str {
    if a.len() > b.len() { return a }
    return b
}
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20
```

---

## 5. 结构体与方法

### 5.1 结构体定义

```klc
type Point {
    x: f64
    y: f64
}

type User {
<<<<<<< HEAD
    id: i64
    name: str
}

-- 支持 pub 可见性标记
pub type PublicType {
    field: i64
=======
    id: u64
    name: str
    email: Option[str]
    active: bool = true  -- 默认值
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20
}
```

### 5.2 创建实例

```klc
-- 按字段名
let p = Point { x: 3.0, y: 4.0 }

-- 简写（变量名 = 字段名）
let x = 3.0
let y = 4.0
let p = Point { x, y }
<<<<<<< HEAD
```

### 5.3 impl 方法块

```klc
impl Point {
    -- 关联函数（无 self）
    fn origin() -> Point = Point { x: 0.0, y: 0.0 }

    -- 实例方法（隐式 self）
    fn distance(self) -> f64 {
        return math::sqrt(self.x * self.x + self.y * self.y)
    }

    -- 可变方法
    fn translate(self, dx: f64, dy: f64) {
        self.x = self.x + dx
        self.y = self.y + dy
    }
}

-- 使用
let p1 = Point::origin()    -- :: 风格
let p2 = Point.origin()     -- . 风格等价
io.println(p1.distance())   -- 0.0
```

### 5.4 字段访问与修改

```klc
-- 读取字段
let name = user.name

-- 修改字段（变量需为 mut）
let mut user = User { id: 1, name: "Alice" }
user.name = "Bob"
=======

-- 带默认值
let user = User { id: 1, name: "Alice" }
-- email 默认 None, active 默认 true
```

### 5.3 方法和关联函数

```klc
impl Point {
    -- 关联函数（没有 self）
    fn origin() -> Point = Point { x: 0.0, y: 0.0 }

    -- 不可变方法（&self）
    fn distance(self) -> f64 {
        return math.sqrt(self.x * self.x + self.y * self.y)
    }

    -- 可变方法（&mut self）
    fn translate(self mut, dx: f64, dy: f64) {
        self.x = self.x + dx
        self.y = self.y + dy
    }

    -- 消费方法（own self）
    fn into_tuple(own self) -> (f64, f64) {
        return (self.x, self.y)
    }
}

-- 使用
let p = Point { x: 3.0, y: 4.0 }
io.println(p.distance())    -- 5.0
let d = p.distance()        -- 借用 p
let (x, y) = p.into_tuple() -- p 被消费
```

### 5.4 枚举（代数数据类型 / ADT）

```klc
type Option[T] {
    Some(T)
    None
}

type Result[T, E] {
    Ok(T)
    Err(E)
}

-- 自定义枚举
type Shape {
    Circle(f64)
    Rectangle(f64, f64)
    Triangle { a: f64, b: f64, c: f64 }
}
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20
```

---

<<<<<<< HEAD
## 6. 枚举

### 6.1 枚举定义

```klc
type Option {
    Some(i64)
    None
}

type Result {
    Ok(i64)
    Err(str)
}

-- 复杂枚举
type Shape {
    Circle(f64)                  -- 半径
    Rectangle(f64, f64)          -- 宽, 高
    Triangle { a: f64, b: f64, c: f64 }  -- 三边
}
```

### 6.2 枚举构造

```klc
let some_val = Some(42)
let none_val = None
let error = Err("something went wrong")
let circle = Circle(5.0)
```

### 6.3 枚举判别

```klc
-- 内置辅助函数
is_some(val)        -- 是否为 Some
is_none(val)        -- 是否为 None

-- 匹配枚举变体
match val {
    Some(n) => n
    None => 0
=======
## 6. 泛型

### 6.1 泛型函数

```klc
fn identity<T>(value: T) -> T = value

fn swap<T>(a: borrow mut T, b: borrow mut T) {
    let temp = *a
    *a = *b
    *b = temp
}
```

### 6.2 泛型结构体

```klc
type Pair<A, B> {
    first: A
    second: B
}

type Stack<T> {
    items: [T]
}

impl<T> Stack<T> {
    fn new() -> Stack<T> = Stack { items: [] }

    fn push(self mut, item: T) {
        self.items.push(item)
    }

    fn pop(self mut) -> Option<T> {
        if self.items.is_empty() {
            return None
        }
        return Some(self.items.pop())
    }
}
```

### 6.3 泛型约束（Trait / Interface）

```klc
-- 定义接口
trait Display {
    fn to_str(self) -> str
}

trait Comparable {
    fn compare(self, other: borrow Self) -> i32
}

-- 泛型约束
fn max<T: Comparable>(a: borrow T, b: borrow T) -> borrow T {
    if a.compare(b) > 0 { return a }
    return b
}

-- 为类型实现 trait
impl Display for Point {
    fn to_str(self) -> str = "Point(" ++ self.x.to_str() ++ ", " ++ self.y.to_str() ++ ")"
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20
}
```

---

## 7. 控制流

### 7.1 条件分支

```klc
<<<<<<< HEAD
-- if 作为语句
if score >= 90 {
    io.println("A")
} else if score >= 80 {
    io.println("B")
} else {
    io.println("C")
}

-- if 作为表达式（返回值）
let max = if a > b { a } else { b }

-- if/else 块内支持 let 绑定
let result = if 5 > 3 {
    let tmp = 10
    tmp + 1
} else {
    0
}
```

### 7.2 while 循环

```klc
=======
-- if 表达式的值是返回值
let grade = if score >= 90 {
    'A'
} else if score >= 80 {
    'B'
} else {
    'C'
}

-- 三元等价写法
let max = if a > b { a } else { b }
```

### 7.2 循环

```klc
-- while 循环
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20
let mut i = 0
while i < 10 {
    io.println(i)
    i = i + 1
}
<<<<<<< HEAD
```

### 7.3 for-in 循环（遍历数组）

```klc
let arr = [3, 1, 4, 1, 5]
for x in arr {
    io.println(x)
}
```

### 7.4 循环控制

```klc
loop {
    i = i + 1
    if i > 100 { break }
    if i % 2 == 0 { continue }
    io.println(i)
}
```

### 7.5 返回与退出

```klc
return value        -- 从函数返回
return              -- 返回 Null
exit(0)            -- 退出程序
=======

-- loop 无限循环（break 返回值）
let result = loop {
    i = i - 1
    if i == 0 {
        break "done!"
    }
}

-- for 遍历
for item in list {
    io.println(item)
}

-- 带索引的 for
for (index, item) in list.enumerate() {
    io.println(index.to_str() ++ ": " ++ item)
}

-- 范围遍历
for i in 0..10 {
    io.println(i)
}

-- 可包含终点
for i in 0..=10 {
    -- 0, 1, 2, ..., 10
}
```

### 7.3 短路操作符

```klc
-- 空值合并（类似 ?. 和 ??）
let name = user?.profile?.name ?? "anonymous"

-- 逻辑短路
if ptr != null and ptr.value > 0 {
    ...
}
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20
```

---

## 8. 模式匹配

### 8.1 match 表达式

```klc
match value {
    0 => "zero"
    1 | 2 => "one or two"
<<<<<<< HEAD
=======
    3..=10 => "between 3 and 10"
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20
    n => "other: " ++ n.to_str()
}
```

<<<<<<< HEAD
### 8.2 带守卫条件

```klc
match value {
    n if n < 0 => "negative"
    n if n > 0 => "positive"
    n => "zero"
}
```

### 8.3 枚举匹配
=======
### 8.2 枚举匹配
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20

```klc
type Command {
    Quit
<<<<<<< HEAD
    Move { x: i64, y: i64 }
    Write(str)
}

fn handle(cmd: Command) -> str {
=======
    Move { x: i32, y: i32 }
    Write(str)
    ChangeColor(u8, u8, u8)
}

fn handle(cmd: borrow Command) -> str {
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20
    return match cmd {
        Quit => "quitting"
        Move { x, y } => "move to (" ++ x.to_str() ++ ", " ++ y.to_str() ++ ")"
        Write(text) => "write: " ++ text
<<<<<<< HEAD
=======
        ChangeColor(r, g, b) => "change color"
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20
    }
}
```

<<<<<<< HEAD
---

## 9. 模块系统

### 9.1 模块声明

```klc
-- 文件: math_utils.klc
mod math_utils

pub fn add(a: i64, b: i64) -> i64 = a + b
fn helper() -> i64 = 0   -- 私有函数（无 pub）
```

### 9.2 导入
=======
### 8.3 守卫条件

```klc
match value {
    n if n < 0 => "negative"
    n if n == 0 => "zero"
    n if n > 0 and n < 100 => "positive small"
    n => "positive large"
}
```

### 8.4 let-else 模式

```klc
fn get_config(key: str) -> str {
    let Some(value) = config_map.get(key) else {
        return "default"
    }
    return value
}
```

---

## 9. 并发模型

### 9.1 任务（轻量级协程）

```klc
-- task 定义协程
task worker(id: i32, delay_ms: u64) {
    time.sleep(delay_ms)
    io.println("worker " ++ id.to_str() ++ " done")
}

fn main() {
    -- go 关键字启动协程
    let t1 = go worker(1, 100)
    let t2 = go worker(2, 200)
    let t3 = go worker(3, 50)

    -- 等待完成
    t1.wait()
    t2.wait()
    t3.wait()
}
```

### 9.2 通道（Channel）

```klc
fn main() {
    -- 创建通道
    let (sender, receiver) = channel::<i32>(16)

    -- 生产者任务
    let producer = go {
        for i in 1..=10 {
            sender <- i  -- 发送
        }
    }

    -- 消费者任务
    let consumer = go {
        loop {
            match <-receiver {  -- 接收
                Some(value) => io.println("got: " ++ value.to_str())
                None => break
            }
        }
    }

    producer.wait()
    consumer.wait()
}
```

### 9.3 async/await（语法糖）

```klc
-- async 函数
async fn fetch_data(url: str) -> Result[str, Error> {
    let response = await http.get(url)?
    return Ok(response.body)
}

async fn main() {
    -- 并行执行
    let (result1, result2) = join!(
        fetch_data("https://api.example.com/a"),
        fetch_data("https://api.example.com/b")
    )
}
```

---

## 10. 错误处理

### 10.1 Result 和 Option

```klc
-- 标准返回值
fn divide(a: i64, b: i64) -> Result[i64, str] {
    if b == 0 {
        return Err("division by zero")
    }
    return Ok(a / b)
}
```

### 10.2 ? 操作符（传播错误）

```klc
fn complex_calc(x: i64, y: i64) -> Result[i64, str] {
    let a = divide(x, y)?       -- 错误自动向上传播
    let b = divide(a, y)?       -- 同上
    let c = divide(b, y)?
    return Ok(c)
}
```

### 10.3 错误处理模式

```klc
-- match 处理
match divide(10, 0) {
    Ok(result) => io.println("result: " ++ result.to_str())
    Err(msg) => io.println("error: " ++ msg)
}

-- 提供默认值
let result = divide(10, 0).unwrap_or(0)

-- 短路
let result = divide(10, 2).expect("division should succeed")
```

---

## 11. 模块系统

### 11.1 模块定义

```klc
-- 文件: math.klc
mod math

pub fn add(a: i32, b: i32) -> i32 = a + b
pub fn sub(a: i32, b: i32) -> i32 = a - b

-- 私有函数（无 pub 前缀）
fn helper() { ... }
```

### 11.2 导入
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20

```klc
-- 文件: main.klc
mod main
<<<<<<< HEAD
use math_utils

fn main() {
    let r = math_utils.add(1, 2)
}
```

### 9.3 标准库导入

```klc
mod main
use io          -- IO 库
use math        -- 数学库
use mat         -- 矩阵库
```

---

## 10. 标准库

### 10.1 io — 输入输出 & 文件操作

```klc
-- 控制台输出
io.println("Hello")           -- 打印并换行
io.print("no newline")        -- 打印不换行
println("shortcut")           -- 无 io. 前缀也支持
print("shortcut")             -- 同上

-- 控制台输入
let input = read_line()

-- 文件操作
io.write("path.txt", "content")
io.append("path.txt", "more")
let content = io.read("path.txt")
let lines = io.read_lines("path.txt")    -- 返回字符串数组
let exists = io.exists("path.txt")       -- 返回 bool
io.delete("path.txt")
io.mkdir("dir_name")
let files = io.list_dir(".")             -- 返回文件名数组
let size = io.file_size("path.txt")
```

### 10.2 math — 数学库

```klc
use math

-- 常量
math.pi()            -- 3.141592653589793
math.e()             -- 2.718281828459045

-- 三角函数
math.sin(x)          -- 正弦
math.cos(x)          -- 余弦
math.tanh(x)         -- 双曲正切

-- 指数与对数
math.exp(x)          -- e^x
math.sqrt(x)         -- 平方根
math.log(x)          -- 自然对数 (ln)
math.ln(x)           -- 同上
math.log2(x)         -- 以2为底
math.log10(x)        -- 以10为底

-- 其他
math.abs(x)          -- 绝对值（整数/浮点）
math.min(a, b)       -- 最小值
math.max(a, b)       -- 最大值
math.pow(base, exp)  -- 幂运算
math.floor(x)        -- 向下取整
math.ceil(x)         -- 向上取整
math.round(x)        -- 四舍五入
```

### 10.3 fmt — 字符串格式化

```klc
-- 格式化说明符: %s %d %f %.Nf %x %b
fmt("Name: %s, Age: %d", "Alice", 30)
fmt("Pi = %.6f", 3.14159265)
fmt("Hex: 0x%x, Bin: %b", 255, 15)
format("Same as fmt: %s", "hello")   -- format 是 fmt 的别名
```

### 10.4 Array 方法

```klc
let arr = [3, 1, 4, 1, 5]

arr.len()            -- 获取长度
arr.push(9)          -- 尾部追加
arr.pop()            -- 尾部弹出
arr.contains(4)      -- 是否包含元素
arr.index_of(1)      -- 元素首次出现位置
arr.reverse()        -- 反转
arr.sort()           -- 排序
arr.join(", ")       -- 用分隔符连接为字符串
arr.is_empty()       -- 是否为空
arr.clear()          -- 清空

-- 索引访问
let first = arr[0]
arr[0] = 99
```

### 10.5 Map 方法

```klc
let m = {"a": 1, "b": 2}

m.len()              -- 大小
m.insert("c", 3)     -- 插入/更新（也可用 set）
m.remove("a")        -- 删除键
m.contains("b")      -- 是否包含键（也可用 has_key）
m.keys()             -- 返回所有键（字符串数组）
m.values()           -- 返回所有值（数组）
m.is_empty()         -- 是否为空
m.clear()            -- 清空

-- 索引访问
let v = m["b"]
m["c"] = 4
```

### 10.6 String 方法

```klc
let s = "  hello KLC  "

s.trim()             -- 去除首尾空格 → "hello KLC"
s.to_upper()         -- 转大写 → "  HELLO KLC  "
s.to_uppercase()     -- 同上
s.to_lower()         -- 转小写
s.starts_with("  ")  -- 是否以...开头
s.ends_with("  ")    -- 是否以...结尾
s.split(" ")         -- 分割为字符串数组
s.replace("KLC", "World")
s.len()              -- 字符串长度
s.char_at(0)         -- 指定位置的字符
s.chars()            -- 转为字符数组
```

### 10.7 mat — 矩阵标准库

```klc
use mat

-- 创建与基本操作
let m1 = mat::create(2, 3)           -- 创建 2×3 零矩阵
mat::set(m1, 0, 1, 5.0)             -- 设置元素 [0][1] = 5.0
let val = mat::get(m1, 0, 1)        -- 读取元素

-- 矩阵运算
let sum = mat::add(a, b)             -- 逐元素加法
let prod = mat::mul(a, b)            -- 标准矩阵乘法（单线程）
let prod_p = mat::parallel_mul(a, b) -- 13线程并行矩阵乘法
let scaled = mat::mul_scalar(m, 2.0) -- 标量乘法
let t = mat::transpose(m)            -- 转置
let dims = mat::shape(m)             -- 返回 [行数, 列数]

mat::print(m)                        -- 格式化打印矩阵
```

### 10.8 transformer — AI 推理引擎

```klc
-- 创建 Transformer 模型
let model = transformer::create(512, 8, 10000)
-- 参数: d_model(嵌入维度), heads(注意力头数), vocab_size(词汇量, 可选)

-- 训练
transformer::train_step(model, x_input, y_target, 0.01)
-- 参数: 模型, 输入矩阵, 目标矩阵, 学习率

-- 前向推理
let logits = transformer::forward(model, input_matrix)

-- 持久化
transformer::save(model, "model.klc_model")
let loaded = transformer::load("model.klc_model")

-- 调试
transformer::print(model)
```

---

## 11. 字节码优化

KLC v1.0.3-正式版 内置多层编译器优化，默认开启：

### 11.1 AST 层优化（每次 compile/run 自动执行）

| 优化项 | 说明 |
|--------|------|
| 常量折叠 | `1 + 2` 编译期求值为 `3` |
| 死代码删除 | 移除 `if false { ... }` 分支 |
| 运算内联 | `math::exp(0.0)` 编译期求值 |
| 循环简化 | 识别并优化简单循环模式 |
| 多 pass 收敛 | 优化迭代直到 AST 稳定 |

### 11.2 VM 执行层优化

| 优化项 | 说明 |
|--------|------|
| 固定容量栈 | `Box<[Value; 4096]>` 堆分配栈，零 realloc |
| 零 clone 指令 | 每次仅 clone 当前一条指令 |
| 数学内联 | math 函数编译为直接 f64 运算 |
| 内存池化 | StringPool 复用相同字符串 |

### 11.3 尾调用优化 (TCO)

编译器自动识别尾调用并转换为跳转而非入栈。

---

## 12. CLI 工具

### 12.1 运行脚本

```bash
klc <source.klc>            # 直接运行
klc run <source.klc>        # 同上
klc run --debug <source.klc> # 显示 Tokens/AST/Bytecode
```

### 12.2 构建与检查

```bash
klc check <source.klc>      # 语法检查（不执行）
klc build <source.klc>      # 构建项目
klc build --native <source> # 生成 Windows PE 可执行文件
klc build --no-opt          # 禁用优化
klc build -g                # 附带 DWARF 调试信息
klc build -o output.exe <source>  # 指定输出路径
```

### 12.3 代码格式化

```bash
klc fmt <file>              # 格式化文件
klc fmt --check <file>      # 仅检查不修改
klc fmt --indent 2 <file>   # 指定缩进宽度（默认4）
```

### 12.4 开发者工具

```bash
klc version                 # 查看版本
klc help                    # 帮助信息
klc --ide                   # 启动图形化 IDE
klc new <project>           # 创建新项目
```

=======

use math                  -- 导入整个模块
use math::{add, sub}     -- 选择性导入
use math as m            -- 别名

fn main() {
    let result = math.add(1, 2)
    let result = m.add(1, 2)
}
```

>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20
---

## 附录 A: 关键字列表

```
<<<<<<< HEAD
and       as        bool      break     char
continue  else      enum      exit      f64
fn        for       if        impl      in
i64       let       loop      match     mod
mut       not       null      or        pub
return    self      str       type      use
while
```

---

## 附录 B: 运算符优先级

| 优先级 | 运算符                        | 结合性 |
|--------|-------------------------------|--------|
| 14     | `.` `::` `[]`                 | 左     |
| 13     | `-`(取负) `!` `not`           | 右     |
| 12     | `*` `/` `%`                   | 左     |
| 11     | `+` `-` `++`                  | 左     |
| 9      | `==` `!=` `<` `<=` `>` `>=`  | 左     |
| 7      | `and`                         | 左     |
| 6      | `or`                          | 左     |
| 4      | `=`                           | 右     |

---

## 附录 C: 所有权语法（计划中，v0.8.x 仅解析）

v1.0.3-正式版 解析器可识别 `own`、`borrow`、`borrow mut` 关键字，但当前版本**不做所有权检查**，仅作为语法占位保留：

```klc
-- 当前可编译但不执行所有权检查
fn take_data(own value: Data) { ... }
fn read_data(borrow value: Data) { ... }
fn modify_data(borrow mut value: Data) { ... }
```

完整所有权系统计划在后续版本中实现。

---

## 附录 D: 泛型语法（计划中，v0.8.x 仅解析）

泛型参数 `<T>` 语法可被解析器接受，但不执行类型检查：

```klc
-- 当前可编译但无泛型语义
type Stack<T> { items: Array }
fn identity<T>(value: T) -> T = value
```

---

## 附录 E: 并发模型（计划中，v0.8.x 仅解析）

`task`、`go`、`channel`、`async`/`await` 关键字已保留，语法解析占位，语义尚未实现。

---

## 附录 F: 文件扩展名与编码

- 源文件扩展名: `.klc`
- 模型文件扩展名: `.klc_model`
- 编码: UTF-8

---

*KLC Language Specification v1.0.3-正式版 — 2026-05-29*
=======
and       any       as        async     await     
bool      borrow    break     char      const     
continue  else      enum      fn        f32       
f64       for       go        i16       i32       
i64       i8        if        impl      in        
let       loop      match     mod       mut       
not       or        own       pub       return    
self      str       task      trait     type      
u16       u32       u64       u8        use       
while     yield
```

## 附录 B: 操作符优先级

| 优先级 | 操作符                        | 结合性 |
|--------|-------------------------------|--------|
| 16     | `.` `?.`                      | 左     |
| 15     | `()` `[]`                     | 左     |
| 14     | `-`(取负) `!` `not`          | 右     |
| 13     | `as`                          | 左     |
| 12     | `*` `/` `%`                   | 左     |
| 11     | `+` `-` `++`                  | 左     |
| 10     | `<<` `>>`                     | 左     |
| 9      | `&`(按位)                     | 左     |
| 8      | `^`(按位异或)                 | 左     |
| 7      | `|`(按位)                    | 左     |
| 6      | `==` `!=` `<` `<=` `>` `>=`  | 左     |
| 5      | `and`                         | 左     |
| 4      | `or`                          | 左     |
| 3      | `..` `..=`                    | 左     |
| 2      | `=` `+=` `-=` `*=` `/=` 等   | 右     |
| 1      | `|>`                          | 左     |

---

*KLC Language Specification v0.3.1-beta — 草案，持续演进*
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20
