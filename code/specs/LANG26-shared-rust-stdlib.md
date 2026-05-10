# LANG26 - shared Rust standard library for LANG VM languages

## Overview

Every language implemented on top of the LANG VM needs a standard library:
numbers, strings, bytes, lists, maps, sets, I/O, time, diagnostics, and common
compiler utilities.  Without a shared layer, each language runtime will
reimplement the same builtins and each JIT/AOT backend will need a different
answer for `print`, `string-length`, `map-get`, or `read-file`.

LANG15 defines `liblang-runtime`: execution mechanism for VM fallback,
dispatch, IIR tables, and runtime entry points.

LANG26 defines `liblang-std`: a Rust standard library that is visible to
languages and callable from interpreter, JIT, and AOT code.

```text
language frontend
    |
    v
IIR call_builtin / call_std
    |
    +--> interpreter calls Rust stdlib shim
    +--> JIT emits direct or indirect stdlib call
    +--> AOT links liblang_std_<target>.a
```

The goal is one implementation of common behavior, shared by Twig, Tetrad,
Brainfuck, Dartmouth BASIC, Ruby, TypeScript, Lua, Perl, and future languages.

## Goals

- Provide a Rust implementation of common language-visible standard functions.
- Expose the same functions to interpreter, JIT, and AOT paths.
- Keep stable function IDs and signatures so compiled code can link by index,
  not by string lookup.
- Attach type, refinement, effect, and capability metadata to every standard
  function.
- Let each language choose its surface names while sharing the same underlying
  implementation.
- Make pure stdlib functions inlineable by JIT/AOT backends.
- Make effectful stdlib functions capability-gated and deterministic in tests.

## Non-goals

- Replacing per-language semantics.  Ruby method lookup, JavaScript coercions,
  and Twig list syntax still belong to each language binding.
- Replacing `liblang-runtime`.  This spec is about library services, not VM
  dispatch or deopt machinery.
- Exposing raw host OS APIs directly to user code.
- Implementing a package manager.
- Making every stdlib function available on every target.  Embedded targets can
  link smaller profiles.

## Layering

```text
lang-stdlib-protocol
    stable IDs, signatures, effects, capability metadata

lang-stdlib-core
    pure Rust implementations: math, strings, bytes, collections, diagnostics

lang-stdlib-host
    effectful adapters: stdio, filesystem, env, time, random, process

lang-stdlib-ffi
    C ABI symbols and static/dynamic library packaging

<lang>-runtime
    maps language surface names to stdlib function IDs
```

`lang-runtime-core` remains the owner of `LangBinding`, value representation,
GC hooks, deopt, ICs, and frame materialization.  `lang-stdlib-*` is a consumer
of that substrate.

## Crates

### `lang-stdlib-protocol`

Pure data crate.  No OS access.

Owns:

- `StdFnId`
- `StdModuleId`
- `StdSignature`
- `StdType`
- `StdRefinement`
- `StdEffect`
- `StdCapability`
- manifest reader/writer

Example:

```rust
pub struct StdSignature {
    pub id: StdFnId,
    pub module: &'static str,
    pub name: &'static str,
    pub params: &'static [StdParam],
    pub return_type: StdType,
    pub effects: &'static [StdEffect],
    pub capabilities: &'static [StdCapability],
    pub purity: Purity,
}
```

### `lang-stdlib-core`

Pure and deterministic functions.

Initial modules:

- `core/bool`
- `core/order`
- `core/option`
- `core/result`
- `num/int`
- `num/float`
- `bytes`
- `string`
- `list`
- `vector`
- `map`
- `set`
- `hash`
- `diagnostic`
- `source_span`

This crate should avoid host I/O.  It must be usable in tests, WASM, and
eventually embedded targets.

### `lang-stdlib-host`

Effectful host functions.

Initial modules:

- `io/stdin`
- `io/stdout`
- `io/stderr`
- `fs`
- `env`
- `time`
- `random`
- `process`

Every function in this crate has explicit capability metadata.  Tests can
provide a fake host adapter.

### `lang-stdlib-ffi`

C ABI and target packaging.

Builds:

