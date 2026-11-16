# The crate's own behaviour tests, with and without the ghost face (SH13: a plain build never
# compiles the ghost module, so both configurations are tested).
cargo test -p vos-sched-fp-bitmap --lib --test behaviour -- --nocapture
cargo test -p vos-sched-fp-bitmap --features ghost --lib --test behaviour
cargo test -p vos-sched-fp-bitmap --doc
# SH1-SH18 as class K, kind production (VOS03), and the (empty) capability footprint.
cargo run -q -p vos-shape-check -- .
cargo run -q -p coding_adventures_ca_capability_analyzer -- --dir .
# VOS02 10.1: the generated document code/specs/vos-sched-fp-bitmap.warrant must equal what
# `vos-link gen-doc` writes now, and satisfy CD01-CD08 and the R6 rules.  vos-link is a
# standalone cargo root (code/programs/rust/vos-link), so it is run by manifest path, the way
# its own BUILD runs vos-shape-check from the workspace.
cargo run -q --manifest-path ../../../programs/rust/vos-link/Cargo.toml -- check-doc .
# NOT YET HERE (VOS-D063): the VER04h evidence harness.  The generator refuses the document
# (W0701 `claim closure_at_m calls no operation`: the nine lemmas of the contract block need
# the `lemma` entry kind, VER-BACKLOG WX-SV item 3), so there is no
# tests/generated/warrant_evidence.rs to check for drift or to run.  Once it exists, add
#   cargo run -q -p coding_adventures_warrant_grammar --example gen_evidence -- --check vos-sched-fp-bitmap
#   cargo test -q -p vos-sched-fp-bitmap --features ghost --test warrant_evidence
# and the sabotage lines (WARRANT_EVIDENCE_FAULT=... WARRANT_EVIDENCE_FILTER=...).
