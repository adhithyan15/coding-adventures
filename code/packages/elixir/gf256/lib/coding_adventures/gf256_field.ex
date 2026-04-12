defmodule CodingAdventures.GF256Field do
  @moduledoc """
  A parameterizable GF(2^8) field struct.

  Holds the primitive polynomial and the precomputed log/antilog tables
  for that polynomial. Create instances via `CodingAdventures.GF256.new_field/1`.

  Example:
      aes_field = CodingAdventures.GF256.new_field(0x11B)
      CodingAdventures.GF256.multiply(aes_field, 0x53, 0x8C)  # → 1
  """

  defstruct [:polynomial, :alog, :log]

  @type t :: %__MODULE__{
          polynomial: non_neg_integer(),
          alog: list(non_neg_integer()),
          log: list(non_neg_integer())
        }
end
