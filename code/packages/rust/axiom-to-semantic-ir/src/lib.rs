//! # axiom-to-semantic-ir
//!
//! Axiom CST → narrow-waist Semantic IR, **v0.1.0** (MA-13e).
//!
//! This is the **sixth** frontend to target
//! [SIR23](../../../specs/SIR23-symbolic-pattern-semantic-ir.md), the
//! symbolic-expression/pattern-matching domain extension of the SIR10
//! narrow-waist IR (Stream B of
//! [`HML01`](../../../specs/HML01-math-to-semantic-ir.md)) — sibling to
//! `wolfram-to-semantic-ir`, `macsyma-to-semantic-ir`, `derive-to-semantic-ir`,
//! `reduce-to-semantic-ir`, and `maple-to-semantic-ir`. It consumes the
//! generic `GrammarASTNode` CST produced by
//! `coding-adventures-axiom-parser` and emits a [`semantic_ir::Module`]. See
//! `lower.rs`'s module doc comment for the full scope, the central design
//! decision for `:`/`::`/`has` (this crate's own genuinely new territory
//! relative to every prior SIR23 frontend), and every other node-by-node
//! mapping.
//!
//! This is the **last** item in Axiom's native (non-oracle-tested) pipeline
//! per [`MA13`](../../../specs/MA13-axiom-language.md) §6 — the four prior
//! merged PRs shipped `axiom-lexer` (MA-13b), `axiom-parser` (MA-13c), and
//! `axiom-runtime`/`axiom-repl` (MA-13d). Oracle/golden testing (native
//! `axiom-runtime` vs. this crate → `semantic-ir-to-javascript` → `node`) is
//! an explicitly separate follow-on task, not part of this crate.
//!
//! ## Pipeline
//!
//! ```text
//! Axiom source
//!    │
//!    ▼  coding_adventures_axiom_parser::try_parse_axiom(src)
//! parser::grammar_parser::GrammarASTNode   (generic CST, rooted at `program`
//!                                            -- exactly ONE expression, see
//!                                            lower.rs's module doc comment)
//!    │
//!    ▼  axiom_to_semantic_ir::compile
//! semantic_ir::Module                      (per SIR10 + SIR23)
//! ```
//!
//! ## Public API
//!
//! ```
//! use coding_adventures_axiom_to_semantic_ir::compile_source;
//! let module = compile_source("1 + 2", "demo").unwrap();
//! assert!(module.functions.iter().any(|f| f.name == "main"));
//! ```

mod lower;
pub use lower::{compile, AxiomLowerError, AXIOM_COERCE, AXIOM_DECLARE, AXIOM_HAS, COMPOUND_EXPRESSION};

/// Parse `source` as Axiom and lower it into a [`semantic_ir::Module`] in one
/// step, mirroring every other `-to-semantic-ir` frontend's `compile_source`
/// convenience wrapper.
///
/// `source` is expected to be exactly ONE Axiom expression/statement (see
/// `lower.rs`'s module doc comment for why `axiom.grammar`'s own `program`
/// rule is scoped this way) — matching a single line submitted to
/// `axiom-repl`'s own numbered prompt, not a multi-line worksheet file the
/// way `maple-to-semantic-ir::compile_source`/`reduce-to-semantic-ir::
/// compile_source` accept.
///
/// Like `maple-to-semantic-ir::compile_source`/`derive-to-semantic-ir::
/// compile_source`/`reduce-to-semantic-ir::compile_source` (and unlike
/// `wolfram-to-semantic-ir::compile_source`, which spawns an enlarged-stack
/// worker thread because Wolfram's own precedence cascade makes its parser's
/// `MAX_RULE_DEPTH` unsafe on a bare stack), this function needs no worker
/// thread at all: `coding_adventures_axiom_parser`'s own `MAX_RULE_DEPTH`
/// (140) is already documented — see that crate's `src/lib.rs` doc comment —
/// as safe on a bare default (~2 MiB) stack with comfortable margin (its own
/// doc comment measures four independent recursion shapes in *rule-frame*
/// terms and finds the binding constraint is nested function calls' floor of
/// 211 rule frames — 140 sits about 33.6% below that floor). Its own test
/// suite directly confirms input thousands of levels past the cap still
/// fails cleanly with a `Result::Err` on a bare default-stack thread, never a
/// crash. So this crate mirrors `maple-to-semantic-ir`'s (and
/// `derive-to-semantic-ir`'s / `reduce-to-semantic-ir`'s) simple,
/// worker-thread-free `compile_source` shape rather than
/// `wolfram-to-semantic-ir`'s.
pub fn compile_source(
    source: &str,
    module_name: &str,
) -> Result<semantic_ir::Module, AxiomLowerError> {
    let tree = coding_adventures_axiom_parser::try_parse_axiom(source).map_err(|msg| AxiomLowerError {
        message: format!("parse error: {msg}"),
        line: 1,
        column: 1,
    })?;
    compile(&tree, module_name)
}
