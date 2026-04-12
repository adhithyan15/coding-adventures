defmodule Ls00.UTF16Test do
  use ExUnit.Case, async: true

  @moduledoc """
  Tests for UTF-16 offset conversion.

  This is the most important correctness test in the entire package. If this
  function is wrong, every feature that depends on cursor position will be wrong:
  hover, go-to-definition, references, completion, rename, signature help.
  """

  alias Ls00.DocumentManager

  # ---------------------------------------------------------------------------
  # UTF-16 Offset Conversion Tests
  # ---------------------------------------------------------------------------

  test "ASCII simple: 'hello world' char 6 -> byte 6" do
    # "world" starts at byte 6 in "hello world"
    assert DocumentManager.convert_utf16_offset_to_byte_offset("hello world", 0, 6) == 6
  end

  test "start of file: char 0 -> byte 0" do
    assert DocumentManager.convert_utf16_offset_to_byte_offset("abc", 0, 0) == 0
  end

  test "end of short string: char 3 -> byte 3" do
    assert DocumentManager.convert_utf16_offset_to_byte_offset("abc", 0, 3) == 3
  end

  test "second line: line 1 char 0" do
    # "hello\nworld" -- line 1 starts at byte 6
    assert DocumentManager.convert_utf16_offset_to_byte_offset("hello\nworld", 1, 0) == 6
  end

  test "emoji: guitar emoji takes 2 UTF-16 units but 4 UTF-8 bytes" do
    # "A🎸B"
    # UTF-8 bytes:  A (1 byte) + 🎸 (4 bytes) + B (1 byte) = 6 bytes total
    # UTF-16 units: A (1 unit) + 🎸 (2 units) + B (1 unit) = 4 units total
    # "B" is at UTF-16 character 3, byte offset 5.
    text = "A\u{1F3B8}B"
    assert DocumentManager.convert_utf16_offset_to_byte_offset(text, 0, 3) == 5
  end

  test "emoji at start: guitar emoji then hello" do
    # "🎸hello"
    # 🎸 = 2 UTF-16 units = 4 UTF-8 bytes
    # "h" is at UTF-16 char 2, byte offset 4
    text = "\u{1F3B8}hello"
    assert DocumentManager.convert_utf16_offset_to_byte_offset(text, 0, 2) == 4
  end

  test "2-byte UTF-8 BMP codepoint: cafe with accent" do
    # "cafe!" -- e-acute is U+00E9, which is:
    # UTF-8:  2 bytes (0xC3 0xA9)
    # UTF-16: 1 code unit
    # So UTF-16 char 4 = byte offset 5 (c=1, a=1, f=1, e-acute=2 bytes)
    text = "caf\u00e9!"
    assert DocumentManager.convert_utf16_offset_to_byte_offset(text, 0, 4) == 5
  end

  test "multiline with emoji" do
    # line 0: "A🎸B\n"  (A=1, 🎸=4, B=1, \n=1 = 7 bytes)
    # line 1: "hello"
    # "hello" starts at byte 7, char 0 on line 1
    text = "A\u{1F3B8}B\nhello"
    assert DocumentManager.convert_utf16_offset_to_byte_offset(text, 1, 0) == 7
  end

  test "beyond line end clamps to newline" do
    # If character is past the end of the line, we stop at the newline.
    text = "ab\ncd"
    assert DocumentManager.convert_utf16_offset_to_byte_offset(text, 0, 100) == 2
  end

  test "CJK characters: 3-byte UTF-8, 1 UTF-16 unit" do
    # "中文" -- each Chinese character is 3 UTF-8 bytes but 1 UTF-16 code unit.
    # So "文" is at UTF-16 character 1, byte offset 3.
    text = "\u4e2d\u6587"
    assert DocumentManager.convert_utf16_offset_to_byte_offset(text, 0, 1) == 3
  end
end
