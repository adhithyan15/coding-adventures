defmodule CodingAdventures.RegisterVM.Types.VMError do
  @moduledoc """
  Represents a runtime error that occurred during VM execution.

  ## When does a VMError occur?

  A VMError is produced when the interpreter encounters an unrecoverable
  situation at a specific instruction:

  - Accessing a global variable that does not exist
  - Calling something that is not a function
  - Stack overflow (call depth exceeds limit)
  - A `Throw` instruction with an error value in the accumulator
  - Instruction pointer going out of bounds

  ## Fields

  - `message` — human-readable description of what went wrong. Suitable
    for display in error messages and stack traces.

  - `instruction_index` — the value of `ip` (instruction pointer) at the
    moment of the error. Combined with the CodeObject's name, this pinpoints
    the exact instruction that failed.

  - `opcode` — the integer opcode of the failing instruction, or -1 if the
    error happened outside instruction execution (e.g., ip out of bounds).
    Useful for debugging: "I was trying to execute LdaGlobal and it failed."
  """

  defstruct [
    :message,
    instruction_index: 0,
    opcode: 0
  ]
end
