# Ensure the Rust addon is built first — the TS wrapper's smoke tests
# require matrix_rust_napi.node to exist at the resolved path.  Each
# transitive workspace Rust dep is rebuilt as part of `cargo build`
# inside matrix-rust-napi, so we don't need to chain installs the
# way pure-TS packages do.
cd ../../rust/matrix-rust-napi && npm run build
cd - >/dev/null

# Standard TypeScript package build + test.
npm install --silent
npx vitest run --coverage
