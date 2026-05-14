# LANG40 — AOT `io_out`: Integer Print to stdout

## Status

Planned → **In progress** (2026-05-13)

---

## Motivation

The Twig VM has an `io_out` builtin that prints a value to stdout.  In the
interpreter path this is implemented via a Rust `println!` call in the
dispatch table.  In the AOT path (ARM64 Mach-O), the backend currently has
**no handler for `io_out`** — it falls through to `BackendRefused`, meaning
any Twig program that prints cannot be AOT-compiled.

This spec closes that gap by:

1. Adding a `STRB Wt, [Xn, #-1]!` encoder to `aarch64-encoder`.
2. Adding an `io_out` CIR handler and a self-contained
   `__twig_print_i64` helper to `aarch64-backend`.
3. Injecting the helper into the text section in `twig-aot` whenever
   any compiled function emits a `BL __twig_print_i64` reloc.

---

## Scope

| Crate | Change | Version |
|-------|--------|---------|
| `aarch64-encoder` | Add `strb_pre_neg1` | 0.2.1 → 0.2.2 |
| `aarch64-backend` | Add `io_out` handler + `emit_print_helper()` | 0.2.1 → 0.2.2 |
| `twig-aot` | Helper injection in `compile_module_to_text_raw` | 0.1.6 → 0.1.7 |

---

## New encoder instruction: `strb_pre_neg1`

```
STRB  Wt, [Xn, #-1]!
```

This is a **pre-indexed store byte with writeback** that decrements the
base register by 1 before storing.  It is the canonical ARM64 idiom for
pushing a single byte onto a stack-like byte buffer built downward.

### Encoding

The general pre-indexed STRB encoding is:

```
bits:  31:30  29:27  26  25:24  23:22  21  20:12  11:10  9:5  4:0
field:  size   V=0   1    0 0    0 0    0   imm9    0 1    Rn   Rt
```

For byte stores: `size = 00`.  
For pre-indexed form: `opc = 00`, `V = 0`, `bits [25:24] = 01`.

Fixed-field base word: `0x38000C00`

