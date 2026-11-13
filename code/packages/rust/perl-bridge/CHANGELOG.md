# Changelog — perl-bridge

## Unreleased

- `BUILD`/`BUILD_windows` build, lint and test instead of echoing. Both recipes were
  `echo "perl-bridge: compile-only crate (requires Perl shared library to link)"`, so
  CI never compiled the crate on its own. They now run `cargo test`, `cargo build
  --release`, `cargo clippy --all-targets -D warnings` and a smoke test on every OS,
  with `# needs-toolchain: perl`; `BUILD_windows` runs `tests/run-windows.ps1`, which
  selects the native Strawberry Perl once (Git Bash's MSYS Perl is first on PATH) and
  runs the same steps. The crate left the W6 Windows echo-skip allowlist (BUILD-002).
- Added `examples/perl_bridge_smoke.rs`, `tests/lib/PerlBridgeSmoke.pm` and
  `tests/smoke.pl`: a real XS extension built on the crate and loaded into the real
  Perl through DynaLoader. It exercises the integer, unsigned, float and string
  conversions (embedded NULs included), the `Sv*OK` / `SvTRUE` / `SvROK` predicates,
  arrays, hashes, reference counts, `call_coderef`, `croak`/`warn`, the interpreter
  context and the XS stack frame (50 checks). It passes on Linux Perl 5.38 and on
  Strawberry Perl 5.42 for Windows.
- Added unit tests for the flag constants and struct layouts that come from `sv.h` and
  the shim rather than from a symbol.
- `build.rs` passes `-undefined dynamic_lookup` for the cdylib on macOS (the flag the XS
  consumers' build scripts already pass); nothing changes elsewhere.
- Show the C compiler's own output when the Windows shim compile fails
  (`ExtUtils::CBuilder` `quiet=>0`). The first Windows full build failed
  `perl/conduit` and `perl/irc-server-native` here with only "error building
  xs.dll", which hid the real diagnostic.
- Release callback error strings through the same C shim that allocated them.
  This fixes a Windows allocator mismatch when a Perl handler throws: Conduit
  must return its `on_error` response and keep serving later requests.
- Build and archive the C shim on native Windows using the selected Strawberry
  Perl's compiler configuration, instead of skipping the shim entirely.
- Propagate the exact CORE import library to Rust consumers, including MSVC
  cdylibs; validate the interpreter's platform, pointer/IV width and NV size.
- On Windows, prefer the Perl on PATH over mise unless PERL_BIN is explicit,
  keeping compilation aligned with the BUILD recipe's loading interpreter.
