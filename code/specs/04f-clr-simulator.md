# 04f - CLR Simulator

## Overview

The Common Language Runtime executes Common Intermediate Language (CIL) stored
inside CLI assemblies. As of April 16, 2026:

- the latest modern product release is .NET 10, originally released on
  November 11, 2025
- .NET Framework still spans the classic CLR line from 1.0 through 4.8.1
- Microsoft's standards page still points to ECMA-335 / ISO/IEC 23271:2012 as
  the CLI standard baseline

For this repo, the important design decision is:

- use `CLR` for the top-level simulator package
- use `CLI` and `CIL` for the lower reusable packages

That keeps the implementation aligned with the standard and makes the decoder
pipeline reusable beyond one simulator.

## Where it fits

```text
Source -> Lexer -> Parser -> CLR compiler -> PE/CLI/CIL pipeline -> CLR simulator
```

Like the JVM, the simulator is only the last stage. Most of the real work is in
parsing the container and metadata:

```text
PE bytes -> PE decoder -> CLI header decoder -> metadata stream decoders
         -> metadata table decoders -> signature decoder -> IL method decoder
         -> disassembler -> runtime model -> simulator
```

## Core research findings

### 1. Versioning is mostly a metadata/runtime-family problem, not an opcode problem

The CIL opcode space is comparatively stable. Versioning pressure shows up more
in:

- runtime family (`clr` vs `coreclr`)
- metadata version strings
- metadata tables and coded indices
- generic signatures and token resolution
- verification and runtime semantics
- PE/CLI packaging details

### 2. The real input unit is a managed PE/CLI assembly, not raw IL bytes

A CLR simulator should eventually accept real assemblies, not just naked IL.
The decoder stack must understand:

- DOS stub and PE signature
- COFF header and optional header
- CLI data directory / COR header
- metadata root
- metadata streams such as `#~`, `#Strings`, `#US`, `#GUID`, `#Blob`
- method bodies in tiny or fat format
- extra sections such as exception handling data

### 3. Exact product versions should map to reusable profiles

The public interface should accept exact targets such as:

- `netfx_1_0`
- `netfx_2_0`
- `netfx_4_8_1`
- `netcore_3_1`
- `net_8_0`
- `net_10_0`

Internally those resolve to a smaller set of `CliProfile` compatibility buckets.

## Exact version strategy

### Public target names

```python
simulate_assembly(data, target="netfx_4_8_1")
decode_cli_module(data, target="net_10_0")
```

### Internal profile model

```python
@dataclass(frozen=True)
class CliProfile:
    target: str
    runtime_family: Literal["clr", "coreclr"]
    ecma_baseline: str
    metadata_version_string: str
    table_schema_mode: str
    signature_feature_set: frozenset[str]
    il_semantics_mode: str
```

### Official runtime/version map

From Microsoft's runtime/version docs:

| Product line | Runtime family |
|--------------|----------------|
| .NET Framework 1.0 | CLR 1.0 |
| .NET Framework 1.1 | CLR 1.1 |
| .NET Framework 2.0 | CLR 2.0 |
| .NET Framework 3.0 | CLR 2.0 |
| .NET Framework 3.5 | CLR 2.0 |
| .NET Framework 4.0 through 4.8.1 | CLR 4 |
| .NET Core 1.0 through 3.1 | CoreCLR |
| .NET 5 through .NET 10 | CoreCLR |

Microsoft also states that .NET Core and .NET 5+ do not expose a separate CLR
version number the way .NET Framework did, so a simulator cannot key only on a
CLR number for modern releases.

### Compatibility buckets

Implementation can group exact targets like this:

- `clr_1x`: .NET Framework 1.0 and 1.1
- `clr_2x`: .NET Framework 2.0, 3.0, 3.5
- `clr_4x`: .NET Framework 4.0 through 4.8.1
- `coreclr_1_to_3`: .NET Core 1.0 through 3.1
- `net_5_plus`: .NET 5 through current

The public API stays exact even when the implementation shares logic.

## Proposed package structure

### Python-first package split

