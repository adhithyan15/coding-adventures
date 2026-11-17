# The crate's own behaviour tests, with and without the ghost face (SH13: a plain build never
# compiles the ghost module, so both configurations are tested).  They include the mutation
# sanity check of the soundness and coverage predicates (an oracle-ignoring dispatch is not
# covering, a non-member dispatch is not sound).
cargo test -p vos-sched-any --lib --test behaviour -- --nocapture
cargo test -p vos-sched-any --features ghost --lib --test behaviour
cargo test -p vos-sched-any --doc
# SH1-SH18 as class K, kind any (VOS03), and the (empty) capability footprint.
cargo run -q -p vos-shape-check -- .
cargo run -q -p coding_adventures_ca_capability_analyzer -- --dir .
# VOS02 10.1: the generated document code/specs/vos-sched-any.warrant must equal what
# `vos-link gen-doc` writes now, and satisfy CD01-CD08 and the R6 rules for kind any (the `-any`
# name, an oracle field outside the build cone, the `any` obligations and no `canonical`).
# vos-link is a standalone cargo root (code/programs/rust/vos-link), run by manifest path.
cargo run -q --manifest-path ../../../programs/rust/vos-link/Cargo.toml -- check-doc .
# VER04h: the evidence harness is generated from code/specs/vos-sched-any.warrant and
# evidence.bindings.  Fail if the checked-in file drifted.
cargo run -q -p coding_adventures_warrant_grammar --example gen_evidence -- --check vos-sched-any
# Run it and rewrite the evidence manifest.  Every test passes: no control is known-inert (no claim
# has `init` as its subject, so VOS-D065 does not arise here) and no tolerance script is needed.
WARRANT_EVIDENCE_OUT=evidence/test-manifest.json cargo test -q -p vos-sched-any --features ghost --test warrant_evidence
# Sabotage: each fault makes the suite of the claim it names report `not ok`.  insert, remove and
# dispatch are each attacked in both projections (wrong-ctor on the answer, field-zero on the
# image) for the safety tier, dispatch for the any tier (soundness and both coverage claims), and
# alpha and the examples.
WARRANT_EVIDENCE_FAULT=wrong-ctor WARRANT_EVIDENCE_FILTER=/exhaustive/insert_safe cargo test -q -p vos-sched-any --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=field-zero WARRANT_EVIDENCE_FILTER=/exhaustive/insert_safe cargo test -q -p vos-sched-any --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=wrong-ctor WARRANT_EVIDENCE_FILTER=/exhaustive/remove_safe cargo test -q -p vos-sched-any --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=field-zero WARRANT_EVIDENCE_FILTER=/exhaustive/remove_safe cargo test -q -p vos-sched-any --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=wrong-ctor WARRANT_EVIDENCE_FILTER=/exhaustive/dispatch_safe cargo test -q -p vos-sched-any --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=field-zero WARRANT_EVIDENCE_FILTER=/exhaustive/dispatch_safe cargo test -q -p vos-sched-any --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=field-zero WARRANT_EVIDENCE_FILTER=/exhaustive/safety_alpha_build cargo test -q -p vos-sched-any --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=wrong-ctor WARRANT_EVIDENCE_FILTER=/exhaustive/dispatch_sound cargo test -q -p vos-sched-any --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=field-zero WARRANT_EVIDENCE_FILTER=/exhaustive/dispatch_sound cargo test -q -p vos-sched-any --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=wrong-ctor WARRANT_EVIDENCE_FILTER=/exhaustive/dispatch_covers_run cargo test -q -p vos-sched-any --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=wrong-ctor WARRANT_EVIDENCE_FILTER=/exhaustive/dispatch_covers_idle cargo test -q -p vos-sched-any --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=field-zero WARRANT_EVIDENCE_FILTER=/example/n_is_eight cargo test -q -p vos-sched-any --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=field-zero WARRANT_EVIDENCE_FILTER=/example/insert_three_sets_bit_three cargo test -q -p vos-sched-any --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=wrong-ctor WARRANT_EVIDENCE_FILTER=/example/oracle_names_the_thread cargo test -q -p vos-sched-any --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
