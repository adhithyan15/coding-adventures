## 0.2.0

- **Security fix.** Inputs were converted with `bytes(value)`, and `bytes(n)` of
  an integer makes `n` zero bytes, so an integer and a run of zero bytes
  compared equal. Inputs are now validated: `bytes`, `bytearray`, byte-format
  `memoryview`, or a list or tuple of `int` 0..255 (not `bool`); anything else
  raises `TypeError` or `ValueError`. **Breaking:** other `Sequence[int]` types
  and non-byte `memoryview`s are no longer accepted.
- **Security fix.** `ct_select_bytes` branched on the secret `choice` bit, which
  CT01 forbids. The mask is now `-int(choice) & 0xFF`, and a non-`bool` choice
  raises `TypeError` instead of being truthiness-tested.
- `ct_eq_u64` rejects `bool`.
- Regression tests for every rejected input shape and for fresh-copy output.
