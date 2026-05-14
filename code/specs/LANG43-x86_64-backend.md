# LANG43 — `x86_64-backend`: X86-64 Native Backend

**Status:** Draft — 2026-05-14

## Motivation

`aarch64-backend` brings the LANG VM to ARM64 hosts (Apple Silicon,
modern Linux ARM).  `x86_64-backend` brings it to x86-64 hosts —
which today dominate CI runners, cloud VMs, and developer laptops.

This spec defines the backend that consumes CIR via the `Backend`
trait (LANG05) and produces x86-64 machine code via `x86_64-encoder`
(LANG41).  It plugs into both `aot-core` (LANG04) and `jit-core`
(LANG03) without changes to either — the `Backend` trait is
target-agnostic.

The goal of V1 is **functional parity with `aarch64-backend`** after
LANG38 (arithmetic completeness) + LANG39 (globals) + LANG40 (I/O):
every CIR opcode currently supported on ARM64 must also compile on
x86-64.  Subsequent PRs match `aarch64-backend` PR-for-PR; this spec
lays out the V1 cut and the major design decisions.

## Non-goals (V1)

- **Floating-point / SSE / AVX** — `aarch64-backend` has no float
  support either.  Defer until both backends grow it together.
- **Closures** (LANG34/35) — defer; mirrors the AArch64 path's
  `closures: NOT YET` line.
- **Local-register allocator** — V1 uses stack spill exclusively
  (matches AArch64).  A real allocator can replace the spill core
  later behind the same public API.
- **Microsoft x64 ABI** — V1 targets **System V AMD64 ABI** only
  (Linux, macOS x86-64, FreeBSD).  Windows x86-64 is a follow-up;
  see § Out of scope below.

## Package layout

```
code/packages/rust/x86_64-backend/
├── Cargo.toml          # deps: x86_64-encoder, jit-core, vm-core
├── README.md
├── CHANGELOG.md
└── src/
    └── lib.rs
```

Mirror of `aarch64-backend`'s structure.

---

## Backend trait implementation

```rust
#[derive(Debug, Default, Clone, Copy)]
pub struct X86_64Backend;

impl X86_64Backend {
    pub fn new() -> Self { X86_64Backend }
}

impl Backend for X86_64Backend {
    fn name(&self) -> &str { "x86_64" }

    fn compile_function(
        &self,
        ctx: &FunctionContext,
        ir: &[CIRInstr],
    ) -> Option<CompileResult> { /* … */ }

    fn compile(&self, _ir: &[CIRInstr]) -> Option<Vec<u8>> {
        // Same "needs context" rationale as AArch64:
        // we can't lay out a prologue without knowing arg count.
        None
    }
}
```

