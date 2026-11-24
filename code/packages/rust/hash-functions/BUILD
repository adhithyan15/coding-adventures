# needs-toolchain: python
cargo test --locked -q -p coding_adventures_hash_functions --lib -- --nocapture
cargo test --locked -q -p coding_adventures_hash_functions --doc
# DT17 v1 shared fixture: compare every case through the Rust public API.
cargo test --locked -q -p coding_adventures_hash_functions --test hash_functions_v1_fixture
# The library is pure: no capability (the analyzer scans the crate source, src/).  The OS
# randomness of the old analysis helpers lives in hash-analysis (WX-P1a2).
cargo run --locked -q -p coding_adventures_ca_capability_analyzer -- --dir .
# VER08b: the eight covered byte-string hashes run inside a seccomp/Landlock jail derived from
# required_capabilities.json (no capabilities), watched by an independent strace.
# The jail is Linux-only, so on macOS this plain run reports the gate NOT-LIVE (RCG010)
# without failing; Linux uses BUILD_linux, which runs the same line with RCG_STRICT=1.
# (Windows uses BUILD_windows.)  Everything else in BUILD_linux is identical to this file.
cargo test --locked -q -p coding_adventures_hash_functions --test runtime_capability
# VER04h: the evidence harness is generated from code/specs/hash-functions.warrant and
# evidence.bindings.  Fail if the checked-in file drifted, rewrite the evidence manifest from a
# real run, and prove each sabotage mode makes a cheap exhaustive claim about the faulted
# operation report `not ok`.  The first (WX-FAULT1) controls attack a Bytes -> U32/U64 result:
# flip-result-bit XORs the least significant bit of the integer.
cargo run --locked -q -p coding_adventures_warrant_grammar --example gen_evidence -- --check hash-functions
WARRANT_EVIDENCE_OUT=evidence/test-manifest.json cargo test --locked -q -p coding_adventures_hash_functions --test warrant_evidence
# Negative controls (WD-HARN1c): each line plants a fault (WARRANT_EVIDENCE_FAULT), aims the
# generated test target at one cheap claim (WARRANT_EVIDENCE_FILTER) and hands the run to the
# shared checker code/scripts/warrant_negctl.py, which passes only if the sabotaged suite exited
# non-zero, printed exactly one TAP plan, reported a real `not ok`, did not `Bail out!` and numbered
# its test points 1..N.  The rule lives in the script (and its tests), not in this file.
WARRANT_EVIDENCE_FAULT=flip-result-bit WARRANT_EVIDENCE_FILTER=/exhaustive/fnv1a_32_one_byte_matches_both_models python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p coding_adventures_hash_functions --test warrant_evidence
WARRANT_EVIDENCE_FAULT=flip-result-bit WARRANT_EVIDENCE_FILTER=/exhaustive/fnv1a_64_one_byte_matches_both_models python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p coding_adventures_hash_functions --test warrant_evidence
WARRANT_EVIDENCE_FAULT=flip-result-bit WARRANT_EVIDENCE_FILTER=/exhaustive/djb2_one_byte_matches_both_models python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p coding_adventures_hash_functions --test warrant_evidence
WARRANT_EVIDENCE_FAULT=flip-result-bit WARRANT_EVIDENCE_FILTER=/exhaustive/polynomial_rolling_one_byte_is_the_byte python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p coding_adventures_hash_functions --test warrant_evidence
WARRANT_EVIDENCE_FAULT=missing-error WARRANT_EVIDENCE_FILTER=/exhaustive/polynomial_params_zero_modulus_is_rejected python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p coding_adventures_hash_functions --test warrant_evidence
WARRANT_EVIDENCE_FAULT=flip-result-bit WARRANT_EVIDENCE_FILTER=/exhaustive/polynomial_params_abc_every_parameter_pair python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p coding_adventures_hash_functions --test warrant_evidence
WARRANT_EVIDENCE_FAULT=flip-result-bit WARRANT_EVIDENCE_FILTER=/exhaustive/murmur3_32_one_byte_matches_the_model python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p coding_adventures_hash_functions --test warrant_evidence
WARRANT_EVIDENCE_FAULT=flip-result-bit WARRANT_EVIDENCE_FILTER=/exhaustive/murmur3_seed_len_0 python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p coding_adventures_hash_functions --test warrant_evidence
WARRANT_EVIDENCE_FAULT=flip-result-bit WARRANT_EVIDENCE_FILTER=/exhaustive/siphash_len_0 python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p coding_adventures_hash_functions --test warrant_evidence
# siphash_2_4's key is `secret`: its constant-time obligation is attempted on the real source,
# the front end refuses it, and the proof manifest records it `not-live` with the blocker.
WARRANT_PROOF_OUT=$PWD/evidence cargo test --locked -q -p coding_adventures_warrant_ir --test hash_functions_proofs
# Staleness: the two manifests written above are deterministic (fixed seed, content hashes, no
# timestamps or toolchain data) and the repo forces LF, so the committed files must equal the
# fresh ones.
git diff --exit-code -- evidence/
# VER04h: the report is derived from the manifests written above; --check cross-checks it
# against them and fails closed on any disagreement.  --allow-unverified: the report still
# cites `unverified::` static attestations (AST audits no tool re-runs, same rows as the other
# adopters).  --profile pins the audit profile the report was generated under.
cargo run --locked -p coding_adventures_ca_assurance_report -- --check assurance_report.json --allow-unverified --profile non-cryptographic-hash --profile-kind transform
