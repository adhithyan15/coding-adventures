//! # maxima-to-semantic-ir
//!
//! Maxima source → narrow-waist Semantic IR, **v0.1.0** — the next item in
//! [`HML01`](../../../specs/HML01-math-to-semantic-ir.md) Stream B's rollout,
//! right after `macsyma-to-semantic-ir`.
//!
//! Maxima and Macsyma share the *exact same* algebraic surface — see
//! [`coding_adventures_maxima_runtime`]'s own doc comment: "A program
//! written for one runs on the other." This is a **stronger** guarantee
//! than Octave's relationship to MATLAB (which needs `octave-runtime`'s
//! `octavify` source-rewriting shim for a small set of surface departures —
//! `#` comments, `endif`/`endfor`/…, `!=`/`!`): Maxima needs **zero**
//! surface normalization at all. So where `octave-to-semantic-ir` is a thin
//! wrapper (shim, then delegate), this crate is thinner still — a direct
//! re-export of [`macsyma_to_semantic_ir`]'s public API under Maxima's own
//! name, with no shim function in between at all.
//!
//! ## Pipeline
//!
//! ```text
//! Maxima source
//!    │
//!    ▼  macsyma_to_semantic_ir::compile_source   (no shim -- same surface)
//! semantic_ir::Module                            (per SIR10 + SIR23)
//! ```
//!
//! ## Public API
//!
//! ```
//! use maxima_to_semantic_ir::compile_source;
//! let module = compile_source("diff(x^3, x)$\n", "demo").unwrap();
//! assert!(module.functions.iter().any(|f| f.name == "main"));
//! ```
//!
//! `compile` (taking an already-parsed `GrammarASTNode`) is re-exported
//! too, for symmetry with `macsyma-to-semantic-ir` and every other
//! `-to-semantic-ir` frontend's own `compile`/`compile_source` pair — there
//! is no Maxima-specific CST; the only tree ever built is the Macsyma one
//! `macsyma_to_semantic_ir::compile`/`compile_source` construct directly.
//!
//! ## Scope
//!
//! Identical to `macsyma-to-semantic-ir` v0.1.0's own scope (see that
//! crate's module doc comment for the full node-by-node mapping and the
//! disclosed "no pattern-matching/rewrite-rule syntax" boundary) — there is
//! nothing Maxima-specific to add or restrict, since the surface is
//! unchanged.

pub use macsyma_to_semantic_ir::{compile, compile_source, MacsymaLowerError};

#[cfg(test)]
mod tests {
    use super::*;

    fn main_fn(m: &semantic_ir::Module) -> &semantic_ir::Function {
        m.functions
            .iter()
            .find(|f| f.name == "main")
            .expect("module must have a main function")
    }

    #[test]
    fn bare_arithmetic_compiles_unchanged() {
        let module = compile_source("1 + 2$\n", "demo").unwrap();
        assert!(!main_fn(&module).body.stmts.is_empty());
    }

    #[test]
    fn a_maxima_style_display_terminated_statement_compiles() {
        // `;` (display) vs `$` (suppress) is the same Macsyma terminator
        // pair Maxima itself uses unchanged -- no rewriting needed.
        let module = compile_source("x: 3;\n", "demo").unwrap();
        assert!(!main_fn(&module).body.stmts.is_empty());
    }

    #[test]
    fn diff_and_integrate_builtins_bridge_to_canonical_heads() {
        let module = compile_source("diff(x^3, x)$\nintegrate(x, x)$\n", "demo").unwrap();
        assert_eq!(main_fn(&module).body.stmts.len(), 2);
    }

    #[test]
    fn control_flow_compiles_and_the_lowered_module_validates() {
        let src = "block([x: 0], for i: 1 thru 5 do x: x + i, x)$\n";
        let module = compile_source(src, "demo").unwrap();
        let report = semantic_ir::validate(&module);
        assert!(
            report.is_ok(),
            "lowered module must pass validation: {:?}",
            report.issues
        );
    }

    #[test]
    fn a_malformed_program_errors_cleanly_not_panicking() {
        assert!(compile_source("(((\n", "demo").is_err());
    }

    #[test]
    fn compile_and_compile_source_are_both_reexported() {
        // Symmetry check: both entry points must exist under this crate's
        // own name, mirroring every other `-to-semantic-ir` frontend's
        // `compile`/`compile_source` pair (unlike `octave-to-semantic-ir`,
        // which deliberately has no `compile(tree, ...)` since its shim
        // rewrites text, not a tree -- Maxima has no shim at all, so the
        // full pair applies here).
        let tree = coding_adventures_macsyma_parser::create_macsyma_parser("1$\n")
            .parse()
            .expect("parse should succeed");
        let module = compile(&tree, "demo").expect("compile should succeed");
        assert!(main_fn(&module).body.stmts.len() == 1);
    }
}
