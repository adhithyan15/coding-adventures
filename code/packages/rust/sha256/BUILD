cargo test -p coding_adventures_sha256 -- --nocapture
# VER04h: the evidence harness is generated from code/specs/sha256.warrant and
# evidence.bindings.  Fail if the checked-in file drifted.
cargo run -q -p coding_adventures_warrant_grammar --example gen_evidence -- --check sha256
# Run the generated harness: every example, exhaustive, property, fuzz suite,
# driver obligation and sabotage control must pass.
cargo test -q -p coding_adventures_sha256 --test warrant_evidence
