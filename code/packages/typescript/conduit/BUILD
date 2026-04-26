set -e

# BUILD — coding-adventures-conduit (TypeScript/Node.js port)
#
# Steps:
#   1. Build the Rust N-API cdylib (conduit_native_node)
#   2. Copy the .node file to the package root
#   3. Install npm dependencies
#   4. Type-check with tsc --noEmit
#   5. Run the test suite (unit + E2E via real TCP)
#
# Prerequisites: Node.js ≥18 (for built-in fetch), Rust toolchain.
# cargo, node, npm, npx are expected on PATH (mise shims locally; GitHub
# Actions installs on CI). All other TypeScript BUILD files in this repo
# follow the same pattern.

# ── Step 1: Rust cdylib ────────────────────────────────────────────────────────
# The cdylib is a workspace-isolated crate — its own Cargo.toml lives in
# ext/conduit_native_node/ so it doesn't collide with the repo-wide Rust
# workspace.  We build in release mode so that the .node module has full
# optimisation (important for the tight request/response hot path).
cd ext/conduit_native_node && cargo build --release && cd ../..

# ── Step 2: Copy .node to package root ────────────────────────────────────────
# The compiled library name and extension differs by OS:
#   macOS : libconduit_native_node.dylib  → conduit_native_node.node
#   Linux : libconduit_native_node.so     → conduit_native_node.node
#   Windows: conduit_native_node.dll      → conduit_native_node.node
#
# Node.js requires() the file as `conduit_native_node.node` regardless of OS.
# The if/elif chain below is written as a one-liner so the build tool's
# exec.Command-per-line model doesn't need multi-line variable refs.
node -e "const {execSync}=require('child_process'),p='ext/conduit_native_node/target/release/',os=process.platform,src=os==='darwin'?p+'libconduit_native_node.dylib':os==='win32'?p+'conduit_native_node.dll':p+'libconduit_native_node.so',fs=require('fs');if(!fs.existsSync(src))throw new Error('Built .node not found at '+src);fs.copyFileSync(src,'conduit_native_node.node');console.log('Copied',src,'→ conduit_native_node.node');"

# ── Step 3: Install npm dependencies ─────────────────────────────────────────
npm ci --quiet

# ── Step 4: Type-check ────────────────────────────────────────────────────────
# Compile the src/ tree to dist/ for publishing, and also check the tests.
npx tsc --project tsconfig.json
npx tsc --project tsconfig.test.json --noEmit

# ── Step 5: Run tests ────────────────────────────────────────────────────────
# Unit tests (halt_error, request, handler_context, application) run without
# the native module.  E2E tests (server.test.ts) start a real TCP server via
# serveBackground() and make requests with Node's built-in fetch.
npx vitest run
