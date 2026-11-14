cargo test --manifest-path=Cargo.toml -v
# VER04j section 14.4 (WX-N): the pilot evidence harness is generated from
# code/specs/x25519.warrant and evidence.bindings.  Fail if the checked-in file
# drifted, run it, and prove each sabotage mode makes a claim report `not ok`.
cargo run -q -p coding_adventures_warrant_grammar --example gen_evidence -- --check x25519
cargo test -q -p coding_adventures_x25519 --test warrant_evidence
WARRANT_EVIDENCE_FAULT=flip-result-bit cargo test -q -p coding_adventures_x25519 --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=missing-error cargo test -q -p coding_adventures_x25519 --test warrant_evidence 2>/dev/null | grep -q '^not ok '
