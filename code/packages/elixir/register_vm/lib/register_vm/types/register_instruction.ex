defmodule CodingAdventures.RegisterVM.Types.RegisterInstruction do
  @moduledoc """
  A RegisterInstruction is one decoded operation in the instruction stream.

  ## Operand Encoding

  Unlike a stack-based VM where operands are implicit (pop from stack),
  a register-based VM makes operands explicit in the instruction itself.
  Each instruction carries a list of integer operands whose meaning depends
  on the opcode:

  | Opcode category      | Typical operands                                |
  |----------------------|-------------------------------------------------|
  | Load constant        | `[constant_pool_index]`                         |
  | Register move        | `[register_index]` or `[src_reg, dst_reg]`      |
  | Arithmetic           | `[rhs_register_index, feedback_slot_index]`     |
  | Jump                 | `[byte_offset]`  (signed, relative to next ip)  |
  | Variable access      | `[name_pool_index, feedback_slot_index]`        |
  | Call                 | `[func_reg, first_arg_reg, argc, feedback_slot]`|
  | Property access      | `[object_reg, name_pool_index, feedback_slot]`  |

  ## Why a feedback_slot field?

  The `feedback_slot` field is the index into the current frame's
  `feedback_vector` where this instruction records its type observations.
  Having it as a first-class field (instead of always the last operand)
  makes the interpreter code cleaner and avoids confusion with operands
  that happen to appear last.

  ## Examples

      # LdaConstant 7  — load constants[7] into acc
      %RegisterInstruction{opcode: 0x00, operands: [7]}

      # Star r2  — store acc into registers[2]
      %RegisterInstruction{opcode: 0x11, operands: [2]}

      # Add r1, slot=3  — acc = acc + registers[1], record in feedback[3]
      %RegisterInstruction{opcode: 0x30, operands: [1, 3]}

      # JumpIfFalse +5  — if acc is falsy, skip forward 5 instructions
      %RegisterInstruction{opcode: 0x52, operands: [5]}
  """

  defstruct [
    :opcode,
    operands: [],
    feedback_slot: nil
  ]
end
