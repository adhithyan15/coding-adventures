# BUILD — coding_adventures_conduit (Elixir Conduit; WEB06)
#
# IMPORTANT: The build tool runs each non-comment line as a SEPARATE
# subprocess with the working directory reset to the package root each
# time. Multi-step operations that require a directory change MUST be
# chained with && on a single line so they run in the same subprocess.
#
# Tools (cargo, mix, elixir, erlang) are expected on PATH (mise shims
# locally; CI installs them directly via setup-actions).

# Step 1: build the Rust NIF cdylib.
cd native/conduit_native && cargo build --release

# Step 2: copy the platform-appropriate library into priv/ as conduit_native.so
mkdir -p priv && { cp native/conduit_native/target/release/libconduit_native.dylib priv/conduit_native.so 2>/dev/null || cp native/conduit_native/target/release/libconduit_native.so priv/conduit_native.so; }

# Step 3: compile + test (mix coverage threshold is configured in mix.exs).
mix deps.get --quiet && mix compile && mix test --cover
