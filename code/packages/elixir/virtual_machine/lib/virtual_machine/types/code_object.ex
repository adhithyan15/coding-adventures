defmodule CodingAdventures.VirtualMachine.Types.CodeObject do
  @moduledoc """
  A compiled program — instructions plus their associated data.

  ## What is a CodeObject?

  When a compiler or assembler translates source code into bytecode, it
  produces a CodeObject. This bundles three things:

  1. **instructions**: the list of `Instruction` structs to execute, in order.
  2. **constants**: a pool of literal values referenced by instructions.
     For example, `LOAD_CONST 0` means "push constants[0] onto the stack."
  3. **names**: a pool of variable/function names. `LOAD_NAME 2` means
     "look up names[2] in the current scope."

  ## Why separate pools?

  Instructions are compact — just an opcode and an index. The actual values
  (which could be large strings, floats, or nested structures) live in the
  constants pool. This is exactly how CPython, the JVM, and the CLR work.

  ## Example

      alias CodingAdventures.VirtualMachine.Types.Instruction

      # A program that computes 3 + 4 and prints the result:
      %CodeObject{
        instructions: [
          %Instruction{opcode: 0x01, operand: 0},   # LOAD_CONST 0 -> push 3
          %Instruction{opcode: 0x01, operand: 1},   # LOAD_CONST 1 -> push 4
          %Instruction{opcode: 0x02, operand: nil},  # ADD -> pop 3 and 4, push 7
          %Instruction{opcode: 0x03, operand: nil},  # PRINT -> pop 7, output "7"
          %Instruction{opcode: 0xFF, operand: nil}   # HALT
        ],
        constants: [3, 4],
        names: []
      }
  """

  alias CodingAdventures.VirtualMachine.Types.Instruction

  @type t :: %__MODULE__{
          instructions: [Instruction.t()],
          constants: [any()],
          names: [String.t()]
        }

  defstruct instructions: [], constants: [], names: []
end
