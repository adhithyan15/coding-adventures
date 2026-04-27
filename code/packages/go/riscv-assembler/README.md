# riscv-assembler

Assembles a small, compiler-friendly RV32I assembly syntax into bytes for
`riscv-simulator`.

## Supported input

- RV32I integer instructions implemented by `riscv-simulator`
- CSR instructions: `csrrw`, `csrrs`, `csrrc`, `mret`
- Labels and `.text` / `.data` sections
- Data directives: `.byte`, `.word`, `.zero`, `.space`
- Pseudo-instructions used by compiler output: `li`, `la`, `mv`, `j`, `call`,
  `ret`, `nop`, `halt`

## Example

```go
result, err := riscvassembler.Assemble(`
_start:
  li a0, 42
  halt
`)
if err != nil {
    panic(err)
}

sim := riscvsimulator.NewRiscVSimulator(4096)
sim.Run(result.Bytes)
```
