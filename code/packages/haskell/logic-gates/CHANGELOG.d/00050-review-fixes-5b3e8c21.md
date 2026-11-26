## 0.4.0 (review follow-ups)

- `-Werror` removed from `logic-gates.cabal` (a published library must not carry it); `BUILD`/`BUILD_windows` run `cabal test --ghc-options=-Werror`.
- `BUILD` now FAILS when `cabal` is missing (sibling packages echo and skip; this package deliberately does not).
- `takeChecked` returns an already-forced `Right`; the guarantee is "for any total input" -- a bottom element supplied by the caller is raised when the snapshot is taken (tested with `muxN [0, undefined] [0]`).
- BREAKING: `validateBits` (used by arithmetic, block-ram, fpga) is bounded at 65536 elements; longer or infinite lists return `InvalidShape` instead of hanging.
- `validateBit` names are bounded: truncated to 64 characters, control characters replaced by `?` (internal names unchanged).
- Tests: valid-path rows for `encoder`, `priorityEncoder`, `triState`, `mux4`, `srLatch`.
