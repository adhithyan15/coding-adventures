defmodule CodingAdventures.WasmExecution.TrapError do
  @moduledoc """
  An unrecoverable WASM runtime error (a "trap").

  In WASM, a trap is a fatal error that immediately halts execution of the
  current module. Traps occur on:

  - Out-of-bounds memory access
  - Out-of-bounds table access
  - Division by zero (integer division only)
  - Integer overflow in division (e.g., i32.div_s(-2147483648, -1))
  - Unreachable instruction executed
  - Type mismatch in call_indirect

  We model traps as a custom exception so that host code can distinguish
  them from other errors using pattern matching or rescue clauses.
  """

  defexception [:message]

  @impl true
  def exception(msg) when is_binary(msg) do
    %__MODULE__{message: msg}
  end
end
