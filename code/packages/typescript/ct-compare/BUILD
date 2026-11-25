# VER31 F5: the property (VER05a) and fuzz (VER06b) engines are local file:
# devDependencies; install and build them (leaf first) before this package's npm ci.
cd ../pbt && npm ci --silent && npx tsc
cd ../fuzz && npm ci --silent && npx tsc
cd ../warrant-corpus && npm ci --silent
npm ci && npm run build && npx vitest run --coverage
# VER31 F6: regenerate the bench manifest from a real run (timings go to stdout only,
# so the committed file reproduces byte for byte).
WARRANT_BENCH_OUT=evidence/bench-manifest.json node --experimental-strip-types tests/microbenchmarks.ts > /dev/null
node --experimental-strip-types tests/warrant-evidence.ts check evidence/test-manifest.json
node --experimental-strip-types tests/warrant-evidence.ts sabotage
node --experimental-strip-types ../warrant-corpus/src/bin.ts validate-manifest evidence/test-manifest.json --repo-root ../../../.. --corpus ../../../specs/fixtures/ct-compare-warrant-v1 --registry ../../../specs/fixtures/ct-compare-warrant-v1/consumers.json
# needs-toolchain: rust
# WD-004b2 (VER04i 8.3): the Rust loader re-checks the lane (digests, source freshness, labels);
# --require-active makes an absent, planned or unloadable lane a build failure.
cargo run --quiet --manifest-path ../../rust/ca-assurance-report/Cargo.toml -- --check-lane typescript/ct-compare --repo-root ../../../.. --require-active
