# frozen_string_literal: true

# Tests for coding_adventures_des — DES and 3DES block cipher.
#
# Coverage targets:
#   - NIST FIPS 46 / SP 800-20 known-answer test vectors
#   - Key schedule (expand_key): 16 subkeys of 6 bytes each
#   - Single-block encrypt/decrypt
#   - Round-trip: decrypt(encrypt(x)) == x
#   - ECB mode: multi-block, PKCS#7 padding, boundary conditions
#   - 3DES (TDEA) encrypt/decrypt
#   - Backward compatibility: K1=K2=K3 reduces to single DES
#   - Error handling: invalid key/block lengths, bad ciphertext

require "minitest/autorun"
require_relative "../lib/coding_adventures_des"

# Shorthand: decode a hex string (spaces ignored) to a binary String.
def h(hex_str)
  [hex_str.gsub(/\s+/, "")].pack("H*")
end

module CodingAdventures
  class TestDesEncryptBlock < Minitest::Test
    def test_fips_vector_1
      # Classic DES example from Stallings / FIPS 46 worked example.
      key   = h("133457799BBCDFF1")
      plain = h("0123456789ABCDEF")
      assert_equal h("85E813540F0AB405"), Des.des_encrypt_block(plain, key)
    end

    def test_sp800_20_table1_row0
      key   = h("0101010101010101")
      plain = h("95F8A5E5DD31D900")
      assert_equal h("8000000000000000"), Des.des_encrypt_block(plain, key)
    end

    def test_sp800_20_table1_row1
      key   = h("0101010101010101")
      plain = h("DD7F121CA5015619")
      assert_equal h("4000000000000000"), Des.des_encrypt_block(plain, key)
    end

    def test_sp800_20_table2_key_variable
      key   = h("8001010101010101")
      plain = h("0000000000000000")
      assert_equal h("95A8D72813DAA94D"), Des.des_encrypt_block(plain, key)
    end

    def test_sp800_20_table2_row1
      key   = h("4001010101010101")
      plain = h("0000000000000000")
      assert_equal h("0EEC1487DD8C26D5"), Des.des_encrypt_block(plain, key)
    end
  end

  class TestDesDecryptBlock < Minitest::Test
    def test_decrypt_fips_vector_1
      key    = h("133457799BBCDFF1")
      cipher = h("85E813540F0AB405")
      assert_equal h("0123456789ABCDEF"), Des.des_decrypt_block(cipher, key)
    end

    def test_roundtrip_fips_vector
      key   = h("133457799BBCDFF1")
      plain = h("0123456789ABCDEF")
      assert_equal plain, Des.des_decrypt_block(Des.des_encrypt_block(plain, key), key)
    end

    def test_roundtrip_multiple_keys
      keys = [
        h("133457799BBCDFF0"),
        h("FFFFFFFFFFFFFFFF"),
        h("0000000000000000"),
        h("FEDCBA9876543210"),
      ]
      plain = h("0123456789ABCDEF")
      keys.each do |key|
        assert_equal plain, Des.des_decrypt_block(Des.des_encrypt_block(plain, key), key)
      end
    end

    def test_roundtrip_all_byte_ranges
      key = h("FEDCBA9876543210")
      (0...256).step(16) do |start|
        block = (start...(start + 8)).map { |v| v & 0xFF }.pack("C*")
        assert_equal block, Des.des_decrypt_block(Des.des_encrypt_block(block, key), key)
      end
    end
  end

  class TestExpandKey < Minitest::Test
    KEY = h("0133457799BBCDFF")

    def test_returns_16_subkeys
      assert_equal 16, Des.expand_key(KEY).length
    end

    def test_each_subkey_is_6_bytes
      Des.expand_key(KEY).each do |sk|
        assert_equal 6, sk.bytesize
      end
    end

    def test_different_keys_different_subkeys
      sk1 = Des.expand_key(h("0133457799BBCDFF"))
      sk2 = Des.expand_key(h("FEDCBA9876543210"))
      refute_equal sk1, sk2
    end

    def test_subkeys_not_all_same
      subkeys = Des.expand_key(KEY)
      assert_operator subkeys.uniq.length, :>, 1
    end

    def test_invalid_key_too_short
      err = assert_raises(ArgumentError) { Des.expand_key("\x00" * 7) }
      assert_match(/8 bytes/, err.message)
    end

    def test_invalid_key_too_long
      err = assert_raises(ArgumentError) { Des.expand_key("\x00" * 9) }
      assert_match(/8 bytes/, err.message)
    end
  end

  class TestEcbEncrypt < Minitest::Test
    KEY = h("0133457799BBCDFF")

    def test_8_byte_input_gives_16_bytes_out
      ct = Des.des_ecb_encrypt(h("0123456789ABCDEF"), KEY)
      assert_equal 16, ct.bytesize
    end

    def test_sub_block_gives_one_block_out
      ct = Des.des_ecb_encrypt("hello", KEY)
      assert_equal 8, ct.bytesize
    end

    def test_16_byte_input_gives_24_bytes_out
      ct = Des.des_ecb_encrypt("\x00" * 16, KEY)
      assert_equal 24, ct.bytesize
    end

    def test_empty_input_gives_8_bytes_out
      ct = Des.des_ecb_encrypt("", KEY)
      assert_equal 8, ct.bytesize
    end

    def test_output_is_binary_string
      ct = Des.des_ecb_encrypt("test", KEY)
      assert_kind_of String, ct
    end

    def test_deterministic
      plain = "Hello, World!!!"
      assert_equal Des.des_ecb_encrypt(plain, KEY), Des.des_ecb_encrypt(plain, KEY)
    end
  end

  class TestEcbDecrypt < Minitest::Test
    KEY = h("0133457799BBCDFF")

    def test_roundtrip_short
      plain = "hello"
      assert_equal plain.b, Des.des_ecb_decrypt(Des.des_ecb_encrypt(plain, KEY), KEY)
    end

    def test_roundtrip_exact_block
      plain = "ABCDEFGH"
      assert_equal plain.b, Des.des_ecb_decrypt(Des.des_ecb_encrypt(plain, KEY), KEY)
    end

    def test_roundtrip_multi_block
      plain = "The quick brown fox jumps"
      assert_equal plain.b, Des.des_ecb_decrypt(Des.des_ecb_encrypt(plain, KEY), KEY)
    end

    def test_roundtrip_empty
      plain = ""
      assert_equal plain.b, Des.des_ecb_decrypt(Des.des_ecb_encrypt(plain, KEY), KEY)
    end

    def test_roundtrip_256_bytes
      plain = (0..255).map(&:chr).join.b
      assert_equal plain, Des.des_ecb_decrypt(Des.des_ecb_encrypt(plain, KEY), KEY)
    end

    def test_invalid_length_not_multiple_of_8
      err = assert_raises(ArgumentError) { Des.des_ecb_decrypt("\x00" * 7, KEY) }
      assert_match(/multiple of 8/, err.message)
    end

    def test_empty_ciphertext_raises
      assert_raises(ArgumentError) { Des.des_ecb_decrypt("", KEY) }
    end

    def test_corrupted_padding_raises
      ct = Des.des_ecb_encrypt("test data", KEY)
      corrupted = ct.dup.b
      corrupted.setbyte(-1, corrupted.bytes.last ^ 0xFF)
      assert_raises(ArgumentError) { Des.des_ecb_decrypt(corrupted, KEY) }
    end
  end

  class TestTdea < Minitest::Test
    K1    = h("0123456789ABCDEF")
    K2    = h("23456789ABCDEF01")
    K3    = h("456789ABCDEF0123")
    PLAIN = h("6BC1BEE22E409F96")
    CIPHER = h("3B6423D418DEFC23")

    def test_tdea_encrypt
      assert_equal CIPHER, Des.tdea_encrypt_block(PLAIN, K1, K2, K3)
    end

    def test_tdea_decrypt
      assert_equal PLAIN, Des.tdea_decrypt_block(CIPHER, K1, K2, K3)
    end

    def test_roundtrip_random_keys
      k1 = h("FEDCBA9876543210")
      k2 = h("0F1E2D3C4B5A6978")
      k3 = h("7869584A3B2C1D0E")
      plain = h("0123456789ABCDEF")
      ct = Des.tdea_encrypt_block(plain, k1, k2, k3)
      assert_equal plain, Des.tdea_decrypt_block(ct, k1, k2, k3)
    end

    def test_ede_backward_compat_k1_eq_k2_eq_k3
      # When K1=K2=K3, 3DES EDE reduces to single DES.
      # E(K, D(K, E(K, P))) = E(K, P)
      key   = h("0133457799BBCDFF")
      plain = h("0123456789ABCDEF")
      assert_equal Des.des_encrypt_block(plain, key), Des.tdea_encrypt_block(plain, key, key, key)
    end

    def test_ede_decrypt_backward_compat
      key = h("FEDCBA9876543210")
      ct  = h("0123456789ABCDEF")
      assert_equal Des.des_decrypt_block(ct, key), Des.tdea_decrypt_block(ct, key, key, key)
    end

    def test_roundtrip_repeated_byte_patterns
      k1 = h("1234567890ABCDEF")
      k2 = h("FEDCBA0987654321")
      k3 = h("0F0F0F0F0F0F0F0F")
      [0x00, 0xFF, 0xA5, 0x5A].each do |val|
        plain = val.chr * 8
        ct = Des.tdea_encrypt_block(plain, k1, k2, k3)
        assert_equal plain.b, Des.tdea_decrypt_block(ct, k1, k2, k3)
      end
    end
  end

  class TestInvalidInputs < Minitest::Test
    KEY = h("0133457799BBCDFF")

    def test_encrypt_block_wrong_block_size_short
      err = assert_raises(ArgumentError) { Des.des_encrypt_block("\x00" * 7, KEY) }
      assert_match(/8 bytes/, err.message)
    end

    def test_encrypt_block_wrong_block_size_long
      err = assert_raises(ArgumentError) { Des.des_encrypt_block("\x00" * 16, KEY) }
      assert_match(/8 bytes/, err.message)
    end

    def test_decrypt_block_wrong_block_size
      err = assert_raises(ArgumentError) { Des.des_decrypt_block("\x00" * 9, KEY) }
      assert_match(/8 bytes/, err.message)
    end

    def test_encrypt_wrong_key_size
      err = assert_raises(ArgumentError) { Des.des_encrypt_block("\x00" * 8, "\x00" * 4) }
      assert_match(/8 bytes/, err.message)
    end

    def test_decrypt_wrong_key_size
      err = assert_raises(ArgumentError) { Des.des_decrypt_block("\x00" * 8, "\x00" * 16) }
      assert_match(/8 bytes/, err.message)
    end
  end
end
