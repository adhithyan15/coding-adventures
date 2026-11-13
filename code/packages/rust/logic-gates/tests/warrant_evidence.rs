//! Runs the Warrant-derived evidence harness against the real public API.
//!
//! Everything interesting lives in `generated/warrant_evidence.rs`, which is
//! produced by `cargo run -p coding_adventures_warrant_grammar --example
//! gen_evidence -- logic-gates` from `code/specs/logic-gates.warrant` and
//! `evidence.bindings`; never edit it by hand (BUILD fails if it drifts).  This
//! shim only gives the generated code a `main`.  Set
//! `WARRANT_EVIDENCE_OUT=<file>` to write the evidence manifest, and
//! `WARRANT_EVIDENCE_FAULT=<fault>` to run the suites against a deliberately
//! broken adapter (they must then fail).

#[allow(dead_code, unused_variables, unused_parens, clippy::all)]
mod generated {
    include!("generated/warrant_evidence.rs");
}

fn main() {
    // The crate source the evidence describes (every `src/**/*.rs`); its framed
    // SHA-256 is recorded in the manifest (`source_sha256`) so the report
    // checker can detect staleness.
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    std::process::exit(generated::run_main(&src));
}
