cargo test -q -p coding_adventures_warrant_corpus_gen
cargo clippy -p coding_adventures_warrant_corpus_gen --all-targets -- -D warnings
# The crate's only OS access is in fsio.rs, cli.rs and the bin (required_capabilities.json
# declares it); the analyzer checks the source against that manifest.
cargo run -q -p coding_adventures_ca_capability_analyzer -- --dir .
# The dev-only test-hooks feature (seeded-wrong evaluators) must not reach the shipped binary.
f=$(cargo tree -e normal --depth 0 -f '{f}' -p coding_adventures_warrant_corpus_gen) && ! echo "$f" | grep -q test-hooks
# The gate, end to end through the real binary: generate ct-compare into a temp directory, --check
# it (must pass), flip a chunk line, --check again: it must FAIL with the located drift finding
# (a panic or a usage error does not count). The committed fixture directory and a --check against
# it are slice S3d (VER04r 16.3 [D10]).
d=$(mktemp -d) && cd ../../../.. && g="cargo run -q --manifest-path code/packages/rust/Cargo.toml -p coding_adventures_warrant_corpus_gen --bin warrant-corpus-gen --" && a="--warrant code/specs/ct-compare.warrant --profile code/specs/fixtures/ct-compare-warrant-v2/corpus.profile --bindings code/packages/rust/ct-compare/evidence.bindings --out $d" && $g $a && $g $a --check && sed 's/^0 /9 /' $d/chunks/eq_agrees.txt > $d/eq.tmp && mv $d/eq.tmp $d/chunks/eq_agrees.txt && if $g $a --check 2>$d/err.txt; then false; else grep -q 'chunks/eq_agrees.txt: differs at line 1, column 1' $d/err.txt; fi; r=$?; rm -rf $d; exit $r
