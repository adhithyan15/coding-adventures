cd ../warrant-corpus && npm ci --silent
npm ci && npm run build && npx vitest run --coverage
node --experimental-strip-types tests/warrant-evidence.ts check evidence/test-manifest.json
node --experimental-strip-types tests/warrant-evidence.ts sabotage
node --experimental-strip-types ../warrant-corpus/src/bin.ts validate-manifest evidence/test-manifest.json --repo-root ../../../.. --corpus ../../../specs/fixtures/ct-compare-warrant-v1 --registry ../../../specs/fixtures/ct-compare-warrant-v1/consumers.json
# WD-004b2 (VER04i 8.3): the Rust loader re-checks the lane (digests, source freshness, labels);
# --require-active makes an absent, planned or unloadable lane a build failure.
cargo run --quiet --manifest-path ../../rust/ca-assurance-report/Cargo.toml -- --check-lane typescript/ct-compare --repo-root ../../../.. --require-active
