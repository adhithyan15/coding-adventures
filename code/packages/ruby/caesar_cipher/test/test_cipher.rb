# frozen_string_literal: true

require_relative "test_helper"

# ============================================================================
# Tests for CodingAdventures::CaesarCipher encrypt / decrypt / rot13
# ============================================================================
#
# We verify:
#   - Basic encryption with known shift values
#   - Case preservation (uppercase stays uppercase, lowercase stays lowercase)
#   - Non-alphabetic characters pass through unchanged
#   - Round-trip: decrypt(encrypt(text, s), s) == text for all shifts
#   - Edge cases: empty strings, shift = 0, negative shifts, large shifts
#   - ROT13 self-inverse property
# ============================================================================

class TestCipher < Minitest::Test
  CC = CodingAdventures::CaesarCipher

  # --------------------------------------------------------------------------
  # Encryption basics
  # --------------------------------------------------------------------------

  def test_encrypt_hello_shift_3
    # The classic example: HELLO with shift 3 becomes KHOOR.
    #   H(7)  + 3 = K(10)
    #   E(4)  + 3 = H(7)
    #   L(11) + 3 = O(14)
    #   L(11) + 3 = O(14)
    #   O(14) + 3 = R(17)
    assert_equal "KHOOR", CC.encrypt("HELLO", 3)
  end

  def test_encrypt_lowercase
    assert_equal "khoor", CC.encrypt("hello", 3)
  end

  def test_encrypt_mixed_case
    assert_equal "Khoor, Zruog!", CC.encrypt("Hello, World!", 3)
  end

  def test_encrypt_shift_1
    assert_equal "BCD", CC.encrypt("ABC", 1)
  end

  def test_encrypt_shift_25
    # Shift 25 is the same as shift -1: each letter moves one back.
    assert_equal "ZAB", CC.encrypt("ABC", 25)
  end

  def test_encrypt_wrapping_xyz_shift_3
    # X(23)+3=A(0), Y(24)+3=B(1), Z(25)+3=C(2)
    assert_equal "abc", CC.encrypt("xyz", 3)
  end

  def test_encrypt_full_alphabet
    # Shift the entire alphabet by 13 (ROT13).
    plain  = "abcdefghijklmnopqrstuvwxyz"
    cipher = "nopqrstuvwxyzabcdefghijklm"
    assert_equal cipher, CC.encrypt(plain, 13)
  end

  # --------------------------------------------------------------------------
  # Non-alpha passthrough
  # --------------------------------------------------------------------------

  def test_digits_pass_through
    assert_equal "123", CC.encrypt("123", 5)
  end

  def test_punctuation_pass_through
    assert_equal "!@#$%", CC.encrypt("!@#$%", 10)
  end

  def test_spaces_pass_through
    assert_equal "B C", CC.encrypt("A B", 1)
  end

  def test_mixed_content_preserves_non_alpha
    assert_equal "Ifmmp, Xpsme! 123", CC.encrypt("Hello, World! 123", 1)
  end

  # --------------------------------------------------------------------------
  # Case preservation
  # --------------------------------------------------------------------------

  def test_case_preservation
    result = CC.encrypt("AaBbZz", 1)
    assert_equal "BbCcAa", result
  end

  # --------------------------------------------------------------------------
  # Edge cases
  # --------------------------------------------------------------------------

  def test_empty_string
    assert_equal "", CC.encrypt("", 5)
  end

  def test_shift_zero
    assert_equal "HELLO", CC.encrypt("HELLO", 0)
  end

  def test_shift_26_is_identity
    # Shifting by 26 (full alphabet) should return the original text.
    assert_equal "HELLO", CC.encrypt("HELLO", 26)
  end

  def test_shift_52_is_identity
    assert_equal "HELLO", CC.encrypt("HELLO", 52)
  end

  def test_negative_shift
    # shift -1 is equivalent to shift 25
    assert_equal "ZAB", CC.encrypt("ABC", -1)
  end

  def test_negative_shift_large
    # -27 % 26 == 25 in Ruby
    assert_equal "ZAB", CC.encrypt("ABC", -27)
  end

  # --------------------------------------------------------------------------
  # Decryption
  # --------------------------------------------------------------------------

  def test_decrypt_khoor_shift_3
    assert_equal "HELLO", CC.decrypt("KHOOR", 3)
  end

  def test_decrypt_lowercase
    assert_equal "hello", CC.decrypt("khoor", 3)
  end

  def test_decrypt_mixed_case_with_punctuation
    assert_equal "Hello, World!", CC.decrypt("Khoor, Zruog!", 3)
  end

  def test_decrypt_empty_string
    assert_equal "", CC.decrypt("", 7)
  end

  def test_decrypt_shift_zero
    assert_equal "HELLO", CC.decrypt("HELLO", 0)
  end

  # --------------------------------------------------------------------------
  # Round-trip: decrypt(encrypt(text, s), s) == text
  # --------------------------------------------------------------------------

  def test_round_trip_all_shifts
    text = "The Quick Brown Fox Jumps Over The Lazy Dog!"
    (0..25).each do |shift|
      encrypted = CC.encrypt(text, shift)
      decrypted = CC.decrypt(encrypted, shift)
      assert_equal text, decrypted, "Round-trip failed for shift #{shift}"
    end
  end

  def test_round_trip_negative_shift
    text = "Testing negative shifts"
    (-25..-1).each do |shift|
      encrypted = CC.encrypt(text, shift)
      decrypted = CC.decrypt(encrypted, shift)
      assert_equal text, decrypted, "Round-trip failed for shift #{shift}"
    end
  end

  # --------------------------------------------------------------------------
  # ROT13
  # --------------------------------------------------------------------------

  def test_rot13_hello
    assert_equal "URYYB", CC.rot13("HELLO")
  end

  def test_rot13_lowercase
    assert_equal "uryyb", CC.rot13("hello")
  end

  def test_rot13_self_inverse
    # Applying ROT13 twice should return the original text.
    text = "Hello, World! 123"
    assert_equal text, CC.rot13(CC.rot13(text))
  end

  def test_rot13_self_inverse_all_letters
    text = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz"
    assert_equal text, CC.rot13(CC.rot13(text))
  end

  def test_rot13_preserves_non_alpha
    assert_equal "Uryyb, Jbeyq! 456", CC.rot13("Hello, World! 456")
  end

  def test_rot13_empty_string
    assert_equal "", CC.rot13("")
  end

  def test_rot13_is_encrypt_with_13
    text = "Some arbitrary text!"
    assert_equal CC.encrypt(text, 13), CC.rot13(text)
  end
end
