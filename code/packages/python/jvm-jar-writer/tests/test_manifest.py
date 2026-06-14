"""Unit tests for the manifest encoder.

The 72-byte line-folding rule is the trickiest invariant — every
line in the encoded manifest (including continuation lines) must
be ≤ 72 bytes including the trailing CRLF.
"""

from __future__ import annotations

import pytest

from jvm_jar_writer import JarManifest, JarWriterError, encode_manifest


def _lines(blob: bytes) -> list[bytes]:
    """Split ``blob`` on CRLF (preserving ordering)."""
    return blob.split(b"\r\n")


class TestVersionAndMainClass:
    def test_version_always_present(self) -> None:
        out = encode_manifest(JarManifest())
        assert out.startswith(b"Manifest-Version: 1.0\r\n")

    def test_main_class_when_set(self) -> None:
        out = encode_manifest(JarManifest(main_class="com.example.Main"))
        assert b"Main-Class: com.example.Main\r\n" in out

    def test_main_class_omitted_when_unset(self) -> None:
        out = encode_manifest(JarManifest())
        assert b"Main-Class:" not in out

    def test_extra_attributes_in_order(self) -> None:
        out = encode_manifest(
            JarManifest(
                extra_attributes={
                    "Built-By": "twig",
                    "Created-By": "jvm-jar-writer 0.1",
                }
            )
        )
        # Both attributes present, in insertion order.
        built_idx = out.index(b"Built-By: ")
        created_idx = out.index(b"Created-By: ")
        assert built_idx < created_idx


class TestSectionTermination:
    def test_ends_with_blank_crlf(self) -> None:
        out = encode_manifest(JarManifest())
        # Last 4 bytes should be ``CRLF CRLF`` (terminator after
        # the last attribute's CRLF).
        assert out.endswith(b"\r\n\r\n")


class TestLineLength:
    def test_short_value_no_folding(self) -> None:
        out = encode_manifest(
            JarManifest(extra_attributes={"X-Short": "abc"})
        )
        assert b"X-Short: abc\r\n" in out

    def test_value_just_over_first_line_room_folds(self) -> None:
        # ``X-Long: `` is 8 bytes; first-line room = 72 - 8 - 2 = 62 bytes.
        # A 70-byte value forces a fold after the first 62 bytes.
        long_value = "v" * 70
        out = encode_manifest(
            JarManifest(extra_attributes={"X-Long": long_value})
        )
        # Find the X-Long line + the continuation that follows.
        # Each ASCII line ≤ 72 bytes including CRLF.
        for line in _lines(out):
            assert len(line) + 2 <= 72, (
                f"line too long ({len(line) + 2} bytes including CRLF): {line!r}"
            )

    def test_continuation_starts_with_space(self) -> None:
        long_value = "v" * 200
        out = encode_manifest(
            JarManifest(extra_attributes={"X-VeryLong": long_value})
        )
        lines = _lines(out)
        # Find the X-VeryLong line; the line after it must start
        # with a single SP (continuation marker).
        cont_seen = False
        for i, line in enumerate(lines):
            if line.startswith(b"X-VeryLong: "):
                # The next non-empty line should be a continuation.
                next_line = lines[i + 1]
                assert next_line.startswith(b" "), (
                    "continuation lines must start with one SP"
                )
                cont_seen = True
                break
        assert cont_seen, "expected to find X-VeryLong attribute"

    def test_very_long_value_correctly_reassembles(self) -> None:
        """Folding must be reversible: stripping the leading SP
        from continuation lines and concatenating reproduces the
        original value."""
        long_value = "x" * 500 + "y" * 200
        out = encode_manifest(
            JarManifest(extra_attributes={"X-Long": long_value})
        )
        lines = _lines(out)
        # Find the start of X-Long.
        for i, line in enumerate(lines):
            if line.startswith(b"X-Long: "):
                value_chunks = [line[len(b"X-Long: "):]]
                j = i + 1
                while j < len(lines) and lines[j].startswith(b" "):
                    value_chunks.append(lines[j][1:])  # strip the SP
                    j += 1
                reassembled = b"".join(value_chunks).decode("utf-8")
                assert reassembled == long_value
                return
        pytest.fail("expected to find X-Long attribute")


class TestRejectedInputs:
    def test_invalid_attr_name_rejected(self) -> None:
        with pytest.raises(JarWriterError, match="attribute name"):
            encode_manifest(JarManifest(extra_attributes={"1Bad": "v"}))

    def test_attr_name_with_colon_rejected(self) -> None:
        with pytest.raises(JarWriterError, match="attribute name"):
            encode_manifest(
                JarManifest(extra_attributes={"Key:Has:Colon": "v"})
            )

    def test_attr_value_with_newline_rejected(self) -> None:
        with pytest.raises(JarWriterError, match="line break"):
            encode_manifest(JarManifest(extra_attributes={"X": "a\nb"}))

    def test_attr_value_with_cr_rejected(self) -> None:
        with pytest.raises(JarWriterError, match="line break"):
            encode_manifest(JarManifest(extra_attributes={"X": "a\rb"}))

    def test_extremely_long_attr_name_rejected(self) -> None:
        # 70+ byte name leaves no room for value on the first line.
        long_name = "X" + "y" * 69 + "z"  # 71 bytes
        with pytest.raises(JarWriterError, match="too long|attribute name"):
            encode_manifest(JarManifest(extra_attributes={long_name: "v"}))