`CompileResult` carries the byte stream plus the relocation list the
packager will resolve (`ExternalReloc` from `x86_64-encoder`,
re-exported as `Reloc` for parity with the AArch64 backend's API).

---

## ABI: System V AMD64

V1 conforms strictly to the System V AMD64 ABI as documented in the
draft revision 1.0 (Matz et al.).  The relevant rules for the LANG
VM's integer-only V1:

- **Argument registers** (in order): `RDI`, `RSI`, `RDX`, `RCX`,
  `R8`, `R9`.  Up to six integer/pointer arguments fit in registers;
  beyond that, push right-to-left on the stack.
- **Return value**: `RAX`.
- **Caller-saved (volatile)**: `RAX`, `RCX`, `RDX`, `RSI`, `RDI`,
  `R8`–`R11`.  Subset our spill allocator may freely clobber as
  scratch.
- **Callee-saved**: `RBX`, `RBP`, `R12`–`R15`.  V1 only clobbers
  `RBP` (used as frame pointer), so we save and restore it in the
  prologue/epilogue.
- **Stack alignment**: 16-byte-aligned **at the point of a `CALL`
  instruction** (so on entry, RSP ≡ 8 (mod 16) because the CALL
  pushed an 8-byte return address).  After `push rbp`, RSP ≡ 0 (mod
  16) — the prologue's frame allocation must keep it that way.
- **Red zone**: 128 bytes below RSP are scratch for leaf functions.
  V1 *does not* use the red zone; all locals live in the proper
  frame.

### Prologue / epilogue

```asm
;  --- prologue ---
push  rbp
mov   rbp, rsp
sub   rsp, <frame>          ; <frame> rounded up to 16-byte multiple

;  spill arg registers to their stack slots
mov   [rbp - 8 - N*8 - 0],  rdi      ; arg 0
mov   [rbp - 8 - N*8 - 8],  rsi      ; arg 1
... up to 6 args ...
                                     ; arg 7+ already on stack, accessed via [rbp + 16 + ...]

;  --- body ---

;  --- epilogue ---
mov   rsp, rbp                       ; or `add rsp, <frame>`; mov is one byte shorter for large frames
pop   rbp
ret
```

`<frame>` = `(num_virtuals * 8 + 15) & !15`.  Sub-128B frames could
use the red zone, but uniformity wins for V1.

### Stack alignment invariant

After `push rbp; sub rsp, frame`, RSP must be `≡ 0 (mod 16)`.  Since
`push rbp` shifts the alignment by 8, we need `frame ≡ 8 (mod 16)`.
The allocator rounds `frame` up to 16 *and then adds 8 if needed*.
The corresponding check is part of the test suite.

When calling out (e.g., `__twig_print_i64`), the call site adjusts
RSP if necessary to restore the 16-byte alignment at the call.  In
V1, all locals are 8-byte slots and we don't pass stack arguments to
external calls, so this works out naturally.

---

## Register allocation: stack spill (V1)

Identical strategy to `aarch64-backend`:

1. Each CIR virtual register `v0`, `v1`, … is assigned a fixed
   8-byte stack slot at `[rbp - 8 - slot_idx * 8]` (so `v0` is at
   `[rbp - 8]`, `v1` at `[rbp - 16]`, …).
2. Every instruction:
   - Loads its source operands into scratch registers (`RAX`, `RCX`,
     `RDX` reserved as the three-way scratch set).
   - Performs the operation.
   - Stores the destination back to its stack slot.
3. Constants are materialised into a scratch register fresh each
   use.

The choice of `RAX`/`RCX`/`RDX` as scratch is deliberate:

- `RAX` is the return register; useful to dock results there
  directly when the next instr is `ret`.
- `RCX` is the only valid shift-count register (`SHL r/m, CL`); having
  it in the scratch set means shift lowering doesn't need to move it
  around.
- `RDX` is the high half of the implicit dividend (`RDX:RAX`) for
  `IDIV`/`DIV`; the divide lowering can sequence `CQO` (sign-extend
  RAX into RDX) without touching anything else in the scratch set.

`RBX`, `R12`–`R15` are callee-saved but unused by V1 — leaving them
preserved means we don't have to save them in the prologue.

---

## CIR opcode coverage (V1)

Matches `aarch64-backend` after LANG38+LANG39+LANG40.  See the table
in `aarch64-backend/src/lib.rs` doc-comment for the canonical list.
The mapping below shows how each family lowers on x86-64.

### Constants

| CIR | x86-64 lowering |
|---|---|
| `const_u8 .. const_u64` | `MOV reg, imm` (sign-extended imm32 for small, `MOVABS reg, imm64` for full) → `MOV [slot], reg` |
| `const_i8 .. const_i64` | Same |
| `const_bool` | `MOV reg, 0` or `MOV reg, 1` → `MOV [slot], reg` |

### Integer arithmetic

| CIR | Lowering |
|---|---|
| `add_<ty>` | `MOV rax, [lhs]; ADD rax, [rhs]; MOV [dst], rax` |
| `sub_<ty>` | `MOV rax, [lhs]; SUB rax, [rhs]; MOV [dst], rax` |
| `mul_<ty>` | `MOV rax, [lhs]; IMUL rax, [rhs]; MOV [dst], rax` |

V1 uses 64-bit ops for *every* typed integer mnemonic — exactly like
`aarch64-backend`'s "result is not masked to declared width" rule.
A later PR can add explicit masking.

### Division and modulo (matches LANG38 wave)

x86-64 division is **awkward**: `IDIV r/m64` divides `RDX:RAX` by
`r/m64`, yielding quotient in `RAX`, remainder in `RDX`.  We must:

1. Load dividend into `RAX`.
2. Sign-extend (`CQO`) or zero (`XOR RDX, RDX`) into `RDX`.
3. `IDIV [divisor]` or `DIV [divisor]`.
4. Move `RAX` (for `div`) or `RDX` (for `mod`) into the destination
   slot.

For signed:

```
mov  rax, [lhs]
cqo                     ; RDX:RAX = sign-extend(RAX)
mov  rcx, [rhs]
idiv rcx
mov  [dst], rax         ; or [dst], rdx for mod
```

For unsigned, replace `cqo` with `xor rdx, rdx` and `idiv` with
`div`.

The `RCX` move is necessary because `IDIV r/m64` cannot use an
immediate operand and `[rhs]` lives in memory — loading into `RCX`
keeps the encoding straightforward (rather than `IDIV qword ptr
[rbp - ...]` which is encodable but adds disp32 handling).

### Comparisons

`cmp_<op>_<ty>` lowers to:

```
mov  rax, [lhs]
cmp  rax, [rhs]
set<cc>  cl              ; setcc has 8-bit destination
movzx rax, cl            ; zero-extend to 64 bits
mov  [dst], rax
```

Condition selection:

| CIR predicate | Unsigned cc | Signed cc |
|---|---|---|
| `eq` | `E` | `E` |
| `ne` | `NE` | `NE` |
| `lt` | `B` | `L` |
| `le` | `BE` | `LE` |
| `gt` | `A` | `G` |
| `ge` | `AE` | `GE` |

### Logical (LANG38 parity)

`and_<ty>`, `or_<ty>`, `xor_<ty>` → `AND` / `OR` / `XOR` on `RAX` after
loading lhs/rhs, store to dst.

### Shifts (LANG38 parity)

Variable shift on x86-64 *must* use `CL` as the shift count.  The
backend pre-loads `[rhs]` into `CL`:

```
mov  rax, [lhs]
mov  rcx, [rhs]
shl  rax, cl            ; or shr / sar
mov  [dst], rax
```

`shr` for unsigned types, `sar` for signed.

### Unary (LANG38 parity)

- `neg_<ty>` → `MOV rax, [src]; NEG rax; MOV [dst], rax`.
- `not_<ty>` → `MOV rax, [src]; NOT rax; MOV [dst], rax`.
- `mov_<ty>` → `MOV rax, [src]; MOV [dst], rax`.

### Control flow

| CIR | Lowering |
|---|---|
| `label L` | `bind(L)` — no bytes emitted |
| `jmp L` | `JMP rel32` to label |
| `jmp_if_true v, L` | `MOV rax, [v]; TEST rax, rax; JNE L` |
| `jmp_if_false v, L` | `MOV rax, [v]; TEST rax, rax; JE L` |

### Returns

- `ret_<ty> v` → `MOV rax, [v]; <epilogue>; RET`.
- `ret_void` → `<epilogue>; RET`.

The epilogue is `MOV RSP, RBP; POP RBP`.

### Type guards

`type_assert` → `UD2` (two-byte invalid-opcode trap).  Matches the
AArch64 backend's `UDF` lowering — AOT has no deoptimisation, so
guard failures abort.

### Calls (matches LANG39 wave)

Cross-function `call` uses `CALL rel32` with an `ExternalReloc` of
kind `PltRel32`.  The packager resolves the displacement at link
time.  Arguments are loaded into the System V argument registers
(RDI, RSI, RDX, RCX, R8, R9) before the call; the return value
arrives in `RAX` and is stored to the destination slot.

V1 supports up to six arguments per call.  Beyond that → return
`BackendRefused`.

### Globals (matches LANG39 wave)

`global_load name` →

```
lea  rax, [rip + name]   ; PcRel32 reloc
mov  rax, [rax]
mov  [dst], rax
```

`global_store name, v` →

```
mov  rcx, [v]
lea  rax, [rip + name]   ; PcRel32 reloc
mov  [rax], rcx
```

### I/O (matches LANG40 wave)

`io_out v` lowers to a `CALL` to `__twig_print_i64` (Linux/macOS
shared runtime), passing `[v]` in `RDI`:

```
mov  rdi, [v]
call __twig_print_i64    ; PltRel32 reloc
```

The runtime archive provides the symbol, resolved by the system
linker.

---

## Out of scope (deferred PRs)

- **Microsoft x64 ABI** — argument registers differ (`RCX, RDX, R8,
  R9`), 32-byte home stack space is required, unwind tables (`.pdata`
  / `.xdata`) needed.  Follow-up spec **LANG44** or similar; the
  backend stays inside `x86_64-backend` with a runtime ABI toggle.
- **Floats / SSE2** — both backends will gain this together.
- **Local register allocator** — replace the spill core in place; no
  trait changes.
- **Closures** — needs cross-platform lowering decision (see
  LANG34/35).
- **Variable-shift via BMI2 `SHLX`/`SHRX`/`SARX`** — modern CPUs only;
  pure throughput optimisation.

---

## Test plan

Coverage target ≥ 95%.  Mirror the AArch64 backend's test suite
1-for-1:

1. **Per-CIR-opcode unit tests** — small CIR sequences, assert the
   emitted byte stream against a hand-verified reference.
2. **Prologue/epilogue framing** — empty function, single-arg, six-arg,
   seven-arg (must refuse); assert RSP is 16-byte aligned at the
   point of a hypothetical inner call.
3. **End-to-end factorial** — compile, run via JIT loader, assert
   `factorial(10) == 3628800`.
4. **Comparison polarity** — exhaustive signed/unsigned × six
   predicates × tiny truth tables.
5. **Division by zero trap** — `IDIV` by 0 raises `#DE`; the runtime
   sees a SIGFPE; assert the test harness catches it.
6. **Global load/store** — round-trip a 64-bit global through
   `global_store` then `global_load`.
7. **`io_out` integration** — call `__twig_print_i64`, capture
   stdout, assert correct decimal output.
8. **Backend refusal** — feed a `mul_f64` CIR instr, assert
   `compile_function` returns `None` (deopt path).
9. **Register allocation correctness** — function with 100 virtuals
   spills correctly; no slot collisions; frame size matches.

Cross-validation: a subset of tests decode emitted bytes via
`iced-x86` and assert the disassembly matches the expected mnemonic
sequence.  Dev-only dep.

---

## Registration

Once V1 lands, `jit-core` and `aot-core` register the backend in
their default backend registries:

```rust
// in codegen-core or wherever the default registry is built
registry.register(Box::new(X86_64Backend::new()));
```

No trait changes; both pipelines transparently pick up the new
target via `target_triple()`-based lookup.

---

## Risk register

| Risk | Mitigation |
|---|---|
| Variable-length encoding edge cases (REX.B / SIB requirement when base ∈ {RSP, RBP, R12, R13}) | Encoder normalises these internally; backend never sees them. |
| Division clobber surprises | All scratch regs (`RAX`, `RCX`, `RDX`) reserved up front; spill allocator never assumes their preservation across an instruction boundary. |
| Stack alignment violations at calls | One test per call site shape; alignment check is a single arithmetic invariant. |
| Mach-O vs ELF reloc kind differences | Encoder emits abstract `ExternalRelocKind`; `code-packager` maps to OS-specific reloc types. |
| Mismatch between AArch64 and x86-64 CIR coverage as both backends evolve | Each future LANG NN that touches AArch64 backend should produce a sibling commit on x86-64.  This is a discipline issue, tracked in the changelog. |
