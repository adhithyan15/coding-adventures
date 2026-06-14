# Language × Backend platform matrix — every language on every (non-BEAM) backend

**Goal:** verify, *by running*, that **every language frontend in the repo** executes
correctly on **every backend except BEAM** — the same completeness bar McCarthy Lisp
already clears, extended to the whole language family. LLVM coverage for every
language is an explicit priority.

This is the generalization of the McCarthy `MCCARTHY-LISP-PLATFORM-MATRIX.md` /
`CLR-REAL-RUNTIME-VERIFICATION.md` chapters: McCarthy was the reference language run
on all 8 backends; this chapter brings the other six languages up to the same
cross-backend bar (minus BEAM).

## Why this is mostly verification, not new backends

Every language frontend lowers to **one shared IIR** (`interpreter-ir::IIRModule`):

```
Twig / Nib / Brainfuck / Dartmouth BASIC / Oct / ALGOL 60   (+ McCarthy)
        │  each via <lang>_iir_compiler::compile_source
        ▼
                 IIRModule  (the lingua franca)
        │
        ├── vm-core::VMCore                  → VM        (generic IIR interpreter, all 6)
        ├── jit-core::JITCore + GenericCirJit → JIT      (generic IIR JIT, all 6)
        ├── twig-aot + aarch64/x86_64-backend→ native AOT
        ├── iir-to-llvm                      → LLVM  → real clang
        ├── iir-to-wasm                      → WASM  → wasm-runtime
        ├── iir-to-jvm-class-file            → JVM   → real java
        ├── iir-to-cil-bytecode              → CLR   → real ilasm + real dotnet
        └── iir-to-beam                      → BEAM  (OUT OF SCOPE — see below)
```

Each backend consumes the shared IIR, so a backend is *language-agnostic by
construction*: a frontend that lowers to IIR can in principle reach every backend
for free. The work here is therefore mostly **(a)** adding cross-language conformance
that proves each `(language, backend)` cell by running it, and **(b)** fixing the
real lowering / runtime gaps that running surfaces — not writing new code generators.

The genuine exceptions, where new wiring (not just a test) is required — and a
correction the LM0 probe surfaced by **running** each backend:

- **VM and JIT are McCarthy-*specialized*, not general IIR interpreters.** Verified
  in LM0: `mccarthy_lisp_vm::run` rejects ordinary arithmetic/comparison ops with
  `UnsupportedOp("add" / "mul" / "cmp_eq" / "cmp_lt" / "mod")` and the I/O ops with
  `UnknownBuiltin("print_i64")` / `UnsupportedOp("alloc_bytes")`. It only ran the
  *constant* programs (Twig `42`, Nib `return 42`, ALGOL `result := 42`, Oct void→0).
  So the **VM and JIT columns are real op-coverage work**, not free: each must grow
  the integer-arithmetic / comparison / (for I/O langs) tape + print ops before it
  can run the non-McCarthy languages. The JIT additionally needs a generic
  `run_on_jit(language, source)` (it has only `run_mccarthy_on_jit` today).
- **The code-generator backends are general.** Native AOT, LLVM, WASM, JVM, and CLR
  all compile the full IIR (existing tests already run Twig and ALGOL arithmetic on
  WASM; LM0 runs all six languages on native AOT). So those five columns are mostly
  **conformance tests + I/O wiring**, not new code generators.
- **I/O languages produce results via stdout.** Brainfuck (`putchar`/`getchar`) and
  Dartmouth BASIC (`PRINT`) print rather than return an exit code, so their
  conformance captures stdout, and each backend's I/O intrinsics (`io_out` /
  `putchar` / `print_i64`) must be exercised end-to-end.

