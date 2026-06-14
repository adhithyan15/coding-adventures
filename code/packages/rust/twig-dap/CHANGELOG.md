# Changelog — `twig-dap`

## 0.3.2 — 2026-05-11

**LANG25 follow-up — Hide compiler-internal registers from Variables panel.**

Compiler-generated temporaries (names starting with `_`, e.g. `_r1`, `_n2`)
were leaking into the VS Code Variables panel alongside user-visible names.

### Root cause

`build_sidecar` called `declare_variable` for every instruction destination,
including internal temporaries the user never wrote.

### Fix

In the SSA-temporaries loop (Step 4b of `build_sidecar`), skip any dest name
that starts with `_`.  Slot indices for user-visible variables are unaffected
because both the sidecar and the VM's debug server derive slot indices from the
*full* alphabetically-sorted name list (including `_`-prefixed names).

### New tests

- `internal_temporaries_are_not_declared_as_variables` — confirms `_r1` is
  absent from `live_variables` while user-visible `result` and param `n`
  remain present.
- `internal_temp_slot_indices_do_not_displace_params` — confirms that even
  when `_r1` (alpha-sorted before `n`) occupies slot 0, param `n` is still
  declared with `reg_index = 1`, matching the VM's `get_slot` index.

## 0.3.1 — 2026-05-11

**LANG25 follow-up — alphabetical slot ordering + variable introspection e2e test.**

The 0.3.0 `build_sidecar` assigned slot indices in *declaration order* (params
first, then SSA temps in instruction order).  The twig-vm debug server assigns
them by *alphabetical sort of all live frame names at each stop*.  These two
orderings disagreed, producing wrong values in the variables panel.

### Changes

- `build_sidecar` now assigns `reg_index` by collecting ALL variable names
  (params + instruction dests), sorting alphabetically, and assigning slot
  indices in that order.  This matches the stable ordering used by
  `DebugServer::new_with_module` in twig-vm 0.7.1.
- Updated `use std::collections::{HashMap, HashSet}` (HashSet needed for name
  de-duplication).
- Updated doc-comment on `build_sidecar` to explain the alphabetical slot
  assignment and give a concrete example for `sq(x)`.
- Added `twig-ir-compiler` to `[dev-dependencies]` so the new e2e test can
  call `compile_source` directly.

### New e2e test: `end_to_end_variable_introspection`

The second integration test in `tests/end_to_end.rs` exercises the full chain:

1. Compile `(define (sq x) (* x x))\n(sq 7)` into an IIR module.
2. Call `build_sidecar` to get the sidecar; confirm `x` is declared with a
   stable slot index.
3. Spawn `twig-vm --debug-port` and connect via `TcpVmConnection`.
4. Set a breakpoint at `sq:0`, continue, wait for the breakpoint stop.
5. Read the call stack to find the `sq` frame index.
6. Call `get_slot(frame_idx, x_slot)` and assert the returned string
   contains `"7"` (the argument passed to `sq`).

This is the first test that proves the sidecar `reg_index` and the VM `get_slot`
slot index are consistent end-to-end.

## 0.3.0 — 2026-05-10

**LANG25-25C — Variable introspection: emit variable declarations in the debug sidecar.**

The DAP `variables` panel was always empty because `build_sidecar` never called
`DebugSidecarWriter::declare_variable`.  This release fixes that.

### How the variable→register mapping works

Twig's IIR is in SSA form: every variable name appears as a `dest` exactly once.
The VM's `VMFrame` assigns register slots in two phases:

1. **Parameters** — `VMFrame::for_function` maps `params[i]` to slot `i` before
   execution begins.
2. **SSA temporaries** — `VMFrame::assign` allocates the next sequential slot
   (`name_to_reg.len()` at the point of first write) as each new dest variable is
   encountered at runtime.

`build_sidecar` now mirrors this process statically by walking the
`IIRFunction::instructions` array in declaration order, which equals execution
order for SSA code where each name is defined exactly once.

### Live-range strategy

- **Parameters** are emitted with `live_start=0, live_end=n_instrs` — live for
  the entire function body.
- **SSA temporaries** are emitted with `live_start=def_instr, live_end=n_instrs`
  — visible from the defining instruction through function exit.  This is a
  conservative V1 approximation (the same strategy LLDB uses for `-O0` locals)
  that ensures the variable is always shown once it has a value.

