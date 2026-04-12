"""Tests for DocumentManager — open, change, close operations.

The DocumentManager is the core state manager for open files. These tests
verify that documents are tracked correctly through their lifecycle:
open -> apply changes -> close.
"""

from __future__ import annotations

import pytest

from ls00 import DocumentManager, TextChange, Position, Range


class TestDocumentManagerOpen:
    """Tests for opening and retrieving documents."""

    def test_open_and_get(self) -> None:
        """Opening a document makes it retrievable via get()."""
        dm = DocumentManager()
        dm.open("file:///test.txt", "hello world", 1)

        doc = dm.get("file:///test.txt")
        assert doc is not None
        assert doc.text == "hello world"
        assert doc.version == 1

    def test_get_missing_returns_none(self) -> None:
        """Getting a non-open document returns None."""
        dm = DocumentManager()
        assert dm.get("file:///nonexistent.txt") is None


class TestDocumentManagerClose:
    """Tests for closing documents."""

    def test_close_removes_document(self) -> None:
        """Closing a document makes it no longer retrievable."""
        dm = DocumentManager()
        dm.open("file:///test.txt", "hello", 1)
        dm.close("file:///test.txt")

        assert dm.get("file:///test.txt") is None

    def test_close_nonexistent_is_safe(self) -> None:
        """Closing a document that was never opened does not raise."""
        dm = DocumentManager()
        dm.close("file:///never-opened.txt")  # should not raise


class TestDocumentManagerApplyChanges:
    """Tests for applying incremental and full changes."""

    def test_full_replacement(self) -> None:
        """A change with range=None replaces the entire document."""
        dm = DocumentManager()
        dm.open("file:///test.txt", "hello world", 1)

        dm.apply_changes(
            "file:///test.txt",
            [TextChange(range=None, new_text="goodbye world")],
            2,
        )

        doc = dm.get("file:///test.txt")
        assert doc is not None
        assert doc.text == "goodbye world"
        assert doc.version == 2

    def test_incremental_change(self) -> None:
        """A change with a range replaces only that range."""
        dm = DocumentManager()
        dm.open("file:///test.txt", "hello world", 1)

        # Replace "world" (chars 6-11) with "Go"
        dm.apply_changes(
            "file:///test.txt",
            [TextChange(
                range=Range(
                    start=Position(line=0, character=6),
                    end=Position(line=0, character=11),
                ),
                new_text="Go",
            )],
            2,
        )

        doc = dm.get("file:///test.txt")
        assert doc is not None
        assert doc.text == "hello Go"

    def test_apply_changes_not_open_raises(self) -> None:
        """Applying changes to a non-open document raises KeyError."""
        dm = DocumentManager()
        with pytest.raises(KeyError):
            dm.apply_changes(
                "file:///notopen.txt",
                [TextChange(range=None, new_text="x")],
                1,
            )

    def test_incremental_with_emoji(self) -> None:
        """Incremental change works correctly with emoji (UTF-16 surrogate pairs).

        "A guitar B" -- emoji is 4 UTF-8 bytes, 2 UTF-16 code units.
        Replace "B" (UTF-16 char 3) with "X".
        """
        dm = DocumentManager()
        dm.open("file:///test.txt", "A\U0001F3B8B", 1)

        dm.apply_changes(
            "file:///test.txt",
            [TextChange(
                range=Range(
                    start=Position(line=0, character=3),
                    end=Position(line=0, character=4),
                ),
                new_text="X",
            )],
            2,
        )

        doc = dm.get("file:///test.txt")
        assert doc is not None
        assert doc.text == "A\U0001F3B8X"

    def test_incremental_multi_change(self) -> None:
        """Multiple incremental changes applied in sequence."""
        dm = DocumentManager()
        dm.open("uri", "hello world", 1)

        dm.apply_changes(
            "uri",
            [TextChange(
                range=Range(
                    start=Position(0, 0),
                    end=Position(0, 5),
                ),
                new_text="hi",
            )],
            2,
        )

        doc = dm.get("uri")
        assert doc is not None
        assert doc.text == "hi world"
