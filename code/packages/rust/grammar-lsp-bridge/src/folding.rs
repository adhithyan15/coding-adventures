//! Extract folding ranges from the AST.
//!
//! ## Implementation plan (LS02 PR A)
//!
//! Walk every GrammarASTNode recursively (depth-first).
//! For each node where node.end_line > node.start_line:
//!   emit FoldingRange { start_line: node.start_line, end_line: node.end_line, kind: Region }.
//!
//! This requires position info on GrammarASTNode (start_line, end_line, start_col, end_col).
//! PREREQUISITE: verify this exists in grammar-tools before implementing.
//! File: code/packages/rust/grammar-tools/src/lib.rs
//!
//! No per-language configuration needed — any multi-line compound node is foldable.

use crate::LanguageSpec;

/// Extract folding ranges from the parsed AST.
///
/// ## TODO — implement (LS02 PR A)
pub fn folding_ranges(
    spec: &'static LanguageSpec,
    ast: &(),  // TODO: replace with &GrammarASTNode
) -> Vec<()> {   // TODO: replace with ls00::FoldingRange
    let _ = (spec, ast);
    vec![]
}
