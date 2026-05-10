# TW04 Phase 4f — BEAM Multi-Module Lowering

## Status: Shipped (2026-05-04)

## Context

Phase 4d shipped per-module JVM compilation: each Twig module compiles to its own
`.class` file; cross-module `invokestatic` stitches them together at the JVM level.

Phase 4e delivered the CLR equivalent: every real Twig module becomes a TypeDef row
inside a single PE/CLI assembly.

Phase 4f delivers the BEAM equivalent: every real Twig module (excluding the synthetic
`host` module) compiles to its own **`.beam` file**.  All `.beam` files are loaded in
the same `erl` session so cross-module `call_ext` instructions resolve at runtime.

## BEAM multi-module architecture

### Why one `.beam` file per module (not one file for all)?

Unlike CLR — where cross-assembly calls require `AssemblyRef` + `MemberRef` tokens that
the `cli-assembly-writer` did not support — BEAM's `call_ext` opcode natively addresses
any `{Module, Function, Arity}` triple that is visible in the code path at runtime.
There is no "import table" that must be pre-computed across all modules; the runtime
resolves the MFA at the first call.  This makes one-file-per-module the natural mapping.

### Module naming

Twig module names use `/` as a namespace separator (e.g. `"a/math"`).  BEAM atom names
must be valid Erlang atoms; `/` is illegal in an unquoted atom.  The mapping rule is:

```
"a/math"     → "a_math"
"user/hello" → "user_hello"
```

`module_name_to_beam_module(name)` performs this substitution.

### `.beam` file layout

Each module produces one standard BEAM file:

- **Module atom** — derived from the Twig module name via `module_name_to_beam_module`.
- **Export table** — `main/0` for the entry module; each declared export for dep modules;
  plus the mandatory `module_info/0` and `module_info/1` stubs.
- **Import table** — one entry per unique cross-module call target encountered in the IR,
  encoded as `{CallerAtom, FnAtom, Arity}`.
- **Atom table** — module atom, all interned function-name atoms, all symbol-literal atoms.
- **Code section** — BEAM binary instructions (see `ir-to-beam`).

### Cross-module call semantics

`IrOp.CALL IrLabel("a/math/add")` labels containing `/` are cross-module calls.  The
label string encodes both the foreign module name and the function name:

```
"a/math/add"  →  module "a_math", function "add"
```

Unlike JVM/CLR — where all parameters are passed through a fixed-width call-register
frame regardless of callee arity — BEAM's `call_ext N Mfa` encodes the callee's **exact
arity** in both the instruction prefix (`N`) and the MFA triple.  A mismatch causes a
loader crash or undefined-function error at runtime.

Arity tracking therefore requires two sources:

1. **Call-site arity** — recorded in `_Compiler._cross_module_arities` at each
   cross-module `IrOp.CALL` site (the number of arguments the caller passes).
2. **Declared arity** — obtained by re-running `_Compiler` on each compiled dep module
   and reading `_fn_params`; this gives the exact declared parameter count for each
   exported function.

`compile_modules` merges both sources into `BEAMBackendConfig.external_function_arities`
before passing the config to `lower_ir_to_beam`.

## GC-safe y-register initialisation

### The problem

BEAM's garbage collector traces **all** allocated y-register slots as potential heap
pointers.  Our compiler allocates a flat frame of `max_reg_index + 1` y-slots per
function via `allocate K, N` but only writes a subset of them (param slots and holding
registers) before body code runs.  Unwritten slots contain stack-word garbage.  If any
garbage value resembles a valid BEAM tagged heap pointer, the GC follows it and corrupts
the heap — causing non-deterministic SIGSEGV at recursion depth ≥ 4, when the first GC
cycle fires inside a deeply nested call stack.

### Approaches that failed

| Approach | Why it failed |
|----------|--------------|
| `allocate_zero` (abstract opcode 14) | Abstract `.S` opcode 14 ≠ binary opcode 14. Binary opcode 14 = `allocate_heap` (3 operands). Loader returns `{error, badfile}`. |
| `init y(i)` (opcode 17) | OTP 28 rejects `init` with Y-register operands: "please re-compile with an OTP 28 compiler". |
| `init_yregs` (opcode 172) | The correct OTP 24+ replacement, but requires Z-tagged extended-list compact-term encoding that our encoder does not support. |

### The fix

Emit `move {atom, 0}, y(i)` for every y-register slot immediately after `allocate`,
before copying function arguments and executing body code.

Atom index 0 is the nil atom (`[]`) by BEAM convention — a tagged **immediate** that is
NOT a heap pointer.  The GC can safely trace a nil slot without following a pointer chain.

This uses only standard `move` opcodes already handled by our encoder and is valid across
all supported OTP versions.

```erlang
%% Generated prologue for a function with 3 y-register slots:
allocate 3, arity
move {atom,0}, y(0)          %% nil-init slot 0
move {atom,0}, y(1)          %% nil-init slot 1
move {atom,0}, y(2)          %% nil-init slot 2
move x(0), y(2)              %% copy first arg into param slot
move x(1), y(3)              %% copy second arg into param slot
...                          %% body code
```

## Explicit module pre-loading in `-noshell` mode

`erl -pa <dir>` adds the directory to the code path but does NOT guarantee that all
`.beam` files are auto-loaded before a `call_ext` fires.  In `-noshell` mode, the lazy
code-loader may not have loaded a dep module by the time the entry module first calls
into it, producing:

