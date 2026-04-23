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

This path supports simple Nib programs and nested function calls that only need
the current fixed-register ABI. Arguments are copied into virtual registers in
order, and the return value is reported from virtual `v1`/physical `x6`.

The remaining ABI gaps are local values that must survive across calls, recursion
depth beyond the hidden backend stack, and richer argument/return conventions.
