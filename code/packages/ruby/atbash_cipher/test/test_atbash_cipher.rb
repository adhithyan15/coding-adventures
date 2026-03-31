# frozen_string_literal: true

# Comprehensive tests for the Atbash cipher implementation.
#
# These tests verify that the Atbash cipher correctly reverses the alphabet
# for both uppercase and lowercase letters, preserves non-alphabetic
# characters, and satisfies the self-inverse property.

require "minitest/autorun"
require "coding_adventures_atbash_cipher"

class TestAtbashCipher < Minitest::Test
  # --- Version ---

  def test_version_exists
    refute_nil CodingAdventures::AtbashCipher::VERSION
    assert_equal "0.1.0", CodingAdventures::AtbashCipher::VERSION
  end

  # --- Basic Encryption ---

  def test_encrypt_hello_uppercase
    # H(7)->S(18), E(4)->V(21), L(11)->O(14), L(11)->O(14), O(14)->L(11)
    assert_equal "SVOOL", CodingAdventures::AtbashCipher.encrypt("HELLO")
  end

  def test_encrypt_hello_lowercase
    assert_equal "svool", CodingAdventures::AtbashCipher.encrypt("hello")
  end

  def test_encrypt_mixed_case_with_punctuation
    assert_equal "Svool, Dliow! 123", CodingAdventures::AtbashCipher.encrypt("Hello, World! 123")
  end

  def test_encrypt_full_uppercase_alphabet
    assert_equal "ZYXWVUTSRQPONMLKJIHGFEDCBA",
                 CodingAdventures::AtbashCipher.encrypt("ABCDEFGHIJKLMNOPQRSTUVWXYZ")
  end

  def test_encrypt_full_lowercase_alphabet
    assert_equal "zyxwvutsrqponmlkjihgfedcba",
                 CodingAdventures::AtbashCipher.encrypt("abcdefghijklmnopqrstuvwxyz")
  end

  # --- Case Preservation ---

  def test_uppercase_stays_uppercase
    result = CodingAdventures::AtbashCipher.encrypt("ABC")
    assert_equal "ZYX", result
  end

  def test_lowercase_stays_lowercase
    result = CodingAdventures::AtbashCipher.encrypt("abc")
    assert_equal "zyx", result
  end

  def test_mixed_case_preserved
    assert_equal "ZyXwVu", CodingAdventures::AtbashCipher.encrypt("AbCdEf")
  end

  # --- Non-Alpha Passthrough ---

  def test_digits_unchanged
    assert_equal "12345", CodingAdventures::AtbashCipher.encrypt("12345")
  end

  def test_punctuation_unchanged
    assert_equal "!@#$%", CodingAdventures::AtbashCipher.encrypt("!@#$%")
  end

  def test_spaces_unchanged
    assert_equal "   ", CodingAdventures::AtbashCipher.encrypt("   ")
  end

  def test_mixed_alpha_and_digits
    assert_equal "Z1Y2X3", CodingAdventures::AtbashCipher.encrypt("A1B2C3")
  end

  def test_newlines_and_tabs
    assert_equal "Z\nY\tX", CodingAdventures::AtbashCipher.encrypt("A\nB\tC")
  end

  # --- Self-Inverse Property ---

  def test_self_inverse_hello
    assert_equal "HELLO", CodingAdventures::AtbashCipher.encrypt(CodingAdventures::AtbashCipher.encrypt("HELLO"))
  end

  def test_self_inverse_lowercase
    assert_equal "hello", CodingAdventures::AtbashCipher.encrypt(CodingAdventures::AtbashCipher.encrypt("hello"))
  end

  def test_self_inverse_mixed
    input = "Hello, World! 123"
    assert_equal input, CodingAdventures::AtbashCipher.encrypt(CodingAdventures::AtbashCipher.encrypt(input))
  end

  def test_self_inverse_full_alphabet
    alpha = "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
    assert_equal alpha, CodingAdventures::AtbashCipher.encrypt(CodingAdventures::AtbashCipher.encrypt(alpha))
  end

  def test_self_inverse_empty
    assert_equal "", CodingAdventures::AtbashCipher.encrypt(CodingAdventures::AtbashCipher.encrypt(""))
  end

  def test_self_inverse_long_text
    text = "The quick brown fox jumps over the lazy dog! 42"
    assert_equal text, CodingAdventures::AtbashCipher.encrypt(CodingAdventures::AtbashCipher.encrypt(text))
  end

  # --- Edge Cases ---

  def test_empty_string
    assert_equal "", CodingAdventures::AtbashCipher.encrypt("")
  end

  def test_single_a
    assert_equal "Z", CodingAdventures::AtbashCipher.encrypt("A")
  end

  def test_single_z
    assert_equal "A", CodingAdventures::AtbashCipher.encrypt("Z")
  end

  def test_single_m_n_boundary
    assert_equal "N", CodingAdventures::AtbashCipher.encrypt("M")
    assert_equal "M", CodingAdventures::AtbashCipher.encrypt("N")
  end

  def test_single_digit
    assert_equal "5", CodingAdventures::AtbashCipher.encrypt("5")
  end

  def test_no_letter_maps_to_itself
    # 25 - p == p only when p == 12.5, which is not an integer,
    # so no letter position can satisfy this equation.
    ("A".."Z").each do |letter|
      refute_equal letter, CodingAdventures::AtbashCipher.encrypt(letter),
                   "#{letter} maps to itself!"
    end
    ("a".."z").each do |letter|
      refute_equal letter, CodingAdventures::AtbashCipher.encrypt(letter),
                   "#{letter} maps to itself!"
    end
  end

  # --- Decrypt ---

  def test_decrypt_svool
    assert_equal "HELLO", CodingAdventures::AtbashCipher.decrypt("SVOOL")
  end

  def test_decrypt_lowercase
    assert_equal "hello", CodingAdventures::AtbashCipher.decrypt("svool")
  end

  def test_decrypt_is_encrypt_inverse
    texts = ["HELLO", "hello", "Hello, World! 123", "", "42"]
    texts.each do |text|
      assert_equal text, CodingAdventures::AtbashCipher.decrypt(CodingAdventures::AtbashCipher.encrypt(text))
    end
  end

  def test_encrypt_decrypt_equivalence
    texts = ["HELLO", "svool", "Test!", ""]
    texts.each do |text|
      assert_equal CodingAdventures::AtbashCipher.encrypt(text),
                   CodingAdventures::AtbashCipher.decrypt(text)
    end
  end
end
