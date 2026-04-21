# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_argon2id"

class Argon2idTest < Minitest::Test
  A = CodingAdventures::Argon2id

  # RFC 9106 §5.3 gold-standard vector.
  RFC_PASSWORD = ("\x01".b * 32).freeze
  RFC_SALT = ("\x02".b * 16).freeze
  RFC_KEY = ("\x03".b * 8).freeze
  RFC_AD = ("\x04".b * 12).freeze
  RFC_EXPECTED_HEX = "0d640df58d78766c08c037a34a8b53c9d01ef0452d75b65eb52520e96b01e659"

  def test_rfc_9106_section_5_3_vector
    hex = A.argon2id_hex(RFC_PASSWORD, RFC_SALT, 3, 32, 4, 32,
                         key: RFC_KEY, associated_data: RFC_AD)
    assert_equal RFC_EXPECTED_HEX, hex
  end

  def test_hex_matches_binary
    tag = A.argon2id(RFC_PASSWORD, RFC_SALT, 3, 32, 4, 32,
                     key: RFC_KEY, associated_data: RFC_AD)
    hex = A.argon2id_hex(RFC_PASSWORD, RFC_SALT, 3, 32, 4, 32,
                         key: RFC_KEY, associated_data: RFC_AD)
    assert_equal tag.unpack1("H*"), hex
  end

  def test_rejects_short_salt
    assert_raises(ArgumentError) { A.argon2id("pw", "short", 1, 8, 1, 32) }
  end

  def test_rejects_zero_time_cost
    assert_raises(ArgumentError) { A.argon2id("pw", "a" * 8, 0, 8, 1, 32) }
  end

  def test_rejects_tag_length_under_4
    assert_raises(ArgumentError) { A.argon2id("pw", "a" * 8, 1, 8, 1, 3) }
  end

  def test_rejects_memory_below_floor
    assert_raises(ArgumentError) { A.argon2id("pw", "a" * 8, 1, 7, 1, 32) }
  end

  def test_rejects_zero_parallelism
    assert_raises(ArgumentError) { A.argon2id("pw", "a" * 8, 1, 8, 0, 32) }
  end

  def test_rejects_unsupported_version
    assert_raises(ArgumentError) do
      A.argon2id("pw", "a" * 8, 1, 8, 1, 32, version: 0x10)
    end
  end

  def test_deterministic
    assert_equal A.argon2id_hex("pw", "a" * 8, 1, 8, 1, 32),
                 A.argon2id_hex("pw", "a" * 8, 1, 8, 1, 32)
  end

  def test_differs_on_password
    refute_equal A.argon2id_hex("pw1", "a" * 8, 1, 8, 1, 32),
                 A.argon2id_hex("pw2", "a" * 8, 1, 8, 1, 32)
  end

  def test_differs_on_salt
    refute_equal A.argon2id_hex("pw", "a" * 8, 1, 8, 1, 32),
                 A.argon2id_hex("pw", "b" * 8, 1, 8, 1, 32)
  end

  def test_key_binds
    a = A.argon2id_hex("pw", "a" * 8, 1, 8, 1, 32)
    b = A.argon2id_hex("pw", "a" * 8, 1, 8, 1, 32, key: "k1")
    c = A.argon2id_hex("pw", "a" * 8, 1, 8, 1, 32, key: "k2")
    refute_equal a, b
    refute_equal b, c
  end

  def test_ad_binds
    a = A.argon2id_hex("pw", "a" * 8, 1, 8, 1, 32)
    b = A.argon2id_hex("pw", "a" * 8, 1, 8, 1, 32, associated_data: "x")
    c = A.argon2id_hex("pw", "a" * 8, 1, 8, 1, 32, associated_data: "y")
    refute_equal a, b
    refute_equal b, c
  end

  def test_tag_length_4
    assert_equal 4, A.argon2id("pw", "a" * 8, 1, 8, 1, 4).bytesize
  end

  def test_tag_length_16
    assert_equal 16, A.argon2id("pw", "a" * 8, 1, 8, 1, 16).bytesize
  end

  def test_tag_length_65_crosses_h_prime_boundary
    assert_equal 65, A.argon2id("pw", "a" * 8, 1, 8, 1, 65).bytesize
  end

  def test_tag_length_128
    assert_equal 128, A.argon2id("pw", "a" * 8, 1, 8, 1, 128).bytesize
  end

  def test_multi_lane
    assert_equal 32, A.argon2id("pw", "a" * 8, 1, 16, 2, 32).bytesize
  end

  def test_multi_pass
    refute_equal A.argon2id_hex("pw", "a" * 8, 1, 8, 1, 32),
                 A.argon2id_hex("pw", "a" * 8, 2, 8, 1, 32)
  end
end
