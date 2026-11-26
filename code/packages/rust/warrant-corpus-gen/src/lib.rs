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
//!   WarrantFile ── lower ──▶ ir.core IR ── cost ──▶ ir_cost, wraps rules      <- S3a
//!          │                      │
//!          │                      ├─ evaluate, enumerate, digest  ──────────── S3b (this PR)
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
//! ## What the crate contains
//!
//! | module | slice | what it is |
//! |---|---|---|
//! | [`json`] | S3a | the VER04i 9.1 canonical writer (no `serde`) |
//! | [`profile`] | S3a | the strict `corpus.profile` reader (VER04r 8.4) |
//! | [`types`] | S3a | the small type language of the IR (S11b: `Option`, records and variants, the width in the type) and its unification rules |
//! | [`ir`] | S3a | the 26 `ir.core` tags (Appendix A), `ir.fixed` nodes (S10b), the `ir.tuple` shapes (S10t-2), the four `ir.data` tags and ten patterns (S11b) and their JSON form |
//! | [`patterns`] | S10t-2 | the six `match` patterns, the usefulness check and the corpus errors (`ir`) |
//! | [`lower`] | S3a | syntax-directed lowering with the annotating typer; level refusals |
//! | [`analysis`] | S3a | the pure-guard rule (4.4) and the v0 static arithmetic guard (3.3 rule 3) |
//! | [`arith`] | S3a | the single wrapping rule (3.2): `wraps`, the 2^127 window, `MAX_IR_VALUE` |
//! | [`cost`] | S3a | worst-path `ir_cost`, `max_ir_cost`, total IR work (4.2 rule 6) |
//! | [`view`] | S3a | the lowering parity view compared byte for byte with the oracle |
//! | [`value`], [`eval`] | S3b | the evaluator (`ir.core`, S10t-2 `ir.tuple`, S10c-2 `ir.fixed`, S11c-2 `ir.data`): values, wraps, the pure trace for `expected_calls` |
//! | [`fault_types`] | S11c-2 | the typing of every fault row (R67, R94, R96 to R98) |
//! | [`domains`] | S3b | binder descriptors and labels over the S2 `domain` API (`nat_shift`, `fx_edge` since S10c-2); the one 16.2 refusal left; the consumer-side `descriptor_size` |
//! | [`stream`] | S3b | the 2.4 stream encoders, framing, FNV-1a-64, the slim base set |
//! | [`suite`] | S3b | product enumeration, admission, chunks, counts and digests of one suite |
//! | [`enumerate`] | S3b | a whole document at `full` depth; the enumeration parity view |
//! | [`census`] | S3b | the per-claim census of what the generator can emit (VER04r 16.2) |
//!
//! [`analyse`] runs the S3a pipeline on a parsed document and [`enumerate_document`] the S3b
//! one (it runs [`analyse`] first).  [`generate`] is the entry point S3c to S3d fill in, and
//! until they do it **fails closed**: it validates what it can and then refuses with
//! `code == "unimplemented"` rather than emit a corpus whose faults, controls, witness
//! chunks and files do not exist yet.
//!
//! ## Fail closed
//!
//! The generator computes a digest for every suite it emits, so it cannot emit a suite it
//! cannot evaluate.  A construct beyond what it evaluates (`ir.core`, `ir.tuple`, `ir.fixed` since S10c-2 and, since
//! S11c-2, `ir.data`) refuses the **whole document**, naming the level or feature (`ir.list`, `ir.seq`, `fixed-cast`,
//! `err-value`, `inout`, ...), and nothing is written.  `ir.data` was **lowered, typed and costed** by S11b and is
//! evaluated, written (stream tags `40 41 64 65`, TFV `L0 L1 L2`) and fault-tested since S11c-2 (VER04r 16.14 R37), which
//! removed the guard S11b kept.
//! Every `match` over the grammar ends in an explicit refusal, never a skip.
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
pub mod census;
pub mod cli;
pub mod cost;
pub mod domains;
pub mod emit;
pub mod enumerate;
pub mod error;
pub mod eval;
pub mod fault_types;
pub mod faults;
pub mod fsio;
pub mod ir;
pub mod json;
pub mod lower;
pub mod patterns;
pub mod profile;
pub mod stream;
pub mod suite;
pub mod tfv;
pub mod types;
pub mod value;
pub mod view;

pub use enumerate::{enumerate_document, Enumeration};
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
    let lowered = lower::lower_document_omitting(file, &profile.omit_examples)?;
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

/// The three kinds of file of a fixture that the generator owns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpusV2 {
    /// `corpus.json`, canonical.
    pub corpus_json: String,
    /// `chunks/<file>.txt` as `(file name, contents)`.
    pub chunk_files: Vec<(String, String)>,
    /// `cases.tfv`.
    pub cases_tfv: String,
}

impl CorpusV2 {
    /// Every file as `(path relative to the fixture directory, contents)`: `corpus.json`,
    /// `cases.tfv`, then `chunks/<file>` in suite order.
    pub fn files(&self) -> Vec<(String, &str)> {
        let mut v = vec![
            ("corpus.json".to_string(), self.corpus_json.as_str()),
            ("cases.tfv".to_string(), self.cases_tfv.as_str()),
        ];
        v.extend(
            self.chunk_files
                .iter()
                .map(|(n, c)| (format!("chunks/{n}"), c.as_str())),
        );
        v
    }
}

/// The inputs of [`generate`].
#[derive(Debug, Clone, Copy)]
pub struct GenInput<'a> {
    /// The parsed document.
    pub file: &'a WarrantFile<'a>,
    /// The specification's path as written into `corpus.json` (`spec`), repo-relative.
    pub spec_path: &'a str,
    /// The specification's raw bytes (`spec_sha256`).
    pub spec_bytes: &'a [u8],
    /// The raw `corpus.profile` bytes (read strictly by [`profile::parse_profile`]).
    pub profile_bytes: &'a [u8],
    /// The Rust bindings' `faults=` lists, to cross-check the profile against (VER04r 8.4).
    pub binding_faults: Option<&'a [(String, Vec<String>)]>,
}

pub use emit::{generate, generate_gated};
