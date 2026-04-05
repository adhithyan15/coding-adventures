# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_vigenere_cipher"

# Tests for the Vigenere cipher implementation.
#
# The parity test vectors are shared across all 9 language implementations
# to ensure identical behavior.
class TestVigenereCipher < Minitest::Test
  VC = CodingAdventures::VigenereCipher

  # Long English text for cryptanalysis testing (~300 chars).
  LONG_ENGLISH_TEXT = "The quick brown fox jumps over the lazy dog near the riverbank where " \
    "the tall grass sways gently in the warm summer breeze and the birds " \
    "sing their melodious songs while the sun sets behind the distant " \
    "mountains casting long shadows across the peaceful valley below and " \
    "the farmers return from the golden fields carrying baskets of fresh " \
    "wheat and corn while their children play happily in the meadows " \
    "chasing butterflies and picking wildflowers that grow abundantly " \
    "along the winding country roads that lead through the ancient forest " \
    "where owls hoot softly in the towering oak trees above the mossy " \
    "ground covered with fallen leaves and acorns from the previous autumn"

  # -------------------------------------------------------------------------
  # Encryption Tests
  # -------------------------------------------------------------------------

  def test_encrypt_parity_attackatdawn
    assert_equal "LXFOPVEFRNHR", VC.encrypt("ATTACKATDAWN", "LEMON")
  end

  def test_encrypt_parity_mixed_case
    assert_equal "Rijvs, Uyvjn!", VC.encrypt("Hello, World!", "key")
  end

  def test_encrypt_empty
    assert_equal "", VC.encrypt("", "KEY")
  end

  def test_encrypt_single_char
    assert_equal "B", VC.encrypt("A", "B")
    assert_equal "Z", VC.encrypt("Z", "A")
    assert_equal "A", VC.encrypt("Z", "B")
  end

  def test_encrypt_preserves_non_alpha
    assert_equal "123!@#", VC.encrypt("123!@#", "key")
  end

  def test_encrypt_key_does_not_advance_on_non_alpha
    # Key "AB" (shifts 0, 1): 'A' +0 = 'A', space passes, 'A' +1 = 'B'
    assert_equal "A B", VC.encrypt("A A", "AB")
  end

  def test_encrypt_case_insensitive_key
    r1 = VC.encrypt("HELLO", "KEY")
    r2 = VC.encrypt("HELLO", "key")
    r3 = VC.encrypt("HELLO", "Key")
    assert_equal r1, r2
    assert_equal r2, r3
  end

  def test_encrypt_invalid_key_empty
    assert_raises(ArgumentError) { VC.encrypt("hello", "") }
  end

  def test_encrypt_invalid_key_non_alpha
    assert_raises(ArgumentError) { VC.encrypt("hello", "key1") }
  end

  def test_encrypt_long_key
    # H + A(0) = H, i + B(1) = j
    assert_equal "Hj", VC.encrypt("Hi", "ABCDEFGHIJ")
  end

  # -------------------------------------------------------------------------
  # Decryption Tests
  # -------------------------------------------------------------------------

  def test_decrypt_parity_attackatdawn
    assert_equal "ATTACKATDAWN", VC.decrypt("LXFOPVEFRNHR", "LEMON")
  end

  def test_decrypt_parity_mixed_case
    assert_equal "Hello, World!", VC.decrypt("Rijvs, Uyvjn!", "key")
  end

  def test_decrypt_empty
    assert_equal "", VC.decrypt("", "KEY")
  end

  def test_decrypt_single_char
    assert_equal "A", VC.decrypt("B", "B")
  end

  def test_decrypt_preserves_non_alpha
    assert_equal "123!@#", VC.decrypt("123!@#", "key")
  end

  def test_decrypt_invalid_key_empty
    assert_raises(ArgumentError) { VC.decrypt("hello", "") }
  end

  def test_decrypt_invalid_key_non_alpha
    assert_raises(ArgumentError) { VC.decrypt("hello", "k3y") }
  end

  # -------------------------------------------------------------------------
  # Round-Trip Tests
  # -------------------------------------------------------------------------

  def test_round_trip
    cases = [
      ["ATTACKATDAWN", "LEMON"],
      ["Hello, World!", "key"],
      ["The quick brown fox!", "SECRET"],
      ["abcdefghijklmnopqrstuvwxyz", "Z"],
      ["AAAAAA", "ABCDEF"],
      ["12345 numbers 67890", "test"],
      ["MiXeD CaSe TeXt!!!", "MiXeD"],
      ["", "anykey"]
    ]

    cases.each do |text, key|
      encrypted = VC.encrypt(text, key)
      decrypted = VC.decrypt(encrypted, key)
      assert_equal text, decrypted, "Round-trip failed for #{text.inspect} with key #{key.inspect}"
    end
  end

  # -------------------------------------------------------------------------
  # Cryptanalysis Tests
  # -------------------------------------------------------------------------

  def test_find_key_length_secret
    ciphertext = VC.encrypt(LONG_ENGLISH_TEXT, "SECRET")
    assert_equal 6, VC.find_key_length(ciphertext)
  end

  def test_find_key_length_lemon
    ciphertext = VC.encrypt(LONG_ENGLISH_TEXT, "LEMON")
    assert_equal 5, VC.find_key_length(ciphertext)
  end

  def test_find_key_length_short
    ciphertext = VC.encrypt(LONG_ENGLISH_TEXT, "DAWN")
    assert_equal 4, VC.find_key_length(ciphertext)
  end

  def test_find_key_secret
    ciphertext = VC.encrypt(LONG_ENGLISH_TEXT, "SECRET")
    assert_equal "SECRET", VC.find_key(ciphertext, 6)
  end

  def test_find_key_lemon
    ciphertext = VC.encrypt(LONG_ENGLISH_TEXT, "LEMON")
    assert_equal "LEMON", VC.find_key(ciphertext, 5)
  end

  def test_break_cipher_secret
    ciphertext = VC.encrypt(LONG_ENGLISH_TEXT, "SECRET")
    key, plaintext = VC.break_cipher(ciphertext)
    assert_equal "SECRET", key
    assert_equal LONG_ENGLISH_TEXT, plaintext
  end

  def test_break_cipher_lemon
    ciphertext = VC.encrypt(LONG_ENGLISH_TEXT, "LEMON")
    key, plaintext = VC.break_cipher(ciphertext)
    assert_equal "LEMON", key
    assert_equal LONG_ENGLISH_TEXT, plaintext
  end

  # -------------------------------------------------------------------------
  # Edge Cases
  # -------------------------------------------------------------------------

  def test_key_a_is_identity
    text = "Hello, World!"
    assert_equal text, VC.encrypt(text, "A")
  end

  def test_key_z_wraps
    assert_equal "Z", VC.encrypt("A", "Z")
    assert_equal "A", VC.encrypt("B", "Z")
  end

  def test_only_non_alpha
    assert_equal "123 !@# $%^", VC.encrypt("123 !@# $%^", "key")
    assert_equal "123 !@# $%^", VC.decrypt("123 !@# $%^", "key")
  end
end
