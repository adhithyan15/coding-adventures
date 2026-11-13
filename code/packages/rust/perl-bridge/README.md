# perl-bridge

Thin Rust wrappers over Perl's C API, with a local C shim for the interpreter's
header macros and thread context. There are no external Rust dependencies.

On native Windows, install Strawberry Perl with its compiler tools on PATH.
The bridge uses that Perl's `ExtUtils::CBuilder` to compile the shim and links
the exact import library recorded in `Config{archlibexp}/CORE`. `PERL_BIN` may
select an interpreter explicitly; run the extension tests with that same Perl.
The Perl packages' `BUILD_windows` helper selects native Perl consistently
even when Git Bash has prepended its MSYS interpreter to PATH.
MSYS/Cygwin Perl and incompatible IV, pointer or NV layouts are rejected.
Callback error buffers are allocated and released inside the same C shim,
preserving Perl's allocator choice when a handler throws an exception.

On Unix the existing C compiler/archive build is retained. macOS extension
crates enable dynamic lookup for Perl's host-provided symbols.

## XSUBs that can die

Perl's `croak` (what `die` calls) ends in a `longjmp`. On Unix that just jumps; on
Windows it is an SEH unwind that walks through every Rust frame in between, and Rust
aborts ("panic in a function that cannot unwind") if the unwind reaches a plain
`extern "C"` frame. An XSUB that can reach `die` therefore has to be
`unsafe extern "C-unwind" fn(*mut CV)` and registered with `newXS_unwind` (the
existing `newXS` takes a plain `extern "C"` function and is unchanged). The smoke
example does this for every XSUB.

## Testing

`BUILD` (Unix) and `BUILD_windows` (which runs `tests\run-windows.ps1`, so the native
Strawberry Perl is selected once) run the same steps on every OS:

1. `cargo test -p perl-bridge`: pins the flag constants and struct layouts that must
   agree with `sv.h` and the C shim.
2. `cargo clippy --all-targets -- -D warnings`.
3. `cargo build --release --example perl_bridge_smoke`, then
   `perl tests/smoke.pl target/release/examples`. The example is a real XS extension
   built on the crate; the script stages it as
   `auto/PerlBridgeSmoke/PerlBridgeSmoke.<dlext>`, loads it through DynaLoader
   (`tests/lib/PerlBridgeSmoke.pm`) into the same Perl that compiled the shim, and
   checks the results from the Perl side with `Test::More` (core, no CPAN module):
   integer, unsigned, float and string conversions (embedded NULs included), the
   `Sv*OK` / `SvTRUE` / `SvROK` predicates, arrays, hashes, reference counts,
   `call_coderef` (a value, a `die` in the callback), `croak` and `warn` through
   `longjmp`, the interpreter context, and the XS stack frame.

A successful Rust link alone does not verify XS argument and return-value handling, so
the XS packages that use the bridge (`code/packages/perl/conduit`, `gf256_native`, ...)
also load it for real. The recipe needs a Perl with its headers on `PATH`
(`# needs-toolchain: perl`).

Not covered by the smoke test: Perl running on another OS thread (`set_context` is only
round-tripped on the main thread; Conduit exercises the threaded case), and Perl
releases other than the one the runner provides.
