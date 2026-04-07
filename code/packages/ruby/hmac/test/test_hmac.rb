# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_hmac"

# Shorthand alias
HMAC = CodingAdventures::Hmac

class TestHmacSHA256 < Minitest::Test
  # RFC 4231 test vectors for HMAC-SHA256

  def test_tc1_20_byte_key_hi_there
    key = "\x0b" * 20
    assert_equal(
      "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7",
      HMAC.hmac_sha256_hex(key, "Hi There")
    )
  end

  def test_tc2_jefe
    assert_equal(
      "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843",
      HMAC.hmac_sha256_hex("Jefe", "what do ya want for nothing?")
    )
  end

  def test_tc3_0xaa_key_0xdd_data
    key  = "\xaa" * 20
    data = "\xdd" * 50
    assert_equal(
      "773ea91e36800e46854db8ebd09181a72959098b3ef8c122d9635514ced565fe",
      HMAC.hmac_sha256_hex(key, data)
    )
  end

  def test_tc6_longer_than_block_size_key
    key = "\xaa" * 131
    assert_equal(
      "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54",
      HMAC.hmac_sha256_hex(key, "Test Using Larger Than Block-Size Key - Hash Key First")
    )
  end

  def test_tc7_longer_than_block_size_key_and_data
    key  = "\xaa" * 131
    data = "This is a test using a larger than block-size key and a larger than block-size data. " \
           "The key needs to be hashed before being used by the HMAC algorithm."
    assert_equal(
      "9b09ffa71b942fcb27635fbcd5b0e944bfdc63644f0713938a7f51535c3a35e2",
      HMAC.hmac_sha256_hex(key, data)
    )
  end
end

class TestHmacSHA512 < Minitest::Test
  # RFC 4231 test vectors for HMAC-SHA512

  def test_tc1_20_byte_key_hi_there
    key = "\x0b" * 20
    assert_equal(
      "87aa7cdea5ef619d4ff0b4241a1d6cb02379f4e2ce4ec2787ad0b30545e17cdedaa833b7d6b8a702038b274eaea3f4e4be9d914eeb61f1702e696c203a126854",
      HMAC.hmac_sha512_hex(key, "Hi There")
    )
  end

  def test_tc2_jefe
    assert_equal(
      "164b7a7bfcf819e2e395fbe73b56e0a387bd64222e831fd610270cd7ea2505549758bf75c05a994a6d034f65f8f0e6fdcaeab1a34d4a6b4b636e070a38bce737",
      HMAC.hmac_sha512_hex("Jefe", "what do ya want for nothing?")
    )
  end

  def test_tc6_longer_than_block_size_key
    key = "\xaa" * 131
    assert_equal(
      "80b24263c7c1a3ebb71493c1dd7be8b49b46d1f41b4aeec1121b013783f8f3526b56d037e05f2598bd0fd2215d6a1e5295e64f73f63f0aec8b915a985d786598",
      HMAC.hmac_sha512_hex(key, "Test Using Larger Than Block-Size Key - Hash Key First")
    )
  end
end

class TestHmacMD5 < Minitest::Test
  # RFC 2202 test vectors for HMAC-MD5

  def test_tc1_16_byte_key
    key = "\x0b" * 16
    assert_equal("9294727a3638bb1c13f48ef8158bfc9d", HMAC.hmac_md5_hex(key, "Hi There"))
  end

  def test_tc2_jefe
    assert_equal(
      "750c783e6ab0b503eaa86e310a5db738",
      HMAC.hmac_md5_hex("Jefe", "what do ya want for nothing?")
    )
  end

  def test_tc6_longer_than_block_size_key
    key = "\xaa" * 80
    assert_equal(
      "6b1ab7fe4bd7bf8f0b62e6ce61b9d0cd",
      HMAC.hmac_md5_hex(key, "Test Using Larger Than Block-Size Key - Hash Key First")
    )
  end
end

class TestHmacSHA1 < Minitest::Test
  # RFC 2202 test vectors for HMAC-SHA1

  def test_tc1_20_byte_key
    key = "\x0b" * 20
    assert_equal("b617318655057264e28bc0b6fb378c8ef146be00", HMAC.hmac_sha1_hex(key, "Hi There"))
  end

  def test_tc2_jefe
    assert_equal(
      "effcdf6ae5eb2fa2d27416d5f184df9c259a7c79",
      HMAC.hmac_sha1_hex("Jefe", "what do ya want for nothing?")
    )
  end

  def test_tc6_longer_than_block_size_key
    key = "\xaa" * 80
    assert_equal(
      "aa4ae5e15272d00e95705637ce8a3b55ed402112",
      HMAC.hmac_sha1_hex(key, "Test Using Larger Than Block-Size Key - Hash Key First")
    )
  end
end

class TestHmacReturnLengths < Minitest::Test
  def test_md5_returns_16_bytes
    assert_equal 16, HMAC.hmac_md5("k", "m").bytesize
  end

  def test_sha1_returns_20_bytes
    assert_equal 20, HMAC.hmac_sha1("k", "m").bytesize
  end

  def test_sha256_returns_32_bytes
    assert_equal 32, HMAC.hmac_sha256("k", "m").bytesize
  end

  def test_sha512_returns_64_bytes
    assert_equal 64, HMAC.hmac_sha512("k", "m").bytesize
  end
end

class TestHmacKeyHandling < Minitest::Test
  def test_empty_key_and_message_sha256
    assert_equal 32, HMAC.hmac_sha256("", "").bytesize
  end

  def test_empty_key_and_message_sha512
    assert_equal 64, HMAC.hmac_sha512("", "").bytesize
  end

  def test_long_keys_of_different_lengths_differ
    k65 = "\x01" * 65
    k66 = "\x01" * 66
    refute_equal HMAC.hmac_sha256_hex(k65, "msg"), HMAC.hmac_sha256_hex(k66, "msg")
  end
end

class TestHmacAuthenticationProperties < Minitest::Test
  def test_deterministic
    assert_equal HMAC.hmac_sha256("k", "m"), HMAC.hmac_sha256("k", "m")
  end

  def test_key_sensitivity
    refute_equal HMAC.hmac_sha256("k1", "m"), HMAC.hmac_sha256("k2", "m")
  end

  def test_message_sensitivity
    refute_equal HMAC.hmac_sha256("k", "m1"), HMAC.hmac_sha256("k", "m2")
  end

  def test_hex_matches_bytes
    tag = HMAC.hmac_sha256("k", "m")
    hex = HMAC.hmac_sha256_hex("k", "m")
    assert_equal hex, tag.unpack1("H*")
  end
end
