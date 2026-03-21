defmodule CodingAdventures.VirtualMachine.Types.BuiltinFunction do
  @moduledoc """
  A host-language function callable from within the VM.

  ## Why Builtins?

  Some operations are impractical to implement in bytecode — printing to
  the console, reading files, getting the current time, performing complex
  math. These "built-in" functions are written in Elixir and made available
  to VM programs by name.

  When bytecode executes a CALL_BUILTIN instruction, the VM looks up the
  function name in its builtins registry and invokes the Elixir function.

  ## Fields

  - **name**: the string name programs use to call this function (e.g., "print")
  - **implementation**: the actual Elixir function. Its signature depends on
    how the specific VM implementation handles builtins — typically it
    receives the current VM state and returns a new state.
  """

  @type t :: %__MODULE__{
          name: String.t(),
          implementation: function()
        }

  defstruct [:name, :implementation]
end
