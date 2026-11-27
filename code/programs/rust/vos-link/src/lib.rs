//! # `vos-link`
//!
//! The host tool of the verified-OS document pipeline (backlog VOS-033, spec
//! `code/specs/VOS02-interface-contracts-and-profile-linking.md` sections 2, 6.8, 10 and
//! 10.1).  Crate class **H**: it reads files and writes one.  Everything that decides
//! anything is the pure library `vos-manifest`; this program supplies the files.
//!
//! ```text
//!   vos-link gen-doc   code/packages/rust/<impl>     writes code/specs/<impl>.warrant
//!   vos-link check-doc code/packages/rust/<impl>     exit 1 on any drift or rule failure
//!   vos-link gen-build code/packages/rust/<impl>     writes the canonical region of BUILD and BUILD_windows (VOS04 5.6)
//!   vos-link check-component code/packages/rust/<impl>   file set, BUILD byte-equal to gen-build's, row, floor, manifest
//!   vos-link gen-contract    code/packages/rust/<contract>   writes warrant/lemmas.table (VOS-039b-1)
//!   vos-link check-contract  code/packages/rust/<contract>   lemmas, mutants, config mutants, the table, byte for byte
//!   vos-link check-publish   [--root R]                      every evidence-only vos-* crate says publish = false (VOS-D104)
//! ```
//!
//! ## The pipeline
//!
//! ```text
//!   <impl>/vos.toml ──────────► kind, contract, obligations
//!   <impl>/warrant/impl.part ─┐
//!   <contract>/contract.vos ──┼─► vos-manifest ─► assemble ─► the document text
//!   <contract>/warrant/*.part ┘                                  │
//!                                                                ├─► gen-doc: verify, then write
//!   code/specs/<impl>.warrant (on disk) ◄────────────────────────┴─► check-doc: verify, then compare
//! ```
//!
//! ## What "verify" is (VOS02 section 10.1, D-GD8)
//!
//! | Code | Check |
//! |---|---|
//! | `CK11` | a contract with `sat = "model"`: the BUILD lane line, a well-formed `warrant/lemmas.sat` covering every lemma, none ran = fail |
//! | `CC12` | an implementation makes no SAT claim (`vos.toml`, BUILD or `warrant/`) |
//! | `GD01`..`GD05` | the inputs: part admission, `contract.vos`, tier files, claim-only obligations, `vos.toml` |
//! | `CD01` | the document exists and its marker skeleton parses |
//! | `CD02` | each marker digest equals the digest of its block |
//! | `CD03` | each copied block equals the contract package's part, byte for byte |
//! | `CD04` | `warrant-grammar` parses and type-checks it; the trailer is `assume ... none` |
//! | `CD05` | no claim shares an obligation's name with a different `claim_sha256` |
//! | `CD06` | `carries` is **computed from the parsed claims** and must equal `vos.toml`'s `obligations` |
//! | `CD07` | the R6 rules a document alone decides, by kind (production carries exactly one of `canonical` and `representation`) |
//! | `CD08` | regeneration: the file equals what `gen-doc` would write now |
//!
//! ## Exit codes (as `vos-shape-check`)
//!
//! `0` no findings, `1` findings, `2` no verdict: a rule could not be applied (an
//! unreadable or oversized file, a path outside the repository or through a symlink, a
//! usage error).  A rule that fails on bytes that *were* admitted is a finding.

pub mod bounds;
pub mod build_gen;
pub mod check;
pub mod component;
pub mod contract_check;
pub mod enums;
pub mod error;
pub mod fsio;
pub mod inputs;
pub mod ops;
pub mod oracle;
pub mod publish;
pub mod sat_lane;

pub use build_gen::gen_build;
pub use component::check_component;
pub use contract_check::{check_contract, gen_contract};
pub use error::{Finding, NoVerdict};
pub use ops::{check_doc, gen_doc, Outcome};
pub use publish::check_publish;
