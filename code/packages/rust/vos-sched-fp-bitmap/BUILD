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
# VER04h: the evidence harness is generated from code/specs/vos-sched-fp-bitmap.warrant and
# evidence.bindings.  Fail if the checked-in file drifted.
cargo run -q -p coding_adventures_warrant_grammar --example gen_evidence -- --check vos-sched-fp-bitmap
# Run it and rewrite the evidence manifest.  Three controls are known to fail (VOS-D065);
# warrant/check_evidence.sh names them and demands exactly those.
sh warrant/check_evidence.sh
# Sabotage: each fault makes the model-pinned suite of its operation report `not ok`.
WARRANT_EVIDENCE_FAULT=wrong-ctor WARRANT_EVIDENCE_FILTER=/exhaustive/insert_safe cargo test -q -p vos-sched-fp-bitmap --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=field-zero WARRANT_EVIDENCE_FILTER=/exhaustive/canon_insert_img cargo test -q -p vos-sched-fp-bitmap --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=wrong-ctor WARRANT_EVIDENCE_FILTER=/exhaustive/remove_safe cargo test -q -p vos-sched-fp-bitmap --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=field-zero WARRANT_EVIDENCE_FILTER=/exhaustive/canon_remove_img cargo test -q -p vos-sched-fp-bitmap --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=wrong-ctor WARRANT_EVIDENCE_FILTER=/exhaustive/dispatch_policy cargo test -q -p vos-sched-fp-bitmap --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=field-zero WARRANT_EVIDENCE_FILTER=/exhaustive/canon_dispatch_img cargo test -q -p vos-sched-fp-bitmap --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=field-zero WARRANT_EVIDENCE_FILTER=/exhaustive/safety_alpha_build cargo test -q -p vos-sched-fp-bitmap --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=field-zero WARRANT_EVIDENCE_FILTER=/example/n_is_eight cargo test -q -p vos-sched-fp-bitmap --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=field-zero WARRANT_EVIDENCE_FILTER=/example/insert_three_sets_bit_three cargo test -q -p vos-sched-fp-bitmap --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
