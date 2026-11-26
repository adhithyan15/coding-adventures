## 0.1.1

- `validateBit` now rejects negative zero (`-0`) with `RangeError` via `Object.is`;
  it was previously accepted because `-0 !== 0` is false. Same rule as
  `@coding-adventures/logic-gates` 0.2.0. Changelog converted to CHANGELOG.d.
