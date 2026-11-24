# The fixture's own behaviour tests.
cargo test --locked -p vos-probe-ready --lib --test behaviour -- --nocapture
# VER04h: the evidence harness is generated from code/specs/vos-probe-ready.warrant and
# evidence.bindings.  Fail if the checked-in file drifted.
cargo run --locked -q -p coding_adventures_warrant_grammar --example gen_evidence -- --check vos-probe-ready
# Run the generated harness: every example, exhaustive, property, fuzz suite and sabotage control
# must pass (the faults are real here, so no control is a known failure).
cargo test --locked -q -p vos-probe-ready --test warrant_evidence
