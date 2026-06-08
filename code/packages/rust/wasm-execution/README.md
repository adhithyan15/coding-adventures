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
  `nil`); the heap is an append-only arena bounded by the VM instruction budget.
  `(CAR (CONS 7 9))` executes to `7`.  Register struct field counts via
  `WasmExecutionEngine::set_struct_field_counts` (the parser will supply these
  automatically in L3b-3a-3c).
- **`ref.test` / `ref.test null`** (0.4.0) — the WasmGC type-test op that
  McCarthy `pair?` lowers to: pop a reference, push `i32 1` if it is a (non-null)
  `$LispyPair` struct reference, else `0`. `pair?(cons)` → 1; `pair?(atom)` and
  `pair?(nil)` → 0.

All GC failure modes (null dereference, out-of-range field, missing arity,
unknown opcode) are clean traps, never panics.

## Dependencies

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
