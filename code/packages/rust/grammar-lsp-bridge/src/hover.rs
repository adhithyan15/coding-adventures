//! Hover info at a cursor position.
//!
//! ## Implementation plan (LS02 PR A)
//!
//! 1. Walk the AST to find the token at (line, column).
//!    Prefer the token whose range contains the cursor.
//!    If the token is a NAME:
//!
//! 2. Check spec.keyword_names first → return "keyword: <name>" hover.
//!
//! 3. Check the DeclarationTable (from symbols::build_declaration_table):
//!    - If found and is_function → return "function: <name> [line N]" hover.
//!    - If found and !is_function → return "variable: <name> [line N]" hover.
//!
//! 4. If not found → return "unresolved: <name>" hover (dim, informational).
//!
//! 5. For non-NAME tokens (INTEGER, BOOL, etc.) → return type label hover
//!    based on the token kind in spec.token_kind_map.
//!
//! 6. Return Option<ls00::HoverResult>.
//!    None if no meaningful info at cursor.

use crate::LanguageSpec;
use crate::symbols::DeclarationTable;

/// Hover info for the symbol at `(line, col)`.
///
/// ## TODO — implement (LS02 PR A)
pub fn hover(
    spec: &'static LanguageSpec,
    ast: &(),              // TODO: replace with &GrammarASTNode
    decl_table: &DeclarationTable,
    line: u32,
    col: u32,
) -> Option<()> {          // TODO: replace with ls00::HoverResult
    let _ = (spec, ast, decl_table, line, col);
    None
}
