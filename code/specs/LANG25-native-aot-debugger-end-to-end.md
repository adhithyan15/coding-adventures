# LANG25 - native AOT, JIT, and debugger end-to-end completion plan

## Overview

LANG22 defines the intended compilation model: every language lowers to
InterpreterIR, can run in the LANG VM, can tier into JIT, can compile AOT with
or without PGO, and can fall back to the runtime for dynamic operations.  The
current implementation has many of the right crates, but the end-to-end path is
not complete yet.

This spec names the missing work required to make the LANG VM chain usable
across:

- macOS arm64
- macOS x86_64
- Linux x86_64
- Linux arm64
- Windows x86_64

The immediate goal is not AOT-only closed-world compilation.  Dynamic languages
must continue to link the runtime.  The goal is:

```text
Twig / Tetrad / Brainfuck / Dartmouth BASIC / Ruby / TypeScript / ...
        |
        v
InterpreterIR + debug sidecar + language binding
        |
        +--> LANG VM interpreter
        +--> JIT native code
        +--> AOT native executable
        +--> AOT with PGO
        +--> WASM / JVM / CLR through existing codegen paths
```

## Current state

### Working pieces

- `interpreter-ir` is the common IR.
- `vm-core`, `twig-vm`, `lang-runtime-core`, and `vm-runtime` provide the
  interpreter/runtime foundation.
- `jit-core` can specialise IIR to CIR and drive pluggable backends.
- `aot-core` can infer, specialise, link function blobs, and write `.aot`
  snapshots with an optional IIR table.
- `aarch64-encoder` and `aarch64-backend` can lower a small integer/control-flow
  subset to ARM64 bytes.
- `code-packager` can emit several container formats, including Mach-O, ELF,
  PE, raw, Intel HEX, and WASM.
- `ldp-format` implements the profile artefact format.
- `debug-sidecar`, `native-debug-info`, `dap-adapter-core`, `twig-dap`, and the
  generated VS Code extension exist.

### Known breakages

- `twig-aot` does not compile on Windows because the CLI calls a macOS/unix-only
  function and its test imports `std::os::unix` unconditionally.
- `jit-loader-macos` doctests fail on non-macOS because the crate-level cfg hides
  `CodePage` while the doc example still imports it.
- `dap-adapter-core` launch does not establish `vm_conn`, so real DAP requests
  such as `stackTrace` fail with `no VM`.
- The VS Code extension has a `vitest run` test script but no tests.
- `twig-aot` manually compiles each function with `AArch64Backend`; it does not
  use the full `AOTCore` runtime-fallback snapshot path.
- `aarch64-backend` does not lower `call`, `call_runtime`, closures, properties,
  heap operations, or runtime dispatch.  `type_assert` traps instead of deopts.
- `jit-core` and `aot-core` do not yet produce/consume `.ldp` profile artefacts.
- `native-debug-info` is not wired into `twig-aot` or `aot-core` output.
- Several critical crates lack build-tool metadata, so CI can miss regressions.

## Non-goals

- Pure closed-world AOT for dynamic languages.
- AOT-only execution without `liblang-runtime` for untyped programs.
- Cross-binary PGO.
- Shipping every language frontend in this PR stream.  The first target is the
  native pipeline; languages can then plug into it.

## Target model

The current API names macOS arm64 directly, for example
`compile_file_macos_arm64`.  That should become target driven:

```rust
pub struct NativeTarget {
    pub os: TargetOs,
    pub arch: TargetArch,
    pub object_format: ObjectFormat,
    pub abi: Abi,
    pub link_strategy: LinkStrategy,
}

pub enum TargetOs { Macos, Linux, Windows }
pub enum TargetArch { Aarch64, X86_64 }
pub enum ObjectFormat { MachO, Elf, Coff }
pub enum Abi { DarwinAarch64, SysVAmd64, Win64, LinuxAarch64 }
pub enum LinkStrategy { SystemLinker, Lld, SnapshotOnly }
```

