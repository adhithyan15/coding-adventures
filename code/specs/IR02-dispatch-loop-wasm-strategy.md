# IR02 — Dispatch-Loop WASM Lowering Strategy

## Problem

The `ir-to-wasm-compiler`'s existing `_FunctionLowerer` requires **structured
control flow**: it only recognises `JUMP`/`BRANCH` instructions that target
labels matching the naming conventions `loop_N_start` / `loop_N_end` (for
loops) and `if_N_else` / `if_N_end` (for if/else).  Any other label target
raises `WasmLoweringError: unexpected unstructured control flow`.

Languages like Dartmouth BASIC use **unstructured control flow**: `GOTO` jumps
to an arbitrary line label and `IF … THEN` branches to one.  The IR compiler
for BASIC therefore emits `JUMP _line_N` and `BRANCH_NZ v_cmp _line_N`
instructions that the existing lowerer cannot handle.

## Background: How Other Compilers Solve This

Three strategies exist (see research survey in conversation history):

| Strategy | Complexity | Output Quality | Who Uses It |
|---|---|---|---|
| **Dispatch loop** | Low (~200 LOC) | Correct but slower | CPython, Lua VM, all classic BASIC |
| **Relooper** | Medium (~400 LOC) | Good | Emscripten, Binaryen |
| **Stackifier** | High (~600 LOC) | Near-optimal | LLVM WebAssembly backend |

For BASIC programs (10–200 lines, no performance requirements, always
reducible CFGs) the **dispatch loop** is the right choice: it is trivially
correct for all possible control flow and requires far less implementation
complexity than Relooper or Stackifier.

## Design: Dispatch-Loop Lowering Strategy

### Core Idea

Replace arbitrary labels and jumps with a virtual **program counter** (`$pc`)
stored in a WASM local variable, and a `block { loop { … } }` dispatch table
that routes execution to the right segment each iteration.

```
┌─────────────────────────────────────────┐
│  block $prog_end                        │
│    loop $dispatch                       │
│      block $seg_0   ; label _start      │
│        pc ≠ 0 → br 0  (skip)           │
│        [instructions for segment 0]     │
│        pc = next_idx; br 1 (continue)  │
│      end                                │
│      block $seg_1   ; label _line_10    │
│        pc ≠ 1 → br 0  (skip)           │
│        [instructions for segment 1]     │
│        pc = next_idx; br 1             │
│      end                                │
│      … one block per label …           │
│    end loop  ; falls through → exit    │
│  end block                             │
└─────────────────────────────────────────┘
```

### Terminology

| Term | Meaning |
|---|---|
| **Segment** | The sequence of IR instructions between two consecutive `LABEL` opcodes |
| **Segment index** | A small integer assigned to each label in instruction-stream order |
| `$pc` | A WASM `i32` local that holds the segment index currently being executed |

### br Depth Table

All br depths are measured from *inside a segment block*.

