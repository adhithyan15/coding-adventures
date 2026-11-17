# The crate's own behaviour tests, with and without the ghost face (SH13: a plain build never
# compiles the ghost module, so both configurations are tested).
cargo test -p vos-sched-rr --lib --test behaviour -- --nocapture
cargo test -p vos-sched-rr --features ghost --lib --test behaviour
cargo test -p vos-sched-rr --doc
# SH1-SH18 as class K, kind production (VOS03), and the (empty) capability footprint.
cargo run -q -p vos-shape-check -- .
cargo run -q -p coding_adventures_ca_capability_analyzer -- --dir .
# VOS02 10.1: the generated document code/specs/vos-sched-rr.warrant must equal what
# `vos-link gen-doc` writes now, and satisfy CD01-CD08 and the R6 rules.  vos-link is a
# standalone cargo root (code/programs/rust/vos-link), so it is run by manifest path, the way
# its own BUILD runs vos-shape-check from the workspace.
cargo run -q --manifest-path ../../../programs/rust/vos-link/Cargo.toml -- check-doc .
# VER04h: the evidence harness is generated from code/specs/vos-sched-rr.warrant and
# evidence.bindings.  Fail if the checked-in file drifted.
cargo run -q -p coding_adventures_warrant_grammar --example gen_evidence -- --check vos-sched-rr
# Run it and rewrite the evidence manifest.  Three controls are known to fail (VOS-D065);
# warrant/check_evidence.sh names them and demands exactly those.
sh warrant/check_evidence.sh
# Sabotage: each fault makes the suite of its claim report `not ok`.  The claims rep_init,
# dispatch_rep_post and safety_init have no line: no fault the generator offers changes what they see
# (field-zero is inert on the zero initial state and self-consistent for rep_post; VOS-D065, VOS-D087).
WARRANT_EVIDENCE_FAULT=field-zero WARRANT_EVIDENCE_FILTER=/exhaustive/rep_alpha cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=wrong-ctor WARRANT_EVIDENCE_FILTER=/exhaustive/insert_rep_lo cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=field-zero WARRANT_EVIDENCE_FILTER=/exhaustive/insert_rep_lo cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=wrong-ctor WARRANT_EVIDENCE_FILTER=/exhaustive/insert_rep_hi cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=field-zero WARRANT_EVIDENCE_FILTER=/exhaustive/insert_rep_hi cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=wrong-ctor WARRANT_EVIDENCE_FILTER=/exhaustive/remove_rep_lo cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=field-zero WARRANT_EVIDENCE_FILTER=/exhaustive/remove_rep_lo cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=wrong-ctor WARRANT_EVIDENCE_FILTER=/exhaustive/remove_rep_hi cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=field-zero WARRANT_EVIDENCE_FILTER=/exhaustive/remove_rep_hi cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=wrong-ctor WARRANT_EVIDENCE_FILTER=/exhaustive/dispatch_rep cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=field-zero WARRANT_EVIDENCE_FILTER=/exhaustive/dispatch_rep cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=wrong-ctor WARRANT_EVIDENCE_FILTER=/exhaustive/insert_safe cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=field-zero WARRANT_EVIDENCE_FILTER=/exhaustive/insert_safe cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=wrong-ctor WARRANT_EVIDENCE_FILTER=/exhaustive/remove_safe cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=field-zero WARRANT_EVIDENCE_FILTER=/exhaustive/remove_safe cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=wrong-ctor WARRANT_EVIDENCE_FILTER=/exhaustive/dispatch_safe cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=field-zero WARRANT_EVIDENCE_FILTER=/exhaustive/dispatch_safe cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=field-zero WARRANT_EVIDENCE_FILTER=/exhaustive/safety_alpha_build cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=field-zero WARRANT_EVIDENCE_FILTER=/exhaustive/pointer_kept cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=wrong-ctor WARRANT_EVIDENCE_FILTER=/exhaustive/dispatch_det_out cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=field-zero WARRANT_EVIDENCE_FILTER=/exhaustive/dispatch_det_img cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=field-zero WARRANT_EVIDENCE_FILTER=/example/n_is_eight cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=field-zero WARRANT_EVIDENCE_FILTER=/example/insert_three_sets_bit_three cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=wrong-ctor WARRANT_EVIDENCE_FILTER=/example/dispatch_from_two_picks_three cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=field-zero WARRANT_EVIDENCE_FILTER=/example/dispatch_from_two_moves_pointer_to_four cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | grep -q '^not ok '
