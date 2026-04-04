defmodule CodingAdventures.GF256Native do
  @moduledoc """
  Native (Rust NIF) implementation of GF(256) Galois Field arithmetic.

  GF(256) is the finite field with 256 elements, used in:
  - Reed-Solomon error correction (QR codes, CDs, DVDs)
  - AES encryption (MixColumns, SubBytes)
  - General-purpose error-correcting codes

  ## Elements

  Elements are integers in the range 0–255. In GF(256), they represent
  polynomials over GF(2) with degree ≤ 7, reduced modulo the primitive
  polynomial `p(x) = x⁸ + x⁴ + x³ + x² + 1`.

  ## Characteristic 2

  Addition and subtraction are both XOR in GF(256) — no carry, no overflow.

  ## Multiplication

  Uses precomputed logarithm/antilogarithm tables for O(1) time complexity.
  """

  @on_load :load_nif

  @doc false
  def load_nif do
    priv_dir = :code.priv_dir(:coding_adventures_gf256_native)
    nif_path = Path.join(priv_dir, "gf256_native")
    :erlang.load_nif(to_charlist(nif_path), 0)
  end

  @doc """
  Add two GF(256) elements. In characteristic 2, addition is XOR.

      iex> add(83, 202)
      153
  """
  def add(_a, _b), do: :erlang.nif_error(:not_loaded)

  @doc """
  Subtract two GF(256) elements. Equal to add/2 in characteristic 2.

      iex> subtract(83, 202)
      153
  """
  def subtract(_a, _b), do: :erlang.nif_error(:not_loaded)

  @doc """
  Multiply two GF(256) elements using log/antilog tables.

      iex> multiply(2, 2)
      4
  """
  def multiply(_a, _b), do: :erlang.nif_error(:not_loaded)

  @doc """
  Divide `a` by `b` in GF(256).

  Returns `badarg` if `b` is 0 (division by zero is undefined).
  `divide(0, b)` returns 0 for any non-zero `b`.

      iex> divide(4, 2)
      2
  """
  def divide(_a, _b), do: :erlang.nif_error(:not_loaded)

  @doc """
  Raise a GF(256) element to a non-negative integer power.

  `power(0, 0)` returns 1 by convention.

      iex> power(2, 8)
      29
  """
  def power(_base, _exp), do: :erlang.nif_error(:not_loaded)

  @doc """
  Multiplicative inverse: `multiply(a, inverse(a)) == 1`.

  Returns `badarg` if `a` is 0 (zero has no multiplicative inverse).

      iex> inverse(1)
      1
  """
  def inverse(_a), do: :erlang.nif_error(:not_loaded)
end
