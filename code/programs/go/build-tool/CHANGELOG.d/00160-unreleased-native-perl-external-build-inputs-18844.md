## Unreleased — Native Perl external build inputs (#18844)

- Helper-only and shared `perl-bridge` manifest/build/source edits select
  Conduit, IRC Server Native and Conduit Hello as runtime test consumers.
  Missing consumers fail closed, language filters are retained, and docs or
  lookalike paths do not trigger the mapping. The regression test fails without
  registration because a helper-only edit previously selected no packages.
- Both platform fronts declare Rust for all three native recipes. A test reads
  the real BUILD files and verifies helper-only changes request both Perl and
  Rust for Linux, macOS and Windows, so cold CI does not rely on ambient Cargo.
- The Windows native-suite gate includes these consumers and the helper path;
  the same real-metadata test checks that helper-only plans execute Windows CI.
