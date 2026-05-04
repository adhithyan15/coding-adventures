//! Completion items at a cursor position.
//!
//! ## Implementation plan (LS02 PR A)
//!
//! 1. Collect keyword completions from spec.keyword_names:
//!    each → CompletionItem { label: name, kind: Keyword, detail: "keyword" }.
//!
//! 2. Collect user-defined symbol completions from the DeclarationTable:
//!    each → CompletionItem {
//!      label: name,
//!      kind: Function | Variable (based on DeclEntry.is_function),
//!      detail: "defined on line N",
//!    }
//!
//! 3. Sort: keywords first (alphabetical), then user symbols (alphabetical).
//!
//! 4. ls00 filters by the prefix the user has typed — we return all items
//!    and the framework does the filtering.
//!
//! 5. Return Vec<ls00::CompletionItem>.
//!
//! Note: scope-aware completion (only names visible at cursor) is Phase 2.
//! Phase 1 returns all top-level declarations regardless of cursor position.

use crate::LanguageSpec;
use crate::symbols::DeclarationTable;

/// Completion items for the document.
///
/// ## TODO — implement (LS02 PR A)
pub fn completion_items(
    spec: &'static LanguageSpec,
    decl_table: &DeclarationTable,
) -> Vec<()> {  // TODO: replace with ls00::CompletionItem
    let _ = (spec, decl_table);
    vec![]
}
