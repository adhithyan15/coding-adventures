## 0.69.0 — 2026-06-12 — generic register VM column (LANG-MATRIX Phase V, slice 1)

Begins the **VM column** of the matrix — and does it the way the project intends: a
**generic** register VM, not a per-language one. `lang_matrix.rs` gains a `Backend::Vm`
runner that interprets the **shared** `IIRModule` (the *same* `compile_source_to_iir`
output every code-gen column consumes) on `vm_core::VMCore`, the general register VM whose
instruction dispatch already covers arithmetic / comparison / bitwise / control-flow /
memory / `call_builtin`. **There is no per-language code in the runner** — a future Ruby/JS
frontend that lowers to IIR would run identically.

Correcting a prior mischaracterisation: the LM0 probe's "the VM rejects `add`/`mul`/`cmp_*`"
was about `mccarthy-lisp-vm`, a *deliberately separate* lisp interpreter (its value model is
`lispy-runtime`'s tagged `LispyValue`, and McCarthy lowers arithmetic to `call_builtin`, so
its IIR has no `add`). The general `VMCore` has always handled scalar arithmetic — Brainfuck
ran on it for years.

- `run_vm`: source → `compile_source_to_iir` → `VMCore::execute`. The I/O languages print
  through a registered builtin closure — `print_i64` (Dartmouth BASIC) appends to a capture
  buffer (the VM sibling of the wasm `PrintHost` / LLVM `@__print_i64` / JVM `BasicRuntime` /
  CLR `Console.WriteLine`); `putchar`/`getchar` are registered for the next slice. An
  expression language's `Int` result is the exit code; an I/O language's stdout is the buffer.
- `lang_matrix.rs` adds `Vm` to the `Backend` enum + the floor test (in-process, always runs)
  and tags Twig / Nib / Oct / ALGOL / Dartmouth BASIC with `Vm`.
- Verified by RUNNING in-process: Twig→42, Nib→42, Oct→0, ALGOL `17 mod 5`→2, BASIC→`42`.
  5/6 of the VM column; only Brainfuck-on-VM remains (it needs the `alloc_bytes`/`load_byte`/
  `store_byte` tape ops added to `vm-core`, the next slice). No `vm-core` change in this slice.

