defmodule CodingAdventures.PolynomialNative do
  @moduledoc """
  Native (Rust NIF) implementation of polynomial arithmetic.

  Polynomials are represented as lists of floats where the index equals the
  degree of the coefficient:

      [3.0, 0.0, 1.0]  →  3 + 0·x + 1·x²  =  3 + x²

  This "little-endian" layout (constant term first) matches the Rust
  `polynomial` crate's internal representation.

  ## How the NIF works

  On first use, `@on_load :load_nif` triggers `load_nif/0`, which calls
  `:erlang.load_nif/2` to load the compiled Rust shared library from the
  application's `priv` directory. The BEAM then replaces these stub
  functions with the native Rust implementations.

  If the NIF has not been compiled yet (e.g., in a pure Elixir test
  environment), the stubs raise `:not_loaded` so the failure is explicit.

  ## Building the NIF

  Run `mix compile` from this package directory. Mix's make compiler will
  invoke `cargo build --release` in `native/polynomial_native/` and copy
  the resulting shared library into `priv/`.
  """

  @on_load :load_nif

  @doc false
  def load_nif do
    # :code.priv_dir returns the path to the application's priv directory,
    # where Mix places compiled NIF shared libraries.
    priv_dir = :code.priv_dir(:coding_adventures_polynomial_native)
    nif_path = Path.join(priv_dir, "polynomial_native")
    # load_nif expects a path WITHOUT the extension (.so/.dylib/.dll) —
    # the BEAM appends the correct extension for the current OS automatically.
    :erlang.load_nif(to_charlist(nif_path), 0)
  end

  # -------------------------------------------------------------------------
  # NIF stub functions
  # -------------------------------------------------------------------------
  #
  # These are placeholder implementations that the BEAM replaces at load time
  # with the real Rust functions. If the NIF failed to load, calling any of
  # these raises a :not_loaded error with a helpful message.
  #
  # The pattern `_poly` (underscore prefix) suppresses "unused variable"
  # warnings in the Elixir compiler — these stubs never actually run.

  @doc """
  Remove trailing near-zero coefficients.

      iex> normalize([1.0, 0.0, 0.0])
      [1.0]
  """
  def normalize(_poly), do: :erlang.nif_error(:not_loaded)

  @doc """
  Return the degree of the polynomial (highest non-zero exponent index).
  The zero polynomial returns 0 by convention.

      iex> degree([3.0, 0.0, 2.0])
      2
  """
  def degree(_poly), do: :erlang.nif_error(:not_loaded)

  @doc """
  Return the zero polynomial `[0.0]` — the additive identity.
  """
  def zero(), do: :erlang.nif_error(:not_loaded)

  @doc """
  Return the one polynomial `[1.0]` — the multiplicative identity.
  """
  def one(), do: :erlang.nif_error(:not_loaded)

  @doc """
  Add two polynomials term-by-term.

      iex> add([1.0, 2.0], [3.0, 4.0])
      [4.0, 6.0]
  """
  def add(_a, _b), do: :erlang.nif_error(:not_loaded)

  @doc """
  Subtract polynomial `b` from polynomial `a` term-by-term.
  """
  def subtract(_a, _b), do: :erlang.nif_error(:not_loaded)

  @doc """
  Multiply two polynomials (convolution). Result degree = deg(a) + deg(b).
  """
  def multiply(_a, _b), do: :erlang.nif_error(:not_loaded)

  @doc """
  Polynomial long division, returning `{quotient, remainder}`.

  Returns `badarg` if the divisor is the zero polynomial.
  """
  def divmod(_a, _b), do: :erlang.nif_error(:not_loaded)

  @doc """
  Polynomial long division — returns only the quotient.

  Returns `badarg` if the divisor is the zero polynomial.
  """
  def divide(_a, _b), do: :erlang.nif_error(:not_loaded)

  @doc """
  Polynomial long division — returns only the remainder.

  Returns `badarg` if the divisor is the zero polynomial.
  """
  def modulo(_a, _b), do: :erlang.nif_error(:not_loaded)

  @doc """
  Evaluate the polynomial at `x` using Horner's method.

      iex> evaluate([3.0, 0.0, 1.0], 2.0)
      7.0
  """
  def evaluate(_poly, _x), do: :erlang.nif_error(:not_loaded)

  @doc """
  Greatest common divisor of two polynomials via the Euclidean algorithm.
  """
  def gcd(_a, _b), do: :erlang.nif_error(:not_loaded)
end
