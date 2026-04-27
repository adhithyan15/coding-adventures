# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_argon2d"

class Argon2dTest < Minitest::Test
  A = CodingAdventures::Argon2d

  # ─── RFC 9106 §5.1 gold-standard vector ───────────────────────────────
  #
  #   password = 32 × 0x01
  #   salt     = 16 × 0x02
  #   key      =  8 × 0x03
  #   ad       = 12 × 0x04
  #   t=3, m=32, p=4, T=32
  #   tag      = 512b391b6f1162975371d30919734294 f868e3be3984f3c1a13a4db9fabe4acb
  RFC_PASSWORD = ("\x01".b * 32).freeze
  RFC_SALT = ("\x02".b * 16).freeze
  RFC_KEY = ("\x03".b * 8).freeze
  RFC_AD = ("\x04".b * 12).freeze
  RFC_EXPECTED_HEX = "512b391b6f1162975371d30919734294f868e3be3984f3c1a13a4db9fabe4acb"

  def test_rfc_9106_section_5_1_vector
    hex = A.argon2d_hex(RFC_PASSWORD, RFC_SALT, 3, 32, 4, 32,
                        key: RFC_KEY, associated_data: RFC_AD)
    assert_equal RFC_EXPECTED_HEX, hex
  end

  def test_hex_matches_binary
    tag = A.argon2d(RFC_PASSWORD, RFC_SALT, 3, 32, 4, 32,
                    key: RFC_KEY, associated_data: RFC_AD)
    hex = A.argon2d_hex(RFC_PASSWORD, RFC_SALT, 3, 32, 4, 32,
                        key: RFC_KEY, associated_data: RFC_AD)
    assert_equal tag.unpack1("H*"), hex
  end

  # ─── Validation ──────────────────────────────────────────────────────
  def test_rejects_short_salt
    assert_raises(ArgumentError) { A.argon2d("pw", "short", 1, 8, 1, 32) }
  end

  def test_rejects_zero_time_cost
    assert_raises(ArgumentError) { A.argon2d("pw", "a" * 8, 0, 8, 1, 32) }
  end

  def test_rejects_tag_length_under_4
    assert_raises(ArgumentError) { A.argon2d("pw", "a" * 8, 1, 8, 1, 3) }
  end

  def test_rejects_memory_below_floor
    assert_raises(ArgumentError) { A.argon2d("pw", "a" * 8, 1, 7, 1, 32) }
  end

  def test_rejects_zero_parallelism
    assert_raises(ArgumentError) { A.argon2d("pw", "a" * 8, 1, 8, 0, 32) }
  end

  def test_rejects_unsupported_version
    assert_raises(ArgumentError) do
      A.argon2d("pw", "a" * 8, 1, 8, 1, 32, version: 0x10)
    end
  end

  # ─── Determinism and separation ──────────────────────────────────────
  def test_deterministic
    a = A.argon2d_hex("pw", "a" * 8, 1, 8, 1, 32)
    b = A.argon2d_hex("pw", "a" * 8, 1, 8, 1, 32)
    assert_equal a, b
  end

  def test_differs_on_password
    a = A.argon2d_hex("pw1", "a" * 8, 1, 8, 1, 32)
    b = A.argon2d_hex("pw2", "a" * 8, 1, 8, 1, 32)
    refute_equal a, b
  end

  def test_differs_on_salt
    a = A.argon2d_hex("pw", "a" * 8, 1, 8, 1, 32)
    b = A.argon2d_hex("pw", "b" * 8, 1, 8, 1, 32)
    refute_equal a, b
  end

  def test_key_binds
    a = A.argon2d_hex("pw", "a" * 8, 1, 8, 1, 32)
    b = A.argon2d_hex("pw", "a" * 8, 1, 8, 1, 32, key: "k1")
    c = A.argon2d_hex("pw", "a" * 8, 1, 8, 1, 32, key: "k2")
    refute_equal a, b
    refute_equal b, c
  end

  def test_ad_binds
    a = A.argon2d_hex("pw", "a" * 8, 1, 8, 1, 32)
    b = A.argon2d_hex("pw", "a" * 8, 1, 8, 1, 32, associated_data: "x")
    c = A.argon2d_hex("pw", "a" * 8, 1, 8, 1, 32, associated_data: "y")
    refute_equal a, b
    refute_equal b, c
  end

  # ─── Tag length variants ─────────────────────────────────────────────
  def test_tag_length_4
    tag = A.argon2d("pw", "a" * 8, 1, 8, 1, 4)
    assert_equal 4, tag.bytesize
  end

  def test_tag_length_16
    tag = A.argon2d("pw", "a" * 8, 1, 8, 1, 16)
    assert_equal 16, tag.bytesize
  end

  def test_tag_length_65_crosses_h_prime_boundary
    tag = A.argon2d("pw", "a" * 8, 1, 8, 1, 65)
    assert_equal 65, tag.bytesize
  end

  def test_tag_length_128
    tag = A.argon2d("pw", "a" * 8, 1, 8, 1, 128)
    assert_equal 128, tag.bytesize
  end

  # ─── Parallelism / passes ────────────────────────────────────────────
  def test_multi_lane
    tag = A.argon2d("pw", "a" * 8, 1, 16, 2, 32)
    assert_equal 32, tag.bytesize
  end

  def test_multi_pass
    a = A.argon2d_hex("pw", "a" * 8, 1, 8, 1, 32)
    b = A.argon2d_hex("pw", "a" * 8, 2, 8, 1, 32)
    refute_equal a, b
  end
end
