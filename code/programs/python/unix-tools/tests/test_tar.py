"""Tests for the tar tool.

=== What These Tests Verify ===

These tests exercise the tar implementation, including:

1. Creating archives
2. Extracting archives
3. Listing archive contents
4. Compression (gzip, bzip2, xz)
5. Verbose mode
6. Directory change (-C)
7. Exclude patterns
8. Keep old files (-k)
9. Strip components
10. Spec loading and CLI Builder integration
"""

from __future__ import annotations

import sys
import tarfile
from pathlib import Path
from typing import Any

import pytest

SPEC_FILE = str(Path(__file__).parent.parent / "tar.json")
sys.path.insert(0, str(Path(__file__).parent.parent))

from tar_tool import create_archive, extract_archive, list_archive


def parse_argv(argv: list[str]) -> Any:  # noqa: ANN401
    from cli_builder import Parser

    return Parser(SPEC_FILE, argv).parse()


# ---------------------------------------------------------------------------
# Spec loading
# ---------------------------------------------------------------------------


class TestSpecLoading:
    def test_spec_file_exists(self) -> None:
        assert Path(SPEC_FILE).exists()

    def test_help(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["tar", "--help"])
        assert isinstance(result, HelpResult)

    def test_version(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["tar", "--version"])
        assert isinstance(result, VersionResult)


# ---------------------------------------------------------------------------
# Creating archives
# ---------------------------------------------------------------------------