| Package | Responsibility | Primary output |
|---------|----------------|----------------|
| `cli-version-profiles` | Exact target lookup and compatibility buckets | `CliProfile` |
| `pe-container-decoder` | Parse DOS/PE/COFF container structure | `DecodedPEImage` |
| `cli-header-decoder` | Decode the CLI header and managed-data directories | `DecodedCliHeader` |
| `cli-metadata-streams-decoder` | Parse metadata root and stream headers | `DecodedMetadataRoot` |
| `cli-metadata-tables-decoder` | Decode `#~` / `#-` table rows and coded indices | typed metadata rows |
| `cli-signature-decoder` | Decode blob signatures for methods, fields, locals, generics | `DecodedSignature` |
| `cil-method-body-decoder` | Decode tiny/fat method bodies and EH sections | `DecodedMethodBody` |
| `cil-bytecode-disassembler` | Pretty-print IL instructions and resolved operands | `CilDisassembly` |
| `cil-bytecode-builder` | Build CIL method-body bytes from operations, metadata tokens, and labels | raw IL bytes |
| `ir-to-cil-bytecode` | Lower compiler IR into composable CIL method artifacts | `CILProgramArtifact` |
| `cli-assembly-writer` | Wrap CIL method artifacts in PE/CLI metadata and sections | managed PE bytes |
| `cli-runtime-model` | Tokens, stack types, frames, heap refs, value types | `CliRuntimeState` |
| `clr-vm-simulator` | Execute decoded IL method bodies | `ExecutionResult[ClrState]` |
| `brainfuck-clr-compiler` | Brainfuck frontend facade over the CLR pipeline | PE/CLI bytes + VM result |
| `nib-clr-compiler` | Nib frontend facade over the CLR pipeline | PE/CLI bytes + VM result |
| `clr-simulator` | Backward-compatible facade package | thin composition layer |

### Naming note

Lower layers should prefer `cli` and `cil` because those are the standardized
units. `clr-vm-simulator` is the execution package because users think in terms
of the CLR.

## Container and metadata boundaries

### `pe-container-decoder`

This package should only parse the PE shell:

- DOS stub
- PE signature
- COFF header
- optional header
- section table
- data-directory entries

It should not know how to decode CLI metadata streams.

### `cli-header-decoder`

Must decode the managed runtime header that points to:

- metadata
- resources
- strong name signature
- VTable fixups
- entry point token or RVA

### `cli-metadata-streams-decoder`

Should decode the metadata root and stream directory for streams such as:

- `#~` or `#-`
- `#Strings`
- `#US`
- `#GUID`
- `#Blob`

### `cli-metadata-tables-decoder`

Must handle table schema and coded-index interpretation for rows such as:

- `Module`
- `TypeRef`, `TypeDef`, `TypeSpec`
- `Field`
- `MethodDef`
- `Param`
- `MemberRef`
- `StandAloneSig`
- `CustomAttribute`
- `Assembly`, `AssemblyRef`
- `GenericParam`, `GenericParamConstraint`
- `MethodSpec`

### `cli-signature-decoder`

This should be its own package because signature blobs are reused everywhere:

- method signatures
- field signatures
- local variable signatures
- property signatures
- generic instantiations

## IL method bodies and disassembly

### `cil-method-body-decoder`

Must support:

- tiny method headers
- fat method headers
- max stack
- local signature token
- raw IL bytes
- extra sections
- small and fat exception-handling sections

### `cil-bytecode-disassembler`

Must support:

- single-byte opcodes
- `0xFE`-prefixed two-byte opcodes
- branch target reconstruction
- metadata-token operand labeling
- method/field/type token pretty-printing

### `cil-bytecode-builder`

The builder is the first reusable emitter-side package. It should stay below
IR lowering and PE/CLI assembly writing:

```text
IR backend -> ir-to-cil-bytecode -> CIL bytecode builder -> CLI assembly writer
```

It must support:

