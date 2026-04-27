# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# UTF-16 Offset Conversion Tests
# ================================================================
#
# This is the most important correctness test in the entire package.
# If this function is wrong, every feature that depends on cursor
# position will be wrong: hover, go-to-definition, references,
# completion, rename, signature help.
#
# LSP uses UTF-16 code units for character offsets. Ruby strings are
# UTF-8. This test verifies the conversion between the two.
#
# ================================================================

class TestUTF16Conversion < Minitest::Test
  # ASCII simple: "hello world" -- "world" starts at byte 6.
  def test_ascii_simple
    text = "hello world"
    assert_equal 6, CodingAdventures::Ls00.convert_utf16_offset_to_byte_offset(text, 0, 6)
  end

  # Start of file: byte 0.
  def test_start_of_file
    text = "abc"
    assert_equal 0, CodingAdventures::Ls00.convert_utf16_offset_to_byte_offset(text, 0, 0)
  end

  # End of short string.
  def test_end_of_short_string
    text = "abc"
    assert_equal 3, CodingAdventures::Ls00.convert_utf16_offset_to_byte_offset(text, 0, 3)
  end

  # Second line: "hello\nworld" -- line 1 starts at byte 6.
  def test_second_line
    text = "hello\nworld"
    assert_equal 6, CodingAdventures::Ls00.convert_utf16_offset_to_byte_offset(text, 1, 0)
  end

  # Emoji: guitar emoji (U+1F3B8) takes 2 UTF-16 units but 4 UTF-8 bytes.
  # "A\u{1F3B8}B"
  # UTF-8 bytes:  A (1) + emoji (4) + B (1) = 6 bytes
  # UTF-16 units: A (1) + emoji (2) + B (1) = 4 units
  # "B" is at UTF-16 character 3, byte offset 5.
  def test_emoji_guitar
    text = "A\u{1F3B8}B"
    assert_equal 5, CodingAdventures::Ls00.convert_utf16_offset_to_byte_offset(text, 0, 3)
  end

  # Emoji at start: "\u{1F3B8}hello"
  # emoji = 2 UTF-16 units = 4 UTF-8 bytes
  # "h" is at UTF-16 char 2, byte offset 4
  def test_emoji_at_start
    text = "\u{1F3B8}hello"
    assert_equal 4, CodingAdventures::Ls00.convert_utf16_offset_to_byte_offset(text, 0, 2)
  end

  # 2-byte UTF-8 (BMP codepoint: e-acute U+00E9).
  # "cafe-acute!" -- e-acute is 2 UTF-8 bytes but 1 UTF-16 code unit.
  # UTF-16 char 4 = byte offset 5 (c=1, a=1, f=1, e-acute=2 bytes)
  def test_two_byte_utf8_bmp
    text = "caf\u00e9!"
    assert_equal 5, CodingAdventures::Ls00.convert_utf16_offset_to_byte_offset(text, 0, 4)
  end

  # Multiline with emoji.
  # line 0: "A\u{1F3B8}B\n" (A=1, emoji=4, B=1, \n=1 = 7 bytes)
  # line 1: "hello" starts at byte 7
  def test_multiline_with_emoji
    text = "A\u{1F3B8}B\nhello"
    assert_equal 7, CodingAdventures::Ls00.convert_utf16_offset_to_byte_offset(text, 1, 0)
  end

  # Beyond line end clamps to newline.
  # If character is past the end of the line, we stop at the newline.
  def test_beyond_line_end_clamps
    text = "ab\ncd"
    assert_equal 2, CodingAdventures::Ls00.convert_utf16_offset_to_byte_offset(text, 0, 100)
  end

  # Chinese character: 3-byte UTF-8 / 1-unit UTF-16.
  # "zhong-wen" -- each Chinese character is 3 UTF-8 bytes but 1 UTF-16 code unit.
  # Second character is at UTF-16 character 1, byte offset 3.
  def test_chinese_character
    text = "\u4e2d\u6587" # two CJK characters
    assert_equal 3, CodingAdventures::Ls00.convert_utf16_offset_to_byte_offset(text, 0, 1)
  end
end
