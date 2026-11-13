cargo test -q -p gf256 --lib --test gf256_test -- --nocapture
# VER08b: the whole public API runs inside a seccomp/Landlock jail derived from
# required_capabilities.json (no capabilities), watched by an independent strace.
# The jail is Linux-only, so on macOS this plain run reports the gate NOT-LIVE (RCG010)
# without failing; Linux uses BUILD_linux instead, which runs the same line with
# RCG_STRICT=1 so a NOT-LIVE gate is a failure there.  (Windows uses BUILD_windows.)
cargo test -q -p gf256 --test runtime_capability
# VER04h: the evidence harness is generated from code/specs/gf256.warrant and
# evidence.bindings.  Fail if the checked-in file drifted, rewrite the evidence
# manifest from a real run, and prove each sabotage mode makes the model-pinned
# exhaustive suite report `not ok` (each filter names the cheap claim that pins
# the faulted behaviour, so a fault that a 2^24-case law cannot see is not run
# to completion).
cargo run -q -p coding_adventures_warrant_grammar --example gen_evidence -- --check gf256
WARRANT_EVIDENCE_OUT=evidence/test-manifest.json cargo test -q -p gf256 --test warrant_evidence
WARRANT_EVIDENCE_FAULT=or-for-xor WARRANT_EVIDENCE_FILTER=/exhaustive/add_matches_xor_model cargo test -q -p gf256 --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=wrapping-sub WARRANT_EVIDENCE_FILTER=/exhaustive/subtract_matches_xor_model cargo test -q -p gf256 --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=mul-as-add WARRANT_EVIDENCE_FILTER=/exhaustive/multiply_matches_peasant_model cargo test -q -p gf256 --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=alt-poly WARRANT_EVIDENCE_FILTER=/exhaustive/multiply_matches_peasant_model cargo test -q -p gf256 --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=generator-shift WARRANT_EVIDENCE_FILTER=/exhaustive/multiply_matches_peasant_model cargo test -q -p gf256 --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=missing-error WARRANT_EVIDENCE_FILTER=/exhaustive/divide_by_zero_rejected cargo test -q -p gf256 --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=exp-mod-256 WARRANT_EVIDENCE_FILTER=/exhaustive/power_reduces_mod_group_order cargo test -q -p gf256 --test warrant_evidence 2>/dev/null | grep -q '^not ok '
# The labels/bound rows are attempted on the real source; the front end refuses
# it, so the proof manifest records every obligation `not-live` with the blocker.
WARRANT_PROOF_OUT=$PWD/evidence cargo test -q -p coding_adventures_warrant_ir --test gf256_proofs
# VER04h: the report is derived from the manifests written above; --check
# cross-checks it against them and fails closed on any disagreement.
# --allow-unverified: the report still cites nine `unverified::` static attestations
# (AST audits no tool re-runs, same rows as ct-compare); the default is to refuse
# them, and this flag keeps the dependence visible until checked evidence replaces them.
cargo run -p coding_adventures_ca_assurance_report -- --check assurance_report.json --allow-unverified
