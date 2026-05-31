//! KLC 字节码定义 — 指令集、值类型、编译后程序结构

use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;
use std::cell::RefCell;

// ============================================================================
<<<<<<< HEAD
// Transformer 模型数据结构（阶段6：纯Rust推理后端）
// ============================================================================

/// 单层 Transformer 层权重
#[derive(Clone)]
pub struct TransformerLayerData {
    /// Multi-head attention: Q/K/V 投影矩阵 [d_model × d_model]
    pub q_proj: Vec<Vec<f64>>,
    pub k_proj: Vec<Vec<f64>>,
    pub v_proj: Vec<Vec<f64>>,
    /// 注意力输出投影 [d_model × d_model]
    pub o_proj: Vec<Vec<f64>>,
    /// LayerNorm 1: 缩放和偏移 [1 × d_model]
    pub ln1_gamma: Vec<Vec<f64>>,
    pub ln1_beta: Vec<Vec<f64>>,
    /// 前馈网络: W1 [d_model × d_ff], W2 [d_ff × d_model]
    pub ffn_w1: Vec<Vec<f64>>,
    pub ffn_w2: Vec<Vec<f64>>,
    /// LayerNorm 2: 缩放和偏移 [1 × d_model]
    pub ln2_gamma: Vec<Vec<f64>>,
    pub ln2_beta: Vec<Vec<f64>>,
}

/// 完整 Transformer 模型（推理用）
#[derive(Clone)]
pub struct TransformerModelData {
    pub d_model: usize,
    pub n_heads: usize,
    pub d_ff: usize,
    pub n_layers: usize,
    pub vocab_size: usize,
    pub max_seq_len: usize,
    /// Token 嵌入矩阵 [vocab_size × d_model]
    pub token_embedding: Vec<Vec<f64>>,
    /// 正弦位置编码 [max_seq_len × d_model]（预计算）
    pub pos_encoding: Vec<Vec<f64>>,
    /// 各层权重
    pub layers: Vec<TransformerLayerData>,
    /// 输出投影矩阵 [d_model × vocab_size]
    pub output_projection: Vec<Vec<f64>>,
}

// ============================================================================
=======
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20
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
<<<<<<< HEAD
    Matrix(Rc<RefCell<Vec<Vec<f64>>>>),
    /// 阶段6: Transformer 推理模型
    TransformerModel(Rc<RefCell<TransformerModelData>>),
=======
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20
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
<<<<<<< HEAD
            (Value::Matrix(a), Value::Matrix(b)) => {
                let ra = a.borrow();
                let rb = b.borrow();
                if ra.len() != rb.len() { return false; }
                for (row_a, row_b) in ra.iter().zip(rb.iter()) {
                    if row_a.len() != row_b.len() { return false; }
                    for (va, vb) in row_a.iter().zip(row_b.iter()) {
                        if va != vb { return false; }
                    }
                }
                true
            }
            (Value::TransformerModel(a), Value::TransformerModel(b)) => {
                let ma = a.borrow();
                let mb = b.borrow();
                ma.d_model == mb.d_model
                    && ma.n_heads == mb.n_heads
                    && ma.d_ff == mb.d_ff
                    && ma.n_layers == mb.n_layers
                    && ma.vocab_size == mb.vocab_size
                    && ma.max_seq_len == mb.max_seq_len
            }
=======
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20
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
<<<<<<< HEAD
            Value::Matrix(data) => {
                let m = data.borrow();
                write!(f, "Matrix[{}x{}]", m.len(), m.first().map_or(0, |r| r.len()))
            }
            Value::TransformerModel(model) => {
                let m = model.borrow();
                write!(f, "TransformerModel(d={}, heads={}, d_ff={}, layers={}, vocab={})",
                    m.d_model, m.n_heads, m.d_ff, m.n_layers, m.vocab_size)
            }
=======
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20
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
<<<<<<< HEAD
            Value::Matrix(data) => {
                let m = data.borrow();
                write!(f, "Matrix(rows={}, cols={}", m.len(), m.first().map_or(0, |r| r.len()))
            }
            Value::TransformerModel(model) => {
                let m = model.borrow();
                write!(f, "TransformerModel(d={}, heads={}, layers={}, vocab={})",
                    m.d_model, m.n_heads, m.n_layers, m.vocab_size)
            }
=======
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20
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
<<<<<<< HEAD
    ReadLine,
    RegFn(String, String),
    IsVariant(String),
    EnumGet(usize),
    /// 创建枚举值: EnumNew(enum_type, variant_name, field_count)
    /// 栈: val1, val2, ... → Enum { type, variant, [val1, val2, ...] }
    EnumNew(String, String, usize),
    /// 字符串切片: 栈 top=end, top-1=start, top-2=string → substring
    #[allow(dead_code)]
    SubStr,
    /// 字符串查找: 栈 top=needle, top-1=haystack → index (i64) 或 -1
    #[allow(dead_code)]
    StrFind,
    /// 字符串重复: 栈 top=count, top-1=string → repeated_string
    #[allow(dead_code)]
    StrRepeat,
    /// 异步任务调度: Spawn(func_name, arg_count) → 将调用派发到线程池
    #[allow(dead_code)]
    Spawn(String, usize),
    /// 等待所有异步任务完成
    #[allow(dead_code)]
    WaitAll,
=======
    RegFn(String, String),
    IsVariant(String),
    EnumGet(usize),
>>>>>>> 1e7cd86eb6ec8e464f8cb02b273e397c600e8c20
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
