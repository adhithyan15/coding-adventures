# build-tool: deps=lua/conduit
set -e

# BUILD — conduit-hello (Lua demo program)
#
# conduit-hello is a runnable program, not an installable package. It depends on
# the Lua `conduit` package (which wraps the Rust `conduit_native` cdylib). We
# build that native library into the conduit package directory first — exactly
# as the conduit package's own BUILD does — so `require("conduit")` resolves the
# `.so`, then run the demo's E2E tests against it.
#
# Each line runs in its own shell starting from this directory
# (code/programs/lua/conduit-hello/), so each `cd` is transient.
#
#   ../../../packages/lua/conduit → code/packages/lua/conduit

# Step 1: compile the Rust native extension for the conduit package.
(cd ../../../packages/lua/conduit/ext/conduit_native && cargo build --release)

# Step 2: copy the built cdylib next to the conduit Lua module so `require` finds it.
(cd ../../../packages/lua/conduit && python3 -c "import glob,shutil,sys,os; libs=glob.glob('ext/conduit_native/target/release/libconduit_native.*'); src=next((l for l in libs if l.endswith(('.so','.dylib','.dll'))),None); sys.exit('no libconduit_native found') if not src else shutil.copy(src,'conduit/conduit_native.dll' if os.name=='nt' else 'conduit/conduit_native.so')")

# Step 3: install luasocket so the E2E tests run (they self-skip if it's absent).
luarocks install luasocket 2>/dev/null || true

# Step 4: run the demo's E2E tests (busted), from the tests directory so the
# relative package paths resolve.
cd tests && busted . --verbose --pattern=test_
