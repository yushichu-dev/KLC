# KLC 语言规范 v0.3.1-beta

> **设计哲学**: "Let it flow" — 代码读起来应该像自然语言一样流畅  
> **核心三角**: 高性能 × 简洁易学 × 内存安全

---

## 0. 目录

1. [词汇约定](#1-词汇约定)
2. [变量与类型](#2-变量与类型)
3. [函数](#3-函数)
4. [所有权系统](#4-所有权系统)
5. [结构体与方法](#5-结构体与方法)
6. [泛型](#6-泛型)
7. [控制流](#7-控制流)
8. [模式匹配](#8-模式匹配)
9. [并发模型](#9-并发模型)
10. [错误处理](#10-错误处理)
11. [模块系统](#11-模块系统)

---

## 1. 词汇约定

### 1.1 注释
```
-- 单行注释
--- 文档注释（会生成文档）
--|
  多行注释
  支持嵌套
|--
```

### 1.2 标识符
- 字母开头，可含字母、数字、下划线
- 惯例: `snake_case` 用于变量/函数, `PascalCase` 用于类型
- 保留关键字见附录 A

### 1.3 语句分隔
- **不使用分号**，以换行分隔语句
- 允许尾随逗号

---

## 2. 变量与类型

### 2.1 核心原则：默认不可变

```klc
-- 不可变变量（默认）
let name = "KLC"
let version = 0.1

-- 可变变量（显式标注 mut）
let mut counter = 0
counter = counter + 1

-- 带类型标注
let name: str = "KLC"
let mut count: i32 = 0
```

### 2.2 内建类型

| 类型     | 说明           | 示例                     |
|----------|----------------|-------------------------|
| `i8` `i16` `i32` `i64` | 有符号整数 | `let x: i64 = 42`     |
| `u8` `u16` `u32` `u64` | 无符号整数 | `let x: u32 = 100`   |
| `f32` `f64`            | 浮点数      | `let pi: f64 = 3.14`  |
| `bool`                  | 布尔值      | `let ok = true`       |
| `char`                  | 字符(Unicode)| `let c = 'A'`        |
| `str`                   | 字符串(不可变) | `let s = "hello"`  |
| `any`                   | 动态类型    | `let x: any = ...`    |

### 2.3 复合类型

```klc
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
```

### 2.4 类型推断

KLC 拥有强大的类型推断，大部分场景省略类型标注：

```klc
let x = 42          -- 推断为 i32
let y = 3.14        -- 推断为 f64
let s = "hello"     -- 推断为 str
let list = [1, 2, 3]  -- 推断为 [i32]
```

---

## 3. 函数

### 3.1 函数定义

```klc
-- 标准形式
fn greet(name: str) -> str {
    return "Hello, " ++ name
}

-- 单表达式简写
fn add(a: i32, b: i32) -> i32 = a + b

-- 无返回值
fn print_greeting(name: str) {
    io.println("Hi, " ++ name)
}

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
    id: u64
    name: str
    email: Option[str]
    active: bool = true  -- 默认值
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
```

---

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
}
```

---

## 7. 控制流

### 7.1 条件分支

```klc
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
let mut i = 0
while i < 10 {
    io.println(i)
    i = i + 1
}

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
```

---

## 8. 模式匹配

### 8.1 match 表达式

```klc
match value {
    0 => "zero"
    1 | 2 => "one or two"
    3..=10 => "between 3 and 10"
    n => "other: " ++ n.to_str()
}
```

### 8.2 枚举匹配

```klc
type Command {
    Quit
    Move { x: i32, y: i32 }
    Write(str)
    ChangeColor(u8, u8, u8)
}

fn handle(cmd: borrow Command) -> str {
    return match cmd {
        Quit => "quitting"
        Move { x, y } => "move to (" ++ x.to_str() ++ ", " ++ y.to_str() ++ ")"
        Write(text) => "write: " ++ text
        ChangeColor(r, g, b) => "change color"
    }
}
```

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

```klc
-- 文件: main.klc
mod main

use math                  -- 导入整个模块
use math::{add, sub}     -- 选择性导入
use math as m            -- 别名

fn main() {
    let result = math.add(1, 2)
    let result = m.add(1, 2)
}
```

---

## 附录 A: 关键字列表

```
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
