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

Verify through a real consumer, for example `code/packages/perl/conduit`:
build the release DLL, stage it as prescribed by `BUILD_windows`, and run
`prove -Ilib -v t/` with the same interpreter. A successful Rust link alone
does not verify XS argument and return-value handling.
