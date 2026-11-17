cargo test -q -p coding_adventures_warrant_corpus_gen
cargo clippy -p coding_adventures_warrant_corpus_gen --all-targets -- -D warnings
# S3a is pure in-memory computation (required_capabilities.json declares nothing); the
# analyzer checks the source, tests included, against that manifest.
cargo run -q -p coding_adventures_ca_capability_analyzer -- --dir .
