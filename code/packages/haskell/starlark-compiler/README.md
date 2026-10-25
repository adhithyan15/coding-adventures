# starlark-compiler

`starlark-compiler` is the pure Haskell provider for the standalone STP01
Starlark opcode profile. It exposes the 61 assigned byte values, exact neutral
names and categories, all 195 decode holes, and the four operator maps.

Despite the historical package identity, this component does not parse or
compile source code. `LoadModule`, `ImportFrom`, `Print`, and `Halt` are inert
data constructors. The package performs no execution, module loading, output,
filesystem access, process launch, environment lookup, or networking.

## Usage

```haskell
import qualified Data.Map.Strict as Map
import StarlarkCompiler

opFromByte 0x20              -- Just Add
opFromByte 0x2E              -- Nothing (an unassigned hole)
opByte Add                   -- 0x20
opName Add                   -- "Add"
opCategory Add               -- Arithmetic
binaryOpcode "//"            -- Just FloorDiv
Map.lookup "not in" compareOpMap -- Just CmpNotIn
```

The maps are immutable `Data.Map.Strict.Map` values. Inserting or deleting a
key returns a different map and cannot corrupt a retained value or a later
lookup.

## Validation

From this directory:

```sh
cabal test all
cabal check
```

The named `package starlark-compiler` stanza in `cabal.project` enables HPC
for this library and its test executable. Do not add the global
`--enable-coverage` flag: Cabal 3.10.3.0 applies a conflicting planning mode
to dependencies with internal libraries. Dependency bounds stay unchanged.

The native suite dynamically reads the shared
`code/specs/fixtures/starlark-opcode-provider-v1/cases.json` corpus. It visits
all 359 records through public APIs, verifies every byte and hole, proves map
ownership, and rejects bounded transport, shape, ordering, type, and semantic
corruption. Expected values come from the independent fixture rather than from
the provider implementation.

Recovery validation used GHC 9.4.8 and Cabal 3.10.3.0 on Windows: 13 examples
passed, including independent range, ordering, depth and map-ownership
witnesses. Production-only HPC measured 447/447 expressions and 61/61
alternatives; the 17/39 top-level declaration figure is a different metric.
`cabal check`, a `-Wall -Werror` build and the actual Unix BUILD command passed.
The Windows CI front still explicitly skips Haskell; that skip grants no
native qualification. The test reader's oversized-file IO rejection branch
is not covered by its in-memory oversized-transport witness.

See [`STP01-starlark-opcode-provider.md`](../../../specs/STP01-starlark-opcode-provider.md)
for the normative profile.