- `liblang_std_macos_arm64.a`
- `liblang_std_macos_x86_64.a`
- `liblang_std_linux_arm64.a`
- `liblang_std_linux_x86_64.a`
- `lang_std_windows_x86_64.lib`

Also supports dynamic libraries where useful for development:

- `liblang_std.dylib`
- `liblang_std.so`
- `lang_std.dll`

## Value ABI

Stdlib calls need a common ABI that all language runtimes can use.  The ABI must
not force every language to share the same high-level object representation.

Use `lang-runtime-core` values at the boundary:

```c
typedef struct lang_std_value {
    uint8_t tag;
    uint8_t flags;
    uint16_t reserved;
    uint32_t type_id;
    uint64_t payload;
} lang_std_value_t;

typedef struct lang_std_slice {
    const lang_std_value_t *ptr;
    uint32_t len;
} lang_std_slice_t;

typedef struct lang_std_result {
    uint8_t ok;
    uint8_t reserved[7];
    lang_std_value_t value;
    uint32_t error_code;
} lang_std_result_t;
```

For language-specific objects, the value carries an opaque heap reference and a
type id.  The active `LangBinding` remains responsible for materializing,
tracing, comparing, and finalizing that object.

## Call ABI

Two call shapes are required.

### Generic dispatch

Used by interpreter fallback and early AOT/JIT:

```c
lang_std_result_t lang_std_call(
    uint32_t fn_id,
    const lang_std_value_t *args,
    uint32_t argc,
    void *runtime_context);
```

This is simple and stable.  It is not the final performance path for hot code.

### Direct symbols

Used by JIT/AOT for known monomorphic signatures:

```c
uint64_t lang_std_num_i64_add_checked(int64_t a, int64_t b, uint8_t *trap);
uint64_t lang_std_string_len(lang_std_value_t s);
lang_std_result_t lang_std_io_write_byte(uint8_t b, void *runtime_context);
```

The manifest maps each `StdFnId` to optional direct symbols.  Backends can
choose between direct calls, generic dispatch, or inlining.

## IIR integration

The initial implementation can continue to use:

```text
call_builtin "std/string/len" value
call_builtin "std/io/write-byte" byte
```

The long-term IIR should add an indexed form:

```text
call_std fn_id args...
```

Lowering rule:

- frontend names resolve to `StdFnId` during type checking or module linking;
- interpreter uses `lang_std_call`;
- JIT either inlines, emits direct symbol calls, or emits `lang_std_call`;
- AOT records relocations against direct symbols or the generic dispatcher.

String lookup should not occur inside hot runtime loops.

## Manifest

The standard library manifest is generated from Rust declarations and checked
into the build artifacts:

```json
{
  "version": 1,
  "functions": [
    {
      "id": 1001,
      "module": "std/io",
      "name": "write-byte",
      "params": [{ "name": "b", "type": "(Int 0 256)" }],
      "return": "Int",
      "purity": "effectful",
      "effects": ["stdout"],
      "capabilities": ["io.stdout.write"],
      "generic_symbol": "lang_std_call",
      "direct_symbol": "lang_std_io_write_byte"
    }
  ]
}
```

The manifest feeds:

- frontend import resolution;
- type checking;
- refinement checking;
- JIT/AOT lowering;
- documentation generation;
- capability auditing.

## Type and refinement metadata

Every stdlib function must have a signature.  Public functions should avoid
`any` unless polymorphism is the point.

Examples:

```text
std/io/write-byte : (b : (Int 0 256)) -> Int
std/bytes/get     : (buf : Bytes, i : (Index (bytes-len buf))) -> Byte
std/string/slice  : (s : String, start : (Index (string-len s)),
                     end : (Int start (string-len s))) -> String
std/list/head     : (xs : (NonEmptyList T)) -> T
std/map/get       : (m : Map<K,V>, k : K) -> Option<V>
```

These signatures become proof obligations for `lang-refinement-checker`.

If a caller passes an unconstrained integer to `write-byte`, strict mode rejects
it unless a guard proves `0 <= b < 256`.  Lenient mode emits a runtime check.

## Effects and capabilities

Each function declares effects:

- `pure`
- `alloc`
- `read_stdout`
- `write_stdout`
- `read_stdin`
- `read_fs`
- `write_fs`
- `read_env`
- `time`
- `random`
- `process`
- `network`

