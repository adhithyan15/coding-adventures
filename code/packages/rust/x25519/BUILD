# needs-toolchain: python
cargo test --locked --manifest-path=Cargo.toml -v
# VER04j section 14.4 (WX-N): the pilot evidence harness is generated from
# code/specs/x25519.warrant and evidence.bindings.  Fail if the checked-in file
# drifted, run it, and prove each sabotage mode makes a claim report `not ok`.
cargo run --locked -q -p coding_adventures_warrant_grammar --example gen_evidence -- --check x25519
cargo test --locked -q -p coding_adventures_x25519 --test warrant_evidence
WARRANT_EVIDENCE_FAULT=flip-result-bit python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p coding_adventures_x25519 --test warrant_evidence
WARRANT_EVIDENCE_FAULT=missing-error python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p coding_adventures_x25519 --test warrant_evidence
