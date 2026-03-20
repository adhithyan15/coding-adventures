defmodule CodingAdventures.VirtualMachine.Types.VMTrace do
  @moduledoc """
  A snapshot of one step of VM execution.

  ## Tracing — the Key to Understanding

  When learning how a computer works, the most valuable thing is to SEE
  each step. A VMTrace captures a complete snapshot of what happened when
  the VM executed one instruction:

  - What was the program counter (pc) — which instruction were we at?
  - What instruction was executed?
  - What did the stack look like BEFORE the instruction?
  - What did the stack look like AFTER?
  - What are the current variable bindings?
  - Did the instruction produce any output?
  - A human-readable description of what happened.

  ## Example trace for ADD

      %VMTrace{
        pc: 2,
        instruction: %Instruction{opcode: 0x02, operand: nil},
        stack_before: [3, 4],
        stack_after: [7],
        variables: %{},
        output: nil,
        description: "Execute 0x02"
      }

  Reading this, you can see: "At PC=2, the VM executed ADD. The stack had
  [3, 4] on it. After ADD, the stack has [7]. No output was produced."
  """

  alias CodingAdventures.VirtualMachine.Types.Instruction

  @type t :: %__MODULE__{
          pc: non_neg_integer(),
          instruction: Instruction.t(),
          stack_before: [any()],
          stack_after: [any()],
          variables: map(),
          output: String.t() | nil,
          description: String.t()
        }

  defstruct [:pc, :instruction, :stack_before, :stack_after, :variables, :output, :description]
end