class TestCreateArchive:
    def test_create_simple(self, tmp_path: Path) -> None:
        """Create a tar archive with a single file."""
        f = tmp_path / "hello.txt"
        f.write_text("hello world\n")
        archive = tmp_path / "test.tar"

        result = create_archive(
            str(archive), ["hello.txt"],
            directory=str(tmp_path),
        )
        assert result is True
        assert archive.exists()

        # Verify the archive contains the file.
        with tarfile.open(str(archive)) as tar:
            names = tar.getnames()
            assert "hello.txt" in names

    def test_create_multiple_files(self, tmp_path: Path) -> None:
        """Create an archive with multiple files."""
        (tmp_path / "a.txt").write_text("aaa\n")
        (tmp_path / "b.txt").write_text("bbb\n")
        archive = tmp_path / "test.tar"

        result = create_archive(
            str(archive), ["a.txt", "b.txt"],
            directory=str(tmp_path),
        )
        assert result is True

        with tarfile.open(str(archive)) as tar:
            names = tar.getnames()
            assert "a.txt" in names
            assert "b.txt" in names

    def test_create_directory(self, tmp_path: Path) -> None:
        """Create an archive containing a directory."""
        d = tmp_path / "mydir"
        d.mkdir()
        (d / "file.txt").write_text("content\n")
        archive = tmp_path / "test.tar"

        result = create_archive(
            str(archive), ["mydir"],
            directory=str(tmp_path),
        )
        assert result is True

        with tarfile.open(str(archive)) as tar:
            names = tar.getnames()
            assert any("mydir" in n for n in names)

    def test_create_verbose(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """Verbose mode prints filenames."""
        f = tmp_path / "hello.txt"
        f.write_text("hello\n")
        archive = tmp_path / "test.tar"

        create_archive(
            str(archive), ["hello.txt"],
            directory=str(tmp_path),
            verbose=True,
        )
        captured = capsys.readouterr()
        assert "hello.txt" in captured.out

    def test_create_with_exclude(self, tmp_path: Path) -> None:
        """Exclude patterns filter out matching files."""
        (tmp_path / "keep.txt").write_text("keep\n")
        (tmp_path / "skip.bak").write_text("skip\n")
        archive = tmp_path / "test.tar"

        create_archive(
            str(archive), ["keep.txt", "skip.bak"],
            directory=str(tmp_path),
            exclude_patterns=["*.bak"],
        )

        with tarfile.open(str(archive)) as tar:
            names = tar.getnames()
            assert "keep.txt" in names
            assert "skip.bak" not in names

    def test_create_nonexistent_file(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """Nonexistent files are reported but don't fail the archive."""
        (tmp_path / "exists.txt").write_text("here\n")
        archive = tmp_path / "test.tar"

        result = create_archive(
            str(archive), ["exists.txt", "nope.txt"],
            directory=str(tmp_path),
        )
        assert result is True
        captured = capsys.readouterr()
        assert "nope.txt" in captured.err


# ---------------------------------------------------------------------------
# Creating compressed archives
# ---------------------------------------------------------------------------


class TestCompression:
    def test_create_gzip(self, tmp_path: Path) -> None:
        """Create a gzip-compressed archive."""
        (tmp_path / "file.txt").write_text("content\n")
        archive = tmp_path / "test.tar.gz"

        result = create_archive(
            str(archive), ["file.txt"],
            directory=str(tmp_path),
            gzip=True,
        )
        assert result is True
        assert archive.exists()

        # Verify it's a valid gzip tar.
        with tarfile.open(str(archive), "r:gz") as tar:
            assert "file.txt" in tar.getnames()

    def test_create_bzip2(self, tmp_path: Path) -> None:
        """Create a bzip2-compressed archive."""
        (tmp_path / "file.txt").write_text("content\n")
        archive = tmp_path / "test.tar.bz2"

        result = create_archive(
            str(archive), ["file.txt"],
            directory=str(tmp_path),
            bzip2=True,
        )
        assert result is True

        with tarfile.open(str(archive), "r:bz2") as tar:
            assert "file.txt" in tar.getnames()

    def test_create_xz(self, tmp_path: Path) -> None:
        """Create an xz-compressed archive."""
        (tmp_path / "file.txt").write_text("content\n")
        archive = tmp_path / "test.tar.xz"

        result = create_archive(
            str(archive), ["file.txt"],
            directory=str(tmp_path),
            xz=True,
        )
        assert result is True

        with tarfile.open(str(archive), "r:xz") as tar:
            assert "file.txt" in tar.getnames()


# ---------------------------------------------------------------------------
# Extracting archives
# ---------------------------------------------------------------------------


class TestExtractArchive:
    def _make_archive(self, tmp_path: Path) -> Path:
        """Helper: create a simple test archive."""
        src_dir = tmp_path / "src"
        src_dir.mkdir()
        (src_dir / "hello.txt").write_text("hello world\n")
        (src_dir / "sub").mkdir()
        (src_dir / "sub" / "nested.txt").write_text("nested\n")

        archive = tmp_path / "test.tar"
        with tarfile.open(str(archive), "w") as tar:
            tar.add(str(src_dir / "hello.txt"), arcname="hello.txt")
            tar.add(str(src_dir / "sub"), arcname="sub")
            tar.add(str(src_dir / "sub" / "nested.txt"), arcname="sub/nested.txt")

        return archive

    def test_extract_all(self, tmp_path: Path) -> None:
        """Extract all files from an archive."""
        archive = self._make_archive(tmp_path)
        extract_dir = tmp_path / "extracted"
        extract_dir.mkdir()

        result = extract_archive(str(archive), directory=str(extract_dir))
        assert result is True
        assert (extract_dir / "hello.txt").read_text() == "hello world\n"
        assert (extract_dir / "sub" / "nested.txt").read_text() == "nested\n"

    def test_extract_specific_files(self, tmp_path: Path) -> None:
        """Extract only specific files."""
        archive = self._make_archive(tmp_path)
        extract_dir = tmp_path / "extracted"
        extract_dir.mkdir()

        result = extract_archive(
            str(archive), ["hello.txt"],
            directory=str(extract_dir),
        )
        assert result is True
        assert (extract_dir / "hello.txt").exists()
        assert not (extract_dir / "sub").exists()

    def test_extract_verbose(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """Verbose extraction prints filenames."""
        archive = self._make_archive(tmp_path)
        extract_dir = tmp_path / "extracted"
        extract_dir.mkdir()

        extract_archive(str(archive), directory=str(extract_dir), verbose=True)
        captured = capsys.readouterr()
        assert "hello.txt" in captured.out

    def test_extract_missing_archive(self, tmp_path: Path) -> None:
        """Extracting from a missing archive returns False."""
        result = extract_archive(str(tmp_path / "nonexistent.tar"))
        assert result is False

    def test_extract_keep_old_files(self, tmp_path: Path) -> None:
        """Keep old files: existing files are not overwritten."""
        archive = self._make_archive(tmp_path)
        extract_dir = tmp_path / "extracted"
        extract_dir.mkdir()
        existing = extract_dir / "hello.txt"
        existing.write_text("original content\n")

        extract_archive(
            str(archive),
            directory=str(extract_dir),
            keep_old_files=True,
        )
        assert existing.read_text() == "original content\n"

    def test_extract_gzip(self, tmp_path: Path) -> None:
        """Extract a gzip-compressed archive."""
        (tmp_path / "file.txt").write_text("content\n")
        archive = tmp_path / "test.tar.gz"

        create_archive(
            str(archive), ["file.txt"],
            directory=str(tmp_path),
            gzip=True,
        )

        extract_dir = tmp_path / "extracted"
        extract_dir.mkdir()

        result = extract_archive(
            str(archive),
            directory=str(extract_dir),
            gzip=True,
        )
        assert result is True
        assert (extract_dir / "file.txt").read_text() == "content\n"


# ---------------------------------------------------------------------------
# Listing archives
# ---------------------------------------------------------------------------


class TestListArchive:
    def _make_archive(self, tmp_path: Path) -> Path:
        """Helper: create a test archive."""
        src_dir = tmp_path / "src"
        src_dir.mkdir()
        (src_dir / "a.txt").write_text("aaa\n")
        (src_dir / "b.txt").write_text("bbb\n")

        archive = tmp_path / "test.tar"
        with tarfile.open(str(archive), "w") as tar:
            tar.add(str(src_dir / "a.txt"), arcname="a.txt")
            tar.add(str(src_dir / "b.txt"), arcname="b.txt")

        return archive

    def test_list_all(self, tmp_path: Path) -> None:
        """List all entries in an archive."""
        archive = self._make_archive(tmp_path)
        success, entries = list_archive(str(archive))
        assert success is True
        assert "a.txt" in entries
        assert "b.txt" in entries

    def test_list_verbose(self, tmp_path: Path) -> None:
        """Verbose listing shows detailed information."""
        archive = self._make_archive(tmp_path)
        success, entries = list_archive(str(archive), verbose=True)
        assert success is True
        # Verbose entries should contain permissions and size.
        assert len(entries) == 2
        assert any("a.txt" in e for e in entries)

    def test_list_specific_files(self, tmp_path: Path) -> None:
        """List only specific files."""
        archive = self._make_archive(tmp_path)
        success, entries = list_archive(str(archive), files=["a.txt"])
        assert success is True
        assert "a.txt" in entries
        assert "b.txt" not in entries

    def test_list_missing_archive(self, tmp_path: Path) -> None:
        """Listing a missing archive returns failure."""
        success, entries = list_archive(str(tmp_path / "nonexistent.tar"))
        assert success is False
        assert len(entries) > 0  # Should contain an error message.

    def test_list_gzip(self, tmp_path: Path) -> None:
        """List a gzip-compressed archive."""
        (tmp_path / "file.txt").write_text("content\n")
        archive = tmp_path / "test.tar.gz"

        create_archive(
            str(archive), ["file.txt"],
            directory=str(tmp_path),
            gzip=True,
        )

        success, entries = list_archive(str(archive), gzip=True)
        assert success is True
        assert "file.txt" in entries


# ---------------------------------------------------------------------------
# Strip components
# ---------------------------------------------------------------------------


class TestStripComponents:
    def test_strip_one(self, tmp_path: Path) -> None:
        """Stripping 1 component removes the top-level directory."""
        src_dir = tmp_path / "src"
        src_dir.mkdir()
        sub = src_dir / "prefix"
        sub.mkdir()
        (sub / "file.txt").write_text("content\n")

        archive = tmp_path / "test.tar"
        with tarfile.open(str(archive), "w") as tar:
            tar.add(str(sub / "file.txt"), arcname="prefix/file.txt")

        extract_dir = tmp_path / "extracted"
        extract_dir.mkdir()

        extract_archive(
            str(archive),
            directory=str(extract_dir),
            strip_components=1,
        )
        # "prefix/file.txt" with 1 component stripped -> "file.txt"
        assert (extract_dir / "file.txt").exists()
