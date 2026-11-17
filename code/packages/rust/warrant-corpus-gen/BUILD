cargo test -q -p coding_adventures_warrant_corpus_gen
cargo clippy -p coding_adventures_warrant_corpus_gen --all-targets -- -D warnings
# The crate's only OS access is in fsio.rs, cli.rs and the bin (required_capabilities.json
# declares it); the analyzer checks the source against that manifest.
cargo run -q -p coding_adventures_ca_capability_analyzer -- --dir .
# The gate, end to end through the real binary: generate ct-compare into a temp directory, --check
# it (must pass), flip a chunk line, --check again (must FAIL). The committed fixture directory
# and a --check against it are slice S3d (VER04r 16.3 [D10]).
d=$(mktemp -d) && cd ../../../.. && g="cargo run -q --manifest-path code/packages/rust/Cargo.toml -p coding_adventures_warrant_corpus_gen --bin warrant-corpus-gen --" && a="--warrant code/specs/ct-compare.warrant --profile code/packages/rust/warrant-corpus-gen/tests/fixtures/ct-compare.corpus.profile --bindings code/packages/rust/ct-compare/evidence.bindings --out $d" && $g $a && $g $a --check && sed -i 's/^0 /9 /' $d/chunks/eq_agrees.txt && ! $g $a --check 2>/dev/null; r=$?; rm -rf $d; exit $r
