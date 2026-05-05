# Changelog — `aarch64-backend`

## 0.1.0 — 2026-05-05

Initial release.  ARM64 native-code backend for jit-core / aot-core,
implementing the shared `Backend` trait via `Backend::compile_function`.

### Implemented CIR coverage

- Constants: `const_u8` … `const_u64`, `const_i8` … `const_i64`, `const_bool`
- Integer arithmetic (typed): `add_<ty>`, `sub_<ty>`, `mul_<ty>`
- Comparisons: `cmp_eq_<ty>` … `cmp_ge_<ty>` (signed and unsigned)
- Control flow: `label`, `jmp`, `jmp_if_true`, `jmp_if_false`
- Returns: `ret_<ty>`, `ret_void`
- Type guards: `type_assert` lowered to `udf` trap

### Register allocation

Stack-spill: every CIR virtual register lives at a fixed 8-byte stack slot.
Each instruction loads sources into scratch `x0..x2`, performs the op, and
stores the destination back.  Trivially correct; suboptimal performance.
A real allocator can replace it without changing the public API.

### AAPCS64 prologue / epilogue

```
stp  fp, lr, [sp, #-frame]!
mov  fp, sp
str  x0..x7, [sp, #(slot)]    ; spill incoming args
<body>
ldp  fp, lr, [sp], #frame
ret
```

Up to 8 parameters are supported.  Frame must fit a 12-bit unsigned offset
(≈ 4088 bytes / ~512 virtual registers).

### Out of scope (deferred)

- Float operations
- `call_runtime`, `send`, `load_property`, `store_property`
- Width-truncation for u8/u16/u32 results
- Real register allocation
