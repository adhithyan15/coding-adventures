# needs-toolchain: perl
# perl-bridge is a Rust wrapper over a C shim compiled against the host Perl's
# headers, so only a real Perl can say whether the shim and the Rust declarations
# agree.  The unit tests pin the constants and layouts; the smoke test builds the
# perl_bridge_smoke example (a real XS extension built on this crate), stages it as
# auto/PerlBridgeSmoke/PerlBridgeSmoke.<dlext> and loads it into the same Perl that
# compiled the shim (build.rs prefers `mise which perl`, so the loader does too),
# which calls every part of the bridge and checks the results from the Perl side.
#
# Own target dir: the recipe finds the smoke library at a fixed path.
cargo test -p perl-bridge
cargo build --release -p perl-bridge
cargo clippy -p perl-bridge --all-targets -- -D warnings
if command -v mise >/dev/null 2>&1; then PERL_BIN=$(mise which perl 2>/dev/null || true); fi && if [ -z "${PERL_BIN:-}" ]; then PERL_BIN=$(command -v perl); fi && cargo build --release -p perl-bridge --example perl_bridge_smoke --target-dir target && "$PERL_BIN" tests/smoke.pl target/release/examples
