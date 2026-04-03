# Changelog — @coding-adventures/paint-vm

## 0.1.0 — 2026-04-03

Initial release implementing the P2D01 PaintVM dispatch-table VM spec.

### Added

- `PaintVM<TContext>` class — generic dispatch-table VM for PaintInstructions
  - `register(kind, handler)` — registers a handler; throws `DuplicateHandlerError` on duplicate
  - `dispatch(instruction, context)` — routes one instruction to its handler
  - `execute(scene, context)` — immediate mode: clear + dispatch all instructions
  - `patch(old, next, context, callbacks?)` — retained mode: structural diff with onDelete/onInsert/onUpdate callbacks; falls back to execute() when no callbacks provided
  - `export(scene, options?)` — pixel export via backend-supplied `exportFn`; throws `ExportNotSupportedError` when not supported
  - `registeredKinds()` — returns list of registered instruction kinds (for debugging/testing)
- Wildcard `"*"` handler support for opt-in graceful degradation
- `ExportOptions` interface: `scale`, `channels`, `bit_depth`, `color_space`
- Error classes: `UnknownInstructionError`, `DuplicateHandlerError`, `ExportNotSupportedError`, `NullContextError`
- `deepEqual()` helper — structural equality for patch() diffing; compares primitives, arrays, and plain objects recursively
