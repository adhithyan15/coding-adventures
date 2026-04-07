defmodule CodingAdventures.HmacTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.Hmac

  # ===========================================================================
  # RFC 4231 Test Vectors — HMAC-SHA256
  # ===========================================================================
  # All test vectors from RFC 4231 Section 4.

  describe "hmac_sha256/2 — RFC 4231 vectors" do
    test "TC1: short key, short data" do
      key = :binary.copy(<<0x0B>>, 20)
      data = "Hi There"

      assert Hmac.hmac_sha256_hex(key, data) ==
               "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
    end

    test "TC2: key = 'Jefe'" do
      assert Hmac.hmac_sha256_hex("Jefe", "what do ya want for nothing?") ==
               "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
    end

    test "TC3: key and data are repeated bytes" do
      key = :binary.copy(<<0xAA>>, 20)
      data = :binary.copy(<<0xDD>>, 50)

      assert Hmac.hmac_sha256_hex(key, data) ==
               "773ea91e36800e46854db8ebd09181a72959098b3ef8c122d9635514ced565fe"
    end

    test "TC4: longer key" do
      key = Base.decode16!("0102030405060708090a0b0c0d0e0f10111213141516171819", case: :lower)
      data = :binary.copy(<<0xCD>>, 50)

      assert Hmac.hmac_sha256_hex(key, data) ==
               "82558a389a443c0ea4cc819899f2083a85f0faa3e578f8077a2e3ff46729665b"
    end

    test "TC6: key longer than block size" do
      key = :binary.copy(<<0xAA>>, 131)
      data = "Test Using Larger Than Block-Size Key - Hash Key First"

      assert Hmac.hmac_sha256_hex(key, data) ==
               "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54"
    end

    test "TC7: key longer than block size, longer data" do
      key = :binary.copy(<<0xAA>>, 131)
      data = "This is a test using a larger than block-size key and a larger than block-size data. The key needs to be hashed before being used by the HMAC algorithm."

      assert Hmac.hmac_sha256_hex(key, data) ==
               "9b09ffa71b942fcb27635fbcd5b0e944bfdc63644f0713938a7f51535c3a35e2"
    end
  end

  # ===========================================================================
  # RFC 4231 Test Vectors — HMAC-SHA512
  # ===========================================================================

  describe "hmac_sha512/2 — RFC 4231 vectors" do
    test "TC1: short key, short data" do
      key = :binary.copy(<<0x0B>>, 20)
      data = "Hi There"

      assert Hmac.hmac_sha512_hex(key, data) ==
               "87aa7cdea5ef619d4ff0b4241a1d6cb02379f4e2ce4ec2787ad0b30545e17cdedaa833b7d6b8a702038b274eaea3f4e4be9d914eeb61f1702e696c203a126854"
    end

    test "TC2: key = 'Jefe'" do
      assert Hmac.hmac_sha512_hex("Jefe", "what do ya want for nothing?") ==
               "164b7a7bfcf819e2e395fbe73b56e0a387bd64222e831fd610270cd7ea2505549758bf75c05a994a6d034f65f8f0e6fdcaeab1a34d4a6b4b636e070a38bce737"
    end

    test "TC3: key and data are repeated bytes" do
      key = :binary.copy(<<0xAA>>, 20)
      data = :binary.copy(<<0xDD>>, 50)

      assert Hmac.hmac_sha512_hex(key, data) ==
               "fa73b0089d56a284efb0f0756c890be9b1b5dbdd8ee81a3655f83e33b2279d39bf3e848279a722c806b485a47e67c807b946a337bee8942674278859e13292fb"
    end

    test "TC4: longer key" do
      key = Base.decode16!("0102030405060708090a0b0c0d0e0f10111213141516171819", case: :lower)
      data = :binary.copy(<<0xCD>>, 50)

      assert Hmac.hmac_sha512_hex(key, data) ==
               "b0ba465637458c6990e5a8c5f61d4af7e576d97ff94b872de76f8050361ee3dba91ca5c11aa25eb4d679275cc5788063a5f19741120c4f2de2adebeb10a298dd"
    end

    test "TC6: key longer than block size (128 bytes for SHA-512)" do
      key = :binary.copy(<<0xAA>>, 131)
      data = "Test Using Larger Than Block-Size Key - Hash Key First"

      assert Hmac.hmac_sha512_hex(key, data) ==
               "80b24263c7c1a3ebb71493c1dd7be8b49b46d1f41b4aeec1121b013783f8f3526b56d037e05f2598bd0fd2215d6a1e5295e64f73f63f0aec8b915a985d786598"
    end

    test "TC7: key longer than block size, longer data" do
      key = :binary.copy(<<0xAA>>, 131)
      data = "This is a test using a larger than block-size key and a larger than block-size data. The key needs to be hashed before being used by the HMAC algorithm."

      assert Hmac.hmac_sha512_hex(key, data) ==
               "e37b6a775dc87dbaa4dfa9f96e5e3ffddebd71f8867289865df5a32d20cdc944b6022cac3c4982b10d5eeb55c3e4de15134676fb6de0446065c97440fa8c6a58"
    end
  end

  # ===========================================================================
  # RFC 2202 Test Vectors — HMAC-MD5
  # ===========================================================================

  describe "hmac_md5/2 — RFC 2202 vectors" do
    test "TC1: short key, short data" do
      key = :binary.copy(<<0x0B>>, 16)
      data = "Hi There"
      assert Hmac.hmac_md5_hex(key, data) == "9294727a3638bb1c13f48ef8158bfc9d"
    end

    test "TC2: key = 'Jefe'" do
      assert Hmac.hmac_md5_hex("Jefe", "what do ya want for nothing?") ==
               "750c783e6ab0b503eaa86e310a5db738"
    end

    test "TC3: key and data are repeated bytes" do
      key = :binary.copy(<<0xAA>>, 16)
      data = :binary.copy(<<0xDD>>, 50)
      assert Hmac.hmac_md5_hex(key, data) == "56be34521d144c88dbb8c733f0e8b3f6"
    end

    test "TC6: key longer than block size" do
      key = :binary.copy(<<0xAA>>, 80)
      data = "Test Using Larger Than Block-Size Key - Hash Key First"
      assert Hmac.hmac_md5_hex(key, data) == "6b1ab7fe4bd7bf8f0b62e6ce61b9d0cd"
    end

    test "TC7: key longer than block size, longer data" do
      key = :binary.copy(<<0xAA>>, 80)
      data = "Test Using Larger Than Block-Size Key and Larger Than One Block-Size Data"
      assert Hmac.hmac_md5_hex(key, data) == "6f630fad67cda0ee1fb1f562db3aa53e"
    end
  end

  # ===========================================================================
  # RFC 2202 Test Vectors — HMAC-SHA1
  # ===========================================================================

  describe "hmac_sha1/2 — RFC 2202 vectors" do
    test "TC1: short key, short data" do
      key = :binary.copy(<<0x0B>>, 20)
      data = "Hi There"
      assert Hmac.hmac_sha1_hex(key, data) == "b617318655057264e28bc0b6fb378c8ef146be00"
    end

    test "TC2: key = 'Jefe'" do
      assert Hmac.hmac_sha1_hex("Jefe", "what do ya want for nothing?") ==
               "effcdf6ae5eb2fa2d27416d5f184df9c259a7c79"
    end

    test "TC3: key and data are repeated bytes" do
      key = :binary.copy(<<0xAA>>, 20)
      data = :binary.copy(<<0xDD>>, 50)
      assert Hmac.hmac_sha1_hex(key, data) == "125d7342b9ac11cd91a39af48aa17b4f63f175d3"
    end

    test "TC6: key longer than block size" do
      key = :binary.copy(<<0xAA>>, 80)
      data = "Test Using Larger Than Block-Size Key - Hash Key First"
      assert Hmac.hmac_sha1_hex(key, data) == "aa4ae5e15272d00e95705637ce8a3b55ed402112"
    end

    test "TC7: key longer than block size, longer data" do
      key = :binary.copy(<<0xAA>>, 80)
      data = "Test Using Larger Than Block-Size Key and Larger Than One Block-Size Data"
      assert Hmac.hmac_sha1_hex(key, data) == "e8e99d0f45237d786d6bbaa7965c7808bbff1a91"
    end
  end

  # ===========================================================================
  # Return type and format
  # ===========================================================================

  describe "return types" do
    test "hmac_md5 returns 16-byte binary" do
      result = Hmac.hmac_md5("key", "msg")
      assert is_binary(result)
      assert byte_size(result) == 16
    end

    test "hmac_sha1 returns 20-byte binary" do
      result = Hmac.hmac_sha1("key", "msg")
      assert is_binary(result)
      assert byte_size(result) == 20
    end

    test "hmac_sha256 returns 32-byte binary" do
      result = Hmac.hmac_sha256("key", "msg")
      assert is_binary(result)
      assert byte_size(result) == 32
    end

    test "hmac_sha512 returns 64-byte binary" do
      result = Hmac.hmac_sha512("key", "msg")
      assert is_binary(result)
      assert byte_size(result) == 64
    end

    test "hmac_sha256_hex returns 64-char lowercase hex string" do
      result = Hmac.hmac_sha256_hex("key", "msg")
      assert is_binary(result)
      assert String.length(result) == 64
      assert result =~ ~r/^[0-9a-f]+$/
    end

    test "hmac_sha512_hex returns 128-char lowercase hex string" do
      result = Hmac.hmac_sha512_hex("key", "msg")
      assert String.length(result) == 128
      assert result =~ ~r/^[0-9a-f]+$/
    end
  end

  # ===========================================================================
  # Key handling edge cases
  # ===========================================================================

  describe "key handling" do
    test "empty key raises ArgumentError" do
      assert_raise ArgumentError, "HMAC key must not be empty", fn ->
        Hmac.hmac_sha256("", "message")
      end
    end

    test "empty message is valid" do
      result = Hmac.hmac_sha256("key", "")
      assert byte_size(result) == 32
    end

    test "empty key and empty message raises ArgumentError" do
      assert_raise ArgumentError, "HMAC key must not be empty", fn ->
        Hmac.hmac_sha256("", "")
      end
    end

    test "key exactly block size (64 bytes) is not hashed" do
      key64 = :binary.copy(<<0x01>>, 64)
      # Just verify it produces a valid 32-byte result
      result = Hmac.hmac_sha256(key64, "msg")
      assert byte_size(result) == 32
    end

    test "key of 65 bytes (> block size) is hashed before use" do
      key65 = :binary.copy(<<0x01>>, 65)
      result = Hmac.hmac_sha256(key65, "msg")
      assert byte_size(result) == 32
    end

    test "key of 128 bytes is exactly SHA-512 block size" do
      key128 = :binary.copy(<<0x01>>, 128)
      result = Hmac.hmac_sha512(key128, "msg")
      assert byte_size(result) == 64
    end

    test "key of 129 bytes is > SHA-512 block size and gets hashed" do
      key129 = :binary.copy(<<0x01>>, 129)
      result = Hmac.hmac_sha512(key129, "msg")
      assert byte_size(result) == 64
    end
  end

  # ===========================================================================
  # Determinism and authentication properties
  # ===========================================================================

  describe "authentication properties" do
    test "same key + same message = same tag (deterministic)" do
      assert Hmac.hmac_sha256("key", "msg") == Hmac.hmac_sha256("key", "msg")
    end

    test "different key → different tag (key sensitivity)" do
      t1 = Hmac.hmac_sha256("key1", "message")
      t2 = Hmac.hmac_sha256("key2", "message")
      assert t1 != t2
    end

    test "different message → different tag (message sensitivity)" do
      t1 = Hmac.hmac_sha256("key", "message1")
      t2 = Hmac.hmac_sha256("key", "message2")
      assert t1 != t2
    end

    test "HMAC is not prefix-malleable (length extension resistance)" do
      # If HMAC were just hash(key || msg), an attacker could extend the message.
      # Verify that HMAC("key", "msg") != HMAC("key", "msg" <> extra) for any extra.
      base = Hmac.hmac_sha256("key", "base_message")
      extended = Hmac.hmac_sha256("key", "base_message" <> "extra")
      assert base != extended
    end

    test "generic hmac/4 matches named variant for sha256" do
      import CodingAdventures.Sha256, only: [sha256: 1]
      key = "test-key"
      msg = "test-message"
      assert Hmac.hmac(&sha256/1, 64, key, msg) == Hmac.hmac_sha256(key, msg)
    end

    test "generic hmac/4 matches named variant for sha512" do
      import CodingAdventures.Sha512, only: [sha512: 1]
      key = "test-key"
      msg = "test-message"
      assert Hmac.hmac(&sha512/1, 128, key, msg) == Hmac.hmac_sha512(key, msg)
    end

    test "hex variants match binary variants" do
      key = "key"
      msg = "message"
      assert Hmac.hmac_sha256_hex(key, msg) == Base.encode16(Hmac.hmac_sha256(key, msg), case: :lower)
      assert Hmac.hmac_sha512_hex(key, msg) == Base.encode16(Hmac.hmac_sha512(key, msg), case: :lower)
    end
  end

  # ===========================================================================
  # Binary data (non-ASCII)
  # ===========================================================================

  describe "binary data" do
    test "binary key and message work" do
      key = :binary.copy(<<0x00>>, 16)
      msg = :binary.copy(<<0xFF>>, 16)
      result = Hmac.hmac_sha256(key, msg)
      assert byte_size(result) == 32
    end

    test "all-zero key and message" do
      result = Hmac.hmac_sha256(:binary.copy(<<0>>, 32), :binary.copy(<<0>>, 32))
      assert byte_size(result) == 32
    end

    test "HMAC-SHA256 produces different results for different hash functions" do
      key = "same-key"
      msg = "same-message"
      assert Hmac.hmac_md5(key, msg) != Hmac.hmac_sha256(key, msg)
      assert Hmac.hmac_sha1(key, msg) != Hmac.hmac_sha256(key, msg)
      assert Hmac.hmac_sha256(key, msg) != Hmac.hmac_sha512(key, msg)
    end
  end
end
