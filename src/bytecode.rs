//! KLC 字节码定义 — 指令集、值类型、编译后程序结构

use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;
use std::cell::RefCell;

// ============================================================================
// 值类型
// ============================================================================

#[derive(Clone)]
pub enum Value {
    Integer(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Char(char),
    Null,
    Struct(Rc<RefCell<(String, Vec<(String, Value)>)>>),
    Array(Rc<RefCell<Vec<Value>>>),
    Enum(Rc<RefCell<(String, String, Vec<Value>)>>),
    Map(Rc<RefCell<HashMap<String, Value>>>),
    Function(String),
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Integer(a), Value::Integer(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Char(a), Value::Char(b)) => a == b,
            (Value::Null, Value::Null) => true,
            _ => false,
        }
    }
}

impl Eq for Value {}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Value::Integer(a), Value::Integer(b)) => a.partial_cmp(b),
            (Value::Float(a), Value::Float(b)) => a.partial_cmp(b),
            (Value::String(a), Value::String(b)) => Some(a.cmp(b)),
            (Value::Bool(a), Value::Bool(b)) => Some(a.cmp(b)),
            (Value::Char(a), Value::Char(b)) => Some(a.cmp(b)),
            _ => None,
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Integer(n) => write!(f, "{}", n),
            Value::Float(n) => write!(f, "{}", n),
            Value::String(s) => write!(f, "{:?}", s),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Char(c) => write!(f, "{:?}", c),
            Value::Null => write!(f, "null"),
            Value::Struct(inner) => {
                let b = inner.borrow();
                write!(f, "{{{}: ", b.0)?;
                for (i, (name, val)) in b.1.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}={}", name, val)?;
                }
                write!(f, "}}")
            }
            Value::Array(items) => {
                let items = items.borrow();
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
            Value::Enum(inner) => {
                let b = inner.borrow();
                if b.2.is_empty() {
                    write!(f, "{}::{}", b.0, b.1)
                } else {
                    write!(f, "{}::{}({:?})", b.0, b.1, b.2)
                }
            }
            Value::Map(m) => {
                let m = m.borrow();
                write!(f, "{{")?;
                for (i, (k, v)) in m.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            }
            Value::Function(name) => write!(f, "<fn {}>", name),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Integer(n) => write!(f, "{}", n),
            Value::Float(n) => {
                if n.fract() == 0.0 && n.abs() < 1e15 {
                    write!(f, "{}", n)
                } else {
                    write!(f, "{:.6}", n)
                }
            }
            Value::String(s) => write!(f, "{}", s),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Char(c) => write!(f, "{}", c),
            Value::Null => write!(f, "null"),
            other => write!(f, "{:?}", other),
        }
    }
}

// ============================================================================
// 字节码指令
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    Const(usize),
    Pop,
    Load(String),
    Store(String),
    InitVar(String),
    Add, Sub, Mul, Div, Mod, Neg,
    Eq, Neq, Lt, Gt, Lte, Gte,
    And, Or, Not,
    Concat,
    ToString,
    StructNew(String, usize),
    StructGet(String),
    StructSet(String),
    Jmp(usize),
    JmpFalse(usize),
    Call(String, usize),
    Return,
    Halt,
    Print,
    PrintLn,
    RegFn(String, String),
    IsVariant(String),
    EnumGet(usize),
}

// ============================================================================
// 编译后函数
// ============================================================================

#[derive(Debug, Clone)]
pub struct CompiledFunction {
    pub name: String,
    pub instructions: Vec<Instruction>,
    pub param_count: usize,
    pub param_names: Vec<String>,
}

// ============================================================================
// 字节码程序
// ============================================================================

#[derive(Debug)]
pub struct BytecodeProgram {
    pub main: Vec<Instruction>,
    pub functions: Vec<CompiledFunction>,
    pub constants: Vec<Value>,
}
