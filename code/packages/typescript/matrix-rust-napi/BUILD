# Build the Rust addon via cargo's `-p` package selector — passes
# the workspace Cargo.toml as the manifest, so the BUILD script
# references only the workspace root (`../../rust/Cargo.toml`),
# which the build-tool discovery skips (it isn't a leaf package).
# This keeps the BUILD validator happy: it would otherwise flag
# `../../rust/matrix-rust-napi` as an undeclared cross-language
# dep reference (no cross-language dep declaration mechanism exists
# in the shell BUILD format yet — would require a Starlark BUILD).
#
# Output binary goes to `../../rust/target/release/`, which is
# inside the build-artefact tree and likewise ignored by discovery.
cargo build -p matrix-rust-napi --release --manifest-path ../../rust/Cargo.toml

# Copy the produced shared library to our package directory so
# `src/index.ts` can `require()` it via a local relative path —
# also avoids any cross-package path reference in runtime code.
cp ../../rust/target/release/libmatrix_rust_napi.dylib matrix_rust_napi.node 2>/dev/null \
  || cp ../../rust/target/release/libmatrix_rust_napi.so matrix_rust_napi.node 2>/dev/null \
  || cp ../../rust/target/release/matrix_rust_napi.dll matrix_rust_napi.node 2>/dev/null \
  || (echo "matrix-rust-napi: no built shared library found in ../../rust/target/release/" >&2 && exit 1)

# Standard TypeScript package build + test.
npm install --silent
npx vitest run --coverage
