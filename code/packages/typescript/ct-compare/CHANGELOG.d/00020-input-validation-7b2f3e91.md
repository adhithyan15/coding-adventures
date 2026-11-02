## 0.2.0

- **Security fix.** Plain arrays were converted with `Uint8Array.from`, which
  silently wraps values outside 0..255 and turns `NaN`, non-numbers and string
  characters into 0, so two *different* inputs could compare equal. Inputs are
  now validated: a `Uint8Array` or `ArrayBuffer`, or an array of integers 0..255;
  anything else throws `TypeError` or `RangeError`. **Breaking:** strings, other
  typed arrays and other iterables are no longer accepted.
- **Security fix.** `ctSelectBytes` branched on the secret `choice` bit (`?:`),
  which CT01 forbids. The mask is now `-Number(choice) & 0xff`, and a
  non-boolean `choice` throws `TypeError` instead of being truthiness-tested.
- Regression tests for every rejected input shape and for fresh-copy output.
- Genuine `Uint8Array`s and `ArrayBuffer`s are recognised with the engine's
  intrinsic getters instead of `instanceof`, so forged prototypes, subclasses
  with an overridden `length`, and proxies cannot misreport their bytes; an
  array's `length` is read exactly once.