- compact integer constants: `ldc.i4.m1`, `ldc.i4.*`, `ldc.i4.s`, `ldc.i4`
- local and argument short forms plus prefixed wide forms
- metadata-token operands for `call`, `callvirt`, static fields, and arrays
- `0xFE`-prefixed comparison opcodes
- named labels with automatic short-to-long branch promotion
- deterministic errors for duplicate or unknown labels

### `ir-to-cil-bytecode`

This package lowers the repo's shared `compiler_ir` into CIL method-body
artifacts without taking ownership of CLI metadata, PE layout, or runtime helper
implementations. It should stay fully composable:

- collect callable IR regions from the entry label and `CALL` targets
- validate that branches remain inside their callable region
- assign static data offsets while enforcing data-size limits
- map virtual registers to CIL locals
- optionally pass a virtual-register window into internal calls for frontends
  such as Nib whose IR calling convention uses `v2+` argument registers
- emit method bodies through `cil-bytecode-builder`
- request helper calls for memory and syscall behavior through named helper specs
- resolve method/helper call operands through an injectable metadata-token provider

The default token provider may assign deterministic placeholder tokens for tests
and standalone bytecode inspection. A future CLI assembly writer must be able to
replace it with real `MethodDef` and `MemberRef` tokens.

### `cli-assembly-writer`

This package serializes CIL method artifacts into a managed PE/CLI container. It
should be the first assembly-writing layer, while still keeping runtime helpers
and high-level language frontends outside its ownership:

- emit a PE32 shell with section headers and the CLI data directory
- emit the CLI header with metadata RVA/size and entry point token
- write metadata root stream headers and `#~`, `#Strings`, `#Blob`, and `#US`
  streams
- write Module, TypeRef, TypeDef, MethodDef, MemberRef, StandAloneSig, and
  Assembly rows
- choose tiny method headers for small no-local methods and fat headers for
  methods with locals or larger bodies
- preserve method and helper token conventions from `ir-to-cil-bytecode`

It should not implement host runtime helpers itself. Helper references stay as
metadata references so later packages can provide real helper bodies, native
bridges, or simulator-host bindings.

## Runtime model

### `cli-runtime-model`

This package owns the shared execution-state vocabulary for later CLR VM
packages. It should stay independent of PE decoding and instruction execution
so decoders, assembly writers, VM interpreters, and host bridges can compose
against the same types:

- represent CLI type identities, method signatures, method descriptors, field
  descriptors, and decoded metadata tokens
- carry typed `CliValue` instances through argument slots, local slots, and the
  evaluation stack
- model managed heap references, object fields, boxed values, and unboxing
  checks
- preserve mutable frame/thread state while exposing immutable snapshots for
  traces and tests
- provide a token resolver protocol plus a map-backed resolver for tests and
  small execution adapters
- centralize `call` / `callvirt` argument popping, receiver checks, and
  `callvirt` null checks
- locate exception handlers over protected IL ranges

The simulator should model CLR concepts directly rather than flattening
everything into a generic stack machine:

- evaluation stack
- local variable slots
- arguments
- current method and method signature
- metadata token resolution
- reference types vs value types
- boxing and unboxing
- `call` vs `callvirt` behavior
- exception handler lookup

Suggested state split:

- `CliValue`
- `CliEvaluationStack`
- `ClrFrame`
- `ClrThreadState`
- `CliTokenResolver`
- `ClrHeapRef`

### `clr-vm-simulator`

This package executes decoded and disassembled IL against `cli-runtime-model`.
It is the first real VM package in the CLR pipeline and should stay focused on
execution, not PE parsing or assembly writing:

- build runtime frames from decoded `MethodDef` signatures and local-slot
  counts
- execute the compiler-backend opcode slice: int32 constants, arguments,
  locals, arithmetic, bitwise operations, comparisons, branches, `call`,
  `callvirt`, and `ret`
- invoke internal `MethodDef` targets and external `MemberRef` host bindings
- route `System.Console.WriteLine` and compiler helper calls through an
  injectable host bridge
- support the shared compiler-helper syscall slice used by the current
  frontends: write byte, read byte, and exit
