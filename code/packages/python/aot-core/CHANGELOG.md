# Changelog — coding-adventures-aot-core

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.1.0] — 2026-04-22

### Added

**Static type inference (`aot_core.infer`)**
- `_literal_type(value)` — maps Python literals to LANG types (`bool`, `u8`–`u64`, `f64`, `str`, `any`).
- `_promote(a, b)` — numeric rank promotion; `any` dominates; non-numeric combinations resolve to `any`; `str + str → any` (handled separately in `_infer_instr`).
- `_resolve(src, env)` — resolves a source operand (literal or register name) to its inferred type.
- `infer_types(fn)` — flow-insensitive single-pass inference over an `IIRFunction`; seeds the type environment from parameter declarations, then infers each instruction's destination type from its sources and opcode semantics.  Supports: `const`, arithmetic (`add`, `sub`, `mul`, `div`, `mod`), bitwise (`and`, `or`, `xor`, `shl`, `shr`), comparison (`cmp_lt`, `cmp_le`, `cmp_gt`, `cmp_ge`, `cmp_eq`, `cmp_ne`), and unary (`neg`, `not`) families.  Unknown opcodes produce `"any"`.

**AOT specialization (`aot_core.specialise`)**
- `_spec_type(instr, inferred)` — selects the concrete type for an instruction: explicit `type_hint` wins, else consults the inferred-type dict, else falls back to `"any"`.
- `aot_specialise(fn, inferred?)` — lowers an `IIRFunction` to a flat `list[CIRInstr]` using the inferred-type map produced by `infer_types`.  Mirrors the structure of `jit_core.specialise` but driven by static inference rather than profiler observations.  Handles:
  - `const` → `const_<type>` (picks type from literal when hint is absent)
  - `ret` → `ret_<type>`; `ret_void` is passed through unchanged
  - Arithmetic/bitwise/comparison: typed variant (e.g. `add_u8`, `cmp_lt_bool`) for known types, `call_runtime("generic_*")` for `"any"`; `str + str` emits `call_runtime("str_concat")`
  - Unary: typed variant or `call_runtime("generic_*")` fallback
  - `type_assert` guard emission when inferred type is known but source register is `"any"`
  - Passthrough opcodes: `jmp`, `jmp_if_false`, `label`, `call`, `store_mem`, and any unrecognised opcode

**`.aot` snapshot binary format (`aot_core.snapshot`)**
- 26-byte header: `magic="AOT\x00"` (4 B), `version=0x0100` (2 B), `flags` (4 B), `entry_point_offset` (4 B), `vm_iir_table_offset` (4 B), `vm_iir_table_size` (4 B), `native_code_size` (4 B).
- `FLAG_VM_RUNTIME = 0x01` — set when the snapshot contains an IIR table for the VM interpreter path.
- `write(native_code, iir_table?, entry_point_offset?)` — serialises to bytes.
- `read(data)` — deserialises; raises `ValueError` on truncated input, bad magic, or out-of-bounds sections.
- `AOTSnapshot` dataclass with `has_vm_runtime` property.

**Binary linker (`aot_core.link`)**
- `link(fn_binaries)` — concatenates per-function native-code blobs, returning `(combined_code, byte_offset_map)`.
- `entry_point_offset(offsets, entry="main")` — looks up `"main"` (or a caller-supplied name) in the offset map; returns 0 if not found.

**VM runtime container (`aot_core.vm_runtime`)**
- `VmRuntime(library_bytes?)` — wraps a pre-compiled interpreter library blob.
- `serialise_iir_table(fns)` — encodes a list of `IIRFunction` objects as a JSON byte string; records `name`, `params`, `return_type`, `instructions` (with `op`, `dest`, `srcs`, `type_hint`, `observed_type`, `observation_count`, `deopt_anchor`), and `type_status`.
- `deserialise_iir_table(data)` — inverse of the above.

**AOT compilation controller (`aot_core.core`)**
- `AOTCore(backend, optimization_level=1, vm_runtime?)` — orchestrates the full ahead-of-time pipeline.
- `compile(module)` — for each function: `infer_types` → `aot_specialise` → optional constant-folding/DCE → `backend.compile`.  Functions for which the backend returns `None` (or raises) are routed to the IIR table for VM-fallback execution.  Writes a `.aot` snapshot, appending `vm_runtime.library_bytes` when an IIR table is present.
- `compile_to_file(module, path)` — convenience wrapper that calls `compile` and writes the resulting bytes to disk.
- `stats()` — returns an `AOTStats` snapshot capturing `functions_compiled`, `functions_untyped`, `total_binary_size`, `compilation_time_ns`, and `optimization_level`; accumulates across multiple `compile` calls and returns an independent copy each time.
- `_compile_fn(fn)` — per-function pipeline step; returns `None` on any exception so the caller can fall back to the IIR table.
- `_is_fully_typed(fn, inferred)` — returns `True` iff every instruction destination in the function is resolved to a concrete (non-`"any"`) type.
- Optimization levels: 0 = no optimizations; 1 = constant folding + dead-code elimination (via `ir_optimizer`); 2 = same as 1 (reserved for future AOT-specific passes).

**Compilation statistics (`aot_core.stats`)**
- `AOTStats` dataclass: `functions_compiled`, `functions_untyped`, `compilation_time_ns`, `total_binary_size`, `optimization_level`.

**Package surface (`aot_core.__init__`)**
- Public exports: `AOTCore`, `AOTStats`, `AOTSnapshot`, `VmRuntime`, `infer_types`, `aot_specialise`.

### Tests

- 164 unit tests across 7 test modules; **100% line coverage**.
- `tests/conftest.py` — shared helpers: `make_instr`, `make_fn`, `make_mod`, `make_cir`, `MockAOTBackend`.
- `tests/test_infer.py` — 48 tests covering `_literal_type`, `_promote`, `_resolve`, and `infer_types` (const, arithmetic, promotion, comparison, unary, multi-instruction chains, typed hints).
- `tests/test_specialise.py` — 37 tests covering `_spec_type`, const translation, `ret` translation, binary/unary ops (typed, inferred, generic, guard emission), passthrough ops, and fallback.
- `tests/test_snapshot.py` — 20 tests covering write/read round-trips, header format, error handling, and `AOTSnapshot` properties.
- `tests/test_link.py` — tests covering `link` and `entry_point_offset` for empty, single, and multiple function binaries.
- `tests/test_vm_runtime.py` — tests covering `VmRuntime` construction, IIR table serialisation/deserialisation for functions with params, instructions, type status, and deopt anchors.
- `tests/test_core.py` — integration tests for the full compile pipeline, untyped-function IIR routing, backend-failure fallback, optimization levels 0/1/2, `compile_to_file`, stats accumulation, and `_is_fully_typed`.
