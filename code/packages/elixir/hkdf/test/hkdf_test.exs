defmodule CodingAdventures.HkdfTest do
  @moduledoc """
  Tests for CodingAdventures.Hkdf — RFC 5869 HKDF.

  All three test vectors from RFC 5869 Appendix A, plus edge cases
  for error handling and SHA-512 support.
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.Hkdf

  # Shorthand: decode hex to binary.
  defp h(hex_str), do: Base.decode16!(hex_str, case: :mixed)
  defp to_hex(bin), do: Base.encode16(bin, case: :lower)

  # =========================================================================
  # RFC 5869 Test Vectors — HKDF-SHA256
  # =========================================================================

  describe "RFC 5869 Test Case 1: basic SHA-256" do
    # All three parameters (salt, IKM, info) are non-empty.
    # Output length (42) is not a multiple of HashLen (32), testing truncation.

    setup do
      %{
        ikm: h("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b"),
        salt: h("000102030405060708090a0b0c"),
        info: h("f0f1f2f3f4f5f6f7f8f9"),
        expected_prk: "077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844ad7c2b3e5",
        expected_okm:
          "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865"
      }
    end

    test "extract produces correct PRK", ctx do
      prk = Hkdf.extract(ctx.salt, ctx.ikm)
      assert to_hex(prk) == ctx.expected_prk
    end

    test "expand produces correct OKM", ctx do
      prk = h(ctx.expected_prk)
      okm = Hkdf.expand(prk, ctx.info, 42)
      assert to_hex(okm) == ctx.expected_okm
    end

    test "combined hkdf produces correct OKM", ctx do
      okm = Hkdf.hkdf(ctx.salt, ctx.ikm, ctx.info, 42)
      assert to_hex(okm) == ctx.expected_okm
    end

    test "hex convenience functions work", ctx do
      assert Hkdf.extract_hex(ctx.salt, ctx.ikm) == ctx.expected_prk
      assert Hkdf.hkdf_hex(ctx.salt, ctx.ikm, ctx.info, 42) == ctx.expected_okm
    end
  end

  describe "RFC 5869 Test Case 2: longer inputs" do
    # 80-byte IKM, salt, and info.
    # L = 82 requires ceil(82/32) = 3 HMAC iterations in expand.

    setup do
      %{
        ikm:
          h(
            "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f404142434445464748494a4b4c4d4e4f"
          ),
        salt:
          h(
            "606162636465666768696a6b6c6d6e6f707172737475767778797a7b7c7d7e7f808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9fa0a1a2a3a4a5a6a7a8a9aaabacadaeaf"
          ),
        info:
          h(
            "b0b1b2b3b4b5b6b7b8b9babbbcbdbebfc0c1c2c3c4c5c6c7c8c9cacbcccdcecfd0d1d2d3d4d5d6d7d8d9dadbdcdddedfe0e1e2e3e4e5e6e7e8e9eaebecedeeeff0f1f2f3f4f5f6f7f8f9fafbfcfdfeff"
          ),
        expected_prk: "06a6b88c5853361a06104c9ceb35b45cef760014904671014a193f40c15fc244",
        expected_okm:
          "b11e398dc80327a1c8e7f78c596a49344f012eda2d4efad8a050cc4c19afa97c59045a99cac7827271cb41c65e590e09da3275600c2f09b8367793a9aca3db71cc30c58179ec3e87c14c01d5c1f3434f1d87"
      }
    end

    test "extract produces correct PRK", ctx do
      prk = Hkdf.extract(ctx.salt, ctx.ikm)
      assert to_hex(prk) == ctx.expected_prk
    end

    test "expand produces correct OKM", ctx do
      prk = h(ctx.expected_prk)
      okm = Hkdf.expand(prk, ctx.info, 82)
      assert to_hex(okm) == ctx.expected_okm
    end

    test "combined hkdf produces correct OKM", ctx do
      okm = Hkdf.hkdf(ctx.salt, ctx.ikm, ctx.info, 82)
      assert to_hex(okm) == ctx.expected_okm
    end
  end

  describe "RFC 5869 Test Case 3: empty salt and info" do
    # When salt is empty, HKDF uses HashLen (32) zero bytes as the HMAC key.
    # When info is empty, the expand loop appends only the counter byte.

    setup do
      %{
        ikm: h("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b"),
        salt: "",
        info: "",
        expected_prk: "19ef24a32c717b167f33a91d6f648bdf96596776afdb6377ac434c1c293ccb04",
        expected_okm:
          "8da4e775a563c18f715f802a063c5a31b8a11f5c5ee1879ec3454e5f3c738d2d9d201395faa4b61a96c8"
      }
    end

    test "extract produces correct PRK", ctx do
      prk = Hkdf.extract(ctx.salt, ctx.ikm)
      assert to_hex(prk) == ctx.expected_prk
    end

    test "expand produces correct OKM", ctx do
      prk = h(ctx.expected_prk)
      okm = Hkdf.expand(prk, ctx.info, 42)
      assert to_hex(okm) == ctx.expected_okm
    end

    test "combined hkdf produces correct OKM", ctx do
      okm = Hkdf.hkdf(ctx.salt, ctx.ikm, ctx.info, 42)
      assert to_hex(okm) == ctx.expected_okm
    end
  end

  # =========================================================================
  # Edge Cases
  # =========================================================================

  describe "edge cases" do
    test "default hash is sha256" do
      ikm = h("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b")
      salt = h("000102030405060708090a0b0c")
      info = h("f0f1f2f3f4f5f6f7f8f9")

      expected_okm =
        "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865"

      # Omit hash parameter — should default to sha256
      okm = Hkdf.hkdf(salt, ikm, info, 42)
      assert to_hex(okm) == expected_okm
    end

    test "expand rejects length <= 0" do
      prk = :binary.copy(<<0x01>>, 32)

      assert_raise ArgumentError, fn ->
        Hkdf.expand(prk, "", 0)
      end
    end

    test "expand rejects length > 255 * HashLen" do
      prk = :binary.copy(<<0x01>>, 32)

      # SHA-256: max = 255 * 32 = 8160
      assert_raise ArgumentError, fn ->
        Hkdf.expand(prk, "", 8161)
      end
    end

    test "expand allows length = 255 * HashLen exactly" do
      prk = :binary.copy(<<0x01>>, 32)
      okm = Hkdf.expand(prk, "", 8160)
      assert byte_size(okm) == 8160
    end

    test "expand with length = 1" do
      prk = :binary.copy(<<0x01>>, 32)
      okm = Hkdf.expand(prk, "", 1)
      assert byte_size(okm) == 1
    end

    test "expand with length = HashLen" do
      prk = :binary.copy(<<0x01>>, 32)
      okm = Hkdf.expand(prk, "test", 32)
      assert byte_size(okm) == 32
    end

    test "rejects unsupported hash algorithm" do
      assert_raise ArgumentError, fn ->
        Hkdf.extract("salt", "ikm", :md5)
      end
    end

    test "SHA-512 extract and expand" do
      ikm = h("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b")
      salt = h("000102030405060708090a0b0c")

      prk = Hkdf.extract(salt, ikm, :sha512)
      assert byte_size(prk) == 64

      okm = Hkdf.expand(prk, "info", 64, :sha512)
      assert byte_size(okm) == 64
    end
  end
end
