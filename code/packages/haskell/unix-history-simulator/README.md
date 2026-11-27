# Unix history simulator (Haskell)

This package models the First Edition Unix teaching image in memory and exposes it as a reusable Haskell library. `UnixHistory` keeps inode lookup, lowest-free descriptor allocation, per-open offsets, kernel events, and `cat`/`ls` command semantics as separate operations. `runCommand` returns a JSON value; the library has no argv, stdout, or fixture-file interface.

Run `cabal test unix-history-simulator:simulator-test` in this package for component checks. Its standalone `cabal.project` also includes a fixture-adjacent adapter; the repository BUILD replays the canonical nine cases through that adapter. The library's simulated transitions are deterministic and independent of host files or time.
