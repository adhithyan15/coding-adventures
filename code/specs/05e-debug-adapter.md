# 05e — Debug Adapter Protocol Integration

## Overview

VS Code does not know how to talk to your VM. It speaks a standardised protocol called the **Debug Adapter Protocol (DAP)** — a JSON-RPC dialect over stdio or TCP — and expects a small program called a *debug adapter* to translate between DAP and whatever runtime it is debugging.

This spec describes:
1. What DAP is and how it works
2. The **VM Debug Protocol** — a lightweight wire protocol your VM exposes for debugger control
3. The **debug adapter** that bridges the two using the sidecar (spec `05d`)
4. The **VS Code extension** that wires the whole thing together
5. The algorithms for step-over, step-in, and step-out

The result: every language built on the generic VM gets breakpoints, stepping, and variable inspection in VS Code **for free**, by inheriting this infrastructure.

## Full Architecture

```
┌──────────────────────────────────────────────────────────────┐
│  VS Code                                                     │
│  • shows source file          • highlights current line      │
│  • renders breakpoint glyphs  • shows call stack panel       │
│  • shows variable values      • shows watch expressions      │
└──────────────────┬───────────────────────────────────────────┘
                   │  Debug Adapter Protocol (DAP)
                   │  JSON over stdio (or TCP port)
┌──────────────────▼───────────────────────────────────────────┐
│  Debug Adapter                                               │
│  • speaks DAP to VS Code                                     │
│  • speaks VM Debug Protocol to VM                            │
│  • reads .dbg sidecar for offset↔source translation         │
│  • implements stepping algorithms                            │
└──────────────────┬──────────────┬────────────────────────────┘
                   │              │
          VM Debug Protocol    reads .dbg sidecar
          (TCP socket)
┌──────────────────▼───────────────────────────────────────────┐
│  VM (runtime)                                                │
│  • executes bytecode                                         │
│  • pauses at breakpoint offsets                              │
│  • exposes slots/registers on demand                        │
│  • emits events: stopped, exited                             │
└──────────────────────────────────────────────────────────────┘
```

The key insight is **separation of concerns**:
- The VM knows only about offsets, slots, and registers — no source code.
- The debug adapter knows about both worlds and translates between them.
- VS Code knows only about source code — no offsets.

The sidecar is the dictionary the adapter uses to translate.

## What is DAP?

DAP was created by Microsoft so that one debugger implementation can work with any editor (VS Code, Vim, Emacs, IntelliJ). Before DAP, every editor had to implement its own debugger integration for every language. Now each language writes one debug adapter and every DAP-compatible editor gets it for free.

The protocol is request-response with unsolicited events:

```
VS Code → Adapter:  { "type": "request",  "command": "setBreakpoints", ... }
Adapter → VS Code:  { "type": "response", "command": "setBreakpoints", ... }

Adapter → VS Code:  { "type": "event",    "event": "stopped", "reason": "breakpoint" }
```

Each message is preceded by a `Content-Length` HTTP-style header, followed by a blank line, then the JSON body. This is identical to how the Language Server Protocol works.

## VM Debug Protocol

The VM exposes a minimal TCP server when launched in debug mode. The adapter connects to this server and sends commands as newline-delimited JSON.

### Launch

The VM is launched with a debug flag:
```
vm --debug-port 54321 program.bytecode
```

The VM starts, opens a TCP server on port 54321, and **waits** (does not begin executing) until the adapter sends `CONTINUE`.

### Commands (adapter → VM)

```
{ "cmd": "set_breakpoint",   "offset": 3 }
{ "cmd": "clear_breakpoint", "offset": 3 }
{ "cmd": "continue" }
{ "cmd": "step_instruction" }   ← execute exactly one bytecode instruction
{ "cmd": "get_call_stack" }
{ "cmd": "get_slot",    "frame": 0, "slot": 2 }
{ "cmd": "get_register","reg": 2 }
{ "cmd": "pause" }              ← suspend execution asynchronously
```

### Events (VM → adapter)

```
{ "event": "stopped",  "reason": "breakpoint",  "offset": 3 }
{ "event": "stopped",  "reason": "step",         "offset": 5 }
{ "event": "stopped",  "reason": "pause",        "offset": 11 }
{ "event": "stopped",  "reason": "exception",    "offset": 7,  "message": "division by zero" }
{ "event": "exited",   "exit_code": 0 }
```

### Responses

Every command receives a JSON response before any follow-up events:

```
{ "ok": true }
{ "ok": true,  "stack": [{"offset": 3, "unit_id": 0}, {"offset": 41, "unit_id": 1}] }
{ "ok": true,  "value": { "kind": "integer", "repr": "42" } }
{ "ok": false, "error": "offset 99 is out of range" }
```

### Value representation

Values are serialised as tagged objects so the adapter can display them correctly without knowing the language's type system:

