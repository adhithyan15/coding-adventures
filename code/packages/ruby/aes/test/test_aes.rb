# frozen_string_literal: true

# Tests for coding_adventures_aes — AES block cipher.
#
# Coverage targets:
#   - FIPS 197 Appendix B and C known-answer tests (all three key sizes)
#   - S-box properties: bijection, known values, no fixed points
#   - Key schedule: correct round count, word structure
#   - Single-block encrypt/decrypt
#   - Round-trip: decrypt(encrypt(x)) == x for all key sizes
#   - Avalanche effect
#   - Error handling: wrong block/key lengths

require "minitest/autorun"
require_relative "../lib/coding_adventures_aes"

def h(hex_str)
  [hex_str.gsub(/\s+/, "")].pack("H*")
end

module CodingAdventures
  class TestAes128 < Minitest::Test
    # FIPS 197 Appendix B
    KEY    = h("2b7e151628aed2a6abf7158809cf4f3c")
    PLAIN  = h("3243f6a8885a308d313198a2e0370734")
    CIPHER = h("3925841d02dc09fbdc118597196a0b32")

    def test_encrypt
      assert_equal CIPHER, Aes.aes_encrypt_block(PLAIN, KEY)
    end

    def test_decrypt
      assert_equal PLAIN, Aes.aes_decrypt_block(CIPHER, KEY)
    end

    def test_roundtrip_multiple_plaintexts
      (0...256).step(32) do |start|
        plain = (start...(start + 16)).map { |v| v & 0xFF }.pack("C*")
        ct = Aes.aes_encrypt_block(plain, KEY)
        assert_equal plain, Aes.aes_decrypt_block(ct, KEY)
      end
    end

    def test_appendix_c1
      # FIPS 197 Appendix C.1
      key   = h("000102030405060708090a0b0c0d0e0f")
      plain = h("00112233445566778899aabbccddeeff")
      ct    = h("69c4e0d86a7b0430d8cdb78070b4c55a")
      assert_equal ct,    Aes.aes_encrypt_block(plain, key)
      assert_equal plain, Aes.aes_decrypt_block(ct, key)
    end
  end

  class TestAes192 < Minitest::Test
    # FIPS 197 Appendix C.2
    KEY    = h("000102030405060708090a0b0c0d0e0f1011121314151617")
    PLAIN  = h("00112233445566778899aabbccddeeff")
    CIPHER = h("dda97ca4864cdfe06eaf70a0ec0d7191")

    def test_encrypt
      assert_equal CIPHER, Aes.aes_encrypt_block(PLAIN, KEY)
    end

    def test_decrypt
      assert_equal PLAIN, Aes.aes_decrypt_block(CIPHER, KEY)
    end

    def test_roundtrip
      (0...256).step(32) do |start|
        plain = (start...(start + 16)).map { |v| v & 0xFF }.pack("C*")
        ct = Aes.aes_encrypt_block(plain, KEY)
        assert_equal plain, Aes.aes_decrypt_block(ct, KEY)
      end
    end
  end

  class TestAes256 < Minitest::Test
    KEY    = h("603deb1015ca71be2b73aef0857d7781 1f352c073b6108d72d9810a30914dff4")
    PLAIN  = h("6bc1bee22e409f96e93d7e117393172a")
    CIPHER = h("f3eed1bdb5d2a03c064b5a7e3db181f8")

    def test_encrypt
      assert_equal CIPHER, Aes.aes_encrypt_block(PLAIN, KEY)
    end

    def test_decrypt
      assert_equal PLAIN, Aes.aes_decrypt_block(CIPHER, KEY)
    end

    def test_roundtrip
      (0...256).step(32) do |start|
        plain = (start...(start + 16)).map { |v| v & 0xFF }.pack("C*")
        ct = Aes.aes_encrypt_block(plain, KEY)
        assert_equal plain, Aes.aes_decrypt_block(ct, KEY)
      end
    end

    def test_appendix_c3
      # FIPS 197 Appendix C.3
      key   = h("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f")
      plain = h("00112233445566778899aabbccddeeff")
      ct    = h("8ea2b7ca516745bfeafc49904b496089")
      assert_equal ct,    Aes.aes_encrypt_block(plain, key)
      assert_equal plain, Aes.aes_decrypt_block(ct, key)
    end
  end

  class TestSbox < Minitest::Test
    def test_sbox_length
      assert_equal 256, Aes::SBOX.length
    end

    def test_inv_sbox_length
      assert_equal 256, Aes::INV_SBOX.length
    end

    def test_sbox_is_bijection
      assert_equal (0..255).to_a, Aes::SBOX.sort
    end

    def test_inv_sbox_is_bijection
      assert_equal (0..255).to_a, Aes::INV_SBOX.sort
    end

    def test_sbox_inv_sbox_inverse
      256.times do |b|
        assert_equal b, Aes::INV_SBOX[Aes::SBOX[b]]
      end
    end

    def test_sbox_known_values
      # FIPS 197 Figure 7
      assert_equal 0x63, Aes::SBOX[0x00]
      assert_equal 0x7c, Aes::SBOX[0x01]
      assert_equal 0x16, Aes::SBOX[0xff]
      assert_equal 0xed, Aes::SBOX[0x53]
    end

    def test_inv_sbox_known_values
      assert_equal 0x00, Aes::INV_SBOX[0x63]
      assert_equal 0x01, Aes::INV_SBOX[0x7c]
    end

    def test_no_fixed_points
      256.times do |b|
        refute_equal b, Aes::SBOX[b], "Fixed point at #{b.to_s(16)}"
      end
    end
  end

  class TestExpandKey < Minitest::Test
    def test_aes128_round_count
      assert_equal 11, Aes.expand_key(("\x00" * 16).b).length
    end

    def test_aes192_round_count
      assert_equal 13, Aes.expand_key(("\x00" * 24).b).length
    end

    def test_aes256_round_count
      assert_equal 15, Aes.expand_key(("\x00" * 32).b).length
    end

    def test_round_key_shape
      [16, 24, 32].each do |key_len|
        rks = Aes.expand_key(("\x00" * key_len).b)
        rks.each do |rk|
          assert_equal 4, rk.length
          rk.each do |row|
            assert_equal 4, row.length
            row.each { |v| assert_includes 0..255, v }
          end
        end
      end
    end

    def test_first_round_key_equals_key
      key = h("2b7e151628aed2a6abf7158809cf4f3c")
      rks = Aes.expand_key(key)
      # Reconstruct first 16 bytes column-major
      reconstructed = Array.new(16)
      4.times do |col|
        4.times do |row|
          reconstructed[row + 4 * col] = rks[0][row][col]
        end
      end
      assert_equal key.bytes, reconstructed
    end

    def test_different_keys_different_round_keys
      rks1 = Aes.expand_key(("\x00" * 16).b)
      rks2 = Aes.expand_key(("\x01" * 16).b)
      refute_equal rks1[0], rks2[0]
    end

    def test_invalid_key_15_bytes
      err = assert_raises(ArgumentError) { Aes.expand_key(("\x00" * 15).b) }
      assert_match(/16, 24, or 32/, err.message)
    end

    def test_invalid_key_17_bytes
      err = assert_raises(ArgumentError) { Aes.expand_key(("\x00" * 17).b) }
      assert_match(/16, 24, or 32/, err.message)
    end
  end

  class TestBlockValidation < Minitest::Test
    KEY = ("\x00" * 16).b.freeze

    def test_encrypt_block_too_short
      err = assert_raises(ArgumentError) { Aes.aes_encrypt_block(("\x00" * 15).b, KEY) }
      assert_match(/16 bytes/, err.message)
    end

    def test_encrypt_block_too_long
      err = assert_raises(ArgumentError) { Aes.aes_encrypt_block(("\x00" * 17).b, KEY) }
      assert_match(/16 bytes/, err.message)
    end

    def test_decrypt_block_wrong_size
      err = assert_raises(ArgumentError) { Aes.aes_decrypt_block(("\x00" * 15).b, KEY) }
      assert_match(/16 bytes/, err.message)
    end

    def test_encrypt_wrong_key_size
      err = assert_raises(ArgumentError) { Aes.aes_encrypt_block(("\x00" * 16).b, ("\x00" * 10).b) }
      assert_match(/16, 24, or 32/, err.message)
    end

    def test_decrypt_wrong_key_size
      err = assert_raises(ArgumentError) { Aes.aes_decrypt_block(("\x00" * 16).b, ("\x00" * 20).b) }
      assert_match(/16, 24, or 32/, err.message)
    end
  end

  class TestRoundtrip < Minitest::Test
    def test_all_zeros
      [16, 24, 32].each do |key_len|
        key = ("\x00" * key_len).b
        plain = ("\x00" * 16).b
        assert_equal plain, Aes.aes_decrypt_block(Aes.aes_encrypt_block(plain, key), key)
      end
    end

    def test_all_ff
      [16, 24, 32].each do |key_len|
        key = ("\xFF" * key_len).b
        plain = ("\xFF" * 16).b
        assert_equal plain, Aes.aes_decrypt_block(Aes.aes_encrypt_block(plain, key), key)
      end
    end

    def test_sequential_bytes
      [16, 24, 32].each do |key_len|
        key = (0...key_len).map { |i| i.chr }.join.b
        plain = (0...16).map { |i| i.chr }.join.b
        assert_equal plain, Aes.aes_decrypt_block(Aes.aes_encrypt_block(plain, key), key)
      end
    end

    def test_avalanche_effect
      key = (0...16).map { |i| i.chr }.join.b
      plain1 = ("\x00" * 16).b
      plain2 = ("\x01" + "\x00" * 15).b
      ct1 = Aes.aes_encrypt_block(plain1, key)
      ct2 = Aes.aes_encrypt_block(plain2, key)
      diff_bits = ct1.bytes.zip(ct2.bytes).sum { |a, b| (a ^ b).to_s(2).count("1") }
      assert_operator diff_bits, :>, 32, "Only #{diff_bits} bits differ — poor diffusion"
    end
  end
end
