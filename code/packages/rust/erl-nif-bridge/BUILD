# needs-toolchain: elixir
# erl-nif-bridge declares the erl_nif C API by hand (and, on Windows, calls it
# through the table the BEAM passes to nif_init), so only a real BEAM can say
# whether the declarations are right.  The unit tests pin the Windows callback
# table's order; the smoke test builds the nif_smoke example (a real NIF library
# built on this crate) and loads it into the erl on PATH, which calls every
# function the bridge wraps and checks the results from the Erlang side.
#
# `# needs-toolchain: elixir` makes CI install the pinned Erlang/OTP (27.3.4.11),
# which also provides escript.  Own target dir: the recipe finds the smoke library
# at a fixed path.
cargo test -p erl-nif-bridge
cargo build --release -p erl-nif-bridge
cargo clippy -p erl-nif-bridge --all-targets -- -D warnings
cargo build --release -p erl-nif-bridge --example nif_smoke --target-dir target
escript tests/smoke.escript target/release/examples
