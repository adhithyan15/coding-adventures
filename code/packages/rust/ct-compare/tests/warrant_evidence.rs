//! Runs the Warrant-derived evidence harness against the real public API.
//!
//! Everything interesting lives in `generated/warrant_evidence.rs`, which is
//! produced by `cargo run -p coding_adventures_warrant_grammar --example
//! gen_evidence` from `code/specs/ct-compare.warrant`; never edit it by hand
//! (BUILD fails if it drifts).  This shim only gives the generated code a
//! `main`.  Set `WARRANT_EVIDENCE_OUT=<file>` to write the evidence manifest,
//! and `WARRANT_EVIDENCE_FAULT=<fault>` to run the suites against a deliberately
//! broken adapter (they must then fail).

#[allow(dead_code, unused_variables, unused_parens, clippy::all)]
mod generated {
    include!("generated/warrant_evidence.rs");
}

fn main() {
    std::process::exit(generated::run_main());
}
