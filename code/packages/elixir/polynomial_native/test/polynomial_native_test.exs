defmodule CodingAdventures.PolynomialNativeTest do
  use ExUnit.Case, async: true

  # -------------------------------------------------------------------------
  # NOTE: These tests require the Rust NIF to be compiled.
  # Run `mix compile` before `mix test`. The NIF is built with:
  #   cd native/polynomial_native && cargo build --release
  #
  # Without a compiled NIF, every call raises :not_loaded.
  # -------------------------------------------------------------------------

  alias CodingAdventures.PolynomialNative, as: P

  # -- Fundamentals ----------------------------------------------------------

  test "zero/0 returns [0.0]" do
    assert P.zero() == [0.0]
  end

  test "one/0 returns [1.0]" do
    assert P.one() == [1.0]
  end

  test "normalize strips trailing zeros" do
    assert P.normalize([1.0, 0.0, 0.0]) == [1.0]
  end

  test "normalize of all zeros gives empty list" do
    assert P.normalize([0.0]) == []
  end

  test "normalize of already-normalized poly is identity" do
    assert P.normalize([1.0, 2.0, 3.0]) == [1.0, 2.0, 3.0]
  end

  test "degree of constant polynomial is 0" do
    assert P.degree([5.0]) == 0
  end

  test "degree of quadratic is 2" do
    assert P.degree([3.0, 0.0, 2.0]) == 2
  end

  test "degree of zero polynomial is 0" do
    assert P.degree([]) == 0
  end

  # -- Addition --------------------------------------------------------------

  test "add two polynomials term-by-term" do
    # [1 + 2x] + [3 + 4x] = [4 + 6x]
    assert P.add([1.0, 2.0], [3.0, 4.0]) == [4.0, 6.0]
  end

  test "add polynomials of different lengths" do
    # [1 + 2x + 3x²] + [4 + 5x] = [5 + 7x + 3x²]
    assert P.add([1.0, 2.0, 3.0], [4.0, 5.0]) == [5.0, 7.0, 3.0]
  end

  test "add cancelling polynomials yields zero" do
    assert P.add([1.0, 2.0], [-1.0, -2.0]) == []
  end

  # -- Subtraction -----------------------------------------------------------

  test "subtract removes a polynomial from itself" do
    result = P.subtract([1.0, 2.0, 3.0], [1.0, 2.0, 3.0])
    assert result == []
  end

  test "subtract adjusts coefficients" do
    # [5 + 7x + 3x²] - [1 + 2x + 3x²] = [4 + 5x]
    assert P.subtract([5.0, 7.0, 3.0], [1.0, 2.0, 3.0]) == [4.0, 5.0]
  end

  # -- Multiplication --------------------------------------------------------

  test "multiply (1 + 2x)(3 + 4x) = 3 + 10x + 8x²" do
    assert P.multiply([1.0, 2.0], [3.0, 4.0]) == [3.0, 10.0, 8.0]
  end

  test "multiply by zero poly yields empty" do
    assert P.multiply([1.0, 2.0], []) == []
  end

  test "multiply by one is identity" do
    assert P.multiply([1.0, 2.0, 3.0], P.one()) == [1.0, 2.0, 3.0]
  end

  # -- Division and Modulo ---------------------------------------------------

  test "divide yields quotient" do
    # [5 + x + 3x² + 2x³] / [2 + x] = [3 - x + 2x²]
    assert P.divide([5.0, 1.0, 3.0, 2.0], [2.0, 1.0]) == [3.0, -1.0, 2.0]
  end

  test "modulo yields remainder" do
    rem = P.modulo([5.0, 1.0, 3.0, 2.0], [2.0, 1.0])
    # Verify remainder has degree < divisor degree
    assert length(rem) <= 1
  end

  test "divmod returns {quotient, remainder}" do
    {q, r} = P.divmod([5.0, 1.0, 3.0, 2.0], [2.0, 1.0])
    assert q == [3.0, -1.0, 2.0]
    assert length(r) <= 1
  end

  # When a NIF returns :badarg, Elixir raises ArgumentError (not ErlangError).
  # ErlangError is reserved for Erlang errors with no specific Elixir mapping.
  test "divmod by zero polynomial raises badarg" do
    assert_raise ArgumentError, fn -> P.divmod([1.0, 2.0], []) end
  end

  # -- Evaluate --------------------------------------------------------------

  test "evaluate [3, 0, 1] at x=2 gives 7" do
    # 3 + 0*2 + 1*4 = 7
    assert P.evaluate([3.0, 0.0, 1.0], 2.0) == 7.0
  end

  test "evaluate zero polynomial gives 0" do
    assert P.evaluate([], 99.0) == 0.0
  end

  test "evaluate constant polynomial gives the constant" do
    assert P.evaluate([42.0], 1000.0) == 42.0
  end

  # -- GCD -------------------------------------------------------------------

  test "gcd of (x-1)(x-2) and (x-1) is (x-1)" do
    # [2 - 3x + x²] = (x-1)(x-2)
    # [-1 + x] = (x-1)
    result = P.gcd([2.0, -3.0, 1.0], [-1.0, 1.0])
    # The GCD should be proportional to (x-1); leading coeff may differ.
    assert length(result) == 2
    # Ratio of coefficients should be constant (same polynomial up to scaling).
    [c0, c1] = result
    assert abs(c1 / c0 - (-1.0)) < 1.0e-9
  end

  test "gcd of coprime polynomials is a constant" do
    # x and x+1 are coprime
    result = P.gcd([0.0, 1.0], [1.0, 1.0])
    assert length(result) == 1
  end
end
