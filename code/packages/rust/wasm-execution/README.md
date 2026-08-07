# wasm-execution

WebAssembly 1.0 wasm-execution.

As of 0.3.0 it executes the **WasmGC** value-model opcodes (LANG77 / McCarthy
L3b-3a), so the McCarthy-Lisp → wasm value model can be *run* in-repo, not just
emitted:

- **`i31.new` / `i31.get_s`** (0.2.0) — integer boxing; an `i31ref` is carried
  as its `i32` payload on the value stack (box/unbox are stack-identity no-ops).
- **`struct.new` / `struct.get` / `struct.set`** and **`ref.null` / `ref.is_null`**
  (0.3.0) — a GC object heap backs the lisp **cons cell** (`$LispyPair`).
  References are a new `WasmValue::Ref(Option<handle>)` (`None` = null = lisp
  `nil`). `(CAR (CONS 7 9))` executes to `7`.  Register struct field counts via
  `WasmExecutionEngine::set_struct_field_counts` (the parser will supply these
  automatically in L3b-3a-3c).
- **`ref.test` / `ref.test null`** (0.4.0) — the WasmGC type-test op that
  McCarthy `pair?` lowers to: pop a reference, push `i32 1` if it is a (non-null)
  `$LispyPair` struct reference, else `0`. `pair?(cons)` → 1; `pair?(atom)` and
  `pair?(nil)` → 0.
- **Real garbage collection** (0.6.0, W04) — the object heap is a real
  mark-and-sweep collector, not an append-only arena: unreachable objects
  are reclaimed and their slots reused, so a long-running loop or recursive
  program no longer grows the heap without bound. Precise, cycle-safe root
  scanning (globals, locals, every suspended caller frame, the operand
  stack) with no per-type schema needed — a `GcStruct` field is a tagged
  `WasmValue`, self-describing as a reference or not. Checked at loop
  back-edges and calls (mirroring the same "safepoints" convention the
  native-AOT collector uses), with an adaptive object-count threshold
  mirroring `gc-core::FlatHeap`'s heuristic. Compaction is explicitly out of
  scope — a `Ref` handle is a `Vec` index, so relocating objects would need
  rewriting every handle-holding location instead of just this collector's
  own bookkeeping. See `code/specs/W04-wasm-gc.md` and
  `WasmExecutionEngine::gc_live_object_count()` / `gc_profile()`.

All GC failure modes (null dereference, out-of-range field, missing arity,
unknown opcode, or a handle to an already-collected object) are clean
traps, never panics.

## Dependencies

- gc-core (W04 — `GcProfile`/`GcCycleStats` diagnostics for the object-heap collector)
- wasm-leb128
- wasm-types
- wasm-opcodes
- wasm-module-parser
- virtual-machine

## Development

```bash
# Run tests
bash BUILD
```
