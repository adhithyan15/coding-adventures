# LANG27 - IIR to JVM, CLR, BEAM, and WASM host VM backends

## Overview

LANG25 covers the native AOT/JIT/debugger path.  LANG27 covers the host VM
path: take a language frontend that emits InterpreterIR and make it easy to
produce JVM, CLR, BEAM, WASM, and future VM artifacts without re-solving the
same backend problems in every frontend.

The Python ALGOL pipeline already found many of the hard details:

- activation frames must be explicit;
- lexical scoping needs static links;
- by-name parameters need storage pointers or eval/store thunk descriptors;
- arrays need descriptors, bounded heap allocation, row-major strides, and
  bounds checks;
- labels, switches, and formal procedures need small descriptors;
- nonlocal goto needs pending-transfer state and frame unwind;
- WASM cannot directly encode arbitrary goto and needs a dispatch-loop mode;
- host VM backends need explicit function signatures and target-specific
  packaging metadata.

Those discoveries should live in Rust, next to the LANG VM chain, not only in
the Python ALGOL compiler.

## Goal

Make this a normal frontend contract:

```rust
let module: IIRModule = frontend.compile(source)?;
let artifacts = lang_host_compile(module, HostVmTarget::Wasm)?;
let artifacts = lang_host_compile(module, HostVmTarget::Jvm)?;
let artifacts = lang_host_compile(module, HostVmTarget::Clr)?;
let artifacts = lang_host_compile(module, HostVmTarget::Beam)?;
```

A frontend should provide:

- an `IIRModule`;
- source spans/debug sidecar;
- type/refinement metadata where available;
- language binding metadata for builtins and stdlib names.

It should not need to know JVM constant-pool encoding, CLR metadata tokens,
BEAM atom/export tables, WASM dispatch-loop lowering, or ALGOL-style frame
descriptor layouts.

## First Rust Port

This PR stream starts the port with `host-vm-lowering`.

That crate owns the reusable facts discovered in Python ALGOL:

- ALGOL frame/runtime memory labels and fixed offsets.
- runtime state offsets for current frame, stack pointer, heap pointer, thunk
  state, pending goto label, and output byte accounting.
- array descriptor offsets, dimension entry layout, stride planning, total
  element limits, and allocation-byte planning.
- frame-slot planning for scalar and descriptor storage.
- hidden procedure parameters for static link and thunk heap mark.
- by-name thunk helper labels and signatures.
- label, switch, and procedure descriptor layouts.
- procedure-parameter dispatcher label generation, including names such as
  `_fn_algol_call_procedure_i32_result_procedure_i32_i32`.
- WASM structured-control recognition and dispatch-loop selection.
- target profiles for JVM, CLR, BEAM, WASM, and the pure LANG VM.

The important distinction is that the crate is not ALGOL-only in intent.  ALGOL
is the stress test that revealed the ABI shape.  Twig, Tetrad, BASIC, Ruby,
TypeScript, Lua, Perl, and future frontends can reuse the same host VM facts
when they need frames, descriptors, stdlib calls, or fallback semantics.

## Architecture

```text
frontend
  |
  v
IIRModule + debug sidecar + language binding
  |
  v
iir-host-lowering
  |
  +--> simple typed subset -> compiler-ir::IrProgram
  |       |
  |       +--> ir-to-jvm-class-file
  |       +--> ir-to-cil-bytecode
  |       +--> ir-to-beam
  |       +--> ir-to-wasm-compiler
  |
  +--> rich host IR for closures, frames, descriptors, and stdlib calls
          |
          +--> target backend trait
          +--> backend-specific artifact packagers
```

`compiler-ir::IrProgram` remains valuable for the current simple register and
memory subset.  It should not become a dumping ground for every higher-level
semantic detail.  The lowering stack needs a richer host lowering plan beside
it, so target backends can receive:

- function signatures;
- register and local layout;
- data/heap/static memory plan;
- runtime helper requirements;
- stdlib import table;
- source/debug mapping;
- target capability errors.

## Backend Trait

Add a shared host backend trait:

```rust
pub trait HostVmBackend {
    type Artifact;
    type Error;

    fn target(&self) -> HostVmTarget;
    fn validate(&self, unit: &HostLoweringUnit) -> Vec<Self::Error>;
    fn lower(&self, unit: &HostLoweringUnit) -> Result<Self::Artifact, Self::Error>;
    fn package(&self, artifact: Self::Artifact) -> Result<HostArtifact, Self::Error>;
}
```

The trait should wrap existing crates first:

- JVM: `ir-to-jvm-class-file`
- CLR: `ir-to-cil-bytecode` plus PE/CLI packager work
- BEAM: `ir-to-beam`
- WASM: `ir-to-wasm-compiler` and `wasm-module-encoder`

Later backends implement the same trait without changing frontends.

## Target Nitty Gritty

### JVM

The current Rust JVM backend already follows the right "boring bytecode"
strategy:

- one class or a small class set;
- static methods for callable regions;
- static register arrays and byte memory;
- verifier-friendly bytecode;
- constant-pool use for larger constants;
- ordinary `invokestatic` calls.

Missing host path work:

