# brainfuck-riscv-compiler

End-to-end Brainfuck to RISC-V orchestration.

```text
Brainfuck source
  -> brainfuck parser
  -> brainfuck-ir-compiler
  -> ir-optimizer
  -> ir-to-riscv-compiler
  -> riscv-assembler
  -> riscv-simulator host syscalls
```

`CompileSource` returns the RISC-V assembly string and assembled bytes.
`RunSource` executes those bytes in `riscv-simulator`. Brainfuck `.` and `,`
use the simulator host syscall ABI: write byte (`a7=1`), read byte (`a7=2`),
and exit (`a7=10`).
