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
cabal test all --enable-coverage
cabal check
```

The native suite dynamically reads the shared
`code/specs/fixtures/starlark-opcode-provider-v1/cases.json` corpus. It visits
all 359 records through public APIs, verifies every byte and hole, proves map
ownership, and rejects bounded transport, shape, ordering, type, and semantic
corruption. Expected values come from the independent fixture rather than from
the provider implementation.

See [`STP01-starlark-opcode-provider.md`](../../../specs/STP01-starlark-opcode-provider.md)
for the normative profile.
