## 0.4.0 (LGE-HS-HARDEN)

- BREAKING: `shiftRegister` and `counter` reject an empty state list with `InvalidShape "qBits must not be empty"` instead of silently substituting four zeros (mirrors `register`).
- BREAKING: `LatchState` / `FlipFlopState` fields are strict.
- BREAKING: list-taking functions now reject over-long input (including infinite lazy lists such as `cycle [0]`) with `InvalidShape`: `encoder`, `priorityEncoder`, `muxN` inputs at 65536 (`maxInputWidth`), select lists at 16, `mux4`/`mux8` exact lengths, register widths at 65536, counter at 64. Previously a bare `length` hung forever and `System.Timeout` could not interrupt it.
- New `takeChecked :: String -> Int -> [Int] -> Either LogicGateError [Int]` (walks at most cap+1 cells, returns a fully forced list) and `forceBits`; every list input is snapshotted through it before any other work, and every returned list is forced, so no result carries a thunk that can throw later.
- Build: `-Wall -Wcompat -Wincomplete-patterns -Wincomplete-uni-patterns -Werror` on library and tests; library modules are `{-# LANGUAGE Safe #-}`.
- Added `required_capabilities.json` (no capabilities).
- Tests: wide-int rows (2^31, 2^32, 2^32+1, 2^53+1, minBound, maxBound) on all 11 gates in both positions with decisive-first-argument (no short-circuit) rows; bad-bit rows for demux/decoder/encoder/priorityEncoder/triState/mux4/mux8; infinite-list and cap+1 rejection for every list-taking function; empty-state rejection; forced-result (`show`-based) never-throws checks.
