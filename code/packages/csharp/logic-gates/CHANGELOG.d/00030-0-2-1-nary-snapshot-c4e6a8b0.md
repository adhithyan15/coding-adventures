## [0.2.1]

### Security

- LGE-CS-NARY: `AndN`/`OrN`/`XorN` now copy the input array once, validate the
  copy and compute only from it, closing the check-then-use race (CWE-367) when
  the caller's array is shared across threads.
- Added `LogicGates.MaxInputs` (65536); longer arrays throw `ArgumentException`
  before any copy is made. Shape errors (null, too short, too long) win over
  bit errors; null stays `ArgumentNullException`.
- Bad elements keep throwing `InvalidBitException` named `inputs[i]` (first bad
  element of the snapshot).

### Changed (breaking)

- Arrays longer than 65536 elements, previously accepted by `AndN`/`OrN`/`XorN`,
  now throw `ArgumentException`.
