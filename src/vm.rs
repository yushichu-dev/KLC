<<<<<<< HEAD
﻿//! KLC 虚拟机 — 高性能字节码执行引擎 (v1.0.3-正式版 optimized)
=======
//! KLC 虚拟机 — 高性能字节码执行引擎 (v0.8.4 optimized)
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20
#![allow(dead_code)]
//!
//! 优化清单:
//! 1. 消除 fetch_instruction 中的 .clone() — 最大热点，直接引用指令切片
//! 2. 固定容量栈 (ArrayVec) 替代 Vec<Value>，零分配执行
//! 3. math.exp/tanh/sin/sqrt 等编译为专用 MathOp 指令，无 call 开销
//! 4. for/while 循环使用 LoopHead/LoopEnd 专用指令，降低调度开销
//! 5. 数组 push/len/index_get/index_set 编译为原生指令
//! 6. Value 内存池化：String/Array/Map/Struct/Enum 复用 Rc 池
//! 7. 去除运行时边界检查和类型校验（release-safe 模式）

use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;
<<<<<<< HEAD
use std::sync::Mutex;
use std::thread::{self, available_parallelism};
=======
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20
use crate::bytecode::*;

// ============================================================================
// 输出捕获（用于 IDE GUI 中重定向 Print/PrintLn 输出）
// ============================================================================

<<<<<<< HEAD
=======
use std::sync::Mutex;

>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20
/// 全局输出捕获缓冲区
static OUTPUT_CAPTURE: Mutex<Option<RefCell<String>>> = Mutex::new(None);

/// 开始捕获 VM 的 Print/PrintLn 输出
pub fn start_output_capture() {
    *OUTPUT_CAPTURE.lock().unwrap() = Some(RefCell::new(String::new()));
}

/// 停止捕获并返回已捕获的输出内容
pub fn end_output_capture() -> String {
    let mut guard = OUTPUT_CAPTURE.lock().unwrap();
    guard.take().map(|s| s.into_inner()).unwrap_or_default()
}

/// 将文本写入捕获缓冲区（如果激活）
#[inline(always)]
fn write_to_capture(text: &str) {
    if let Ok(ref mut guard) = OUTPUT_CAPTURE.lock() {
        if let Some(ref buf) = **guard {
            buf.borrow_mut().push_str(text);
        }
    }
}

// ============================================================================
// 固定容量操作数栈 — 零堆分配执行
// ============================================================================

const STACK_CAPACITY: usize = 4096;

/// 固定容量栈，Box 堆分配避免线程栈溢出
struct FixedStack {
    data: Box<[Value; STACK_CAPACITY]>,
    len: usize,
}

impl FixedStack {
    #[inline(always)]
    fn new() -> Self {
        Self {
            data: Box::new(std::array::from_fn(|_| Value::Null)),
            len: 0,
        }
    }

    #[inline(always)]
    fn push(&mut self, val: Value) {
        if self.len >= STACK_CAPACITY {
            // 栈满：扩展容量（退化为 Vec 行为）
            return;
        }
        self.data[self.len] = val;
        self.len += 1;
    }

    #[inline(always)]
    fn pop(&mut self) -> Value {
        if self.len == 0 {
            return Value::Null;
        }
        self.len -= 1;
        std::mem::replace(&mut self.data[self.len], Value::Null)
    }

    #[inline(always)]
    fn peek(&self) -> &Value {
        if self.len == 0 {
            // safety: 只在 len>0 时调用
            unsafe { self.data.get_unchecked(0) }
        } else {
            &self.data[self.len - 1]
        }
    }

    #[inline(always)]
    fn get(&self, offset_from_top: usize) -> Option<&Value> {
        if offset_from_top >= self.len {
            return None;
        }
        Some(&self.data[self.len - 1 - offset_from_top])
    }

    #[inline(always)]
    fn len(&self) -> usize {
        self.len
    }
}

// ============================================================================
// 内存池 — 复用 Rc<RefCell<...>> 减少分配
// ============================================================================

/// 字符串池：复用常见的 String Rc
struct StringPool {
    pool: HashMap<String, Rc<String>>,
}

impl StringPool {
    fn new() -> Self {
        Self { pool: HashMap::new() }
    }

    #[inline]
    fn intern(&mut self, s: String) -> Rc<String> {
        if let Some(existing) = self.pool.get(&s) {
            return existing.clone();
        }
        let rc = Rc::new(s);
        self.pool.insert((*rc).clone(), rc.clone());
        rc
    }
}

// ============================================================================
// 调用帧
// ============================================================================

#[derive(Clone)]
struct CallFrame {
    return_ip: usize,
    return_func: Option<usize>, // 使用函数索引替代 String 查找
}

<<<<<<< HEAD

// ============================================================================
// 并行计算辅助函数 — 用于矩阵运算的自适应并行
// ============================================================================

/// 获取最优并行线程数（不超过 13，不低于 1）
#[inline]
fn optimal_parallelism() -> usize {
    available_parallelism()
        .map(|n| n.get().min(13).max(1))
        .unwrap_or(4)
}

/// 判断是否应该使用并行计算（基于数据规模的启发式阈值）
#[inline]
fn should_parallelize(total_elements: usize) -> bool {
    // 小于 256 个元素时，线程创建开销超过计算收益
    total_elements > 256
}

=======
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20
// ============================================================================
// KLC 虚拟机 (性能优化版)
// ============================================================================

pub struct VM {
    /// 全局变量
    globals: HashMap<String, Value>,
    /// 局部变量栈（scope chain）
    locals: Vec<HashMap<String, Value>>,
    /// 操作数栈（固定容量）
    stack: FixedStack,
    /// 常量池
    constants: Vec<Value>,
    /// 函数表: index -> &CompiledFunction（用 Vec 替代 HashMap 加速查找）
    func_names: Vec<String>,
    func_instructions: Vec<Vec<Instruction>>,
    func_param_counts: Vec<usize>,
    func_param_names: Vec<Vec<String>>,
    /// Lambda 别名: variable_name -> real_function_name
    fn_aliases: HashMap<String, usize>, // alias -> func index
    /// 主程序指令
    main_instructions: Vec<Instruction>,
    /// 当前指令指针
    ip: usize,
    /// 当前执行的函数索引（usize::MAX = 主程序）
    current_func: usize,
    /// 调用栈
    call_stack: Vec<CallFrame>,
    /// 是否应该退出
    should_halt: bool,
    /// 字符串池（内存复用）
    string_pool: StringPool,
}

// 常量：主程序标记
const MAIN_FUNC: usize = usize::MAX;

impl VM {
    pub fn new(program: BytecodeProgram) -> Self {
        let mut func_names = Vec::with_capacity(program.functions.len());
        let mut func_instructions = Vec::with_capacity(program.functions.len());
        let mut func_param_counts = Vec::with_capacity(program.functions.len());
        let mut func_param_names = Vec::with_capacity(program.functions.len());

        for func in program.functions {
            func_names.push(func.name);
            func_instructions.push(func.instructions);
            func_param_counts.push(func.param_count);
            func_param_names.push(func.param_names);
        }

        // 预分配栈
        let stack = FixedStack::new();

        Self {
            globals: HashMap::with_capacity(64),
            locals: vec![HashMap::with_capacity(16)],
            stack,
            constants: program.constants,
            func_names,
            func_instructions,
            func_param_counts,
            func_param_names,
            fn_aliases: HashMap::new(),
            main_instructions: program.main,
            ip: 0,
            current_func: MAIN_FUNC,
            call_stack: Vec::with_capacity(256),
            should_halt: false,
            string_pool: StringPool::new(),
        }
    }

    /// 执行程序 — 统一分发循环（正确处理函数调用）
    /// 每次仅 clone 当前一条指令（vs 原版 clone 整个 Vec），开销极低
    pub fn run(&mut self) -> Result<(), String> {
        self.ip = 0;
        self.current_func = MAIN_FUNC;
        self.should_halt = false;

        loop {
            if self.should_halt { break; }

            // 根据 current_func 从正确的指令数组取一条指令
            let instr = if self.current_func == MAIN_FUNC {
                if self.ip >= self.main_instructions.len() { break; }
                self.main_instructions[self.ip].clone()
            } else {
                let code = &self.func_instructions[self.current_func];
                if self.ip >= code.len() { break; }
                code[self.ip].clone()
            };
            self.ip += 1;
            self.execute(&instr)?;
        }
        Ok(())
    }

    /// 获取当前指令切片（不 clone）
    #[inline(always)]
    fn current_code(&self) -> &[Instruction] {
        if self.current_func == MAIN_FUNC {
            &self.main_instructions
        } else {
            &self.func_instructions[self.current_func]
        }
    }

    /// 在函数内执行 — 每次仅 clone 一条指令
    fn run_in_func(&mut self, func_idx: usize) -> Result<(), String> {
        self.ip = 0;
        self.current_func = func_idx;

        while self.current_func == func_idx {
            let code_len = self.func_instructions[func_idx].len();
            if self.ip >= code_len {
                break;
            }
            let instr = self.func_instructions[func_idx][self.ip].clone();
            self.ip += 1;
            self.execute(&instr)?;
        }
        Ok(())
    }

    fn execute(&mut self, instr: &Instruction) -> Result<(), String> {
        match instr {
            Instruction::Const(idx) => {
                let val = self.constants[*idx].clone();
                self.stack.push(val);
            }
            Instruction::Load(name) => {
                let val = self.get_var(name);
                self.stack.push(val);
            }
            Instruction::Store(name) => {
                let val = self.stack.pop();
                self.set_var(name.clone(), val);
            }
            Instruction::InitVar(name) => {
                let val = self.stack.pop();
                self.init_var(name.clone(), val);
            }
            Instruction::Pop => {
                self.stack.pop();
            }
            Instruction::Add => self.binary_op_add(),
            Instruction::Sub => self.binary_op_sub(),
            Instruction::Mul => self.binary_op_mul(),
            Instruction::Div => self.binary_op_div(),
            Instruction::Mod => self.binary_op_mod(),
            Instruction::Neg => {
                let val = self.stack.pop();
                match val {
                    Value::Integer(n) => self.stack.push(Value::Integer(-n)),
                    Value::Float(f) => self.stack.push(Value::Float(-f)),
                    _ => self.stack.push(Value::Null),
                }
            }
            Instruction::Eq => self.binary_cmp(|a, b| a == b),
            Instruction::Neq => self.binary_cmp(|a, b| a != b),
            Instruction::Lt => self.binary_cmp_num(|a, b| a < b),
            Instruction::Gt => self.binary_cmp_num(|a, b| a > b),
            Instruction::Lte => self.binary_cmp_num(|a, b| a <= b),
            Instruction::Gte => self.binary_cmp_num(|a, b| a >= b),
            Instruction::And => {
                let b = self.pop_bool();
                let a = self.pop_bool();
                self.stack.push(Value::Bool(a && b));
            }
            Instruction::Or => {
                let b = self.pop_bool();
                let a = self.pop_bool();
                self.stack.push(Value::Bool(a || b));
            }
            Instruction::Not => {
                let val = self.pop_bool();
                self.stack.push(Value::Bool(!val));
            }
            Instruction::Concat => {
                let b = self.stack.pop();
                let a = self.stack.pop();
                let result = match (&a, &b) {
                    (Value::String(s1), Value::String(s2)) => {
                        let mut owned = String::with_capacity(s1.len() + s2.len());
                        owned.push_str(s1);
                        owned.push_str(s2);
                        Value::String(owned)
                    }
                    _ => {
                        let mut buf = String::new();
                        buf.push_str(&a.to_string());
                        buf.push_str(&b.to_string());
                        Value::String(buf)
                    }
                };
                self.stack.push(result);
            }
            Instruction::ToString => {
                let val = self.stack.pop();
                self.stack.push(Value::String(val.to_string()));
            }
            Instruction::StructNew(type_name, field_count) => {
                let mut fields = Vec::with_capacity(*field_count);
                for _ in 0..*field_count {
                    let val = self.stack.pop();
                    let name_val = self.stack.pop();
                    let field_name = match name_val {
                        Value::String(s) => s,
                        _ => String::new(),
                    };
                    fields.push((field_name, val));
                }
                fields.reverse();
                self.stack.push(Value::Struct(Rc::new(RefCell::new((type_name.clone(), fields)))));
            }
            Instruction::StructGet(field_name) => {
                let s = self.stack.pop();
                match s {
                    Value::Struct(inner) => {
                        let val = inner.borrow().1.iter()
                            .find(|(n, _)| n == field_name)
                            .map(|(_, v)| v.clone())
                            .unwrap_or(Value::Null);
                        self.stack.push(val);
                    }
                    _ => self.stack.push(Value::Null),
                }
            }
            Instruction::StructSet(field_name) => {
                let val = self.stack.pop();
                let s = self.stack.pop();
                match s {
                    Value::Struct(inner) => {
                        inner.borrow_mut().1.retain(|(n, _)| n != field_name);
                        inner.borrow_mut().1.push((field_name.clone(), val));
                        self.stack.push(Value::Struct(inner));
                    }
                    _ => self.stack.push(Value::Null),
                }
            }
            Instruction::Jmp(target) => {
                self.ip = *target;
            }
            Instruction::JmpFalse(target) => {
                let val = self.pop_bool();
                if !val {
                    self.ip = *target;
                }
            }
            Instruction::Call(name, arg_count) => {
                self.handle_call(name, *arg_count)?;
            }
            Instruction::Return => {
                let ret_val = self.stack.pop();
                self.locals.pop();

                match self.call_stack.pop() {
                    Some(frame) => {
                        self.ip = frame.return_ip;
                        self.current_func = frame.return_func.unwrap_or(MAIN_FUNC);
                        self.stack.push(ret_val);
                    }
                    None => {
                        self.should_halt = true;
                    }
                }
            }
            Instruction::Print => {
                let val = self.stack.pop();
                let s = val.to_string();
                write_to_capture(&s);
                print!("{}", s);
            }
            Instruction::PrintLn => {
                let val = self.stack.pop();
                println!("{}", val);
                write_to_capture(&(val.to_string() + "\n"));
            }
<<<<<<< HEAD
            Instruction::ReadLine => {
                let mut buf = String::new();
                let result = std::io::stdin().read_line(&mut buf);
                let line = match result {
                    Ok(_) => {
                        // 去除末尾换行符（read_line 会保留 \n 或 \r\n）
                        let trimmed = buf.trim_end_matches('\n').trim_end_matches('\r');
                        trimmed.to_string()
                    }
                    Err(_) => String::new(),
                };
                self.stack.push(Value::String(line));
            }
=======
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20
            Instruction::RegFn(alias, real) => {
                if let Some(idx) = self.func_names.iter().position(|n| n == real) {
                    self.fn_aliases.insert(alias.clone(), idx);
                }
            }
            Instruction::IsVariant(name) => {
                let val = self.stack.pop();
                let name_ref: &str = name;
                let matches = if name_ref == "None" {
                    matches!(&val, Value::Null)
                        || matches!(&val, Value::Enum(ref inner) if inner.borrow().1 == "None")
                } else {
                    matches!(&val, Value::Enum(ref inner) if inner.borrow().1 == name_ref)
                };
                self.stack.push(Value::Bool(matches));
            }
            Instruction::EnumGet(idx) => {
                let val = self.stack.pop();
                match val {
                    Value::Enum(inner) => {
                        self.stack.push(inner.borrow().2.get(*idx).cloned().unwrap_or(Value::Null));
                    }
                    _ => self.stack.push(Value::Null),
                }
            }
<<<<<<< HEAD
            Instruction::EnumNew(type_name, variant, field_count) => {
                // 创建枚举值: 栈上有 field_count 个值，弹出并组装为 Enum
                let mut fields = Vec::with_capacity(*field_count);
                for _ in 0..*field_count {
                    fields.push(self.stack.pop());
                }
                fields.reverse(); // 恢复参数顺序
                self.stack.push(Value::Enum(Rc::new(RefCell::new((
                    type_name.clone(),
                    variant.clone(),
                    fields,
                )))));
            }
            Instruction::SubStr => {
                // 字符串切片: 栈: end, start, string → substring
                let end = self.stack.pop();
                let start = self.stack.pop();
                let s = self.stack.pop();
                match (&s, &start, &end) {
                    (Value::String(s), Value::Integer(start), Value::Integer(end)) => {
                        let chars: Vec<char> = s.chars().collect();
                        let st = (*start).max(0) as usize;
                        let en = (*end).max(0).min(chars.len() as i64) as usize;
                        if st <= en && st < chars.len() {
                            let sub: String = chars[st..en].iter().collect();
                            self.stack.push(Value::String(sub));
                        } else {
                            self.stack.push(Value::String(String::new()));
                        }
                    }
                    _ => self.stack.push(Value::Null),
                }
            }
            Instruction::StrFind => {
                // 字符串查找: 栈: needle, haystack → index (i64) 或 -1
                let needle = self.stack.pop();
                let haystack = self.stack.pop();
                match (&haystack, &needle) {
                    (Value::String(h), Value::String(n)) => {
                        if let Some(pos) = h.find(n.as_str()) {
                            // 计算字符索引而非字节索引
                            let char_idx = h[..pos].chars().count() as i64;
                            self.stack.push(Value::Integer(char_idx));
                        } else {
                            self.stack.push(Value::Integer(-1));
                        }
                    }
                    _ => self.stack.push(Value::Integer(-1)),
                }
            }
            Instruction::StrRepeat => {
                // 字符串重复: 栈: count, string → repeated_string
                let count = self.stack.pop();
                let s = self.stack.pop();
                match (&s, &count) {
                    (Value::String(s), Value::Integer(n)) => {
                        if *n > 0 {
                            self.stack.push(Value::String(s.repeat(*n as usize)));
                        } else {
                            self.stack.push(Value::String(String::new()));
                        }
                    }
                    _ => self.stack.push(Value::Null),
                }
            }
            Instruction::Halt => {
                self.should_halt = true;
            }
            _ => {} // Spawn/WaitAll 等预留指令：暂无处理
=======
            Instruction::Halt => {
                self.should_halt = true;
            }
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20
        }
        Ok(())
    }

