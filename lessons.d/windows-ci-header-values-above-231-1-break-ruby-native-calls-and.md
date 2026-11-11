---
category: Cross-platform & Windows BUILD_windows
---

# Windows CI: header values above 2**31-1 break Ruby native calls, and PERL5LIB with semicolons is ignored by MSYS perl

Three Windows-only failures on the lz* hostile-header PR. (1) Ruby on Windows has a 32-bit C long, so passing 0xFFFFFFFF to String#byteslice or Array#[] raises RangeError; clamp header-derived values to the data actually present before any native call, and test by prepending a guard that raises above 2**31-1. (2) An exhaustive 65k-input round-trip sweep took over 60 s under ExUnit on Windows; sample the input space in tests. (3) BUILD_windows set PERL5LIB with a semicolon, but the MSYS perl splits on a colon, so sibling modules were not found; pass them with prove -I instead.