### Changes

- `build_sidecar` now emits parameter and SSA-temporary variable declarations via
  `DebugSidecarWriter::declare_variable` for every function in the module.
- Added `use std::collections::HashMap` (used by the new `name_to_reg` map).
- Updated function-level doc-comment to explain the register-assignment model.
- Removed the "variable declarations are not emitted" known-limitation note.

### Test coverage added (13 new unit tests)

- `params_are_declared_as_variables` — both params of `add(a, b)` appear in the
  variable table.
- `param_register_indices_match_declaration_order` — param 0 → reg 0, param 1 →
  reg 1.
- `params_are_live_for_entire_function` — param is live at all three instruction
  indices.
- `ssa_temp_is_declared_as_variable` — `v0 = const_i32(42)` produces a declared
  variable.
- `ssa_temp_register_comes_after_params` — first temp gets reg 2 when there are 2
  params.
- `ssa_temp_not_live_before_defining_instruction` — `v0` (defined at instr 1) is
  absent at instr 0 and present at instr 1.
- `ssa_temp_live_until_end_of_function` — `v0` remains live at all later
  instructions.
- `type_hint_preserved_for_variable` — `"i32"` type hint round-trips through the
  sidecar for both params and temps.
- `multiple_temps_get_sequential_registers` — three temps defined in order get
  regs 0, 1, 2.
- `void_instructions_do_not_produce_variables` — a `ret` with no `dest` adds
  nothing to the variable table.
- `no_variables_declared_for_function_with_no_params_and_no_dests` — empty
  function returns empty variable list.
- `compile_sidecar_includes_param_variables_for_named_function` — end-to-end:
  compiling `(define (sq x) (* x x))` produces a sidecar where `x` is visible as
  a live variable at instruction 0 of `sq`.

## 0.2.0 — 2026-05-05

**LS03 PR B — Real `TwigDebugAdapter` + `twig-dap` binary.**

The skeleton ships as a complete, working DAP adapter for Twig.  Editors
launch the `twig-dap` binary; it speaks DAP over stdin/stdout to the
editor and the (newline-delimited JSON) VM debug protocol over TCP to
`twig-vm --debug-port <N>`.

### Added
- `TwigDebugAdapter::compile` — runs `twig_ir_compiler::compile_source`
  on the requested file, walks the resulting `IIRFunction::source_map`,
  and emits a `debug_sidecar` byte blob suitable for
  `dap_adapter_core::SidecarIndex`.  Returns the original source path
  as the "bytecode" arg (the `twig-vm` CLI takes Twig source directly —
  there's no separate bytecode artefact).
- `TwigDebugAdapter::launch_vm` — discovers the sibling `twig-vm`
  binary via `std::env::current_exe`'s parent (with `PATH` fallback)
  and spawns it with `--debug-port <PORT> <BYTECODE>`.
- Public `build_sidecar(module, source_path) -> Vec<u8>` helper.
- `find_sibling_binary(name)` — same-directory binary discovery,
  used by `launch_vm` and reusable by other Twig tooling.
- `bin/twig_dap.rs` real `main()` — `DapServer::new(TwigDebugAdapter)
  .run_stdio()`.

### Test coverage
- 8 unit tests for `compile` (sidecar round-trips, line-1 resolves,
  invalid input rejection, missing-file rejection, empty-module shape,
  metadata correctness).
- 1 **end-to-end smoke test** (`tests/end_to_end.rs`) — spawns the real
  `twig-vm` binary in debug mode, connects via `TcpVmConnection`,
  walks through entry stop → set_breakpoint → continue → exited.  Skips
  gracefully when the binary isn't built.

### Dependencies
- `twig-ir-compiler` (workspace path) — for `compile_source`.
- `interpreter-ir` (workspace path) — for `IIRModule` / `SourceLoc`.
- `debug-sidecar` (workspace path) — for `DebugSidecarWriter`.
- `tempfile` (dev-only) — for tests.

## 0.1.0 — 2026-05-04

Initial skeleton. Spec, types, and module structure committed.
Implementation stubs in place with detailed inline TODO guides.
See spec `LS02-grammar-driven-language-server.md` / `LS03-dap-adapter-core.md`.
