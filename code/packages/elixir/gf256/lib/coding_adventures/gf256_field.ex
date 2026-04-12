defmodule CodingAdventures.GF256Field do
  @moduledoc """
  A parameterizable GF(2^8) field struct.

  Holds only the primitive polynomial. Operations use Russian peasant
  (shift-and-XOR) multiplication — no log/antilog tables.
  Create instances via `CodingAdventures.GF256.new_field/1`.

  Example:
      aes_field = CodingAdventures.GF256.new_field(0x11B)
      CodingAdventures.GF256.multiply(aes_field, 0x53, 0xCA)  # → 1
  """

  defstruct [:polynomial]

  @type t :: %__MODULE__{
          polynomial: non_neg_integer()
        }
end