The CLI shape should be:

```bash
twig-aot program.twig -o program
twig-aot program.twig --target macos-arm64 -o program
twig-aot program.twig --target linux-x86_64 -o program
twig-aot program.twig --target windows-x86_64 -o program.exe
```

Default target is the host triple.  Unsupported targets must fail at argument
validation with a clear message, not at Rust compile time.

## Backend work

### ARM64 backend completion

The ARM64 backend is currently an integer/control-flow MVP.  It still needs:

- `call` lowering for intra-module calls.
- `call_runtime` lowering through the `liblang-runtime` C ABI.
- argument and return marshalling for boxed LANG values.
- relocation records for calls whose final address is not known during function
  lowering.
- stack maps or conservative root descriptors for runtime calls.
- deopt stubs for PGO/JIT guards.
- float operations.
- string operations through runtime calls.
- heap allocation, property get/set, closures, and dynamic dispatch through
  `LangBinding`.

### x86_64 backend MVP

Add:

- `x86_64-encoder`
- `x86_64-backend`

The first x86_64 backend should match the ARM64 MVP feature set:

- function prologue/epilogue
- integer constants
- integer add/sub/mul/div
- integer comparisons
- labels and conditional/unconditional jumps
- return values
- up to six integer/pointer args for SysV and four for Win64
- stack alignment for each ABI
- backend trait implementation with `compile_function`

The next step after parity is runtime-call support:

- SysV call ABI for Linux/macOS x86_64
- Win64 call ABI for Windows
- shadow space on Win64
- callee/caller-saved register preservation
- relocation sites for external runtime symbols

## Packaging and linking

`code-packager` can emit executable-ish containers today, but the native path
needs relocatable object output and system linker integration per platform.

Required object packagers:

- Mach-O relocatable object for macOS arm64 and macOS x86_64.
- ELF relocatable object for Linux arm64 and Linux x86_64.
- COFF relocatable object for Windows x86_64.

Required link drivers:

- macOS: Apple `ld` or `clang`, because macOS executable provenance and dyld
  setup are best delegated to Apple tooling.
- Linux: `cc`, `ld`, or `lld`, selectable by config.
- Windows: `link.exe` or `lld-link`, selectable by config.

Each link driver must support:

- native text object
- `liblang-runtime`
- `liblang-std` for shared language-visible standard library calls
- optional debug information
- optional profile metadata section
- predictable temporary-file cleanup
- clear diagnostics containing the linker command and stderr

## Runtime fallback

AOT-no-profile must be conservative.  If a function or instruction cannot be
fully lowered, the compiler should not reject the whole program.  It should
emit a runtime call or place the function in the IIR table and let
`liblang-runtime` interpret it.

Missing pieces:

- Build a real linkable `liblang-runtime` for each supported OS/arch.
- Define a stable C ABI between native code and runtime:
  - `lang_call_function`
  - `lang_call_builtin`
  - `lang_send_message`
  - `lang_get_property`
  - `lang_set_property`
  - `lang_alloc`
  - `lang_deopt`
  - `lang_raise`
- Replace the `aot-core` JSON-only IIR table bridge with the binary IIRT table
  already modeled by `vm-runtime`, or deliberately converge both formats.
- Teach `AOTCore` to emit relocation records for runtime calls.
- Teach `twig-aot` to call `AOTCore` rather than bypassing it with a
  per-function all-or-nothing compile loop.
- Add runtime-level selection:
  - none for fully native typed programs
  - minimal for arithmetic/control fallback
  - standard for builtins and I/O
  - full for GC/profiler/JIT interop

## PGO path

`ldp-format` exists, but the producer and consumer are missing.

Required producer work:

- `jit-core` writes `.ldp` on shutdown or explicit flush.
- The VM profiler records function call counts, instruction observations,
  type states, promotion state, and deopt events.
