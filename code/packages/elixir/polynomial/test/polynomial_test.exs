defmodule CodingAdventures.PolynomialTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.Polynomial, as: Poly

  # ────────────────────────────────────────────────────────────────────────────
  # normalize/1
  # ────────────────────────────────────────────────────────────────────────────

  describe "normalize" do
    test "strips trailing zeros" do
      assert Poly.normalize([1.0, 0.0, 0.0]) == [1.0]
    end

    test "all zeros returns [0.0]" do
      assert Poly.normalize([0.0]) == [0.0]
    end

    test "already normalized is unchanged" do
      assert Poly.normalize([1.0, 2.0, 3.0]) == [1.0, 2.0, 3.0]
    end

    test "strips only trailing zeros, not internal" do
      assert Poly.normalize([1.0, 0.0, 2.0, 0.0]) == [1.0, 0.0, 2.0]
    end

    test "near-zero floats are stripped (1.0e-11 < threshold)" do
      assert Poly.normalize([1.0, 1.0e-11]) == [1.0]
    end

    test "coefficient just above threshold is kept" do
      result = Poly.normalize([1.0, 1.0e-9])
      assert length(result) == 2
    end

    test "empty-like strip results in [0.0] not []" do
      # A polynomial [0.0, 0.0] normalizes to [0.0], not [].
      assert Poly.normalize([0.0, 0.0]) == [0.0]
    end
  end

  # ────────────────────────────────────────────────────────────────────────────
  # degree/1
  # ────────────────────────────────────────────────────────────────────────────

  describe "degree" do
    test "degree of constant polynomial" do
      assert Poly.degree([7.0]) == 0
    end

    test "degree of linear polynomial" do
      assert Poly.degree([1.0, 2.0]) == 1
    end

    test "degree of [1.0, 2.0, 3.0] is 2" do
      assert Poly.degree([1.0, 2.0, 3.0]) == 2
    end

    test "degree of zero polynomial is 0" do
      assert Poly.degree([0.0]) == 0
    end

    test "degree ignores trailing zeros" do
      assert Poly.degree([3.0, 0.0, 0.0]) == 0
    end
  end

  # ────────────────────────────────────────────────────────────────────────────
  # zero/0 and one/0
  # ────────────────────────────────────────────────────────────────────────────

  describe "zero and one" do
    test "zero returns [0.0]" do
      assert Poly.zero() == [0.0]
    end

    test "one returns [1.0]" do
      assert Poly.one() == [1.0]
    end

    test "add(zero, p) == p" do
      p = [3.0, 1.0, 2.0]
      assert Poly.add(Poly.zero(), p) == p
    end

    test "multiply(one, p) == p" do
      p = [3.0, 1.0, 2.0]
      assert Poly.multiply(Poly.one(), p) == p
    end
  end

  # ────────────────────────────────────────────────────────────────────────────
  # add/2
  # ────────────────────────────────────────────────────────────────────────────

  describe "add" do
    test "adds two polynomials of same length" do
      assert Poly.add([1.0, 2.0, 3.0], [4.0, 5.0, 6.0]) == [5.0, 7.0, 9.0]
    end

    test "adds polynomials of different lengths" do
      assert Poly.add([1.0, 2.0, 3.0], [4.0, 5.0]) == [5.0, 7.0, 3.0]
    end

    test "add with zero polynomial returns the other" do
      p = [1.0, 2.0, 3.0]
      assert Poly.add(p, Poly.zero()) == p
    end

    test "commutativity: add(a, b) == add(b, a)" do
      a = [1.0, 2.0]
      b = [3.0, 4.0, 5.0]
      assert Poly.add(a, b) == Poly.add(b, a)
    end

    test "result is normalized (leading zeros stripped)" do
      # [1, 2, 3] + [-1, -2, -3] = [0, 0, 0] → [0.0]
      assert Poly.add([1.0, 2.0, 3.0], [-1.0, -2.0, -3.0]) == [0.0]
    end
  end

  # ────────────────────────────────────────────────────────────────────────────
  # subtract/2
  # ────────────────────────────────────────────────────────────────────────────

  describe "subtract" do
    test "subtracts matching coefficients" do
      assert Poly.subtract([5.0, 7.0, 3.0], [1.0, 2.0, 3.0]) == [4.0, 5.0]
    end

    test "subtract from itself is zero" do
      p = [3.0, 1.0, 2.0]
      assert Poly.subtract(p, p) == [0.0]
    end

    test "subtract zero is identity" do
      p = [1.0, 2.0, 3.0]
      assert Poly.subtract(p, Poly.zero()) == p
    end

    test "subtract longer from shorter" do
      # [1] - [1, 1, 1] = [0, -1, -1]
      result = Poly.subtract([1.0], [1.0, 1.0, 1.0])
      assert result == [0.0, -1.0, -1.0]
    end
  end

  # ────────────────────────────────────────────────────────────────────────────
  # multiply/2
  # ────────────────────────────────────────────────────────────────────────────

  describe "multiply" do
    test "basic multiplication (1+2x)(3+4x) = 3+10x+8x^2" do
      assert Poly.multiply([1.0, 2.0], [3.0, 4.0]) == [3.0, 10.0, 8.0]
    end

    test "multiply by zero is zero" do
      assert Poly.multiply([1.0, 2.0, 3.0], Poly.zero()) == [0.0]
    end

    test "multiply by one is identity" do
      p = [3.0, 2.0, 1.0]
      assert Poly.multiply(p, Poly.one()) == p
    end

    test "multiply constants" do
      assert Poly.multiply([3.0], [4.0]) == [12.0]
    end

    test "multiply (x-1)(x+1) = x^2-1" do
      # (x - 1) = [-1.0, 1.0]  (constant -1, linear 1)
      # (x + 1) = [1.0, 1.0]
      # Result = x^2 - 1 = [-1.0, 0.0, 1.0]
      assert Poly.multiply([-1.0, 1.0], [1.0, 1.0]) == [-1.0, 0.0, 1.0]
    end

    test "result degree = sum of input degrees" do
      # degree 2 * degree 3 = degree 5
      a = [1.0, 0.0, 1.0]
      b = [1.0, 0.0, 0.0, 1.0]
      assert Poly.degree(Poly.multiply(a, b)) == 5
    end
  end

  # ────────────────────────────────────────────────────────────────────────────
  # divmod_poly/2
  # ────────────────────────────────────────────────────────────────────────────

  describe "divmod_poly" do
    test "divides x^2-1 by x-1 giving quotient x+1 and remainder 0" do
      # x^2 - 1 = [-1.0, 0.0, 1.0]
      # x - 1   = [-1.0, 1.0]
      # quotient = x + 1 = [1.0, 1.0]
      # remainder = 0
      {q, r} = Poly.divmod_poly([-1.0, 0.0, 1.0], [-1.0, 1.0])
      assert q == [1.0, 1.0]
      assert r == [0.0]
    end

    test "divides [5,1,3,2] by [2,1] (from spec example)" do
      # 5 + x + 3x^2 + 2x^3  ÷  (2 + x)
      # Quotient: 3 - x + 2x^2 = [3, -1, 2]
      # Remainder: -1
      {q, r} = Poly.divmod_poly([5.0, 1.0, 3.0, 2.0], [2.0, 1.0])
      assert q == [3.0, -1.0, 2.0]
      assert r == [-1.0]
    end

    test "dividend has lower degree than divisor returns (0, dividend)" do
      {q, r} = Poly.divmod_poly([1.0, 2.0], [1.0, 0.0, 1.0])
      assert q == [0.0]
      assert r == [1.0, 2.0]
    end

    test "divmod by one returns (dividend, 0)" do
      p = [4.0, 3.0, 2.0, 1.0]
      {q, r} = Poly.divmod_poly(p, [1.0])
      assert q == p
      assert r == [0.0]
    end

    test "divmod by zero raises ArgumentError" do
      assert_raise ArgumentError, fn ->
        Poly.divmod_poly([1.0, 2.0], [0.0])
      end
    end

    test "verify: a = b*q + r for general case" do
      a = [5.0, 1.0, 3.0, 2.0]
      b = [2.0, 1.0]
      {q, r} = Poly.divmod_poly(a, b)
      # Reconstruct: b*q + r should equal a
      reconstructed = Poly.add(Poly.multiply(b, q), r)
      assert_polynomials_close(reconstructed, a)
    end

    test "exact division leaves zero remainder" do
      # (x+1)(x+2) = x^2 + 3x + 2
      product = Poly.multiply([1.0, 1.0], [2.0, 1.0])
      {_q, r} = Poly.divmod_poly(product, [1.0, 1.0])
      assert r == [0.0]
    end
  end

  # ────────────────────────────────────────────────────────────────────────────
  # divide/2 and modulo/2
  # ────────────────────────────────────────────────────────────────────────────

  describe "divide" do
    test "returns quotient" do
      q = Poly.divide([-1.0, 0.0, 1.0], [-1.0, 1.0])
      assert q == [1.0, 1.0]
    end

    test "raises on zero divisor" do
      assert_raise ArgumentError, fn ->
        Poly.divide([1.0], [0.0])
      end
    end
  end

  describe "modulo" do
    test "returns remainder" do
      r = Poly.modulo([5.0, 1.0, 3.0, 2.0], [2.0, 1.0])
      assert r == [-1.0]
    end

    test "raises on zero divisor" do
      assert_raise ArgumentError, fn ->
        Poly.modulo([1.0], [0.0])
      end
    end

    test "exact division gives zero remainder" do
      r = Poly.modulo([-1.0, 0.0, 1.0], [-1.0, 1.0])
      assert r == [0.0]
    end
  end

  # ────────────────────────────────────────────────────────────────────────────
  # evaluate/2
  # ────────────────────────────────────────────────────────────────────────────

  describe "evaluate" do
    test "evaluate [3.0, 0.0, 1.0] at 2.0 gives 7.0" do
      # 3 + 0*2 + 1*4 = 3 + 0 + 4 = 7
      assert Poly.evaluate([3.0, 0.0, 1.0], 2.0) == 7.0
    end

    test "evaluate [3.0, 1.0, 2.0] at 4.0 gives 39.0" do
      # 3 + 4 + 2*16 = 39 (from spec example)
      assert Poly.evaluate([3.0, 1.0, 2.0], 4.0) == 39.0
    end

    test "evaluate zero polynomial is 0" do
      assert Poly.evaluate([0.0], 5.0) == 0.0
    end

    test "evaluate constant polynomial" do
      assert Poly.evaluate([7.0], 100.0) == 7.0
    end

    test "evaluate at x=0 returns constant term" do
      assert Poly.evaluate([42.0, 10.0, 3.0], 0.0) == 42.0
    end

    test "evaluate at x=1 sums all coefficients" do
      # p(1) = 1 + 2 + 3 = 6
      assert Poly.evaluate([1.0, 2.0, 3.0], 1.0) == 6.0
    end
  end

  # ────────────────────────────────────────────────────────────────────────────
  # gcd/2
  # ────────────────────────────────────────────────────────────────────────────

  describe "gcd" do
    test "gcd of (x-1) and (x^2-1) is (x-1)" do
      # x^2 - 1 = [-1.0, 0.0, 1.0]
      # x - 1   = [-1.0, 1.0]
      # GCD = x - 1  (monic form: [-1.0, 1.0])
      result = Poly.gcd([-1.0, 0.0, 1.0], [-1.0, 1.0])
      assert_polynomials_close(result, [-1.0, 1.0])
    end

    test "gcd with one is one (monic)" do
      result = Poly.gcd([1.0, 1.0], [1.0])
      assert result == [1.0]
    end

    test "gcd of coprime polynomials is constant 1" do
      # x and x+1 are coprime
      result = Poly.gcd([0.0, 1.0], [1.0, 1.0])
      assert_polynomials_close(result, [1.0])
    end

    test "gcd with zero polynomial is the other polynomial" do
      p = [2.0, 1.0]
      result = Poly.gcd(p, [0.0])
      # GCD(p, 0) = p, made monic
      assert_polynomials_close(result, [2.0, 1.0])
    end

    test "gcd is commutative" do
      a = [-1.0, 0.0, 1.0]
      b = [-1.0, 1.0]
      assert_polynomials_close(Poly.gcd(a, b), Poly.gcd(b, a))
    end

    test "gcd of polynomial with itself is itself (monic)" do
      p = [2.0, 4.0]
      result = Poly.gcd(p, p)
      # GCD(p, p) = p, monic = [0.5, 1.0]
      assert length(result) == 2
      assert_float_close(List.last(result), 1.0)
    end
  end

  # ────────────────────────────────────────────────────────────────────────────
  # Helpers
  # ────────────────────────────────────────────────────────────────────────────

  defp assert_polynomials_close(a, b) do
    # Compare polynomials element-wise with a small float tolerance.
    na = CodingAdventures.Polynomial.normalize(a)
    nb = CodingAdventures.Polynomial.normalize(b)
    assert length(na) == length(nb),
           "Polynomial lengths differ: #{inspect(na)} vs #{inspect(nb)}"

    Enum.zip(na, nb)
    |> Enum.each(fn {x, y} ->
      assert_float_close(x, y)
    end)
  end

  defp assert_float_close(a, b) do
    assert abs(a - b) < 1.0e-9,
           "Expected #{a} to be close to #{b}"
  end
end
