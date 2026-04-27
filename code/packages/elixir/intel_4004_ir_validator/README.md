# Intel 4004 IR Validator (Elixir)

Validates `compiler_ir` programs against the Intel 4004 backend constraints.

The validator reports all detected issues as rule-tagged errors so callers can show actionable diagnostics before lowering IR to assembly.

## Example

```elixir
alias CodingAdventures.CompilerIr.{IrImmediate, IrInstruction, IrProgram, IrRegister}
alias CodingAdventures.Intel4004IrValidator

program =
  IrProgram.new("_start")
  |> IrProgram.add_instruction(%IrInstruction{
    opcode: :load_imm,
    operands: [%IrRegister{index: 0}, %IrImmediate{value: 0}],
    id: 1
  })

Intel4004IrValidator.validate(program)
# []
```
