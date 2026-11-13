# Windows test fixes in internal/legacy

- `TestConstructPATH` expected the host list separator (`;` on Windows), but
  `ConstructPATH` always uses POSIX `:` because the PATH is for the Linux
  recording environment (KILN04 §4.2). Since #19290 it failed on Windows.
- `TestLookupRootAndFlagsAreInTheKey` used `/elsewhere/checkout` as an
  absolute root, which is not absolute on Windows. The test now passes it
  through `filepath.Abs`.

Both are test-only. Neither changes key computation or lookup behaviour.
