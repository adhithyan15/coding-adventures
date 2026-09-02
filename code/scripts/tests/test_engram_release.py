"""Tests for the Engram release payload helpers."""

from __future__ import annotations

import sys
import tempfile
import unittest
import zipfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import engram_release  # noqa: E402


COMMIT = "a" * 40


class ValidateIdentifiersTests(unittest.TestCase):
    def test_accepts_a_matching_version_and_tag(self) -> None:
        engram_release.validate_identifiers("0.3.0", "engram-v0.3.0", COMMIT)

    def test_accepts_prerelease_and_build_metadata(self) -> None:
        engram_release.validate_identifiers("1.0.0-rc.1", "engram-v1.0.0-rc.1")
        engram_release.validate_identifiers("1.0.0+build.5", "engram-v1.0.0+build.5")

    def test_rejects_loose_versions(self) -> None:
        # A release tag is a permanent public identifier, so near-misses are
        # rejected rather than normalised.
        for version in ["1.2", "1.2.3.4", "01.2.3", "v1.2.3", "1.2.3 ", ""]:
            with self.subTest(version=version):
                with self.assertRaises(ValueError):
                    engram_release.validate_identifiers(
                        version, f"engram-v{version}"
                    )

    def test_rejects_a_tag_that_does_not_match_its_version(self) -> None:
        with self.assertRaises(ValueError):
            engram_release.validate_identifiers("0.3.0", "engram-v0.3.1")

    def test_rejects_another_products_tag_prefix(self) -> None:
        # Publishing an Engram payload under `task-app-v…` would be silently
        # wrong rather than obviously wrong, so the prefix is part of the check.
        with self.assertRaises(ValueError):
            engram_release.validate_identifiers("0.3.0", "task-app-v0.3.0")

    def test_rejects_a_short_or_malformed_commit(self) -> None:
        for commit in ["abc123", "z" * 40, "A" * 39]:
            with self.subTest(commit=commit):
                with self.assertRaises(ValueError):
                    engram_release.validate_identifiers(
                        "0.3.0", "engram-v0.3.0", commit
                    )

    def test_accepts_an_uppercase_commit(self) -> None:
        engram_release.validate_identifiers("0.3.0", "engram-v0.3.0", "A" * 40)


class ArtifactNamesTests(unittest.TestCase):
    def test_names_the_web_and_desktop_payloads(self) -> None:
        self.assertEqual(
            engram_release.artifact_names("0.4.0"),
            [
                "engram-web-v0.4.0.zip",
                "engram-desktop-linux-v0.4.0.AppImage",
                "engram-desktop-macos-v0.4.0.zip",
                "engram-desktop-windows-v0.4.0.exe",
            ],
        )

    def test_desktop_names_match_the_declared_set(self) -> None:
        # The publish job asserts the files on disk equal `artifact_names`, so
        # a per-platform name that drifts from that list turns a successful
        # build into a failed release -- and vice versa, a release that quietly
        # ships less than it claims.
        declared = set(engram_release.artifact_names("0.4.0"))
        for platform in engram_release.DESKTOP_TARGETS:
            self.assertIn(
                engram_release.desktop_artifact_name("0.4.0", platform), declared
            )

    def test_macos_ships_a_zip_not_a_dmg(self) -> None:
        # Deliberate: signing and notarisation need credentials this build does
        # not have, and macOS refuses an unsigned dmg with an error that reads
        # like file corruption. A zip is honest about what it is.
        self.assertTrue(
            engram_release.desktop_artifact_name("0.4.0", "macos").endswith(".zip")
        )

    def test_rejects_an_unknown_platform(self) -> None:
        with self.assertRaises(ValueError):
            engram_release.desktop_artifact_name("0.4.0", "solaris")

    def test_rejects_an_invalid_version(self) -> None:
        with self.assertRaises(ValueError):
            engram_release.artifact_names("0.3")
        with self.assertRaises(ValueError):
            engram_release.desktop_artifact_name("0.3", "linux")


def _write_bundle(
    root: Path,
    *,
    index: bool = True,
    wasm: bool = True,
    assets: bool = True,
    empty_wasm: bool = False,
    root_absolute: bool = False,
) -> Path:
    """Build a web-bundle directory, optionally with a piece missing."""

    source = root / "dist"
    source.mkdir(parents=True, exist_ok=True)
    if index:
        href = "/assets/index-abc.js" if root_absolute else "./assets/index-abc.js"
        (source / "index.html").write_text(
            f'<!doctype html>\n<script type="module" src="{href}"></script>\n',
            encoding="utf-8",
        )
    if wasm:
        payload = b"" if empty_wasm else b"\x00asm\x01\x00\x00\x00"
        (source / engram_release.WASM_ENGINE).write_bytes(payload)
    if assets:
        (source / "assets").mkdir(exist_ok=True)
        (source / "assets" / "index-abc.js").write_text("export {}\n", encoding="utf-8")
    return source


