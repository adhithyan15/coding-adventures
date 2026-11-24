## 0.3.0

- BREAKING: errors are `LogicGateError`, not `String`. `InvalidBit { param, value }` names the first invalid parameter by the public function's own parameter name; `InvalidShape String` covers list-length/shape failures. Match on constructors, never on message text (LG02 section 7).
- BREAKING: `validateBit` now takes the parameter name (`validateBit "a" x`); new `validateBits` for lists.
- Sequential circuits (`dLatch`, `dFlipFlop`, `register`, ...) validate their own arguments up front so inner gate names never leak.
- Tests: structural `Left (InvalidBit ...)` for every Int in [-1024,1024] except 0/1 plus Int boundaries at every argument position of the 11 basic gates, first-invalid-parameter ordering, and NAND-built gates never reporting an inner name. Run in a scratch copy outside `cabal.project` (the shared project fails on a `containers` conflict). QuickCheck not added: no Haskell package in the repo depends on it.
- Removed partial `head` and defaulted-type warnings (`ghc -Wall` clean).
- `muxN` is total: sel must be 1..16 bits and inputs exactly `2 ^ length sel`, else `InvalidShape` (previously `init`/`last` threw on short sel).
- `decoder`/`demux`/`muxN` cap select bits at `maxSelectBits = 16` before computing `2 ^ n` (no DoS, no Int wraparound).
- `register`/`shiftRegister` cap width at `maxDataWidth = 65536`; `counter` at `maxCounterWidth = 64`.
- `register` now rejects mismatched `qBits`/`masterQBits` lengths with `InvalidShape` instead of silently zero-filling.
- Traversals use zip instead of `!!`; `foldlInc` uses `foldl'`. `mux4` documents shape-before-bit ordering.
