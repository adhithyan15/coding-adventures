# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_pbkdf2"

PBKDF2 = CodingAdventures::PBKDF2

class TestRfc6070Sha1 < Minitest::Test
  # RFC 6070 official test vectors for PBKDF2-HMAC-SHA1.
  # These cover single-iteration, multi-iteration, long inputs, and null bytes.

  def test_vector1_c1
    dk = PBKDF2.pbkdf2_hmac_sha1("password", "salt", 1, 20)
    assert_equal "0c60c80f961f0e71f3a9b524af6012062fe037a6", dk.unpack1("H*")
  end

  def test_vector2_c4096
    dk = PBKDF2.pbkdf2_hmac_sha1("password", "salt", 4096, 20)
    assert_equal "4b007901b765489abead49d926f721d065a429c1", dk.unpack1("H*")
  end

  def test_vector3_long_password_salt
    dk = PBKDF2.pbkdf2_hmac_sha1(
      "passwordPASSWORDpassword",
      "saltSALTsaltSALTsaltSALTsaltSALTsalt",
      4096,
      25
    )
    assert_equal "3d2eec4fe41c849b80c8d83662c0e44a8b291a964cf2f07038", dk.unpack1("H*")
  end

  def test_vector4_null_bytes
    dk = PBKDF2.pbkdf2_hmac_sha1("pass\x00word", "sa\x00lt", 4096, 16)
    assert_equal "56fa6aa75548099dcc37d7f03425e0c3", dk.unpack1("H*")
  end
end

class TestRfc7914Sha256 < Minitest::Test
  def test_vector1_c1_64bytes
    dk = PBKDF2.pbkdf2_hmac_sha256("passwd", "salt", 1, 64)
    expected = "55ac046e56e3089fec1691c22544b605" \
               "f94185216dde0465e68b9d57c20dacbc" \
               "49ca9cccf179b645991664b39d77ef31" \
               "7c71b845b1e30bd509112041d3a19783"
    assert_equal expected, dk.unpack1("H*")
  end

  def test_output_length
    assert_equal 32, PBKDF2.pbkdf2_hmac_sha256("key", "salt", 1, 32).bytesize
  end

  def test_truncation_consistency
    short = PBKDF2.pbkdf2_hmac_sha256("key", "salt", 1, 16)
    full  = PBKDF2.pbkdf2_hmac_sha256("key", "salt", 1, 32)
    assert_equal short, full[0, 16]
  end

  def test_multi_block
    dk64 = PBKDF2.pbkdf2_hmac_sha256("password", "salt", 1, 64)
    dk32 = PBKDF2.pbkdf2_hmac_sha256("password", "salt", 1, 32)
    assert_equal 64, dk64.bytesize
    assert_equal dk32, dk64[0, 32]
  end
end

class TestSha512 < Minitest::Test
  def test_output_length
    assert_equal 64, PBKDF2.pbkdf2_hmac_sha512("secret", "nacl", 1, 64).bytesize
  end

  def test_truncation
    short = PBKDF2.pbkdf2_hmac_sha512("secret", "nacl", 1, 32)
    full  = PBKDF2.pbkdf2_hmac_sha512("secret", "nacl", 1, 64)
    assert_equal short, full[0, 32]
  end

  def test_multi_block
    assert_equal 128, PBKDF2.pbkdf2_hmac_sha512("key", "salt", 1, 128).bytesize
  end
end

class TestHexVariants < Minitest::Test
  def test_sha1_hex_rfc6070
    assert_equal(
      "0c60c80f961f0e71f3a9b524af6012062fe037a6",
      PBKDF2.pbkdf2_hmac_sha1_hex("password", "salt", 1, 20)
    )
  end

  def test_sha256_hex_matches_bytes
    raw = PBKDF2.pbkdf2_hmac_sha256("passwd", "salt", 1, 32)
    hex = PBKDF2.pbkdf2_hmac_sha256_hex("passwd", "salt", 1, 32)
    assert_equal raw.unpack1("H*"), hex
  end

  def test_sha512_hex_matches_bytes
    raw = PBKDF2.pbkdf2_hmac_sha512("secret", "nacl", 1, 64)
    hex = PBKDF2.pbkdf2_hmac_sha512_hex("secret", "nacl", 1, 64)
    assert_equal raw.unpack1("H*"), hex
  end
end

class TestValidation < Minitest::Test
  def test_empty_password_raises_sha256
    assert_raises(ArgumentError) { PBKDF2.pbkdf2_hmac_sha256("", "salt", 1, 32) }
  end

  def test_empty_password_raises_sha1
    assert_raises(ArgumentError) { PBKDF2.pbkdf2_hmac_sha1("", "salt", 1, 20) }
  end

  def test_zero_iterations_raises
    assert_raises(ArgumentError) { PBKDF2.pbkdf2_hmac_sha256("pw", "salt", 0, 32) }
  end

  def test_negative_iterations_raises
    assert_raises(ArgumentError) { PBKDF2.pbkdf2_hmac_sha256("pw", "salt", -1, 32) }
  end

  def test_zero_key_length_raises
    assert_raises(ArgumentError) { PBKDF2.pbkdf2_hmac_sha256("pw", "salt", 1, 0) }
  end

  def test_empty_salt_allowed
    assert_equal 32, PBKDF2.pbkdf2_hmac_sha256("password", "", 1, 32).bytesize
  end

  def test_deterministic
    a = PBKDF2.pbkdf2_hmac_sha256("secret", "nacl", 100, 32)
    b = PBKDF2.pbkdf2_hmac_sha256("secret", "nacl", 100, 32)
    assert_equal a, b
  end

  def test_different_salts
    a = PBKDF2.pbkdf2_hmac_sha256("password", "salt1", 1, 32)
    b = PBKDF2.pbkdf2_hmac_sha256("password", "salt2", 1, 32)
    refute_equal a, b
  end

  def test_different_passwords
    a = PBKDF2.pbkdf2_hmac_sha256("password1", "salt", 1, 32)
    b = PBKDF2.pbkdf2_hmac_sha256("password2", "salt", 1, 32)
    refute_equal a, b
  end

  def test_different_iterations
    a = PBKDF2.pbkdf2_hmac_sha256("password", "salt", 1, 32)
    b = PBKDF2.pbkdf2_hmac_sha256("password", "salt", 2, 32)
    refute_equal a, b
  end
end
