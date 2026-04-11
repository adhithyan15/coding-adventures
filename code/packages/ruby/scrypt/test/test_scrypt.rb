# frozen_string_literal: true

# test_scrypt.rb — Minitest suite for CodingAdventures::Scrypt
#
# Test strategy:
#   1. RFC 7914 official test vectors — the ground truth for correctness.
#   2. Hex variant — scrypt_hex/6 must match scrypt/6.unpack1("H*").
#   3. Output length — dk_len bytes returned exactly.
#   4. Determinism — calling scrypt twice with identical inputs yields identical output.
#   5. Parameter validation — ArgumentError for invalid N, r, p, dk_len.
#   6. Edge cases — various valid parameter combinations.

require "minitest/autorun"
require "coding_adventures_scrypt"

class TestScrypt < Minitest::Test
  # ─── RFC 7914 Test Vectors ─────────────────────────────────────────────────

  # RFC 7914 § B — Test vector 1
  # scrypt("", "", N=16, r=1, p=1, dkLen=64)
  #
  # This vector is critical: it uses an empty password string. Our
  # implementation handles this by calling Hmac.hmac directly, bypassing
  # the non-empty-key guard in the public hmac_sha256 method.
  def test_rfc7914_vector1
    result = CodingAdventures::Scrypt.scrypt_hex("", "", 16, 1, 1, 64)
    # Verified against OpenSSL::KDF.scrypt and Python hashlib.scrypt.
    expected = "77d6576238657b203b19ca42c18a0497" \
               "f16b4844e3074ae8dfdffa3fede21442" \
               "fcd0069ded0948f8326a753a0fc81f17" \
               "e8d3e0fb2e0d3628cf35e20c38d18906"
    assert_equal expected, result,
      "RFC 7914 vector 1 mismatch: check Salsa20/8 round indices and uint32 masking"
  end

  # RFC 7914 § B — Test vector 2
  # scrypt("password", "NaCl", N=1024, r=8, p=16, dkLen=64)
  #
  # This vector exercises the parallelisation factor p=16 (16 RoMix lanes)
  # and a much larger N=1024, requiring a 1024-entry lookup table per lane.
  # It also uses the common English words "password" and "NaCl" (table salt)
  # as a nod to real-world password hashing usage.
  def test_rfc7914_vector2
    result = CodingAdventures::Scrypt.scrypt_hex("password", "NaCl", 1024, 8, 16, 64)
    # Verified against OpenSSL::KDF.scrypt and Python hashlib.scrypt.
    expected = "fdbabe1c9d3472007856e7190d01e9fe" \
               "7c6ad7cbc8237830e77376634b373162" \
               "2eaf30d92e22a3886ff109279d9830da" \
               "c727afb94a83ee6d8360cbdfa2cc0640"
    assert_equal expected, result,
      "RFC 7914 vector 2 mismatch: check BlockMix interleaving and PBKDF2 boundary conditions"
  end

  # ─── Hex Variant ──────────────────────────────────────────────────────────

  # scrypt_hex must be identical to scrypt/6 with .unpack1("H*") applied.
  def test_hex_variant_consistency
    binary = CodingAdventures::Scrypt.scrypt("pass", "salt", 16, 1, 1, 16)
    hex = CodingAdventures::Scrypt.scrypt_hex("pass", "salt", 16, 1, 1, 16)
    assert_equal binary.unpack1("H*"), hex
  end

  # ─── Output Length ────────────────────────────────────────────────────────

  # The returned binary string must be exactly dk_len bytes.
  def test_output_length_matches_dk_len
    [1, 16, 32, 64].each do |len|
      result = CodingAdventures::Scrypt.scrypt("pw", "s", 16, 1, 1, len)
      assert_equal len, result.bytesize,
        "Expected #{len} bytes but got #{result.bytesize}"
      assert_equal Encoding::ASCII_8BIT, result.encoding,
        "Expected binary encoding (ASCII-8BIT)"
    end
  end

  # ─── Determinism ──────────────────────────────────────────────────────────

  # Two calls with the same arguments must return the same key.
  # scrypt is a deterministic function — no randomness is introduced after
  # the salt is fixed.
  def test_determinism
    args = ["mypassword", "mysalt", 16, 1, 1, 32]
    first = CodingAdventures::Scrypt.scrypt(*args)
    second = CodingAdventures::Scrypt.scrypt(*args)
    assert_equal first, second, "scrypt must be deterministic"
  end

  # Different passwords must produce different keys.
  def test_different_passwords_produce_different_keys
    a = CodingAdventures::Scrypt.scrypt("password1", "salt", 16, 1, 1, 32)
    b = CodingAdventures::Scrypt.scrypt("password2", "salt", 16, 1, 1, 32)
    refute_equal a, b, "Different passwords should produce different keys"
  end

  # Different salts must produce different keys.
  def test_different_salts_produce_different_keys
    a = CodingAdventures::Scrypt.scrypt("password", "salt1", 16, 1, 1, 32)
    b = CodingAdventures::Scrypt.scrypt("password", "salt2", 16, 1, 1, 32)
    refute_equal a, b, "Different salts should produce different keys"
  end

  # ─── Parameter Validation ─────────────────────────────────────────────────

  # N must be a power of 2 and >= 2. Non-power-of-2 values should raise.
  def test_invalid_n_not_power_of_two
    assert_raises(ArgumentError) { CodingAdventures::Scrypt.scrypt("p", "s", 3, 1, 1, 32) }
    assert_raises(ArgumentError) { CodingAdventures::Scrypt.scrypt("p", "s", 6, 1, 1, 32) }
    assert_raises(ArgumentError) { CodingAdventures::Scrypt.scrypt("p", "s", 100, 1, 1, 32) }
  end

  # N = 1 is not a valid power-of-2 for scrypt (need at least 2 table entries).
  def test_invalid_n_equals_one
    assert_raises(ArgumentError) { CodingAdventures::Scrypt.scrypt("p", "s", 1, 1, 1, 32) }
  end

  # N = 0 and negatives must raise.
  def test_invalid_n_zero_or_negative
    assert_raises(ArgumentError) { CodingAdventures::Scrypt.scrypt("p", "s", 0, 1, 1, 32) }
    assert_raises(ArgumentError) { CodingAdventures::Scrypt.scrypt("p", "s", -4, 1, 1, 32) }
  end

  # N > 2^20 must raise (memory guard).
  def test_invalid_n_too_large
    assert_raises(ArgumentError) { CodingAdventures::Scrypt.scrypt("p", "s", 2**21, 1, 1, 32) }
  end

  # r must be >= 1.
  def test_invalid_r_zero_or_negative
    assert_raises(ArgumentError) { CodingAdventures::Scrypt.scrypt("p", "s", 16, 0, 1, 32) }
    assert_raises(ArgumentError) { CodingAdventures::Scrypt.scrypt("p", "s", 16, -1, 1, 32) }
  end

  # p must be >= 1.
  def test_invalid_p_zero_or_negative
    assert_raises(ArgumentError) { CodingAdventures::Scrypt.scrypt("p", "s", 16, 1, 0, 32) }
    assert_raises(ArgumentError) { CodingAdventures::Scrypt.scrypt("p", "s", 16, 1, -1, 32) }
  end

  # dk_len must be between 1 and 2^20 inclusive.
  def test_invalid_dk_len_zero
    assert_raises(ArgumentError) { CodingAdventures::Scrypt.scrypt("p", "s", 16, 1, 1, 0) }
  end

  def test_invalid_dk_len_too_large
    assert_raises(ArgumentError) { CodingAdventures::Scrypt.scrypt("p", "s", 16, 1, 1, 2**20 + 1) }
  end

  # p * r > 2^30 must raise.
  def test_invalid_p_times_r_overflow
    # 2^15 * 2^16 = 2^31 > 2^30
    assert_raises(ArgumentError) { CodingAdventures::Scrypt.scrypt("p", "s", 4, 2**16, 2**15, 32) }
  end

  # p*128*r > 2^30 must raise (memory cap), even when p*r ≤ 2^30.
  def test_invalid_p_128_r_memory_cap
    # p=1, r=2^24: p*r = 2^24 ≤ 2^30 (passes old guard), but
    # p*128*r = 128 * 2^24 = 2^31 > 2^30 (triggers memory-cap guard).
    assert_raises(ArgumentError) { CodingAdventures::Scrypt.scrypt("p", "s", 2, 2**24, 1, 32) }
  end

  # ─── Valid Boundary Values ─────────────────────────────────────────────────

  # N=2 is the smallest valid cost factor.
  def test_minimum_valid_n
    result = CodingAdventures::Scrypt.scrypt("pw", "salt", 2, 1, 1, 16)
    assert_equal 16, result.bytesize
  end

  # N=2^20 is the maximum allowed cost factor. This test is slow (~seconds)
  # with r=1, p=1 but should still complete.
  # Skipped to keep the suite fast; enable manually when needed.
  # def test_maximum_valid_n
  #   result = CodingAdventures::Scrypt.scrypt("pw", "salt", 2**20, 1, 1, 16)
  #   assert_equal 16, result.bytesize
  # end

  # dk_len=1 — single byte output.
  def test_minimum_dk_len
    result = CodingAdventures::Scrypt.scrypt("pw", "salt", 16, 1, 1, 1)
    assert_equal 1, result.bytesize
  end

  # dk_len=2^20 — maximum allowed output.
  # Skipped to keep the suite fast.
  # def test_maximum_dk_len
  #   result = CodingAdventures::Scrypt.scrypt("pw", "salt", 16, 1, 1, 2**20)
  #   assert_equal 2**20, result.bytesize
  # end

  # ─── Binary String Encoding ────────────────────────────────────────────────

  # Input may be a UTF-8 String containing only ASCII characters — must work.
  def test_utf8_input_coerced_to_binary
    result = CodingAdventures::Scrypt.scrypt("password", "NaCl", 16, 1, 1, 32)
    assert_equal Encoding::ASCII_8BIT, result.encoding
    assert_equal 32, result.bytesize
  end

  # Explicit binary (.b) input must also work.
  def test_binary_input
    result = CodingAdventures::Scrypt.scrypt("password".b, "NaCl".b, 16, 1, 1, 32)
    assert_equal 32, result.bytesize
  end

  # ─── Parallelism Factor p ─────────────────────────────────────────────────

  # p=2 should produce a different key than p=1 for the same inputs.
  def test_different_p_produces_different_keys
    a = CodingAdventures::Scrypt.scrypt("pw", "salt", 16, 1, 1, 32)
    b = CodingAdventures::Scrypt.scrypt("pw", "salt", 16, 1, 2, 32)
    refute_equal a, b, "Different p should produce different keys"
  end

  # ─── r Factor ─────────────────────────────────────────────────────────────

  # r=2 should produce a different key than r=1 for the same inputs.
  def test_different_r_produces_different_keys
    a = CodingAdventures::Scrypt.scrypt("pw", "salt", 16, 1, 1, 32)
    b = CodingAdventures::Scrypt.scrypt("pw", "salt", 16, 2, 1, 32)
    refute_equal a, b, "Different r should produce different keys"
  end
end
