# BUILD — coding_adventures_irc_server_native (Elixir IRC server binding)
#
# Each non-comment line runs as a SEPARATE subprocess with the working
# directory reset to the package root, so chain directory changes with &&.
# cargo, mix, elixir, erlang are expected on PATH (mise shims locally; CI installs them).

# Step 1: build the Rust NIF cdylib.
cd native/irc_server_native && cargo build --release

# Step 2: copy the platform library into priv/ as irc_server_native.so
mkdir -p priv && { cp native/irc_server_native/target/release/libirc_server_native.dylib priv/irc_server_native.so 2>/dev/null || cp native/irc_server_native/target/release/libirc_server_native.so priv/irc_server_native.so; }

# Step 3: compile + test.
mix deps.get --quiet && mix compile && mix test --cover
