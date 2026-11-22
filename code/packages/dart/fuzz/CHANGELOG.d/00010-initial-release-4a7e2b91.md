## 0.1.0

- Initial Dart fuzz lane (VER06c): nine-function mutator algebra, 65,536-bit
  coverage bitmap with admission, power-schedule energy, FNV crash signatures
  with dedup, two-pass minimization, `fuzzTarget` campaign runner, closed
  five-id vocabulary, and opaque TAP lines.
- Replays 25/25 fuzz-v1 corpus cases with exact bytes and verdicts; 8-byte
  integer paths stay `BigInt`-exact on VM and JS; zero runtime dependencies;
  95% branch+function coverage gate.
