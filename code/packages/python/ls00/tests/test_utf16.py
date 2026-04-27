"""Tests for UTF-16 offset conversion.

This is the most important correctness test in the entire package. If
``convert_utf16_offset_to_byte_offset`` is wrong, every feature that depends
on cursor position will be wrong: hover, go-to-definition, references,
completion, rename, signature help.

UTF-16 conversion is tricky because:
- ASCII characters: 1 UTF-8 byte = 1 UTF-16 code unit
- BMP codepoints (U+0080-U+FFFF): 2-3 UTF-8 bytes = 1 UTF-16 code unit
- Non-BMP codepoints (> U+FFFF): 4 UTF-8 bytes = 2 UTF-16 code units
"""

from __future__ import annotations

import pytest

from ls00 import convert_utf16_offset_to_byte_offset


class TestConvertUTF16OffsetToByteOffset:
    """Verify the critical UTF-16 -> byte offset conversion."""

    def test_ascii_simple(self) -> None:
        """ASCII characters: 1 byte = 1 UTF-16 code unit."""
        text = "hello world"
        assert convert_utf16_offset_to_byte_offset(text, 0, 6) == 6

    def test_start_of_file(self) -> None:
        """Offset 0 always maps to byte 0."""
        text = "abc"
        assert convert_utf16_offset_to_byte_offset(text, 0, 0) == 0

    def test_end_of_short_string(self) -> None:
        """Offset at end of string maps to last byte + 1."""
        text = "abc"
        assert convert_utf16_offset_to_byte_offset(text, 0, 3) == 3

    def test_second_line(self) -> None:
        """Line 1, char 0 maps to the byte after the newline."""
        # "hello\nworld" -- line 1 starts at byte 6
        text = "hello\nworld"
        assert convert_utf16_offset_to_byte_offset(text, 1, 0) == 6

    def test_emoji_surrogate_pair(self) -> None:
        """Emoji (U+1F3B8): 4 UTF-8 bytes but 2 UTF-16 code units.

        "A\U0001F3B8B"
        UTF-8 bytes:  A (1) + guitar (4) + B (1) = 6 bytes
        UTF-16 units: A (1) + guitar (2) + B (1) = 4 units
        "B" is at UTF-16 character 3, byte offset 5.
        """
        text = "A\U0001F3B8B"
        assert convert_utf16_offset_to_byte_offset(text, 0, 3) == 5

    def test_emoji_at_start(self) -> None:
        """Emoji at position 0: "h" is at UTF-16 char 2, byte offset 4."""
        text = "\U0001F3B8hello"
        assert convert_utf16_offset_to_byte_offset(text, 0, 2) == 4

    def test_2byte_utf8_bmp_codepoint(self) -> None:
        """BMP codepoint (e-accent, U+00E9): 2 UTF-8 bytes, 1 UTF-16 code unit.

        "cafe-accent!" -- e-accent is 2 bytes in UTF-8 but 1 unit in UTF-16.
        So UTF-16 char 4 = byte offset 5 (c=1, a=1, f=1, e-accent=2).
        """
        text = "caf\u00e9!"
        assert convert_utf16_offset_to_byte_offset(text, 0, 4) == 5

    def test_multiline_with_emoji(self) -> None:
        """Multi-line: line 0 has emoji, line 1 starts after it.

        line 0: "A guitar B\\n"  (A=1, guitar=4, B=1, \\n=1 = 7 bytes)
        line 1: "hello" starts at byte 7
        """
        text = "A\U0001F3B8B\nhello"
        assert convert_utf16_offset_to_byte_offset(text, 1, 0) == 7

    def test_beyond_line_end_clamps_to_newline(self) -> None:
        """Character past end of line stops at the newline boundary."""
        text = "ab\ncd"
        assert convert_utf16_offset_to_byte_offset(text, 0, 100) == 2

    def test_chinese_character(self) -> None:
        """3-byte UTF-8 / 1-unit UTF-16 codepoints (CJK).

        "zhong wen" -- each Chinese char is 3 UTF-8 bytes, 1 UTF-16 code unit.
        "wen" is at UTF-16 character 1, byte offset 3.
        """
        text = "\u4e2d\u6587"  # zhong wen
        assert convert_utf16_offset_to_byte_offset(text, 0, 1) == 3
