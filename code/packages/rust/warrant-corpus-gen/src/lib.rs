//! # `coding_adventures_warrant_corpus_gen`: the Warrant corpus v2 emitter
//!
//! This crate is **slice S3** of [VER04r](../../../specs/VER04r-warrant-corpus-v2.md): the
//! generator that turns a `.warrant` specification into a `warrant-corpus-v2` fixture.  It is
//! a new *leaf* crate: it depends on the S2 public API of `warrant-grammar` and on nothing
//! that depends on it, so a change here rebuilds no shared crate.
//!
//! ## Where it sits
//!
//! ```text
//!   code/specs/<pkg>.warrant
//!          │  parse + check          (warrant-grammar, S2 public API)
//!          ▼
//!   WarrantFile ── lower ──▶ ir.core IR ── cost ──▶ ir_cost, wraps rules      <- S3a (this PR)
//!          │                      │
//!          │                      ├─ evaluate, enumerate, digest  ──────────── S3b
//!          │                      ├─ faults, controls, emit, --check ───────── S3c
//!          │                      └─ commit the ct-compare fixture ─────────── S3d
//!          ▼
//!   corpus.json + chunks/*.txt + cases.tfv
//!          │
//!          ├──▶ S4 oracle   (python/warrant-oracle: an independent second reading;
//!          │                 compares, never trusts)
//!          └──▶ lanes       (18 languages run the corpus against their own library)
//! ```
//!
//! ## What this PR (S3a) contains
//!
//! | module | what it is |
//! |---|---|
//! | [`json`] | the VER04i 9.1 canonical writer (no `serde`) |
//! | [`profile`] | the strict `corpus.profile` reader (VER04r 8.4) |
//! | [`types`] | the small type language of `ir.core` and its unification rules |
//! | [`ir`] | the 26 `ir.core` tags (Appendix A) and their JSON form |
//! | [`lower`] | syntax-directed lowering with the annotating typer; level refusals; [`census`] |
//! | [`analysis`] | the pure-guard rule (4.4) and the v0 static arithmetic guard (3.3 rule 3) |
//! | [`arith`] | the single wrapping rule (3.2): `wraps`, the 2^127 window, `MAX_IR_VALUE` |
//! | [`cost`] | worst-path `ir_cost`, `max_ir_cost`, total IR work (4.2 rule 6) |
//! | [`view`] | the parity view compared byte for byte with the independent oracle |
//!
//! [`analyse`] runs the whole S3a pipeline on a parsed document; [`generate`] is the entry
//! point S3b to S3d fill in, and until they do it **fails closed**: it validates what S3a can
//! validate and then refuses with `code == "unimplemented"` rather than emit a corpus it
//! cannot digest.
//!
//! ## Fail closed
//!
//! The generator computes a digest for every suite it emits, so it cannot emit a suite it
//! cannot evaluate.  A construct beyond `ir.core` refuses the **whole document**, naming
//! the level (`ir.fixed`, `ir.data`, `ir.seq`, or a feature such as `inout`), and nothing is
//! written.  Every `match` over the grammar ends in an explicit refusal, never a skip.
//!
//! ## An example
//!
//! ```
//! use coding_adventures_warrant_corpus_gen::{analyse, profile::Profile};
//! use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
//! use coding_adventures_warrant_grammar::parser::{parse_document, ParserLimits};
//! use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};
//!
//! let src = r#"warrant demo
//!   model same(a: Bytes, b: Bytes) -> Bool = a == b
//!   op eq(a: Bytes, b: Bytes) -> Bool
//!   assume platform:
//!     none
//!   assume environment:
//!     none
//!   labels eq:
//!     a.contents secret
//!     a.length public
//!     b.contents secret
//!     b.length public
//!   input eq.a:
//!     hostile
//!     ensures no_panic, terminates
//!   input eq.b:
//!     hostile
//!     ensures no_panic, terminates
//!   bound eq:
//!     steps <= 64 * len(a) + 512
//!     heap <= 2 * len(a)
//!     depth <= 32
//!   claim agrees:
//!     forall a: Bytes, b: Bytes. eq(a, b) == same(a, b)
//! "#;
//! let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).unwrap();
//! let layout = capture_layout(&doc, LayoutLimits::default()).unwrap();
//! let file = parse_document(&layout, &doc, ParserLimits::default()).unwrap();
//!
//! let analysis = analyse(&file, &Profile::default()).unwrap();
//! assert_eq!(analysis.lowered.suites[0].id, "warrant/demo/exhaustive/agrees");
//! // admit 1; body = eq 1 + subject_op 3 + model_call 6 = 10 (the call is 1 + 2 args + the
//! // model body's 3); expected = the model_call, 6.  Total 1 + 10 + 6 = 17.
//! assert_eq!(analysis.suite_costs[0].total, 17);
//! ```