```json
{ "kind": "integer",  "repr": "42" }
{ "kind": "float",    "repr": "3.14" }
{ "kind": "string",   "repr": "\"hello\"" }
{ "kind": "boolean",  "repr": "true" }
{ "kind": "nil",      "repr": "nil" }
{ "kind": "list",     "repr": "[3 items]",  "ref": 7 }
{ "kind": "map",      "repr": "{2 keys}",   "ref": 8 }
{ "kind": "object",   "repr": "Point",      "ref": 9 }
```

Compound values (list, map, object) get a `ref` — a handle the adapter can pass to `get_children` to lazily expand the value in the Variables panel.

## VM Implementation Requirements

For the VM to support this protocol, it needs two additions to its eval loop:

### 1. Breakpoint table

Before executing each instruction, the VM checks if the current offset is in the breakpoint set:

```python
def eval_loop(self):
    while True:
        # Check for breakpoint BEFORE executing the instruction
        if self.ip in self.breakpoints:
            self.send_event({"event": "stopped", "reason": "breakpoint", "offset": self.ip})
            self.wait_for_continue()  # blocks until adapter sends "continue" or "step_instruction"

        instruction = self.fetch()
        self.execute(instruction)
```

The breakpoint check adds one hash-set lookup per instruction when in debug mode. In release mode (no debug server), the check is never installed.

### 2. Call stack tracking

The VM maintains a list of active frames for `get_call_stack`:

```python
@dataclass
class Frame:
    unit_id: int       # which execution unit (from the sidecar)
    offset: int        # current instruction pointer within this unit
    base_slot: int     # where this frame's local slots start on the stack
```

The adapter translates `unit_id` + `offset` to a human-readable frame name and source location using the sidecar.

## Debug Adapter Implementation

### Initialisation sequence

```
VS Code                          Adapter                    VM
  │                                │                         │
  ├──initialize──────────────────▶ │                         │
  │ ◀──────────────initialized───── │                         │
  │                                │                         │
  ├──launch (program.bytecode)────▶ │                         │
  │                                ├──spawn VM────────────── ▶ │
  │                                ├──connect TCP────────── ▶ │
  │                                │ ◀──ready────────────────  │
  │ ◀──────────────initialized───── │                         │
  │                                │                         │
  ├──setBreakpoints (line 20)─────▶ │                         │
  │                                ├──source_to_offset(20)   │  (reads .dbg)
  │                                ├──set_breakpoint(0x03)── ▶ │
  │ ◀──────────────breakpoints─────  │                         │
  │                                │                         │
  ├──configurationDone────────────▶ │                         │
  │                                ├──continue──────────────▶ │
  │                                │                         │
  │                                │ ◀──stopped(offset=0x03)─  │  (breakpoint hit)
  │ ◀──────────────stopped event─── │                         │
```

### DAP request handlers

**`setBreakpoints`** — VS Code sends a list of (file, line) pairs:
```python
def handle_set_breakpoints(request):
    source_file = request.source.path
    for bp in request.breakpoints:
        offset = sidecar.source_to_offset(source_file, bp.line)
        if offset:
            vm.set_breakpoint(offset)
            result.append({"verified": True, "line": bp.line})
        else:
            result.append({"verified": False, "message": "line not reachable"})
    return result
```

**`stackTrace`** — VS Code asks for the call stack when paused:
```python
def handle_stack_trace(request):
    frames_from_vm = vm.get_call_stack()
    result = []
    for vm_frame in frames_from_vm:
        unit  = sidecar.find_unit(vm_frame.unit_id)
        loc   = sidecar.offset_to_source(vm_frame.offset)
        result.append({
            "id":     vm_frame.unit_id,
            "name":   unit.name,
            "source": {"path": sidecar.source_files[loc.file_id].path},
            "line":   loc.line,
            "column": loc.column,
        })
    return result
```

**`scopes`** — VS Code asks what variable groups exist in a frame (e.g. "Locals", "Parameters"):
```python
def handle_scopes(frame_id):
    return [
        {"name": "Locals",     "variablesReference": make_ref(frame_id, "locals")},
        {"name": "Parameters", "variablesReference": make_ref(frame_id, "params")},
    ]
```

**`variables`** — VS Code asks for the actual values:
```python
def handle_variables(variables_reference):
    frame_id, group = decode_ref(variables_reference)
    live_vars = sidecar.live_variables(unit_id=frame_id, at_offset=vm.current_offset())
    result = []
    for var in live_vars:
        value = vm.get_slot(frame=frame_id, slot=var.slot)
        result.append({
            "name":               var.name,
            "value":              value.repr,
            "type":               var.type_hint,
            "variablesReference": value.ref if value.kind in ("list","map","object") else 0,
        })
    return result
```

### Stepping algorithms

Stepping is implemented entirely in the adapter — the VM only knows about `step_instruction` (advance one bytecode instruction). The adapter uses the sidecar to decide when to stop.

**Step over (`next`)** — advance until we're on a *different source line at the same or shallower call depth*:

```python
def step_over():
    start_loc   = sidecar.offset_to_source(vm.current_offset())
    start_depth = len(vm.get_call_stack())

    while True:
        vm.step_instruction()  # advance one bytecode instruction
        event = vm.wait_for_event()

        if event.kind == "exited":
            break

        current_loc   = sidecar.offset_to_source(event.offset)
        current_depth = len(vm.get_call_stack())

        # Stop if we're on a new line AND we haven't gone deeper
        if current_loc != start_loc and current_depth <= start_depth:
            break
```

**Step in** — advance until *any* source line changes:

```python
def step_in():
    start_loc = sidecar.offset_to_source(vm.current_offset())
    while True:
        vm.step_instruction()
        event = vm.wait_for_event()
        if event.kind == "exited":
            break
        current_loc = sidecar.offset_to_source(event.offset)
        if current_loc != start_loc:
            break
```

**Step out** — advance until the call stack depth *decreases* (we've returned from the current function):

```python
def step_out():
    start_depth = len(vm.get_call_stack())
    while True:
        vm.step_instruction()
        event = vm.wait_for_event()
        if event.kind == "exited":
            break
        current_depth = len(vm.get_call_stack())
        if current_depth < start_depth:
            break
```

These three algorithms are language-agnostic. They work for BASIC, Python, Lua, or any other language compiled through this toolchain, because they reason only about source locations and call depth — not about language-specific constructs.

## VS Code Extension

The extension is the smallest piece. Its job is to:
1. Tell VS Code that it handles `.bas` files (or `.py`, `.lua`, etc.)
2. Describe how to launch the debug adapter when a debug session starts
3. Provide a launch configuration template for the user

### Directory structure

```
vscode-basic-debug/
├── package.json          ← extension manifest
├── src/
│   └── extension.ts      ← activation code (minimal)
└── adapter/
    └── basic_adapter     ← the compiled debug adapter binary
```

### package.json (key sections)

```json
{
  "name": "vscode-basic-debug",
  "contributes": {
    "languages": [{
      "id": "basic",
      "extensions": [".bas"],
      "aliases": ["BASIC"]
    }],
    "debuggers": [{
      "type": "basic",
      "label": "BASIC Debugger",
      "program": "./adapter/basic_adapter",
      "runtime": "node",
      "languages": ["basic"],
      "configurationAttributes": {
        "launch": {
          "required": ["program"],
          "properties": {
            "program": {
              "type": "string",
              "description": "Path to the .bas file to run"
            },
            "stopOnEntry": {
              "type": "boolean",
              "default": true
            }
          }
        }
      },
      "initialConfigurations": [{
        "type": "basic",
        "request": "launch",
        "name": "Run BASIC program",
        "program": "${file}",
        "stopOnEntry": true
      }]
    }]
  }
}
```

### What the extension.ts does

```typescript
import * as vscode from 'vscode';

export function activate(context: vscode.ExtensionContext) {
    // Register a factory that creates debug adapter sessions.
    // VS Code calls this when the user starts debugging.
    context.subscriptions.push(
        vscode.debug.registerDebugAdapterDescriptorFactory('basic',
            new BasicDebugAdapterFactory(context)
        )
    );
}

class BasicDebugAdapterFactory implements vscode.DebugAdapterDescriptorFactory {
    createDebugAdapterDescriptor(session: vscode.DebugSession) {
        // Launch our adapter as an external process communicating over stdio.
        return new vscode.DebugAdapterExecutable(
            this.context.asAbsolutePath('./adapter/basic_adapter'),
            []
        );
    }
}
```

That is nearly the entire extension. VS Code handles everything else — the breakpoint UI, the Variables panel, the Call Stack panel, the step buttons — because the adapter speaks standard DAP.

## Reuse Across Languages

Every language in the toolchain follows the same pattern:

1. The **compiler** emits `.dbg` alongside `.bytecode` (using the sidecar writer from `05d`).
2. The **VM** is the same generic VM, always listening on a debug port when `--debug-port` is passed.
3. The **debug adapter** is a thin language-specific shell around a **generic adapter library** that handles all DAP boilerplate and the stepping algorithms. The only language-specific code is the adapter's launch configuration (how to compile and run the language's source file).
4. The **VS Code extension** is per-language but tiny — it just registers the file extension and points at the adapter binary.

The generic adapter library does the heavy lifting. A new language author writes:

```
BasicAdapter extends GenericAdapter:
    compile(source_file):
        run("basic_compiler", source_file)   → produces .bytecode + .dbg
    launch_vm(bytecode_file, debug_port):
        run("vm", "--debug-port", debug_port, bytecode_file)
```

Everything else — breakpoints, stepping, variable inspection, call stack — is inherited from the generic adapter.

## Launch vs Attach

The above describes the **launch** flow (the adapter starts the VM). The protocol also supports **attach** (connecting to an already-running VM). This is useful for long-running programs where you want to attach mid-execution:

```json
{
  "type": "basic",
  "request": "attach",
  "name": "Attach to running BASIC program",
  "debugPort": 54321
}
```

In attach mode, the adapter skips the compile and spawn steps and connects directly to the running VM's debug port.