```
{undef,[{a_math,add,2,[]}]}
```

or (with GC corruption) an outright SIGSEGV.

`run_modules` therefore constructs its `-eval` expression to explicitly load every module
before invoking `main/0`:

```erlang
{module,_} = code:load_file(a_math),
{module,_} = code:load_file(b_util),
Result = user_hello:main(),
erlang:halt(Result).
```

Modules are loaded in compilation order (deps before entry) so that inter-dep calls also
resolve before the first cross-module dispatch.

## New `BEAMBackendConfig` fields (ir-to-beam ≥ 0.5.0)

| Field | Type | Default | Purpose |
|-------|------|---------|---------|
| `extra_callable_labels` | `tuple[str, ...]` | `()` | Force-include exported dep-module functions as callable regions even when no local `CALL` targets them |
| `call_register_count` | `int \| None` | `None` | API symmetry with CLR/JVM backends; ignored for BEAM (per-declaration arities, not a fixed-width frame) |
| `external_function_arities` | `dict[str, int]` | `{}` | Maps cross-module call labels (e.g. `"a/math/add"`) to the exact remote arity required for `call_ext N Mfa` |

## New twig-beam-compiler API

### `module_name_to_beam_module(name: str) → str`

Maps a Twig module name to a BEAM module atom string by replacing `/` with `_`:

```
"user/hello" → "user_hello"
"a/math"     → "a_math"
```

### `compile_modules(modules, entry_module, *, optimize=True) → MultiModuleBeamResult`

Compiles a topologically ordered list of `ResolvedModule`s to BEAM.  Skips the synthetic
`host` module.

**Pass 1 (analysis)** — for each module in `[entry] + deps` order:
  - Compile Twig source to IR (`_Compiler`)
  - Optimize IR
  - Record `_cross_module_arities` and `_fn_params` for arity resolution

**Pass 2 (emission)** — for each module:
  - Build `BEAMBackendConfig` with `extra_callable_labels` (module's exports),
    `external_function_arities` (arities from both call-site and declared sources),
    and the module's BEAM atom name
  - Lower IR to `BEAMModule`
  - Encode to `.beam` bytes

Returns a `MultiModuleBeamResult` containing one `ModuleBeamCompileResult` per module.

### `run_modules(modules, entry_module, *, optimize=True, tmp_dir=None, timeout_seconds=30) → MultiModuleBeamExecutionResult`

Calls `compile_modules`, writes each `.beam` file to a temp directory, and invokes:

```
erl -noshell -pa <dir> -eval '<load_stmts> Result = <entry>:main(), erlang:halt(Result).'
```

where `<load_stmts>` are explicit `code:load_file` calls for every compiled module in
compilation order.  Returns a `MultiModuleBeamExecutionResult` with the compile result
plus stdout / stderr / returncode.

Skips when `erl` is not on PATH (test decorator `@requires_beam`).

## Result types

All three result types are frozen dataclasses.

### `ModuleBeamCompileResult`

Per-module artefact produced by `compile_modules`.

| Field | Type | Description |
|-------|------|-------------|
| `twig_module_name` | `str` | Original Twig name (e.g. `"a/math"`) |
| `beam_module` | `str` | BEAM atom string (e.g. `"a_math"`) |
| `ir` | `IrProgram` | Optimized IR for this module |
| `beam_bytes` | `bytes` | Encoded `.beam` file content |
| `exports` | `tuple[str, ...]` | Exported Twig function names |

### `MultiModuleBeamResult`

Aggregate result from `compile_modules`.

| Field | Type | Description |
|-------|------|-------------|
| `entry_module` | `str` | Twig name of the entry module |
| `entry_beam_module` | `str` | BEAM atom of the entry module |
| `module_results` | `tuple[ModuleBeamCompileResult, ...]` | One per compiled module |

### `MultiModuleBeamExecutionResult`

Result from `run_modules`.

| Field | Type | Description |
|-------|------|-------------|
| `compilation` | `MultiModuleBeamResult` | Full compile result |
| `stdout` | `str` | Captured stdout from `erl` |
| `stderr` | `str` | Captured stderr from `erl` |
| `returncode` | `int` | Process exit code (= `erlang:halt/1` argument) |

## Acceptance criteria

End-to-end on real `erl`:

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

Recursive dep-module function (exercises GC-safe y-register initialisation):

```twig
; Module a/math:
(module a/math (export fact))
(define (fact n)
  (if (= n 0) 1 (* n (fact (- n 1)))))

; Entry module calls (a/math/fact 5) → 120
```

## Files changed

| File | Change |
|------|--------|
| `ir-to-beam/backend.py` | Add `extra_callable_labels`, `call_register_count`, `external_function_arities` to `BEAMBackendConfig`; cross-module CALL lowering in `_emit_call`; GC-safe y-register nil-initialisation after `allocate` |
| `twig-beam-compiler/compiler.py` | Add `module_name_to_beam_module`, `compile_modules`, `run_modules`, result types; cross-module arity tracking in `_cross_module_arities`; explicit `code:load_file` pre-loading in `run_modules` |
| `twig-beam-compiler/tests/test_multi_module_beam.py` | New: 38 unit + real-erl end-to-end tests |
| `ir-to-beam/CHANGELOG.md` | v0.5.0 entry |
| `twig-beam-compiler/CHANGELOG.md` | v0.5.0 entry |