- The emitted profile includes language, module identity, source hash, and IIR
  version so stale/cross-language profiles are rejected.

Required consumer work:

- `aot-core` reads `.ldp`.
- AOT validates language/module/source compatibility.
- Profile observations promote `type_hint` for hot stable sites.
- AOT-with-PGO emits type guards and deopt anchors.
- Guard failure materializes a VM frame and resumes in the interpreter.

Acceptance tests:

- Run a Twig program under JIT, flush `.ldp`, then AOT-compile with that
  `.ldp`.
- Verify the AOT-PGO binary returns the same result as interpreter execution.
- Verify a deliberately stale profile is rejected.
- Verify a guard mismatch deopts instead of trapping.

## Deopt and stack maps

JIT and AOT-with-PGO need the same deopt protocol.

Missing pieces:

- Native frame descriptors for each compiled function.
- Register/stack slot maps at each deopt anchor.
- Materialization of boxed values from unboxed native registers.
- Inlined-frame representation.
- Runtime entry point that rebuilds an interpreter frame from native state.
- Tests for guard mismatch, nested calls, and live heap references.

AOT-no-profile can ship before this, because it must not speculate.
AOT-with-PGO and high-quality JIT require it.

## Debugger completion

The debugger needs to work in three execution modes:

- interpreter mode
- JIT mode
- AOT binary with runtime/debug server enabled

Immediate fixes:

- `dap-adapter-core` launch must allocate/select a debug port, launch the VM,
  connect with retry, and store `vm_conn`.
- `twig-dap` must emit variable declarations/live ranges into the debug sidecar.
- The current e2e test must fail if no breakpoint is hit.
- Add a stdio DAP e2e test that drives the real adapter process:
  - initialize
  - launch
  - setBreakpoints
  - configurationDone
  - stopped event
  - stackTrace
  - scopes
  - variables
  - continue
  - terminated
- Add VS Code extension tests or change the generated script so empty tests do
  not make CI misleading.

AOT debug mode:

- Native binaries should accept a debug flag or environment variable that starts
  the same VM debug protocol server when runtime support is linked.
- Fully native frames should be represented through native debug info plus the
  sidecar.
- Runtime fallback frames should be represented through the existing VM debug
  protocol.

## Native debug info

`native-debug-info` is currently a library, not part of the AOT output path.

Missing pieces:

- Convert debug sidecar line tables to DWARF line tables for ELF/Mach-O.
- Convert debug sidecar line tables to CodeView for PE/COFF.
- Emit function names, source files, line tables, and local variable ranges.
- Embed or link debug sections during native packaging.
- Keep DAP sidecar IDs and native debug IDs stable enough to correlate frames.

## Language frontend requirements

Every language frontend that wants the "free" VM/JIT/AOT path must implement:

- parse source
- lower to `IIRModule`
- preserve source spans
- emit debug sidecar
- expose `LangBinding` for runtime builtins/dynamic semantics
- provide conformance tests against interpreter, JIT, AOT, and AOT-PGO

Language-specific notes:

- Tetrad should be the Tier A typed golden path.
- Twig should be the Tier C dynamic golden path.
- Brainfuck should lower to IIR and use the same AOT/JIT path instead of
  maintaining separate JVM/CLR/WASM-only compiler logic.
- Dartmouth BASIC should lower to IIR first, then reuse the shared backend
  matrix.
- Ruby, TypeScript, Lua, and Perl ports should start by sharing the LANG VM
  package skeleton and route dynamic semantics through `LangBinding`.

## Build and CI requirements

Build metadata must cover the critical native chain:

- `aarch64-encoder`
- `aarch64-backend`
- `x86_64-encoder`
- `x86_64-backend`
- `aot-core`
- `vm-runtime`
- `lang-runtime-core`
- `ldp-format`
- `jit-core`
- `jit-loader-macos`
- `code-packager`
- `native-debug-info`
- `debug-sidecar`
- `dap-adapter-core`
- `twig-dap`
- `twig-vm`
- `twig-aot`

