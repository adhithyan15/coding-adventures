---
category: Native extensions & FFI
---

# WASI / WASM host-side bounds

`iovs_len`, per-buffer length, total read/write bytes, `random_get` `buf_len`, function arity, data-segment sizes — all are guest-controlled and must be capped before allocation, slicing, or invoking host providers. Validate every length against remaining section bytes AND a package-level max.
