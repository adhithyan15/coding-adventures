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

## 0.1.1 — 2026-05-05

### Added
- `mov_<ty>` lowering — typed register-to-register move (load + store
  via the stack-spill regalloc).  Used by aot-core when lowering
  `call_builtin "_move"`.

### Fixed
- **Stack frame layout bug**: virtual register slot 0 was at `[sp + 0]`,
  but the prologue's `stp fp, lr, [sp, #-frame]!` saves `fp` at the
  same offset.  The first `str x0, [sp]` clobbered the saved `fp`,
  so the function's `ldp fp, lr, [sp], #frame` epilogue restored a
  garbage `fp` and `ret` returned to a garbage address — instant
  SIGSEGV.

  Fix: virtual slot offsets now start at +16 to leave room for the
  saved `fp/lr`.  The frame-size cap drops from 4080 to 504 bytes —
  reflecting the actual `stp_pre`/`ldp_post` 7-bit signed immediate
  range (the prior 4080 was wishful thinking).

### Note

The fix is what made real Twig programs (`(+ 30 12)`, `(if ...)`)
actually run end-to-end on Apple Silicon.  Pre-fix, the encoder + IR
+ Mach-O writer were all correct, but the program SIGSEGV'd on return
because of the saved-fp clobber.
