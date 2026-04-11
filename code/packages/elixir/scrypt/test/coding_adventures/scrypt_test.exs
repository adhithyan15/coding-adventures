defmodule CodingAdventures.ScryptTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.Scrypt

  # Helper: decode a lowercase hex string to a binary.
  defp from_hex(s), do: Base.decode16!(s, case: :lower)

  # ──────────────────────────────────────────────────────────────────────────────
  # RFC 7914 §12 Test Vectors
  #
  # These are the official test vectors from the scrypt RFC. Both must pass
  # for the implementation to be considered correct.
  # ──────────────────────────────────────────────────────────────────────────────

  describe "RFC 7914 test vectors" do
    @tag timeout: 60_000
    test "vector 1 — empty password, empty salt, n=16, r=1, p=1, dk_len=64" do
      # This vector uses empty password and salt — valid per the RFC.
      # scrypt does not prohibit empty password (unlike our user-facing PBKDF2).
      expected =
        from_hex(
          "77d6576238657b203b19ca42c18a0497" <>
            "f16b4844e3074ae8dfdffa3fede21442" <>
            "fcd0069ded0948f8326a753a0fc81f17" <>
            "e8d3e0fb2e0d3628cf35e20c38d18906"
        )

      result = Scrypt.scrypt("", "", 16, 1, 1, 64)
      assert result == expected
    end

    @tag timeout: 120_000
    test "vector 2 — password, NaCl salt, n=1024, r=8, p=16, dk_len=64" do
      # This vector is computationally heavier: n=1024, r=8, p=16.
      # Memory usage: 128 * 8 * 1024 = 1 MiB per block, 16 blocks = 16 MiB.
      expected =
        from_hex(
          "fdbabe1c9d3472007856e7190d01e9fe" <>
            "7c6ad7cbc8237830e77376634b373162" <>
            "2eaf30d92e22a3886ff109279d9830da" <>
            "c727afb94a83ee6d8360cbdfa2cc0640"
        )

      result = Scrypt.scrypt("password", "NaCl", 1024, 8, 16, 64)
      assert result == expected
    end
  end

  # ──────────────────────────────────────────────────────────────────────────────
  # Output Length Tests
  # ──────────────────────────────────────────────────────────────────────────────

  describe "output length" do
    test "dk_len=1 returns 1 byte" do
      result = Scrypt.scrypt("pass", "salt", 2, 1, 1, 1)
      assert byte_size(result) == 1
    end

    test "dk_len=16 returns 16 bytes" do
      result = Scrypt.scrypt("pass", "salt", 2, 1, 1, 16)
      assert byte_size(result) == 16
    end

    test "dk_len=32 returns 32 bytes" do
      result = Scrypt.scrypt("pass", "salt", 2, 1, 1, 32)
      assert byte_size(result) == 32
    end

    test "dk_len=64 returns 64 bytes" do
      result = Scrypt.scrypt("pass", "salt", 2, 1, 1, 64)
      assert byte_size(result) == 64
    end

    test "dk_len=100 returns 100 bytes" do
      result = Scrypt.scrypt("pass", "salt", 2, 1, 1, 100)
      assert byte_size(result) == 100
    end
  end

  # ──────────────────────────────────────────────────────────────────────────────
  # Determinism Test
  #
  # scrypt is a deterministic KDF: same inputs always produce the same output.
  # ──────────────────────────────────────────────────────────────────────────────

  describe "determinism" do
    test "same inputs always produce the same output" do
      result1 = Scrypt.scrypt("password", "salt", 4, 1, 1, 32)
      result2 = Scrypt.scrypt("password", "salt", 4, 1, 1, 32)
      assert result1 == result2
    end

    test "scrypt_hex is consistent with scrypt" do
      raw = Scrypt.scrypt("password", "salt", 4, 1, 1, 32)
      hex = Scrypt.scrypt_hex("password", "salt", 4, 1, 1, 32)
      assert hex == Base.encode16(raw, case: :lower)
    end
  end

  # ──────────────────────────────────────────────────────────────────────────────
  # Sensitivity Tests
  #
  # Different inputs must produce different outputs. This verifies that all
  # input parameters are actually used in the derivation.
  # ──────────────────────────────────────────────────────────────────────────────

  describe "different inputs produce different outputs" do
    test "different passwords" do
      r1 = Scrypt.scrypt("password1", "salt", 4, 1, 1, 32)
      r2 = Scrypt.scrypt("password2", "salt", 4, 1, 1, 32)
      assert r1 != r2
    end

    test "different salts" do
      r1 = Scrypt.scrypt("password", "salt1", 4, 1, 1, 32)
      r2 = Scrypt.scrypt("password", "salt2", 4, 1, 1, 32)
      assert r1 != r2
    end

    test "different N values" do
      r1 = Scrypt.scrypt("password", "salt", 2, 1, 1, 32)
      r2 = Scrypt.scrypt("password", "salt", 4, 1, 1, 32)
      assert r1 != r2
    end

    test "different r values" do
      r1 = Scrypt.scrypt("password", "salt", 4, 1, 1, 32)
      r2 = Scrypt.scrypt("password", "salt", 4, 2, 1, 32)
      assert r1 != r2
    end

    test "different p values" do
      r1 = Scrypt.scrypt("password", "salt", 4, 1, 1, 32)
      r2 = Scrypt.scrypt("password", "salt", 4, 1, 2, 32)
      assert r1 != r2
    end
  end

  # ──────────────────────────────────────────────────────────────────────────────
  # Hex Variant Tests
  # ──────────────────────────────────────────────────────────────────────────────

  describe "scrypt_hex/6" do
    test "returns lowercase hex string" do
      result = Scrypt.scrypt_hex("pass", "salt", 4, 1, 1, 16)
      assert is_binary(result)
      # Hex string for 16 bytes should be 32 hex characters
      assert String.length(result) == 32
      # All chars should be valid hex
      assert result =~ ~r/^[0-9a-f]+$/
    end

    test "matches Base.encode16 of raw output" do
      raw = Scrypt.scrypt("pass", "salt", 4, 1, 1, 32)
      hex = Scrypt.scrypt_hex("pass", "salt", 4, 1, 1, 32)
      assert hex == Base.encode16(raw, case: :lower)
    end
  end

  # ──────────────────────────────────────────────────────────────────────────────
  # Validation Error Tests
  # ──────────────────────────────────────────────────────────────────────────────

  describe "validation errors" do
    test "N not a power of 2 raises ArgumentError" do
      assert_raise ArgumentError, ~r/N must be a power of 2/, fn ->
        Scrypt.scrypt("pass", "salt", 3, 1, 1, 32)
      end
    end

    test "N = 1 raises ArgumentError (not >= 2)" do
      assert_raise ArgumentError, ~r/N must be a power of 2/, fn ->
        Scrypt.scrypt("pass", "salt", 1, 1, 1, 32)
      end
    end

    test "N = 0 raises ArgumentError" do
      assert_raise ArgumentError, ~r/N must be a power of 2/, fn ->
        Scrypt.scrypt("pass", "salt", 0, 1, 1, 32)
      end
    end

    test "N negative raises ArgumentError" do
      assert_raise ArgumentError, ~r/N must be a power of 2/, fn ->
        Scrypt.scrypt("pass", "salt", -4, 1, 1, 32)
      end
    end

    test "N > 2^20 raises ArgumentError" do
      assert_raise ArgumentError, ~r/N must not exceed 2\^20/, fn ->
        Scrypt.scrypt("pass", "salt", 2_097_152, 1, 1, 32)
      end
    end

    test "r = 0 raises ArgumentError" do
      assert_raise ArgumentError, ~r/r must be a positive integer/, fn ->
        Scrypt.scrypt("pass", "salt", 2, 0, 1, 32)
      end
    end

    test "r negative raises ArgumentError" do
      assert_raise ArgumentError, ~r/r must be a positive integer/, fn ->
        Scrypt.scrypt("pass", "salt", 2, -1, 1, 32)
      end
    end

    test "p = 0 raises ArgumentError" do
      assert_raise ArgumentError, ~r/p must be a positive integer/, fn ->
        Scrypt.scrypt("pass", "salt", 2, 1, 0, 32)
      end
    end

    test "p negative raises ArgumentError" do
      assert_raise ArgumentError, ~r/p must be a positive integer/, fn ->
        Scrypt.scrypt("pass", "salt", 2, 1, -1, 32)
      end
    end

    test "dk_len = 0 raises ArgumentError" do
      assert_raise ArgumentError, ~r/dk_len must be between/, fn ->
        Scrypt.scrypt("pass", "salt", 2, 1, 1, 0)
      end
    end

    test "dk_len negative raises ArgumentError" do
      assert_raise ArgumentError, ~r/dk_len must be between/, fn ->
        Scrypt.scrypt("pass", "salt", 2, 1, 1, -1)
      end
    end

    test "dk_len > 2^20 raises ArgumentError" do
      assert_raise ArgumentError, ~r/dk_len must be between/, fn ->
        Scrypt.scrypt("pass", "salt", 2, 1, 1, 1_048_577)
      end
    end

    test "p * r > 2^30 raises ArgumentError" do
      # p=2, r=2^30 should overflow the limit
      # 2 * 1_073_741_824 = 2_147_483_648 > 2^30 = 1_073_741_824
      assert_raise ArgumentError, ~r/p \* r exceeds limit/, fn ->
        Scrypt.scrypt("pass", "salt", 2, 1_073_741_825, 1, 32)
      end
    end
  end

  # ──────────────────────────────────────────────────────────────────────────────
  # Edge Cases
  # ──────────────────────────────────────────────────────────────────────────────

  describe "edge cases" do
    test "empty password is allowed (RFC 7914 vector 1)" do
      # RFC 7914 explicitly tests with empty password — scrypt must allow it
      result = Scrypt.scrypt("", "salt", 2, 1, 1, 32)
      assert byte_size(result) == 32
    end

    test "empty salt is allowed (RFC 7914 vector 1)" do
      result = Scrypt.scrypt("password", "", 2, 1, 1, 32)
      assert byte_size(result) == 32
    end

    test "minimum valid N=2" do
      result = Scrypt.scrypt("pass", "salt", 2, 1, 1, 32)
      assert byte_size(result) == 32
    end

    test "dk_len=1 (minimum)" do
      result = Scrypt.scrypt("pass", "salt", 2, 1, 1, 1)
      assert byte_size(result) == 1
    end

    test "binary password with null bytes" do
      result = Scrypt.scrypt("pass\x00word", "salt", 2, 1, 1, 32)
      assert byte_size(result) == 32
    end

    test "binary salt with null bytes" do
      result = Scrypt.scrypt("password", "sa\x00lt", 2, 1, 1, 32)
      assert byte_size(result) == 32
    end
  end
end
