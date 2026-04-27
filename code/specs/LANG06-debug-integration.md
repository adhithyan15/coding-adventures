# LANG06 — Debug Integration: VSCode Debugger for Every Language

## Overview

Any language built on `vm-core` (LANG02) gets full VSCode debugger support —
breakpoints, stepping, stack inspection, variable watching — for free.

The existing specs `05d` (debug sidecar format) and `05e` (debug adapter
protocol) define the mechanism.  This spec defines the **integration points**
that `vm-core` and the language's bytecode compiler must expose so that the
generic debug infrastructure can attach without language-specific modification.

The result:

```
New language "BASIC" built on LANG pipeline
  → tetrad-lexer  → tetrad-parser → tetrad-compiler (emits IIRModule)
  → vm-core executes IIRModule
  → LANG06 debug hooks active automatically
  → VSCode can: set breakpoints, step in/out, inspect registers and call stack
```

No debug-specific code needed in the language's own packages.

---

## What the language must provide

Only one thing: the **bytecode compiler must emit a debug sidecar** alongside
the `IIRModule`.  The debug sidecar (spec `05d`) maps `IIRInstr` indices back
to source file positions.

```python
from interpreter_ir import IIRModule
from debug_sidecar import DebugSidecarWriter

class MyLangCompiler:
    def compile(self, ast, source_path: str) -> tuple[IIRModule, bytes]:
        writer = DebugSidecarWriter(source_path)

        # For each instruction emitted:
        writer.record(instr_index=0, line=10, col=5, scope="main")

        module = IIRModule(...)
        sidecar = writer.finish()
        return module, sidecar
```

That's the entire language-specific contribution.  Everything else is handled
by `vm-core` and the generic debug adapter.

---

## vm-core debug hooks

`vm-core` exposes a `DebugHooks` interface that the debug adapter attaches to:

```python
class DebugHooks:
    """Callbacks the debug adapter registers with vm-core."""

    def on_instruction(self, frame: VMFrame, instr: IIRInstr) -> None:
        """Called before each instruction is dispatched.

        The hook can call vm.pause() to suspend execution.
        """

    def on_call(self, caller: VMFrame, callee: IIRFunction) -> None:
        """Called when a CALL instruction pushes a new frame."""

    def on_return(self, frame: VMFrame, return_value: Any) -> None:
        """Called when a RET instruction pops a frame."""

    def on_exception(self, frame: VMFrame, error: Exception) -> None:
        """Called when an unhandled exception is raised."""
```

```python
# Attaching the debug adapter:
vm = VMCore(...)
adapter = GenericDebugAdapter(sidecar=sidecar_bytes, port=4711)
vm.attach_debug_hooks(adapter.hooks)
vm.execute(module)
```

When `vm.pause()` is called inside `on_instruction`, the dispatch loop
suspends and the debug adapter takes control.

### Pause / resume API

```python
vm.pause()          # suspend at current instruction
vm.step_over()      # resume; pause at next instruction in same frame
vm.step_in()        # resume; pause at first instruction of next called frame
vm.step_out()       # resume; pause at return site of current frame
vm.continue_()      # resume until next breakpoint or end
vm.set_breakpoint(instr_idx: int, fn_name: str)
vm.clear_breakpoint(instr_idx: int, fn_name: str)
```

These map directly to the DAP `next`, `stepIn`, `stepOut`, `continue`,
`setBreakpoints` requests (spec `05e`).

---

## Stack inspection

When paused, the debug adapter can inspect the full call stack:

```python
vm.call_stack() -> list[VMFrame]
```

Each `VMFrame` exposes:

```python
frame.fn.name          # function name
frame.ip               # current instruction index
frame.registers[i]     # current value of register i
```

The debug adapter uses the sidecar to translate `frame.ip` → `(source_line, column)`.

### Variable names

The sidecar (spec `05d`) maps each register index to the source-level variable
name at the current scope.  The debug adapter reads this mapping and presents
`registers[2]` as `x` (or whatever the source-level name is) in the VSCode
Variables panel.

```
VSCode Variables panel:
  Local variables:
    a = 10         (register 0)
    b = 20         (register 1)
    result = 30    (register 2)
```

---

## Breakpoint protocol

Breakpoints are set at source lines.  The generic debug adapter resolves them
to `IIRInstr` indices using the sidecar:

```
User sets breakpoint at line 42 of myprogram.basic
  → adapter reads sidecar: line 42 → instr_index 17 in function "main"
  → adapter calls vm.set_breakpoint(17, "main")
  → vm.on_instruction checks: if ip == 17 and fn == "main": vm.pause()
```

Conditional breakpoints are supported:

```python
vm.set_breakpoint(17, "main", condition="a > 10")
```

The condition is evaluated as an expression over the current register file.
The expression evaluator is a tiny interpreter over `IIRInstr` arithmetic —
no language-specific parser needed.

---

## Hot-reload and live editing

When the source is edited while paused:

1. The language frontend re-compiles the changed function to a new `IIRFunction`
2. `vm-core` patches the running module: `vm.patch_function(fn_name, new_fn)`
3. Execution resumes with the new function body

This is safe when the function is currently on the call stack only if:
- The register count has not changed
- The edit does not affect instructions already executed in the current frame

If either condition is violated, `vm-core` raises `HotReloadConflict` and the
debug adapter notifies the user that a full restart is required.

---

## Integration with JIT

When a function is JIT-compiled (LANG03), the debug hooks become more complex
because the compiled binary does not have per-instruction checkpoints.

`jit-core` respects `vm.is_debug_mode()`:

```python
if vm.is_debug_mode():
    # Do not compile this function; keep it interpreted.
    return False
```

In debug mode, all functions remain interpreted so the `on_instruction` hook
fires for every instruction.  Debugging and maximum performance are mutually
exclusive — this is the same trade-off made by the JVM, V8, and CPython.

---

## VSCode extension

A single generic VSCode extension (`coding-adventures-debug`) works for all
languages built on `vm-core`.  It:

1. Detects the language from the source file extension
2. Launches the correct language's compiler to produce `IIRModule` + sidecar
3. Launches `vm-core` with the debug adapter attached on port 4711
4. Speaks DAP to VSCode

The extension is configured via `.vscode/launch.json`:

```json
{
  "type": "coding-adventures",
  "request": "launch",
  "name": "Debug BASIC program",
  "program": "${file}",
  "language": "basic"
}
```

No language-specific extension needed.  A new language built on the LANG
pipeline automatically works with this extension.

---

## Package additions

The debug integration adds no new packages — it is a set of interfaces in
existing packages:

| Package | Addition |
|---------|----------|
| `vm-core` | `DebugHooks`, `pause/resume/step API`, `set_breakpoint`, `patch_function` |
| `debug-sidecar` (05d) | unchanged — already language-agnostic |
| `debug-adapter` (05e) | `GenericDebugAdapter` uses `IIRModule` + sidecar |

The language bytecode compiler adds a `DebugSidecarWriter` call per emitted
instruction — approximately 3 lines of code per compiler.
