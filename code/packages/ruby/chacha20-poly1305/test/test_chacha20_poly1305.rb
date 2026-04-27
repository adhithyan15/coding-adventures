# frozen_string_literal: true

# Tests for ChaCha20-Poly1305 (RFC 8439)
# =======================================
#
# These tests verify the implementation against the official test vectors
# from RFC 8439 (Sections 2.4.2, 2.5.2, and 2.8.2), plus additional
# edge-case and property tests.

require "minitest/autorun"
require_relative "../lib/coding_adventures_chacha20_poly1305"

class TestChaCha20Poly1305 < Minitest::Test
  CC = CodingAdventures::Chacha20Poly1305

  # ===================================================================
  # RFC 8439 Test Vectors
  # ===================================================================

  CHACHA20_KEY = ["000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"].pack("H*")
  CHACHA20_NONCE = ["000000000000004a00000000"].pack("H*")
  CHACHA20_COUNTER = 1
  CHACHA20_PLAINTEXT = "Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.".b
  CHACHA20_EXPECTED_CT = [
    "6e2e359a2568f98041ba0728dd0d6981" \
    "e97e7aec1d4360c20a27afccfd9fae0b" \
    "f91b65c5524733ab8f593dabcd62b357" \
    "1639d624e65152ab8f530c359f0861d8" \
    "07ca0dbf500d6a6156a38e088a22b65e" \
    "52bc514d16ccf806818ce91ab7793736" \
    "5af90bbf74a35be6b40b8eedf2785e42" \
    "874d"
  ].pack("H*")

  POLY1305_KEY = ["85d6be7857556d337f4452fe42d506a80103808afb0db2fd4abff6af4149f51b"].pack("H*")
  POLY1305_MESSAGE = "Cryptographic Forum Research Group".b
  POLY1305_EXPECTED_TAG = ["a8061dc1305136c6c22b8baf0c0127a9"].pack("H*")

  AEAD_KEY = ["808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9f"].pack("H*")
  AEAD_NONCE = ["070000004041424344454647"].pack("H*")
  AEAD_AAD = ["50515253c0c1c2c3c4c5c6c7"].pack("H*")
  AEAD_PLAINTEXT = "Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.".b
  AEAD_EXPECTED_CT = [
    "d31a8d34648e60db7b86afbc53ef7ec2" \
    "a4aded51296e08fea9e2b5a736ee62d6" \
    "3dbea45e8ca9671282fafb69da92728b" \
    "1a71de0a9e060b2905d6a5b67ecd3b36" \
    "92ddbd7f2d778b8c9803aee328091b58" \
    "fab324e4fad675945585808b4831d7bc" \
    "3ff4def08e4b7a9de576d26586cec64b" \
    "6116"
  ].pack("H*")
  AEAD_EXPECTED_TAG = ["1ae10b594f09e26a7e902ecbd0600691"].pack("H*")

  # ===================================================================
  # ChaCha20 Tests
  # ===================================================================

  def test_chacha20_rfc_section_2_4_2
    ct = CC.chacha20_encrypt(CHACHA20_PLAINTEXT, CHACHA20_KEY, CHACHA20_NONCE, CHACHA20_COUNTER)
    assert_equal CHACHA20_EXPECTED_CT, ct
  end

  def test_chacha20_decrypt_is_encrypt
    ct = CC.chacha20_encrypt(CHACHA20_PLAINTEXT, CHACHA20_KEY, CHACHA20_NONCE, CHACHA20_COUNTER)
    pt = CC.chacha20_encrypt(ct, CHACHA20_KEY, CHACHA20_NONCE, CHACHA20_COUNTER)
    assert_equal CHACHA20_PLAINTEXT, pt
  end

  def test_chacha20_empty_plaintext
    ct = CC.chacha20_encrypt("".b, CHACHA20_KEY, CHACHA20_NONCE, 0)
    assert_equal "".b, ct
  end

  def test_chacha20_single_byte
    ct = CC.chacha20_encrypt("\x00".b, CHACHA20_KEY, CHACHA20_NONCE, 0)
    assert_equal 1, ct.bytesize
    pt = CC.chacha20_encrypt(ct, CHACHA20_KEY, CHACHA20_NONCE, 0)
    assert_equal "\x00".b, pt
  end

  def test_chacha20_exactly_64_bytes
    data = (0...64).map(&:chr).join.b
    ct = CC.chacha20_encrypt(data, CHACHA20_KEY, CHACHA20_NONCE, 0)
    pt = CC.chacha20_encrypt(ct, CHACHA20_KEY, CHACHA20_NONCE, 0)
    assert_equal data, pt
  end

  def test_chacha20_multi_block
    data = ((0...256).map { |i| (i % 256).chr }.join * 2).b
    ct = CC.chacha20_encrypt(data, CHACHA20_KEY, CHACHA20_NONCE, 0)
    pt = CC.chacha20_encrypt(ct, CHACHA20_KEY, CHACHA20_NONCE, 0)
    assert_equal data, pt
  end

  def test_chacha20_invalid_key_length
    assert_raises(ArgumentError) { CC.chacha20_encrypt("hello".b, "short".b, CHACHA20_NONCE, 0) }
  end

  def test_chacha20_invalid_nonce_length
    assert_raises(ArgumentError) { CC.chacha20_encrypt("hello".b, CHACHA20_KEY, "short".b, 0) }
  end

  # ===================================================================
  # Poly1305 Tests
  # ===================================================================

  def test_poly1305_rfc_section_2_5_2
    tag = CC.poly1305_mac(POLY1305_MESSAGE, POLY1305_KEY)
    assert_equal POLY1305_EXPECTED_TAG, tag
  end

  def test_poly1305_empty_message
    tag = CC.poly1305_mac("".b, POLY1305_KEY)
    assert_equal 16, tag.bytesize
  end

  def test_poly1305_single_byte
    tag = CC.poly1305_mac("\x00".b, POLY1305_KEY)
    assert_equal 16, tag.bytesize
  end

  def test_poly1305_exactly_16_bytes
    tag = CC.poly1305_mac(("\x00" * 16).b, POLY1305_KEY)
    assert_equal 16, tag.bytesize
  end

  def test_poly1305_different_messages_different_tags
    tag1 = CC.poly1305_mac("hello".b, POLY1305_KEY)
    tag2 = CC.poly1305_mac("world".b, POLY1305_KEY)
    refute_equal tag1, tag2
  end

  def test_poly1305_different_keys_different_tags
    key2 = (0...32).map(&:chr).join.b
    tag1 = CC.poly1305_mac("hello".b, POLY1305_KEY)
    tag2 = CC.poly1305_mac("hello".b, key2)
    refute_equal tag1, tag2
  end

  def test_poly1305_invalid_key_length
    assert_raises(ArgumentError) { CC.poly1305_mac("hello".b, "short".b) }
  end

  # ===================================================================
  # AEAD Tests
  # ===================================================================

  def test_aead_encrypt_rfc_section_2_8_2
    ct, tag = CC.aead_encrypt(AEAD_PLAINTEXT, AEAD_KEY, AEAD_NONCE, AEAD_AAD)
    assert_equal AEAD_EXPECTED_CT, ct
    assert_equal AEAD_EXPECTED_TAG, tag
  end

  def test_aead_decrypt_rfc_section_2_8_2
    pt = CC.aead_decrypt(AEAD_EXPECTED_CT, AEAD_KEY, AEAD_NONCE, AEAD_AAD, AEAD_EXPECTED_TAG)
    assert_equal AEAD_PLAINTEXT, pt
  end

  def test_aead_roundtrip
    key = (0...32).map(&:chr).join.b
    nonce = (0...12).map(&:chr).join.b
    aad = "additional data".b
    plaintext = "Hello, ChaCha20-Poly1305!".b

    ct, tag = CC.aead_encrypt(plaintext, key, nonce, aad)
    pt = CC.aead_decrypt(ct, key, nonce, aad, tag)
    assert_equal plaintext, pt
  end

  def test_aead_empty_plaintext
    key = (0...32).map(&:chr).join.b
    nonce = (0...12).map(&:chr).join.b
    aad = "authenticate only".b

    ct, tag = CC.aead_encrypt("".b, key, nonce, aad)
    assert_equal "".b, ct
    assert_equal 16, tag.bytesize

    pt = CC.aead_decrypt(ct, key, nonce, aad, tag)
    assert_equal "".b, pt
  end

  def test_aead_empty_aad
    key = (0...32).map(&:chr).join.b
    nonce = (0...12).map(&:chr).join.b

    ct, tag = CC.aead_encrypt("secret".b, key, nonce, "".b)
    pt = CC.aead_decrypt(ct, key, nonce, "".b, tag)
    assert_equal "secret".b, pt
  end

  def test_aead_tampered_ciphertext_fails
    key = (0...32).map(&:chr).join.b
    nonce = (0...12).map(&:chr).join.b
    aad = "aad".b

    ct, tag = CC.aead_encrypt("secret message".b, key, nonce, aad)

    tampered = ct.dup
    tampered.setbyte(0, tampered.getbyte(0) ^ 0x01)

    assert_raises(RuntimeError) { CC.aead_decrypt(tampered, key, nonce, aad, tag) }
  end

  def test_aead_tampered_tag_fails
    key = (0...32).map(&:chr).join.b
    nonce = (0...12).map(&:chr).join.b

    ct, _tag = CC.aead_encrypt("secret".b, key, nonce, "".b)

    bad_tag = ("\x00" * 16).b
    assert_raises(RuntimeError) { CC.aead_decrypt(ct, key, nonce, "".b, bad_tag) }
  end

  def test_aead_wrong_aad_fails
    key = (0...32).map(&:chr).join.b
    nonce = (0...12).map(&:chr).join.b

    ct, tag = CC.aead_encrypt("secret".b, key, nonce, "correct aad".b)
    assert_raises(RuntimeError) { CC.aead_decrypt(ct, key, nonce, "wrong aad".b, tag) }
  end

  def test_aead_wrong_key_fails
    key1 = (0...32).map(&:chr).join.b
    key2 = (1...33).map { |i| (i % 256).chr }.join.b
    nonce = (0...12).map(&:chr).join.b

    ct, tag = CC.aead_encrypt("secret".b, key1, nonce, "".b)
    assert_raises(RuntimeError) { CC.aead_decrypt(ct, key2, nonce, "".b, tag) }
  end

  def test_aead_wrong_nonce_fails
    key = (0...32).map(&:chr).join.b
    nonce1 = (0...12).map(&:chr).join.b
    nonce2 = (1...13).map { |i| (i % 256).chr }.join.b

    ct, tag = CC.aead_encrypt("secret".b, key, nonce1, "".b)
    assert_raises(RuntimeError) { CC.aead_decrypt(ct, key, nonce2, "".b, tag) }
  end

  def test_aead_invalid_key_length_encrypt
    assert_raises(ArgumentError) { CC.aead_encrypt("hello".b, "short".b, ("\x00" * 12).b) }
  end

  def test_aead_invalid_nonce_length_encrypt
    assert_raises(ArgumentError) { CC.aead_encrypt("hello".b, ("\x00" * 32).b, "short".b) }
  end

  def test_aead_invalid_tag_length_decrypt
    assert_raises(ArgumentError) { CC.aead_decrypt("hello".b, ("\x00" * 32).b, ("\x00" * 12).b, "".b, "short".b) }
  end

  def test_aead_large_plaintext
    key = (0...32).map(&:chr).join.b
    nonce = (0...12).map(&:chr).join.b
    plaintext = ("A" * 1024).b

    ct, tag = CC.aead_encrypt(plaintext, key, nonce, "".b)
    pt = CC.aead_decrypt(ct, key, nonce, "".b, tag)
    assert_equal plaintext, pt
  end
end
