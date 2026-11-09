cargo run -p coding_adventures_ca_capability_analyzer -- --dir . --check-module
cargo run -p coding_adventures_ca_capability_analyzer -- --dir . --verify-cert effect_certificate.json
cargo test -p coding_adventures_ct_compare -- --nocapture
python3 ../../../scripts/ct_compare_tfv.py
# VER04h: the evidence harness is generated from code/specs/ct-compare.warrant.
# Fail if the checked-in file drifted, rewrite the evidence manifest from a real
# run, and prove each sabotage mode makes the exhaustive suites report `not ok`.
cargo run -q -p coding_adventures_warrant_grammar --example gen_evidence -- --check
WARRANT_EVIDENCE_OUT=evidence/test-manifest.json cargo test -q -p coding_adventures_ct_compare --test warrant_evidence
WARRANT_EVIDENCE_FAULT=invert WARRANT_EVIDENCE_FILTER=/exhaustive/ cargo test -q -p coding_adventures_ct_compare --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=ignore-last-byte WARRANT_EVIDENCE_FILTER=/exhaustive/ cargo test -q -p coding_adventures_ct_compare --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=ignore-high-bit WARRANT_EVIDENCE_FILTER=/exhaustive/ cargo test -q -p coding_adventures_ct_compare --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=swap-choice WARRANT_EVIDENCE_FILTER=/exhaustive/ cargo test -q -p coding_adventures_ct_compare --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=missing-error WARRANT_EVIDENCE_FILTER=/exhaustive/ cargo test -q -p coding_adventures_ct_compare --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_PROOF_OUT=$PWD/evidence cargo test -q -p coding_adventures_warrant_ir --test warrant_proofs
WARRANT_PROOF_FAULT=secret-branch cargo test -q -p coding_adventures_warrant_ir --test warrant_proofs 2>/dev/null | grep -q '^not ok '
WARRANT_BENCH_OUT=evidence/bench-manifest.json cargo test -q -p coding_adventures_ct_compare --test microbenchmarks
# VER04h: the report is derived from the manifests written above; --check
# cross-checks it against them and fails closed on any disagreement.
cargo run -p coding_adventures_ca_assurance_report -- --check assurance_report.json
