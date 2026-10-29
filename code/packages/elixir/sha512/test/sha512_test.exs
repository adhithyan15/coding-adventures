defmodule CodingAdventures.Sha512Test do
  import Bitwise

  @moduledoc """
  Tests for the SHA-512 implementation.

  Test vectors come from FIPS 180-4 (the official SHA-2 standard). Any correct
  SHA-512 implementation must produce exactly these digests for these inputs.

  We test the one-shot sha512/1 function, output format properties, block
  boundary edge cases, and large inputs.
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.Sha512, as: S

  @fixture Path.expand("../../../../specs/fixtures/sha512-v1/cases.txt", __DIR__)

  defp fixture_lines do
    lines = File.read!(@fixture) |> parse_fixture()
    assert Enum.count(lines, &String.starts_with?(&1, "V|")) == 9
    assert Enum.count(lines, &String.starts_with?(&1, "L|")) == 6
    lines
  end

  defp parse_fixture(raw) do
    if byte_size(raw) not in 1..4096 or not String.valid?(raw),
      do: raise(ArgumentError, "invalid fixture bytes")

    lines =
      raw
      |> String.split("\n", trim: true)
      |> Enum.map(&String.trim_trailing(&1, "\r"))
      |> Enum.reject(&String.starts_with?(&1, "#"))

    if length(lines) not in 1..64, do: raise(ArgumentError, "invalid fixture row count")
    Enum.each(lines, &validate_fixture_row/1)
    lines
  end

  defp validate_fixture_row(line) do
    case String.split(String.trim_trailing(line, "\r"), "|", trim: false) do
      ["V", id, input, chunks, digest] ->
        unless id =~ ~r/\A[a-z0-9]+(?:-[a-z0-9]+)*\z/ and byte_size(input) <= 8192 and
                 input =~ ~r/\A(?:[0-9a-f]{2})*\z/ and digest =~ ~r/\A[0-9a-f]{128}\z/ do
          raise ArgumentError, "malformed vector row"
        end

        byte_count = div(byte_size(input), 2)

        parsed =
          cond do
            chunks == "" ->
              []

            chunks =~ ~r/\A[1-9][0-9]*(?:,[1-9][0-9]*)*\z/ ->
              String.split(chunks, ",")
              |> Enum.map(fn raw ->
                {size, ""} = Integer.parse(raw)
                if size > 4096, do: raise(ArgumentError, "malformed chunk"), else: size
              end)

            true ->
              raise ArgumentError, "malformed chunks"
          end

        if byte_count > 4096 or byte_count == 0 != (chunks == "") or length(parsed) > 64 or
             Enum.sum(parsed) != byte_count,
           do: raise(ArgumentError, "malformed vector size")

      ["L", id, high, low, expected] ->
        unless id =~ ~r/\A[a-z0-9]+(?:-[a-z0-9]+)*\z/ and high =~ ~r/\A[0-9a-f]{16}\z/ and
                 low =~ ~r/\A[0-9a-f]{16}\z/ and
                 (expected == "ERROR" or expected =~ ~r/\A[0-9a-f]{32}\z/) do
          raise ArgumentError, "malformed length row"
        end

      _ ->
        raise ArgumentError, "unknown or malformed fixture row"
    end
  end

  test "fixture reader rejects malformed rows" do
    digest = String.duplicate("0", 128)

    for row <- [
          "X|id|||",
          "V|UPPER|00|1|#{digest}",
          "V|bad|0|1|#{digest}",
          "V|bad|00|2|#{digest}",
          "L|bad|000000000000000G|0000000000000000|ERROR",
          "L|bad|0000000000000000|0000000000000000|ABC"
        ] do
      assert_raise ArgumentError, fn -> parse_fixture(row) end
    end
  end

  test "HF04 portable digest corpus" do
    for line <- Enum.filter(fixture_lines(), &String.starts_with?(&1, "V|")) do
      ["V", id, input_hex, _chunks, expected] = String.split(line, "|", trim: false)
      data = Base.decode16!(input_hex, case: :lower)
      assert S.sha512_hex(data) == expected, id
    end
  end

  test "HF04 portable length corpus" do
    for line <- Enum.filter(fixture_lines(), &String.starts_with?(&1, "L|")) do
      ["L", id, high_hex, low_hex, expected] = String.split(line, "|", trim: false)
      {high, ""} = Integer.parse(high_hex, 16)
      {low, ""} = Integer.parse(low_hex, 16)
      byte_count = high <<< 64 ||| low

      if expected == "ERROR" do
        assert_raise ArgumentError, ~r/less than 2\^128 bits/, fn ->
          S.encode_bit_length(byte_count)
        end
      else
        assert Base.encode16(S.encode_bit_length(byte_count), case: :lower) == expected, id
      end
    end
  end

  # ─── FIPS 180-4 Test Vectors ─────────────────────────────────────────────

  describe "FIPS 180-4 test vectors" do
    test "empty string" do
      assert S.sha512_hex("") ==
               "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce" <>
                 "47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e"
    end

    test "'abc'" do
      assert S.sha512_hex("abc") ==
               "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a" <>
                 "2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
    end

    test "896-bit (112-byte) message" do
      msg =
        "abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmn" <>
          "hijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu"

      assert byte_size(msg) == 112

      assert S.sha512_hex(msg) ==
               "8e959b75dae313da8cf4f72814fc143f8f7779c6eb9f7fa17299aeadb6889018" <>
                 "501d289e4900f7e4331b99dec4b5433ac7d329eeb6dd26545e96e55b874be909"
    end

    test "one million 'a' characters" do
      data = String.duplicate("a", 1_000_000)

      assert S.sha512_hex(data) ==
               "e718483d0ce769644e2e42c7bc15b4638e1f98b13b2044285632a803afa973eb" <>
                 "de0ff244877ea60a4cb0432ce577c31beb009c5c2c49aa2e4eadb217ad8cc09b"
    end
  end

  # ─── Output Format ───────────────────────────────────────────────────────

  describe "output format" do
    test "sha512/1 returns a 64-byte binary" do
      result = S.sha512("test")
      assert is_binary(result)
      assert byte_size(result) == 64
    end

    test "sha512_hex/1 returns a 128-character string" do
      assert String.length(S.sha512_hex("")) == 128
      assert String.length(S.sha512_hex("hello")) == 128
    end

    test "sha512_hex is lowercase" do
      hex = S.sha512_hex("abc")
      assert hex == String.downcase(hex)
      assert String.match?(hex, ~r/\A[0-9a-f]+\z/)
    end

    test "deterministic — same input same output" do
      assert S.sha512("hello") == S.sha512("hello")
    end

    test "avalanche — one-byte change flips many bits" do
      h1 = S.sha512("hello")
      h2 = S.sha512("helo")
      refute h1 == h2
      # XOR the digests; count differing bits
      xor_bytes =
        Enum.zip(:binary.bin_to_list(h1), :binary.bin_to_list(h2))
        |> Enum.map(fn {a, b} -> Bitwise.bxor(a, b) end)

      bits_different =
        Enum.sum(
          Enum.map(xor_bytes, fn b ->
            Integer.to_string(b, 2)
            |> String.graphemes()
            |> Enum.count(&(&1 == "1"))
          end)
        )

      assert bits_different > 100
    end
  end

  # ─── Block Boundary Tests ────────────────────────────────────────────────
  #
  # SHA-512 processes 128-byte blocks. Key boundaries:
  #
  #   111 bytes: fits in one block (111 + 1 + 16 = 128)
  #   112 bytes: overflows — padding needs a second block
  #   128 bytes: one data block + one full padding block

  describe "block boundaries" do
    test "111 bytes — exactly one block after padding" do
      r = S.sha512(:binary.copy(<<0>>, 111))
      assert byte_size(r) == 64
      assert r == S.sha512(:binary.copy(<<0>>, 111))
    end

    test "112 bytes — requires second padding block" do
      assert byte_size(S.sha512(:binary.copy(<<0>>, 112))) == 64
    end

    test "111 and 112 bytes produce different digests" do
      refute S.sha512(:binary.copy(<<0>>, 111)) == S.sha512(:binary.copy(<<0>>, 112))
    end

    test "128 bytes" do
      assert byte_size(S.sha512(:binary.copy(<<0>>, 128))) == 64
    end

    test "255 bytes" do
      assert byte_size(S.sha512(:binary.copy(<<0>>, 255))) == 64
    end

    test "256 bytes" do
      assert byte_size(S.sha512(:binary.copy(<<0>>, 256))) == 64
    end

    test "all boundary sizes produce distinct digests" do
      sizes = [111, 112, 127, 128, 255, 256]
      digests = Enum.map(sizes, fn n -> S.sha512_hex(:binary.copy(<<0>>, n)) end)
      assert length(Enum.uniq(digests)) == 6
    end
  end

  # ─── Edge Cases ──────────────────────────────────────────────────────────

  describe "edge cases" do
    test "single null byte differs from empty" do
      r = S.sha512(<<0>>)
      assert byte_size(r) == 64
      refute r == S.sha512("")
    end

    test "single 0xFF byte" do
      assert byte_size(S.sha512(<<0xFF>>)) == 64
    end

    test "all 256 byte values" do
      data = Enum.reduce(0..255, <<>>, fn i, acc -> acc <> <<i>> end)
      assert byte_size(S.sha512(data)) == 64
    end

    test "every single-byte input produces a unique digest" do
      digests =
        Enum.map(0..255, fn i -> S.sha512_hex(<<i>>) end)
        |> Enum.uniq()

      assert length(digests) == 256
    end

    test "1000 zero bytes" do
      assert byte_size(S.sha512(:binary.copy(<<0>>, 1000))) == 64
    end

    test "sha512_hex matches Base.encode16 of sha512" do
      data = "hello"
      assert S.sha512_hex(data) == Base.encode16(S.sha512(data), case: :lower)
    end
  end
end
