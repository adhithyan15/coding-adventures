# frozen_string_literal: true

# Comprehensive tests for the Scytale cipher implementation.
#
# Covers encryption, decryption, round-trip, padding, key validation,
# brute force, and edge cases.

require "minitest/autorun"
require "coding_adventures_scytale_cipher"

class TestScytaleCipherEncrypt < Minitest::Test
  def test_hello_world_key3
    assert_equal "HLWLEOODL R ", CodingAdventures::ScytaleCipher.encrypt("HELLO WORLD", 3)
  end

  def test_abcdef_key2
    assert_equal "ACEBDF", CodingAdventures::ScytaleCipher.encrypt("ABCDEF", 2)
  end

  def test_abcdef_key3
    assert_equal "ADBECF", CodingAdventures::ScytaleCipher.encrypt("ABCDEF", 3)
  end

  def test_key_equals_length
    assert_equal "ABCD", CodingAdventures::ScytaleCipher.encrypt("ABCD", 4)
  end

  def test_empty_string
    assert_equal "", CodingAdventures::ScytaleCipher.encrypt("", 2)
  end
end

class TestScytaleCipherDecrypt < Minitest::Test
  def test_hello_world_key3
    assert_equal "HELLO WORLD", CodingAdventures::ScytaleCipher.decrypt("HLWLEOODL R ", 3)
  end

  def test_acebdf_key2
    assert_equal "ABCDEF", CodingAdventures::ScytaleCipher.decrypt("ACEBDF", 2)
  end

  def test_empty_string
    assert_equal "", CodingAdventures::ScytaleCipher.decrypt("", 2)
  end
end

class TestScytaleCipherRoundTrip < Minitest::Test
  def test_round_trip_hello_world
    text = "HELLO WORLD"
    assert_equal text, CodingAdventures::ScytaleCipher.decrypt(
      CodingAdventures::ScytaleCipher.encrypt(text, 3), 3
    )
  end

  def test_round_trip_various_keys
    text = "The quick brown fox jumps over the lazy dog!"
    (2..(text.length / 2)).each do |key|
      ct = CodingAdventures::ScytaleCipher.encrypt(text, key)
      pt = CodingAdventures::ScytaleCipher.decrypt(ct, key)
      assert_equal text, pt, "Round trip failed for key=#{key}"
    end
  end

  def test_round_trip_with_punctuation
    text = "Hello, World! 123"
    ct = CodingAdventures::ScytaleCipher.encrypt(text, 4)
    assert_equal text, CodingAdventures::ScytaleCipher.decrypt(ct, 4)
  end
end

class TestScytaleCipherKeyValidation < Minitest::Test
  def test_key_zero
    assert_raises(ArgumentError) { CodingAdventures::ScytaleCipher.encrypt("HELLO", 0) }
  end

  def test_key_one
    assert_raises(ArgumentError) { CodingAdventures::ScytaleCipher.encrypt("HELLO", 1) }
  end

  def test_key_negative
    assert_raises(ArgumentError) { CodingAdventures::ScytaleCipher.encrypt("HELLO", -1) }
  end

  def test_key_too_large
    assert_raises(ArgumentError) { CodingAdventures::ScytaleCipher.encrypt("HI", 3) }
  end

  def test_decrypt_key_zero
    assert_raises(ArgumentError) { CodingAdventures::ScytaleCipher.decrypt("HELLO", 0) }
  end

  def test_decrypt_key_too_large
    assert_raises(ArgumentError) { CodingAdventures::ScytaleCipher.decrypt("HI", 3) }
  end
end

class TestScytaleCipherBruteForce < Minitest::Test
  def test_finds_original
    original = "HELLO WORLD"
    ct = CodingAdventures::ScytaleCipher.encrypt(original, 3)
    results = CodingAdventures::ScytaleCipher.brute_force(ct)
    found = results.find { |r| r[:key] == 3 }
    refute_nil found
    assert_equal original, found[:text]
  end

  def test_returns_all_keys
    results = CodingAdventures::ScytaleCipher.brute_force("ABCDEFGHIJ")
    assert_equal [2, 3, 4, 5], results.map { |r| r[:key] }
  end

  def test_short_text
    assert_equal [], CodingAdventures::ScytaleCipher.brute_force("AB")
  end
end

class TestScytaleCipherPadding < Minitest::Test
  def test_padding_stripped
    ct = CodingAdventures::ScytaleCipher.encrypt("HELLO", 3)
    assert_equal "HELLO", CodingAdventures::ScytaleCipher.decrypt(ct, 3)
  end

  def test_no_padding_needed
    ct = CodingAdventures::ScytaleCipher.encrypt("ABCDEF", 2)
    assert_equal 6, ct.length
  end
end

class TestScytaleCipherVersion < Minitest::Test
  def test_version
    assert_equal "0.1.0", CodingAdventures::ScytaleCipher::VERSION
  end
end