- expose the backend through `HostVmBackend`;
- thread source/debug attributes from the sidecar;
- carry closure/runtime-class metadata through the shared lowering unit;
- ensure compiler-ir parity with the Python opcode surface, especially f64 and
  bitwise operations.

### CLR

The CLR path needs to preserve these details:

- CIL method bodies are not enough; real execution needs PE/CLI packaging;
- call instructions require metadata tokens;
- helper calls are required for target-neutral memory operations;
- local signatures and max-stack must be explicit;
- debug symbols should use a portable PDB or a sidecar until PDB emission is
  implemented.

Missing host path work:

- expose `ir-to-cil-bytecode` through `HostVmBackend`;
- finish the PE/CLI packager for real dotnet execution;
- put helper metadata in the shared lowering unit instead of per-frontend code.

### BEAM

BEAM is not a linear-memory machine.  The host path must not pretend it is.

Current BEAM lowering accepts a limited compiler-ir subset.  It should continue
to reject unsupported memory-heavy programs with clear capability errors.

Missing host path work:

- map modules, exports, functions, and closure captures to BEAM-native shapes;
- use the shared lowering plan to decide when a program cannot target BEAM yet;
- expand the BEAM subset deliberately rather than smuggling byte-array semantics
  into the VM.

### WASM

WASM is the pressure point for ALGOL-style control flow.

Simple `if_N_else`, `if_N_end`, `loop_N_start`, and `loop_N_end` targets can use
structured lowering.  Arbitrary labels, switches, nonlocal gotos, recursive
switch selection, and designational expressions require dispatch-loop lowering.

Missing host path work:

- make dispatch-loop lowering an explicit target strategy;
- carry function signatures with WASM value types;
- thread WASI imports and scratch memory through the shared plan;
- emit name/source-map/DWARF sidecars from the debug metadata.

## Python ALGOL Details To Finish Porting

`host-vm-lowering` ports the ABI facts and planning rules.  The next Rust work
must port the executable lowering pieces:

- Rust ALGOL semantic checker or a shared semantic descriptor format.
- compiler-ir opcode parity with Python:
  - bitwise OR/XOR/NOT and immediate variants;
  - f64 load/store/arithmetic/comparison/conversions/math ops;
  - closure, cons/list, condition/restart, and exit opcodes if the host path
    still uses compiler-ir for those features.
- Rust emitter helpers for:
  - enter/leave frame;
  - runtime-state load/store;
  - heap allocation and zero/copy loops;
  - array descriptor materialization;
  - array index and dimension guards;
  - eval/store thunk dispatch;
  - label/switch/procedure descriptor dispatch;
  - nonlocal goto pending-transfer unwind;
  - deterministic runtime failure guards.
- conformance fixtures ported from Python ALGOL:
  - dynamic multidimensional bounds;
  - Jensen's device / by-name expressions;
  - by-name array element writeback;
  - formal procedure calls;
  - switch parameters and recursive switch selection;
  - nonlocal goto across blocks and procedures;
  - string output guard limits;
  - standard real math.

## Stdlib Integration

LANG26 `liblang-std` should be visible to this host path.

IIR `call_builtin` or `call_std` lowers to:

- direct host imports where the target has a standard primitive;
- runtime helper calls where the target needs an adapter;
- pure Rust stdlib calls for native/JIT/AOT paths;
- WASI or host shims for WASM;
- clear capability errors for targets that cannot provide the requested effect.

The frontend should only name the language-level builtin.  The host lowering
unit resolves it to a stable stdlib function id and target import/helper plan.

## Concurrency Integration

LANG28 concurrency is part of the host VM contract.  Host backends may use
target-native facilities internally, but they must preserve VM task semantics:

- JVM and CLR can use host thread pools as workers, not one host thread per VM
  task;
- BEAM can map VM tasks and channels to BEAM processes/mailboxes where the
  semantics match, and otherwise fall back to the shared runtime scheduler;
- WASM starts with a single-thread event-loop scheduler and can add worker/WASI
  thread support later;
- real OS thread and process APIs still flow through LANG26 `liblang-std`
  capability checks.

The shared lowering unit should carry concurrency helper requirements just like
stdlib calls, runtime descriptors, and debug metadata.

## Debug Path

The source map chain should flow as:

```text
source span
  -> AST node
  -> IIR instruction
  -> host lowering unit instruction/helper
  -> target bytecode offset
```

Target mappings:

- JVM: `SourceFile`, `LineNumberTable`, and sidecar metadata.
- CLR: portable PDB eventually, sidecar first.
- BEAM: line chunks and sidecar metadata.
- WASM: name custom section, source maps, or DWARF.

The debugger should not need target-specific source recovery logic in each
frontend.

## Definition Of Done

LANG27 is complete when:

- a frontend that emits typed or untyped IIR can request JVM, CLR, BEAM, or
  WASM through one Rust API;
- the API reports target capability errors before byte emission;
- all backends consume shared function signatures, helper requirements, stdlib
  mappings, and debug mappings;
- ALGOL-derived fixtures pass against every backend that claims the needed
  capabilities;
- JVM, CLR, BEAM, WASM, and pure LANG VM execution agree for the shared
  conformance suite;
- adding a future VM means implementing `HostVmBackend`, not touching every
  frontend.
