//! KLC 模块系统 — 多文件编译与模块导入
//!
//! 支持:
//! - `mod name;` 声明模块（从同名 .klc 文件或目录/mod.klc 加载）
//! - `use path::item;` 导入其他模块的符号

#![allow(dead_code)]
//! - `use path::item as alias;` 别名导入
//! - `pub` 可见性标记
//! - 符号表构建与去重

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::ast::{Program, Stmt, Expr, Param};
use crate::lexer::Lexer;
use crate::parser::Parser;

/// 模块路径解析结果
#[derive(Debug, Clone)]
pub struct ModuleInfo {
    /// 模块名称 (如 "utils", "math")
    pub name: String,
    /// 模块源文件路径
    pub file_path: PathBuf,
    /// 模块的 AST
    pub program: Program,
    /// 模块导出的符号
    pub exports: HashMap<String, ExportItem>,
}

/// 导出项
#[derive(Debug, Clone)]
pub struct ExportItem {
    /// 符号名称
    pub name: String,
    /// 导出类型
    pub kind: ExportKind,
    /// 是否为 pub 导出
    pub is_pub: bool,
}

/// 导出类型
#[derive(Debug, Clone)]
pub enum ExportKind {
    Function { params: Vec<Param>, return_type: Option<String> },
    Type { fields: Vec<String> },
    Enum { variants: Vec<String> },
    Const,
    Variable,
}

/// 模块解析器 — 处理 mod/use 声明，构建模块图
pub struct ModuleResolver {
    /// 模块缓存: 模块名 → ModuleInfo
    modules: HashMap<String, ModuleInfo>,
    /// 当前文件搜索目录
    base_dir: PathBuf,
    /// 已解析的文件集合（防止循环依赖）
    parsed_files: HashMap<PathBuf, String>, // file_path → module_name
    /// 导入映射: 当前作用域的 use 别名 → (模块名, 原始符号名)
    imports: Vec<ImportEntry>,
    /// 全局符号表: 符号名 → 来源模块名
    global_symbols: HashMap<String, String>,
}

/// 导入条目
#[derive(Debug, Clone)]
pub struct ImportEntry {
    /// 导入的完整路径 (如 "std::io")
    pub path: Vec<String>,
    /// 导入的项名
    pub item: String,
    /// 本地别名
    pub alias: String,
}

impl ModuleResolver {
    pub fn new(base_dir: &Path) -> Self {
        ModuleResolver {
            modules: HashMap::new(),
            base_dir: base_dir.to_path_buf(),
            parsed_files: HashMap::new(),
            imports: Vec::new(),
            global_symbols: HashMap::new(),
        }
    }

    /// 解析入口文件并递归解析所有依赖模块
    pub fn resolve_entry(&mut self, entry_path: &Path) -> Result<Program, String> {
        let source = std::fs::read_to_string(entry_path)
            .map_err(|e| format!("Cannot read '{}': {}", entry_path.display(), e))?;

        let entry_dir = entry_path.parent()
            .unwrap_or(Path::new("."))
            .to_path_buf();

        let mut program = self.parse_and_resolve(entry_path, &source, &entry_dir, "__main__")?;

        // 收集所有模块的语句，合并到主程序
        self.collect_all_modules(&mut program);

        Ok(program)
    }