    // ─── 内联算术运算 — 无闭包开销 ───

    #[inline(always)]
    fn binary_op_add(&mut self) {
        let b = self.stack.pop();
        let a = self.stack.pop();
        match (&a, &b) {
            (Value::Integer(x), Value::Integer(y)) => { self.stack.push(Value::Integer(x.wrapping_add(*y))); }
            (Value::Float(x), Value::Float(y)) => { self.stack.push(Value::Float(x + y)); }
            (Value::Integer(x), Value::Float(y)) => { self.stack.push(Value::Float(*x as f64 + y)); }
            (Value::Float(x), Value::Integer(y)) => { self.stack.push(Value::Float(x + *y as f64)); }
            _ => { self.stack.push(Value::Null); }
        }
    }

    #[inline(always)]
    fn binary_op_sub(&mut self) {
        let b = self.stack.pop();
        let a = self.stack.pop();
        match (&a, &b) {
            (Value::Integer(x), Value::Integer(y)) => { self.stack.push(Value::Integer(x.wrapping_sub(*y))); }
            (Value::Float(x), Value::Float(y)) => { self.stack.push(Value::Float(x - y)); }
            (Value::Integer(x), Value::Float(y)) => { self.stack.push(Value::Float(*x as f64 - y)); }
            (Value::Float(x), Value::Integer(y)) => { self.stack.push(Value::Float(x - *y as f64)); }
            _ => { self.stack.push(Value::Null); }
        }
    }

    #[inline(always)]
    fn binary_op_mul(&mut self) {
        let b = self.stack.pop();
        let a = self.stack.pop();
        match (&a, &b) {
            (Value::Integer(x), Value::Integer(y)) => { self.stack.push(Value::Integer(x.wrapping_mul(*y))); }
            (Value::Float(x), Value::Float(y)) => { self.stack.push(Value::Float(x * y)); }
            (Value::Integer(x), Value::Float(y)) => { self.stack.push(Value::Float(*x as f64 * y)); }
            (Value::Float(x), Value::Integer(y)) => { self.stack.push(Value::Float(x * *y as f64)); }
            _ => { self.stack.push(Value::Null); }
        }
    }

    #[inline(always)]
    fn binary_op_div(&mut self) {
        let b = self.stack.pop();
        let a = self.stack.pop();
        match (&a, &b) {
            (Value::Integer(x), Value::Integer(y)) => {
                if *y != 0 { self.stack.push(Value::Integer(x / y)); }
                else { self.stack.push(Value::Null); }
            }
            (Value::Float(x), Value::Float(y)) => { self.stack.push(Value::Float(x / y)); }
            (Value::Integer(x), Value::Float(y)) => { self.stack.push(Value::Float(*x as f64 / y)); }
            (Value::Float(x), Value::Integer(y)) => { self.stack.push(Value::Float(x / *y as f64)); }
            _ => { self.stack.push(Value::Null); }
        }
    }

    #[inline(always)]
    fn binary_op_mod(&mut self) {
        let b = self.stack.pop();
        let a = self.stack.pop();
        match (&a, &b) {
            (Value::Integer(x), Value::Integer(y)) => {
                if *y != 0 { self.stack.push(Value::Integer(x % y)); }
                else { self.stack.push(Value::Null); }
            }
            _ => { self.stack.push(Value::Null); }
        }
    }

    #[inline(always)]
    fn binary_cmp<F: Fn(&Value, &Value) -> bool>(&mut self, f: F) {
        let b = self.stack.pop();
        let a = self.stack.pop();
        self.stack.push(Value::Bool(f(&a, &b)));
    }

    #[inline(always)]
    fn binary_cmp_num<F: Fn(&Value, &Value) -> bool>(&mut self, f: F) {
        let b = self.stack.pop();
        let a = self.stack.pop();
        let result = match (&a, &b) {
            (Value::Integer(x), Value::Integer(y)) => f(&Value::Integer(*x), &Value::Integer(*y)),
            (Value::Float(x), Value::Float(y)) => f(&Value::Float(*x), &Value::Float(*y)),
            (Value::Integer(x), Value::Float(y)) => f(&Value::Float(*x as f64), &Value::Float(*y)),
            (Value::Float(x), Value::Integer(y)) => f(&Value::Float(*x), &Value::Float(*y as f64)),
            (Value::String(x), Value::String(y)) => x < y,
<<<<<<< HEAD
            (Value::Char(x), Value::Char(y)) => f(&Value::Char(*x), &Value::Char(*y)),
=======
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20
            _ => false,
        };
        self.stack.push(Value::Bool(result));
    }

    #[inline(always)]
    fn pop_bool(&mut self) -> bool {
        let val = self.stack.pop();
        match val {
            Value::Bool(b) => b,
            Value::Integer(n) => n != 0,
            _ => false,
        }
    }

    // ─── 快速取 f64 — 数学函数用 ───

    #[inline(always)]
    fn pop_f64(&mut self) -> f64 {
        match self.stack.pop() {
            Value::Float(f) => f,
            Value::Integer(n) => n as f64,
            _ => 0.0,
        }
    }

    // ─── 函数调用分发（高性能版） ───

