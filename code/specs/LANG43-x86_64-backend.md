# LANG43 — `x86_64-backend`: X86-64 Native Backend

**Status:** Draft — 2026-05-14

> The x86-64 port spans four specs (LANG44 encoder, **LANG43** backend,
> LANG45 object emitters, LANG46 twig-aot driver).  Together they bring
> Twig source → native binary that runs on **Linux x86-64** and
> **Windows x86-64**.  This spec is the CIR-lowering layer; both ABIs
> are in V1 scope so neither OS waits behind the other.

## Motivation

`aarch64-backend` brings the LANG VM to ARM64 hosts (Apple Silicon,
modern Linux ARM).  `x86_64-backend` brings it to x86-64 hosts —
which today dominate CI runners, cloud VMs, and developer laptops.

This spec defines the backend that consumes CIR via the `Backend`
trait (LANG05) and produces x86-64 machine code via `x86_64-encoder`
(LANG44).  It plugs into both `aot-core` (LANG04) and `jit-core`
(LANG03) without changes to either — the `Backend` trait is
target-agnostic.

The goal of V1 is **functional parity with `aarch64-backend`** after
LANG38 (arithmetic completeness) + LANG39 (globals) + LANG40 (I/O) +
LANG41 (runtime library) on **both** Linux and Windows x86-64: every
CIR opcode currently supported on ARM64 must also compile on x86-64,
and the same Twig program must produce a working ELF executable on
Linux and a working PE executable on Windows.

## Non-goals (V1)

- **Floating-point / SSE / AVX** — `aarch64-backend` has no float
  support either.  Defer until both backends grow it together.
- **Closures** (LANG34/35) — defer; mirrors the AArch64 path's
  `closures: NOT YET` line.
- **Local-register allocator** — V1 uses stack spill exclusively
  (matches AArch64).  A real allocator can replace the spill core
  later behind the same public API.
- **macOS x86-64** — Intel Macs are a shrinking population; defer.
  The encoder works there, but `code-packager` would need a Mach-O
  variant tuned for `CPU_TYPE_X86_64` (LANG45 covers only ELF + PE
  in V1).
- **Windows SEH unwind tables** (`.pdata` / `.xdata`) — required for
  proper exception unwinding from a debugger or another module.
  V1 emits no unwind data; uncaught traps terminate the process,
  which is the same behaviour Linux gets without `.eh_frame`.
  Adding tables is a follow-up; spec stays in `x86_64-backend` once
  needed.

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

## ABI selection

`X86_64Backend` ships with **two ABI modes** in V1, selected at
backend construction time:

```rust
pub enum X86_64Abi {
    /// System V AMD64 — Linux, FreeBSD, (Intel) macOS.
    SysV,
    /// Microsoft x64 — Windows desktop/server.
    MsX64,
}

impl X86_64Backend {
    pub fn new() -> Self { Self::with_abi(X86_64Abi::SysV) }
    pub fn with_abi(abi: X86_64Abi) -> Self { /* ... */ }
}
```

The choice flows from `FunctionContext::target_triple` or an explicit
constructor argument from `twig-aot` (LANG46).  Both ABIs share the
same CIR-lowering logic — only the prologue/epilogue, arg register
table, callee-saved set, and call-site stack-alignment math differ.

### Side-by-side: System V AMD64 vs Microsoft x64

| Concern | System V AMD64 (Linux) | Microsoft x64 (Windows) |
|---|---|---|
| Reference | System V AMD64 ABI draft 1.0 (Matz et al.) | MSDN: "x64 calling convention" |
| Integer arg registers (in order) | RDI, RSI, RDX, RCX, R8, R9 | RCX, RDX, R8, R9 |
| Max GPR-passed integer args | 6 | 4 |
| Return value | RAX | RAX |
| Caller-saved (volatile) | RAX, RCX, RDX, RSI, RDI, R8–R11 | RAX, RCX, RDX, R8–R11 |
| Callee-saved | RBX, RBP, R12–R15 | RBX, RBP, RDI, RSI, R12–R15 |
| Shadow / home space | None | **32 bytes** allocated by caller, *always* (even for ≤4 args) |
| Stack alignment at CALL | 16-byte | 16-byte |
| Red zone | 128 B below RSP | **None** — must not read/write below RSP |
| Unwind metadata | `.eh_frame` (optional in V1) | `.pdata` / `.xdata` (optional in V1; see Non-goals) |

V1 implements all four shaded lines (arg regs, callee-saved, shadow
space, red-zone discipline) and ignores the unwind-metadata rows.

### Prologue / epilogue — System V

```asm
;  --- prologue ---
push  rbp
mov   rbp, rsp
sub   rsp, <frame>          ; <frame> chosen so RSP ≡ 0 (mod 16)

;  spill incoming arg registers to their slots (up to 6)
mov   [rbp - 8],  rdi
mov   [rbp - 16], rsi
mov   [rbp - 24], rdx
mov   [rbp - 32], rcx
mov   [rbp - 40], r8
mov   [rbp - 48], r9

;  --- body ---

;  --- epilogue ---
mov   rsp, rbp
pop   rbp
ret
```

