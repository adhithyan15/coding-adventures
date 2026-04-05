defmodule CodingAdventures.ReedSolomonTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.ReedSolomon, as: RS
  alias CodingAdventures.ReedSolomon.TooManyErrors
  alias CodingAdventures.ReedSolomon.InvalidInput

  # Cross-language test vector:
  # build_generator(2) must equal [8, 6, 1] in all implementations.
  # g(x) = (x + α)(x + α²) = (x+2)(x+4)
  # Constant term:   GF.multiply(2, 4) = 8
  # x coefficient:   GF.add(2, 4) = 6  (XOR: 2 XOR 4 = 6)
  # x² coefficient: 1 (monic)

  # ── build_generator ──────────────────────────────────────────────────────────

  describe "build_generator/1" do
    test "n_check=2 gives [8, 6, 1]" do
      assert RS.build_generator(2) == [8, 6, 1]
    end

    test "n_check=2 polynomial has roots α and α²" do
      # If g is correct, g(α) = 0 and g(α²) = 0
      alias CodingAdventures.GF256, as: GF
      g = RS.build_generator(2)
      alpha = GF.power(2, 1)
      alpha2 = GF.power(2, 2)
      # Evaluate g at α: g[0] + g[1]*α + g[2]*α² must be 0 in GF(256)
      eval = fn x ->
        g
        |> Enum.with_index()
        |> Enum.reduce(0, fn {c, i}, acc ->
          GF.add(acc, GF.multiply(c, GF.power(x, i)))
        end)
      end

      assert eval.(alpha) == 0
      assert eval.(alpha2) == 0
    end

    test "n_check=4 returns polynomial of length 5" do
      g = RS.build_generator(4)
      assert length(g) == 5
    end

    test "n_check=4 is monic (last coefficient = 1)" do
      g = RS.build_generator(4)
      assert List.last(g) == 1
    end

    test "n_check=4 has correct constant term" do
      # Constant term = α¹ * α² * α³ * α⁴ = α^(1+2+3+4) = α^10
      alias CodingAdventures.GF256, as: GF
      g = RS.build_generator(4)
      expected = GF.power(2, 10)
      assert Enum.at(g, 0) == expected
    end

    test "n_check=8 returns polynomial of length 9" do
      g = RS.build_generator(8)
      assert length(g) == 9
    end

    test "n_check=8 is monic" do
      g = RS.build_generator(8)
      assert List.last(g) == 1
    end

    test "all roots are zero for n_check=4" do
      alias CodingAdventures.GF256, as: GF
      g = RS.build_generator(4)

      eval = fn x ->
        g
        |> Enum.with_index()
        |> Enum.reduce(0, fn {c, i}, acc ->
          GF.add(acc, GF.multiply(c, GF.power(x, i)))
        end)
      end

      Enum.each(1..4, fn i ->
        alpha_i = GF.power(2, i)
        assert eval.(alpha_i) == 0, "g(α^#{i}) should be 0"
      end)
    end
  end

  # ── encode ───────────────────────────────────────────────────────────────────

  describe "encode/2" do
    test "output length = message length + n_check" do
      msg = [1, 2, 3, 4, 5]
      codeword = RS.encode(msg, 4)
      assert length(codeword) == 9
    end

    test "systematic: message bytes are preserved at front" do
      msg = [72, 101, 108, 108, 111]
      codeword = RS.encode(msg, 8)
      assert Enum.take(codeword, 5) == msg
    end

    test "check bytes are not all zero for non-trivial message" do
      msg = [1, 2, 3]
      codeword = RS.encode(msg, 4)
      check = Enum.drop(codeword, 3)
      refute Enum.all?(check, &(&1 == 0))
    end

    test "encoding all-zero message gives all-zero check bytes" do
      msg = [0, 0, 0, 0]
      codeword = RS.encode(msg, 4)
      check = Enum.drop(codeword, 4)
      assert Enum.all?(check, &(&1 == 0))
    end

    test "syndromes of a valid codeword are all zero" do
      msg = [10, 20, 30, 40, 50]
      n_check = 8
      codeword = RS.encode(msg, n_check)
      synd = RS.syndromes(codeword, n_check)
      assert Enum.all?(synd, &(&1 == 0))
    end

    test "single-byte message encodes correctly" do
      msg = [0xFF]
      codeword = RS.encode(msg, 2)
      assert length(codeword) == 3
      assert Enum.at(codeword, 0) == 0xFF
    end

    test "n_check=16 encodes correctly" do
      msg = Enum.to_list(1..20)
      codeword = RS.encode(msg, 16)
      assert length(codeword) == 36
      assert Enum.take(codeword, 20) == msg
      synd = RS.syndromes(codeword, 16)
      assert Enum.all?(synd, &(&1 == 0))
    end

    test "raises InvalidInput for n_check=0" do
      assert_raise InvalidInput, fn -> RS.encode([1, 2, 3], 0) end
    end

    test "raises InvalidInput for odd n_check" do
      assert_raise InvalidInput, fn -> RS.encode([1, 2, 3], 3) end
    end

    test "raises InvalidInput if codeword exceeds 255 bytes" do
      msg = Enum.to_list(1..250)
      assert_raise InvalidInput, fn -> RS.encode(msg, 8) end
    end
  end

  # ── syndromes ────────────────────────────────────────────────────────────────

  describe "syndromes/2" do
    test "valid codeword has all-zero syndromes" do
      msg = [1, 2, 3, 4]
      n_check = 4
      codeword = RS.encode(msg, n_check)
      assert RS.syndromes(codeword, n_check) == [0, 0, 0, 0]
    end

    test "corrupted codeword has non-zero syndromes" do
      msg = [10, 20, 30]
      n_check = 4
      codeword = RS.encode(msg, n_check)
      corrupted = List.replace_at(codeword, 0, Bitwise.bxor(Enum.at(codeword, 0), 1))
      synd = RS.syndromes(corrupted, n_check)
      refute Enum.all?(synd, &(&1 == 0))
    end

    test "returns n_check syndrome values" do
      codeword = RS.encode([1, 2, 3], 8)
      assert length(RS.syndromes(codeword, 8)) == 8
    end
  end

  # ── decode — no errors ───────────────────────────────────────────────────────

  describe "decode/2 — no errors" do
    test "decodes uncorrupted codeword correctly" do
      msg = [72, 101, 108, 108, 111]
      n_check = 8
      codeword = RS.encode(msg, n_check)
      assert RS.decode(codeword, n_check) == msg
    end

    test "decodes all-zero message" do
      msg = [0, 0, 0, 0, 0]
      n_check = 4
      codeword = RS.encode(msg, n_check)
      assert RS.decode(codeword, n_check) == msg
    end

    test "decodes all-0xFF message" do
      msg = List.duplicate(0xFF, 8)
      n_check = 4
      codeword = RS.encode(msg, n_check)
      assert RS.decode(codeword, n_check) == msg
    end
  end

  # ── decode — single error ────────────────────────────────────────────────────

  describe "decode/2 — single error" do
    test "corrects 1 error at position 0 with n_check=2" do
      msg = [1, 2, 3, 4, 5]
      n_check = 2
      codeword = RS.encode(msg, n_check)
      corrupted = List.replace_at(codeword, 0, Bitwise.bxor(Enum.at(codeword, 0), 0xFF))
      assert RS.decode(corrupted, n_check) == msg
    end

    test "corrects 1 error in the middle of message" do
      msg = [10, 20, 30, 40, 50, 60]
      n_check = 4
      codeword = RS.encode(msg, n_check)
      corrupted = List.replace_at(codeword, 3, Bitwise.bxor(Enum.at(codeword, 3), 0xAA))
      assert RS.decode(corrupted, n_check) == msg
    end

    test "corrects 1 error in the check bytes" do
      msg = [1, 2, 3]
      n_check = 4
      codeword = RS.encode(msg, n_check)
      check_pos = 4
      corrupted = List.replace_at(codeword, check_pos, Bitwise.bxor(Enum.at(codeword, check_pos), 0x55))
      assert RS.decode(corrupted, n_check) == msg
    end

    test "corrects 1 error at last position" do
      msg = [1, 2, 3, 4]
      n_check = 4
      codeword = RS.encode(msg, n_check)
      last = length(codeword) - 1
      corrupted = List.replace_at(codeword, last, Bitwise.bxor(Enum.at(codeword, last), 0x01))
      assert RS.decode(corrupted, n_check) == msg
    end
  end

  # ── decode — multiple errors ──────────────────────────────────────────────────

  describe "decode/2 — multiple errors at capacity" do
    test "corrects 2 errors with n_check=4" do
      msg = [10, 20, 30, 40, 50]
      n_check = 4
      codeword = RS.encode(msg, n_check)

      corrupted =
        codeword
        |> List.replace_at(0, Bitwise.bxor(Enum.at(codeword, 0), 0xFF))
        |> List.replace_at(3, Bitwise.bxor(Enum.at(codeword, 3), 0xAA))

      assert RS.decode(corrupted, n_check) == msg
    end

    test "corrects 4 errors with n_check=8" do
      msg = String.to_charlist("Hello, World!")
      n_check = 8
      codeword = RS.encode(msg, n_check)

      corrupted =
        codeword
        |> List.replace_at(0, Bitwise.bxor(Enum.at(codeword, 0), 0xFF))
        |> List.replace_at(3, Bitwise.bxor(Enum.at(codeword, 3), 0xAA))
        |> List.replace_at(7, Bitwise.bxor(Enum.at(codeword, 7), 0x55))
        |> List.replace_at(10, Bitwise.bxor(Enum.at(codeword, 10), 0x0F))

      assert RS.decode(corrupted, n_check) == msg
    end

    test "corrects t=8 errors with n_check=16" do
      msg = Enum.to_list(1..20)
      n_check = 16
      codeword = RS.encode(msg, n_check)

      positions = [0, 2, 4, 6, 8, 10, 12, 14]

      corrupted =
        Enum.reduce(positions, codeword, fn p, acc ->
          List.replace_at(acc, p, Bitwise.bxor(Enum.at(acc, p), 0xAB))
        end)

      assert RS.decode(corrupted, n_check) == msg
    end

    test "corrects errors at consecutive positions" do
      msg = [100, 101, 102, 103, 104, 105]
      n_check = 8
      codeword = RS.encode(msg, n_check)

      corrupted =
        codeword
        |> List.replace_at(1, Bitwise.bxor(Enum.at(codeword, 1), 0x11))
        |> List.replace_at(2, Bitwise.bxor(Enum.at(codeword, 2), 0x22))
        |> List.replace_at(3, Bitwise.bxor(Enum.at(codeword, 3), 0x33))
        |> List.replace_at(4, Bitwise.bxor(Enum.at(codeword, 4), 0x44))

      assert RS.decode(corrupted, n_check) == msg
    end

    test "corrects errors spanning message and check bytes" do
      msg = [1, 2, 3, 4, 5]
      n_check = 8
      codeword = RS.encode(msg, n_check)
      n = length(codeword)

      corrupted =
        codeword
        |> List.replace_at(0, Bitwise.bxor(Enum.at(codeword, 0), 0x01))
        |> List.replace_at(n - 1, Bitwise.bxor(Enum.at(codeword, n - 1), 0x02))
        |> List.replace_at(n - 2, Bitwise.bxor(Enum.at(codeword, n - 2), 0x03))
        |> List.replace_at(div(n, 2), Bitwise.bxor(Enum.at(codeword, div(n, 2)), 0x04))

      assert RS.decode(corrupted, n_check) == msg
    end
  end

  # ── decode — error handling ──────────────────────────────────────────────────

  describe "decode/2 — error handling" do
    test "raises TooManyErrors when t+1 errors are introduced" do
      msg = [1, 2, 3, 4, 5]
      n_check = 4  # t = 2; introduce 3 errors
      codeword = RS.encode(msg, n_check)

      corrupted =
        codeword
        |> List.replace_at(0, Bitwise.bxor(Enum.at(codeword, 0), 0xFF))
        |> List.replace_at(1, Bitwise.bxor(Enum.at(codeword, 1), 0xAA))
        |> List.replace_at(2, Bitwise.bxor(Enum.at(codeword, 2), 0x55))

      assert_raise TooManyErrors, fn -> RS.decode(corrupted, n_check) end
    end

    test "raises InvalidInput for n_check=0" do
      assert_raise InvalidInput, fn -> RS.decode([1, 2, 3], 0) end
    end

    test "raises InvalidInput for odd n_check" do
      assert_raise InvalidInput, fn -> RS.decode([1, 2, 3, 4], 3) end
    end

    test "raises InvalidInput if received is shorter than n_check" do
      assert_raise InvalidInput, fn -> RS.decode([1, 2], 4) end
    end
  end

  # ── error_locator ────────────────────────────────────────────────────────────

  describe "error_locator/1" do
    test "all-zero syndromes give locator [1]" do
      synd = [0, 0, 0, 0]
      assert RS.error_locator(synd) == [1]
    end

    test "single-error syndromes give degree-1 locator" do
      msg = [1, 2, 3, 4, 5]
      n_check = 4
      codeword = RS.encode(msg, n_check)
      corrupted = List.replace_at(codeword, 2, Bitwise.bxor(Enum.at(codeword, 2), 0x55))
      synd = RS.syndromes(corrupted, n_check)
      locator = RS.error_locator(synd)
      # Degree-1 locator: [1 + something*x] → length 2
      assert length(locator) == 2
      assert Enum.at(locator, 0) == 1
    end

    test "two-error syndromes give degree-2 locator" do
      msg = [1, 2, 3, 4, 5]
      n_check = 8
      codeword = RS.encode(msg, n_check)

      corrupted =
        codeword
        |> List.replace_at(0, Bitwise.bxor(Enum.at(codeword, 0), 0xFF))
        |> List.replace_at(4, Bitwise.bxor(Enum.at(codeword, 4), 0xAA))

      synd = RS.syndromes(corrupted, n_check)
      locator = RS.error_locator(synd)
      assert length(locator) == 3
      assert Enum.at(locator, 0) == 1
    end
  end

  # ── round-trip property tests ────────────────────────────────────────────────

  describe "encode/decode round-trip" do
    test "round-trip with various message lengths" do
      Enum.each([1, 5, 10, 20, 50, 100], fn len ->
        msg = Enum.map(1..len, fn i -> rem(i * 7, 256) end)
        n_check = 8
        codeword = RS.encode(msg, n_check)
        assert RS.decode(codeword, n_check) == msg, "Failed for length #{len}"
      end)
    end

    test "round-trip survives exactly t errors at many positions" do
      msg = Enum.to_list(1..10)
      n_check = 4
      codeword = RS.encode(msg, n_check)
      n = length(codeword)

      # Test all pairs of positions — n_check=4 means t=2
      Enum.each(0..(n - 2), fn i ->
        Enum.each((i + 1)..(n - 1), fn j ->
          corrupted =
            codeword
            |> List.replace_at(i, Bitwise.bxor(Enum.at(codeword, i), 0xFF))
            |> List.replace_at(j, Bitwise.bxor(Enum.at(codeword, j), 0xAA))

          assert RS.decode(corrupted, n_check) == msg,
                 "Failed to correct errors at positions #{i}, #{j}"
        end)
      end)
    end
  end
end
