# NIB02 — Nib to WASM Pipeline

## Overview

This spec defines a unix-pipes-style compilation path from Nib source to a
real WebAssembly 1.0 binary:

```
Nib source
  -> nib-parser
  -> nib-type-checker
  -> nib-ir-compiler
  -> ir-optimizer
  -> ir-to-wasm-compiler
  -> wasm-module-encoder
  -> .wasm bytes
```

The key design constraint is package composability:

- `ir-to-wasm-compiler` is generic over `IrProgram`
- `wasm-module-encoder` is generic over `WasmModule`
- `nib-wasm-compiler` is only orchestration glue

This mirrors the existing Intel 4004 flow, where source compilation,
target-specific lowering, and final packaging are separate tools.

## Package Decomposition

### 1. `wasm-module-encoder`

Input:

- `wasm_types.WasmModule`

Output:

- raw `.wasm` bytes

Responsibilities:

- emit the WASM header
- encode standard sections in canonical order
- stay reusable for any producer that can build a `WasmModule`

### 2. `ir-to-wasm-compiler`

Input:

- `compiler_ir.IrProgram`
- explicit function signature hints (`label -> param_count/export_name`)

Output:

- `wasm_types.WasmModule`

Responsibilities:

- map IR virtual registers to WASM locals
- lower IR arithmetic and memory ops to WASM instructions
- turn reducible Nib-style label/branch patterns into structured WASM control flow
- lay out IR `.data` declarations into linear memory

### 3. `nib-wasm-compiler`

Input:

- Nib source text

Output:

- all intermediate artifacts plus final `.wasm` bytes

Responsibilities:

- run parse/type-check/IR/optimize stages
- derive function signature hints from the typed Nib AST
- call `ir-to-wasm-compiler`
- validate and encode the resulting module

## Prototype ABI

The prototype uses a simple, uniform ABI between IR and WASM:

- every lowered IR function returns one `i32`
- the return value is always the current value of IR register `v1`
- Nib functions without an explicit source-level return still produce `0`
  because WASM locals default to zero and the frontend emits a trailing `RET`
- function parameters are surfaced as WASM params, then copied into IR working
  registers `v2`, `v3`, `v4`, ...

This keeps the lowering generic and preserves the existing Nib IR calling
convention without introducing target-specific stack frames.

## Virtual Register Mapping

For a function with `P` WASM parameters:

- WASM param indices: `0 .. P-1`
- IR working local for `vN`: WASM local index `P + N`

At function entry:

- param `0` is copied into `v2`
- param `1` is copied into `v3`
- ...

All IR working locals are declared as `i32`.

## Data and Memory Model

IR data declarations become one linear-memory image:

- each `IrDataDecl(label, size, init)` is assigned a byte offset
- offsets are dense and stable within the module
- memory minimum size is `ceil(total_bytes / 65536)`, with a minimum of one
  page whenever memory is needed
- each data declaration emits a WASM data segment at its assigned offset

Opcode lowering:

- `LOAD_ADDR dst, label` -> `i32.const offset(label)` -> `local.set dst`
- `LOAD_BYTE` -> `i32.load8_u`
- `STORE_BYTE` -> `i32.store8`
- `LOAD_WORD` -> `i32.load`
- `STORE_WORD` -> `i32.store`

If the module uses linear memory, it also exports `memory`.

## Supported IR Subset

The prototype backend supports the IR ops that the current Nib frontend emits,
plus the core load/store ops for future static-data access:

- `LOAD_IMM`
- `LOAD_ADDR`
- `LOAD_BYTE`
- `STORE_BYTE`
- `LOAD_WORD`
- `STORE_WORD`
- `ADD`
- `ADD_IMM`
- `SUB`
- `AND`
- `AND_IMM`
- `CMP_EQ`
- `CMP_NE`
- `CMP_LT`
- `CMP_GT`
- `LABEL`
- `JUMP`
- `BRANCH_Z`
- `BRANCH_NZ`
- `CALL`
- `RET`
- `HALT`
- `NOP`
- `COMMENT`

`SYSCALL` is out of scope for the first prototype.

## Control-Flow Lowering

WASM requires structured control flow, so arbitrary labels cannot be emitted
directly. The prototype therefore supports the reducible patterns produced by
`nib-ir-compiler`.

### If lowering

Nib IR shape:

```text
BRANCH_Z vC, if_K_else
  then...
JUMP if_K_end
LABEL if_K_else
  else...
LABEL if_K_end
```

WASM shape:

```text
local.get vC
if
  then...
else
  else...
end
```

### Loop lowering

Nib IR shape:

```text
LABEL loop_K_start
  head...
BRANCH_Z vCond, loop_K_end
  body...
JUMP loop_K_start
LABEL loop_K_end
```

WASM shape:

```text
block
  loop
    head...
    local.get vCond
    i32.eqz
    br_if 1
    body...
    br 0
  end
end
```

This is intentionally conservative. Generic irreducible CFG restructuring is a
future backend improvement.

## Export Policy

- `_start` is exported as `_start`
- `_fn_NAME` is exported as `NAME`
- memory, when present, is exported as `memory`

The module does **not** use the WASM start section in the prototype. Execution
is explicit via the exported entry points.

## Validation and Testing

Every package in the pipeline must ship with its own tests and maintain at
least 80% code coverage.

Required coverage targets:

- `wasm-module-encoder`: section encoding and parser round-trip coverage
- `ir-to-wasm-compiler`: arithmetic, calls, branches, loops, and memory ops
- `nib-wasm-compiler`: end-to-end source compilation and runtime execution

## Future Work

- richer signature metadata at the IR layer
- multi-result / void-precise WASM signatures instead of uniform `-> i32`
- name-section emission for better debugging
- generic CFG structuring beyond the current Nib-generated patterns
- WASI/syscall lowering
