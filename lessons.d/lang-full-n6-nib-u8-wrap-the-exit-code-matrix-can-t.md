# LANG-FULL N6 (Nib u8 wrap) — the exit-code matrix can't prove a u8 wrap

**Date:** 2026-06-15

Attempted N6 (make `200u8 + 100u8 = 44` run cross-backend). Three findings that
block a HONEST proof — recorded so the next attempt doesn't ship a false positive:

1. **`Expect::Exit` is `& 0xFF` — it CANNOT distinguish a u8 wrap from no wrap.**
   `lang_matrix.rs` reads the process exit code, which the OS/C-runtime truncates
   to 8 bits (documented at the top of the file: "process exit code (`& 0xFF`)").
   `200 + 100 = 300`; `300 & 0xFF = 44` **whether or not the backend masks**. So a
   Nib `fn main() -> u8 { return 200 + 100; }` "passes" `Expect::Exit(44)` even when
   the arithmetic is plain i64 with no E2 mask. For u8 the exit-code truncation IS
   the u8 mask, so the test proves nothing. u16/u32 are worse — their masked values
   (4464, …) exceed 255 and can't ride an exit code at all. **A real N6 proof needs
   the value at full width**: either a stdout print (`Expect::Stdout`, but Nib has
   no `print`/`out` — Oct does, via O-OUT) or a return-value-at-width harness like
   `iir-to-wasm/tests/width_wrap.rs` (which calls `load_and_run` and reads the raw
   i64 result, NOT an exit code). Decide the proof mechanism BEFORE writing N6.

2. **The Nib `type_hint` lookup was a silent no-op.** `arith_result_hint` /
   `lookup_node_type` consult `types: HashMap<usize, NibType>` keyed by AST-node
   pointer address. Tagging the `add` op with the inferred `u8` produced `i64`
   anyway (unit-tested: the emitted `add` carried `"i64"`, not `"u8"`). Either the
   checker doesn't type the `add_expr` node, or the pointer keys don't match the
   nodes the compiler walks (the checker types one AST, `compile_typed` may walk a
   moved/rebuilt one). Verify the lookup actually returns `Some(U8)` (a direct unit
   test on the emitted hint) BEFORE relying on it — the i64 collapse hid this for
   every prior Nib item because everything was i64 regardless.

3. **JVM + CLR E2 mask legs were only structurally tested and don't fire for the
   real shape.** Their executed proof was explicitly deferred to "the integration
   PR." With the universal shape (i64 slots + narrow hint on the op — see
   `iir-to-llvm`'s `e2_*` tests), a u8 add over `long` operands returns the unmasked
   value on JVM (`iir_type_to_jvm("u8") = Int` → `IADD`, but operands ride `long`
   slots; the `iand` mask path doesn't match). Native/LLVM/WASM/VM/JIT mask
   correctly; JVM/CLR need a fix for the i64-operand case. These are two real
   remaining E2 backend legs, not done.

Net: N6 is a multi-part item (proof-mechanism design + Nib type-lookup fix + JVM/CLR
backend fixes), NOT a one-line frontend wiring. The native-AOT E2 leg (PR #5887,
merged) IS real — its `aarch64-backend` proof installs and *calls* the generated
code and reads the raw u64 (44), so it is not exit-code-confounded.