Capabilities are finer grained and host-policy facing:

- `io.stdout.write`
- `io.stdin.read`
- `fs.read`
- `fs.write`
- `env.read`
- `time.now`
- `random.secure`
- `process.exit`

The runtime context passed to `lang_std_call` carries the active capability
grant.  JIT/AOT code must not bypass the capability check for effectful
functions.  Pure functions may be inlined freely.

## Profiles

Different targets should link different stdlib profiles:

| Profile | Contents | Typical target |
|---------|----------|----------------|
| `core` | pure functions only | embedded, pure computation |
| `cli` | core + stdio + env + process exit | command-line tools |
| `files` | cli + filesystem | compilers, formatters |
| `full` | all stable modules | desktop/server |
| `test` | full API backed by deterministic fakes | CI and fixtures |

AOT should pick the smallest profile that satisfies the program's imports.
JIT/interpreter can load the full host profile during development.

## Language surface mapping

Each language owns its surface syntax and naming.  The shared stdlib only
provides canonical functions and signatures.

Examples:

| Language | Surface | Shared stdlib target |
|----------|---------|----------------------|
| Twig | `(host/write-byte b)` | `std/io/write-byte` |
| Twig | `(string-length s)` | `std/string/len` |
| Brainfuck | `.` | `std/io/write-byte` |
| Brainfuck | `,` | `std/io/read-byte` |
| Dartmouth BASIC | `PRINT` | `std/io/write-string` / `std/io/write-byte` |
| Ruby | `String#length` | `std/string/len` where semantics match |
| TypeScript | `console.log` shim | `std/io/write-string` |
| Tetrad | typed `print-u8` | `std/io/write-byte` |

If a language has different semantics, it can wrap the shared primitive in its
own runtime.  The wrapper still uses the shared implementation where possible.

## JIT behavior

The JIT lowering policy:

1. Resolve `call_std fn_id`.
2. If the function is pure and has a backend intrinsic, inline it.
3. Else if a direct symbol exists for the observed monomorphic signature, emit a
   direct call.
4. Else emit `lang_std_call(fn_id, args, argc, cx)`.
5. Record profile feedback for argument shapes, result type, trap count, and
   effect usage.

The JIT must never inline effectful operations across capability checks,
safepoints, or observable ordering boundaries.

## AOT behavior

AOT lowering policy:

1. Resolve all stdlib calls to `StdFnId` during link planning.
2. Compute the required stdlib profile.
3. Link `liblang_std_<target>` for that profile.
4. Emit relocations for direct symbols or `lang_std_call`.
5. Include the stdlib manifest version in the AOT metadata.
6. Fail if the target does not support a required effectful function.

For AOT-with-PGO, hot pure stdlib calls can be inlined if the target backend
supports the intrinsic and the function's manifest entry is marked stable.

## Interpreter behavior

`vm-core` and `vm-runtime` should route standard builtins through the same
registry:

```rust
StdRegistry::resolve("std/io/write-byte") -> StdFnId
StdRegistry::call(fn_id, args, cx) -> Result<Value, RuntimeError>
```

Per-language `LangBinding::resolve_builtin` can delegate to this registry for
standard names, then fall back to language-specific builtins.

## Error model

Stdlib errors use a stable error code table:

```text
0       ok
1xxx    type/refinement violation
2xxx    allocation failure
3xxx    capability denied
4xxx    I/O failure
5xxx    unsupported target/profile
6xxx    invalid argument count/signature
```

Language runtimes map these to their own error surfaces:

- Twig diagnostic or runtime trap
- Ruby exception
- TypeScript exception-like object
- BASIC runtime error code
- Brainfuck EOF behavior where specified

The shared stdlib should preserve enough structured detail for good diagnostics
without forcing one language's exception model onto another.

## Determinism and testing

`lang-stdlib-host` must support a deterministic host adapter:

- fixed stdin bytes;
- captured stdout/stderr buffers;
- in-memory filesystem;
- deterministic clock;
- seeded random source;
- denied-by-default process/network operations.

This lets every language run the same stdlib conformance tests without touching
the real host system.

## Security

