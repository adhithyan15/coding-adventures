cargo test --locked -q -p coding_adventures_hash_analysis --lib -- --nocapture
cargo test --locked -q -p coding_adventures_hash_analysis --doc
# The one capability this crate declares is rand:read:* (avalanche_score draws OS randomness);
# the analyzer checks the source against required_capabilities.json.
cargo run --locked -q -p coding_adventures_ca_capability_analyzer -- --dir .
