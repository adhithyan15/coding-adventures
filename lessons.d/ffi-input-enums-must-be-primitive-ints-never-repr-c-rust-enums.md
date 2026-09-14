---
category: Rust
---

# FFI input enums must be primitive ints, never `repr(C)` Rust enums

Foreign callers can pass any bit pattern; observing an out-of-range Rust enum is UB before validation runs. Use `u32`/`c_int` in the ABI struct, then `TryFrom`.
