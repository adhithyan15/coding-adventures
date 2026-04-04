defmodule CodingAdventures.GF256NativeTest do
  use ExUnit.Case, async: true
  # Import Bitwise so the ^^^ (XOR) and other bitwise operators are in scope.
  # Without this, `0x53 ^^^ 0xCA` fails with "undefined function ^^^/2".
  # In Elixir ≥ 1.14 the `^^^` (XOR) operator from Bitwise is available but
  # must be explicitly imported — it is not part of the kernel.
  use Bitwise

  # -------------------------------------------------------------------------
  # NOTE: These tests require the Rust NIF to be compiled.
  # Run `mix compile` before `mix test`. The NIF is built with:
  #   cd native/gf256_native && cargo build --release
  # Without a compiled NIF, every call raises :not_loaded.
  # -------------------------------------------------------------------------

  alias CodingAdventures.GF256Native, as: GF

  # -- Addition (XOR) --------------------------------------------------------

  test "add is XOR" do
    assert GF.add(0x53, 0xCA) == 0x53 ^^^ 0xCA
  end

  test "add with zero is identity" do
    assert GF.add(42, 0) == 42
    assert GF.add(0, 42) == 42
  end

  test "add element to itself gives zero (characteristic 2)" do
    assert GF.add(255, 255) == 0
    assert GF.add(42, 42) == 0
  end

  test "add is commutative" do
    assert GF.add(83, 202) == GF.add(202, 83)
  end

  # -- Subtraction (also XOR) ------------------------------------------------

  test "subtract equals add in characteristic 2" do
    assert GF.subtract(83, 202) == GF.add(83, 202)
  end

  test "subtract element from itself gives zero" do
    assert GF.subtract(99, 99) == 0
  end

  # -- Multiplication --------------------------------------------------------

  test "multiply by zero gives zero" do
    assert GF.multiply(255, 0) == 0
    assert GF.multiply(0, 255) == 0
  end

  test "multiply by one is identity" do
    assert GF.multiply(83, 1) == 83
    assert GF.multiply(1, 83) == 83
  end

  test "multiply 2*2 = 4 (no overflow yet)" do
    assert GF.multiply(2, 2) == 4
  end

  test "multiply is commutative" do
    assert GF.multiply(17, 31) == GF.multiply(31, 17)
  end

  test "multiply 2^8 reduces modulo primitive polynomial" do
    # 2^8 = 256 → XOR with 0x11D = 285 → 256 XOR 285 = 29
    assert GF.power(2, 8) == 29
  end

  # -- Division --------------------------------------------------------------

  # When a NIF returns :badarg, Elixir raises ArgumentError (not ErlangError).
  # ErlangError is for other Erlang errors that have no specific Elixir mapping.

  test "divide by zero raises badarg" do
    assert_raise ArgumentError, fn -> GF.divide(5, 0) end
  end

  test "divide zero by non-zero gives zero" do
    assert GF.divide(0, 7) == 0
  end

  test "divide is inverse of multiply" do
    a = 83
    b = 202
    product = GF.multiply(a, b)
    assert GF.divide(product, b) == a
  end

  # -- Power -----------------------------------------------------------------

  test "power(x, 0) = 1 for non-zero x" do
    assert GF.power(5, 0) == 1
  end

  test "power(0, 0) = 1 by convention" do
    assert GF.power(0, 0) == 1
  end

  test "power(0, n) = 0 for n > 0" do
    assert GF.power(0, 5) == 0
  end

  test "power(2, 1) = 2" do
    assert GF.power(2, 1) == 2
  end

  # -- Inverse ---------------------------------------------------------------

  test "inverse of zero raises badarg" do
    assert_raise ArgumentError, fn -> GF.inverse(0) end
  end

  test "inverse of one is one" do
    assert GF.inverse(1) == 1
  end

  test "a * inverse(a) = 1 for various elements" do
    for a <- [2, 17, 83, 128, 255] do
      inv = GF.inverse(a)
      assert GF.multiply(a, inv) == 1, "multiply(#{a}, inverse(#{a})) should be 1"
    end
  end

  test "inverse of inverse is identity" do
    a = 83
    assert GF.inverse(GF.inverse(a)) == a
  end
end
