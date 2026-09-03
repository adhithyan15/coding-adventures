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
    def test_names_the_web_desktop_and_compose_payloads(self) -> None:
        # Spelled out rather than derived from the tables this asserts against,
        # so adding a payload has to be a deliberate edit here as well. The
        # publish job compares the files on disk to this set, and a list that
        # silently grew with its source would let a release ship a payload
        # nobody decided to ship.
        self.assertEqual(
            engram_release.artifact_names("0.4.0"),
            [
                "engram-web-v0.4.0.zip",
                "engram-desktop-linux-v0.4.0.AppImage",
                "engram-desktop-macos-v0.4.0.zip",
                "engram-desktop-windows-v0.4.0.exe",
                "engram-compose-linux-v0.4.0.zip",
                "engram-compose-macos-v0.4.0.zip",
                "engram-compose-windows-v0.4.0.zip",
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

    def test_compose_names_match_the_declared_set(self) -> None:
        # Same reasoning as the desktop names: the publish job asserts the files
        # on disk equal `artifact_names`, so a Compose payload named one way and
        # declared another turns a successful build into a failed release.
        declared = set(engram_release.artifact_names("0.4.0"))
        for platform in engram_release.COMPOSE_TARGETS:
            self.assertIn(
                engram_release.compose_artifact_name("0.4.0", platform), declared
            )

    def test_compose_ships_every_platform(self) -> None:
        # Electron alone would not demonstrate the thing Mosaic exists to prove:
        # that one declarative package yields real native apps everywhere. The
        # JVM makes Compose the cheapest breadth of the five native backends, so
        # a release missing a platform here is a gap worth failing on.
        self.assertEqual(
            sorted(engram_release.COMPOSE_TARGETS),
            ["linux", "macos", "windows"],
        )

    def test_compose_rejects_an_unknown_platform(self) -> None:
        with self.assertRaises(ValueError):
            engram_release.compose_artifact_name("0.4.0", "solaris")

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


def _write_compose_dist(root: Path) -> Path:
    """A stand-in for what `createDistributable` leaves behind."""

    dist = root / "app" / "Engram.app" / "Contents"
    (dist / "app").mkdir(parents=True)
    (dist / "MacOS").mkdir(parents=True)
    (dist / "app" / "engram-host.jar").write_bytes(b"PK\x03\x04stub")
    (dist / "app" / "libengram_capi.dylib").write_bytes(b"\xcf\xfa\xed\xfe")
    launcher = dist / "MacOS" / "Engram"
    launcher.write_text("#!/bin/sh\nexec java -jar app/engram-host.jar\n")
    launcher.chmod(0o755)
    return root / "app"


class ArchiveComposeTests(unittest.TestCase):
    def test_archives_a_distribution_under_a_named_root(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            dist = _write_compose_dist(root)
            output = engram_release.archive_compose(
                "0.4.0", "macos", dist, root / "out", COMMIT
            )

            self.assertEqual(output.name, "engram-compose-macos-v0.4.0.zip")
            with zipfile.ZipFile(output) as archive:
                names = archive.namelist()
                self.assertIn(
                    "engram-compose-macos-v0.4.0/Engram.app/Contents/app/"
                    "libengram_capi.dylib",
                    names,
                )
                self.assertEqual(
                    archive.read(
                        "engram-compose-macos-v0.4.0/SOURCE_COMMIT"
                    ).decode(),
                    f"{COMMIT}\n",
                )

    def test_the_launcher_is_still_executable_after_a_round_trip(self) -> None:
        # The reason this uses `zipfile` rather than a shell command: whatever
        # archives the app has to carry the executable bits, or it extracts to
        # a bundle that cannot be launched -- a broken download that every
        # structural check passes. `ZipFile.write` puts the mode in
        # `external_attr`; this is the test that says so.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            dist = _write_compose_dist(root)
            output = engram_release.archive_compose(
                "0.4.0", "macos", dist, root / "out", COMMIT
            )
            member = "engram-compose-macos-v0.4.0/Engram.app/Contents/MacOS/Engram"
            with zipfile.ZipFile(output) as archive:
                mode = archive.getinfo(member).external_attr >> 16
            self.assertTrue(mode & 0o111, f"launcher lost its executable bit: {mode:o}")

    def test_refuses_a_distribution_with_no_engine(self) -> None:
        # The bug this repository has already shipped once in another form:
        # Gradle's `createDistributable` succeeds whether or not the engine was
        # copied in, so the app launches and then cannot open a deck. The shell
        # script's symbol check runs `nm`, which is skipped on Windows -- this
        # is the check that covers every platform.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            dist = _write_compose_dist(root)
            next(dist.rglob("libengram_capi.dylib")).unlink()
            with self.assertRaises(ValueError) as caught:
                engram_release.archive_compose(
                    "0.4.0", "macos", dist, root / "out", COMMIT
                )
            self.assertIn("engram_capi", str(caught.exception))

    def test_refuses_an_empty_engine(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            dist = _write_compose_dist(root)
            next(dist.rglob("libengram_capi.dylib")).write_bytes(b"")
            with self.assertRaises(ValueError) as caught:
                engram_release.archive_compose(
                    "0.4.0", "macos", dist, root / "out", COMMIT
                )
            self.assertIn("empty", str(caught.exception))

    def test_accepts_the_engine_under_each_platform_name(self) -> None:
        # The filename differs per platform (`libengram_capi.so`,
        # `libengram_capi.dylib`, `engram_capi.dll`), so a check written against
        # one spelling would reject the other two -- turning a correct build
        # into a failed release.
        for filename in [
            "libengram_capi.so",
            "libengram_capi.dylib",
            "engram_capi.dll",
        ]:
            with self.subTest(filename=filename):
                with tempfile.TemporaryDirectory() as tmp:
                    root = Path(tmp)
                    dist = _write_compose_dist(root)
                    engine = next(dist.rglob("libengram_capi.dylib"))
                    engine.rename(engine.with_name(filename))
                    engram_release.archive_compose(
                        "0.4.0", "macos", dist, root / "out", COMMIT
                    )

    def test_a_symlink_is_stored_as_a_link_not_as_its_targets_bytes(self) -> None:
        # The one that matters most. `ZipFile.write` opens the path, so a
        # symlink would be archived as its TARGET'S CONTENT under an innocuous
        # in-tree name -- and this archive is published as a public release
        # asset. One planted link would bake `.git/config` (which holds the
        # checkout token) into a download that looks entirely ordinary.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            dist = _write_compose_dist(root)
            secret = dist / "Engram.app" / "Contents" / "app" / "real.txt"
            secret.write_text("SENSITIVE\n", encoding="utf-8")
            link = secret.with_name("link.txt")
            link.symlink_to("real.txt")

            output = engram_release.archive_compose(
                "0.4.0", "macos", dist, root / "out", COMMIT
            )
            member = (
                "engram-compose-macos-v0.4.0/Engram.app/Contents/app/link.txt"
            )
            with zipfile.ZipFile(output) as archive:
                info = archive.getinfo(member)
                self.assertNotIn(b"SENSITIVE", archive.read(member))
                self.assertEqual(archive.read(member), b"real.txt")
            # S_IFLNK, so an extractor recreates a link rather than a file.
            self.assertEqual((info.external_attr >> 16) & 0o170000, 0o120000)

    def test_a_stored_symlink_extracts_as_a_symlink(self) -> None:
        # The mode bits above are only a claim about what an extractor will do.
        # This runs a real one and looks at the result.
        import shutil
        import subprocess

        if shutil.which("unzip") is None:
            self.skipTest("unzip is not installed")
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            dist = _write_compose_dist(root)
            target = dist / "Engram.app" / "Contents" / "app" / "real.txt"
            target.write_text("payload\n", encoding="utf-8")
            (target.with_name("link.txt")).symlink_to("real.txt")

            output = engram_release.archive_compose(
                "0.4.0", "macos", dist, root / "out", COMMIT
            )
            out = root / "extracted"
            out.mkdir()
            subprocess.run(
                ["unzip", "-q", str(output), "-d", str(out)], check=True
            )
            restored = (
                out
                / "engram-compose-macos-v0.4.0"
                / "Engram.app"
                / "Contents"
                / "app"
                / "link.txt"
            )
            self.assertTrue(restored.is_symlink(), "extracted as a plain file")
            self.assertEqual(restored.read_text(), "payload\n")

    def test_files_under_a_symlinked_directory_are_not_silently_dropped(self) -> None:
        # `rglob` does not descend into a symlinked directory, so following
        # links loses everything beneath one while still reporting success. A
        # macOS `.app` from jpackage bundles a runtime full of symlinked
        # directories, which is exactly this case.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            dist = _write_compose_dist(root)
            contents = dist / "Engram.app" / "Contents"
            (contents / "runtime").mkdir()
            (contents / "runtime" / "release").write_text("JAVA\n", encoding="utf-8")
            (contents / "Home").symlink_to("runtime")

            output = engram_release.archive_compose(
                "0.4.0", "macos", dist, root / "out", COMMIT
            )
            with zipfile.ZipFile(output) as archive:
                self.assertIn(
                    "engram-compose-macos-v0.4.0/Engram.app/Contents/Home",
                    archive.namelist(),
                )

    def test_refuses_a_symlink_pointing_outside_the_payload(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            dist = _write_compose_dist(root)
            (root / "secret.txt").write_text("SENSITIVE\n", encoding="utf-8")
            (dist / "Engram.app" / "escape.txt").symlink_to(root / "secret.txt")

            with self.assertRaises(ValueError) as caught:
                engram_release.archive_compose(
                    "0.4.0", "macos", dist, root / "out", COMMIT
                )
            self.assertIn("escapes the payload", str(caught.exception))

    def test_a_symlink_cannot_satisfy_the_engine_check(self) -> None:
        # Otherwise a link named `libengram_capi.dylib` would pass the presence
        # and non-empty checks while shipping no engine at all.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            dist = _write_compose_dist(root)
            engine = next(dist.rglob("libengram_capi.dylib"))
            other = engine.with_name("something.txt")
            other.write_text("not an engine\n", encoding="utf-8")
            engine.unlink()
            engine.symlink_to("something.txt")

            with self.assertRaises(ValueError) as caught:
                engram_release.archive_compose(
                    "0.4.0", "macos", dist, root / "out", COMMIT
                )
            self.assertIn("engram_capi", str(caught.exception))

    def test_refuses_a_member_name_a_windows_extractor_would_split(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            dist = _write_compose_dist(root)
            (dist / "Engram.app" / "..\\..\\evil.exe").write_bytes(b"MZ")
            with self.assertRaises(ValueError) as caught:
                engram_release.archive_compose(
                    "0.4.0", "macos", dist, root / "out", COMMIT
                )
            self.assertIn("unsafe archive member name", str(caught.exception))

    def test_refuses_a_distribution_that_was_never_built(self) -> None:
        # The failure this replaced was the opposite shape -- a packaging step
        # that could not find its tool. A missing distribution should say so
        # rather than produce an empty archive the publish job accepts.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            with self.assertRaises(ValueError) as caught:
                engram_release.archive_compose(
                    "0.4.0", "macos", root / "nope", root / "out", COMMIT
                )
            self.assertIn("does not exist", str(caught.exception))

    def test_refuses_an_unknown_platform_and_a_bad_commit(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            dist = _write_compose_dist(root)
            with self.assertRaises(ValueError):
                engram_release.archive_compose(
                    "0.4.0", "solaris", dist, root / "out", COMMIT
                )
            with self.assertRaises(ValueError):
                engram_release.archive_compose(
                    "0.4.0", "macos", dist, root / "out", "abc"
                )

    def test_every_platform_the_release_declares_can_be_archived(self) -> None:
        # Windows is the point of this test: `zip` does not exist in Git Bash
        # there, so the platform built and then failed to package. Nothing in
        # this path is platform-specific any more, and this pins that.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            dist = _write_compose_dist(root)
            for platform in engram_release.COMPOSE_TARGETS:
                with self.subTest(platform=platform):
                    output = engram_release.archive_compose(
                        "0.4.0", platform, dist, root / platform, COMMIT
                    )
                    self.assertTrue(output.is_file())
                    self.assertIn(
                        output.name, engram_release.artifact_names("0.4.0")
                    )


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