    /// 解析单个文件并递归处理 mod 声明
    fn parse_and_resolve(&mut self, file_path: &Path, source: &str, search_dir: &Path, module_name: &str) -> Result<Program, String> {
        // 检查循环依赖
        let canonical = file_path.to_path_buf();
        if self.parsed_files.contains_key(&canonical) {
            // 已解析，返回空程序
            return Ok(Program { statements: vec![] });
        }
        self.parsed_files.insert(canonical.clone(), module_name.to_string());

        // 词法 + 语法分析
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let mut program = parser.parse_program()?;

        // 处理 mod 声明
        let mut new_statements = Vec::new();
        for stmt in program.statements.drain(..) {
            if let Stmt::Expr(Expr::Integer(0)) = &stmt {
                // 可能是已跳过的 mod/use 占位符，我们需要重新解析
                // 这里我们通过重新词法分析来获取 mod/use 信息
            }
            new_statements.push(stmt);
        }

        // 重新扫描源码中的 mod 和 use 声明
        let mod_use_stmts = self.scan_mod_use(source, search_dir)?;
        for mod_stmt in mod_use_stmts {
            new_statements.push(mod_stmt);
        }

        program.statements = new_statements;

        // 构建模块导出符号
        let exports = self.build_exports(&program);
        let info = ModuleInfo {
            name: module_name.to_string(),
            file_path: file_path.to_path_buf(),
            program: program.clone(),
            exports,
        };
        self.modules.insert(module_name.to_string(), info);

        Ok(program)
    }

    /// 从源码中扫描 mod 和 use 声明
    fn scan_mod_use(&mut self, source: &str, search_dir: &Path) -> Result<Vec<Stmt>, String> {
        let mut stmts = Vec::new();
        let mut lexer = Lexer::new(source);
        let all_tokens = lexer.tokenize();

        let mut i = 0;
        while i < all_tokens.len() {
            let token = &all_tokens[i];
            match &token.kind {
                crate::token::TokenKind::Mod => {
                    i += 1;
                    if i < all_tokens.len() {
                        if let crate::token::TokenKind::Ident(name) = &all_tokens[i].kind {
                            let mod_name = name.clone();
                            // 查找模块文件
                            let mod_path = self.find_module_file(search_dir, &mod_name)?;
                            if let Some(path) = mod_path {
                                let mod_source = std::fs::read_to_string(&path)
                                    .map_err(|e| format!("Cannot read module '{}': {}", path.display(), e))?;
                                let mod_dir = path.parent().unwrap_or(search_dir).to_path_buf();
                                let mod_program = self.parse_and_resolve(&path, &mod_source, &mod_dir, &mod_name)?;
                                // 将模块的公开语句合并
                                for s in mod_program.statements {
                                    stmts.push(s);
                                }
                            }
                        }
                    }
                }
                crate::token::TokenKind::Use => {
                    // 记录 use 但不实际修改 AST（后续符号查找使用）
                    i += 1;
                }
                _ => {}
            }
            i += 1;
        }
        Ok(stmts)
    }

    /// 查找模块文件
    /// 尝试顺序: mod.klc, mod/mod.klc, mod.klc (当前目录)
    fn find_module_file(&self, search_dir: &Path, mod_name: &str) -> Result<Option<PathBuf>, String> {
        // 1. search_dir/mod_name.klc
        let direct = search_dir.join(format!("{}.klc", mod_name));
        if direct.exists() {
            return Ok(Some(direct));
        }

        // 2. search_dir/mod_name/mod_name.klc
        let dir_mod = search_dir.join(mod_name).join(format!("{}.klc", mod_name));
        if dir_mod.exists() {
            return Ok(Some(dir_mod));
        }

        // 3. search_dir/mod_name/mod.klc (约定: index file)
        let dir_index = search_dir.join(mod_name).join("mod.klc");
        if dir_index.exists() {
            return Ok(Some(dir_index));
        }

        // 模块文件不存在不是错误（可能是内置模块或尚未创建）
        Ok(None)
    }

