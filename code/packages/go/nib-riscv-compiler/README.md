# nib-riscv-compiler

End-to-end Nib to RISC-V orchestration.

```text
Nib source
  -> nib-parser
  -> nib-type-checker
  -> nib-ir-compiler
  -> ir-optimizer
  -> ir-to-riscv-compiler
  -> riscv-assembler
  -> riscv-simulator
```

`CompileSource` returns the RISC-V assembly string and assembled bytes.
`RunSource` executes those bytes in `riscv-simulator` and reports the Nib return
value from physical register `x6`, the RISC-V register currently used for Nib's
`v1` return slot.

## Current scope

This first path is intended for simple Nib programs whose `main` does not make
nested function calls. The current starter calling convention uses `ra` directly,
so a function that calls another function needs stack-frame support before it can
return reliably.
