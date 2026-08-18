//! # maxima-iir-compiler
//!
//! Maxima source → [`interpreter_ir::IIRModule`], **v0.1.0** — the next
//! item in [`macsyma-iir-vm.md`](../../../specs/macsyma-iir-vm.md)'s
//! Wave 5 rollout, right after `macsyma-iir-compiler` itself.
//!
//! Maxima and Macsyma share the *exact same* algebraic surface (see
//! [`coding_adventures_maxima_runtime`]'s own doc comment: "A program
//! written for one runs on the other"), and `maxima-to-semantic-ir`
//! already established the precedent on the Semantic-IR side: **zero**
//! surface normalization needed, so that crate is a direct re-export of
//! `macsyma-to-semantic-ir`'s public API with no shim function. This
//! crate does the identical thing one layer over, for IIR: a direct
//! re-export of [`macsyma_iir_compiler`]'s public API under Maxima's own
//! name.
//!
//! ## No `maxima-vm`
//!
//! Unlike every other language in this rollout, Maxima gets **no**
//! dedicated VM crate. `macsyma_vm::run` executes any `IIRModule`
//! regardless of the module's `language` field — nothing in its dispatch
//! loop is Macsyma-specific — and Maxima has never had any runtime code
//! of its own anywhere in this repo (`maxima-runtime` is itself a thin
//! façade over `macsyma_runtime::MacsymaSession`). Adding a `maxima-vm`
//! crate that was a byte-for-byte copy of `macsyma-vm` would be pure
//! duplication with no offsetting benefit, unlike the *other* five
//! languages in this rollout (Derive/Reduce/Maple/Axiom/Wolfram), which
//! each get their own dedicated VM per this rollout's explicit
//! VM-sharing decision (`macsyma-iir-vm.md` §6) — that decision was
//! about genuinely *different* languages eventually diverging, not about
//! an alias language with no surface of its own to diverge.
//!
//! ## Pipeline
//!
//! ```text
//! Maxima source
//!    │
//!    ▼  macsyma_iir_compiler::compile_source   (no shim -- same surface)
//! interpreter_ir::IIRModule                    (v0 subset)
//!    │
//!    ▼  macsyma_vm::run
//! dynval_runtime::LispyValue
//! ```
//!
//! ## Public API
//!
//! ```
//! use maxima_iir_compiler::compile_source;
//! let module = compile_source("2 + 3$\n", "demo").unwrap();
//! let value = macsyma_vm::run(&module).unwrap();
//! assert_eq!(value.as_int(), Some(5));
//! ```
//!
//! `compile` (taking an already-parsed `GrammarASTNode`) is re-exported
//! too, for symmetry with `macsyma-iir-compiler` and every other
//! `-iir-compiler` frontend's own `compile`/`compile_source` pair — there
//! is no Maxima-specific CST; the only tree ever built is the Macsyma one
//! `macsyma_iir_compiler::compile`/`compile_source` construct directly.
//!
//! ## Scope
//!
//! Identical to `macsyma-iir-compiler` v0's own scope (see that crate's
//! `src/lower.rs` module doc comment for the full accepted/rejected
//! construct list) — there is nothing Maxima-specific to add or
//! restrict, since the surface is unchanged.

pub use macsyma_iir_compiler::{compile, compile_source, MacsymaIirError};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_arithmetic_compiles_and_runs() {
        let module = compile_source("2 + 3$\n", "demo").unwrap();
        let value = macsyma_vm::run(&module).unwrap();
        assert_eq!(value.as_int(), Some(5));
    }

    #[test]
    fn a_maxima_style_display_terminated_statement_compiles() {
        // `;` (display) vs `$` (suppress) is the same Macsyma terminator
        // pair Maxima itself uses unchanged -- no rewriting needed.
        let module = compile_source("x: 3;\n", "demo").unwrap();
        let value = macsyma_vm::run(&module).unwrap();
        assert_eq!(value.as_int(), Some(3));
    }

    #[test]
    fn out_of_scope_construct_is_rejected_identically_to_macsyma() {
        assert!(compile_source("1.5$\n", "demo").is_err());
    }

    #[test]
    fn a_malformed_program_errors_cleanly_not_panicking() {
        assert!(compile_source("(((\n", "demo").is_err());
    }

    #[test]
    fn compile_and_compile_source_are_both_reexported() {
        // Symmetry check: both entry points must exist under this crate's
        // own name, mirroring `maxima-to-semantic-ir`'s identical
        // `compile`/`compile_source` symmetry test.
        let tree = coding_adventures_macsyma_parser::create_macsyma_parser("1$\n")
            .parse()
            .expect("parse should succeed");
        let module = compile(&tree, "demo").expect("compile should succeed");
        let value = macsyma_vm::run(&module).unwrap();
        assert_eq!(value.as_int(), Some(1));
    }
}
