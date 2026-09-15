## 0.70.0 — 2026-06-13 — Brainfuck on the VM — VM COLUMN COMPLETE (LANG-MATRIX Phase V, slice 2)

Completes the **VM column**: Brainfuck now runs on the generic register VM too, so **all
six matrix languages run on the one `vm_core::VMCore` interpreter** via the shared IIR.
Verified by RUNNING `++++++++[>++++++++<-]>+.` on the VM: it prints `A`.

The lowering work is in `vm-core` 0.4.0 (the byte-tape ops `alloc_bytes`/`load_byte`/
`store_byte` over its flat `memory` — see that crate's changelog). This crate's change is
the test tag: `tests/lang_matrix.rs` adds `Vm` to the Brainfuck `Prog`. `run_vm` already
registered the `putchar` builtin (slice 1), so Brainfuck's `.` captures bytes (→ `A`) with
no further wiring. No `lang-aot/src` change.