| Instruction | Target depth | Meaning |
|---|---|---|
| `br 0` | `block $seg_N` | Skip this segment (pc didn't match) |
| `br 1` | `loop $dispatch` | Continue dispatch (re-enter loop from top) |
| `br 2` | `block $prog_end` | Exit program (HALT / END / STOP) |

When the lowerer emits an `if` block to implement `BRANCH_Z` / `BRANCH_NZ`,
all depths increase by one while inside that `if`:

| Instruction | Target depth (inside `if`) | Meaning |
|---|---|---|
| `br 0` | `if` | End of if block |
| `br 1` | `block $seg_N` | Skip this segment |
| `br 2` | `loop $dispatch` | Continue dispatch |
| `br 3` | `block $prog_end` | Exit program |

### Segment Termination

Each segment ends in exactly one of four ways:

| Terminator | IR Opcode | Emitted WASM |
|---|---|---|
| Unconditional jump | `JUMP label` | `i32.const target_idx; local.set $pc; br 1` |
| Conditional branch | `BRANCH_Z reg, label` / `BRANCH_NZ reg, label` | `if { i32.const target_idx; local.set $pc; br 2 } end` + fall-through |
| Halt | `HALT` | `br 2` |
| Fall-through | (reaches next `LABEL`) | `i32.const next_idx; local.set $pc; br 1` |

Note that `BRANCH_Z` / `BRANCH_NZ` does **not** terminate the segment — it
emits a conditional jump but then falls through to the next instruction.  The
segment is only truly terminated when no more IR instructions remain before
the next label.

### Local Variable Layout

The dispatch loop lowerer adds exactly **one extra local** (`$pc`) after all
the IR virtual-register locals:

```
index 0 .. param_count-1        : function parameters (copied from WASM params)
index param_count .. param_count+max_reg-1 : IR virtual registers (r0, r1, …)
index param_count + max_reg     : $pc  (dispatch loop program counter)
```

The existing `_local_index(reg)` mapping (`param_count + reg`) is unchanged,
so all existing `_emit_simple` helpers work without modification.

### WASI / Memory

The dispatch loop lowerer uses the **same WASI context** as the structured
lowerer: the same scratch memory layout (`iovec` at offset 0, `nwritten` at
8, byte at 12) and the same `fd_write` function-index plumbing.  No new
memory or import infrastructure is required.

## Interface

### New parameter: `strategy`

```python
class IrToWasmCompiler:
    def compile(
        self,
        program: IrProgram,
        function_signatures: list[FunctionSignature] | None = None,
        *,
        strategy: str = "structured",   # NEW
    ) -> WasmModule: ...
```

`strategy` may be:

| Value | Behaviour |
|---|---|
| `"structured"` | Existing behaviour — uses `_FunctionLowerer`; raises `WasmLoweringError` on unstructured CF |
| `"dispatch_loop"` | New behaviour — uses `_DispatchLoopLowerer`; handles arbitrary jumps |

### New class: `_DispatchLoopLowerer`

Lives in `ir_to_wasm_compiler/compiler.py` alongside `_FunctionLowerer`.
Same constructor signature as `_FunctionLowerer`.  Implements `.lower() →
FunctionBody`.

## Algorithm

```
_DispatchLoopLowerer.lower():
  1. index_labels()          — walk instructions, assign segment indices
  2. emit_pc_init()          — i32.const 0; local.set $pc
  3. emit_block_open()       — block (void)
  4. emit_loop_open()        — loop (void)
  5. for each consecutive LABEL in instructions:
       emit_segment(seg_idx, instructions_until_next_label)
  6. emit_loop_close()       — end
  7. emit_block_close()      — end
  8. emit_end()              — end  (function return)
  9. return FunctionBody(locals=(..., i32), code=bytes)

emit_segment(seg_idx, instrs):
  1. emit: block (void)
  2. emit: local.get $pc; i32.const seg_idx; i32.ne; br_if 0
  3. for each instr in instrs:
       JUMP label       → emit_jump(label); terminated=True; break
       BRANCH_Z/NZ reg  → emit_conditional_branch(reg, label)
       HALT             → emit_halt(); terminated=True; break
       SYSCALL n        → emit_syscall(n)   (reuse existing helper)
       COMMENT          → skip
       other            → emit_simple(instr) (reuse existing helper)
  4. if not terminated:
       emit: i32.const (seg_idx+1); local.set $pc; br 1
  5. emit: end
```

## Correctness Properties

1. **All IR labels are reachable**: Every label becomes a segment. The
   dispatch loop tests `$pc` against each segment index in order; when a
   match is found the segment executes.

2. **Arbitrary forward and backward jumps**: `JUMP label` and `BRANCH_Z/NZ`
   simply set `$pc` to the target's segment index and restart the loop — there
   is no notion of "forward only" or "backward only".

3. **Fall-through preserved**: The source-order sequence of labels is also the
   segment-index sequence.  A fall-through emits `$pc = seg_idx + 1`, which
   is exactly the next segment in instruction-stream order — matching the
   original IR's implicit sequential execution.

4. **Identical SYSCALL output**: The WASI `fd_write` helper is reused
   unchanged, so all `PRINT` output is byte-for-byte identical to what the
   structured lowerer would produce on programs it can handle.

## Limitations

1. **Performance**: Each dispatch iteration scans all segments in order —
   O(n) per step.  For BASIC programs with n < 200 lines this is irrelevant.

2. **Dead segments**: An unreachable segment (e.g. code after an
   unconditional `END`) still appears in the dispatch table and is visited
   (and immediately skipped) on every iteration.

3. **`CALL` / multi-function programs**: The dispatch loop lowerer applies
   per-function, same as the structured lowerer.  Inter-function calls still
   use the existing `CALL` instruction path.  However, it is the caller's
   responsibility to ensure all functions in the program use the same
   strategy; mixing strategies in one module is not supported.

## Packages Affected

| Package | Change |
|---|---|
| `ir-to-wasm-compiler` | Add `_DispatchLoopLowerer`; add `strategy` parameter to `IrToWasmCompiler.compile()` |
| `dartmouth-basic-wasm-compiler` | Pass `strategy="dispatch_loop"` in runner |
| All other packages | Unchanged |

## Testing

### `ir-to-wasm-compiler` tests

Add a new `TestDispatchLoopLowerer` class that exercises:

- Simple forward jump (`JUMP label`)
- Simple conditional branch (`BRANCH_Z`, `BRANCH_NZ`)
- Backward jump (loop)
- Fall-through (no explicit jump at end of segment)
- `HALT` exits the program
- Mixed: multiple jumps and fall-throughs in the same program

### `dartmouth-basic-wasm-compiler` tests

The 37 existing end-to-end tests become the integration test suite — all
should pass once the dispatch loop strategy is wired in.
