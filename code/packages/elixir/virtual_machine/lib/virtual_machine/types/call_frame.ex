defmodule CodingAdventures.VirtualMachine.Types.CallFrame do
  @moduledoc """
  A saved execution context for function calls.

  ## The Call Stack

  When a program calls a function, the VM needs to remember where to
  come back to when the function finishes. It also needs to save the
  caller's local variables so they are not clobbered by the function's
  own variables.

  A CallFrame captures all of this:

  - **return_address**: the program counter value to restore when the
    function returns (i.e., the instruction after the CALL).
  - **saved_variables**: the caller's variable bindings, so they can
    be restored on return.
  - **saved_locals**: the caller's local variable list.

  Each function call pushes a CallFrame onto the call stack. Each
  return pops one off and restores the saved state.

  ## Analogy

  Think of a CallFrame like a bookmark with a sticky note. The bookmark
  (return_address) marks where you were reading. The sticky note
  (saved_variables) records what you were thinking about at that point.
  When you finish the detour (function), you pull out the bookmark and
  pick up right where you left off.
  """

  @type t :: %__MODULE__{
          return_address: non_neg_integer(),
          saved_variables: map(),
          saved_locals: [any()]
        }

  defstruct [:return_address, :saved_variables, :saved_locals]
end
