# cil-bytecode-builder

Composable Common Intermediate Language bytecode builder for method bodies.

This package is the first emitter-side piece of the Python CLR pipeline. It
does not write PE files or CLI metadata. It focuses on the smaller reusable
unit that every CLR backend needs: turning readable CIL operations into raw IL
method-body bytes.

## Where It Fits

```text
compiler-ir
  -> ir-to-clr-assembly
  -> cil-bytecode-builder
  -> cli-assembly-file
  -> clr-runtime / dotnet
```

## Usage

```python
from cil_bytecode_builder import CILBytecodeBuilder

builder = CILBytecodeBuilder()
builder.emit_ldc_i4(1)
builder.emit_ldc_i4(2)
builder.emit_add()
builder.emit_stloc(0)
builder.emit_ret()

assert builder.assemble() == bytes([0x17, 0x18, 0x58, 0x0A, 0x2A])
```

Labels are resolved in a second pass. Branches start in their short form and
automatically promote to long form when the target is out of signed-byte range.