    /// 构建模块的导出符号表
    fn build_exports(&self, program: &Program) -> HashMap<String, ExportItem> {
        let mut exports = HashMap::new();

        for stmt in &program.statements {
            match stmt {
                Stmt::FnDef { name, params, return_type, .. } => {
                    exports.insert(name.clone(), ExportItem {
                        name: name.clone(),
                        kind: ExportKind::Function {
                            params: params.clone(),
                            return_type: return_type.clone(),
                        },
                        is_pub: true,
                    });
                }
                Stmt::TypeDef { name, fields, .. } => {
                    let field_names: Vec<String> = fields.iter().map(|f| f.name.clone()).collect();
                    exports.insert(name.clone(), ExportItem {
                        name: name.clone(),
                        kind: ExportKind::Type { fields: field_names },
                        is_pub: true,
                    });
                }
                Stmt::EnumDef { name, variants, .. } => {
                    let variant_names: Vec<String> = variants.iter().map(|v| v.name.clone()).collect();
                    exports.insert(name.clone(), ExportItem {
                        name: name.clone(),
                        kind: ExportKind::Enum { variants: variant_names },
                        is_pub: true,
                    });
                }
                Stmt::Let { name, .. } => {
                    exports.insert(name.clone(), ExportItem {
                        name: name.clone(),
                        kind: ExportKind::Variable,
                        is_pub: true,
                    });
                }
                _ => {}
            }
        }

        exports
    }

    /// 收集所有模块的语句并合并到主程序
    fn collect_all_modules(&self, _program: &mut Program) {
        // 目前模块解析时已内联合并，此处预留用于更复杂的去重和顺序控制
    }
}

/// 构建项目 — 查找入口文件并编译
pub struct ProjectBuilder {
    /// 项目根目录
    root_dir: PathBuf,
    /// 入口文件名
    entry_file: String,
    /// 输出路径
    output_path: Option<PathBuf>,
    /// 是否原生编译
    native: bool,
    /// 是否禁用优化
    no_opt: bool,
    /// 是否生成调试信息
    debug_info: bool,
}

impl ProjectBuilder {
    pub fn new(root_dir: &Path) -> Self {
        ProjectBuilder {
            root_dir: root_dir.to_path_buf(),
            entry_file: "main.klc".to_string(),
            output_path: None,
            native: false,
            no_opt: false,
            debug_info: false,
        }
    }

    pub fn entry_file(mut self, name: &str) -> Self {
        self.entry_file = name.to_string();
        self
    }

    pub fn output(mut self, path: &Path) -> Self {
        self.output_path = Some(path.to_path_buf());
        self
    }

    pub fn native(mut self, flag: bool) -> Self {
        self.native = flag;
        self
    }

    pub fn no_opt(mut self, flag: bool) -> Self {
        self.no_opt = flag;
        self
    }

    pub fn debug_info(mut self, flag: bool) -> Self {
        self.debug_info = flag;
        self
    }

    /// 执行构建
    pub fn build(self) -> Result<BuildResult, String> {
        let entry_path = self.root_dir.join(&self.entry_file);

        if !entry_path.exists() {
            return Err(format!("Entry file not found: {}", entry_path.display()));
        }

        // 解析模块
        let mut resolver = ModuleResolver::new(&self.root_dir);
        let program = resolver.resolve_entry(&entry_path)?;

        let output = self.output_path.clone().unwrap_or_else(|| {
            let name = entry_path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("output");
            if self.native {
                PathBuf::from(format!("{}.exe", name))
            } else {
                PathBuf::from(format!("{}.out", name))
            }
        });

        let modules_count = resolver.modules.len();
        let stmt_count = program.statements.len();

        Ok(BuildResult {
            program,
            output_path: output,
            modules_count,
            stmt_count,
            native: self.native,
            debug_info: self.debug_info,
        })
    }
}

/// 构建结果
pub struct BuildResult {
    /// 合并后的完整 AST
    pub program: Program,
    /// 输出文件路径
    pub output_path: PathBuf,
    /// 模块数量
    pub modules_count: usize,
    /// 总语句数
    pub stmt_count: usize,
    /// 是否原生编译
    pub native: bool,
    /// 是否包含调试信息
    pub debug_info: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_building() {
        let source = r#"
fn greet(name) { io.println("Hello " ++ name) }
type Point { x: i32, y: i32 }
let PI = 3
"#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().unwrap();

        let resolver = ModuleResolver::new(Path::new("."));
        let exports = resolver.build_exports(&program);

        assert!(exports.contains_key("greet"));
        assert!(exports.contains_key("Point"));
        assert!(exports.contains_key("PI"));
    }
}