Effectful stdlib functions are capability-gated.  Compilers and backends must
not lower them to raw syscalls unless the capability check is preserved.

Rules:

- Pure functions can be inlined.
- Allocation functions must preserve GC safepoints and write barriers.
- I/O and filesystem functions must call through the host adapter.
- Process and network functions are optional and denied by default.
- AOT binaries embed the capability manifest they were linked with.

## Initial API set

### Core

- `std/core/id`
- `std/core/equal`
- `std/core/compare`
- `std/core/hash`

### Integers

- `std/num/i64/add-checked`
- `std/num/i64/sub-checked`
- `std/num/i64/mul-checked`
- `std/num/i64/div-checked`
- `std/num/i64/mod`
- `std/num/i64/abs`
- `std/num/i64/min`
- `std/num/i64/max`

### Bytes and strings

- `std/bytes/len`
- `std/bytes/get`
- `std/bytes/set`
- `std/bytes/slice`
- `std/string/len`
- `std/string/concat`
- `std/string/slice`
- `std/string/from-bytes-utf8`
- `std/string/to-bytes-utf8`

### Collections

- `std/list/empty`
- `std/list/cons`
- `std/list/head`
- `std/list/tail`
- `std/list/is-empty`
- `std/vector/new`
- `std/vector/len`
- `std/vector/get`
- `std/vector/set`
- `std/map/new`
- `std/map/get`
- `std/map/set`
- `std/set/new`
- `std/set/contains`

### Host

- `std/io/write-byte`
- `std/io/write-string`
- `std/io/read-byte`
- `std/env/get`
- `std/time/now-ms`
- `std/process/exit`

### Compiler support

- `std/source-span/new`
- `std/source-span/merge`
- `std/diagnostic/error`
- `std/diagnostic/warning`
- `std/diagnostic/render`

The compiler support module exists because the self-hosted Twig compiler will
need these utilities immediately.

## Build and packaging

Add build metadata for:

- `lang-stdlib-protocol`
- `lang-stdlib-core`
- `lang-stdlib-host`
- `lang-stdlib-ffi`

CI must build and test:

- Rust unit tests for pure functions;
- host adapter deterministic tests;
- manifest generation stability tests;
- C ABI smoke tests;
- interpreter call tests;
- JIT lowering tests for pure intrinsics;
- AOT link/run tests per supported platform.

## PR plan

### 26A - protocol and manifest

- Add `lang-stdlib-protocol`.
- Define IDs, signatures, effects, capabilities, and manifest format.
- Generate a manifest for a tiny initial API.

### 26B - pure core

- Add `lang-stdlib-core`.
- Implement integer, bool, bytes, string, list basics.
- Add refined signatures for bounds-sensitive functions.

### 26C - interpreter registry

- Add `StdRegistry`.
- Let `LangBinding::resolve_builtin` delegate standard names.
- Route Twig and Brainfuck I/O builtins through the registry.

### 26D - host adapter

- Add `lang-stdlib-host`.
- Implement stdio/env/time/process-exit through capability-checked adapters.
- Add deterministic test adapter.

### 26E - FFI and AOT link

- Add `lang-stdlib-ffi`.
- Export `lang_std_call` and a small set of direct symbols.
- Link `liblang-std` into `twig-aot` for stdlib calls.

### 26F - JIT intrinsics

- Teach `jit-core` and native backends to inline selected pure stdlib calls.
- Keep effectful calls behind capability-preserving runtime calls.

### 26G - language conformance

- Add shared stdlib conformance tests.
- Run them through Twig, Brainfuck, Dartmouth BASIC, and Tetrad first.
- Extend to Ruby, TypeScript, Lua, and Perl as those frontends land.

## Definition of done

LANG26 is complete when:

- a standard function has one Rust implementation and is callable from the
  interpreter, JIT, and AOT paths;
- AOT binaries link only the stdlib profile they need;
- JIT can inline pure stdlib intrinsics and call effectful stdlib functions
  safely;
- every stdlib function has type/refinement/effect metadata;
- Twig, Brainfuck, Dartmouth BASIC, and Tetrad can all use the same stdlib I/O
  and core collection functions;
- the self-hosted Twig compiler can depend on shared source-span and diagnostic
  utilities instead of private ad hoc helpers.
