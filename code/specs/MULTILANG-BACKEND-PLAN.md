# Multi-language backend coverage plan

**Date:** 2026-06-01.  Drafted in response to the user's request to
plan + execute "get every IIR-supported language onto JVM / CLR /
BEAM / WASM, add LLVM, add AOT debugging".

## 1. Current coverage matrix

Based on `tests/backend_encode.rs` (PR #4735 just landed): real-encoder
output, not just validator handshake.

| Language    | wasm                 | jvm                 | clr                 | beam               |
|-------------|----------------------|---------------------|---------------------|--------------------|
| Twig        | ✅ (task #17)         | ✅ (task #17)        | ✅ (task #17)        | ✅ (task #17)       |
| Brainfuck   | ✅ (task #12)         | ✅ (task #13)        | ✅ (task #14)        | ❌ intentional (#16) |
| BASIC arith | ✅                    | ✅                   | ✅                   | n/a               |
| BASIC IF/FOR| ⚠️ wasm lower no cmp | ✅                   | ✅                   | n/a               |
| BASIC PRINT | ❌ host import        | ❌ host import       | ❌ host import       | n/a               |
| Nib         | ✅                    | ✅                   | ✅                   | n/a               |
| Oct         | ✅                    | ✅                   | ✅                   | n/a               |

`n/a` columns (BEAM for everyone except Twig) are **intentional**: the
languages don't fit the actor/immutable-binding model.  Validator
acceptance is proven (`backend_compat.rs`), so the IR shape is
portable, but we don't claim execution.

## 2. Concrete gaps (with patches identified)

### G1: `iir-to-wasm` is missing typed `cmp_*` opcode lowerings

**Symptom:** `IIR -> WasmModule: UnsupportedOp { op: "cmp_gt" }` (and
the rest of the cmp family).

**Fix:** Add a match arm in `iir-to-wasm/src/lower.rs` (around
line 782 where `add | sub | mul | div | rem` already live) that maps:

```text
cmp_eq   → I64_EQ   (0x51)
cmp_ne   → I64_NE   (0x52)
cmp_lt   → I64_LT_S (0x53)   (or I32_LT_S for i32 hint)
cmp_le   → I64_LE_S (0x57)
cmp_gt   → I64_GT_S (0x55)
cmp_ge   → I64_GE_S (0x59)
```

Per-type hint dispatch the same way `add` already does (`is_i64_hint`,
`is_float_hint`, default i32).  Result is i32 (wasm comparisons return
i32 truthy/zero).

**Validator:** already accepts cmp_* (only the *lower* step rejects);
no validator changes needed.

**Tests:** flip the two `#[ignore]`d tests in
`dartmouth-basic-iir-compiler/tests/backend_encode.rs` to active,
plus a fresh `wasm_cmp_*` op-level test in the crate.

**Estimated effort:** 1 PR.  Small.

### G2: `iir-to-wasm` is missing the `print_i64` host import

**Symptom:** `wasm validator must accept BASIC IIR; got
["UnsupportedOp ... print_i64 ... not in the WASM backend's
host-import whitelist (supported: ["putchar", "getchar"])"]`.

**Fix:** Add `"print_i64"` to `CALL_BUILTIN_SUPPORTED_NAMES` and
declare a corresponding import in the wasm module (`env.print_i64 :
[i64] -> []`).

**Tests:** flip the `print_is_blocked_until_backends_whitelist_print_i64`
regression marker (it should start failing — meaning PRINT now works).

**Estimated effort:** 1 PR.  Small.

### G3: `iir-to-jvm-class-file` is missing the `print_i64` host import

**Fix:** Mirror G2 in the JVM backend.  Add to whitelist; declare an
`env/BasicRuntime.println(J)V`-style method ref + invokestatic.

**Estimated effort:** 1 PR.  Small/medium (need to invent the host
runtime class name convention).

### G4: `iir-to-cil-bytecode` is missing the `print_i64` host import

**Fix:** Mirror G2/G3 in the CLR backend.  Use
`System.Console.WriteLine(Int64)` as the underlying host method.

**Estimated effort:** 1 PR.  Small.

### G5: Twig real-encoder smoke tests don't exist (only validator)

**Symptom:** task #17 covered Twig → IIR-to-* end-to-end at the
*validator* level, but unlike Brainfuck there's no
`twig-ir-compiler/tests/wasm_e2e.rs` etc. that actually lowers and
asserts magic-prefix bytes.

**Fix:** add `backend_encode.rs` to `twig-ir-compiler/tests/`
matching the BASIC/Nib/Oct pattern from PR #4735.

**Estimated effort:** 1 PR.  Small.

### G6: BASIC FOR/NEXT does not lower to CLR

Closer look needed; the `basic_for_loop_lowers_to_clr_assembly` test
passes, but does FOR/NEXT exercise cmp_le?  The CLR lower handles
cmp ops, so this is likely already covered.  Treat as **probably
already done**; verify after G1 lands.

### G7: BEAM emission for procedural languages

**Decision:** explicitly defer.  Same posture Brainfuck took (task #16).
The validator passes; we document that "running on BEAM requires an
actor-model rewrite of the source program" and leave it there.

## 3. LLVM backend addition

### Design choice: textual LLVM IR vs llvm-sys

We will start with **textual LLVM IR (`.ll`) emission**.  Reasons:

1. Zero external crate dependency (no llvm-sys / inkwell build
   complexity).
2. Easy to inspect output by hand.
3. Easy to test (compare emitted text against fixtures).
4. The final `llc` step is `/usr/bin/llc` or `clang -c`; that's a
   system tool, like the linker we already shell out to.

A future PR can swap the text emitter for an in-process llvm-sys
binding if performance matters.

### Crate plan

Create a new crate `iir-to-llvm` mirroring the structure of
`iir-to-wasm`:

```text
code/packages/rust/iir-to-llvm/
  Cargo.toml               # dev-deps: interpreter-ir
  BUILD                    # cargo test -p iir-to-llvm
  README.md
  CHANGELOG.md
  src/
    lib.rs                 # public API
    validate.rs            # validate_for_llvm(&IIRModule) -> Vec<String>
    lower.rs               # lower_iir_to_llvm(&IIRModule, &cfg) -> Result<LLVMModule>
    text.rs                # LLVMModule::to_string() -> String
  tests/
    smoke.rs               # canonical examples → text → assert
```

### Phases

- **LLVM01:** `validate_for_llvm` + minimal lower for `const_i64 / ret_i64`
  + textual emission.  Smoke test: `fn main() -> i64 { return 42; }`
  emits `define i64 @main() { ret i64 42 }` and the text passes
  `llc --filetype=null /tmp/x.ll` if `llc` is on PATH (test skipped
  otherwise — mirrors how `lang-aot` smoke tests gate on the system
  linker).
- **LLVM02:** add typed arithmetic (`add / sub / mul / div / rem`),
  typed comparisons (`cmp_*`), control flow (`label / jmp / jmp_if_*`).
- **LLVM03:** add cross-function `call`, local variables (`mov` ↔
  `alloca`+`store`+`load`), `print_i64` builtin (using `declare
  void @print_i64(i64)` + a tiny host shim in `lang-aot`).
- **LLVM04:** wire into `lang-aot` as an optional `--target=llvm`
  emitter; produce `.ll` text on stdout or to a file.

LLVM01 alone unblocks the architectural claim "we have an LLVM
backend"; LLVM02-04 fill it in incrementally.

## 4. AOT debugging plan

**Good news:** `native-debug-info` already exists (LANG14).  It
emits DWARF 4 + CodeView 4 and embeds them into ELF / Mach-O / COFF.

### Current state

- `native-debug-info` produces the debug sections from a
  `debug-sidecar` byte blob.
- We have per-language source-loc threading already merged
  (BASIC #4587, Nib #4590, Oct #4583 — and Twig has it from earlier).
- `IIRFunction::source_map` is populated by every frontend.

### Gap

Wiring is incomplete: the AOT backends (`x86_64-backend` / `aarch64-backend`)
don't currently consult `IIRFunction::source_map` to drive
`native-debug-info`'s DWARF emission and embed the sections into the
object they hand to the linker.

### Phases

- **AOT-DBG-01:** audit — produce a doc in `code/specs/` that
  spells out the exact wiring path (where in `twig-aot` /
  `aarch64-backend` / `x86_64-backend` the call sites would live).
  Skip if the audit finds the wiring already exists in a
  partly-disabled state.
- **AOT-DBG-02:** plumb `IIRFunction::source_map` →
  `debug-sidecar` blob → `DwarfEmitter` → output object's
  `.debug_*` sections.  Single-function programs first
  (single-CU).
- **AOT-DBG-03:** smoke test: compile a BASIC program with
  `--debug`, run `objdump -W <exe>` and assert at least one
  `.debug_line` row maps to the expected source line.
- **AOT-DBG-04:** native-DAP integration — extend the existing
  `basic-dap` / `nib-dap` / `oct-dap` so `launch_vm` can take
  `gdbserver <port> -- <native_exe>` as the spawned process
  instead of the VM, when the user is debugging an AOT binary.
  This re-uses the existing DAP infrastructure for native debugging.

## 5. Ordered execution sequence

Items are sized as roughly-1-PR steps so the babysitter loop can
chew through them.

1. **G1**: `iir-to-wasm` cmp opcode lowerings.  Unblocks BASIC IF/FOR.
2. **G5**: Twig real-encoder smoke tests (mirror BASIC/Nib/Oct shape).
3. **G2**: `iir-to-wasm` print_i64 host import.
4. **G3**: `iir-to-jvm` print_i64 host import.
5. **G4**: `iir-to-clr` print_i64 host import.
6. **LLVM01**: iir-to-llvm crate skeleton + return-42.
7. **LLVM02**: arithmetic + cmp + control flow.
8. **LLVM03**: calls + locals + print_i64.
9. **LLVM04**: lang-aot --target=llvm wiring.
10. **AOT-DBG-01**: audit + doc.
11. **AOT-DBG-02**: DWARF emission wiring.
12. **AOT-DBG-03**: smoke test (objdump -W).
13. **AOT-DBG-04**: native DAP integration.

Each PR ships per the repo standards: spec → tests → impl →
CHANGELOG → README → security-review → push → babysitter cron.

After (13), the multi-backend story is end-to-end across every
language on every relevant backend (BEAM intentionally out of scope
for procedural languages, documented).

## 6. Out of scope (for this plan)

- Cross-function calls in BASIC (BASIC has no user-defined functions
  in V1 beyond DEF FN, which is deferred).
- BEAM support for non-actor languages (intentional, documented).
- Floating-point on backends that don't have it yet (BASIC is
  integer-only V1; deferred).
- llvm-sys / inkwell integration (deferred — text emission first).
- macOS code signing for AOT binaries (orthogonal, separate work).
