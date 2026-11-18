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
# Run it and rewrite the evidence manifest.  Three controls are known to fail: `init` / `field-flip`, whose only
# member claim (`rep_init`) cannot see a flip that stays in the image of rep (VOS-D087, VOS-D098);
# warrant/check_evidence.sh names them and demands exactly those.
sh warrant/check_evidence.sh
# Sabotage: each fault makes the suite of its claim report `not ok`.  A negative control passes only if the sabotaged run
# printed a `not ok ` line AND no `Bail out!` anywhere (not anchored: a subject's unterminated stdout write could put it
# mid-line).  A pipeline's status is its last command's, so the old `grep -q '^not ok '` passed a run where one test
# failed as intended and a later one hung (observer timeout: `Bail out!`, exit 70) and the hang went unreported.  The
# pipe also hides the binary's exit status, so this reads the TAP on stdout.  The claims rep_init,
# dispatch_rep_post and safety_init have no field-zero line (inert on the zero initial state, self-consistent
# for rep_post; VOS-D065, VOS-D087): their lines are the field-flip ones at the end (WX-SV2).
WARRANT_EVIDENCE_FAULT=field-zero WARRANT_EVIDENCE_FILTER=/exhaustive/rep_alpha cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | awk '/^not ok /{n=1} /Bail out!/{b=1} END{exit !(n&&!b)}'
WARRANT_EVIDENCE_FAULT=wrong-ctor WARRANT_EVIDENCE_FILTER=/exhaustive/insert_rep_lo cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | awk '/^not ok /{n=1} /Bail out!/{b=1} END{exit !(n&&!b)}'
WARRANT_EVIDENCE_FAULT=field-zero WARRANT_EVIDENCE_FILTER=/exhaustive/insert_rep_lo cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | awk '/^not ok /{n=1} /Bail out!/{b=1} END{exit !(n&&!b)}'
WARRANT_EVIDENCE_FAULT=wrong-ctor WARRANT_EVIDENCE_FILTER=/exhaustive/insert_rep_hi cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | awk '/^not ok /{n=1} /Bail out!/{b=1} END{exit !(n&&!b)}'
WARRANT_EVIDENCE_FAULT=field-zero WARRANT_EVIDENCE_FILTER=/exhaustive/insert_rep_hi cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | awk '/^not ok /{n=1} /Bail out!/{b=1} END{exit !(n&&!b)}'
WARRANT_EVIDENCE_FAULT=wrong-ctor WARRANT_EVIDENCE_FILTER=/exhaustive/remove_rep_lo cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | awk '/^not ok /{n=1} /Bail out!/{b=1} END{exit !(n&&!b)}'
WARRANT_EVIDENCE_FAULT=field-zero WARRANT_EVIDENCE_FILTER=/exhaustive/remove_rep_lo cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | awk '/^not ok /{n=1} /Bail out!/{b=1} END{exit !(n&&!b)}'
WARRANT_EVIDENCE_FAULT=wrong-ctor WARRANT_EVIDENCE_FILTER=/exhaustive/remove_rep_hi cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | awk '/^not ok /{n=1} /Bail out!/{b=1} END{exit !(n&&!b)}'
WARRANT_EVIDENCE_FAULT=field-zero WARRANT_EVIDENCE_FILTER=/exhaustive/remove_rep_hi cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | awk '/^not ok /{n=1} /Bail out!/{b=1} END{exit !(n&&!b)}'
WARRANT_EVIDENCE_FAULT=wrong-ctor WARRANT_EVIDENCE_FILTER=/exhaustive/dispatch_rep cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | awk '/^not ok /{n=1} /Bail out!/{b=1} END{exit !(n&&!b)}'
WARRANT_EVIDENCE_FAULT=field-zero WARRANT_EVIDENCE_FILTER=/exhaustive/dispatch_rep cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | awk '/^not ok /{n=1} /Bail out!/{b=1} END{exit !(n&&!b)}'
WARRANT_EVIDENCE_FAULT=wrong-ctor WARRANT_EVIDENCE_FILTER=/exhaustive/insert_safe cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | awk '/^not ok /{n=1} /Bail out!/{b=1} END{exit !(n&&!b)}'
WARRANT_EVIDENCE_FAULT=field-zero WARRANT_EVIDENCE_FILTER=/exhaustive/insert_safe cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | awk '/^not ok /{n=1} /Bail out!/{b=1} END{exit !(n&&!b)}'
WARRANT_EVIDENCE_FAULT=wrong-ctor WARRANT_EVIDENCE_FILTER=/exhaustive/remove_safe cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | awk '/^not ok /{n=1} /Bail out!/{b=1} END{exit !(n&&!b)}'
WARRANT_EVIDENCE_FAULT=field-zero WARRANT_EVIDENCE_FILTER=/exhaustive/remove_safe cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | awk '/^not ok /{n=1} /Bail out!/{b=1} END{exit !(n&&!b)}'
WARRANT_EVIDENCE_FAULT=wrong-ctor WARRANT_EVIDENCE_FILTER=/exhaustive/dispatch_safe cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | awk '/^not ok /{n=1} /Bail out!/{b=1} END{exit !(n&&!b)}'
WARRANT_EVIDENCE_FAULT=field-zero WARRANT_EVIDENCE_FILTER=/exhaustive/dispatch_safe cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | awk '/^not ok /{n=1} /Bail out!/{b=1} END{exit !(n&&!b)}'
WARRANT_EVIDENCE_FAULT=field-zero WARRANT_EVIDENCE_FILTER=/exhaustive/safety_alpha_build cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | awk '/^not ok /{n=1} /Bail out!/{b=1} END{exit !(n&&!b)}'
WARRANT_EVIDENCE_FAULT=field-zero WARRANT_EVIDENCE_FILTER=/exhaustive/pointer_kept cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | awk '/^not ok /{n=1} /Bail out!/{b=1} END{exit !(n&&!b)}'
WARRANT_EVIDENCE_FAULT=wrong-ctor WARRANT_EVIDENCE_FILTER=/exhaustive/dispatch_det_out cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | awk '/^not ok /{n=1} /Bail out!/{b=1} END{exit !(n&&!b)}'
WARRANT_EVIDENCE_FAULT=field-zero WARRANT_EVIDENCE_FILTER=/exhaustive/dispatch_det_img cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | awk '/^not ok /{n=1} /Bail out!/{b=1} END{exit !(n&&!b)}'
WARRANT_EVIDENCE_FAULT=field-zero WARRANT_EVIDENCE_FILTER=/example/n_is_eight cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | awk '/^not ok /{n=1} /Bail out!/{b=1} END{exit !(n&&!b)}'
WARRANT_EVIDENCE_FAULT=field-zero WARRANT_EVIDENCE_FILTER=/example/insert_three_sets_bit_three cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | awk '/^not ok /{n=1} /Bail out!/{b=1} END{exit !(n&&!b)}'
WARRANT_EVIDENCE_FAULT=wrong-ctor WARRANT_EVIDENCE_FILTER=/example/dispatch_from_two_picks_three cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | awk '/^not ok /{n=1} /Bail out!/{b=1} END{exit !(n&&!b)}'
WARRANT_EVIDENCE_FAULT=field-zero WARRANT_EVIDENCE_FILTER=/example/dispatch_from_two_moves_pointer_to_four cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | awk '/^not ok /{n=1} /Bail out!/{b=1} END{exit !(n&&!b)}'
# WX-SV2 (VER04j 4.8): `field-flip` flips the lowest bit of the first numeric field, so it is never inert.  Two operations that
# list it would cancel (an `init` flipped and the `alpha_w` that reads it flipped again), so WARRANT_EVIDENCE_ATTACK aims the run at one.
WARRANT_EVIDENCE_FAULT=field-flip WARRANT_EVIDENCE_ATTACK=init WARRANT_EVIDENCE_FILTER=/exhaustive/safety_init cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | awk '/^not ok /{n=1} /Bail out!/{b=1} END{exit !(n&&!b)}'
WARRANT_EVIDENCE_FAULT=field-flip WARRANT_EVIDENCE_ATTACK=alpha_w WARRANT_EVIDENCE_FILTER=/exhaustive/safety_init cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | awk '/^not ok /{n=1} /Bail out!/{b=1} END{exit !(n&&!b)}'
WARRANT_EVIDENCE_FAULT=field-flip WARRANT_EVIDENCE_ATTACK=alpha_w WARRANT_EVIDENCE_FILTER=/exhaustive/rep_init cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | awk '/^not ok /{n=1} /Bail out!/{b=1} END{exit !(n&&!b)}'
WARRANT_EVIDENCE_FAULT=field-flip WARRANT_EVIDENCE_ATTACK=alpha_w WARRANT_EVIDENCE_FILTER=/exhaustive/dispatch_rep_post cargo test -q -p vos-sched-rr --features ghost --test warrant_evidence 2>/dev/null | awk '/^not ok /{n=1} /Bail out!/{b=1} END{exit !(n&&!b)}'
# VOS-D070, VOS04 5.6 item G: line coverage of src/ must stay at or above the `coverage-floor` of vos.toml (95, the
# repository's package standard).  The floor is the --fail-under below; `vos-link check-component` fails if the two differ,
# if --include-files does not name this crate (without it tarpaulin scores the whole workspace) or if the line is missing.
# A run that measures no line of the crate exits non-zero ("none ran = fail").  Tarpaulin is a Linux gate here, as everywhere.
if [ "$(uname)" = "Linux" ]; then cargo tarpaulin -p vos-sched-rr --features ghost --lib --test behaviour --ignore-tests --engine llvm --include-files 'vos-sched-rr/src/*' --fail-under 95 --skip-clean; fi
# VOS04 5.6: the component kit's check.  LAST, so it reads the manifest the generated suite above just rewrote: the file set,
# BUILD lines 1-6 and 9 with their "none ran = fail" guards (the manifest: cases, property >= 4096, fuzz >= 5000, controls
# that report `not ok`), the build-tool row, publish = false for an `-any`, and the coverage gate.  vos-link is a standalone cargo root.
cargo run -q --manifest-path ../../../programs/rust/vos-link/Cargo.toml -- check-component .
