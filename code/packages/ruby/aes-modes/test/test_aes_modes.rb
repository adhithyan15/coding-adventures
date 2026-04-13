# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_aes_modes"

# =============================================================================
# Tests for AES modes of operation --- ECB, CBC, CTR, GCM.
#
# Uses NIST SP 800-38A test vectors for ECB, CBC, CTR, and NIST GCM
# specification test vectors for GCM.
# =============================================================================

class TestAesModes < Minitest::Test
  M = CodingAdventures::AesModes

  # NIST SP 800-38A test vectors
  NIST_KEY = ["2b7e151628aed2a6abf7158809cf4f3c"].pack("H*")

  NIST_PT_BLOCKS = [
    "6bc1bee22e409f96e93d7e117393172a",
    "ae2d8a571e03ac9c9eb76fac45af8e51",
    "30c81c46a35ce411e5fbc1191a0a52ef",
    "f69f2445df4f9b17ad2b417be66c3710"
  ].map { |h| [h].pack("H*") }

  # =========================================================================
  # PKCS#7 Padding
  # =========================================================================

  def test_pkcs7_pad_short
    result = M.pkcs7_pad("hello".b)
    assert_equal 16, result.bytesize
    assert_equal [11] * 11, result.bytes[5..]
  end

  def test_pkcs7_pad_aligned
    result = M.pkcs7_pad("0123456789abcdef".b)
    assert_equal 32, result.bytesize
    assert_equal [16] * 16, result.bytes[16..]
  end

  def test_pkcs7_roundtrip
    (0..33).each do |length|
      data = (0...length).map { |i| i % 256 }.pack("C*")
      assert_equal data, M.pkcs7_unpad(M.pkcs7_pad(data)), "Failed for length #{length}"
    end
  end

  def test_pkcs7_unpad_invalid
    assert_raises(ArgumentError) { M.pkcs7_unpad("".b) }
    assert_raises(ArgumentError) { M.pkcs7_unpad("hello".b) }
  end

  # =========================================================================
  # ECB Mode
  # =========================================================================

  ECB_CT_BLOCKS = [
    "3ad77bb40d7a3660a89ecaf32466ef97",
    "f5d3d58503b9699de785895a96fdbaaf",
    "43b1cd7f598ece23881b00e3ed030688",
    "7b0c785e27e8ad3f8223207104725dd4"
  ].map { |h| [h].pack("H*") }

  def test_ecb_single_block
    ct = M.ecb_encrypt(NIST_PT_BLOCKS[0], NIST_KEY)
    assert_equal ECB_CT_BLOCKS[0], ct.byteslice(0, 16)
  end

  def test_ecb_roundtrip
    pt = NIST_PT_BLOCKS.join
    ct = M.ecb_encrypt(pt, NIST_KEY)
    assert_equal pt, M.ecb_decrypt(ct, NIST_KEY)
  end

  def test_ecb_identical_blocks
    block = "A".b * 16
    ct = M.ecb_encrypt(block * 3, NIST_KEY)
    assert_equal ct.byteslice(0, 16), ct.byteslice(16, 16)
    assert_equal ct.byteslice(16, 16), ct.byteslice(32, 16)
  end

  def test_ecb_empty
    ct = M.ecb_encrypt("".b, NIST_KEY)
    assert_equal "".b, M.ecb_decrypt(ct, NIST_KEY)
  end

  def test_ecb_various_lengths
    [1, 15, 16, 17, 31, 32, 48, 100].each do |length|
      pt = (0...length).map { |i| i % 256 }.pack("C*")
      ct = M.ecb_encrypt(pt, NIST_KEY)
      assert_equal pt, M.ecb_decrypt(ct, NIST_KEY), "Failed for length #{length}"
    end
  end

  # =========================================================================
  # CBC Mode
  # =========================================================================

  CBC_IV = ["000102030405060708090a0b0c0d0e0f"].pack("H*")

  CBC_CT_BLOCKS = [
    "7649abac8119b246cee98e9b12e9197d",
    "5086cb9b507219ee95db113a917678b2",
    "73bed6b8e3c1743b7116e69e22229516",
    "3ff1caa1681fac09120eca307586e1a7"
  ].map { |h| [h].pack("H*") }

  def test_cbc_single_block
    ct = M.cbc_encrypt(NIST_PT_BLOCKS[0], NIST_KEY, CBC_IV)
    assert_equal CBC_CT_BLOCKS[0], ct.byteslice(0, 16)
  end

  def test_cbc_all_nist_blocks
    pt = NIST_PT_BLOCKS.join
    ct = M.cbc_encrypt(pt, NIST_KEY, CBC_IV)
    CBC_CT_BLOCKS.each_with_index do |expected, i|
      assert_equal expected, ct.byteslice(i * 16, 16), "Block #{i} mismatch"
    end
  end

  def test_cbc_roundtrip
    pt = NIST_PT_BLOCKS.join
    ct = M.cbc_encrypt(pt, NIST_KEY, CBC_IV)
    assert_equal pt, M.cbc_decrypt(ct, NIST_KEY, CBC_IV)
  end

  def test_cbc_different_iv
    pt = "A".b * 16
    iv1 = "\x00".b * 16
    iv2 = "\x01".b * 16
    refute_equal M.cbc_encrypt(pt, NIST_KEY, iv1), M.cbc_encrypt(pt, NIST_KEY, iv2)
  end

  def test_cbc_invalid_iv
    assert_raises(ArgumentError) { M.cbc_encrypt("test".b, NIST_KEY, "short".b) }
    assert_raises(ArgumentError) { M.cbc_decrypt("\x00".b * 16, NIST_KEY, "short".b) }
  end

  def test_cbc_empty
    iv = "\x00".b * 16
    ct = M.cbc_encrypt("".b, NIST_KEY, iv)
    assert_equal "".b, M.cbc_decrypt(ct, NIST_KEY, iv)
  end

  def test_cbc_various_lengths
    iv = "\x00".b * 16
    [1, 15, 16, 17, 31, 32, 48, 100].each do |length|
      pt = (0...length).map { |i| i % 256 }.pack("C*")
      ct = M.cbc_encrypt(pt, NIST_KEY, iv)
      assert_equal pt, M.cbc_decrypt(ct, NIST_KEY, iv), "Failed for length #{length}"
    end
  end

  # =========================================================================
  # CTR Mode
  # =========================================================================

  def test_ctr_roundtrip
    nonce = "\x00".b * 12
    pt = "Hello, CTR mode! This is a test of counter mode encryption.".b
    ct = M.ctr_encrypt(pt, NIST_KEY, nonce)
    assert_equal pt, M.ctr_decrypt(ct, NIST_KEY, nonce)
  end

  def test_ctr_same_length
    nonce = "\x00".b * 12
    [1, 5, 15, 16, 17, 31, 32, 100].each do |length|
      pt = "A".b * length
      ct = M.ctr_encrypt(pt, NIST_KEY, nonce)
      assert_equal length, ct.bytesize
    end
  end

  def test_ctr_nonce_reuse_attack
    nonce = "\x00".b * 12
    p1 = "Attack at dawn!!".b
    p2 = "Attack at dusk!!".b
    c1 = M.ctr_encrypt(p1, NIST_KEY, nonce)
    c2 = M.ctr_encrypt(p2, NIST_KEY, nonce)

    ct_xor = M.xor_bytes(c1, c2)
    pt_xor = M.xor_bytes(p1, p2)
    assert_equal ct_xor, pt_xor
  end

  def test_ctr_invalid_nonce
    assert_raises(ArgumentError) { M.ctr_encrypt("test".b, NIST_KEY, "short".b) }
  end

  def test_ctr_empty
    nonce = "\x00".b * 12
    ct = M.ctr_encrypt("".b, NIST_KEY, nonce)
    assert_equal "".b, ct
  end

  def test_ctr_decrypt_is_encrypt
    nonce = "\x00".b * 12
    pt = "Symmetric!".b
    ct = M.ctr_encrypt(pt, NIST_KEY, nonce)
    assert_equal pt, M.ctr_encrypt(ct, NIST_KEY, nonce)
  end

  # =========================================================================
  # GCM Mode
  # =========================================================================

  GCM_KEY = ["feffe9928665731c6d6a8f9467308308"].pack("H*")
  GCM_IV = ["cafebabefacedbaddecaf888"].pack("H*")

  GCM_PT = [
    "d9313225f88406e5a55909c5aff5269a",
    "86a7a9531534f7da2e4c303d8a318a72",
    "1c3c0c95956809532fcf0e2449a6b525",
    "b16aedf5aa0de657ba637b391aafd255"
  ].map { |h| [h].pack("H*") }.join

  GCM_CT = [
    "42831ec2217774244b7221b784d0d49c",
    "e3aa212f2c02a4e035c17e2329aca12e",
    "21d514b25466931c7d8f6a5aac84aa05",
    "1ba30b396a0aac973d58e091473f5985"
  ].map { |h| [h].pack("H*") }.join

  GCM_TAG = ["4d5c2af327cd64a62cf35abd2ba6fab4"].pack("H*")

  # Test Case 4
  GCM_AAD_TC4 = ["feedfacedeadbeeffeedfacedeadbeefabaddad2"].pack("H*")
  GCM_PT_TC4 = [
    "d9313225f88406e5a55909c5aff5269a",
    "86a7a9531534f7da2e4c303d8a318a72",
    "1c3c0c95956809532fcf0e2449a6b525",
    "b16aedf5aa0de657ba637b39"
  ].map { |h| [h].pack("H*") }.join

  GCM_CT_TC4 = [
    "42831ec2217774244b7221b784d0d49c",
    "e3aa212f2c02a4e035c17e2329aca12e",
    "21d514b25466931c7d8f6a5aac84aa05",
    "1ba30b396a0aac973d58e091"
  ].map { |h| [h].pack("H*") }.join

  GCM_TAG_TC4 = ["5bc94fbc3221a5db94fae95ae7121a47"].pack("H*")

  def test_gcm_encrypt_nist_tc3
    ct, tag = M.gcm_encrypt(GCM_PT, GCM_KEY, GCM_IV)
    assert_equal GCM_CT, ct
    assert_equal GCM_TAG, tag
  end

  def test_gcm_decrypt_nist_tc3
    pt = M.gcm_decrypt(GCM_CT, GCM_KEY, GCM_IV, "".b, GCM_TAG)
    assert_equal GCM_PT, pt
  end

  def test_gcm_encrypt_nist_tc4
    ct, tag = M.gcm_encrypt(GCM_PT_TC4, GCM_KEY, GCM_IV, GCM_AAD_TC4)
    assert_equal GCM_CT_TC4, ct
    assert_equal GCM_TAG_TC4, tag
  end

  def test_gcm_decrypt_nist_tc4
    pt = M.gcm_decrypt(GCM_CT_TC4, GCM_KEY, GCM_IV, GCM_AAD_TC4, GCM_TAG_TC4)
    assert_equal GCM_PT_TC4, pt
  end

  def test_gcm_roundtrip
    pt = "Hello, GCM! This is authenticated encryption.".b
    aad = "additional data".b
    ct, tag = M.gcm_encrypt(pt, GCM_KEY, GCM_IV, aad)
    assert_equal pt, M.gcm_decrypt(ct, GCM_KEY, GCM_IV, aad, tag)
  end

  def test_gcm_tampered_ciphertext
    pt = "Secret message!".b
    ct, tag = M.gcm_encrypt(pt, GCM_KEY, GCM_IV)
    tampered = ct.dup
    tampered.setbyte(0, tampered.getbyte(0) ^ 1)
    assert_raises(ArgumentError) { M.gcm_decrypt(tampered, GCM_KEY, GCM_IV, "".b, tag) }
  end

  def test_gcm_tampered_aad
    pt = "Secret message!".b
    aad = "metadata".b
    ct, tag = M.gcm_encrypt(pt, GCM_KEY, GCM_IV, aad)
    assert_raises(ArgumentError) { M.gcm_decrypt(ct, GCM_KEY, GCM_IV, "wrong".b, tag) }
  end

  def test_gcm_tampered_tag
    pt = "Secret message!".b
    ct, tag = M.gcm_encrypt(pt, GCM_KEY, GCM_IV)
    bad_tag = tag.dup
    bad_tag.setbyte(0, bad_tag.getbyte(0) ^ 1)
    assert_raises(ArgumentError) { M.gcm_decrypt(ct, GCM_KEY, GCM_IV, "".b, bad_tag) }
  end

  def test_gcm_empty_plaintext
    aad = "authenticate only".b
    ct, tag = M.gcm_encrypt("".b, GCM_KEY, GCM_IV, aad)
    assert_equal "".b, ct
    assert_equal 16, tag.bytesize
    assert_equal "".b, M.gcm_decrypt(ct, GCM_KEY, GCM_IV, aad, tag)
  end

  def test_gcm_invalid_iv
    assert_raises(ArgumentError) { M.gcm_encrypt("test".b, GCM_KEY, "short".b) }
  end

  def test_gcm_invalid_tag
    assert_raises(ArgumentError) { M.gcm_decrypt("test".b, GCM_KEY, GCM_IV, "".b, "short".b) }
  end
end
