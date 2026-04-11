defmodule CodingAdventures.Pbkdf2Test do
  use ExUnit.Case, async: true

  alias CodingAdventures.Pbkdf2

  defp from_hex(s), do: Base.decode16!(s, case: :lower)

  # ──────────────────────────────────────────────────────────────────────────────
  # RFC 6070 — PBKDF2-HMAC-SHA1
  # ──────────────────────────────────────────────────────────────────────────────

  describe "RFC 6070 PBKDF2-HMAC-SHA1" do
    test "vector 1 — c=1, dkLen=20" do
      dk = Pbkdf2.pbkdf2_hmac_sha1("password", "salt", 1, 20)
      assert dk == from_hex("0c60c80f961f0e71f3a9b524af6012062fe037a6")
    end

    test "vector 2 — c=4096, dkLen=20" do
      dk = Pbkdf2.pbkdf2_hmac_sha1("password", "salt", 4096, 20)
      assert dk == from_hex("4b007901b765489abead49d926f721d065a429c1")
    end

    test "vector 3 — long password and salt" do
      dk =
        Pbkdf2.pbkdf2_hmac_sha1(
          "passwordPASSWORDpassword",
          "saltSALTsaltSALTsaltSALTsaltSALTsalt",
          4096,
          25
        )

      assert dk == from_hex("3d2eec4fe41c849b80c8d83662c0e44a8b291a964cf2f07038")
    end

    test "vector 4 — null bytes in password and salt" do
      dk = Pbkdf2.pbkdf2_hmac_sha1("pass\x00word", "sa\x00lt", 4096, 16)
      assert dk == from_hex("56fa6aa75548099dcc37d7f03425e0c3")
    end
  end

  # ──────────────────────────────────────────────────────────────────────────────
  # RFC 7914 — PBKDF2-HMAC-SHA256
  # ──────────────────────────────────────────────────────────────────────────────

  describe "RFC 7914 PBKDF2-HMAC-SHA256" do
    test "vector 1 — c=1, dkLen=64" do
      dk = Pbkdf2.pbkdf2_hmac_sha256("passwd", "salt", 1, 64)

      expected =
        from_hex(
          "55ac046e56e3089fec1691c22544b605" <>
            "f94185216dde0465e68b9d57c20dacbc" <>
            "49ca9cccf179b645991664b39d77ef31" <>
            "7c71b845b1e30bd509112041d3a19783"
        )

      assert dk == expected
    end

    test "output length matches requested key_length" do
      dk = Pbkdf2.pbkdf2_hmac_sha256("key", "salt", 1, 32)
      assert byte_size(dk) == 32
    end

    test "truncation is consistent with prefix of longer key" do
      short = Pbkdf2.pbkdf2_hmac_sha256("key", "salt", 1, 16)
      full = Pbkdf2.pbkdf2_hmac_sha256("key", "salt", 1, 32)
      assert short == binary_part(full, 0, 16)
    end

    test "multi-block: first 32 bytes match single-block result" do
      dk64 = Pbkdf2.pbkdf2_hmac_sha256("password", "salt", 1, 64)
      dk32 = Pbkdf2.pbkdf2_hmac_sha256("password", "salt", 1, 32)
      assert byte_size(dk64) == 64
      assert binary_part(dk64, 0, 32) == dk32
    end
  end

  # ──────────────────────────────────────────────────────────────────────────────
  # SHA-512 sanity checks
  # ──────────────────────────────────────────────────────────────────────────────

  describe "PBKDF2-HMAC-SHA512" do
    test "output length" do
      assert byte_size(Pbkdf2.pbkdf2_hmac_sha512("secret", "nacl", 1, 64)) == 64
    end

    test "truncation consistent" do
      short = Pbkdf2.pbkdf2_hmac_sha512("secret", "nacl", 1, 32)
      full = Pbkdf2.pbkdf2_hmac_sha512("secret", "nacl", 1, 64)
      assert short == binary_part(full, 0, 32)
    end

    test "multi-block 128 bytes" do
      assert byte_size(Pbkdf2.pbkdf2_hmac_sha512("key", "salt", 1, 128)) == 128
    end
  end

  # ──────────────────────────────────────────────────────────────────────────────
  # Hex variants
  # ──────────────────────────────────────────────────────────────────────────────

  describe "hex variants" do
    test "SHA1 hex matches RFC 6070 vector 1" do
      assert Pbkdf2.pbkdf2_hmac_sha1_hex("password", "salt", 1, 20) ==
               "0c60c80f961f0e71f3a9b524af6012062fe037a6"
    end

    test "SHA256 hex matches bytes" do
      raw = Pbkdf2.pbkdf2_hmac_sha256("passwd", "salt", 1, 32)
      hex = Pbkdf2.pbkdf2_hmac_sha256_hex("passwd", "salt", 1, 32)
      assert hex == Base.encode16(raw, case: :lower)
    end

    test "SHA512 hex matches bytes" do
      raw = Pbkdf2.pbkdf2_hmac_sha512("secret", "nacl", 1, 64)
      hex = Pbkdf2.pbkdf2_hmac_sha512_hex("secret", "nacl", 1, 64)
      assert hex == Base.encode16(raw, case: :lower)
    end
  end

  # ──────────────────────────────────────────────────────────────────────────────
  # Validation
  # ──────────────────────────────────────────────────────────────────────────────

  describe "validation" do
    test "empty password raises ArgumentError" do
      assert_raise ArgumentError, "PBKDF2 password must not be empty", fn ->
        Pbkdf2.pbkdf2_hmac_sha256("", "salt", 1, 32)
      end
    end

    test "empty password SHA1 raises ArgumentError" do
      assert_raise ArgumentError, "PBKDF2 password must not be empty", fn ->
        Pbkdf2.pbkdf2_hmac_sha1("", "salt", 1, 20)
      end
    end

    test "zero iterations raises ArgumentError" do
      assert_raise ArgumentError, "PBKDF2 iterations must be positive", fn ->
        Pbkdf2.pbkdf2_hmac_sha256("pw", "salt", 0, 32)
      end
    end

    test "negative iterations raises ArgumentError" do
      assert_raise ArgumentError, "PBKDF2 iterations must be positive", fn ->
        Pbkdf2.pbkdf2_hmac_sha256("pw", "salt", -1, 32)
      end
    end

    test "zero key_length raises ArgumentError" do
      assert_raise ArgumentError, "PBKDF2 key_length must be positive", fn ->
        Pbkdf2.pbkdf2_hmac_sha256("pw", "salt", 1, 0)
      end
    end

    test "empty salt is allowed" do
      dk = Pbkdf2.pbkdf2_hmac_sha256("password", "", 1, 32)
      assert byte_size(dk) == 32
    end

    test "is deterministic" do
      a = Pbkdf2.pbkdf2_hmac_sha256("secret", "nacl", 100, 32)
      b = Pbkdf2.pbkdf2_hmac_sha256("secret", "nacl", 100, 32)
      assert a == b
    end

    test "different salts produce different keys" do
      a = Pbkdf2.pbkdf2_hmac_sha256("password", "salt1", 1, 32)
      b = Pbkdf2.pbkdf2_hmac_sha256("password", "salt2", 1, 32)
      refute a == b
    end

    test "different passwords produce different keys" do
      a = Pbkdf2.pbkdf2_hmac_sha256("password1", "salt", 1, 32)
      b = Pbkdf2.pbkdf2_hmac_sha256("password2", "salt", 1, 32)
      refute a == b
    end

    test "different iterations produce different keys" do
      a = Pbkdf2.pbkdf2_hmac_sha256("password", "salt", 1, 32)
      b = Pbkdf2.pbkdf2_hmac_sha256("password", "salt", 2, 32)
      refute a == b
    end
  end
end
