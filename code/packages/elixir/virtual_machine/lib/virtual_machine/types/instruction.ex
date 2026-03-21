defmodule CodingAdventures.VirtualMachine.Types.Instruction do
  @moduledoc """
  A single bytecode instruction.

  ## Anatomy of an Instruction

  Every instruction has two parts:

      ┌──────────┬──────────┐
      │  opcode  │ operand  │
      │  (what)  │ (with)   │
      └──────────┴──────────┘

  The **opcode** is an integer that identifies the operation. For example:
  - 0x01 might mean LOAD_CONST (push a constant onto the stack)
  - 0x02 might mean ADD (pop two values, push their sum)
  - 0xFF might mean HALT (stop execution)

  The **operand** is optional extra data. LOAD_CONST needs an operand
  (which constant to load), but ADD does not (it always pops two values).

  ## Examples

      # Push constant at index 0 onto the stack
      %Instruction{opcode: 0x01, operand: 0}

      # Add the top two stack values
      %Instruction{opcode: 0x02, operand: nil}

      # Halt the VM
      %Instruction{opcode: 0xFF, operand: nil}
  """

  @type t :: %__MODULE__{
          opcode: integer(),
          operand: any() | nil
        }

  defstruct [:opcode, :operand]
end
