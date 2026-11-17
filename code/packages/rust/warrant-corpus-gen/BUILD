cargo test -q -p coding_adventures_warrant_corpus_gen
cargo clippy -p coding_adventures_warrant_corpus_gen --all-targets -- -D warnings
# The crate's only OS access is in fsio.rs, cli.rs and the bin (required_capabilities.json
# declares it); the analyzer checks the source against that manifest.
cargo run -q -p coding_adventures_ca_capability_analyzer -- --dir .
# The dev-only test-hooks feature (seeded-wrong evaluators) must not reach the shipped binary.
f=$(cargo tree -e normal --depth 0 -f '{f}' -p coding_adventures_warrant_corpus_gen) && ! echo "$f" | grep -q test-hooks
# The gate, end to end through the real binary: generate ct-compare into a temp directory, --check
# it (must pass), flip a chunk line, --check again: it must FAIL with the located drift finding
# (a panic or a usage error does not count).
d=$(mktemp -d) && cd ../../../.. && g="cargo run -q --manifest-path code/packages/rust/Cargo.toml -p coding_adventures_warrant_corpus_gen --bin warrant-corpus-gen --" && a="--warrant code/specs/ct-compare.warrant --profile code/specs/fixtures/ct-compare-warrant-v2/corpus.profile --bindings code/packages/rust/ct-compare/evidence.bindings --out $d" && $g $a && $g $a --check && sed 's/^0 /9 /' $d/chunks/eq_agrees.txt > $d/eq.tmp && mv $d/eq.tmp $d/chunks/eq_agrees.txt && if $g $a --check 2>$d/err.txt; then false; else grep -q 'chunks/eq_agrees.txt: differs at line 1, column 1' $d/err.txt; fi; r=$?; rm -rf $d; exit $r
# CV2-S3d: the committed ct-compare v2 fixture is exactly what this generator writes. --check
# compares corpus.json, cases.tfv and every chunk file byte for byte, and corpus.json records
# `generator` {name, version}, so a version bump of this crate fails the line until the fixture
# is regenerated and the diff reviewed (VER04r 16.8 [D13]); the same bump also changes the oracle's
# accept-blind-v2 fixture (the_committed_accept_blind_fixture_is_what_the_generator_writes compares
# its bytes, generator.version included) and the oracle's --expect-generator pin. --profile names the fixture's own
# hand-written corpus.profile, so an edit to it moves profile_sha256 and fails here ([D14]);
# --bindings cross-checks the profile's faults against evidence.bindings (VER04r 8.4). Edits to
# the spec, the fixture or the bindings select this package through a build-tool consumer row
# (repository_inputs.go, "rust/warrant-corpus-gen").
cd ../../../.. && cargo run -q --manifest-path code/packages/rust/Cargo.toml -p coding_adventures_warrant_corpus_gen --bin warrant-corpus-gen -- --check --warrant code/specs/ct-compare.warrant --profile code/specs/fixtures/ct-compare-warrant-v2/corpus.profile --bindings code/packages/rust/ct-compare/evidence.bindings --out code/specs/fixtures/ct-compare-warrant-v2
