# frozen_string_literal: true

require_relative "test_helper"

# ============================================================================
# Tests for CodingAdventures::CaesarCipher brute_force / frequency_analysis
# ============================================================================
#
# We verify:
#   - Brute force returns exactly 25 results (shifts 1..25)
#   - Brute force includes the correct decryption
#   - Frequency analysis correctly identifies the shift for known English text
#   - Edge cases: empty strings, non-alphabetic input, short texts
#   - The ENGLISH_FREQUENCIES constant is well-formed
# ============================================================================

class TestAnalysis < Minitest::Test
  CC = CodingAdventures::CaesarCipher

  # --------------------------------------------------------------------------
  # ENGLISH_FREQUENCIES constant
  # --------------------------------------------------------------------------

  def test_english_frequencies_has_26_entries
    assert_equal 26, CC::ENGLISH_FREQUENCIES.size
  end

  def test_english_frequencies_covers_all_letters
    ("a".."z").each do |letter|
      assert CC::ENGLISH_FREQUENCIES.key?(letter),
        "Missing frequency for '#{letter}'"
    end
  end

  def test_english_frequencies_sum_approximately_one
    total = CC::ENGLISH_FREQUENCIES.values.sum
    assert_in_delta 1.0, total, 0.001,
      "Frequencies should sum to ~1.0, got #{total}"
  end

  def test_english_frequencies_all_positive
    CC::ENGLISH_FREQUENCIES.each do |letter, freq|
      assert_operator freq, :>, 0, "Frequency for '#{letter}' should be positive"
    end
  end

  def test_english_frequencies_frozen
    assert CC::ENGLISH_FREQUENCIES.frozen?,
      "ENGLISH_FREQUENCIES should be frozen"
  end

  # --------------------------------------------------------------------------
  # Brute force
  # --------------------------------------------------------------------------

  def test_brute_force_returns_25_results
    results = CC.brute_force("KHOOR")
    assert_equal 25, results.size
  end

  def test_brute_force_returns_shift_plaintext_pairs
    results = CC.brute_force("KHOOR")
    results.each do |pair|
      assert_kind_of Array, pair
      assert_equal 2, pair.size
      assert_kind_of Integer, pair[0]
      assert_kind_of String, pair[1]
    end
  end

  def test_brute_force_shifts_range_1_to_25
    results = CC.brute_force("KHOOR")
    shifts = results.map(&:first)
    assert_equal (1..25).to_a, shifts
  end

  def test_brute_force_contains_correct_decryption
    # "KHOOR" was encrypted with shift 3, so decrypting with shift 3
    # should give "HELLO".
    results = CC.brute_force("KHOOR")
    match = results.find { |shift, _text| shift == 3 }
    refute_nil match, "Expected to find shift 3 in brute force results"
    assert_equal "HELLO", match[1]
  end

  def test_brute_force_empty_string
    results = CC.brute_force("")
    assert_equal 25, results.size
    results.each do |_shift, text|
      assert_equal "", text
    end
  end

  def test_brute_force_non_alpha
    results = CC.brute_force("123!@#")
    results.each do |_shift, text|
      assert_equal "123!@#", text
    end
  end

  def test_brute_force_lowercase
    results = CC.brute_force("khoor")
    match = results.find { |shift, _text| shift == 3 }
    refute_nil match
    assert_equal "hello", match[1]
  end

  def test_brute_force_preserves_case
    results = CC.brute_force("Khoor")
    match = results.find { |shift, _text| shift == 3 }
    assert_equal "Hello", match[1]
  end

  # --------------------------------------------------------------------------
  # Frequency analysis
  # --------------------------------------------------------------------------

  def test_frequency_analysis_known_english_shift_3
    # A reasonably long English sentence encrypted with shift 3.
    plaintext = "The quick brown fox jumps over the lazy dog and then rests by the warm fire"
    ciphertext = CC.encrypt(plaintext, 3)
    shift, decrypted = CC.frequency_analysis(ciphertext)
    assert_equal 3, shift
    assert_equal plaintext, decrypted
  end

  def test_frequency_analysis_shift_7
    plaintext = "Frequency analysis is a powerful technique for breaking simple substitution ciphers"
    ciphertext = CC.encrypt(plaintext, 7)
    shift, decrypted = CC.frequency_analysis(ciphertext)
    assert_equal 7, shift
    assert_equal plaintext, decrypted
  end

  def test_frequency_analysis_shift_13
    plaintext = "This is a test of the emergency broadcast system for frequency analysis purposes"
    ciphertext = CC.encrypt(plaintext, 13)
    shift, decrypted = CC.frequency_analysis(ciphertext)
    assert_equal 13, shift
    assert_equal plaintext, decrypted
  end

  def test_frequency_analysis_shift_0
    plaintext = "Already in the clear with no encryption applied to this text whatsoever"
    shift, decrypted = CC.frequency_analysis(plaintext)
    assert_equal 0, shift
    assert_equal plaintext, decrypted
  end

  def test_frequency_analysis_uppercase
    plaintext = "THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG AND RUNS AROUND THE PARK AGAIN"
    ciphertext = CC.encrypt(plaintext, 10)
    shift, decrypted = CC.frequency_analysis(ciphertext)
    assert_equal 10, shift
    assert_equal plaintext, decrypted
  end

  def test_frequency_analysis_empty_string
    shift, text = CC.frequency_analysis("")
    assert_equal 0, shift
    assert_equal "", text
  end

  def test_frequency_analysis_no_letters
    shift, text = CC.frequency_analysis("12345!@#$%")
    assert_equal 0, shift
    assert_equal "12345!@#$%", text
  end

  def test_frequency_analysis_preserves_non_alpha
    plaintext = "Hello, World! The quick brown fox jumps over the lazy dog. Numbers: 12345."
    ciphertext = CC.encrypt(plaintext, 5)
    shift, decrypted = CC.frequency_analysis(ciphertext)
    assert_equal 5, shift
    assert_equal plaintext, decrypted
  end

  def test_frequency_analysis_long_text
    # Longer texts should always be correctly identified.
    plaintext = "In cryptography a Caesar cipher also known as Caesars cipher the shift " \
                "cipher Caesars code or Caesar shift is one of the simplest and most widely " \
                "known encryption techniques It is a type of substitution cipher in which " \
                "each letter in the plaintext is replaced by a letter some fixed number of " \
                "positions down the alphabet For example with a left shift of three D would " \
                "be replaced by A E would become B and so on"
    ciphertext = CC.encrypt(plaintext, 17)
    shift, decrypted = CC.frequency_analysis(ciphertext)
    assert_equal 17, shift
    assert_equal plaintext, decrypted
  end
end
