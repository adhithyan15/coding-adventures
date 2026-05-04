//! Emit LSP semantic tokens from the token stream.
//!
//! ## Implementation plan (LS02 PR A)
//!
//! Algorithm (token-stream pass only — no AST needed):
//!
//! 1. Receive the Vec<ls00::Token> from tokenize::run().
//!
//! 2. For each token:
//!    a. Look up token.kind (Option<String>) in spec.token_kind_map.
//!    b. If found → emit an LspSemanticToken with the mapped type.
//!    c. If not found → skip (punctuation, whitespace have no semantic colour).
//!
//! 3. LSP semantic tokens use *delta* encoding (each token's position is
//!    relative to the previous one). Compute deltas as you walk the list.
//!
//! 4. Return Vec<ls00::SemanticToken>.
//!
//! ## Note on Function vs Variable classification
//!
//! The raw token pass emits Variable for all NAME tokens. Function names
//! get reclassified in document_symbols / hover passes (which know from
//! the declaration table whether a name binds a lambda). This is fine —
//! editors that support document_symbols will use the richer classification.
//! Raw semantic tokens remain conservative (Variable for all names).

use crate::LanguageSpec;

/// Emit semantic tokens for `tokens` using `spec.token_kind_map`.
///
/// ## TODO — implement (LS02 PR A)
pub fn run(spec: &'static LanguageSpec, tokens: &[()]) -> Vec<()> {
    let _ = (spec, tokens);
    vec![]
}
