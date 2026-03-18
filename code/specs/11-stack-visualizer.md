# 11 — Stack Visualizer (TUI)

## Overview

The stack visualizer is a Terminal UI (TUI) application built with Textual that visually walks through every stage of the computing stack. It takes a source program and shows — in real time — how it transforms from text to tokens to AST to bytecode to execution.

This is a program (in `code/programs/`), not a package. It is the primary way to experience the computing stack interactively.

## What it looks like

```
┌─────────────────────────────────────────────────────────────────────┐
│  Source Code                                                        │
│  x = 1 + 2                                                         │
├────────────────┬────────────────┬────────────────┬─────────────────┤
│  Tokens        │  AST           │  Bytecode      │  Execution      │
│                │                │                │                 │
│  NAME 'x'     │  Assignment    │  LOAD_CONST 1  │  Stack: []      │
│  EQUALS       │  ├─ Name(x)   │  LOAD_CONST 2  │  Stack: [1]     │
│  NUMBER '1'   │  └─ BinaryOp  │  ADD           │  Stack: [1, 2]  │
│  PLUS         │     ├─ 1      │  STORE 'x'    │  Stack: [3]     │
│  NUMBER '2'   │     ├─ +      │  HALT          │  Stack: []      │
│  EOF          │     └─ 2      │                │  Vars: {x: 3}  │
├────────────────┴────────────────┴────────────────┴─────────────────┤
│  [Step] [Run] [Reset]   Target: [VM] [RISC-V] [ARM]   Step: 3/5   │
└─────────────────────────────────────────────────────────────────────┘
```

## Features

### MVP
- Source code input area at the top
- Four panels: Tokens, AST, Bytecode/Assembly, Execution trace
- **Step** button: advance one bytecode/machine instruction
- **Run** button: execute all remaining instructions
- **Reset** button: restart from the beginning
- Current instruction highlighted in the bytecode panel
- Current stack state shown in the execution panel
- Target selector: VM, RISC-V, ARM

### Visual feedback
- Stages that have been processed get a green border
- The currently active stage gets a yellow/highlighted border
- The current instruction in bytecode/assembly is highlighted
- Stack pushes and pops are visually indicated

## Technology

- **Textual** — Python TUI framework (rich terminal UI with widgets, CSS-like styling)
- **Pipeline package** — provides all data via PipelineResult and stage snapshots

## Key classes

```python
class StackVisualizer(App):
    """Main Textual application."""

    def compose(self) -> ComposeResult: ...
        # Build the UI layout

    def on_button_pressed(self, event: Button.Pressed) -> None: ...
        # Handle Step, Run, Reset buttons

    def action_step(self) -> None: ...
        # Execute one instruction, update all panels

    def action_run(self) -> None: ...
        # Execute all remaining instructions

    def action_reset(self) -> None: ...
        # Re-run the pipeline, reset to step 0

class TokenPanel(Static): ...
class ASTPanel(Static): ...
class BytecodePanel(Static): ...
class ExecutionPanel(Static): ...
```

## Data Flow

```
Input:  Source code entered by user in the TUI
Output: Visual display of all pipeline stages + step-through execution
```

## Dependencies

- `textual>=0.50`
- `coding-adventures-pipeline`

## Test Strategy

- Snapshot tests: verify panel content for known programs
- Verify step advances correctly (instruction index increments)
- Verify reset returns to initial state
- Verify target switching re-runs the pipeline

## Future Extensions

- **Editable source:** Live re-compilation as you type
- **Register view:** For hardware targets, show register state
- **Memory view:** Hex dump of simulated memory
- **Breakpoints:** Set breakpoints on specific instructions
- **Diff view:** Side-by-side comparison of VM vs hardware execution
