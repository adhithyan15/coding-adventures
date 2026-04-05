defmodule CodingAdventures.Sha256Test do
  @moduledoc """
  Tests for the SHA-256 implementation.

  Test vectors come from FIPS 180-4 (the official SHA-2 standard). Any correct
  SHA-256 implementation must produce exactly these digests for these inputs.
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.Sha256, as: S
  alias CodingAdventures.Sha256.Hasher

  # ─── FIPS 180-4 Test Vectors ───────────────────────────────────────────

  describe "FIPS 180-4 test vectors" do
    test "empty string" do
      assert S.sha256_hex("") ==
               "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    end

    test "'abc'" do
      assert S.sha256_hex("abc") ==
               "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    end

    test "448-bit (56-byte) message" do
      msg = "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
      assert byte_size(msg) == 56

      assert S.sha256_hex(msg) ==
               "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    end

    test "1,000,000 x 'a'" do
      data = String.duplicate("a", 1_000_000)

      assert S.sha256_hex(data) ==
               "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
    end
  end

  # ─── Output Format ─────────────────────────────────────────────────────

  describe "output format" do
    test "digest is 32 bytes" do
      assert byte_size(S.sha256("")) == 32
      assert byte_size(S.sha256("hello world")) == 32
      assert byte_size(S.sha256(:binary.copy(<<0>>, 1000))) == 32
    end

    test "hex string is 64 chars" do
      assert String.length(S.sha256_hex("")) == 64
      assert String.length(S.sha256_hex("hello")) == 64
    end

    test "hex string is lowercase" do
      hex = S.sha256_hex("abc")
      assert hex =~ ~r/^[0-9a-f]{64}$/
    end

    test "deterministic" do
      assert S.sha256("hello") == S.sha256("hello")
    end
  end

  # ─── Avalanche Effect ──────────────────────────────────────────────────

  describe "avalanche" do
    test "single-character difference flips many bits" do
      h1 = S.sha256("hello")
      h2 = S.sha256("helo")
      assert h1 != h2

      bits_different =
        :binary.bin_to_list(h1)
        |> Enum.zip(:binary.bin_to_list(h2))
        |> Enum.map(fn {a, b} ->
          xored = Bitwise.bxor(a, b)

          Enum.reduce(0..7, 0, fn bit, acc ->
            acc + Bitwise.band(Bitwise.bsr(xored, bit), 1)
          end)
        end)
        |> Enum.sum()

      assert bits_different > 40
    end
  end

  # ─── Block Boundaries ──────────────────────────────────────────────────

  describe "block boundaries" do
    test "55 bytes" do
      assert byte_size(S.sha256(:binary.copy(<<0>>, 55))) == 32
    end

    test "56 bytes" do
      assert byte_size(S.sha256(:binary.copy(<<0>>, 56))) == 32
    end

    test "55 and 56 differ" do
      assert S.sha256(:binary.copy(<<0>>, 55)) != S.sha256(:binary.copy(<<0>>, 56))
    end

    test "64 bytes" do
      assert byte_size(S.sha256(:binary.copy(<<0>>, 64))) == 32
    end

    test "128 bytes" do
      assert byte_size(S.sha256(:binary.copy(<<0>>, 128))) == 32
    end

    test "all boundary sizes produce distinct digests" do
      sizes = [55, 56, 63, 64, 127, 128]
      digests = Enum.map(sizes, fn n -> S.sha256(:binary.copy(<<0>>, n)) end)
      assert length(Enum.uniq(digests)) == 6
    end
  end

  # ─── Edge Cases ────────────────────────────────────────────────────────

  describe "edge cases" do
    test "null byte differs from empty" do
      assert S.sha256(<<0>>) != S.sha256("")
    end

    test "all 256 byte values" do
      data = :binary.list_to_bin(Enum.to_list(0..255))
      assert byte_size(S.sha256(data)) == 32
    end

    test "every single byte produces a unique digest" do
      digests =
        Enum.map(0..255, fn byte_val -> S.sha256(<<byte_val>>) end)
        |> Enum.uniq()

      assert length(digests) == 256
    end
  end

  # ─── Streaming API ─────────────────────────────────────────────────────

  describe "streaming API (Hasher)" do
    test "single write matches one-shot" do
      h = Hasher.new() |> Hasher.update("abc")
      assert Hasher.digest(h) == S.sha256("abc")
    end

    test "split at byte boundary" do
      h = Hasher.new() |> Hasher.update("ab") |> Hasher.update("c")
      assert Hasher.digest(h) == S.sha256("abc")
    end

    test "split at block boundary" do
      data = :binary.copy(<<0>>, 128)
      h = Hasher.new() |> Hasher.update(binary_part(data, 0, 64)) |> Hasher.update(binary_part(data, 64, 64))
      assert Hasher.digest(h) == S.sha256(data)
    end

    test "byte at a time" do
      data = :binary.list_to_bin(Enum.to_list(0..99))

      h =
        Enum.reduce(0..99, Hasher.new(), fn byte_val, acc ->
          Hasher.update(acc, <<byte_val>>)
        end)

      assert Hasher.digest(h) == S.sha256(data)
    end

    test "empty hasher matches empty one-shot" do
      h = Hasher.new()
      assert Hasher.digest(h) == S.sha256("")
    end

    test "digest is non-destructive" do
      h = Hasher.new() |> Hasher.update("abc")
      assert Hasher.digest(h) == Hasher.digest(h)
    end

    test "hex_digest returns correct string" do
      h = Hasher.new() |> Hasher.update("abc")

      assert Hasher.hex_digest(h) ==
               "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    end

    test "copy produces independent hasher" do
      h = Hasher.new() |> Hasher.update("ab")
      h2 = Hasher.copy(h)
      h2 = Hasher.update(h2, "c")
      h = Hasher.update(h, "x")
      assert Hasher.digest(h2) == S.sha256("abc")
      assert Hasher.digest(h) == S.sha256("abx")
    end

    test "streaming million a in chunks" do
      data = String.duplicate("a", 1_000_000)

      h =
        Hasher.new()
        |> Hasher.update(binary_part(data, 0, 500_000))
        |> Hasher.update(binary_part(data, 500_000, 500_000))

      assert Hasher.digest(h) == S.sha256(data)
    end
  end
end