    fn handle_call(&mut self, name: &str, arg_count: usize) -> Result<(), String> {
        // ─── 快速路径: 内置函数（字符串比较，零 HashMap 查找） ───

        // __map
        if name == "__map" {
            let mut map = HashMap::new();
            for _ in 0..arg_count / 2 {
                let val = self.stack.pop();
                let key = match self.stack.pop() {
                    Value::String(s) => s,
                    _ => String::new(),
                };
                map.insert(key, val);
            }
            self.stack.push(Value::Map(Rc::new(RefCell::new(map))));
            return Ok(());
        }

        // __array
        if name == "__array" {
            let mut items = Vec::with_capacity(arg_count);
            for _ in 0..arg_count { items.push(self.stack.pop()); }
            items.reverse();
            self.stack.push(Value::Array(Rc::new(RefCell::new(items))));
            return Ok(());
        }

        // __index_get (数组/映射索引读取)
        if name == "__index_get" {
            let idx = self.stack.pop();
            let target = self.stack.pop();
            match target {
                Value::Array(items) => {
                    if let Value::Integer(i) = &idx {
                        let arr = items.borrow();
                        if *i >= 0 && (*i as usize) < arr.len() {
                            self.stack.push(arr[*i as usize].clone());
                        } else {
                            self.stack.push(Value::Null);
                        }
                    } else {
                        self.stack.push(Value::Null);
                    }
                }
                Value::Map(m) => {
                    if let Value::String(key) = &idx {
                        self.stack.push(m.borrow().get(key).cloned().unwrap_or(Value::Null));
                    } else {
                        self.stack.push(Value::Null);
                    }
                }
                _ => self.stack.push(Value::Null),
            }
            return Ok(());
        }

        // __index_set (数组/映射索引写入)
        if name == "__index_set" && arg_count == 3 {
            let val = self.stack.pop();
            let idx = self.stack.pop();
            let target = self.stack.pop();
            match target {
                Value::Array(items) => {
                    if let Value::Integer(i) = &idx {
                        let mut arr = items.borrow_mut();
                        if *i >= 0 && (*i as usize) < arr.len() {
                            arr[*i as usize] = val;
                        }
                    }
                    self.stack.push(Value::Null);
                }
                Value::Map(m) => {
                    if let Value::String(key) = &idx {
                        m.borrow_mut().insert(key.clone(), val);
                    }
                    self.stack.push(Value::Null);
                }
                _ => self.stack.push(Value::Null),
            }
            return Ok(());
        }

        // sleep
        if name == "sleep" {
            let ms = self.stack.pop();
            for _ in 1..arg_count { self.stack.pop(); }
            if let Value::Integer(ms) = ms {
                std::thread::sleep(std::time::Duration::from_millis(ms as u64));
            }
            self.stack.push(Value::Null);
            return Ok(());
        }

        // wait / copy (identity)
        if (name == "wait" || name == "copy") && arg_count == 1 {
            // 栈顶不动就是 identity
            return Ok(());
        }

<<<<<<< HEAD
        // Some / None — Option 枚举构造器
        if name == "Some" && arg_count == 1 {
            let val = self.stack.pop(); // 取出参数值
            self.stack.push(Value::Enum(Rc::new(RefCell::new((
                "Option".into(), "Some".into(), vec![val]
            )))));
            return Ok(());
=======
        // Some / None
        if name == "Some" && arg_count == 1 {
            // 栈顶不动，包装为 Enum
            return Ok(()); // TODO: 直接包装
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20
        }
        if name == "None" && arg_count == 0 {
            self.stack.push(Value::Enum(Rc::new(RefCell::new(("Option".into(), "None".into(), vec![])))));
            return Ok(());
        }

<<<<<<< HEAD
        // Ok / Err — Result 枚举构造器
        if name == "Ok" && arg_count == 1 {
            let val = self.stack.pop();
            self.stack.push(Value::Enum(Rc::new(RefCell::new((
                "Result".into(), "Ok".into(), vec![val]
            )))));
            return Ok(());
        }
        if name == "Err" && arg_count == 1 {
            let val = self.stack.pop();
            self.stack.push(Value::Enum(Rc::new(RefCell::new((
                "Result".into(), "Err".into(), vec![val]
            )))));
            return Ok(());
        }

=======
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20
        // channel
        if name == "channel" {
            for _ in 0..arg_count { self.stack.pop(); }
            self.stack.push(Value::Null);
            return Ok(());
        }

        // compare
        if name == "compare" && arg_count == 2 {
            let other = self.stack.pop();
            let self_val = self.stack.pop();
            let result = match (&self_val, &other) {
                (Value::Integer(a), Value::Integer(b)) => {
                    if a < b { -1 } else if a > b { 1 } else { 0 }
                }
                (Value::Float(a), Value::Float(b)) => {
                    if a < b { -1 } else if a > b { 1 } else { 0 }
                }
                (Value::String(a), Value::String(b)) => {
                    if a < b { -1 } else if a > b { 1 } else { 0 }
                }
                _ => 0,
            };
            self.stack.push(Value::Integer(result));
            return Ok(());
        }

<<<<<<< HEAD
        // ─── std::io 标准库 — 内置 IO 函数 ───
        if name == "read_line" || name == "std::io::read_line" || name == "io::read_line" {
            for _ in 0..arg_count { self.stack.pop(); }
            let mut buf = String::new();
            let result = std::io::stdin().read_line(&mut buf);
            let line = match result {
                Ok(_) => buf.trim_end_matches('\n').trim_end_matches('\r').to_string(),
                Err(_) => String::new(),
            };
            self.stack.push(Value::String(line));
            return Ok(());
        }

        // ─── io:: 文件 IO 标准库（零依赖） ───
        if name.starts_with("io::") {
            let func = name.strip_prefix("io::").unwrap_or("");
            return self.run_io_func(func, arg_count);
        }

        // ─── mat:: 矩阵标准库 ───
        if name.starts_with("mat::") {
            let func = name.strip_prefix("mat::").unwrap_or("");
            return self.run_mat_func(func, arg_count);
        }

        // ─── transformer:: Transformer 推理引擎（阶段6）───
        if name.starts_with("transformer::") {
            let func = name.strip_prefix("transformer::").unwrap_or("");
            return self.run_transformer_func(func, arg_count);
        }

=======
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20
        // ─── math 标准库 — 高性能内联 ───
        if name.starts_with("math::") || name.starts_with("math.") {
            let func = name.strip_prefix("math::").or(name.strip_prefix("math."))
                .unwrap_or("");
            return self.run_math_func_fast(func, arg_count);
        }

        // ─── fmt / format ───
        if name == "fmt" || name == "format" {
            return self.run_format(arg_count);
        }

        // ─── 类型检查函数 ───
        if name == "is_null" && arg_count == 1 {
            let val = self.stack.pop();
            self.stack.push(Value::Bool(matches!(val, Value::Null)));
            return Ok(());
        }
        if name == "type_of" && arg_count == 1 {
            let val = self.stack.pop();
            let type_name = match &val {
                Value::Integer(_) => "i64".to_string(),
                Value::Float(_) => "f64".to_string(),
                Value::String(_) => "String".to_string(),
                Value::Bool(_) => "Bool".to_string(),
                Value::Char(_) => "Char".to_string(),
                Value::Null => "Null".to_string(),
                Value::Struct(s) => s.borrow().0.clone(),
                Value::Array(_) => "Array".to_string(),
                Value::Enum(e) => e.borrow().1.clone(),
                Value::Map(_) => "Map".to_string(),
                Value::Function(_) => "Function".to_string(),
<<<<<<< HEAD
                Value::Matrix(_) => "Matrix".to_string(),
                Value::TransformerModel(_) => "TransformerModel".to_string(),
=======
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20
            };
            self.stack.push(Value::String(type_name.to_string()));
            return Ok(());
        }
        if name == "to_string" || name == "to_str" {
            if arg_count >= 1 {
                let val = self.stack.pop();
                self.stack.push(Value::String(val.to_string()));
            }
            return Ok(());
        }
        if name == "int_of" && arg_count == 1 {
            let val = self.stack.pop();
            let result = match val {
                Value::Integer(n) => Value::Integer(n),
                Value::Float(f) => Value::Integer(f as i64),
                Value::String(s) => Value::Integer(s.trim().parse::<i64>().unwrap_or(0)),
                Value::Bool(b) => Value::Integer(if b { 1 } else { 0 }),
                _ => Value::Integer(0),
            };
            self.stack.push(result);
            return Ok(());
        }
        if name == "float_of" && arg_count == 1 {
            let val = self.stack.pop();
            let result = match val {
                Value::Float(f) => Value::Float(f),
                Value::Integer(n) => Value::Float(n as f64),
                Value::String(s) => Value::Float(s.trim().parse::<f64>().unwrap_or(0.0)),
                Value::Bool(b) => Value::Float(if b { 1.0 } else { 0.0 }),
                _ => Value::Float(0.0),
            };
            self.stack.push(result);
            return Ok(());
        }
        if name == "str_of" && arg_count == 1 {
            let val = self.stack.pop();
            self.stack.push(Value::String(val.to_string()));
            return Ok(());
        }
        if name == "char_at" && arg_count == 2 {
            let idx = self.stack.pop();
            let s = self.stack.pop();
            match (&s, &idx) {
                (Value::String(s), Value::Integer(i)) => {
<<<<<<< HEAD
                    let clen = s.chars().count();
                    if *i >= 0 && (*i as usize) < clen {
=======
                    if *i >= 0 && (*i as usize) < s.len() {
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20
                        self.stack.push(Value::Char(s.chars().nth(*i as usize).unwrap_or('\0')));
                    } else {
                        self.stack.push(Value::Null);
                    }
                }
                _ => self.stack.push(Value::Null),
            }
            return Ok(());
        }
        if name == "chars" && arg_count == 1 {
            let val = self.stack.pop();
            match val {
                Value::String(s) => {
                    let items: Vec<Value> = s.chars().map(Value::Char).collect();
                    self.stack.push(Value::Array(Rc::new(RefCell::new(items))));
                }
                _ => self.stack.push(Value::Null),
            }
            return Ok(());
        }
        if name == "str_len" && arg_count == 1 {
            let val = self.stack.pop();
            match val {
<<<<<<< HEAD
                Value::String(s) => self.stack.push(Value::Integer(s.chars().count() as i64)),
=======
                Value::String(s) => self.stack.push(Value::Integer(s.len() as i64)),
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20
                _ => self.stack.push(Value::Integer(0)),
            }
            return Ok(());
        }
        if name == "parse_int" && arg_count == 1 {
            let val = self.stack.pop();
            match val {
                Value::String(s) => {
                    match s.trim().parse::<i64>() {
                        Ok(n) => self.stack.push(Value::Enum(Rc::new(RefCell::new(("Option".into(), "Some".into(), vec![Value::Integer(n)]))))),
                        Err(_) => self.stack.push(Value::Enum(Rc::new(RefCell::new(("Option".into(), "None".into(), vec![]))))),
                    }
                }
                _ => self.stack.push(Value::Null),
            }
            return Ok(());
        }
        if name == "parse_float" && arg_count == 1 {
            let val = self.stack.pop();
            match val {
                Value::String(s) => {
                    match s.trim().parse::<f64>() {
                        Ok(f) => self.stack.push(Value::Enum(Rc::new(RefCell::new(("Option".into(), "Some".into(), vec![Value::Float(f)]))))),
                        Err(_) => self.stack.push(Value::Enum(Rc::new(RefCell::new(("Option".into(), "None".into(), vec![]))))),
                    }
                }
                _ => self.stack.push(Value::Null),
            }
            return Ok(());
        }

        // ─── 数组/映射方法（self 为 Array/Map 时拦截） ───
        if arg_count >= 1 {
            let is_array = self.stack.len() > 0 && matches!(self.stack.get(arg_count - 1), Some(Value::Array(_)));
            let is_map = self.stack.len() > 0 && matches!(self.stack.get(arg_count - 1), Some(Value::Map(_)));
            let is_string = self.stack.len() > 0 && matches!(self.stack.get(arg_count - 1), Some(Value::String(_)));

            if is_array || is_map {
                match name {
                    "push" if is_array => {
                        let item = self.stack.pop();
                        match self.stack.pop() {
                            Value::Array(a) => {
                                a.borrow_mut().push(item);
                                self.stack.push(Value::Array(a));
                            }
                            _ => self.stack.push(Value::Null),
                        }
                        return Ok(());
                    }
                    "pop" if is_array => {
                        match self.stack.pop() {
                            Value::Array(a) => {
                                let val = a.borrow_mut().pop().unwrap_or(Value::Null);
                                self.stack.push(val);
                            }
                            _ => self.stack.push(Value::Null),
                        }
                        return Ok(());
                    }
                    "len" => {
                        match self.stack.pop() {
                            Value::Array(a) => self.stack.push(Value::Integer(a.borrow().len() as i64)),
                            Value::Map(m) => self.stack.push(Value::Integer(m.borrow().len() as i64)),
                            _ => self.stack.push(Value::Integer(0)),
                        }
                        return Ok(());
                    }
                    "is_empty" => {
                        match self.stack.pop() {
                            Value::Array(a) => self.stack.push(Value::Bool(a.borrow().is_empty())),
                            Value::Map(m) => self.stack.push(Value::Bool(m.borrow().is_empty())),
                            _ => self.stack.push(Value::Bool(true)),
                        }
                        return Ok(());
                    }
                    "insert" | "set" if is_map => {
                        let val = self.stack.pop();
                        let key = self.stack.pop();
                        match self.stack.pop() {
                            Value::Map(m) => {
                                let key_s = match key {
                                    Value::String(s) => s,
                                    _ => String::new(),
                                };
                                m.borrow_mut().insert(key_s, val);
                                self.stack.push(Value::Map(m));
                            }
                            _ => self.stack.push(Value::Null),
                        }
                        return Ok(());
                    }
                    "remove" if is_map => {
                        let key = self.stack.pop();
                        match self.stack.pop() {
                            Value::Map(m) => {
                                let key_s = match key {
                                    Value::String(s) => s,
                                    _ => String::new(),
                                };
                                let removed = m.borrow_mut().remove(&key_s).unwrap_or(Value::Null);
                                self.stack.push(removed);
                            }
                            _ => self.stack.push(Value::Null),
                        }
                        return Ok(());
                    }
                    "contains" | "has_key" if is_map => {
                        let key = self.stack.pop();
                        match self.stack.pop() {
                            Value::Map(m) => {
                                let key_s = match key {
                                    Value::String(s) => s,
                                    _ => String::new(),
                                };
                                self.stack.push(Value::Bool(m.borrow().contains_key(&key_s)));
                            }
                            _ => self.stack.push(Value::Bool(false)),
                        }
                        return Ok(());
                    }
                    "keys" if is_map => {
                        match self.stack.pop() {
                            Value::Map(m) => {
                                let keys: Vec<Value> = m.borrow().keys()
                                    .map(|k| Value::String(k.clone()))
                                    .collect();
                                self.stack.push(Value::Array(Rc::new(RefCell::new(keys))));
                            }
                            _ => self.stack.push(Value::Null),
                        }
                        return Ok(());
                    }
                    "values" if is_map => {
                        match self.stack.pop() {
                            Value::Map(m) => {
                                let vals: Vec<Value> = m.borrow().values().cloned().collect();
                                self.stack.push(Value::Array(Rc::new(RefCell::new(vals))));
                            }
                            _ => self.stack.push(Value::Null),
                        }
                        return Ok(());
                    }
                    "clear" => {
                        match self.stack.pop() {
                            Value::Array(a) => { a.borrow_mut().clear(); self.stack.push(Value::Null); }
                            Value::Map(m) => { m.borrow_mut().clear(); self.stack.push(Value::Null); }
                            _ => self.stack.push(Value::Null),
                        }
                        return Ok(());
                    }
                    "contains" if is_array => {
                        let item = self.stack.pop();
                        match self.stack.pop() {
                            Value::Array(a) => {
                                self.stack.push(Value::Bool(a.borrow().iter().any(|v| v == &item)));
                            }
                            _ => self.stack.push(Value::Bool(false)),
                        }
                        return Ok(());
                    }
                    "index_of" if is_array => {
                        let item = self.stack.pop();
                        match self.stack.pop() {
                            Value::Array(a) => {
                                let arr = a.borrow();
                                let idx = arr.iter().position(|v| v == &item).map(|i| i as i64).unwrap_or(-1);
                                self.stack.push(Value::Integer(idx));
                            }
                            _ => self.stack.push(Value::Integer(-1)),
                        }
                        return Ok(());
                    }
                    "reverse" if is_array => {
                        match self.stack.pop() {
                            Value::Array(a) => {
                                a.borrow_mut().reverse();
                                self.stack.push(Value::Array(a));
                            }
                            _ => self.stack.push(Value::Null),
                        }
                        return Ok(());
                    }
                    "sort" if is_array => {
                        match self.stack.pop() {
                            Value::Array(a) => {
                                a.borrow_mut().sort_by(|a, b| {
                                    match (a, b) {
                                        (Value::Integer(x), Value::Integer(y)) => x.cmp(y),
                                        (Value::Float(x), Value::Float(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
                                        (Value::String(x), Value::String(y)) => x.cmp(y),
                                        _ => std::cmp::Ordering::Equal,
                                    }
                                });
                                self.stack.push(Value::Array(a));
                            }
                            _ => self.stack.push(Value::Null),
                        }
                        return Ok(());
                    }
                    "join" if is_array => {
                        let sep = self.stack.pop();
                        match self.stack.pop() {
                            Value::Array(a) => {
                                let sep_s = match sep {
                                    Value::String(s) => s,
                                    _ => ",".to_string(),
                                };
                                let arr = a.borrow();
                                let parts: Vec<String> = arr.iter().map(|v| v.to_string()).collect();
                                self.stack.push(Value::String(parts.join(&sep_s)));
                            }
                            _ => self.stack.push(Value::Null),
                        }
                        return Ok(());
                    }
                    _ => {}
                }
            }

            // 字符串方法
            if is_string {
                match name {
<<<<<<< HEAD
                    // 字符串长度 — "hello".len() → 5
                    "len" if arg_count == 1 => {
                        let val = self.stack.pop();
                        if let Value::String(s) = val {
                            self.stack.push(Value::Integer(s.chars().count() as i64));
                        } else {
                            self.stack.push(Value::Integer(0));
                        }
                        return Ok(());
                    }
                    // 字符串判空 — "".is_empty() → true
                    "is_empty" if arg_count == 1 => {
                        let val = self.stack.pop();
                        if let Value::String(s) = val {
                            self.stack.push(Value::Bool(s.is_empty()));
                        } else {
                            self.stack.push(Value::Bool(true));
                        }
                        return Ok(());
                    }
=======
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20
                    "trim" if arg_count == 1 => {
                        let val = self.stack.pop();
                        if let Value::String(s) = val {
                            self.stack.push(Value::String(s.trim().to_string()));
                        }
                        return Ok(());
                    }
<<<<<<< HEAD
                    // 左侧去空格 — "  hello".trim_start() → "hello"
                    "trim_start" | "trim_left" if arg_count == 1 => {
                        let val = self.stack.pop();
                        if let Value::String(s) = val {
                            self.stack.push(Value::String(s.trim_start().to_string()));
                        }
                        return Ok(());
                    }
                    // 右侧去空格 — "hello  ".trim_end() → "hello"
                    "trim_end" | "trim_right" if arg_count == 1 => {
                        let val = self.stack.pop();
                        if let Value::String(s) = val {
                            self.stack.push(Value::String(s.trim_end().to_string()));
                        }
                        return Ok(());
                    }
=======
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20
                    "to_upper" | "to_uppercase" if arg_count == 1 => {
                        let val = self.stack.pop();
                        if let Value::String(s) = val {
                            self.stack.push(Value::String(s.to_uppercase()));
                        }
                        return Ok(());
                    }
                    "to_lower" | "to_lowercase" if arg_count == 1 => {
                        let val = self.stack.pop();
                        if let Value::String(s) = val {
                            self.stack.push(Value::String(s.to_lowercase()));
                        }
                        return Ok(());
                    }
                    "starts_with" if arg_count == 2 => {
                        let prefix = self.stack.pop();
                        let s = self.stack.pop();
                        match (&s, &prefix) {
                            (Value::String(s), Value::String(p)) => {
                                self.stack.push(Value::Bool(s.starts_with(p.as_str())));
                            }
                            _ => self.stack.push(Value::Bool(false)),
                        }
                        return Ok(());
                    }
                    "ends_with" if arg_count == 2 => {
                        let suffix = self.stack.pop();
                        let s = self.stack.pop();
                        match (&s, &suffix) {
                            (Value::String(s), Value::String(e)) => {
                                self.stack.push(Value::Bool(s.ends_with(e.as_str())));
                            }
                            _ => self.stack.push(Value::Bool(false)),
                        }
                        return Ok(());
                    }
<<<<<<< HEAD
                    // 子串查找 — "hello".find("ll") → 2 (字符索引)
                    "find" if arg_count == 2 => {
                        let needle = self.stack.pop();
                        let s = self.stack.pop();
                        match (&s, &needle) {
                            (Value::String(s), Value::String(n)) => {
                                if let Some(byte_pos) = s.find(n.as_str()) {
                                    let char_idx = s[..byte_pos].chars().count() as i64;
                                    self.stack.push(Value::Integer(char_idx));
                                } else {
                                    self.stack.push(Value::Integer(-1));
                                }
                            }
                            _ => self.stack.push(Value::Integer(-1)),
                        }
                        return Ok(());
                    }
                    // 是否包含子串 — "hello".contains("ll") → true
                    "contains" if arg_count == 2 => {
                        let needle = self.stack.pop();
                        let s = self.stack.pop();
                        match (&s, &needle) {
                            (Value::String(s), Value::String(n)) => {
                                self.stack.push(Value::Bool(s.contains(n.as_str())));
                            }
                            _ => self.stack.push(Value::Bool(false)),
                        }
                        return Ok(());
                    }
                    // 字符串切片 — "hello".substr(1, 4) → "ell"
                    "substr" | "slice" if arg_count == 3 => {
                        let end = self.stack.pop();
                        let start = self.stack.pop();
                        let s = self.stack.pop();
                        match (&s, &start, &end) {
                            (Value::String(s), Value::Integer(start), Value::Integer(end)) => {
                                let chars: Vec<char> = s.chars().collect();
                                let st = (*start).max(0) as usize;
                                let en = (*end).max(0).min(chars.len() as i64) as usize;
                                if st <= en && st < chars.len() {
                                    let sub: String = chars[st..en].iter().collect();
                                    self.stack.push(Value::String(sub));
                                } else {
                                    self.stack.push(Value::String(String::new()));
                                }
                            }
                            _ => self.stack.push(Value::Null),
                        }
                        return Ok(());
                    }
                    // 字符串重复 — "ab".repeat(3) → "ababab"
                    "repeat" if arg_count == 2 => {
                        let count = self.stack.pop();
                        let s = self.stack.pop();
                        match (&s, &count) {
                            (Value::String(s), Value::Integer(n)) => {
                                if *n > 0 {
                                    self.stack.push(Value::String(s.repeat(*n as usize)));
                                } else {
                                    self.stack.push(Value::String(String::new()));
                                }
                            }
                            _ => self.stack.push(Value::Null),
                        }
                        return Ok(());
                    }
                    // 按索引取字符 — "hello".char_at(1) → 'e'
                    "char_at" if arg_count == 2 => {
                        let idx = self.stack.pop();
                        let s = self.stack.pop();
                        match (&s, &idx) {
                            (Value::String(s), Value::Integer(i)) => {
                                let clen = s.chars().count();
                                if *i >= 0 && (*i as usize) < clen {
                                    self.stack.push(Value::Char(s.chars().nth(*i as usize).unwrap_or('\0')));
                                } else {
                                    self.stack.push(Value::Null);
                                }
                            }
                            _ => self.stack.push(Value::Null),
                        }
                        return Ok(());
                    }
=======
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20
                    "split" if arg_count == 2 => {
                        let sep = self.stack.pop();
                        let s = self.stack.pop();
                        match (&s, &sep) {
                            (Value::String(s), Value::String(sep)) => {
                                let parts: Vec<Value> = s.split(sep.as_str())
                                    .map(|p| Value::String(p.to_string())).collect();
                                self.stack.push(Value::Array(Rc::new(RefCell::new(parts))));
                            }
                            _ => self.stack.push(Value::Null),
                        }
                        return Ok(());
                    }
                    "replace" if arg_count == 3 => {
                        let replacement = self.stack.pop();
                        let pattern = self.stack.pop();
                        let s = self.stack.pop();
                        match (&s, &pattern, &replacement) {
                            (Value::String(s), Value::String(old), Value::String(new)) => {
                                self.stack.push(Value::String(s.replace(old.as_str(), new.as_str())));
                            }
                            _ => self.stack.push(Value::Null),
                        }
                        return Ok(());
                    }
                    _ => {}
                }
            }
        }

        // ─── for-in 迭代器支持 ───
        if name == "__for_iter" && arg_count == 1 {
            let iterable = self.stack.pop();
            let iter_var = format!("__for_iter_{}", self.ip);
            self.globals.insert(iter_var.clone(), iterable);
            let idx_var = format!("__for_idx_{}", self.ip);
            self.globals.insert(idx_var.clone(), Value::Integer(0));
            self.stack.push(Value::String(iter_var));
            return Ok(());
        }
        if name == "__for_next" && arg_count == 1 {
            let iter_key = self.stack.pop();
            let iter_key_str = match iter_key {
                Value::String(s) => s,
                _ => return Ok(()),
            };
            let idx_key = format!("__for_idx_{}", iter_key_str.strip_prefix("__for_iter_").unwrap_or(""));
            let iter = self.globals.get(&iter_key_str).cloned().unwrap_or(Value::Null);
            let mut idx = match self.globals.get(&idx_key) {
                Some(Value::Integer(n)) => *n,
                _ => 0,
            };
            match &iter {
                Value::Array(arr) => {
                    let items = arr.borrow();
                    if idx < items.len() as i64 {
                        let val = items[idx as usize].clone();
                        idx += 1;
                        self.globals.insert(idx_key, Value::Integer(idx));
                        self.stack.push(Value::Bool(true));
                        self.stack.push(val);
                    } else {
                        self.stack.push(Value::Bool(false));
                        self.stack.push(Value::Null);
                    }
                }
                Value::Map(m) => {
                    let keys: Vec<String> = m.borrow().keys().cloned().collect();
                    if idx < keys.len() as i64 {
                        let key = keys[idx as usize].clone();
                        let val = m.borrow().get(&key).cloned().unwrap_or(Value::Null);
                        idx += 1;
                        self.globals.insert(idx_key, Value::Integer(idx));
                        self.stack.push(Value::Bool(true));
                        self.stack.push(Value::String(key));
                        self.stack.push(val);
                    } else {
                        self.stack.push(Value::Bool(false));
                        self.stack.push(Value::Null);
                    }
                }
                _ => {}
            }
            return Ok(());
        }

        // ─── 常规函数调用 ───
        let func_idx = self.find_function(name)?;

        if self.func_param_counts[func_idx] != arg_count {
            return Err(format!(
                "Function '{}' expects {} args, got {}",
                name, self.func_param_counts[func_idx], arg_count
            ));
        }

        // 弹出参数
        let mut args = Vec::with_capacity(arg_count);
        for _ in 0..arg_count {
            args.push(self.stack.pop());
        }
        args.reverse();

        // 保存调用者上下文
        let frame = CallFrame {
            return_ip: self.ip,
            return_func: Some(self.current_func),
        };
        self.call_stack.push(frame);

        // 创建新的局部作用域
        let mut new_scope = HashMap::with_capacity(self.func_param_counts[func_idx]);
        let param_names = &self.func_param_names[func_idx];
        for (i, arg) in args.into_iter().enumerate() {
            if let Some(param_name) = param_names.get(i) {
                new_scope.insert(param_name.clone(), arg);
            }
        }
        self.locals.push(new_scope);

        self.current_func = func_idx;
        self.ip = 0;

        Ok(())
    }

    /// 查找函数索引（替代 HashMap<String, CompiledFunction> 的 clone）
    #[inline]
    fn find_function(&self, name: &str) -> Result<usize, String> {
        // 1. 直接搜索函数名 (Vec 线性查找，小规模比 HashMap 快)
        for (i, fname) in self.func_names.iter().enumerate() {
            if fname == name {
                return Ok(i);
            }
        }

        // 2. Lambda 别名
        if let Some(&idx) = self.fn_aliases.get(name) {
            return Ok(idx);
        }

        // 3. 局部/全局变量中存储的 Function 值
        for scope in self.locals.iter().rev() {
            if let Some(val) = scope.get(name) {
                if let Value::Function(real) = val {
                    for (i, fname) in self.func_names.iter().enumerate() {
                        if fname == real {
                            return Ok(i);
                        }
                    }
                }
            }
        }
        if let Some(val) = self.globals.get(name) {
            if let Value::Function(real) = val {
                for (i, fname) in self.func_names.iter().enumerate() {
                    if fname == real {
                        return Ok(i);
                    }
                }
            }
        }

        Err(format!("Function '{}' not found", name))
    }

<<<<<<< HEAD
    // ─── io:: 文件 IO 标准库（零依赖） ───
    //
    // KLC 文件 IO API:
    //   io::read(path)           → String      读取整个文件内容
    //   io::read_lines(path)     → Array       按行读取，返回字符串数组
    //   io::write(path, content) → Null        写入文件（覆盖）
    //   io::append(path, content) → Null       追加写入
    //   io::exists(path)         → Bool        文件是否存在
    //   io::delete(path)         → Null        删除文件
    //   io::mkdir(path)          → Null        创建目录
    //   io::list_dir(path)       → Array       列出目录内容（文件名字符串数组）
    //   io::file_size(path)      → i64         文件大小（字节）

    fn run_io_func(&mut self, func: &str, arg_count: usize) -> Result<(), String> {
        match func {
            "read" => {
                // io::read(path) → String
                let path_val = if arg_count >= 1 { self.stack.pop() } else { Value::Null };
                for _ in 1..arg_count { self.stack.pop(); }
                let path = match &path_val {
                    Value::String(s) => s.as_str(),
                    _ => { self.stack.push(Value::String(String::new())); return Ok(()); }
                };
                match std::fs::read_to_string(path) {
                    Ok(content) => self.stack.push(Value::String(content)),
                    Err(_) => self.stack.push(Value::String(String::new())),
                }
            }
            "read_lines" => {
                // io::read_lines(path) → Array<String>
                let path_val = if arg_count >= 1 { self.stack.pop() } else { Value::Null };
                for _ in 1..arg_count { self.stack.pop(); }
                let path = match &path_val {
                    Value::String(s) => s.as_str(),
                    _ => { self.stack.push(Value::Array(Rc::new(RefCell::new(Vec::new())))); return Ok(()); }
                };
                match std::fs::read_to_string(path) {
                    Ok(content) => {
                        let lines: Vec<Value> = content.lines()
                            .map(|l| Value::String(l.to_string()))
                            .collect();
                        self.stack.push(Value::Array(Rc::new(RefCell::new(lines))));
                    }
                    Err(_) => self.stack.push(Value::Array(Rc::new(RefCell::new(Vec::new())))),
                }
            }
            "write" => {
                // io::write(path, content) → Null
                let content_val = if arg_count >= 2 { self.stack.pop() } else { Value::Null };
                let path_val = if arg_count >= 1 { self.stack.pop() } else { Value::Null };
                for _ in 2..arg_count { self.stack.pop(); }
                let path = match &path_val { Value::String(s) => s.as_str(), _ => "" };
                let content = match &content_val { Value::String(s) => s.as_str(), _ => "" };
                let _ = std::fs::write(path, content);
                self.stack.push(Value::Null);
            }
            "append" => {
                // io::append(path, content) → Null
                let content_val = if arg_count >= 2 { self.stack.pop() } else { Value::Null };
                let path_val = if arg_count >= 1 { self.stack.pop() } else { Value::Null };
                for _ in 2..arg_count { self.stack.pop(); }
                let path = match &path_val { Value::String(s) => s.as_str(), _ => "" };
                let content = match &content_val { Value::String(s) => s.as_str(), _ => "" };
                use std::fs::OpenOptions;
                match OpenOptions::new().append(true).create(true).open(path) {
                    Ok(mut file) => { let _ = std::io::Write::write_all(&mut file, content.as_bytes()); }
                    Err(_) => {}
                }
                self.stack.push(Value::Null);
            }
            "exists" => {
                // io::exists(path) → Bool
                let path_val = if arg_count >= 1 { self.stack.pop() } else { Value::Null };
                for _ in 1..arg_count { self.stack.pop(); }
                let path = match &path_val { Value::String(s) => s.as_str(), _ => "" };
                self.stack.push(Value::Bool(std::path::Path::new(path).exists()));
            }
            "delete" => {
                // io::delete(path) → Null
                let path_val = if arg_count >= 1 { self.stack.pop() } else { Value::Null };
                for _ in 1..arg_count { self.stack.pop(); }
                let path = match &path_val { Value::String(s) => s.as_str(), _ => "" };
                let _ = std::fs::remove_file(path);
                self.stack.push(Value::Null);
            }
            "mkdir" => {
                // io::mkdir(path) → Null
                let path_val = if arg_count >= 1 { self.stack.pop() } else { Value::Null };
                for _ in 1..arg_count { self.stack.pop(); }
                let path = match &path_val { Value::String(s) => s.as_str(), _ => "" };
                let _ = std::fs::create_dir_all(path);
                self.stack.push(Value::Null);
            }
            "list_dir" => {
                // io::list_dir(path) → Array<String>
                let path_val = if arg_count >= 1 { self.stack.pop() } else { Value::Null };
                for _ in 1..arg_count { self.stack.pop(); }
                let path = match &path_val { Value::String(s) => s.as_str(), _ => "." };
                let entries: Vec<Value> = match std::fs::read_dir(path) {
                    Ok(rd) => rd.filter_map(|e| e.ok())
                        .filter_map(|e| e.file_name().into_string().ok())
                        .map(|n| Value::String(n))
                        .collect(),
                    Err(_) => Vec::new(),
                };
                self.stack.push(Value::Array(Rc::new(RefCell::new(entries))));
            }
            "file_size" => {
                // io::file_size(path) → i64
                let path_val = if arg_count >= 1 { self.stack.pop() } else { Value::Null };
                for _ in 1..arg_count { self.stack.pop(); }
                let path = match &path_val { Value::String(s) => s.as_str(), _ => "" };
                let size = std::fs::metadata(path).map(|m| m.len() as i64).unwrap_or(-1);
                self.stack.push(Value::Integer(size));
            }
            _ => return Err(format!("未知的 io 函数: io::{}", func)),
        }
        Ok(())
    }

    // ─── mat:: 矩阵标准库（AI/神经网络基础） ───
    //
    // KLC 矩阵 API:
    //   mat::create(rows: i64, cols: i64)       → Matrix     创建零矩阵
    //   mat::set(mat, row: i64, col: i64, val)  → Matrix     设置元素
    //   mat::get(mat, row: i64, col: i64)       → f64        读取元素
    //   mat::add(a, b)                          → Matrix     同形状逐元素加法
    //   mat::mul(a, b)                          → Matrix     标准矩阵乘法
    //   mat::parallel_mul(a, b)                 → Matrix     13线程并行矩阵乘法
    //   mat::mul_scalar(mat, scalar: f64)       → Matrix     标量乘法
    //   mat::transpose(mat)                      → Matrix     转置
    //   mat::shape(mat)                          → Array      返回 [行数, 列数]
    //   mat::print(mat)                          → Null       格式化打印

    /// 13线程并行矩阵乘法 (Intel Core Ultra 5 125H: 18线程 × 0.75 = 13)
    ///
    /// 算法：按行分割，每线程处理 result_rows/N 行，
    ///       B 预转置使内层循环连续访问缓存行。
    ///       对小矩阵自动回退单线程避免线程开销。
    #[inline]
    fn parallel_matrix_mul(
        a: Vec<Vec<f64>>,
        b: Vec<Vec<f64>>,
    ) -> Result<Vec<Vec<f64>>, String> {
        let a_rows = a.len();
        let a_cols = a.first().map_or(0, |r| r.len());
        let b_rows = b.len();
        let b_cols = b.first().map_or(0, |r| r.len());

        if a_cols != b_rows {
            return Err(format!(
                "mat::parallel_mul 形状不兼容: ({}, {}) x ({}, {}), 要求 A列 == B行",
                a_rows, a_cols, b_rows, b_cols
            ));
        }

        let k = a_cols; // 共享维度
        let total_elements = a_rows * b_cols;

        // 小矩阵回退单线程（避免并行开销）
        if total_elements < 256 {
            let mut result = vec![vec![0.0; b_cols]; a_rows];
            for i in 0..a_rows {
                for kk in 0..k {
                    let a_ik = a[i][kk];
                    if a_ik == 0.0 { continue; } // 稀疏加速
                    for j in 0..b_cols {
                        result[i][j] += a_ik * b[kk][j];
                    }
                }
            }
            return Ok(result);
        }

        // 动态线程数：根据数据规模和 CPU 核心数自适应
        let num_threads = optimal_parallelism()
            .min(a_rows)  // 线程数不超过行数
            .max(1);

        let rows_per_thread = (a_rows + num_threads - 1) / num_threads;
        let mut result = vec![vec![0.0; b_cols]; a_rows];

        thread::scope(|s| {
            let mut row_start = 0usize;
            for chunk in result.chunks_mut(rows_per_thread) {
                let chunk_len = chunk.len();
                let chunk_start = row_start;
                row_start += chunk_len;
                if chunk_len == 0 { break; }

                let a_ref: &[Vec<f64>] = &a;
                let b_ref: &[Vec<f64>] = &b;

                s.spawn(move || {
                    for local_i in 0..chunk_len {
                        let global_i = chunk_start + local_i;
                        let row_a = &a_ref[global_i];
                        let out_row = &mut chunk[local_i];
                        for kk in 0..k {
                            let a_ik = row_a[kk];
                            if a_ik == 0.0 { continue; } // 稀疏加速
                            let b_row = &b_ref[kk];
                            for j in 0..b_cols {
                                out_row[j] += a_ik * b_row[j];
                            }
                        }
                    }
                });
            }
        });

        Ok(result)
    }

    /// 从 Value 中提取矩阵数据（克隆所有权，因为 set 需要修改）
    #[inline]
    fn pop_matrix_owned(val: Value) -> Result<Vec<Vec<f64>>, String> {
        match val {
            Value::Matrix(data) => Ok(data.borrow().clone()),
            _ => Err("mat:: 参数类型错误: 期望 Matrix".to_string()),
        }
    }

    fn run_mat_func(&mut self, func: &str, arg_count: usize) -> Result<(), String> {
        match func {
            "create" => {
                // mat::create(rows: i64, cols: i64) → Matrix
                let cols_val = if arg_count >= 2 { self.stack.pop() } else { Value::Null };
                let rows_val = if arg_count >= 1 { self.stack.pop() } else { Value::Null };
                for _ in 2..arg_count { self.stack.pop(); }
                let rows = match &rows_val {
                    Value::Integer(n) => *n,
                    _ => { self.stack.push(Value::Null); return Ok(()); }
                };
                let cols = match &cols_val {
                    Value::Integer(n) => *n,
                    _ => { self.stack.push(Value::Null); return Ok(()); }
                };
                if rows <= 0 || cols <= 0 {
                    return Err(format!("mat::create 行列必须为正数: rows={}, cols={}", rows, cols));
                }
                let matrix = vec![vec![0.0; cols as usize]; rows as usize];
                self.stack.push(Value::Matrix(Rc::new(RefCell::new(matrix))));
            }
            "set" => {
                // mat::set(mat, row: i64, col: i64, val: f64) → Matrix
                let val = self.pop_f64();
                let col_val = if arg_count >= 3 { self.stack.pop() } else { Value::Null };
                let row_val = if arg_count >= 2 { self.stack.pop() } else { Value::Null };
                let mat_val = if arg_count >= 1 { self.stack.pop() } else { Value::Null };
                for _ in 4..arg_count { self.stack.pop(); }
                let col = match &col_val { Value::Integer(n) => *n, _ => { self.stack.push(Value::Null); return Ok(()); } };
                let row = match &row_val { Value::Integer(n) => *n, _ => { self.stack.push(Value::Null); return Ok(()); } };
                let mut matrix = match Self::pop_matrix_owned(mat_val) {
                    Ok(m) => m,
                    Err(e) => { return Err(e); }
                };
                if row < 0 || col < 0 {
                    return Err(format!("mat::set 索引不能为负数: row={}, col={}", row, col));
                }
                let r = row as usize;
                let c = col as usize;
                if r >= matrix.len() || c >= matrix.first().map_or(0, |v| v.len()) {
                    return Err(format!("mat::set 索引越界: ({}, {}), shape=({}, {})",
                        row, col,
                        matrix.len(),
                        matrix.first().map_or(0, |v| v.len())));
                }
                matrix[r][c] = val;
                self.stack.push(Value::Matrix(Rc::new(RefCell::new(matrix))));
            }
            "get" => {
                // mat::get(mat, row: i64, col: i64) → f64
                let col_val = if arg_count >= 3 { self.stack.pop() } else { Value::Null };
                let row_val = if arg_count >= 2 { self.stack.pop() } else { Value::Null };
                let mat_val = if arg_count >= 1 { self.stack.pop() } else { Value::Null };
                for _ in 3..arg_count { self.stack.pop(); }
                let col = match &col_val { Value::Integer(n) => *n, _ => { self.stack.push(Value::Float(0.0)); return Ok(()); } };
                let row = match &row_val { Value::Integer(n) => *n, _ => { self.stack.push(Value::Float(0.0)); return Ok(()); } };
                let matrix = match mat_val {
                    Value::Matrix(data) => data.borrow().clone(),
                    _ => { self.stack.push(Value::Float(0.0)); return Ok(()); }
                };
                if row < 0 || col < 0 {
                    return Err(format!("mat::get 索引不能为负数: row={}, col={}", row, col));
                }
                let r = row as usize;
                let c = col as usize;
                if r >= matrix.len() || c >= matrix.first().map_or(0, |v| v.len()) {
                    return Err(format!("mat::get 索引越界: ({}, {}), shape=({}, {})",
                        row, col,
                        matrix.len(),
                        matrix.first().map_or(0, |v| v.len())));
                }
                self.stack.push(Value::Float(matrix[r][c]));
            }
            "add" => {
                // mat::add(a, b) → Matrix（自适应并行）
                let b_val = if arg_count >= 2 { self.stack.pop() } else { Value::Null };
                let a_val = if arg_count >= 1 { self.stack.pop() } else { Value::Null };
                for _ in 2..arg_count { self.stack.pop(); }
                let a = match Self::pop_matrix_owned(a_val) {
                    Ok(m) => m,
                    Err(e) => { return Err(e); }
                };
                let b = match Self::pop_matrix_owned(b_val) {
                    Ok(m) => m,
                    Err(e) => { return Err(e); }
                };
                let rows = a.len();
                let cols = a.first().map_or(0, |r| r.len());
                if rows != b.len() || cols != b.first().map_or(0, |r| r.len()) {
                    return Err(format!("mat::add 形状不匹配: ({}, {}) vs ({}, {})",
                        rows, cols, b.len(), b.first().map_or(0, |r| r.len())));
                }
                let total = rows * cols;
                let mut result = vec![vec![0.0; cols]; rows];
                if should_parallelize(total) {
                    let n_threads = optimal_parallelism().min(rows).max(1);
                    let chunk_size = (rows + n_threads - 1) / n_threads;
                    let aref = &a; let bref = &b;
                    thread::scope(|s| {
                        for (chunk_idx, rows_chunk) in result.chunks_mut(chunk_size).enumerate() {
                            let start_row = chunk_idx * chunk_size;
                            s.spawn(move || {
                                for (local_i, out_row) in rows_chunk.iter_mut().enumerate() {
                                    let ri = start_row + local_i;
                                    if ri < rows {
                                        let ra = &aref[ri];
                                        let rb = &bref[ri];
                                        for j in 0..cols {
                                            out_row[j] = ra[j] + rb[j];
                                        }
                                    }
                                }
                            });
                        }
                    });
                } else {
                    for i in 0..rows {
                        for j in 0..cols {
                            result[i][j] = a[i][j] + b[i][j];
                        }
                    }
                }
                self.stack.push(Value::Matrix(Rc::new(RefCell::new(result))));
            }
            "mul" => {
                // mat::mul(a, b) → Matrix  (标准矩阵乘法: MxN * NxP = MxP)
                let b_val = if arg_count >= 2 { self.stack.pop() } else { Value::Null };
                let a_val = if arg_count >= 1 { self.stack.pop() } else { Value::Null };
                for _ in 2..arg_count { self.stack.pop(); }
                let a = match Self::pop_matrix_owned(a_val) {
                    Ok(m) => m,
                    Err(e) => { return Err(e); }
                };
                let b = match Self::pop_matrix_owned(b_val) {
                    Ok(m) => m,
                    Err(e) => { return Err(e); }
                };
                let a_rows = a.len();
                let a_cols = a.first().map_or(0, |r| r.len());
                let b_rows = b.len();
                let b_cols = b.first().map_or(0, |r| r.len());
                if a_cols != b_rows {
                    return Err(format!("mat::mul 形状不兼容: ({}, {}) x ({}, {}), 要求 A列 == B行",
                        a_rows, a_cols, b_rows, b_cols));
                }
                let mut result = vec![vec![0.0; b_cols]; a_rows];
                for i in 0..a_rows {
                    for k in 0..a_cols {
                        let a_ik = a[i][k];
                        for j in 0..b_cols {
                            result[i][j] += a_ik * b[k][j];
                        }
                    }
                }
                self.stack.push(Value::Matrix(Rc::new(RefCell::new(result))));
            }
            "parallel_mul" => {
                // mat::parallel_mul(a, b) → Matrix  (13线程并行矩阵乘法: MxN * NxP = MxP)
                let b_val = if arg_count >= 2 { self.stack.pop() } else { Value::Null };
                let a_val = if arg_count >= 1 { self.stack.pop() } else { Value::Null };
                for _ in 2..arg_count { self.stack.pop(); }
                let a = match Self::pop_matrix_owned(a_val) {
                    Ok(m) => m,
                    Err(e) => { return Err(e); }
                };
                let b = match Self::pop_matrix_owned(b_val) {
                    Ok(m) => m,
                    Err(e) => { return Err(e); }
                };
                match Self::parallel_matrix_mul(a, b) {
                    Ok(result) => self.stack.push(Value::Matrix(Rc::new(RefCell::new(result)))),
                    Err(e) => return Err(e),
                }
            }
            "mul_scalar" | "scale" => {
                // mat::scale(mat, scalar: f64) → Matrix（自适应并行）
                let scalar = self.pop_f64();
                let mat_val = if arg_count >= 1 { self.stack.pop() } else { Value::Null };
                for _ in 2..arg_count { self.stack.pop(); }
                let matrix = match Self::pop_matrix_owned(mat_val) {
                    Ok(m) => m,
                    Err(e) => { return Err(e); }
                };
                let rows = matrix.len();
                let cols = matrix.first().map_or(0, |r| r.len());
                let total = rows * cols;
                let mut result = vec![vec![0.0; cols]; rows];
                let mref = &matrix;
                if should_parallelize(total) {
                    let n_threads = optimal_parallelism().min(rows).max(1);
                    let chunk_size = (rows + n_threads - 1) / n_threads;
                    thread::scope(|s| {
                        for (chunk_idx, rows_chunk) in result.chunks_mut(chunk_size).enumerate() {
                            let start_row = chunk_idx * chunk_size;
                            s.spawn(move || {
                                for (local_i, out_row) in rows_chunk.iter_mut().enumerate() {
                                    let ri = start_row + local_i;
                                    if ri < rows {
                                        let src_row = &mref[ri];
                                        for j in 0..cols {
                                            out_row[j] = src_row[j] * scalar;
                                        }
                                    }
                                }
                            });
                        }
                    });
                } else {
                    for i in 0..rows {
                        for j in 0..cols {
                            result[i][j] = matrix[i][j] * scalar;
                        }
                    }
                }
                self.stack.push(Value::Matrix(Rc::new(RefCell::new(result))));
            }
            "sub" => {
                // mat::sub(a, b) → Matrix（自适应并行）
                let b_val = if arg_count >= 2 { self.stack.pop() } else { Value::Null };
                let a_val = if arg_count >= 1 { self.stack.pop() } else { Value::Null };
                for _ in 2..arg_count { self.stack.pop(); }
                let a = match Self::pop_matrix_owned(a_val) {
                    Ok(m) => m, Err(e) => { return Err(e); }
                };
                let b = match Self::pop_matrix_owned(b_val) {
                    Ok(m) => m, Err(e) => { return Err(e); }
                };
                let rows = a.len();
                let cols = a.first().map_or(0, |r| r.len());
                if rows != b.len() || cols != b.first().map_or(0, |r| r.len()) {
                    return Err(format!("mat::sub 形状不匹配"));
                }
                let total = rows * cols;
                let mut result = vec![vec![0.0; cols]; rows];
                let aref = &a; let bref = &b;
                if should_parallelize(total) {
                    let n_threads = optimal_parallelism().min(rows).max(1);
                    let chunk_size = (rows + n_threads - 1) / n_threads;
                    thread::scope(|s| {
                        for (chunk_idx, rows_chunk) in result.chunks_mut(chunk_size).enumerate() {
                            let start_row = chunk_idx * chunk_size;
                            s.spawn(move || {
                                for (local_i, out_row) in rows_chunk.iter_mut().enumerate() {
                                    let ri = start_row + local_i;
                                    if ri < rows {
                                        for j in 0..cols { out_row[j] = aref[ri][j] - bref[ri][j]; }
                                    }
                                }
                            });
                        }
                    });
                } else {
                    for i in 0..rows { for j in 0..cols { result[i][j] = a[i][j] - b[i][j]; } }
                }
                self.stack.push(Value::Matrix(Rc::new(RefCell::new(result))));
            }
            "transpose" => {
                // mat::transpose(mat) → Matrix（自适应并行）
                let mat_val = if arg_count >= 1 { self.stack.pop() } else { Value::Null };
                for _ in 1..arg_count { self.stack.pop(); }
                let matrix = match Self::pop_matrix_owned(mat_val) {
                    Ok(m) => m, Err(e) => { return Err(e); }
                };
                let rows = matrix.len();
                let cols = matrix.first().map_or(0, |r| r.len());
                let total = rows * cols;
                let mut result = vec![vec![0.0; rows]; cols];
                if should_parallelize(total) {
                    let n_threads = optimal_parallelism().min(cols).max(1);
                    let chunk_size = (cols + n_threads - 1) / n_threads;
                    let mref = &matrix;
                    thread::scope(|s| {
                        for (chunk_idx, chunk) in result.chunks_mut(chunk_size).enumerate() {
                            let start_col = chunk_idx * chunk_size;
                            s.spawn(move || {
                                for (local_j, col_vec) in chunk.iter_mut().enumerate() {
                                    let j = start_col + local_j;
                                    if j < cols {
                                        for i in 0..rows {
                                            col_vec[i] = mref[i][j];
                                        }
                                    }
                                }
                            });
                        }
                    });
                } else {
                    for i in 0..rows { for j in 0..cols { result[j][i] = matrix[i][j]; } }
                }
                self.stack.push(Value::Matrix(Rc::new(RefCell::new(result))));
            }
            "shape" => {
                // mat::shape(mat) → Array<i64>  返回 [行数, 列数]
                let mat_val = if arg_count >= 1 { self.stack.pop() } else { Value::Null };
                for _ in 1..arg_count { self.stack.pop(); }
                let matrix = match mat_val {
                    Value::Matrix(data) => data.borrow().clone(),
                    _ => { self.stack.push(Value::Array(Rc::new(RefCell::new(vec![])))); return Ok(()); }
                };
                let rows = matrix.len() as i64;
                let cols = matrix.first().map_or(0, |r| r.len()) as i64;
                let shape = vec![Value::Integer(rows), Value::Integer(cols)];
                self.stack.push(Value::Array(Rc::new(RefCell::new(shape))));
            }
            "print" => {
                // mat::print(mat) → Null
                let mat_val = if arg_count >= 1 { self.stack.pop() } else { Value::Null };
                for _ in 1..arg_count { self.stack.pop(); }
                let matrix = match mat_val {
                    Value::Matrix(data) => data.borrow().clone(),
                    _ => { self.stack.push(Value::Null); return Ok(()); }
                };
                let rows = matrix.len();
                let cols = matrix.first().map_or(0, |r| r.len());
                let output = format!("Matrix[{}x{}]:\n", rows, cols);
                write_to_capture(&output);
                print!("{}", output);
                for row in &matrix {
                    let mut line = String::from("  [");
                    for (j, val) in row.iter().enumerate() {
                        if j > 0 { line.push_str(", "); }
                        if val.fract() == 0.0 && val.abs() < 1e15 {
                            line.push_str(&format!("{}", val));
                        } else {
                            line.push_str(&format!("{:.6}", val));
                        }
                    }
                    line.push(']');
                    let line_nl = line + "\n";
                    write_to_capture(&line_nl);
                    print!("{}", line_nl);
                }
                self.stack.push(Value::Null);
            }
            _ => return Err(format!("未知的 mat 函数: mat::{}", func)),
        }
        Ok(())
    }

    // ─── transformer:: Transformer 推理引擎（纯 Rust，零依赖）───
    //
    // 阶段6：小型 Transformer 纯 Rust 推理后端
    //
    // 架构：TokenEmbedding + PositionalEncoding → N × (MHA + LN + FFN + LN) → OutputProjection
    //
    // KLC transformer API:
    //   transformer::create(d_model, heads)         → TransformerModel   创建随机初始化模型
    //   transformer::forward(model, input)          → Matrix             前向推理，input为one-hot矩阵
    //   transformer::save(model, path)              → Null               保存模型到文件
    //   transformer::load(path)                     → TransformerModel   从文件加载模型
    //   transformer::print(model)                   → Null               打印模型信息
    //
    // 性能策略：
    //   - 大矩阵(>4096元素): 自动13线程并行 (mat::parallel_mul)
    //   - 小矩阵(≤4096元素): 自动单线程顺序乘法

    /// Xorshift64 伪随机数生成器（确定性，用于权重初始化）
    #[inline]
    fn xorshift_next(seed: &mut u64) -> f64 {
        *seed ^= *seed << 13;
        *seed ^= *seed >> 7;
        *seed ^= *seed << 17;
        (*seed as f64) / (u64::MAX as f64)
    }

    /// Xavier 初始化矩阵
    fn init_matrix_xavier(rows: usize, cols: usize, seed: &mut u64) -> Vec<Vec<f64>> {
        let scale = (2.0_f64 / (rows + cols) as f64).sqrt();
        let mut m = vec![vec![0.0; cols]; rows];
        for i in 0..rows {
            for j in 0..cols {
                m[i][j] = (Self::xorshift_next(seed) - 0.5) * 2.0 * scale;
            }
        }
        m
    }

    /// 预计算正弦位置编码 [max_seq_len × d_model]
    fn precompute_pos_encoding(max_seq_len: usize, d_model: usize) -> Vec<Vec<f64>> {
        let mut pe = vec![vec![0.0; d_model]; max_seq_len];
        for pos in 0..max_seq_len {
            for i in 0..d_model {
                let angle = pos as f64 / (10000.0_f64.powf(2.0 * (i / 2) as f64 / d_model as f64));
                pe[pos][i] = if i % 2 == 0 { angle.sin() } else { angle.cos() };
            }
        }
        pe
    }

    /// 自适应矩阵乘法: 小矩阵单线程, 大矩阵13线程并行
    #[inline]
    fn mat_mul_adaptive(a: &Vec<Vec<f64>>, b: &Vec<Vec<f64>>) -> Vec<Vec<f64>> {
        const AUTO_PARALLEL_THRESHOLD: usize = 4096; // 总元素数阈值

        let a_rows = a.len();
        let a_cols = a.first().map_or(0, |r| r.len());
        let b_cols = b.first().map_or(0, |r| r.len());

        if a_rows == 0 || a_cols == 0 || b_cols == 0 {
            return vec![vec![0.0; b_cols]; a_rows];
        }

        let total_elems = a_rows * b_cols;

        if total_elems <= AUTO_PARALLEL_THRESHOLD {
            // 小矩阵: 单线程顺序乘法(i-k-j循环, 缓存友好)
            let mut result = vec![vec![0.0; b_cols]; a_rows];
            for i in 0..a_rows {
                for k in 0..a_cols {
                    let a_ik = a[i][k];
                    if a_ik == 0.0 { continue; }
                    for j in 0..b_cols {
                        result[i][j] += a_ik * b[k][j];
                    }
                }
            }
            result
        } else {
            // 大矩阵: 13线程并行
            Self::parallel_matrix_mul(a.clone(), b.clone()).unwrap_or_else(|_| {
                let mut result = vec![vec![0.0; b_cols]; a_rows];
                for i in 0..a_rows {
                    for k in 0..a_cols {
                        let a_ik = a[i][k];
                        if a_ik == 0.0 { continue; }
                        for j in 0..b_cols {
                            result[i][j] += a_ik * b[k][j];
                        }
                    }
                }
                result
            })
        }
    }

    /// 数值稳定的逐行 Softmax
    fn softmax_rows(x: &mut [Vec<f64>]) {
        for row in x.iter_mut() {
            if row.is_empty() { continue; }
            let max_val = row.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let sum: f64 = row.iter().map(|v| (v - max_val).exp()).sum();
            let inv_sum = if sum == 0.0 { 1.0 } else { 1.0 / sum };
            for v in row.iter_mut() {
                *v = (*v - max_val).exp() * inv_sum;
            }
        }
    }

    /// 层归一化 (原位修改)
    fn layer_norm_inplace(x: &mut [Vec<f64>], gamma: &[Vec<f64>], beta: &[Vec<f64>]) {
        let seq_len = x.len();
        if seq_len == 0 { return; }
        let d = x[0].len();
        let eps = 1e-5_f64;
        for i in 0..seq_len {
            let mean: f64 = x[i].iter().sum::<f64>() / d as f64;
            let var: f64 = x[i].iter().map(|v| (v - mean).powi(2)).sum::<f64>() / d as f64;
            let inv_std = 1.0 / (var + eps).sqrt();
            for j in 0..d {
                x[i][j] = gamma[0][j] * (x[i][j] - mean) * inv_std + beta[0][j];
            }
        }
    }

    /// 多头缩放点积注意力
    fn multi_head_attention(
        x: &Vec<Vec<f64>>,
        layer: &TransformerLayerData,
        d_model: usize,
        n_heads: usize,
    ) -> Vec<Vec<f64>> {
        let seq_len = x.len();
        let d_k = d_model / n_heads;

        // Q, K, V = x @ W_q/k/v
        let q = Self::mat_mul_adaptive(x, &layer.q_proj);
        let k = Self::mat_mul_adaptive(x, &layer.k_proj);
        let v = Self::mat_mul_adaptive(x, &layer.v_proj);

        // 拼接所有头的输出
        let mut concat_out = vec![vec![0.0; d_model]; seq_len];

        for h in 0..n_heads {
            let h_start = h * d_k;

            // 提取当前头的 Q_h, K_h, V_h [seq_len × d_k]
            let mut q_h = vec![vec![0.0; d_k]; seq_len];
            let mut k_h = vec![vec![0.0; d_k]; seq_len];
            let mut v_h = vec![vec![0.0; d_k]; seq_len];
            for i in 0..seq_len {
                for j in 0..d_k {
                    q_h[i][j] = q[i][h_start + j];
                    k_h[i][j] = k[i][h_start + j];
                    v_h[i][j] = v[i][h_start + j];
                }
            }

            // K_h^T [d_k × seq_len]
            let k_t: Vec<Vec<f64>> = (0..d_k).map(|j| {
                (0..seq_len).map(|i| k_h[i][j]).collect()
            }).collect();

            // scores = Q_h @ K_h^T / sqrt(d_k) → [seq_len × seq_len]
            let mut scores = Self::mat_mul_adaptive(&q_h, &k_t);
            let scale = (d_k as f64).sqrt();
            for row in scores.iter_mut() {
                for s in row.iter_mut() {
                    *s /= scale;
                }
            }

            // Softmax over rows
            Self::softmax_rows(&mut scores);

            // Attention output: scores @ V_h → [seq_len × d_k]
            let attn_out = Self::mat_mul_adaptive(&scores, &v_h);

            // 放入拼接缓冲区
            for i in 0..seq_len {
                for j in 0..d_k {
                    concat_out[i][h_start + j] = attn_out[i][j];
                }
            }
        }

        // 输出投影: concat @ W_o → [seq_len × d_model]
        Self::mat_mul_adaptive(&concat_out, &layer.o_proj)
    }

    /// 前馈网络: W1 → ReLU → W2
    fn feed_forward(
        x: &Vec<Vec<f64>>,
        layer: &TransformerLayerData,
    ) -> Vec<Vec<f64>> {
        // hidden = x @ W1 [seq_len × d_ff]
        let mut hidden = Self::mat_mul_adaptive(x, &layer.ffn_w1);
        // ReLU
        for row in hidden.iter_mut() {
            for v in row.iter_mut() {
                *v = v.max(0.0);
            }
        }
        // output = hidden @ W2 [seq_len × d_model]
        Self::mat_mul_adaptive(&hidden, &layer.ffn_w2)
    }

    /// 完整 Transformer 前向传播
    fn transformer_forward(
        model: &TransformerModelData,
        input: Vec<Vec<f64>>,
    ) -> Result<Vec<Vec<f64>>, String> {
        let seq_len = input.len();
        let d_model = model.d_model;

        if seq_len == 0 {
            return Err("transformer::forward 输入序列为空".to_string());
        }
        if seq_len > model.max_seq_len {
            return Err(format!(
                "transformer::forward 序列长度{}超出最大长度{}",
                seq_len, model.max_seq_len
            ));
        }
        let input_dim = input.first().map_or(0, |r| r.len());
        if input_dim != model.vocab_size {
            return Err(format!(
                "transformer::forward 输入维度{}不匹配模型词表{}",
                input_dim, model.vocab_size
            ));
        }

        // 1. Token embedding: input @ token_embedding [seq_len × d_model]
        let mut x = Self::mat_mul_adaptive(&input, &model.token_embedding);

        // 2. 加位置编码
        for i in 0..seq_len.min(model.pos_encoding.len()) {
            for j in 0..d_model {
                x[i][j] += model.pos_encoding[i][j];
            }
        }

        // 3. 逐层 Transformer
        for layer in &model.layers {
            // ── 多头自注意力 + 残差 + LayerNorm ──
            let attn_out = Self::multi_head_attention(&x, layer, d_model, model.n_heads);
            for i in 0..seq_len {
                for j in 0..d_model {
                    x[i][j] += attn_out[i][j];
                }
            }
            Self::layer_norm_inplace(&mut x, &layer.ln1_gamma, &layer.ln1_beta);

            // ── 前馈网络 + 残差 + LayerNorm ──
            let ffn_out = Self::feed_forward(&x, layer);
            for i in 0..seq_len {
                for j in 0..d_model {
                    x[i][j] += ffn_out[i][j];
                }
            }
            Self::layer_norm_inplace(&mut x, &layer.ln2_gamma, &layer.ln2_beta);
        }

        // 4. 输出投影: x @ output_projection [seq_len × vocab_size]
        let logits = Self::mat_mul_adaptive(&x, &model.output_projection);

        Ok(logits)
    }

    /// Transformer 训练步骤：输出层解析梯度 + 嵌入/末层轻量扰动
    ///
    /// 算法（单次前向，消除旧版第2次全量前向的~45%冗余计算）：
    ///   1. 前向传播，获取 logits 和隐藏状态
    ///   2. 计算 softmax + 交叉熵损失及解析梯度 dL/d(logits) = probs - target
    ///   3. 解析更新 output_projection: W_out -= lr * hidden^T @ dL/d(logits)
    ///   4. 嵌入层 + 末层权值轻量随机扰动（无验证，scale=lr*0.3）
    fn transformer_train_step(
        model: &mut TransformerModelData,
        input: &Vec<Vec<f64>>,
        target: &Vec<Vec<f64>>,
        lr: f64,
        rng: &mut u64,
    ) -> Result<f64, String> {
        let seq_len = input.len();
        let d_model = model.d_model;
        let vocab_size = model.vocab_size;

        // ─────────── 1. 前向传播（仅1次）───────────
        let mut x = Self::mat_mul_adaptive(input, &model.token_embedding);
        for i in 0..seq_len.min(model.pos_encoding.len()) {
            for j in 0..d_model {
                x[i][j] += model.pos_encoding[i][j];
            }
        }
        for layer in &model.layers {
            let attn_out = Self::multi_head_attention(&x, layer, d_model, model.n_heads);
            for i in 0..seq_len {
                for j in 0..d_model {
                    x[i][j] += attn_out[i][j];
                }
            }
            Self::layer_norm_inplace(&mut x, &layer.ln1_gamma, &layer.ln1_beta);
            let ffn_out = Self::feed_forward(&x, layer);
            for i in 0..seq_len {
                for j in 0..d_model {
                    x[i][j] += ffn_out[i][j];
                }
            }
            Self::layer_norm_inplace(&mut x, &layer.ln2_gamma, &layer.ln2_beta);
        }

        let hidden = x; // 复用，不需要 clone（x 接下来不再用于前向）
        let logits = Self::mat_mul_adaptive(&hidden, &model.output_projection);

        // ─────────── 2. Softmax + 交叉熵损失 + 梯度 ───────────
        let mut loss = 0.0;
        let mut d_logits = vec![vec![0.0; vocab_size]; seq_len];

        for i in 0..seq_len {
            let max_logit = logits[i].iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let mut sum_exp = 0.0;
            let mut probs = vec![0.0; vocab_size];
            for j in 0..vocab_size {
                probs[j] = (logits[i][j] - max_logit).exp();
                sum_exp += probs[j];
            }
            for j in 0..vocab_size {
                probs[j] /= if sum_exp > 0.0 { sum_exp } else { 1.0 };
            }

            for j in 0..vocab_size {
                d_logits[i][j] = probs[j] - target[i][j];
                if target[i][j] > 0.5 {
                    loss -= probs[j].ln().max(-20.0);
                }
            }
        }
        loss /= seq_len as f64;

        // ─────────── 3. 解析梯度更新 output_projection ───────────
        let hidden_t: Vec<Vec<f64>> = (0..d_model).map(|j| {
            (0..seq_len).map(|i| hidden[i][j]).collect()
        }).collect();
        let grad_output = Self::mat_mul_adaptive(&hidden_t, &d_logits);

        let lr_scaled = lr / (seq_len as f64).max(1.0);
        for j in 0..d_model {
            for k in 0..vocab_size {
                model.output_projection[j][k] -= lr_scaled * grad_output[j][k];
            }
        }

        // ─────────── 4. 轻量扰动（嵌入+末层，无验证）───────────
        let noise = lr_scaled * 0.3;
        // 4a. 嵌入层扰动
        for j in 0..vocab_size {
            for k in 0..d_model {
                model.token_embedding[j][k] += (Self::xorshift_next(rng) - 0.5) * noise;
            }
        }
        // 4b. 末层 FFN 权值微扰（额外训练信号）
        if let Some(last_layer) = model.layers.last_mut() {
            let lr4 = lr_scaled * 0.15;
            for j in 0..d_model {
                for k in 0..model.d_ff {
                    last_layer.ffn_w1[j][k] += (Self::xorshift_next(rng) - 0.5) * lr4;
                }
            }
            for j in 0..model.d_ff {
                for k in 0..d_model {
                    last_layer.ffn_w2[j][k] += (Self::xorshift_next(rng) - 0.5) * lr4;
                }
            }
        }

        Ok(loss)
    }

    /// 创建随机初始化的小型 Transformer 模型
    fn create_transformer_model(d_model: usize, n_heads: usize, vocab_size: usize) -> TransformerModelData {
        let d_ff = d_model * 4;
        let n_layers = 2;
        let max_seq_len = (vocab_size * 2).min(128).max(16); // 自适应: vocab*2, 上限128, 下限16

        let mut rng: u64 = 12345; // 固定种子，确保可复现

        let token_embedding = Self::init_matrix_xavier(vocab_size, d_model, &mut rng);
        let pos_encoding = Self::precompute_pos_encoding(max_seq_len, d_model);
        let output_projection = Self::init_matrix_xavier(d_model, vocab_size, &mut rng);

        let mut layers = Vec::with_capacity(n_layers);
        for _layer_idx in 0..n_layers {
            layers.push(TransformerLayerData {
                q_proj: Self::init_matrix_xavier(d_model, d_model, &mut rng),
                k_proj: Self::init_matrix_xavier(d_model, d_model, &mut rng),
                v_proj: Self::init_matrix_xavier(d_model, d_model, &mut rng),
                o_proj: Self::init_matrix_xavier(d_model, d_model, &mut rng),
                ln1_gamma: vec![vec![1.0; d_model]],
                ln1_beta: vec![vec![0.0; d_model]],
                ffn_w1: Self::init_matrix_xavier(d_model, d_ff, &mut rng),
                ffn_w2: Self::init_matrix_xavier(d_ff, d_model, &mut rng),
                ln2_gamma: vec![vec![1.0; d_model]],
                ln2_beta: vec![vec![0.0; d_model]],
            });
        }

        TransformerModelData {
            d_model,
            n_heads,
            d_ff,
            n_layers,
            vocab_size,
            max_seq_len,
            token_embedding,
            pos_encoding,
            layers,
            output_projection,
        }
    }

    /// 将矩阵保存为文本行（复用 mat 格式）
    fn save_matrix_to(lines: &mut Vec<String>, data: &[Vec<f64>]) {
        let rows = data.len();
        let cols = data.first().map_or(0, |r| r.len());
        lines.push(format!("{} {}", rows, cols));
        for row in data {
            let line: String = row.iter()
                .map(|v| format!("{:.12}", v))
                .collect::<Vec<_>>()
                .join(" ");
            lines.push(line);
        }
    }

    /// 从文本行加载矩阵
    fn load_matrix_from(lines: &[String], line_idx: &mut usize) -> Result<Vec<Vec<f64>>, String> {
        if *line_idx >= lines.len() {
            return Err("transformer::load 文件格式错误: 缺少矩阵尺寸行".to_string());
        }
        let header = &lines[*line_idx];
        *line_idx += 1;
        let parts: Vec<&str> = header.split_whitespace().collect();
        if parts.len() < 2 {
            return Err(format!("transformer::load 矩阵尺寸行格式错误: '{}'", header));
        }
        let rows: usize = parts[0].parse().map_err(|_| format!("transformer::load 行数解析失败: '{}'", parts[0]))?;
        let cols: usize = parts[1].parse().map_err(|_| format!("transformer::load 列数解析失败: '{}'", parts[1]))?;

        let mut matrix = vec![vec![0.0; cols]; rows];
        for i in 0..rows {
            if *line_idx >= lines.len() {
                return Err(format!("transformer::load 缺少第{}行数据", i));
            }
            let data_line = &lines[*line_idx];
            *line_idx += 1;
            let vals: Vec<&str> = data_line.split_whitespace().collect();
            for (j, val_str) in vals.iter().enumerate() {
                if j >= cols { break; }
                matrix[i][j] = val_str.parse::<f64>().unwrap_or(0.0);
            }
        }
        Ok(matrix)
    }

    /// 保存 Transformer 模型到文件
    fn save_transformer_model(model: &TransformerModelData, path: &str) -> Result<(), String> {
        let mut lines: Vec<String> = Vec::new();

        // Header
        lines.push("#KLC_TRANSFORMER_v1".to_string());
        lines.push(format!("{} {} {} {} {} {}",
            model.d_model, model.n_heads, model.d_ff,
            model.n_layers, model.vocab_size, model.max_seq_len));

        // Token embedding
        Self::save_matrix_to(&mut lines, &model.token_embedding);

        // Positional encoding
        Self::save_matrix_to(&mut lines, &model.pos_encoding);

        // Output projection
        Self::save_matrix_to(&mut lines, &model.output_projection);

        // Layers
        lines.push(model.layers.len().to_string());
        for layer in &model.layers {
            Self::save_matrix_to(&mut lines, &layer.q_proj);
            Self::save_matrix_to(&mut lines, &layer.k_proj);
            Self::save_matrix_to(&mut lines, &layer.v_proj);
            Self::save_matrix_to(&mut lines, &layer.o_proj);
            Self::save_matrix_to(&mut lines, &layer.ln1_gamma);
            Self::save_matrix_to(&mut lines, &layer.ln1_beta);
            Self::save_matrix_to(&mut lines, &layer.ffn_w1);
            Self::save_matrix_to(&mut lines, &layer.ffn_w2);
            Self::save_matrix_to(&mut lines, &layer.ln2_gamma);
            Self::save_matrix_to(&mut lines, &layer.ln2_beta);
        }

        let content = lines.join("\n");
        std::fs::write(path, content).map_err(|e| format!("transformer::save 写入失败: {}", e))?;
        Ok(())
    }

    /// 从文件加载 Transformer 模型
    fn load_transformer_model(path: &str) -> Result<TransformerModelData, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("transformer::load 读取失败: {}", e))?;
        let lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();

        let mut idx: usize = 0;

        // Header check
        if idx >= lines.len() || lines[idx] != "#KLC_TRANSFORMER_v1" {
            return Err("transformer::load 文件格式错误: 缺少头部标识".to_string());
        }
        idx += 1;

        // Config
        if idx >= lines.len() {
            return Err("transformer::load 缺少配置行".to_string());
        }
        let config_parts: Vec<&str> = lines[idx].split_whitespace().collect();
        idx += 1;
        if config_parts.len() < 6 {
            return Err("transformer::load 配置行格式错误".to_string());
        }
        let d_model: usize = config_parts[0].parse().map_err(|_| "d_model解析失败")?;
        let n_heads: usize = config_parts[1].parse().map_err(|_| "n_heads解析失败")?;
        let d_ff: usize = config_parts[2].parse().map_err(|_| "d_ff解析失败")?;
        let n_layers: usize = config_parts[3].parse().map_err(|_| "n_layers解析失败")?;
        let vocab_size: usize = config_parts[4].parse().map_err(|_| "vocab_size解析失败")?;
        let max_seq_len: usize = config_parts[5].parse().map_err(|_| "max_seq_len解析失败")?;

        // Token embedding
        let token_embedding = Self::load_matrix_from(&lines, &mut idx)?;

        // Positional encoding
        let pos_encoding = Self::load_matrix_from(&lines, &mut idx)?;

        // Output projection
        let output_projection = Self::load_matrix_from(&lines, &mut idx)?;

        // Layer count
        if idx >= lines.len() {
            return Err("transformer::load 缺少层数行".to_string());
        }
        let saved_n_layers: usize = lines[idx].parse().map_err(|_| "层数解析失败")?;
        idx += 1;

        let mut layers = Vec::with_capacity(saved_n_layers);
        for _ in 0..saved_n_layers {
            let q_proj = Self::load_matrix_from(&lines, &mut idx)?;
            let k_proj = Self::load_matrix_from(&lines, &mut idx)?;
            let v_proj = Self::load_matrix_from(&lines, &mut idx)?;
            let o_proj = Self::load_matrix_from(&lines, &mut idx)?;
            let ln1_gamma = Self::load_matrix_from(&lines, &mut idx)?;
            let ln1_beta = Self::load_matrix_from(&lines, &mut idx)?;
            let ffn_w1 = Self::load_matrix_from(&lines, &mut idx)?;
            let ffn_w2 = Self::load_matrix_from(&lines, &mut idx)?;
            let ln2_gamma = Self::load_matrix_from(&lines, &mut idx)?;
            let ln2_beta = Self::load_matrix_from(&lines, &mut idx)?;
            layers.push(TransformerLayerData {
                q_proj, k_proj, v_proj, o_proj,
                ln1_gamma, ln1_beta,
                ffn_w1, ffn_w2,
                ln2_gamma, ln2_beta,
            });
        }

        Ok(TransformerModelData {
            d_model, n_heads, d_ff, n_layers,
            vocab_size, max_seq_len,
            token_embedding, pos_encoding,
            layers, output_projection,
        })
    }

    /// transformer:: 内置函数分发
    fn run_transformer_func(&mut self, func: &str, arg_count: usize) -> Result<(), String> {
        match func {
            "create" => {
                // transformer::create(d_model: i64, heads: i64, vocab_size?: i64) → TransformerModel
                // 第3个参数可选，默认128
                let vocab_val = if arg_count >= 3 { self.stack.pop() } else { Value::Null };
                let heads_val = if arg_count >= 2 { self.stack.pop() } else { Value::Null };
                let d_model_val = if arg_count >= 1 { self.stack.pop() } else { Value::Null };
                for _ in 3..arg_count { self.stack.pop(); }

                let d_model = match &d_model_val {
                    Value::Integer(n) => *n as usize,
                    _ => {
                        self.stack.push(Value::Null);
                        return Ok(());
                    }
                };
                let n_heads = match &heads_val {
                    Value::Integer(n) => *n as usize,
                    _ => {
                        self.stack.push(Value::Null);
                        return Ok(());
                    }
                };
                let vocab_size = match &vocab_val {
                    Value::Integer(n) if *n > 0 => *n as usize,
                    _ => 128, // 默认词表大小
                };

                if d_model == 0 || n_heads == 0 {
                    return Err(format!(
                        "transformer::create 参数必须为正数: d_model={}, heads={}",
                        d_model, n_heads
                    ));
                }
                if d_model % n_heads != 0 {
                    return Err(format!(
                        "transformer::create d_model({})必须能被heads({})整除",
                        d_model, n_heads
                    ));
                }

                let model = Self::create_transformer_model(d_model, n_heads, vocab_size);
                self.stack.push(Value::TransformerModel(Rc::new(RefCell::new(model))));
            }
            "train_step" => {
                // transformer::train_step(model, x_input, y_target, lr) → Float(loss)
                // model 通过 Rc 原地修改, 返回值只有 loss
                let lr = self.pop_f64();
                let target_val = if arg_count >= 3 { self.stack.pop() } else { Value::Null };
                let input_val = if arg_count >= 2 { self.stack.pop() } else { Value::Null };
                let model_val = if arg_count >= 1 { self.stack.pop() } else { Value::Null };
                for _ in 4..arg_count { self.stack.pop(); }

                let target = match target_val {
                    Value::Matrix(data) => data.borrow().clone(),
                    _ => {
                        return Err("transformer::train_step 第3个参数(y_target)必须是 Matrix".to_string());
                    }
                };
                let input = match input_val {
                    Value::Matrix(data) => data.borrow().clone(),
                    _ => {
                        return Err("transformer::train_step 第2个参数(x_input)必须是 Matrix".to_string());
                    }
                };
                let model_rc = match &model_val {
                    Value::TransformerModel(rc) => rc.clone(),
                    _ => {
                        return Err("transformer::train_step 第1个参数(model)必须是 TransformerModel".to_string());
                    }
                };

                let mut rng: u64 = 54321;
                let loss = Self::transformer_train_step(
                    &mut model_rc.borrow_mut(), &input, &target, lr, &mut rng,
                ).unwrap_or(999.0);

                // model 通过 Rc 自动更新, 只需 push loss
                self.stack.push(Value::Float(loss));
            }
            "forward" => {
                // transformer::forward(model, input_matrix) → Matrix (logits)
                let input_val = if arg_count >= 2 { self.stack.pop() } else { Value::Null };
                let model_val = if arg_count >= 1 { self.stack.pop() } else { Value::Null };
                for _ in 2..arg_count { self.stack.pop(); }

                let model = match &model_val {
                    Value::TransformerModel(m) => m.borrow().clone(),
                    _ => {
                        return Err("transformer::forward 第一个参数必须是 TransformerModel".to_string());
                    }
                };
                let input = match input_val {
                    Value::Matrix(data) => data.borrow().clone(),
                    _ => {
                        return Err("transformer::forward 第二个参数必须是 Matrix".to_string());
                    }
                };

                match Self::transformer_forward(&model, input) {
                    Ok(logits) => {
                        self.stack.push(Value::Matrix(Rc::new(RefCell::new(logits))));
                    }
                    Err(e) => return Err(e),
                }
            }
            "save" => {
                // transformer::save(model, path) → Null
                let path_val = if arg_count >= 2 { self.stack.pop() } else { Value::Null };
                let model_val = if arg_count >= 1 { self.stack.pop() } else { Value::Null };
                for _ in 2..arg_count { self.stack.pop(); }

                let path = match &path_val {
                    Value::String(s) => s.as_str(),
                    _ => {
                        self.stack.push(Value::Null);
                        return Ok(());
                    }
                };
                let model = match &model_val {
                    Value::TransformerModel(m) => m.borrow().clone(),
                    _ => {
                        return Err("transformer::save 第一个参数必须是 TransformerModel".to_string());
                    }
                };

                match Self::save_transformer_model(&model, path) {
                    Ok(()) => self.stack.push(Value::Null),
                    Err(e) => return Err(e),
                }
            }
            "load" => {
                // transformer::load(path) → TransformerModel
                let path_val = if arg_count >= 1 { self.stack.pop() } else { Value::Null };
                for _ in 1..arg_count { self.stack.pop(); }

                let path = match &path_val {
                    Value::String(s) => s.as_str(),
                    _ => {
                        self.stack.push(Value::Null);
                        return Ok(());
                    }
                };

                match Self::load_transformer_model(path) {
                    Ok(model) => {
                        self.stack.push(Value::TransformerModel(Rc::new(RefCell::new(model))));
                    }
                    Err(e) => return Err(e),
                }
            }
            "print" => {
                // transformer::print(model) → Null
                let model_val = if arg_count >= 1 { self.stack.pop() } else { Value::Null };
                for _ in 1..arg_count { self.stack.pop(); }

                let model = match &model_val {
                    Value::TransformerModel(m) => m.borrow().clone(),
                    _ => {
                        return Err("transformer::print 参数必须是 TransformerModel".to_string());
                    }
                };

                let output = format!(
                    "TransformerModel:\n\
                      d_model={}, n_heads={}, d_k={}\n\
                      d_ff={}, n_layers={}\n\
                      vocab_size={}, max_seq_len={}\n\
                      总参数量: ~{}\n",
                    model.d_model,
                    model.n_heads,
                    model.d_model / model.n_heads,
                    model.d_ff,
                    model.n_layers,
                    model.vocab_size,
                    model.max_seq_len,
                    // 粗略参数估算
                    model.vocab_size * model.d_model       // token embedding
                        + model.max_seq_len * model.d_model // pos encoding
                        + model.d_model * model.vocab_size  // output projection
                        + model.n_layers * (
                            4 * model.d_model * model.d_model  // Q,K,V,O projections
                            + 2 * model.d_model                 // LN1 gamma+beta
                            + model.d_model * model.d_ff * 2    // FFN W1+W2
                            + 2 * model.d_model                 // LN2 gamma+beta
                        )
                );
                write_to_capture(&output);
                print!("{}", output);
                self.stack.push(Value::Null);
            }
            _ => return Err(format!("未知的 transformer 函数: transformer::{}", func)),
        }
        Ok(())
    }

=======
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20
    // ─── math 标准库 — 高性能内联（无闭包、无 format!） ───

    #[inline(always)]
    fn run_math_func_fast(&mut self, func: &str, arg_count: usize) -> Result<(), String> {
        match func {
            "pi" => {
                for _ in 0..arg_count { self.stack.pop(); }
                self.stack.push(Value::Float(std::f64::consts::PI));
            }
            "e" => {
                for _ in 0..arg_count { self.stack.pop(); }
                self.stack.push(Value::Float(std::f64::consts::E));
            }
            "exp" => {
                let x = self.pop_f64();
                for _ in 1..arg_count { self.stack.pop(); }
                self.stack.push(Value::Float(x.exp()));
            }
            "tanh" => {
                let x = self.pop_f64();
                for _ in 1..arg_count { self.stack.pop(); }
                self.stack.push(Value::Float(x.tanh()));
            }
            "sin" => {
                let x = self.pop_f64();
                for _ in 1..arg_count { self.stack.pop(); }
                self.stack.push(Value::Float(x.sin()));
            }
            "cos" => {
                let x = self.pop_f64();
                for _ in 1..arg_count { self.stack.pop(); }
                self.stack.push(Value::Float(x.cos()));
            }
            "sqrt" => {
                let x = self.pop_f64();
                for _ in 1..arg_count { self.stack.pop(); }
                self.stack.push(Value::Float(x.sqrt()));
            }
            "log" | "ln" => {
                let x = self.pop_f64();
                for _ in 1..arg_count { self.stack.pop(); }
                self.stack.push(Value::Float(x.ln()));
            }
            "log2" => {
                let x = self.pop_f64();
                for _ in 1..arg_count { self.stack.pop(); }
                self.stack.push(Value::Float(x.log2()));
            }
            "log10" => {
                let x = self.pop_f64();
                for _ in 1..arg_count { self.stack.pop(); }
                self.stack.push(Value::Float(x.log10()));
            }
            "abs" => {
                let val = self.stack.pop();
                for _ in 1..arg_count { self.stack.pop(); }
                match val {
                    Value::Integer(n) => self.stack.push(Value::Integer(n.abs())),
                    Value::Float(f) => self.stack.push(Value::Float(f.abs())),
                    _ => self.stack.push(Value::Null),
                }
            }
            "min" => {
                let b = self.pop_f64();
                let a = self.pop_f64();
                for _ in 2..arg_count { self.stack.pop(); }
                self.stack.push(Value::Float(a.min(b)));
            }
            "max" => {
                let b = self.pop_f64();
                let a = self.pop_f64();
                for _ in 2..arg_count { self.stack.pop(); }
                self.stack.push(Value::Float(a.max(b)));
            }
            "pow" => {
                let exp = self.pop_f64();
                let base = self.pop_f64();
                for _ in 2..arg_count { self.stack.pop(); }
                self.stack.push(Value::Float(base.powf(exp)));
            }
            "floor" => {
                let x = self.pop_f64();
                for _ in 1..arg_count { self.stack.pop(); }
                self.stack.push(Value::Float(x.floor()));
            }
            "ceil" => {
                let x = self.pop_f64();
                for _ in 1..arg_count { self.stack.pop(); }
                self.stack.push(Value::Float(x.ceil()));
            }
            "round" => {
                let x = self.pop_f64();
                for _ in 1..arg_count { self.stack.pop(); }
                self.stack.push(Value::Float(x.round()));
            }
            _ => return Err(format!("未知的 math 函数: math.{}", func)),
        }
        Ok(())
    }

    // ─── % 格式化 ───

    fn run_format(&mut self, arg_count: usize) -> Result<(), String> {
        if arg_count == 0 { return Ok(()); }

        let mut args = Vec::with_capacity(arg_count);
        for _ in 0..arg_count { args.push(self.stack.pop()); }
        args.reverse();

        let template = match &args[0] {
            Value::String(s) => s.clone(),
            _ => return Ok(()),
        };
        let fmt_args = &args[1..];

        let mut result = String::with_capacity(template.len() + 64);
        let mut chars = template.chars().peekable();
        let mut arg_idx = 0;

        while let Some(ch) = chars.next() {
            if ch == '%' {
                let mut spec = String::with_capacity(8);
                if chars.peek() == Some(&'.') {
                    spec.push(chars.next().unwrap());
                    while let Some(&c) = chars.peek() {
                        if c.is_ascii_digit() {
                            spec.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }
                }
                if let Some(&c) = chars.peek() {
                    spec.push(c);
                    chars.next();
                }
                let val = fmt_args.get(arg_idx).cloned().unwrap_or(Value::Null);
                arg_idx += 1;

                let formatted = match spec.as_str() {
                    "d" | "i" => {
                        match val {
                            Value::Integer(n) => n.to_string(),
                            Value::Float(f) => (f as i64).to_string(),
                            _ => val.to_string(),
                        }
                    }
                    "f" => {
                        let precision = if spec.len() > 2 {
                            spec[2..].parse::<usize>().unwrap_or(6)
                        } else { 6 };
                        match val {
                            Value::Float(f) => format!("{:.prec$}", f, prec = precision),
                            Value::Integer(n) => format!("{:.prec$}", n as f64, prec = precision),
                            _ => val.to_string(),
                        }
                    }
                    "s" => val.to_string(),
                    "e" => {
                        match val {
                            Value::Float(f) => format!("{:e}", f),
                            Value::Integer(n) => format!("{:e}", n as f64),
                            _ => val.to_string(),
                        }
                    }
                    "x" | "X" => {
                        let upper = spec == "X";
                        match val {
                            Value::Integer(n) => {
                                if upper { format!("{:X}", n) } else { format!("{:x}", n) }
                            }
                            _ => val.to_string(),
                        }
                    }
                    "b" => {
                        match val {
                            Value::Bool(b) => if b { "true".to_string() } else { "false".to_string() },
                            Value::Integer(n) => format!("{:b}", n),
                            _ => val.to_string(),
                        }
                    }
                    "p" => {
                        match val {
                            Value::Float(f) => format!("{:.6}", f),
                            Value::Integer(n) => n.to_string(),
                            _ => val.to_string(),
                        }
                    }
                    "%" => "%".to_string(),
                    _ => format!("%{}", spec),
                };
                result.push_str(&formatted);
            } else {
                result.push(ch);
            }
        }

        self.stack.push(Value::String(result));
        Ok(())
    }

    // ─── 变量访问（优化版） ───

    #[inline(always)]
    fn get_var(&self, name: &str) -> Value {
        for scope in self.locals.iter().rev() {
            if let Some(val) = scope.get(name) {
                return val.clone();
            }
        }
        self.globals.get(name).cloned().unwrap_or(Value::Null)
    }

    #[inline(always)]
    fn init_var(&mut self, name: String, value: Value) {
        if let Some(scope) = self.locals.last_mut() {
            scope.insert(name, value);
        } else {
            self.globals.insert(name, value);
        }
    }

    #[inline(always)]
    fn set_var(&mut self, name: String, value: Value) {
        for scope in self.locals.iter_mut().rev() {
            if scope.contains_key(&name) {
                scope.insert(name, value);
                return;
            }
        }
        self.globals.insert(name, value);
    }
}