> **Note (fixed in LM0):** the `Language` enum doc comments in `lang-aot/src/lib.rs`
> were **stale** (they called DartmouthBasic / Oct "placeholders / no Rust
> frontend"); all six frontends are in fact wired into `compile_source_to_iir`.

## Scope

**Languages (6):** Twig, Nib, Brainfuck, Dartmouth BASIC, Oct, ALGOL 60.
(McCarthy Lisp is already complete — it is the reference, not a worklist item.)

**Backends (7):** VM, JIT, native AOT, LLVM, WASM, JVM, CLR.

**Out of scope — BEAM.** The Erlang VM is a purely-functional, immutable-term
runtime; languages with mutable imperative state (Brainfuck's tape, BASIC's
variables/`GOTO`) do not map cleanly onto it, so BEAM stays McCarthy-only. (If a
later chapter wants BEAM for the *expression* languages — Twig/Nib/Oct/ALGOL — it
can be added then; this chapter does not pursue it.)

## Methodology — prove every cell by RUNNING

Mirror the McCarthy W16 capstone (`lang-aot/tests/conformance.rs`):

1. Per language, a **battery** of small programs, each with a known result — an
   integer exit value for expression languages, or a stdout string for the I/O
   languages (Brainfuck/BASIC).
2. A backend-runner table (generalized from the McCarthy conformance runners to take
   a `Language`), each runner gated on its external tool (skip when absent).
3. For every `(program, backend)`: run and assert the result. The in-process
   backends (VM, and the simulators) are the floor; the external-tool backends
   (LLVM/clang, JVM/java, CLR/dotnet+ilasm, native/ld) upgrade the proof to the real
   runtime when installed.

A cell is **✅ only when a test actually runs the program through that backend and
asserts the result** — never on "the frontend lowers to IIR so it *should* work."

## Status legend

`✅` proven by a running test · `◑` in progress · `☐` not started · `⏸` deferred
(a known gap that needs a deeper backend/frontend change — grouped at the end so the
achievable code-gen columns land first).

## The matrix (target — every non-BEAM cell ✅)

| Language        | VM | JIT | native-AOT | LLVM | WASM | JVM | CLR |
|-----------------|----|-----|-----------|------|------|-----|-----|
| Twig            | ✅ | ✅  | ✅        | ✅   | ✅   | ✅  | ✅  |
| Nib             | ✅ | ✅  | ✅        | ✅   | ✅   | ✅  | ✅  |
| Brainfuck       | ✅ | ✅  | ✅        | ✅   | ✅   | ✅  | ✅  |
| Dartmouth BASIC | ✅ | ✅  | ✅        | ✅   | ✅   | ✅  | ✅  |
| Oct             | ✅ | ✅  | ✅        | ✅   | ✅   | ✅  | ✅  |
| ALGOL 60        | ✅ | ✅  | ✅        | ✅   | ✅   | ✅  | ✅  |

**native-AOT is uniformly ✅ as of LM0** — all six languages compile to a host
executable and run with the expected result (`lang-aot/tests/lang_matrix.rs`:
Twig→42, Nib→42, Oct→0, ALGOL `17 mod 5`→2, Brainfuck→stdout `A`, BASIC→stdout `42`).
The VM/JIT columns are op-coverage work (see above); the code-gen columns
(LLVM/WASM/JVM/CLR) are conformance + I/O wiring.

(The starting state is re-verified per slice — the loop trusts running, not this table.)

## Worklist (one PR per item; slice further if large)

### Phase 0 — matrix harness

- ✅ **LM0 — cross-language conformance harness.** New `lang-aot/tests/lang_matrix.rs`:
  per-language program batteries (`Expect::Exit` for Twig/Nib/Oct/ALGOL, `Expect::Stdout`
  for Brainfuck/BASIC) + a host-gated **native-AOT** runner. Proven by RUNNING — all
  six languages run on native AOT (Twig→42, Nib→42, Oct→0, ALGOL `17 mod 5`→2,
  Brainfuck→`A`, BASIC→`42`). Fixed the stale `Language` enum doc comments. The probe
  also established the ground truth that the **VM/JIT are McCarthy-specialized**
  (reframed Phase V below) while the code-gen backends are general.

### Phase L — LLVM for every language (priority)

- ✅ **LM-L Twig / Oct / ALGOL (expression languages on LLVM).** `lang_matrix.rs`
  refactored into a `Backend`-keyed grid (each `Prog` lists its proven backends); a
  `clang`-gated LLVM runner (`.ll` → real `clang` → run) added. Verified by RUNNING:
  Twig→42, Oct→0, ALGOL `17 mod 5`→2. No C runtime needed — these link a bare `.ll`.
- ✅ **LM-L Nib (on LLVM).** Root cause (found by reading + running): the **Nib
  frontend** emitted type-*inconsistent* IIR — its instruction bodies use `i64`
  (`compile_binary_chain` and the `ret` default both emit `"i64"`; the frontend's own
  comment says "Nib's u4/u8/bool all materialise as i64 at the IIR level") while
  `extract_params`/`extract_return_type` left the **function signature** as the narrow
  `u8`. `iir-to-llvm` faithfully emitted `define i8 @double(i8 %x)` but `add i64 %x, %x`
  → `'%x' defined with type 'i8' but expected 'i64'`. (The backend is correct: its
  `test_backend.rs` proves it emits *consistent* narrow types fine.) Fix: complete the
  frontend's own convention — `widen_nib_type` now materialises the integer types
  (`u4`/`u8`/`bcd`) to `i64`, so signature and bodies agree. Verified by RUNNING: Nib on
  LLVM → 42, Nib on native still → 42 (no regression), nib-iir-compiler unit tests +
  `iir-to-llvm` (51) all green.
- ✅ **LM-L BASIC (on LLVM).** `run_llvm` grew the stdout path: BASIC's `.ll` emits
  `call void @__print_i64(i64 42)`, so when the `.ll` references `@__print_i64` the
  runner compiles in a generic `__print_i64` C runtime and the harness compares
  **stdout**. Verified by RUNNING: `10 PRINT 42` → stdout `42` on real `clang`.
- ✅ **LM-L Brainfuck (on LLVM).** The first deferred item, tackled via the
  **frontend-i64** approach (chosen over width-aware slots to avoid touching the
  McCarthy-critical `iir-to-llvm` slot allocator). The i64 materialisation lives in
  `lower_brainfuck_for_aot` Step 5 — **not** the BF frontend — because the frontend's
  `u8`/`u32` hints feed `vm-core`/`jit-core`'s `specialise`, which keys CIR opcode
  widths (`add_u8`/`add_u32`) off them; widening the frontend would break the BF JIT.
  So the pass that *already* builds the tape boundary widens every narrow hint to
  `i64`, and `iir-to-llvm` 0.9.0 grew the tape ops: `alloc_bytes`→`@calloc`,
  `load_byte` (`getelementptr i8`+`load`+`zext`), `store_byte` (`trunc`+`store`),
  `putchar`/`getchar`→libc — plus a slot-dest SSA rename (a slot var assigned 2+ times
  by a real op no longer emits a duplicate `%name`; BF's `ptr`/`v` are the first such
  case, which is why this only surfaced now). Verified by RUNNING
  `++++++++[>++++++++<-]>+.` → stdout `A` on real `clang` (`lang_matrix.rs`); the BF
  `vm-core`/`jit-core` suites stay green (frontend untouched).

The LLVM column is therefore green for **all 6** languages (Twig / Nib / Oct / ALGOL /
BASIC / Brainfuck). The LLVM column is **complete**.

### Phase W — WASM for every language

- ✅ **LM-W expression languages (Twig / Oct / ALGOL on WASM).** Added a `Backend::Wasm`
  runner to `lang_matrix.rs` (source → `iir-to-wasm` → the in-process `wasm-runtime`;
  `main`'s wasm result is the exit value). Verified by RUNNING: Twig→42, Oct→0,
  ALGOL `17 mod 5`→2. In-process, so no host gate.
- ✅ **LM-W Nib (on WASM).** Completed the i64 materialization the LLVM Nib fix
  started: `nib_ty_str` (const literals / `ret` / call results) **and** the
  un-annotated-literal fallback now emit `i64`, not the narrow `u8`. So the bare
  literal argument in `double(21)` is `i64`, matching the `i64` parameter the strict
  WASM backend requires. Verified by RUNNING: Nib on WASM → 42, and still LLVM → 42,
  native → 42 (no regression); nib-iir-compiler (28) + `iir-to-wasm`/`iir-to-llvm`
  suites all green.
- ✅ **LM-W BASIC (on WASM).** BASIC's `PRINT` lowers to a wasm import
  `env.__print_i64 : (i64) -> ()`; the default `WasmRuntime::new()` couldn't resolve it
  (`no body for function 0`). `run_wasm` now installs a tiny test-local `PrintHost`
  (`wasm_execution::HostInterface`) that resolves that single import to a `PrintFunc`
  capturing each printed `i64` into a shared buffer, joined as the program's stdout. The
  expression languages import nothing, so the host is never consulted for them (behaviour
  unchanged). New in-repo dev-deps `wasm-execution` + `wasm-types`. Verified by RUNNING:
  BASIC `10 PRINT 42` → stdout `42`; Twig/Nib/Oct/ALGOL still green (16 proven cells).
  **WASM column now complete for every language except the deferred Brainfuck.**
- ✅ **LM-W Brainfuck (on WASM).** The LLVM byte-tape pattern ported to wasm.
  `iir-to-wasm` 0.13.0 grew `alloc_bytes` (tape base 0 in the module's 1-page linear
  memory), `load_byte` (`i32.load8_u` + `i64.extend_i32_u`), and `store_byte`
  (`i32.wrap_i64` + `i32.store8`); `run_wasm`'s host resolves the existing
  `env.putchar`/`env.getchar` imports, with a `PutcharFunc` capturing stdout **bytes**
  (so `.` of 65 → `A`, not `65`). The i64 widening (shared from LM-L) rippled into wasm
  control flow: the widened loop guard is i64, so `jmp_if_*` now branches via `i64.eqz`,
  `putchar`/`getchar` wrap/extend across the i64↔i32 boundary, and an i64-declared
  comparison result is `i64.extend_i32_u`-widened so the module is well-typed (a latent
  i32-in-i64-local inconsistency the lenient runtime had tolerated). Verified by RUNNING
  `++++++++[>++++++++<-]>+.` → stdout `A` on the in-repo `wasm-runtime`; Twig/Nib/Oct/
  ALGOL/BASIC wasm cells still green. **WASM column now complete for every language.**

### Phase J — JVM for every language

- ✅ **LM-J expression languages (Twig / Nib / Oct / ALGOL on real `java`).** Added a
  `Backend::Jvm` runner to `lang_matrix.rs`: source → `compile_source_to_jvm_class` →
  the W16 wrapper-launcher (inject a `main([Ljava/lang/String;)V` that invokes the
  entry `main()I` and `System.out.println`s its `int`) → real `java` → parse the
  printed integer. Gated on `java`, skips gracefully when absent (like the LLVM
  column on `clang`). Fixed a real backend-glue bug found **by running on real
  `java`**: `concretize_scalar_any_for_jvm` retyped a scalar function's return + body
  to `i32` but left its **parameters** `i64`, so Nib's `double(x)` emitted the
  inconsistent `(J)I` and `java` rejected it with `VerifyError: Expecting to find
  integer on stack` (the laxer in-repo `jvm-simulator` didn't catch it). Now the pass
  concretizes parameter types too. Verified by RUNNING: Twig→42, Nib→42, Oct→0,
  ALGOL `17 mod 5`→2 on real `java` (20 proven matrix cells); jvm_emit + conformance
  suites still green.
- ✅ **LM-J BASIC (on JVM).** BASIC's `print_i64` lowers to `invokestatic
  env/BasicRuntime.println(J)V`. `run_jvm` now compiles that `env.BasicRuntime` host
  class with `javac` onto the classpath (the JVM analogue of the wasm `PrintHost` /
  LLVM print runtime) and, for an I/O program, injects a **discard** launcher (run the
  entry, `pop`/`pop2` its result) instead of the print launcher, then captures
  `System.out`. Fixed a second backend-glue bug found **by running**: a printing
  function must keep the **wide i64** value model — `print_i64` does `lload val;
  println(J)V`, so concretizing the value to `i32` made it `istore`d-as-int but
  `lload`ed-as-long, which `java` rejects with `VerifyError: Accessing value from
  uninitialized register pair`. `concretize_scalar_any_for_jvm` now skips any function
  that calls `print_i64` (the same way it already skips lisp/heap functions), so
  BASIC's entry stays `()J` and the value round-trips as a `long`. Verified by RUNNING:
  BASIC `10 PRINT 42` → `System.out` `42` on real `java` (21 proven matrix cells); the
  expression-language JVM cells + jvm_emit/conformance/wasm_emit suites still green.
  **JVM column now green for every language except the deferred Brainfuck.**
- ✅ **LM-J Brainfuck (on JVM).** The LLVM/WASM byte-tape pattern ported to the JVM.
  The backend already had the *raw* BF tape ops (`load_mem`/`store_mem` →
  `baload`/`bastore` over a static `env/BFRuntime.__tape : [B`) + `putchar`/`getchar`,
  but `lower_brainfuck_for_aot` produces the *lowered* `alloc_bytes`/`load_byte`/
  `store_byte` form. `iir-to-jvm-class-file` 0.11.0 added those: `alloc_bytes` is a
  no-op (the tape is the pre-allocated static field), `load_byte`/`store_byte` reuse
  the `baload`/`bastore`+mask access with `l2i`/`i2l` conversions for the (possibly
  widened) i64 value model, and `jmp_if_*` now reduce an `i64` guard via
  `lload; lconst_0; lcmp` before `ifeq`/`ifne` (an `iload` would read one slot of the
  two-slot long — a verify error). `run_jvm` compiles a new `env.BFRuntime` host class
  (static `byte[] __tape`, `putchar` writing a raw byte, `getchar`) with `javac` and
  captures its `System.out.write` bytes. Verified by RUNNING `++++++++[>++++++++<-]>+.`
  → `A` on real `java`; the existing JVM cells + jvm backend suites still green.
  **JVM column now complete for every language.**

### Phase C — CLR for every language

- ✅ **LM-C expression languages (Twig / Nib / Oct / ALGOL on real CoreCLR).** Added a
  `Backend::Clr` runner to `lang_matrix.rs`: source → textual `.il`
  (`compile_source_to_cil_text`) → real `ilasm -exe` → real `dotnet` → parse the integer
  the entry `Console.WriteLine`s (the CLR-real path generalized from McCarthy; reuses
  `clr_support::find_ilasm`). Gated on `dotnet` + `ilasm`. Two gaps fixed **by running**:
  (1) `iir-to-cil-bytecode` (0.16.0) grew the integer **arithmetic** (`add`/`sub`/`mul`/
  `div`/`mod`→`rem`) and **comparison** (`cmp_*`→`ceq`/`clt`/`cgt`) opcodes it had
  `UnsupportedOp`'d — McCarthy only emitted a constant; (2) `concretize_scalar_any_for_cil`
  now concretizes **parameter** types too (the CLR twin of the JVM `int32(int64)` verifier
  fix). Verified by RUNNING on CoreCLR: Twig→42, Nib→42, Oct→0, ALGOL `17 mod 5`→2 (25
  proven matrix cells); conformance/jvm_emit/wasm_emit/iir-to-cil-bytecode suites green.
  (The `clr-simulator` floor is intentionally *not* used — its `cil_emit.rs` test is
  broken on `main` from an API drift; the real-`dotnet` path is the stronger check.)
- ✅ **LM-C BASIC (on CLR).** Grew the CIL backend (`iir-to-cil-bytecode` 0.17.0) with
  the `print_i64` → `call void [System.Console]System.Console::WriteLine(int32)` lowering
  (the CLR analogue of wasm `env.__print_i64` / JVM `env.BasicRuntime.println`) and made
  the `Run()` launcher I/O-aware: a printing program already wrote its output as a side
  effect, so the launcher **discards** (`pop`) the entry result instead of
  `Console.WriteLine`-ing it — no double-print. `run_clr` now returns the captured
  `Console` output alongside the parsed integer. Verified by RUNNING on CoreCLR: BASIC
  `10 PRINT 42` → `Console` `42` exactly once (26 proven matrix cells); CIL +
  conformance/jvm_emit/wasm_emit suites green. **CLR column now green for every language
  except the deferred Brainfuck — so every code-gen backend (native-AOT, LLVM, WASM, JVM,
  CLR) is green for all five non-Brainfuck languages.**
- ✅ **LM-C Brainfuck (on CLR).** The **last code-gen cell.** The textual `.il` emitter
  (`iir-to-cil-bytecode` 0.18.0) grew the byte-tape ops: `alloc_bytes` → `newarr
  [System.Runtime]System.Byte` into an `unsigned int8[]` local (`FnRegs::build` types an
  `alloc_bytes` dest as the array, not the scalar its concretised hint gives), `load_byte`
  → `ldelem.u1` (unsigned), `store_byte` → `stelem.i1` (8-bit wrap); `putchar` →
  `Console::Write(char)` (so `.` of 65 writes `A`), `getchar` → `Console::Read()`. The
  `Run()` launcher's "prints" test now also matches `putchar`, so a Brainfuck program
  discards the entry result (no double-print). `concretize_scalar_any_for_cil` retypes
  Brainfuck back to `int32` (it doesn't call `print_i64`), and CIL `brfalse` tests any
  width against zero — so, unlike the JVM (`lcmp`) and wasm (`i64.eqz`), the loop guard
  needed **no** i64 branch fix. Verified by RUNNING `++++++++[>++++++++<-]>+.` → `A` on
  real `ilasm`+`dotnet`; the existing CLR cells + CIL suites still green.
  **CLR column now complete for every language — the entire code-gen matrix is done.**

### Phase A — native AOT completeness

- ✅ **LM-A — native AOT uniformly green.** All six languages already run on native
  AOT as of LM0 (the AOT column is ✅ across the board), so nothing further is needed.

## Deferred — deeper backend/frontend work (after the code-gen columns)

Each needs a real change to a backend/interpreter with risk to currently-green
backends, so they are tackled deliberately rather than as a routine conformance slice.

- ✅ **LM-L Brainfuck (on LLVM).** **Done** (see Phase L). The "real wall" — `iir-to-llvm`
  promotes any 2+-assigned variable to an `alloca i64` slot, while BF's `v`/`ptr` were
  narrow (`u8`/`u32`), so a slot-loaded `i64` feeding a `u8` `add` was a type error — was
  resolved by the **frontend-i64** route, but materialised in `lower_brainfuck_for_aot`
  (Step 5) rather than the literal frontend, so `vm-core`/`jit-core`'s width-keyed
  `specialise` stays correct. `iir-to-llvm` 0.9.0 added the tape ops + a slot-dest SSA
  rename (the duplicate-`%name` bug that the "straightforward" tape ops would have hit).
- ✅ **LM-W Brainfuck (on WASM).** **Done** (see Phase W). `iir-to-wasm` 0.13.0 grew the
  byte-tape ops over linear memory; the i64-widening rippled into wasm control flow
  (i64 loop guard → `i64.eqz`, i64-declared cmp results widened, `putchar`/`getchar`
  wrap/extend) — all width fixes, no frontend or slot-model change.
- ✅ **LM-J Brainfuck (on JVM).** **Done** (see Phase J). `iir-to-jvm-class-file` 0.11.0
  grew the lowered byte-tape ops over the static `byte[] __tape` (`alloc_bytes` no-op,
  `load_byte`/`store_byte` with `l2i`/`i2l` conversions) + the `lcmp`-based i64 branch
  conditions — all width fixes over the backend's existing `baload`/`bastore` tape access.
- ✅ **LM-C Brainfuck (on CLR).** **Done** (see Phase C) — the last code-gen cell.
  `iir-to-cil-bytecode` 0.18.0's textual `.il` emitter grew the byte-tape ops over an
  `unsigned int8[]` local (`newarr Byte`/`ldelem.u1`/`stelem.i1`) + `Console::Write(char)`
  putchar; CIL `brfalse` needed no i64 fix. **The entire code-gen wave is complete.**
### Phase V — generic register VM

**Directive: the VM must be _generic_** — a register-based interpreter that consumes the
shared IIR so *any* future frontend (Ruby, JS, …) runs on it with zero VM-specific
rework, exactly like the code-gen backends. The LM0 probe's "the VM rejects
`add`/`mul`/`cmp_*`" was a **mischaracterisation**: it tested `mccarthy_lisp_vm`, which is
a *deliberately separate* lisp interpreter (its value model is `lispy-runtime`'s tagged
`LispyValue` — symbols/cons/nil — and McCarthy's frontend lowers `(+ 1 2)` to a
`call_builtin`, so its IIR genuinely has no `add`). The **general** register VM,
`vm_core::VMCore`, already exists and its dispatch already covers `const`/`mov`,
`add`/`sub`/`mul`/`div`/`mod`/`neg`, `and`/`or`/`xor`/`not`/`shl`/`shr`, all `cmp_*`,
`label`/`jmp`/`jmp_if_*`/`ret`, `load_mem`/`store_mem`/`load_reg`/`store_reg`,
`call`/`call_builtin`, and `io_*` — over a scalar `Value` (Int/Float/Bool/Str/Null). The
matrix's six languages are all scalar, so they share this one VM; Brainfuck already ran on
it for years (`brainfuck-iir-compiler` uses `VMCore`). Phase V is therefore mostly
**wiring**, not interpreter work.

- ✅ **LM-V expression + BASIC (Twig / Nib / Oct / ALGOL / Dartmouth BASIC on the VM).**
  Added a `Backend::Vm` runner to `lang_matrix.rs`: source → `compile_source_to_iir` (the
  *same* shared pipeline every code-gen column uses) → `vm_core::VMCore::execute`. **No
  per-language code** — the scalar languages' arithmetic/comparison/control-flow/memory
  ops are exactly `VMCore`'s existing dispatch. The I/O languages print through a
  registered builtin closure (`print_i64` appends to a capture buffer — the VM sibling of
  the wasm `PrintHost` / LLVM `@__print_i64` / JVM `BasicRuntime` / CLR `Console.WriteLine`);
  `putchar`/`getchar` are registered too, for the next slice. Verified by RUNNING in-process:
  Twig→42, Nib→42, Oct→0, ALGOL `17 mod 5`→2, BASIC `10 PRINT 42`→stdout `42`. The VM column
  is now green for **5 / 6** languages — only Brainfuck is left.
- ✅ **LM-V Brainfuck (on the VM).** The one genuine op-gap, closed generically. `vm-core`
  0.4.0 grew the byte-tape ops `alloc_bytes`/`load_byte`/`store_byte` (the lowered tape form
  `lower_brainfuck_for_aot` emits) in its dispatch — implemented over the **existing flat
  `memory` address space** (the same `HashMap<i64, Value>` `load_mem`/`store_mem` use): a
  cell is `memory[base + idx]` (default `0`), `store_byte` masks to a byte (the 8-bit wrap),
  `load_byte` reads it back unsigned. No new value kind, no per-language code. Verified by
  RUNNING `++++++++[>++++++++<-]>+.` on the VM → `A`. **The VM column is now complete — all
  six languages run on the one `VMCore` interpreter via the shared IIR.**
- ✅ **Phase I — generic JIT (the last column).** Added a `Backend::Jit` runner to
  `lang_matrix.rs`: source → `compile_source_to_iir` (the *same* shared pipeline) →
  `jit_core::JITCore` driving the language-agnostic `GenericCirJit` over the shared IIR.
  `execute_with_jit` eagerly compiles every fully-typed function to JIT bytecode (installing
  a native handler) and interprets the rest on `VMCore`, so each program runs *through the
  JIT pipeline*. **No per-language code** — the I/O builtins are registered as callbacks on
  both tiers, exactly the way a future Ruby/JS frontend would.

  Closing the column surfaced one genuine generic-JIT gap: `GenericCirJit::run` ignored its
  `args`, so a compiled function with parameters (Nib's `double(x) -> x + x`) read its
  parameter as the zero-initialised register and returned `0`. Fixed at the **generic**
  level (`jit-core` 0.4.0): `compile_fn` now calls `compile_function` (passing the
  `FunctionContext` — name, params, return type), `GenericCirJit::compile_function`
  pre-binds the parameters to registers `0..n` in declaration order, and `run` seeds those
  registers from the call arguments. Any frontend whose functions take arguments now JITs
  and runs correctly — the register-VM/JIT design the directive calls for. Verified by
  RUNNING all six languages on the JIT in-process (Twig→42, Nib→42, Oct→0, ALGOL→2,
  BASIC→`42`, Brainfuck→`A`), plus direct `jit-core` unit tests for the param binding.
  **The JIT column is now complete — every language runs on the generic JIT.**

## End state

Every language in the repo runs on every backend except BEAM, **verified by
running**, and the platform matrix is uniformly green (minus the deliberately-empty
BEAM column for the imperative languages). The capstone is a single
`lang_matrix.rs` suite asserting every `(language, backend)` cell agrees with the
known result — the cross-language analog of McCarthy's W16.

The campaign reaches that end state in two waves: first the **code-generator columns**
(native ✅, **LLVM ✅**, **WASM ✅**, **JVM ✅**, **CLR ✅ — all six languages on all five**)
— general over the shared IIR, so each cell was mostly a conformance test plus the
occasional I/O/type fix; **the entire code-gen wave is complete.** The second wave is the
**execution columns** — the generic register **VM** (`vm_core::VMCore`, which consumes the
shared IIR) and the generic **JIT** (`jit_core::JITCore` + the language-agnostic
`GenericCirJit`, also over the shared IIR). Both are deliberately **generic** so a future
Ruby/JS frontend runs on them with zero rework — the same "shared primitive, no
per-language hack" principle as the code-gen backends. **Both execution columns are now
complete — all six languages run on both — so the entire platform matrix is green (every
language on every backend except BEAM, verified by running).**
