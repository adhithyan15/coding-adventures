# coding-adventures-intel-4004-ir-validator

Validates a generic `IrProgram` against Intel 4004 hardware constraints before
assembly generation.

## Pipeline position

```text
Nib source
    -> parser + type checker + IR compiler
IrProgram
    -> intel-4004-ir-validator
Validated IrProgram
    -> ir-to-intel-4004-compiler
Intel 4004 assembly
```