pub mod analysis;
pub mod arith;
pub mod cost;
pub mod error;
pub mod ir;
pub mod json;
pub mod lower;
pub mod profile;
pub mod types;
pub mod view;

pub use error::{code, Beyond, GenError};
pub use lower::{census, lower_document, CensusRow, ItemKind, Lowered};

use coding_adventures_warrant_grammar::parser::WarrantFile;

use cost::{suite_cost, ModelCosts, SuiteCost};
use profile::Profile;

/// Everything S3a computes about a document: the lowered IR and its costs.
#[derive(Debug, Clone)]
pub struct Analysis {
    /// The document lowered to `ir.core`.
    pub lowered: Lowered,
    /// The worst-path cost of every model.
    pub model_costs: ModelCosts,
    /// The cost of every suite, in the order of `lowered.suites`.
    pub suite_costs: Vec<SuiteCost>,
}

/// Lower a parsed document to `ir.core` and cost it against `profile.max_ir_cost`.
///
/// # Errors
///
/// The first refusal: a checker error (`check`), a construct beyond `ir.core` (`level`),
/// a VER04r generation rule (`W0701`), a type error, or a cost limit (`limit`).  Every
/// error is attributed to the claim, model, operation or example it concerns.
pub fn analyse(file: &WarrantFile<'_>, profile: &Profile) -> Result<Analysis, GenError> {
    let lowered = lower_document(file)?;
    let limit = profile.max_ir_cost;
    let model_costs = ModelCosts::compute(&lowered.models, limit)?;
    let mut suite_costs = Vec::with_capacity(lowered.suites.len());
    for s in &lowered.suites {
        suite_costs.push(suite_cost(s, &model_costs, limit)?);
    }
    // Operation refinements and example bodies are expressions too: each is bounded.
    for op in &lowered.ops {
        if let Some(admit) = &op.admit {
            cost::ir_cost(admit, &model_costs, limit).map_err(|e| e.at(&op.name))?;
        }
    }
    for x in &lowered.examples {
        cost::ir_cost(&x.body, &model_costs, limit).map_err(|e| e.at(&x.name))?;
    }
    Ok(Analysis {
        lowered,
        model_costs,
        suite_costs,
    })
}

/// What [`generate`] will return once S3c lands: the three files of a fixture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpusV2 {
    /// `corpus.json`, canonical.
    pub corpus_json: String,
    /// `chunks/<file>.txt` as `(file name, contents)`.
    pub chunk_files: Vec<(String, String)>,
    /// `cases.tfv`.
    pub cases_tfv: String,
}

/// The inputs of [`generate`].
#[derive(Debug, Clone, Copy)]
pub struct GenInput<'a> {
    /// The parsed document.
    pub file: &'a WarrantFile<'a>,
    /// The raw `corpus.profile` bytes (read strictly by [`profile::parse_profile`]).
    pub profile_bytes: &'a [u8],
}

/// Emit a v2 corpus.  **Slice S3a: a fail-closed stub.**  It reads the profile, lowers and
/// costs the document (so every S3a refusal already fires), and then refuses with
/// `code == "unimplemented"`: the evaluator, enumeration and digests (S3b) and the
/// emission (S3c) do not exist yet, and a generator that cannot digest a suite must not
/// emit one.
///
/// # Errors
///
/// Any [`analyse`] refusal; otherwise always `unimplemented`.
pub fn generate(input: &GenInput<'_>) -> Result<CorpusV2, GenError> {
    let profile = profile::parse_profile(input.profile_bytes)?;
    analyse(input.file, &profile)?;
    Err(GenError::new(
        code::UNIMPLEMENTED,
        "",
        "digests and emission arrive in slices S3b and S3c; nothing is written",
    ))
}
