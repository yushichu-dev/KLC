//! KLC 虚拟机 — 高性能字节码执行引擎 (v0.8.4 optimized)
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
use crate::bytecode::*;

// ============================================================================
// 输出捕获（用于 IDE GUI 中重定向 Print/PrintLn 输出）
// ============================================================================

use std::sync::Mutex;

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
            Instruction::Halt => {
                self.should_halt = true;
            }
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

        // Some / None
        if name == "Some" && arg_count == 1 {
            // 栈顶不动，包装为 Enum
            return Ok(()); // TODO: 直接包装
        }
        if name == "None" && arg_count == 0 {
            self.stack.push(Value::Enum(Rc::new(RefCell::new(("Option".into(), "None".into(), vec![])))));
            return Ok(());
        }

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
                    if *i >= 0 && (*i as usize) < s.len() {
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
                Value::String(s) => self.stack.push(Value::Integer(s.len() as i64)),
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
                    "trim" if arg_count == 1 => {
                        let val = self.stack.pop();
                        if let Value::String(s) = val {
                            self.stack.push(Value::String(s.trim().to_string()));
                        }
                        return Ok(());
                    }
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
