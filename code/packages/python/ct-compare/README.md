# python/ct-compare

Constant-time byte equality, byte selection, and unsigned 64-bit equality helpers.

## Accepted inputs

Each byte input must be exactly `bytes`, `bytearray`, a byte-format
`memoryview`, or a `list` or `tuple` of `int` values from 0 to 255 (`bool` is
not accepted). Anything else, including a subclass of those types, raises
`TypeError` or `ValueError` instead of being coerced: coercion once let two
different inputs compare equal, and a subclass could misreport its bytes.

`choice` in `ct_select_bytes` must be a real `bool`.
