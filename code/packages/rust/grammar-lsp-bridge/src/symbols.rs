//! Extract document symbols and build the declaration table.
//!
//! ## Implementation plan (LS02 PR A)
//!
//! ### document_symbols()
//!
//! Walk top-level children of the GrammarASTNode root.
//! For each child whose rule_name is in spec.declaration_rules:
//!
//! 1. Extract the first NAME token child → declaration name.
//!
//! 2. Infer SymbolKind:
//!    Heuristic: if any subsequent child node has a rule_name that looks
//!    like a parameter list ("params", "param_list", "typed_param",
//!    "args", "parameters") OR if any child is an LPAREN token followed
//!    by NAME tokens → SymbolKind::Function.
//!    Otherwise → SymbolKind::Variable.
//!
//!    For Twig specifically: if the define's body contains a lambda child
//!    → Function; else → Variable. But keep this generic.
//!
//! 3. Collect start/end lines from the AST node's position fields.
//!    PREREQUISITE: GrammarASTNode must carry position info.
//!    If it doesn't, file an issue on grammar-tools first.
//!
//! 4. Return Vec<ls00::DocumentSymbol>.
//!
//! ### DeclarationTable (used by hover + completion)
//!
//! A HashMap<String, DeclEntry> where:
//!   DeclEntry { kind: SymbolKind, start_line: u32, start_col: u32 }
//!
//! Built from the same walk as document_symbols(). Cache it on the bridge
//! (keyed by document URI + version) to avoid rebuilding on every hover.

use crate::LanguageSpec;

/// A resolved declaration: name → (kind, position).
///
/// Built by [`build_declaration_table`] and used by hover + completion.
#[derive(Debug, Clone)]
pub struct DeclEntry {
    /// Whether the declaration is a function or a value.
    pub is_function: bool,
    /// 0-based line of the declaration in the source.
    pub line: u32,
    /// 0-based column of the declaration in the source.
    pub column: u32,
}

/// Declaration table: maps declared names to their entry.
pub type DeclarationTable = std::collections::HashMap<String, DeclEntry>;

/// Build the declaration table from the parsed AST.
///
/// ## TODO — implement (LS02 PR A)
pub fn build_declaration_table(
    spec: &'static LanguageSpec,
    ast: &(),  // TODO: replace () with &GrammarASTNode
) -> DeclarationTable {
    let _ = (spec, ast);
    DeclarationTable::new()
}

/// Extract document symbols (outline panel) from the parsed AST.
///
/// ## TODO — implement (LS02 PR A)
pub fn document_symbols(
    spec: &'static LanguageSpec,
    ast: &(),  // TODO: replace () with &GrammarASTNode
) -> Vec<()> {   // TODO: replace () with ls00::DocumentSymbol
    let _ = (spec, ast);
    vec![]
}
