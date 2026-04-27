# IR Optimizer (Elixir)

Optimization passes for the `compiler_ir` package.

The package mirrors the Python, Rust, TypeScript, and Go IR optimizer surface with an Elixir-flavored API: pass modules expose `name/0` and `run/1`, and `CodingAdventures.IrOptimizer` threads an immutable `IrProgram` through the configured pipeline.

## Example

```elixir
alias CodingAdventures.CompilerIr.{IrImmediate, IrInstruction, IrProgram, IrRegister}
alias CodingAdventures.IrOptimizer

program =
  IrProgram.new("_start")
  |> IrProgram.add_instruction(%IrInstruction{
    opcode: :load_imm,
    operands: [%IrRegister{index: 1}, %IrImmediate{value: 5}],
    id: 1
  })
  |> IrProgram.add_instruction(%IrInstruction{
    opcode: :add_imm,
    operands: [%IrRegister{index: 1}, %IrRegister{index: 1}, %IrImmediate{value: 3}],
    id: 2
  })

result = IrOptimizer.optimize(program)
length(result.program.instructions)
# 1
```
