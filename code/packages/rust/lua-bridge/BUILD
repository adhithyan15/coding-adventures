# needs-toolchain: lua
# lua-bridge declares the Lua 5.4 C API by hand, so only a real Lua can say whether
# the declarations are right.  The unit tests pin the constants and layouts that
# need no Lua; the smoke test builds the lua_bridge_smoke example (a real Lua C
# module built on this crate) and loads it into the lua on PATH, which calls every
# part of the bridge and checks the results from the Lua side.
#
# Own target dir: the smoke module is a cdylib, and the recipe finds it at a
# fixed path.
cargo test -p lua-bridge
cargo build --release -p lua-bridge
cargo clippy -p lua-bridge --all-targets -- -D warnings
cargo build --release -p lua-bridge --example lua_bridge_smoke --target-dir target
lua tests/smoke.lua target/release/examples
