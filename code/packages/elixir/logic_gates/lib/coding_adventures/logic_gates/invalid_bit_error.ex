defmodule CodingAdventures.LogicGates.InvalidBitError do
  @moduledoc """
  Raised when a gate argument is not a bit.

  A bit is exactly the integer `0` or the integer `1` (strict `===`):
  `1.0`, `true`, `"1"`, `'1'`, `nil`, lists, tuples, maps, structs and every
  other integer are all rejected.

  Elixir exceptions cannot subclass one another, so this is a distinct
  exception from `ArgumentError`; callers that previously rescued
  `ArgumentError` for bad bits must rescue this module instead.

  Fields:

    * `:param` -- name of the first invalid parameter (e.g. `"a"`, `"b"`)
    * `:value_description` -- a short (well under 64 chars), bounded
      description of the rejected value, built without calling any user
      protocol implementation.
  """

  defexception [:param, :value_description]

  @impl true
  def message(%__MODULE__{param: param, value_description: desc}) do
    "#{param} must be the integer 0 or 1, got #{desc}"
  end
end