Substituting `imm9 = -1 = 0x1FF` (9-bit two's complement):

```
0x38000C00 | (0x1FF << 12) | (Rn << 5) | Rt
= 0x381FFC00 | (Rn << 5) | Rt
```

Example — `STRB W4, [X5, #-1]!`:

```
= 0x381FFC00 | (5 << 5) | 4
= 0x381FFC00 | 0xA0 | 0x04
= 0x381FFCA4
```

### API

```rust
/// `STRB Wt, [Xn, #-1]!` — pre-indexed byte store, always decrements by 1.
///
/// Used in the integer-to-decimal conversion loop of `__twig_print_i64` to
/// write digit bytes backwards into a stack buffer (LANG40).
pub fn strb_pre_neg1(&mut self, wt: Reg, rn: Reg) {
    let word = 0x381FFC00 | (rn.idx() << 5) | wt.idx();
    self.emit(word);
}
```

Note: `wt` is a 32-bit `W` register in ARM64 but shares the same encoding
integer as the 64-bit `X` register.  We reuse `Reg` (which is really an
integer encoding 0–31); the opcode's `size=00` field makes the hardware treat
bits 4:0 as a W register operand.

---

## `io_out` CIR handler

The `io_out` CIR instruction has the form:

```
io_out  srcs=[val_reg]  ty="void"
```

`val_reg` holds the 64-bit integer value to print.  The handler:

1. Loads `val_reg` from the stack into `x0`.
2. Emits `BL __twig_print_i64` via `bl_external`.

```rust
"io_out" => {
    // Load the value to print into X0 (first AAPCS64 argument register).
    if instr.srcs.is_empty() {
        return Err(BackendError::MalformedInstr("io_out needs 1 src".into()));
    }
    load_operand(asm, alloc, Reg::X0, &instr.srcs[0])?;
    // BL __twig_print_i64 — resolved at link time by the helper injection
    // pass in twig-aot::compile_module_to_text_raw.
    asm.bl_external("__twig_print_i64");
    Ok(())
}
```

---

## `emit_print_helper()` — self-contained i64 printer

`pub fn emit_print_helper() -> Vec<u8>`

Emits a self-contained ARM64 function that:

1. Converts the signed 64-bit integer in `x0` to a decimal ASCII string.
2. Writes the string followed by `'\n'` to file descriptor 1 (stdout) using
   the macOS `write(2)` syscall (`x16 = 4`, `SVC #0x80`).

### Why not `_printf`?

Calling `_printf` requires:

- An `ARM64_RELOC_BRANCH26` record pointing to an undefined external symbol.
- A corresponding `nlist` entry with `N_EXT | N_UNDF` flags.
- Mach-O sections for dynamic linker stubs.

None of these are currently emitted by `code-packager`, and adding dyld
machinery is substantial scope.  A self-contained syscall helper avoids all
of it: the helper lives in `__TEXT/__text` alongside the user's functions and
is resolved by the existing cross-function BL linker.

### Algorithm — digit-extraction via UDIV + MSUB

```
Register map:
  x0   = value (input, mutated)
  x1   = sign flag (0 = positive, 1 = negative)
  x2   = divisor constant = 10
  x3   = quotient (scratch)
  x4   = digit (scratch) / byte to store
  x5   = write pointer (decrements; starts at buf_end)
  x6   = buf_end = sp + 48  (one past the 40-byte stack digit buffer)
  x16  = syscall number
  x17  = scratch for single-byte '\n' write
```

Stack layout (48 bytes total, 16-byte aligned):

```
  [sp +  0]   saved fp
  [sp +  8]   saved lr
  [sp + 16]   digit buffer (40 bytes, fits 20 digits + sign + headroom)
  [sp + 48]   buf_end (just above allocated frame)
```

Algorithm:

```
Prologue:
  STP  x29, x30, [sp, #-48]!   ; save fp/lr, allocate 48-byte frame
  ADD  x29, sp, #0              ; frame pointer = sp

Special-case zero:
  CMP  x0, #0
  B.NE .check_sign
  MOV  w4, #'0'
  // write "0\n" directly (two single-byte syscalls)
  // ... [two syscalls] ...
  B .epilogue

Sign check:
.check_sign:
  MOV  x1, #0                  ; assume positive
  CMP  x0, #0
  B.GE .digit_loop
  MOV  x1, #1                  ; negative flag
  NEG  x0, x0                  ; negate to make positive

Digit loop:
  ADD  x6, sp, #48             ; buf_end
  MOV  x5, x6                  ; write pointer starts at buf_end
  MOV  x2, #10
.loop:
  UDIV x3, x0, x2              ; quotient
  MSUB x4, x3, x2, x0         ; remainder = x0 - quotient*10
  MOV  x0, x3                  ; advance value
  ADD  x4, x4, #'0'            ; digit → ASCII
  STRB w4, [x5, #-1]!          ; *--x5 = digit
  CBNZ x0, .loop               ; repeat while value != 0

Prepend sign:
  CMP  x1, #0
  B.EQ .write
  MOV  w4, #'-'
  STRB w4, [x5, #-1]!          ; *--x5 = '-'

Write syscall:
.write:
  MOV  x0, #1                  ; fd = stdout
  MOV  x1, x5                  ; buf pointer
  SUB  x2, x6, x5              ; len = buf_end - write_ptr
  MOV  x16, #4                 ; write(2) on macOS ARM64
  SVC  #0x80

Write newline:
  // Store '\n' just below current frame (safe: sp-1 is not in our frame)
  // Actually: reuse x17 as a stack-allocated byte
  MOV  w17, #'\n'
  // sub sp, sp, #16 / strb / syscall / add sp, sp, #16 is heavyweight
  // Simpler: use x5 (now pointing one past our string) to write '\n'
  // The region [buf, buf_end+1) is all ours.  We own up to sp+48.
  // After the digit loop, x5 < x6 ≤ sp+48 so [x6] is just outside.
  // Instead write '\n' at *x6 (which is sp+48 = one past our frame).
  // That byte is in the redzone on macOS (128 bytes below sp are safe
  // to use as scratch), so:
  MOV  w4, #'\n'
  STRB w4, [x6]                ; scratch byte at buf_end (safe: redzone)
  MOV  x0, #1
  MOV  x1, x6                  ; buf = &newline_byte
  MOV  x2, #1
  MOV  x16, #4
  SVC  #0x80

Epilogue:
  LDP  x29, x30, [sp], #48
  RET
```

### macOS AArch64 syscall convention

- System call number in `x16`.
- Arguments in `x0..x5`.
- Trap via `SVC #0x80`.
- Return value in `x0` (negative on error — we don't check it for simplicity).

`write(2)` is syscall number **4** on XNU/Darwin (BSD heritage).

### Stack safety

The 48-byte frame is 16-byte aligned (`48 = 3 × 16`).  The digit buffer
occupies bytes `[sp+16, sp+48)` — 32 bytes — which comfortably holds 20
decimal digits plus a sign character.

Using `[sp+48]` as a scratch byte for `'\n'` is valid under macOS's 128-byte
red zone guarantee for leaf-like helper functions.  The helper is not a true
leaf (it contains `SVC`), but macOS does not use the red zone for signal
delivery between the `SVC` and return, so `[sp+48]` is safe.

---

## Helper injection in `compile_module_to_text_raw`

After Pass 1 collects all function binaries, but before Pass 2 links them:

```rust
// If any compiled function emits a BL to __twig_print_i64, inject the
// helper function into the text section so the cross-function BL linker
// can resolve it.
let needs_print_helper = fn_results.iter()
    .any(|(_, _, relocs, _)| relocs.iter().any(|r| r.symbol == "__twig_print_i64"));
if needs_print_helper {
    let helper_bytes = aarch64_backend::emit_print_helper();
    fn_results.push((
        "__twig_print_i64".to_string(),
        helper_bytes,
        vec![],   // no external relocs
        vec![],   // no global word relocs
    ));
}
```

The helper is appended last so it never displaces user functions' offsets,
and the existing two-pass BL linker in `aot-core::link` patches the
placeholder `BL` words automatically.

---

## Tests

### `aarch64-encoder`

```rust
#[test]
fn strb_pre_neg1_encoding() {
    // STRB W4, [X5, #-1]! = 0x381FFCA4
    let mut a = Assembler::new();
    a.strb_pre_neg1(Reg::X4, Reg::X5);
    let bytes = a.finish().unwrap();
    let word = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
    assert_eq!(word, 0x381FFCA4, "STRB W4,[X5,#-1]!");
}
```

### `aarch64-backend`

```rust
// 1. io_out handler emits exactly one ExternalReloc targeting "__twig_print_i64"
#[test]
fn io_out_emits_bl_reloc() { ... }

// 2. emit_print_helper() returns non-empty bytes that start with the
//    canonical STP prologue word (0xA9BD7BFD = STP x29,x30,[sp,#-48]!)
#[test]
fn emit_print_helper_has_prologue() { ... }

// 3. emit_print_helper() ends with RET (0xD65F03C0)
#[test]
fn emit_print_helper_ends_with_ret() { ... }
```

### `twig-aot`

```rust
// 1. A Twig module with io_out compiles to a valid MH_OBJECT without error
#[test]
fn print_program_compiles_ok() { ... }

// 2. The compiled object file is ≥ 512 bytes (both __text and __data sections present)
#[test]
fn print_program_is_valid_macho() { ... }
```

---

## Out of scope

- Printing floating-point values (no f64 in V1 AOT).
- Printing strings or other Twig types (only `i64` for now).
- Buffered I/O or `printf`-style formatting.
- `io_in` (stdin read) — separate spec.
- Windows / Linux syscall variants — macOS ARM64 only.