- preserve per-instruction traces with runtime-model frame snapshots
- prove the end-to-end compiler path by running at least one frontend program
  through IR, CIL, PE/CLI writing, decoding, disassembly, and VM execution

### Frontend CLR facades

`brainfuck-clr-compiler` and `nib-clr-compiler` are thin orchestration
packages. They should not duplicate parser, type-checker, IR compiler,
optimizer, CIL, assembly writer, or simulator logic. Their job is to make the
frontend-to-CLR path explicit and testable:

- `compile_source` returns all intermediate artifacts plus managed PE bytes
- `pack_source` aliases compilation for parity with existing JVM/WASM facades
- `write_assembly_file` persists the generated `.dll`
- `run_source` executes the generated assembly through `clr-vm-simulator`

Coverage proof:

- Brainfuck: compile and execute output (`.`) and input/output (`,.`) programs
- Nib: compile and execute direct `main` returns and function calls with
  arguments, exercising the CIL register-window call mode

## CLR MVP execution slice

The first useful executable slice should start at:

- `netfx_2_0`

Reason:

- it includes generics, which are central to real-world CLI code
- it avoids some of the later CLR/CoreCLR product split
- it is a strong bridge profile: backporting to 1.x is manageable and most
  later 4.x code shares the same broad file and IL model

### MVP opcode families

- constants: `ldnull`, `ldc.i4.*`, `ldc.i8`, `ldc.r4`, `ldc.r8`, `ldstr`
- locals/args: `ldarg*`, `starg*`, `ldloc*`, `stloc*`, `ldloca*`
- stack ops: `pop`, `dup`
- arithmetic: `add`, `sub`, `mul`, `div`, `rem`, `neg`
- comparisons and branches: `ceq`, `cgt`, `clt`, `br*`, `beq`, `bne.un`,
  `switch`
- calls: `call`, `callvirt`, `ret`
- object model: `newobj`, `ldfld`, `stfld`, `ldsfld`, `stsfld`
- type ops: `box`, `unbox.any`, `castclass`, `isinst`
- exceptions: `throw`, `leave`, `endfinally`

### Defer for later

- full verifier emulation
- remoting-era behavior
- advanced generics corner cases
- constrained calls and tailcalls
- byref-heavy intrinsics
- async/iterator lowering patterns

## Cross-language portability plan

Like the JVM plan, lower-level metadata should live in normalized repo-owned
manifests rather than being rewritten by hand in every language port:

- `cli-opcodes.json`
- `cli-metadata-tables.json`
- `cli-coded-indices.json`
- `cli-signature-codes.json`
- `cli-profile-map.json`

Each language can then implement decoding and runtime semantics natively while
sharing one metadata source of truth.

## Implementation phases

1. `cli-version-profiles`
2. `pe-container-decoder`
3. `cli-header-decoder`
4. `cli-metadata-streams-decoder`
5. `cli-metadata-tables-decoder`
6. `cli-signature-decoder`
7. `cil-method-body-decoder`
8. `cil-bytecode-disassembler`
9. `cil-bytecode-builder`
10. `ir-to-cil-bytecode`
11. `cli-assembly-writer`
12. `cli-runtime-model`
13. `clr-vm-simulator`
14. Backward-compatible facade updates in `clr-simulator`

## References

- Microsoft CLR overview: https://learn.microsoft.com/en-us/dotnet/standard/clr
- .NET Framework versions and dependencies: https://learn.microsoft.com/en-us/dotnet/framework/install/versions-and-dependencies
- .NET release lifecycle: https://learn.microsoft.com/en-us/lifecycle/products/microsoft-net-and-net-core
- .NET 10 download page: https://dotnet.microsoft.com/en-us/download/dotnet/10.0
- Microsoft PE format reference: https://learn.microsoft.com/en-us/windows/win32/debug/pe-format
- Microsoft standards page for CLI / ECMA-335: https://learn.microsoft.com/en-us/dotnet/fundamentals/standards
- ECMA-335: https://www.ecma-international.org/publications-and-standards/standards/ecma-335/