### Prologue / epilogue — Microsoft x64

```asm
;  --- prologue ---
push  rbp
mov   rbp, rsp
sub   rsp, <frame>          ; <frame> chosen so RSP ≡ 0 (mod 16)

;  spill incoming arg registers to their slots (up to 4)
mov   [rbp - 8],  rcx
mov   [rbp - 16], rdx
mov   [rbp - 24], r8
mov   [rbp - 32], r9

;  --- body ---

;  --- epilogue ---
mov   rsp, rbp
pop   rbp
ret
```

The only structural difference is the arg-spill register set (RDI/RSI
vs RCX/RDX/R8/R9 etc.).  Frame layout is otherwise identical.

### Stack alignment invariant

For both ABIs: at every `CALL` instruction in the emitted body, RSP
must be `≡ 0 (mod 16)`.  The prologue `push rbp` shifts the inherited
`≡ 8 (mod 16)` (post-entry `CALL` alignment) to `≡ 0 (mod 16)`, so
the frame `sub rsp, N` must keep `N ≡ 0 (mod 16)`.

### Microsoft x64 shadow space at call sites

Every call to an external function in Microsoft x64 must reserve **32
bytes of "home" / "shadow" space** on the stack — the callee owns
that space (it's where it would spill RCX/RDX/R8/R9 if it chose to).
The callee does **not** clean it up; the caller deallocates.

V1 reserves the shadow space in the *prologue* by adding 32 to the
frame size whenever the function will issue any `CALL` (i.e., is not a
leaf).  This is simpler than reserve-per-call and costs at most 32
unused bytes for non-leaf functions.

```asm
;  --- prologue (MS x64, non-leaf) ---
push  rbp
mov   rbp, rsp
sub   rsp, <frame> + 32     ; +32 = persistent shadow space for callees
```

System V has no analogous requirement.

### Red zone

System V grants a 128-byte red zone below RSP for *leaf functions* —
they may read/write `[rsp - 0..128]` without subtracting from RSP.
V1 does **not** use the red zone on either ABI; all locals live in
the proper frame.  This keeps the prologue/epilogue uniform across
both targets.

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
kind `PltRel32`.  The packager (LANG45) translates the abstract
reloc kind into:

- `R_X86_64_PLT32` (or `R_X86_64_PC32` for static calls) on ELF
- `IMAGE_REL_AMD64_REL32` on PE/COFF
- `X86_64_RELOC_BRANCH` on Mach-O (deferred V1)

Arguments are loaded into the ABI's argument registers before the
call:

- **System V**: RDI, RSI, RDX, RCX, R8, R9 — up to **six** GPR args.
- **MS x64**: RCX, RDX, R8, R9 — up to **four** GPR args.

The return value arrives in `RAX` and is stored to the destination
slot.

Beyond the per-ABI max → return `BackendRefused`.

On Microsoft x64, the prologue already allocated the 32-byte shadow
space; the call site itself emits only the argument moves + `CALL
rel32` + return-value store.

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

### I/O (matches LANG40 / LANG41 waves)

`io_out v` lowers to a `CALL` to `__twig_print_i64`, passing `[v]`
in the ABI's first argument register:

- **System V (Linux)**: arg 0 → `RDI`
  ```
  mov  rdi, [v]
  call __twig_print_i64    ; PltRel32 reloc
  ```
- **MS x64 (Windows)**: arg 0 → `RCX`
  ```
  mov  rcx, [v]
  call __twig_print_i64    ; PltRel32 reloc
  ```

The runtime archive provides the symbol, resolved by the system
linker.  Per LANG46, separate runtime archives are built for each
target (Linux x86-64 ELF `.a`; Windows x86-64 COFF `.lib`).

---

## Out of scope (deferred PRs)

- **macOS x86-64** — encoder works; needs Mach-O object emitter
  tuned for `CPU_TYPE_X86_64`.  Defer to a follow-up after Linux +
  Windows ship.
- **Windows SEH unwind tables** (`.pdata` / `.xdata`) — needed for
  proper debugger / cross-module unwind; not blocking
  compile-and-run.  Follow-up.
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
| Stack alignment violations at calls | One test per call site shape per ABI; alignment check is a single arithmetic invariant. |
| ELF vs PE vs Mach-O reloc kind differences | Encoder emits abstract `ExternalRelocKind`; `code-packager` (LANG45) maps to OS-specific reloc type IDs. |
| MS x64 shadow space forgotten on a call site | Always reserve 32 B in the prologue of a non-leaf function; one targeted test verifies a six-call function leaves the stack 16-byte aligned and shadow-spaced at each call. |
| Linux vs Windows argument register mismatch in `io_out` | Backend dispatches on `X86_64Abi`; per-ABI golden-byte test fixes the first instruction (`mov rdi, …` vs `mov rcx, …`). |
| Mismatch between AArch64 and x86-64 CIR coverage as both backends evolve | Each future LANG NN that touches AArch64 backend should produce a sibling commit on x86-64.  This is a discipline issue, tracked in the changelog. |
