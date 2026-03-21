# Assembler (Go Port)

**Layer 5 of the computing stack** — translates assembly mnemonics into binary machine code.

## What is an assembler?

An assembler reads human-readable assembly language (`ADD R0, R1, R2`) and converts each instruction into the binary encoding that a CPU can execute. It is the bridge between the compiler's output (assembly) and the CPU simulator's input (machine code).

This is a shell package — implementation forthcoming. The assembler will target the ARM instruction set used by the ARM simulator package.

## Usage

```go
import "github.com/adhithyan15/coding-adventures/code/packages/go/assembler"

// Implementation forthcoming.
```

## Spec

See [06-assembler.md](../../../specs/06-assembler.md) for the full specification.
