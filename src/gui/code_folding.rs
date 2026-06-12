//! KLC IDE Beta — 代码折叠模块
//!
//! 基于大括号分析，支持折叠/展开代码块。
//! 使用左边距区域的 +/- 标记实现。

#![allow(non_snake_case)]

/// 折叠区块信息
#[derive(Debug, Clone)]
pub struct FoldBlock {
    /// 区块开始行号 (0-based)
    pub start_line: usize,
    /// 区块结束行号 (0-based)
    pub end_line: usize,
    /// 是否已折叠
    pub folded: bool,
}

/// 分析源代码，找出可折叠的代码块
pub fn analyze_folds(source: &str) -> Vec<FoldBlock> {
    let lines: Vec<&str> = source.lines().collect();
    let mut blocks = Vec::new();
    let mut stack: Vec<(usize, usize)> = Vec::new(); // (line_index, brace_count)

    let mut line_idx = 0usize;
    for line in &lines {
        let mut open = 0usize;
        let mut close = 0usize;
        for ch in line.chars() {
            match ch {
                '{' => open += 1,
                '}' => close += 1,
                _ => {}
            }
        }

        // 遇到 { 压栈
        for _ in 0..open {
            stack.push((line_idx, 1));
        }

        // 遇到 } 弹栈
        for _ in 0..close {
            if let Some((start, _)) = stack.pop() {
                if line_idx > start + 1 { // 至少跨 2 行才有意义
                    blocks.push(FoldBlock {
                        start_line: start,
                        end_line: line_idx,
                        folded: false,
                    });
                }
            }
        }

        line_idx += 1;
    }

    // 合并嵌套折叠（保留最外层）
    blocks.sort_by_key(|b| (b.start_line, -(b.end_line as i64)));
    let mut result: Vec<FoldBlock> = Vec::new();
    for b in blocks {
        let is_contained = result.iter().any(|r|
            b.start_line >= r.start_line && b.end_line <= r.end_line
        );
        if !is_contained {
            result.push(b);
        }
    }

    result
}

/// 折叠指定行范围：用 `... (N lines folded) ...` 替换
pub fn fold_source(source: &str, blocks: &[FoldBlock]) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let mut result_lines: Vec<String> = Vec::new();
    let mut skip_until: isize = -1;

    for (idx, line) in lines.iter().enumerate() {
        if (skip_until as usize) > idx { continue; }

        // 检查是否需要在这里折叠
        let mut folded_block: Option<&FoldBlock> = None;
        for b in blocks {
            if b.folded && b.start_line == idx {
                folded_block = Some(b);
                break;
            }
        }

        if let Some(b) = folded_block {
            let indent = line.chars().take_while(|c| *c == ' ' || *c == '\t').count();
            let padding = " ".repeat(indent);
            let folded_count = b.end_line - b.start_line - 1;
            result_lines.push(format!("{}{{  // [folded: {} lines]", padding, folded_count));
            // 添加结束行
            if b.end_line < lines.len() {
                result_lines.push(lines[b.end_line].to_string());
            }
            skip_until = b.end_line as isize;
        } else {
            result_lines.push(line.to_string());
        }
    }

    result_lines.join("\r\n")
}

/// 切换折叠状态
pub fn toggle_fold(blocks: &mut [FoldBlock], line: usize) -> bool {
    for b in blocks.iter_mut() {
        if line >= b.start_line && line <= b.end_line {
            b.folded = !b.folded;
            return true;
        }
    }
    false
}
