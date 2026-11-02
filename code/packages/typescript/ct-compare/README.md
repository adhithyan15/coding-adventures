# typescript/ct-compare

Constant-time byte equality, byte selection, and unsigned 64-bit equality helpers.

## Accepted inputs

Each byte input must be a genuine `Uint8Array` (Node's `Buffer` included), a
genuine `ArrayBuffer`, or a plain array of integers from 0 to 255. Anything
else throws a `TypeError` or `RangeError` instead of being coerced: coercion
once let two different inputs compare equal. A typed array is recognised by
its real internal slots and copied; look-alike objects and proxies over typed
arrays are rejected. A `Proxy` or subclass of a plain `Array` is accepted, but
its `length` and each element are read exactly once and validated.

`choice` in `ctSelectBytes` must be a real boolean.

**Pass a `Buffer`, not its `.buffer`.** A small `Buffer` lives inside Node's
shared allocation pool, so `buf.buffer` is the whole pool, not your bytes.
