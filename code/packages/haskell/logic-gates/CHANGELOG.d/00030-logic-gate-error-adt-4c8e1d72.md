## 0.3.0

- BREAKING: errors are `LogicGateError`, not `String`. `InvalidBit { param, value }` names the first invalid parameter by the public function's own parameter name; `InvalidShape String` covers list-length/shape failures. Match on constructors, never on message text (LG02 section 7).
- BREAKING: `validateBit` now takes the parameter name (`validateBit "a" x`); new `validateBits` for lists.
- Sequential circuits (`dLatch`, `dFlipFlop`, `register`, ...) validate their own arguments up front so inner gate names never leak.
- Tests: structural `Left (InvalidBit ...)` for every Int in [-1024,1024] except 0/1 plus Int boundaries at every argument position of the 11 basic gates, first-invalid-parameter ordering, and NAND-built gates never reporting an inner name. Run in a scratch copy outside `cabal.project` (the shared project fails on a `containers` conflict). QuickCheck not added: no Haskell package in the repo depends on it.
- Removed partial `head` and defaulted-type warnings (`ghc -Wall` clean).
