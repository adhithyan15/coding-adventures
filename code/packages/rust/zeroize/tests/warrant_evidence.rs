//! Runs the Warrant-derived evidence harness against the real public API.
//!
//! Everything interesting lives in `generated/warrant_evidence.rs`, produced by
//! `cargo run -p coding_adventures_warrant_grammar --example gen_evidence --
//! zeroize` from `code/specs/zeroize.warrant` and `evidence.bindings`; never edit
//! it by hand (BUILD fails if it drifts).  This shim only gives the generated
//! code a `main`.  `WARRANT_EVIDENCE_FAULT=<fault>` runs the suites against a
//! deliberately broken adapter (they must then fail).

#[allow(dead_code, unused_variables, unused_parens, clippy::all)]
mod generated {
    include!("generated/warrant_evidence.rs");
}

fn main() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    std::process::exit(generated::run_main(&src));
}
