cargo test -q -p coding_adventures_warrant_corpus_gen
cargo clippy -p coding_adventures_warrant_corpus_gen --all-targets -- -D warnings
# The tests include the two oracle parity checks: tests/oracle_parity.rs (ir.core, against this
# crate's own committed dumps) and, since CV2-S10b, tests/fixed_parity.rs, which lowers the
# oracle's fixed_forms_v1.warrant and reproduces its committed fixed_forms_v1.oracle-lowering.json
# byte for byte. Both files lie in python/warrant-oracle/tests/fixtures/fixed-forms-v1/ and are
# read from there (an edit to either selects this package: repository_inputs.go).
# CV2-S10t-2: tests/tuple_parity.rs reproduces the oracle's tuple-forms-v1 dumps (lowering, evaluation, faults,
# refusals) byte for byte, reading them from python/warrant-oracle/tests/fixtures/tuple-forms-v1/ (an edit there
# selects this package: repository_inputs.go).
# CV2-S10c-2: tests/fixed_eval_parity.rs reproduces the oracle's fixed-eval-v1 dumps (lowering, evaluation, faults under the
# shipped and the open gate, values, domains) byte for byte, reading them from python/warrant-oracle/tests/fixtures/fixed-eval-v1/
# (an edit there selects this package: repository_inputs.go).
# CV2-S11c-2: tests/data_eval_parity.rs reproduces the oracle's data-eval-v1 dumps (lowering, evaluation, faults under the
# gate before ir.data and with it, values, domains, refusals with the typing rows) byte for byte, reading them from
# python/warrant-oracle/tests/fixtures/data-eval-v1/ (an edit there selects this package: repository_inputs.go,
# warrantDataEvalFixture), and runs the oracle's verify-v2 on a generator-written corpus when python3 is available.
# CV2-S10-1: tests/omit_example.rs regenerates the oracle's omit-example-v2 fixture (omit.warrant +
# corpus.profile -> corpus/) byte for byte; an edit to that directory selects this package
# (repository_inputs.go). A generator version bump regenerates it too (ct-compare README checklist).
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
# CV2 S10 adopter (x25519): the committed x25519 v2 fixture is exactly what this generator writes
# (same shape as the ct-compare line above; the build tool selects this package for an edit to
# x25519.warrant, the fixture or the x25519 bindings: repository_inputs.go, warrantAdopters).
cd ../../../.. && cargo run -q --manifest-path code/packages/rust/Cargo.toml -p coding_adventures_warrant_corpus_gen --bin warrant-corpus-gen -- --check --warrant code/specs/x25519.warrant --profile code/specs/fixtures/x25519-warrant-v2/corpus.profile --bindings code/packages/rust/x25519/evidence.bindings --out code/specs/fixtures/x25519-warrant-v2
# CV2 S10 adopter (lzw): the committed lzw v2 fixture (six bomb examples omitted by profile lines,
# max_ir_cost 16384). `--release`: about 25 s (in debug about 3 min, because the corpus has 17.5 M cases); the build tool selects this package for an edit to
# lzw.warrant, the fixture or the lzw bindings (repository_inputs.go, warrantAdopters).
cd ../../../.. && cargo run -q --release --manifest-path code/packages/rust/Cargo.toml -p coding_adventures_warrant_corpus_gen --bin warrant-corpus-gen -- --check --warrant code/specs/lzw.warrant --profile code/specs/fixtures/lzw-warrant-v2/corpus.profile --bindings code/packages/rust/lzw/evidence.bindings --out code/specs/fixtures/lzw-warrant-v2
# CV2 S10 adopter (wasm-leb128): the committed wasm-leb128 v2 fixture (44 suites, 46,566,930 cases, 740 chunks,
# 20 faults). `--release` is required: 2 min 11 s measured on a 4-core VM (a debug build takes about 20 min,
# VER04r D38); one line, not sharded. The build tool selects this package for an edit to wasm-leb128.warrant,
# the fixture or the wasm-leb128 bindings (repository_inputs.go, warrantAdopters).
cd ../../../.. && cargo run -q --release --manifest-path code/packages/rust/Cargo.toml -p coding_adventures_warrant_corpus_gen --bin warrant-corpus-gen -- --check --warrant code/specs/wasm-leb128.warrant --profile code/specs/fixtures/wasm-leb128-warrant-v2/corpus.profile --bindings code/packages/rust/wasm-leb128/evidence.bindings --out code/specs/fixtures/wasm-leb128-warrant-v2
# LGE-S1c (logic-gates-basic): the committed cross-lane basic-gates v2 fixture (26 suites, 590,350 cases, 31 faults;
# LG02 section 5). No --bindings: Rust's evidence is generated from logic-gates.warrant, and code/scripts/logic_gates_basic_drift.py
# (rust/logic-gates BUILD) compares this fixture's profile with the bindings instead. `--release` as for the other large adopters;
# the build tool selects this package for an edit to logic-gates-basic.warrant or the fixture (repository_inputs.go, warrantAdopters).
cd ../../../.. && cargo run -q --release --manifest-path code/packages/rust/Cargo.toml -p coding_adventures_warrant_corpus_gen --bin warrant-corpus-gen -- --check --warrant code/specs/logic-gates-basic.warrant --profile code/specs/fixtures/logic-gates-basic-warrant-v2/corpus.profile --out code/specs/fixtures/logic-gates-basic-warrant-v2
