# PR71: Prolog Bytecode VM Convergence

## Goal

Make the Prolog-on-Logic-VM stack runnable through the compact bytecode layer,
not only through the structured `logic-vm` instruction loader.

This closes the next convergence gap:

```text
Prolog source
  -> dialect parser
  -> prolog-loader
  -> prolog-vm-compiler
  -> logic-instructions
  -> logic-bytecode
  -> logic-bytecode-vm
  -> logic-engine proof search
```

## Scope

- Preserve dynamic relation declarations in `logic-bytecode` using a dedicated
  `EMIT_DYNAMIC_RELATION` opcode.
- Teach `logic-bytecode-vm` to load dynamic declarations, expose them in the
  assembled engine program, and run stored queries from an existing
  `logic-engine.State`.
- Add Prolog compiler APIs that lower a compiled Prolog program to bytecode and
  execute source queries through `logic-bytecode-vm`.
- Add bytecode-backed stateful runtimes for source strings, files, linked
  source projects, and linked file projects.
- Keep structured `logic-vm` APIs intact while making the bytecode path share
  the same answer and initialization machinery.

## Public API Additions

- `compile_prolog_to_bytecode(compiled_program)`
- `load_compiled_prolog_bytecode_vm(compiled_program)`
- `run_compiled_prolog_bytecode_query(...)`
- `run_compiled_prolog_bytecode_query_answers(...)`
- `run_compiled_prolog_bytecode_queries(...)`
- `run_compiled_prolog_bytecode_initializations(...)`
- `run_initialized_compiled_prolog_bytecode_query(...)`
- `run_initialized_compiled_prolog_bytecode_query_answers(...)`
- `create_prolog_bytecode_vm_runtime(...)`
- `create_prolog_source_bytecode_vm_runtime(...)`
- `create_swi_prolog_bytecode_vm_runtime(...)`
- `create_iso_prolog_bytecode_vm_runtime(...)`
- file and project variants for bytecode-backed stateful runtimes

## Acceptance Tests

- Dynamic relation declarations round-trip through bytecode encode/decode and
  disassembly.
- `logic-bytecode-vm` preserves dynamic declarations after loading.
- `logic-bytecode-vm` can execute stored queries from a caller-provided state.
- A compiled Prolog source query returns the same answers through
  `logic-vm` and `logic-bytecode-vm`.
- Initialization goals can seed dynamic database state before a bytecode-backed
  source query runs.
- A bytecode-backed ad-hoc runtime can persist `assertz/1` effects with
  `commit=True` and use them in later queries.

## Out Of Scope

This is still a loader-bytecode VM, not a WAM. Clause indexing, WAM registers,
choicepoint bytecode, and instruction-level proof-search opcodes remain future
optimization layers.
