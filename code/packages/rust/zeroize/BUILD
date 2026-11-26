# needs-toolchain: python
cargo test --locked -p coding_adventures_zeroize -- --nocapture
# VER04k section 4 (WX-G): the pilot evidence harness is generated from
# code/specs/zeroize.warrant and evidence.bindings.  Fail if the checked-in file
# drifted, run it (every `ensures` row monitors every call, every claim runs over
# its labelled domain), and prove each sabotage mode makes a claim report `not ok`:
# a mutation lost on a copy, an off-by-one last write, a store of the wrong value.
cargo run --locked -q -p coding_adventures_warrant_grammar --example gen_evidence -- --check zeroize
cargo test --locked -q -p coding_adventures_zeroize --test warrant_evidence
WARRANT_EVIDENCE_FAULT=stale-post python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p coding_adventures_zeroize --test warrant_evidence
WARRANT_EVIDENCE_FAULT=skip-last-write python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p coding_adventures_zeroize --test warrant_evidence
WARRANT_EVIDENCE_FAULT=cursor-off-by-one python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p coding_adventures_zeroize --test warrant_evidence