Main builds should use the sharded build-plan work so the native matrix can
scale without one runner building the whole world.  PR builds can stay affected
package based.

Required CI lanes:

- Windows x86_64: compile/test non-macOS crates, PE/COFF packaging tests,
  CodeView tests, DAP stdio e2e.
- Linux x86_64: ELF packaging tests, x86_64 backend tests, AOT runtime fallback.
- macOS arm64: Mach-O link/run, `jit-loader-macos`, DAP e2e, AOT e2e.
- macOS x86_64 if available: Mach-O x86_64 packaging/backend tests.

Mac provisioning is expensive, so macOS e2e should be limited to the native
packages and languages that need it.  The build plan should avoid scheduling
macOS just because unrelated packages changed.

## PR plan

### 25A - platform hygiene and build coverage

- Fix `twig-aot` cfg so non-macOS hosts compile cleanly.
- Fix `jit-loader-macos` doctest cfg.
- Add missing BUILD/BUILD_windows metadata for debugger and AOT crates.
- Add generated VS Code extension test coverage or correct the script.

### 25B - DAP real launch e2e

- Fix `dap-adapter-core` launch to connect to the VM.
- Make the breakpoint e2e strict.
- Add stdio DAP e2e coverage.
- Emit Twig variables into the sidecar.

### 25C - target model and AOT CLI

- Introduce `NativeTarget`.
- Replace macOS-arm64-only APIs with target-driven APIs.
- Add host default target detection.
- Keep macOS arm64 as the first runnable target.

### 25D - x86_64 encoder/backend MVP

- Add `x86_64-encoder`.
- Add `x86_64-backend`.
- Reach parity with the ARM64 integer/control-flow subset.
- Add SysV and Win64 ABI tests.

### 25E - relocatable object and linker drivers

- Add or complete Mach-O, ELF, and COFF relocatable object packagers.
- Add platform link drivers.
- Make `twig-aot --target` produce runnable host executables where supported.

### 25F - runtime fallback

- Produce linkable `liblang-runtime` artefacts.
- Link the `liblang-std` profile required by the program's standard library imports.
- Lower `call_runtime` and dynamic operations.
- Route `twig-aot` through `AOTCore`.
- Demonstrate an untyped Twig program that runs as AOT plus runtime fallback.

### 25G - PGO producer/consumer

- Make JIT write `.ldp`.
- Make AOT read `.ldp`.
- Add stale-profile rejection and guard/deopt tests.

### 25H - native debug info integration

- Emit DWARF/CodeView from sidecar data.
- Package debug sections into native outputs.
- Correlate native frames with DAP sidecar frames.

### 25I - language conformance matrix

- Add shared tests that every LANG frontend can run:
  - interpret
  - JIT
  - AOT no profile
  - AOT with PGO
  - debugger breakpoint/stack/variables
- Start with Tetrad and Twig, then add Brainfuck and Dartmouth BASIC.

## Definition of done

The native chain is complete when these commands have equivalent behavior:

```bash
twig run examples/fib.twig
twig jit examples/fib.twig
twig-aot examples/fib.twig -o fib
twig-aot examples/fib.twig --profile fib.ldp -o fib-pgo
```

And equivalent target-specific commands work where the host/toolchain supports
them:

```bash
twig-aot examples/fib.twig --target macos-arm64
twig-aot examples/fib.twig --target macos-x86_64
twig-aot examples/fib.twig --target linux-x86_64
twig-aot examples/fib.twig --target linux-arm64
twig-aot examples/fib.twig --target windows-x86_64
```

The debugger is complete when the same source-level breakpoint test passes in:

- interpreted Twig
- JIT-enabled Twig
- AOT Twig with runtime debug mode

For each mode, DAP must return a stack trace, scopes, variables, continue, and
termination without `no VM`, missing variables, or tolerated breakpoint misses.
