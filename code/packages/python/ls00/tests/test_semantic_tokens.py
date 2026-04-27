"""Tests for semantic token encoding.

The semantic token encoding is the compact 5-tuple integer format used by
LSP to transmit token data. These tests verify:
- Empty input produces empty output
- Single token encoding
- Multi-token same-line delta encoding
- Multi-line delta encoding
- Unsorted input is sorted before encoding
- Unknown token types are skipped
- Modifier bitmask encoding
"""

from __future__ import annotations

from ls00 import SemanticToken, encode_semantic_tokens


class TestEncodeSemanticTokensEmpty:
    """Empty or None input should produce empty output."""

    def test_empty_list(self) -> None:
        data = encode_semantic_tokens([])
        assert data == []


class TestEncodeSemanticTokensSingle:
    """A single token should produce one 5-tuple."""

    def test_keyword_at_origin(self) -> None:
        """keyword is at index 15 in the legend."""
        tokens = [
            SemanticToken(
                line=0, character=0, length=5,
                token_type="keyword", modifiers=[],
            ),
        ]
        data = encode_semantic_tokens(tokens)

        assert len(data) == 5
        assert data[0] == 0   # deltaLine
        assert data[1] == 0   # deltaChar
        assert data[2] == 5   # length
        assert data[3] == 15  # tokenTypeIndex (keyword)
        assert data[4] == 0   # modifiers


class TestEncodeSemanticTokensMultipleSameLine:
    """Two tokens on the same line: deltaChar is relative."""

    def test_keyword_then_function(self) -> None:
        tokens = [
            SemanticToken(line=0, character=0, length=3, token_type="keyword"),
            SemanticToken(
                line=0, character=4, length=4,
                token_type="function", modifiers=["declaration"],
            ),
        ]
        data = encode_semantic_tokens(tokens)

        assert len(data) == 10

        # Token A: deltaLine=0, deltaChar=0, length=3, keyword(15), mods=0
        assert data[:5] == [0, 0, 3, 15, 0]
        # Token B: deltaLine=0, deltaChar=4, length=4, function(12), mods=1
        assert data[5:] == [0, 4, 4, 12, 1]


class TestEncodeSemanticTokensMultipleLines:
    """Tokens on different lines: deltaChar is absolute on new line."""

    def test_keyword_line0_number_line2(self) -> None:
        tokens = [
            SemanticToken(line=0, character=0, length=3, token_type="keyword"),
            SemanticToken(line=2, character=4, length=5, token_type="number"),
        ]
        data = encode_semantic_tokens(tokens)

        assert len(data) == 10
        # Token B: deltaLine=2, deltaChar=4 (absolute), number=19
        assert data[5] == 2   # deltaLine
        assert data[6] == 4   # deltaChar
        assert data[8] == 19  # number


class TestEncodeSemanticTokensUnsortedInput:
    """Tokens in reverse order should be sorted before encoding."""

    def test_reverse_order_sorted(self) -> None:
        tokens = [
            SemanticToken(line=1, character=0, length=2, token_type="number"),
            SemanticToken(line=0, character=0, length=3, token_type="keyword"),
        ]
        data = encode_semantic_tokens(tokens)

        assert len(data) == 10
        # After sorting: keyword (line 0) first, number (line 1) second
        assert data[3] == 15  # keyword
        assert data[8] == 19  # number


class TestEncodeSemanticTokensUnknownType:
    """Unknown token types should be skipped."""

    def test_skip_unknown_keep_known(self) -> None:
        tokens = [
            SemanticToken(line=0, character=0, length=3, token_type="unknownType"),
            SemanticToken(line=0, character=4, length=2, token_type="keyword"),
        ]
        data = encode_semantic_tokens(tokens)

        # unknownType skipped, only keyword remains
        assert len(data) == 5


class TestEncodeSemanticTokensModifierBitmask:
    """Modifier bitmask encoding."""

    def test_readonly_bitmask(self) -> None:
        """'readonly' is bit 2 (index 2 in modifier list), value = 4."""
        tokens = [
            SemanticToken(
                line=0, character=0, length=3,
                token_type="variable", modifiers=["readonly"],
            ),
        ]
        data = encode_semantic_tokens(tokens)
        assert data[4] == 4  # readonly = bit 2 = value 4
