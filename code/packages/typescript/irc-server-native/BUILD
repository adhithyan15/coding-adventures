set -e

# BUILD — coding-adventures-irc-server-native (TypeScript/Node.js)
#
# All IRC + TCP logic lives in Rust (irc-net-reactor); this package is a thin
# N-API control surface. cargo, node, npm, npx are expected on PATH (mise shims
# locally; GitHub Actions installs on CI).

# Step 1: build the Rust N-API cdylib (workspace-isolated crate).
cd ext/irc_native_node && cargo build --release && cd ../..

# Step 2: copy the built addon to the package root as irc_native_node.node
# (Node requires it by that name regardless of OS). One-liner so the build
# tool's exec.Command-per-line model needs no cross-line variables.
node -e "const p='ext/irc_native_node/target/release/',os=process.platform,src=os==='darwin'?p+'libirc_native_node.dylib':os==='win32'?p+'irc_native_node.dll':p+'libirc_native_node.so',fs=require('fs');if(!fs.existsSync(src))throw new Error('Built .node not found at '+src);fs.copyFileSync(src,'irc_native_node.node');console.log('Copied',src,'-> irc_native_node.node');"

# Step 3: install npm dependencies.
npm ci --quiet

# Step 4: type-check (emit dist/ for publishing, and check the tests).
npx tsc --project tsconfig.json
npx tsc --project tsconfig.test.json --noEmit

# Step 5: run the test suite (real-socket E2E tests against the Rust engine).
npx vitest run
