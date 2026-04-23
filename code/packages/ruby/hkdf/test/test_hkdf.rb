# frozen_string_literal: true

# ============================================================================
# HKDF Tests — RFC 5869 Test Vectors + Edge Cases
# ============================================================================
#
# These tests verify the HKDF implementation against all three SHA-256 test
# cases from RFC 5869 Appendix A, plus edge cases for error handling.
#
# ============================================================================

require "minitest/autorun"
require "coding_adventures_hkdf"

class TestHKDF < Minitest::Test
  # Helper: convert hex string to binary string.
  def hex(str)
    [str].pack("H*")
  end

  # Helper: convert binary string to hex for comparison.
  def to_hex(str)
    str.unpack1("H*")
  end

  # ==========================================================================
  # RFC 5869 Test Case 1: Basic SHA-256
  # ==========================================================================

  def test_case_1_extract
    ikm  = hex("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b")
    salt = hex("000102030405060708090a0b0c")
    expected_prk = "077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844ad7c2b3e5"

    prk = CodingAdventures::HKDF.hkdf_extract(salt, ikm, "sha256")
    assert_equal expected_prk, to_hex(prk)
  end

  def test_case_1_expand
    prk  = hex("077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844ad7c2b3e5")
    info = hex("f0f1f2f3f4f5f6f7f8f9")
    expected_okm = "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865"

    okm = CodingAdventures::HKDF.hkdf_expand(prk, info, 42, "sha256")
    assert_equal expected_okm, to_hex(okm)
  end

  def test_case_1_combined
    ikm  = hex("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b")
    salt = hex("000102030405060708090a0b0c")
    info = hex("f0f1f2f3f4f5f6f7f8f9")
    expected_okm = "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865"

    okm = CodingAdventures::HKDF.hkdf(salt, ikm, info, 42, "sha256")
    assert_equal expected_okm, to_hex(okm)
  end

  # ==========================================================================
  # RFC 5869 Test Case 2: SHA-256 with longer inputs/outputs
  # ==========================================================================

  def test_case_2_extract
    ikm  = hex("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f404142434445464748494a4b4c4d4e4f")
    salt = hex("606162636465666768696a6b6c6d6e6f707172737475767778797a7b7c7d7e7f808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9fa0a1a2a3a4a5a6a7a8a9aaabacadaeaf")
    expected_prk = "06a6b88c5853361a06104c9ceb35b45cef760014904671014a193f40c15fc244"

    prk = CodingAdventures::HKDF.hkdf_extract(salt, ikm, "sha256")
    assert_equal expected_prk, to_hex(prk)
  end

  def test_case_2_expand
    prk  = hex("06a6b88c5853361a06104c9ceb35b45cef760014904671014a193f40c15fc244")
    info = hex("b0b1b2b3b4b5b6b7b8b9babbbcbdbebfc0c1c2c3c4c5c6c7c8c9cacbcccdcecfd0d1d2d3d4d5d6d7d8d9dadbdcdddedfe0e1e2e3e4e5e6e7e8e9eaebecedeeeff0f1f2f3f4f5f6f7f8f9fafbfcfdfeff")
    expected_okm = "b11e398dc80327a1c8e7f78c596a49344f012eda2d4efad8a050cc4c19afa97c59045a99cac7827271cb41c65e590e09da3275600c2f09b8367793a9aca3db71cc30c58179ec3e87c14c01d5c1f3434f1d87"

    okm = CodingAdventures::HKDF.hkdf_expand(prk, info, 82, "sha256")
    assert_equal expected_okm, to_hex(okm)
  end

  def test_case_2_combined
    ikm  = hex("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f404142434445464748494a4b4c4d4e4f")
    salt = hex("606162636465666768696a6b6c6d6e6f707172737475767778797a7b7c7d7e7f808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9fa0a1a2a3a4a5a6a7a8a9aaabacadaeaf")
    info = hex("b0b1b2b3b4b5b6b7b8b9babbbcbdbebfc0c1c2c3c4c5c6c7c8c9cacbcccdcecfd0d1d2d3d4d5d6d7d8d9dadbdcdddedfe0e1e2e3e4e5e6e7e8e9eaebecedeeeff0f1f2f3f4f5f6f7f8f9fafbfcfdfeff")
    expected_okm = "b11e398dc80327a1c8e7f78c596a49344f012eda2d4efad8a050cc4c19afa97c59045a99cac7827271cb41c65e590e09da3275600c2f09b8367793a9aca3db71cc30c58179ec3e87c14c01d5c1f3434f1d87"

    okm = CodingAdventures::HKDF.hkdf(salt, ikm, info, 82, "sha256")
    assert_equal expected_okm, to_hex(okm)
  end

  # ==========================================================================
  # RFC 5869 Test Case 3: SHA-256 empty salt and info
  # ==========================================================================

  def test_case_3_extract
    ikm  = hex("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b")
    salt = "".b
    expected_prk = "19ef24a32c717b167f33a91d6f648bdf96596776afdb6377ac434c1c293ccb04"

    prk = CodingAdventures::HKDF.hkdf_extract(salt, ikm, "sha256")
    assert_equal expected_prk, to_hex(prk)
  end

  def test_case_3_expand
    prk  = hex("19ef24a32c717b167f33a91d6f648bdf96596776afdb6377ac434c1c293ccb04")
    info = "".b
    expected_okm = "8da4e775a563c18f715f802a063c5a31b8a11f5c5ee1879ec3454e5f3c738d2d9d201395faa4b61a96c8"

    okm = CodingAdventures::HKDF.hkdf_expand(prk, info, 42, "sha256")
    assert_equal expected_okm, to_hex(okm)
  end

  def test_case_3_combined
    ikm  = hex("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b")
    salt = "".b
    info = "".b
    expected_okm = "8da4e775a563c18f715f802a063c5a31b8a11f5c5ee1879ec3454e5f3c738d2d9d201395faa4b61a96c8"

    okm = CodingAdventures::HKDF.hkdf(salt, ikm, info, 42, "sha256")
    assert_equal expected_okm, to_hex(okm)
  end

  # ==========================================================================
  # Edge Cases
  # ==========================================================================

  def test_expand_rejects_zero_length
    prk = "\x00" * 32
    assert_raises(ArgumentError) do
      CodingAdventures::HKDF.hkdf_expand(prk, "".b, 0, "sha256")
    end
  end

  def test_expand_rejects_negative_length
    prk = "\x00" * 32
    assert_raises(ArgumentError) do
      CodingAdventures::HKDF.hkdf_expand(prk, "".b, -1, "sha256")
    end
  end

  def test_expand_rejects_length_exceeding_max_sha256
    prk = "\x00" * 32
    assert_raises(ArgumentError) do
      CodingAdventures::HKDF.hkdf_expand(prk, "".b, 8161, "sha256")
    end
  end

  def test_expand_rejects_length_exceeding_max_sha512
    prk = "\x00" * 64
    assert_raises(ArgumentError) do
      CodingAdventures::HKDF.hkdf_expand(prk, "".b, 16321, "sha512")
    end
  end

  def test_single_byte_output
    ikm  = hex("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b")
    okm = CodingAdventures::HKDF.hkdf("".b, ikm, "".b, 1, "sha256")
    assert_equal 1, okm.bytesize
    # First byte of test case 3 OKM
    assert_equal 0x8d, okm.bytes[0]
  end

  def test_sha512_extract_produces_64_byte_prk
    ikm = hex("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b")
    prk = CodingAdventures::HKDF.hkdf_extract("".b, ikm, "sha512")
    assert_equal 64, prk.bytesize
  end

  def test_different_info_produces_different_output
    ikm  = hex("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b")
    okm1 = CodingAdventures::HKDF.hkdf("".b, ikm, "encryption", 32)
    okm2 = CodingAdventures::HKDF.hkdf("".b, ikm, "authentication", 32)
    refute_equal to_hex(okm1), to_hex(okm2)
  end

  def test_defaults_to_sha256
    ikm  = hex("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b")
    salt = hex("000102030405060708090a0b0c")
    info = hex("f0f1f2f3f4f5f6f7f8f9")
    expected_okm = "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865"

    okm = CodingAdventures::HKDF.hkdf(salt, ikm, info, 42)
    assert_equal expected_okm, to_hex(okm)
  end

  def test_nil_salt_treated_as_empty
    ikm  = hex("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b")
    expected_prk = "19ef24a32c717b167f33a91d6f648bdf96596776afdb6377ac434c1c293ccb04"

    prk = CodingAdventures::HKDF.hkdf_extract(nil, ikm, "sha256")
    assert_equal expected_prk, to_hex(prk)
  end
end
