---
category: Security boundaries
---

# A runtime's case-insensitive path comparison is Unicode folding, so a fail-closed boundary must fold ASCII itself

Four containment helpers (Python `PureWindowsPath` equality, Node `path.win32.relative`,
Go `strings.EqualFold`, Rust `str::to_lowercase`) each looked "case-insensitive on win32"
and each folded differently: U+212A (Kelvin) equals `k` in three of them, U+017F equals `s`
in Go, `İ` lower-cases to two characters in Rust and Python. The rule they were meant to
share (VER04i 9.1) is ASCII-only, so a non-ASCII case variant is "not inside" (fail
closed). Fix: stop calling the library's comparison. Reduce each path to an anchor plus
whole components and fold `A`..`Z` explicitly (the drive letter and the UNC server and
share too). Two traps while doing it: a `re.IGNORECASE` second opinion needs `re.ASCII`
or it re-introduces the Kelvin fold, and `str.upper()` of `ſ` is `S`, so a reserved-name
test needs an explicit ASCII guard. Pin every disputed character, both directions, as
shared fixture data, and only then change the lanes.
