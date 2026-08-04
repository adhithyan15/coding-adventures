from __future__ import annotations

import hashlib
import io
import sys
import tarfile
import tempfile
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS_DIR))

import setup_lua


class _Response(io.BytesIO):
    pass


class DownloadTests(unittest.TestCase):
    def test_falls_back_and_verifies_exact_bytes(self) -> None:
        payload = b"byte-identical official archive"
        expected_sha256 = hashlib.sha256(payload).hexdigest()
        attempts: list[str] = []

        def opener(
            request: setup_lua.urllib.request.Request, *, timeout: int
        ) -> _Response:
            self.assertEqual(setup_lua.DOWNLOAD_TIMEOUT_SECONDS, timeout)
            url = request.full_url
            attempts.append(url)
            if url == "https://primary.invalid/lua.tar.gz":
                raise OSError("primary unavailable")
            return _Response(payload)

        with tempfile.TemporaryDirectory() as directory:
            destination = Path(directory) / "lua.tar.gz"
            selected = setup_lua.download_verified_archive(
                destination,
                urls=(
                    "https://primary.invalid/lua.tar.gz",
                    "https://fallback.invalid/lua.tar.gz",
                ),
                expected_sha256=expected_sha256,
                opener=opener,
            )

            self.assertEqual("https://fallback.invalid/lua.tar.gz", selected)
            self.assertEqual(payload, destination.read_bytes())
            self.assertEqual(
                [
                    "https://primary.invalid/lua.tar.gz",
                    "https://fallback.invalid/lua.tar.gz",
                ],
                attempts,
            )

    def test_rejects_every_unverified_mirror(self) -> None:
        def opener(
            request: setup_lua.urllib.request.Request, *, timeout: int
        ) -> _Response:
            return _Response(b"tampered")

        with tempfile.TemporaryDirectory() as directory:
            destination = Path(directory) / "lua.tar.gz"
            with self.assertRaisesRegex(RuntimeError, "No verified Lua source"):
                setup_lua.download_verified_archive(
                    destination,
                    urls=("https://mirror.invalid/lua.tar.gz",),
                    expected_sha256=hashlib.sha256(b"official").hexdigest(),
                    opener=opener,
                )
            self.assertFalse(destination.exists())


class ExtractionTests(unittest.TestCase):
    def test_rejects_path_traversal(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            archive = root / "unsafe.tar.gz"
            with tarfile.open(archive, "w:gz") as output:
                entry = tarfile.TarInfo("../outside.txt")
                entry.size = 1
                output.addfile(entry, io.BytesIO(b"x"))

            with self.assertRaisesRegex(ValueError, "Unsafe path"):
                setup_lua.extract_verified_archive(archive, root / "extract")
            self.assertFalse((root / "outside.txt").exists())


class WindowsBuildPlanTests(unittest.TestCase):
    def test_entry_points_are_not_compiled_into_the_dll(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            source_root = Path(directory)
            source_dir = source_root / "src"
            source_dir.mkdir()
            for name in ("lapi.c", "lua.c", "luac.c", "print.c"):
                (source_dir / name).touch()

            groups = setup_lua.windows_source_groups(source_root)

            self.assertEqual([source_dir / "lapi.c"], groups["lib"])
            self.assertEqual([source_dir / "lua.c"], groups["lua"])
            self.assertEqual(
                [source_dir / "luac.c", source_dir / "print.c"],
                groups["luac"],
            )


class PinnedSourceTests(unittest.TestCase):
    def test_version_hash_and_fallbacks_are_pinned(self) -> None:
        self.assertEqual("5.4.7", setup_lua.LUA_VERSION)
        self.assertEqual(64, len(setup_lua.LUA_SHA256))
        self.assertEqual(
            "https://lua.org/ftp/lua-5.4.7.tar.gz", setup_lua.LUA_SOURCE_URLS[0]
        )
        self.assertIn("deb.debian.org", setup_lua.LUA_SOURCE_URLS[1])
        self.assertIn("archive.ubuntu.com", setup_lua.LUA_SOURCE_URLS[2])


if __name__ == "__main__":
    unittest.main()
