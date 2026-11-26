# needs-toolchain: python
cargo test --locked -q -p wasm-leb128 --lib -- --nocapture
cargo test --locked -q -p wasm-leb128 --doc
# The library is pure: no capability, test code included (the analyzer reads it).
cargo run --locked -q -p coding_adventures_ca_capability_analyzer -- --dir .
# VER08b: the whole public API (including the max_bits > 64 rejection) runs inside a
# seccomp/Landlock jail derived from required_capabilities.json (no capabilities),
# watched by an independent strace.  The jail is Linux-only, so on macOS this plain run
# reports the gate NOT-LIVE (RCG010) without failing; Linux uses BUILD_linux instead,
# which runs the same line with RCG_STRICT=1 so a NOT-LIVE gate is a failure there.
# (Windows uses BUILD_windows.)  Everything else in BUILD_linux is identical to this file.
cargo test --locked -q -p wasm-leb128 --test runtime_capability
# VER04h: the evidence harness is generated from code/specs/wasm-leb128.warrant and
# evidence.bindings.  Fail if the checked-in file drifted, rewrite the evidence
# manifest from a real run, and prove each sabotage mode makes a model-pinned
# exhaustive suite of each operation report `not ok` (each filter names the cheap
# claim that pins the faulted behaviour; a fault applies to every operation that
# lists it, and the filter selects which operation's claim is attacked).
cargo run --locked -q -p coding_adventures_warrant_grammar --example gen_evidence -- --check wasm-leb128
WARRANT_EVIDENCE_OUT=evidence/test-manifest.json cargo test --locked -q -p wasm-leb128 --test warrant_evidence
# Negative controls (WD-HARN1c): each line plants a fault (WARRANT_EVIDENCE_FAULT), aims the
# generated test target at one cheap claim (WARRANT_EVIDENCE_FILTER) and hands the run to the
# shared checker code/scripts/warrant_negctl.py, which passes only if the sabotaged suite exited
# non-zero, printed exactly one TAP plan, reported a real `not ok`, did not `Bail out!` and numbered
# its test points 1..N.  The rule lives in the script (and its tests), not in this file.
WARRANT_EVIDENCE_FAULT=flip-result-bit WARRANT_EVIDENCE_FILTER=/exhaustive/encode_unsigned_matches_canonical_model python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p wasm-leb128 --test warrant_evidence
WARRANT_EVIDENCE_FAULT=drop-last-result-byte WARRANT_EVIDENCE_FILTER=/exhaustive/encode_unsigned_matches_canonical_model python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p wasm-leb128 --test warrant_evidence
WARRANT_EVIDENCE_FAULT=flip-result-bit WARRANT_EVIDENCE_FILTER=/exhaustive/encode_signed_matches_canonical_model python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p wasm-leb128 --test warrant_evidence
WARRANT_EVIDENCE_FAULT=drop-last-result-byte WARRANT_EVIDENCE_FILTER=/exhaustive/encode_signed_matches_canonical_model python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p wasm-leb128 --test warrant_evidence
WARRANT_EVIDENCE_FAULT=off-by-one-consumed WARRANT_EVIDENCE_FILTER=/exhaustive/decode_unsigned_one_byte python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p wasm-leb128 --test warrant_evidence
WARRANT_EVIDENCE_FAULT=drop-consumed WARRANT_EVIDENCE_FILTER=/exhaustive/decode_unsigned_one_byte python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p wasm-leb128 --test warrant_evidence
WARRANT_EVIDENCE_FAULT=ok-as-err WARRANT_EVIDENCE_FILTER=/exhaustive/decode_unsigned_one_byte python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p wasm-leb128 --test warrant_evidence
WARRANT_EVIDENCE_FAULT=missing-error WARRANT_EVIDENCE_FILTER=/exhaustive/decode_unsigned_empty_input_rejected python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p wasm-leb128 --test warrant_evidence
WARRANT_EVIDENCE_FAULT=off-by-one-consumed WARRANT_EVIDENCE_FILTER=/exhaustive/decode_signed_one_byte python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p wasm-leb128 --test warrant_evidence
WARRANT_EVIDENCE_FAULT=drop-consumed WARRANT_EVIDENCE_FILTER=/exhaustive/decode_signed_one_byte python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p wasm-leb128 --test warrant_evidence
WARRANT_EVIDENCE_FAULT=ok-as-err WARRANT_EVIDENCE_FILTER=/exhaustive/decode_signed_one_byte python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p wasm-leb128 --test warrant_evidence
WARRANT_EVIDENCE_FAULT=missing-error WARRANT_EVIDENCE_FILTER=/exhaustive/decode_signed_empty_input_rejected python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p wasm-leb128 --test warrant_evidence
WARRANT_EVIDENCE_FAULT=off-by-one-consumed WARRANT_EVIDENCE_FILTER=/exhaustive/decode_unsigned_bounded_one_byte_every_width python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p wasm-leb128 --test warrant_evidence
WARRANT_EVIDENCE_FAULT=drop-consumed WARRANT_EVIDENCE_FILTER=/exhaustive/decode_unsigned_bounded_one_byte_every_width python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p wasm-leb128 --test warrant_evidence
WARRANT_EVIDENCE_FAULT=ok-as-err WARRANT_EVIDENCE_FILTER=/exhaustive/decode_unsigned_bounded_one_byte_every_width python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p wasm-leb128 --test warrant_evidence
WARRANT_EVIDENCE_FAULT=missing-error WARRANT_EVIDENCE_FILTER=/exhaustive/decode_unsigned_bounded_one_byte_every_width python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p wasm-leb128 --test warrant_evidence
WARRANT_EVIDENCE_FAULT=off-by-one-consumed WARRANT_EVIDENCE_FILTER=/exhaustive/decode_signed_bounded_one_byte_every_width python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p wasm-leb128 --test warrant_evidence
WARRANT_EVIDENCE_FAULT=drop-consumed WARRANT_EVIDENCE_FILTER=/exhaustive/decode_signed_bounded_one_byte_every_width python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p wasm-leb128 --test warrant_evidence
WARRANT_EVIDENCE_FAULT=ok-as-err WARRANT_EVIDENCE_FILTER=/exhaustive/decode_signed_bounded_one_byte_every_width python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p wasm-leb128 --test warrant_evidence
WARRANT_EVIDENCE_FAULT=missing-error WARRANT_EVIDENCE_FILTER=/exhaustive/decode_signed_bounded_one_byte_every_width python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p wasm-leb128 --test warrant_evidence
# No operand is secret, so there is no constant-time obligation; the steps rows are
# attempted on the real source, the front end refuses it, and the proof manifest
# records every steps-bound obligation `not-live` with the blocker.
WARRANT_PROOF_OUT=$PWD/evidence cargo test --locked -q -p coding_adventures_warrant_ir --test wasm_leb128_proofs
# VER04h: the report is derived from the manifests written above; --check
# cross-checks it against them and fails closed on any disagreement.
# --allow-unverified: the report still cites nine `unverified::` static attestations
# (AST audits no tool re-runs, same rows as gf256 and ct-compare).
cargo run --locked -p coding_adventures_ca_assurance_report -- --check assurance_report.json --allow-unverified
