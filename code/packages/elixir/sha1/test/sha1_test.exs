defmodule CodingAdventures.Sha1Test do
  @moduledoc """
  Tests for the SHA-1 implementation.

  Test vectors come from FIPS 180-4 (the official SHA-1 standard). Any correct
  SHA-1 implementation must produce exactly these digests for these inputs.

  We test both the one-shot sha1/1 function and basic output format properties,
  plus edge cases like empty input, block boundaries, and large inputs.
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.Sha1, as: S

  # ─── FIPS 180-4 Test Vectors ───────────────────────────────────────────────

  describe "FIPS 180-4 test vectors" do
    test "empty string" do
      assert S.sha1_hex("") == "da39a3ee5e6b4b0d3255bfef95601890afd80709"
    end

    test "'abc'" do
      assert S.sha1_hex("abc") == "a9993e364706816aba3e25717850c26c9cd0d89d"
    end

    test "448-bit (56-byte) message" do
      msg = "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
      assert byte_size(msg) == 56
      assert S.sha1_hex(msg) == "84983e441c3bd26ebaae4aa1f95129e5e54670f1"
    end

    test "one million 'a' characters" do
      data = String.duplicate("a", 1_000_000)
      assert S.sha1_hex(data) == "34aa973cd4c4daa4f61eeb2bdbad27316534016f"
    end
  end

  # ─── Output Format ─────────────────────────────────────────────────────────

  describe "output format" do
    test "sha1/1 returns a 20-byte binary" do
      result = S.sha1("test")
      assert is_binary(result)
      assert byte_size(result) == 20
    end

    test "sha1_hex/1 returns a 40-character string" do
      assert String.length(S.sha1_hex("")) == 40
      assert String.length(S.sha1_hex("hello")) == 40
    end

    test "sha1_hex is lowercase" do
      hex = S.sha1_hex("abc")
      assert hex == String.downcase(hex)
      assert String.match?(hex, ~r/\A[0-9a-f]+\z/)
    end

    test "deterministic — same input same output" do
      assert S.sha1("hello") == S.sha1("hello")
    end

    test "avalanche — one-byte change flips many bits" do
      h1 = S.sha1("hello")
      h2 = S.sha1("helo")
      refute h1 == h2
      # XOR the digests; count differing bits
      xor_bytes =
        Enum.zip(:binary.bin_to_list(h1), :binary.bin_to_list(h2))
        |> Enum.map(fn {a, b} -> Bitwise.bxor(a, b) end)
      bits_different =
        Enum.sum(Enum.map(xor_bytes, fn b ->
          Integer.to_string(b, 2) |> String.graphemes() |> Enum.count(&(&1 == "1"))
        end))
      assert bits_different > 20
    end
  end

  # ─── Block Boundary Tests ──────────────────────────────────────────────────
  #
  # SHA-1 processes 64-byte blocks. Block boundaries are the most common source
  # of bugs:
  #
  #   55 bytes: fits in one block (55 + 1 + 8 = 64)
  #   56 bytes: overflows — padding needs a second block
  #   64 bytes: one data block + one full padding block

  describe "block boundaries" do
    test "55 bytes — exactly one block after padding" do
      r = S.sha1(:binary.copy(<<0>>, 55))
      assert byte_size(r) == 20
      assert r == S.sha1(:binary.copy(<<0>>, 55))
    end

    test "56 bytes — requires second padding block" do
      assert byte_size(S.sha1(:binary.copy(<<0>>, 56))) == 20
    end

    test "55 and 56 bytes produce different digests" do
      refute S.sha1(:binary.copy(<<0>>, 55)) == S.sha1(:binary.copy(<<0>>, 56))
    end

    test "64 bytes" do
      assert byte_size(S.sha1(:binary.copy(<<0>>, 64))) == 20
    end

    test "127 bytes" do
      assert byte_size(S.sha1(:binary.copy(<<0>>, 127))) == 20
    end

    test "128 bytes" do
      assert byte_size(S.sha1(:binary.copy(<<0>>, 128))) == 20
    end

    test "all boundary sizes produce distinct digests" do
      sizes = [55, 56, 63, 64, 127, 128]
      digests = Enum.map(sizes, fn n -> S.sha1_hex(:binary.copy(<<0>>, n)) end)
      assert length(Enum.uniq(digests)) == 6
    end
  end

  # ─── Edge Cases ────────────────────────────────────────────────────────────

  describe "edge cases" do
    test "single null byte differs from empty" do
      r = S.sha1(<<0>>)
      assert byte_size(r) == 20
      refute r == S.sha1("")
    end

    test "single 0xFF byte" do
      assert byte_size(S.sha1(<<0xFF>>)) == 20
    end

    test "all 256 byte values" do
      data = Enum.reduce(0..255, <<>>, fn i, acc -> acc <> <<i>> end)
      assert byte_size(S.sha1(data)) == 20
    end

    test "every single-byte input produces a unique digest" do
      digests =
        Enum.map(0..255, fn i -> S.sha1_hex(<<i>>) end)
        |> Enum.uniq()
      assert length(digests) == 256
    end

    test "1000 zero bytes" do
      assert byte_size(S.sha1(:binary.copy(<<0>>, 1000))) == 20
    end

    test "sha1_hex matches Base.encode16 of sha1" do
      data = "hello"
      assert S.sha1_hex(data) == Base.encode16(S.sha1(data), case: :lower)
    end
  end
end
