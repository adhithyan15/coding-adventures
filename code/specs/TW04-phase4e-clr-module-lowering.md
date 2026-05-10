# TW04 Phase 4e — CLR Multi-Module Lowering

## Status: Shipped (2026-05-04)

## Context

Phase 4d shipped per-module JVM compilation: each Twig module compiles to its own
`.class` file; cross-module `invokestatic` stitches them together at the JVM level.

Phase 4e delivers the equivalent on CLR: every real Twig module (excluding the
synthetic `host` module) becomes a **TypeDef row inside a single PE/CLI assembly**.
Cross-module function calls lower to ordinary CIL `call` instructions addressed by the
callee's MethodDef token in the combined assembly.

## CLR multi-module architecture

### Why one assembly (not one assembly per module)?

CLR cross-assembly calls require `AssemblyRef` + `MemberRef` tokens, which the current
`cli-assembly-writer` does not support.  Putting all modules' TypeDef rows into a single
PE file lets every `call` instruction use a simple `MethodDef` token (`0x06xxxxxx`)
— the same token kind already used for same-type calls.

### TypeDef layout

The PE table rows follow this order:

| TypeDef row | Content |
|-------------|---------|
| 1 | `<Module>` pseudo-type (ECMA-335 mandatory) |
| 2 | Entry module's type (has the `.entrypoint` main method) |
| 3 | Dep module 1's type |
| 4 | Dep module 2's type |
| … | (further dep modules in topological order) |

Each module's type is a plain `class` that extends `System.Object`.  All methods on
dep-module types are `public static` — the same flags used for the entry-module's
callables.

### MethodDef layout

Methods are laid out in TypeDef order, then by their position in each module's IR:

```
MethodDef 1..M_entry   — entry module callables
MethodDef M_entry+1..  — dep module 1 callables
                       — dep module 2 callables (etc.)
```

For Phase 4e, multi-module programs must not contain closures or heap types
(`cons`/`symbol`/`nil`) inside **dependency** modules.  These features require
additional TypeDef rows (IClosure, Closure_N, Cons, Symbol, Nil) whose token counts
must be factored into the offset calculation; that extension is deferred to a later
phase.  Single-module programs (compiled via `compile_source`) continue to support
closures and heap types unchanged.

### Helper / MemberRef tokens

Helper methods (`__ca_syscall`, `__ca_mem_load_byte`, etc.) are emitted as `MemberRef`
rows (`0x0Axxxxxx`), not `MethodDef` rows.  All modules share the same five helper
specs, so the MemberRef tokens are identical across modules and no offset adjustment is
needed.

## IR calling convention

The Twig frontend emits `IrOp.CALL IrLabel("mod/fn")` for a cross-module call to
function `fn` in module `mod`.  The label string contains `/` to distinguish it from
local same-module calls (which never contain `/`).

In the CIL backend, a CALL instruction whose label contains `/`:

1. Looks up the pre-assigned token in `CILBackendConfig.external_method_tokens`
2. Marshals all call-register-count locals onto the operand stack (`ldloc 0..N`)
3. Emits `call <token>`
4. Stores the return value into local 1 (`stloc 1`)

## New `CILBackendConfig` fields (ir-to-cil-bytecode ≥ 0.18.0)

| Field | Type | Default | Purpose |
|-------|------|---------|---------|
| `extra_callable_labels` | `tuple[str, ...]` | `()` | Force-include exported dep-module functions even when no local `CALL` targets them (mirrors the JVM backend's same-name field) |
| `external_method_tokens` | `dict[str, int]` | `{}` | Pre-assigned `MethodDef` tokens for cross-module call targets |

## SequentialCILTokenProvider extension

A new `method_token_offset: int = 0` constructor parameter shifts the base MethodDef
token for dep-module providers:

```python
# Dep module 1 whose callables start at MethodDef row M_entry+1:
provider = SequentialCILTokenProvider(
    dep1_main_region_names,
    method_token_offset=M_entry,
)
```

## New twig-clr-compiler API

### `module_name_to_clr_type(name: str) → str`

Maps a Twig module name to a CLR type name by replacing `/` with `_`:

```
"user/hello" → "user_hello"
"a/math"     → "a_math"
```

### `compile_modules(modules, *, entry_module, assembly_name) → MultiModuleClrResult`

Two-pass compilation:

**Pass 1 (analysis)** — for each module in `[entry] + deps` order:
  - Compile Twig source to IR (`_Compiler`)
  - Optimize IR
  - Discover main-type callable names (entry_label + CALL targets + exports)

**Token assignment** — compute cumulative MethodDef row offsets per module.

**Pass 2 (emission)** — for each module:
  - Build `CILBackendConfig` with `extra_callable_labels` (module's exports),
    `external_method_tokens` (methods from all other modules), and
    `SequentialCILTokenProvider(method_token_offset=...)`
  - Compile to `CILProgramArtifact`

**Assembly merge**:
  - Entry module's `CILProgramArtifact.methods` → combined artifact's `methods`
  - Each dep module's callable methods → `CILTypeArtifact` appended to `extra_types`
  - Write one PE assembly from the combined artifact

### `run_modules(modules, *, entry_module, ...) → MultiModuleClrExecutionResult`

Calls `compile_modules`, writes the single `.exe` to a temp directory, runs
`dotnet <name>.exe`, and returns stdout / stderr / returncode.

## Acceptance criteria

End-to-end on real `dotnet`:

```twig
; Module a/math:
(module a/math (export add))
(define (add x y) (+ x y))

; Module user/hello (entry, imports a/math):
(module user/hello (import a/math))
(define n (a/math/add 10 32))
n   ; → exit code 42
```

```python
result = run_modules(modules, entry_module="user/hello")
assert result.returncode == 42
```

Host calls work in the entry module:

```twig
; Module user/io:
(module user/io (import host) (import a/math))
(host/write-byte (a/math/add 10 32))  ; writes byte 42
```

## Files changed

| File | Change |
|------|--------|
| `ir-to-cil-bytecode/backend.py` | Add `extra_callable_labels`, `external_method_tokens` to `CILBackendConfig`; add `method_token_offset` to `SequentialCILTokenProvider`; update `_discover_callable_regions` + CALL lowering |
| `ir-to-cil-bytecode/tests/test_cross_module_clr.py` | New: unit tests for cross-module CALL lowering |
| `twig-clr-compiler/compiler.py` | Extend `_compile_apply` for user-module cross calls; add `compile_modules`, `run_modules`, result types |
| `twig-clr-compiler/tests/test_multi_module_clr.py` | New: unit + real-dotnet end-to-end tests |
| `ir-to-cil-bytecode/CHANGELOG.md` | v0.18.0 entry |
| `twig-clr-compiler/CHANGELOG.md` | New version entry |