class ArchiveWebTests(unittest.TestCase):
    def test_archives_a_complete_bundle(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = _write_bundle(root)
            output = engram_release.archive_web("0.3.0", COMMIT, source, root / "out")

            self.assertTrue(output.is_file())
            self.assertEqual(output.name, "engram-web-v0.3.0.zip")
            with zipfile.ZipFile(output) as archive:
                names = sorted(archive.namelist())
                self.assertEqual(
                    names,
                    [
                        "engram-web-v0.3.0/SOURCE_COMMIT",
                        "engram-web-v0.3.0/assets/index-abc.js",
                        "engram-web-v0.3.0/engram_engine.wasm",
                        "engram-web-v0.3.0/index.html",
                    ],
                )
                self.assertEqual(
                    archive.read("engram-web-v0.3.0/SOURCE_COMMIT").decode(),
                    f"{COMMIT}\n",
                )

    def test_refuses_a_bundle_without_the_engine(self) -> None:
        # The case that matters most: a Vite build succeeds whether or not the
        # engine reached dist/, so this is a runtime failure hiding behind a
        # green build — the user gets an app that cannot import a deck.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = _write_bundle(root, wasm=False)
            with self.assertRaises(ValueError) as caught:
                engram_release.archive_web("0.3.0", COMMIT, source, root / "out")
            self.assertIn(engram_release.WASM_ENGINE, str(caught.exception))

    def test_refuses_an_empty_engine(self) -> None:
        # Present but zero-length would satisfy an existence check and fail at
        # load, which is expensive to diagnose in the wild.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = _write_bundle(root, empty_wasm=True)
            with self.assertRaises(ValueError) as caught:
                engram_release.archive_web("0.3.0", COMMIT, source, root / "out")
            self.assertIn("empty", str(caught.exception))

    def test_refuses_a_bundle_without_an_entry_point_or_assets(self) -> None:
        for kwargs, expected in [
            ({"index": False}, "index.html"),
            ({"assets": False}, "assets"),
        ]:
            with self.subTest(**kwargs):
                with tempfile.TemporaryDirectory() as tmp:
                    root = Path(tmp)
                    source = _write_bundle(root, **kwargs)
                    with self.assertRaises(ValueError) as caught:
                        engram_release.archive_web(
                            "0.3.0", COMMIT, source, root / "out"
                        )
                    self.assertIn(expected, str(caught.exception))

    def test_refuses_a_bundle_that_only_works_at_a_domain_root(self) -> None:
        # The defect v0.3.0 shipped with. Vite defaults to `base: "/"`, so the
        # entry point referenced `/assets/index-HASH.js`. Served from any
        # subdirectory, index.html returns 200 and its script 404s: a blank page
        # that looks like a working deploy. Every other check here asks whether
        # a file is present; this one asks whether the references resolve.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = _write_bundle(root, root_absolute=True)
            with self.assertRaises(ValueError) as caught:
                engram_release.archive_web("0.3.0", COMMIT, source, root / "out")
            message = str(caught.exception)
            self.assertIn("/assets/index-abc.js", message)
            self.assertIn("root of a domain", message)

    def test_accepts_relative_asset_references(self) -> None:
        # The corrected shape, so the guard cannot pass by rejecting everything.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = _write_bundle(root)
            engram_release.archive_web("0.3.0", COMMIT, source, root / "out")

    def test_refuses_a_source_directory_that_does_not_exist(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            with self.assertRaises(ValueError):
                engram_release.archive_web(
                    "0.3.0", COMMIT, root / "nope", root / "out"
                )

    def test_refuses_a_bad_commit_before_writing_anything(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = _write_bundle(root)
            out_dir = root / "out"
            with self.assertRaises(ValueError):
                engram_release.archive_web("0.3.0", "abc", source, out_dir)
            # Validation happens first, so a rejected release leaves no partial
            # payload behind for a later step to pick up.
            self.assertFalse(out_dir.exists())

    def test_archive_is_reproducible(self) -> None:
        # Members are written in sorted order, so the same tree yields the same
        # bytes regardless of filesystem iteration order.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = _write_bundle(root)
            first = engram_release.archive_web("0.3.0", COMMIT, source, root / "a")
            second = engram_release.archive_web("0.3.0", COMMIT, source, root / "b")
            with zipfile.ZipFile(first) as one, zipfile.ZipFile(second) as two:
                self.assertEqual(one.namelist(), two.namelist())


class CommandLineTests(unittest.TestCase):
    def test_validate_succeeds(self) -> None:
        self.assertEqual(
            engram_release.main(
                ["validate", "--version", "0.3.0", "--tag", "engram-v0.3.0"]
            ),
            0,
        )

    def test_validate_reports_a_mismatch_without_a_traceback(self) -> None:
        self.assertEqual(
            engram_release.main(
                ["validate", "--version", "0.3.0", "--tag", "engram-v9.9.9"]
            ),
            1,
        )

    def test_artifact_names_lists_the_payloads(self) -> None:
        self.assertEqual(
            engram_release.main(["artifact-names", "--version", "0.3.0"]), 0
        )


if __name__ == "__main__":
    unittest.main()
