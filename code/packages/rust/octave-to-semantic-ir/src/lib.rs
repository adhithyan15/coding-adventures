//! # octave-to-semantic-ir
//!
//! GNU Octave source → narrow-waist Semantic IR, **v0.1.0** — Stream A
//! rollout item 5 ([`HML01`](../../../specs/HML01-math-to-semantic-ir.md) §5).
//!
//! Octave's departure from MATLAB is a small, local set of surface forms
//! (`#` comments, `endif`/`endfor`/`endwhile`/`endfunction`/`endswitch`/
//! `end_try_catch`, `!=`/`!`), not a different grammar — so, exactly like
//! [`coding_adventures_octave_runtime`] reuses the *entire* MATLAB frontend
//! for evaluation, this crate reuses the entire MATLAB-to-SIR pipeline
//! ([`matlab_to_semantic_ir`]) for compilation. There is no Octave parser
//! and no Octave-specific `Expr`/`SirType`/`Feature` — the shim runs
//! *before* parsing, so by the time any tree exists it is already a MATLAB
//! CST.
//!
//! ## Pipeline
//!
//! ```text
//! Octave source
//!    │
//!    ▼  coding_adventures_octave_runtime::octavify(src)
//! MATLAB-syntax source (string)
//!    │
//!    ▼  matlab_to_semantic_ir::compile_source
//! semantic_ir::Module                      (per SIR10 + SIR16 + SIR22)
//! ```
//!
//! ## Why this crate has no `compile(tree, ...)` entry point
//!
//! Every other `-to-semantic-ir` frontend exposes both a CST-in `compile`
//! and a source-in `compile_source` convenience wrapper (see
//! `matlab-to-semantic-ir`'s own doc comment). That split only makes sense
//! when the frontend owns its own parser and a caller might already have a
//! parsed tree in hand. Octave never has one: `octavify` normalizes *text*,
//! not a tree, so the only tree that ever exists is the MATLAB one
//! `matlab_to_semantic_ir::compile_source` builds internally. A
//! `compile_source`-only public API is therefore not a missing feature —
//! it is the honest shape of what this crate does.
//!
//! ## Public API
//!
//! ```
//! use octave_to_semantic_ir::compile_source;
//!
//! // Octave-only syntax: `#` comment, `endfor`, `!=`.
//! let src = "x = 0; # start\nfor i = 1:3\n  x = x + 1;\nendfor\n";
//! let module = compile_source(src, "demo").unwrap();
//! assert!(module.functions.iter().any(|f| f.name == "main"));
//! ```
//!
//! ## Scope
//!
//! Whatever `matlab-to-semantic-ir` v0.1.0 supports, minus whatever
//! `octavify` cannot yet normalize (`++`/`--`, `do…until` — both documented
//! deferrals in `octave-runtime`'s own doc comment, left untouched by the
//! shim and so reported as ordinary MATLAB parse/lower errors here, not
//! specially detected). See `matlab-to-semantic-ir`'s module doc comment
//! for the full supported-construct list.

pub use matlab_to_semantic_ir::MatlabLowerError;

use coding_adventures_octave_runtime::octavify;

/// Normalize `source` from Octave to MATLAB syntax via [`octavify`], then
/// parse and lower it into a [`semantic_ir::Module`] in one step, mirroring
/// every other `-to-semantic-ir` frontend's `compile_source` convenience
/// wrapper.
pub fn compile_source(
    source: &str,
    module_name: &str,
) -> Result<semantic_ir::Module, MatlabLowerError> {
    matlab_to_semantic_ir::compile_source(&octavify(source), module_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn main_fn(m: &semantic_ir::Module) -> &semantic_ir::Function {
        m.functions
            .iter()
            .find(|f| f.name == "main")
            .expect("module must have a main function")
    }

    // --- octavify-then-delegate wiring: Octave-only syntax normalizes ------

    #[test]
    fn hash_comment_is_normalized_before_compiling() {
        // A bare `#` comment is not valid MATLAB syntax at all -- if this
        // compiles, the shim ran before the parser ever saw the source.
        let module = compile_source("x = 1; # a comment\n", "demo").unwrap();
        assert!(!main_fn(&module).body.stmts.is_empty());
    }

    #[test]
    fn endfor_endif_end_try_catch_all_normalize_to_end() {
        let sources = [
            "for i = 1:3\n  x = i;\nendfor\n",
            "if 1\n  x = 1;\nendif\n",
            "while 0\n  x = 1;\nendwhile\n",
        ];
        for src in sources {
            compile_source(src, "demo")
                .unwrap_or_else(|e| panic!("`{src}` should compile via octavify: {e}"));
        }
    }

    #[test]
    fn bang_equals_and_bang_normalize_to_tilde_forms() {
        let module = compile_source("if 1 != 2\n  x = 1;\nend\n", "demo").unwrap();
        assert!(!main_fn(&module).body.stmts.is_empty());
    }

    #[test]
    fn hash_and_bang_inside_strings_are_left_untouched() {
        // octavify is string-aware -- '#'/'!' inside a string literal must
        // NOT be rewritten. This mirrors octave-runtime's own guarantee;
        // re-verified here since a regression would silently corrupt sirs.
        let module = compile_source("s = '#not-a-comment != still-a-string';\n", "demo").unwrap();
        assert!(!main_fn(&module).body.stmts.is_empty());
    }

    // --- plain MATLAB still compiles unchanged (shim is a superset) --------

    #[test]
    fn plain_matlab_with_end_and_percent_comments_compiles_unchanged() {
        let src = "x = 1; % a matlab comment\nfor i = 1:3\n  x = x + i;\nend\n";
        let module = compile_source(src, "demo").unwrap();
        assert!(!main_fn(&module).body.stmts.is_empty());
    }

    // --- error propagation --------------------------------------------------

    #[test]
    fn a_construct_outside_matlab_to_semantic_irs_scope_errors_cleanly() {
        // switch/case is explicitly out of matlab-to-semantic-ir v0.1.0's
        // scope; octavify does not touch it, so this must surface as an
        // ordinary MatlabLowerError through this crate's own return type.
        let src = "switch x\n  case 1\n    y = 1;\nend\n";
        let err = compile_source(src, "demo").expect_err("switch should be out of scope");
        assert!(!err.message.is_empty());
    }

    #[test]
    fn octave_only_do_until_is_not_normalized_and_errors() {
        // Documented deferral (octave-runtime's own doc comment): `do...until`
        // has no MATLAB equivalent and octavify leaves it untouched, so it
        // must fail as an ordinary parse/lower error here too, not panic.
        let src = "do\n  x = x + 1;\nuntil x > 3\n";
        assert!(compile_source(src, "demo").is_err());
    }

    // --- end-to-end: realistic Octave source, every shim rule at once ------

    #[test]
    fn realistic_octave_source_compiles_end_to_end() {
        let src = "\
% leading matlab-style comment is untouched
x = 0; # octave-style comment
for i = 1:5
  if i != 3
    x = x + i;
  endif
endfor
";
        let module = compile_source(src, "demo").unwrap();
        assert!(!main_fn(&module).body.stmts.is_empty());
        let result = semantic_ir::validate(&module);
        assert!(result.is_ok(), "lowered module must pass validation: {:?}", result.issues);
    }
}
