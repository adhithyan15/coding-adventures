"""Tests for the Engram release payload helpers."""

from __future__ import annotations

import os
import plistlib
import shutil
import struct
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
                "engram-swiftui-macos-v0.4.0.zip",
                "engram-flutter-linux-v0.4.0.zip",
                "engram-flutter-macos-v0.4.0.zip",
                "engram-flutter-windows-v0.4.0.zip",
                "engram-qt-macos-v0.4.0.zip",
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
                # Directories are stored explicitly. Without an entry of their
                # own an EMPTY directory is dropped in silence, and a macOS
                # `.app` bundle can need one to be well-formed.
                self.assertEqual(
                    names,
                    [
                        "engram-web-v0.3.0/SOURCE_COMMIT",
                        "engram-web-v0.3.0/assets/",
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
            (dist / "Engram.app" / "escape.txt").symlink_to("../../secret.txt")

            with self.assertRaises(ValueError) as caught:
                engram_release.archive_compose(
                    "0.4.0", "macos", dist, root / "out", COMMIT
                )
            self.assertIn("escapes the payload", str(caught.exception))

    def test_refuses_an_absolute_symlink_target(self) -> None:
        # An absolute target cannot be right in a relocatable payload, and it
        # publishes the runner's filesystem layout in a public download.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            dist = _write_compose_dist(root)
            (root / "secret.txt").write_text("SENSITIVE\n", encoding="utf-8")
            (dist / "Engram.app" / "escape.txt").symlink_to(root / "secret.txt")

            with self.assertRaises(ValueError) as caught:
                engram_release.archive_compose(
                    "0.4.0", "macos", dist, root / "out", COMMIT
                )
            self.assertIn("absolute", str(caught.exception))

    def test_refuses_a_link_that_escapes_only_after_extraction(self) -> None:
        # The subtle one. `Contents/app/../../../evil` lands back inside the
        # BUILD tree (which has a directory above the distribution root), so a
        # check asking "where does this point now" accepts it. After extraction
        # the same link resolves outside the payload root, which is what the
        # reader actually gets. Verified end-to-end before this guard existed.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            dist = _write_compose_dist(root)
            (root / "evil.txt").write_text("EVIL\n", encoding="utf-8")
            (dist / "Engram.app" / "Contents" / "app" / "sneaky").symlink_to(
                "../../../../evil.txt"
            )

            with self.assertRaises(ValueError) as caught:
                engram_release.archive_compose(
                    "0.4.0", "macos", dist, root / "out", COMMIT
                )
            self.assertIn("escapes the payload", str(caught.exception))

    def test_refuses_a_symlink_target_a_windows_extractor_would_split(self) -> None:
        # The member NAME was checked for backslashes and the symlink TARGET was
        # not -- two halves of one hazard, written separately, one of them
        # missed. Under `posixpath` this target is a single harmless-looking
        # in-root component; an extractor that reads `\` as a separator lands on
        # `../evil.exe`, outside the payload.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            dist = _write_compose_dist(root)
            (dist / "Engram.app" / "winlink").symlink_to("..\\..\\..\\evil.exe")

            with self.assertRaises(ValueError) as caught:
                engram_release.archive_compose(
                    "0.4.0", "macos", dist, root / "out", COMMIT
                )
            self.assertIn("unsafe symlink target", str(caught.exception))

    def test_refuses_an_empty_or_control_character_symlink_target(self) -> None:
        for target in ["", "a\nb"]:
            with self.subTest(target=target):
                with tempfile.TemporaryDirectory() as tmp:
                    root = Path(tmp)
                    dist = _write_compose_dist(root)
                    try:
                        (dist / "Engram.app" / "odd").symlink_to(target)
                    except (OSError, ValueError):
                        self.skipTest("filesystem rejects this link target")
                    with self.assertRaises(ValueError) as caught:
                        engram_release.archive_compose(
                            "0.4.0", "macos", dist, root / "out", COMMIT
                        )
                    self.assertIn("unsafe symlink target", str(caught.exception))

    def test_refuses_members_that_collide_when_case_is_folded(self) -> None:
        # On the reader's macOS or Windows machine these are one file, and the
        # second silently overwrites the first -- a payload that loses a file on
        # extraction while every check here passes.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            dist = _write_compose_dist(root)
            app = dist / "Engram.app" / "Contents" / "app"
            (app / "Config.json").write_text("{}\n", encoding="utf-8")
            try:
                (app / "config.json").write_text("{}\n", encoding="utf-8")
            except OSError:
                self.skipTest("filesystem is case-insensitive")
            if not (app / "Config.json").exists() or len(
                list(app.glob("[Cc]onfig.json"))
            ) < 2:
                self.skipTest("filesystem is case-insensitive")

            with self.assertRaises(ValueError) as caught:
                engram_release.archive_compose(
                    "0.4.0", "macos", dist, root / "out", COMMIT
                )
            self.assertIn("case is folded", str(caught.exception))

    def test_refuses_a_lowercase_source_commit_too(self) -> None:
        # The guard was exact-case but the collision is not. On a case-sensitive
        # build filesystem these archive as two distinct members with no warning
        # at all -- quieter than the bug it replaced -- and collapse on the
        # downloader's machine, where the planted value wins.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            dist = _write_compose_dist(root)
            (dist / "source_commit").write_text(f"{'b' * 40}\n", encoding="utf-8")

            with self.assertRaises(ValueError) as caught:
                engram_release.archive_compose(
                    "0.4.0", "macos", dist, root / "out", COMMIT
                )
            self.assertIn("SOURCE_COMMIT", str(caught.exception))

    def test_a_nested_source_commit_is_still_allowed(self) -> None:
        # It does not collide with the one at the root, so refusing it would be
        # a false positive -- and a guard that rejects legitimate payloads
        # breaks the release just as surely as one that misses an attack.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            dist = _write_compose_dist(root)
            nested = dist / "Engram.app" / "Contents" / "app" / "SOURCE_COMMIT"
            nested.write_text("unrelated\n", encoding="utf-8")

            output = engram_release.archive_compose(
                "0.4.0", "macos", dist, root / "out", COMMIT
            )
            with zipfile.ZipFile(output) as archive:
                self.assertEqual(
                    archive.read("engram-compose-macos-v0.4.0/SOURCE_COMMIT").decode(),
                    f"{COMMIT}\n",
                )

    def test_refuses_a_payload_carrying_its_own_source_commit(self) -> None:
        # Two members with the same name: readers take the LAST, so a planted
        # SOURCE_COMMIT wins over the real one and the provenance guarantee is
        # gone. Python only warns, and CI does not fail on warnings.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            dist = _write_compose_dist(root)
            (dist / "SOURCE_COMMIT").write_text(f"{'b' * 40}\n", encoding="utf-8")

            with self.assertRaises(ValueError) as caught:
                engram_release.archive_compose(
                    "0.4.0", "macos", dist, root / "out", COMMIT
                )
            self.assertIn("SOURCE_COMMIT", str(caught.exception))

    def test_a_rejected_payload_leaves_no_archive_behind(self) -> None:
        # A member rejected mid-walk used to leave a valid, readable, truncated
        # zip on disk -- which would satisfy both `if-no-files-found: error` and
        # the publish job's "files on disk equal the declared set" check.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            dist = _write_compose_dist(root)
            (root / "secret.txt").write_text("SENSITIVE\n", encoding="utf-8")
            (dist / "Engram.app" / "zzz-late").symlink_to("../../secret.txt")

            out = root / "out"
            with self.assertRaises(ValueError):
                engram_release.archive_compose("0.4.0", "macos", dist, out, COMMIT)
            self.assertEqual(
                sorted(p.name for p in out.iterdir()) if out.exists() else [],
                [],
                "a partial archive survived a rejection",
            )

    def test_a_symlink_loop_fails_with_a_message_not_a_traceback(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            dist = _write_compose_dist(root)
            app = dist / "Engram.app"
            (app / "a").symlink_to("b")
            (app / "b").symlink_to("a")

            # The message is asserted, not just the type. `Path.resolve()`
            # raises RuntimeError for a loop on macOS and silently tolerates it
            # on Linux, so this test passed on a developer machine for the
            # wrong reason and failed in CI. Naming the message pins that the
            # explicit walk is what refuses it, on every platform.
            with self.assertRaises(ValueError) as caught:
                engram_release.archive_compose(
                    "0.4.0", "macos", dist, root / "out", COMMIT
                )
            self.assertIn("symlink loop", str(caught.exception))

    def test_a_dangling_symlink_is_still_accepted(self) -> None:
        # The false-positive direction for the loop check: a jpackage bundle
        # legitimately contains links whose target is not present, and refusing
        # those would break the macOS build.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            dist = _write_compose_dist(root)
            (dist / "Engram.app" / "Fw").symlink_to("Versions/Current/Fw")

            output = engram_release.archive_compose(
                "0.4.0", "macos", dist, root / "out", COMMIT
            )
            with zipfile.ZipFile(output) as archive:
                member = "engram-compose-macos-v0.4.0/Engram.app/Fw"
                self.assertEqual(archive.read(member), b"Versions/Current/Fw")

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

    def test_refuses_a_name_windows_would_strip_to_another_member(self) -> None:
        # The bypass an allowlist makes unreachable rather than patching. Win32
        # strips a trailing dot from every path component, so `index.html.` and
        # `index.html` are two members here and ONE file on a Windows
        # downloader's machine -- the later one silently wins. No symlink and
        # no code execution needed: one file committed under a directory that
        # is copied into the payload verbatim.
        for name in ["config.json.", "config.json "]:
            with self.subTest(name=name):
                with tempfile.TemporaryDirectory() as tmp:
                    root = Path(tmp)
                    dist = _write_compose_dist(root)
                    app = dist / "Engram.app" / "Contents" / "app"
                    (app / "config.json").write_text("{}\n", encoding="utf-8")
                    (app / name).write_text("ATTACKER\n", encoding="utf-8")
                    with self.assertRaises(ValueError) as caught:
                        engram_release.archive_compose(
                            "0.4.0", "macos", dist, root / "out", COMMIT
                        )
                    self.assertIn("dot or space", str(caught.exception))

    def test_refuses_a_reserved_windows_device_name(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            dist = _write_compose_dist(root)
            (dist / "Engram.app" / "NUL.txt").write_text("x\n", encoding="utf-8")
            with self.assertRaises(ValueError) as caught:
                engram_release.archive_compose(
                    "0.4.0", "macos", dist, root / "out", COMMIT
                )
            self.assertIn("reserved Windows device name", str(caught.exception))

    def test_refuses_a_name_outside_the_publishable_character_set(self) -> None:
        # A colon is an NTFS alternate-data-stream separator; the allowlist
        # rejects it without having had to know that.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            dist = _write_compose_dist(root)
            (dist / "Engram.app" / "a:b.txt").write_text("x\n", encoding="utf-8")
            with self.assertRaises(ValueError) as caught:
                engram_release.archive_compose(
                    "0.4.0", "macos", dist, root / "out", COMMIT
                )
            self.assertIn("publishable character set", str(caught.exception))

    def test_the_real_payload_shapes_are_accepted(self) -> None:
        # The allowlist's cost is false positives, which fail the release. These
        # are the shapes the actual builds emit -- Vite hashes, a jpackage
        # bundle, a code-signature directory, spaced names -- measured against
        # the app tree rather than guessed.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            dist = _write_compose_dist(root)
            app = dist / "Engram.app" / "Contents"
            (app / "_CodeSignature").mkdir()
            (app / "_CodeSignature" / "CodeResources").write_text("<x/>", encoding="utf-8")
            (app / "Info.plist").write_text("<x/>", encoding="utf-8")
            (app / "app" / "index-D4f_9a2b.js").write_text("//", encoding="utf-8")
            (app / "app" / "NotoSansTamil-Static.ttf").write_bytes(b"\x00\x01\x00\x00")
            (app / "app" / "ASSEMBLY_EXCEPTION").write_text("x", encoding="utf-8")
            (app / "app" / "Java Runtime.cfg").write_text("x", encoding="utf-8")
            (app / "app" / ".jpackage.xml").write_text("<x/>", encoding="utf-8")

            output = engram_release.archive_compose(
                "0.4.0", "macos", dist, root / "out", COMMIT
            )
            self.assertTrue(output.is_file())

    def test_refuses_a_hard_link(self) -> None:
        # `_write_symlink` refuses to follow a symlink out of the payload; a
        # hard link is the same reach with none of the tells.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            dist = _write_compose_dist(root)
            secret = root / "secret.txt"
            secret.write_text("AWS_SECRET=hunter2\n", encoding="utf-8")
            try:
                os.link(secret, dist / "Engram.app" / "harmless.txt")
            except OSError:
                self.skipTest("filesystem does not support hard links")
            with self.assertRaises(ValueError) as caught:
                engram_release.archive_compose(
                    "0.4.0", "macos", dist, root / "out", COMMIT
                )
            self.assertIn("hard link", str(caught.exception))

    def test_an_empty_directory_survives(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            dist = _write_compose_dist(root)
            (dist / "Engram.app" / "Contents" / "PlugIns").mkdir()
            output = engram_release.archive_compose(
                "0.4.0", "macos", dist, root / "out", COMMIT
            )
            with zipfile.ZipFile(output) as archive:
                self.assertIn(
                    "engram-compose-macos-v0.4.0/Engram.app/Contents/PlugIns/",
                    archive.namelist(),
                )

    def test_setuid_and_world_write_are_not_published(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            dist = _write_compose_dist(root)
            planted = dist / "Engram.app" / "planted.sh"
            planted.write_text("#!/bin/sh\n", encoding="utf-8")
            planted.chmod(0o6777)

            output = engram_release.archive_compose(
                "0.4.0", "macos", dist, root / "out", COMMIT
            )
            member = "engram-compose-macos-v0.4.0/Engram.app/planted.sh"
            with zipfile.ZipFile(output) as archive:
                mode = archive.getinfo(member).external_attr >> 16
            self.assertEqual(mode & 0o7000, 0, "setuid/setgid survived")
            self.assertEqual(mode & 0o022, 0, "group/other write survived")
            self.assertTrue(mode & 0o100, "the executable bit was lost")

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


def _mach_o(
    defined: list[str],
    undefined: list[str] = [],
    *,
    dylibs: list[str] | None = None,
    rpaths: list[str] | None = None,
    signed: bool = False,
) -> bytes:
    """A minimal 64-bit Mach-O carrying these symbols, dependencies, and signature.

    Synthesised rather than compiled, so the tests run on the Linux runner too
    -- there is no Mach-O toolchain there, and a fixture that could only be
    built on macOS would skip exactly where CI runs most. Built to the real
    layout, because the parser reads that layout.
    """

    names = [(name, N_SECT) for name in defined] + [
        (name, N_UNDF) for name in undefined
    ]

    strings = b"\x00"
    offsets = []
    for name, _ in names:
        offsets.append(len(strings))
        strings += f"_{name}".encode() + b"\x00"

    commands = b""
    for path in dylibs or []:
        raw = path.encode() + b"\x00"
        size = 24 + len(raw)
        size += (-size) % 8  # commands are 8-byte aligned
        body = struct.pack("<IIIIII", 0x0C, size, 24, 0, 0, 0) + raw
        commands += body.ljust(size, b"\x00")
    for path in rpaths or []:
        raw = path.encode() + b"\x00"
        size = 12 + len(raw)
        size += (-size) % 8
        body = struct.pack("<III", 0x1C | 0x80000000, size, 12) + raw
        commands += body.ljust(size, b"\x00")
    if signed:
        commands += struct.pack("<IIII", 0x1D, 16, 0, 0)

    symtab_size = 24
    symoff = 32 + len(commands) + symtab_size
    stroff = symoff + len(names) * 16
    commands += struct.pack(
        "<IIIIII", 0x2, symtab_size, symoff, len(names), stroff, len(strings)
    )
    ncmds = len(dylibs or []) + len(rpaths or []) + (1 if signed else 0) + 1

    header = struct.pack(
        "<IiiIIIII", 0xFEEDFACF, 0x0100000C, 0, 2, ncmds, len(commands), 0, 0
    )
    table = b"".join(
        struct.pack("<IBBHQ", offset, kind | 0x01, 1, 0, 0x1000)
        for offset, (_, kind) in zip(offsets, names)
    )
    return header + commands + table + strings


def _fat_mach_o(slices: list[bytes]) -> bytes:
    """A universal binary wrapping the given thin slices."""

    header = struct.pack(">II", 0xCAFEBABE, len(slices))
    offset = 8 + 20 * len(slices)
    offset += (-offset) % 4096
    body = b""
    for index, thin in enumerate(slices):
        start = offset + len(body)
        header += struct.pack(">iiIII", 0x0100000C, 0, start, len(thin), 12)
        body += thin
    return header.ljust(offset, b"\x00") + body


N_SECT = 0x0E
N_UNDF = 0x00

ENGINE_SYMBOLS = [
    "eg_engram_app_props",
    "eg_handle_engram_app_event",
    "eg_session_new_demo",
    "eg_snapshot",
    "eg_load_snapshot",
    "eg_export_anki_apkg",
    "eg_merge_anki_apkg",
    "eg_session_free",
    "eg_string_free",
]


def _write_swiftui_bundle(
    root: Path,
    *,
    symbols: int = 9,
    undefined: int = 0,
    executable_name: str = "Engram",
) -> Path:
    """A stand-in for the `.app` the SwiftUI build produces."""

    bundle = root / "swiftui" / "Engram.app"
    (bundle / "Contents" / "MacOS").mkdir(parents=True)
    (bundle / "Contents" / "Resources").mkdir(parents=True)
    (bundle / "Contents" / "Info.plist").write_bytes(
        plistlib.dumps(
            {
                "CFBundleExecutable": executable_name,
                "CFBundleIdentifier": "dev.mosaic.engram",
                "CFBundlePackageType": "APPL",
            }
        )
    )
    (bundle / "Contents" / "PkgInfo").write_text("APPL????", encoding="utf-8")
    binary = bundle / "Contents" / "MacOS" / executable_name
    binary.write_bytes(
        _mach_o(ENGINE_SYMBOLS[:symbols], ENGINE_SYMBOLS[symbols : symbols + undefined])
    )
    binary.chmod(0o755)
    return bundle


class ArchiveSwiftUITests(unittest.TestCase):
    def test_archives_only_the_app_bundle(self) -> None:
        # The bug this caught in review: archiving the SwiftPM project directory
        # instead swept in `.build/` -- module caches, object files, the dSYM,
        # the static archive -- turning a 3.8 MB app into a 47 MB download. The
        # app ran perfectly either way, so only the member list showed it.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            bundle = _write_swiftui_bundle(root)
            # Litter beside the bundle, exactly as a real build leaves it.
            (bundle.parent / ".build" / "release").mkdir(parents=True)
            (bundle.parent / ".build" / "release" / "App").write_bytes(b"\x00" * 4096)
            (bundle.parent / "Package.swift").write_text("// swift-tools-version:5.10\n")

            output = engram_release.archive_swiftui(
                "0.4.0", "macos", bundle, root / "out", COMMIT
            )
            self.assertEqual(output.name, "engram-swiftui-macos-v0.4.0.zip")
            with zipfile.ZipFile(output) as archive:
                names = archive.namelist()
            self.assertEqual(
                [n for n in names if ".build" in n or "Package.swift" in n],
                [],
                "build litter reached the payload",
            )
            self.assertIn(
                "engram-swiftui-macos-v0.4.0/Engram.app/Contents/MacOS/Engram", names
            )
            self.assertIn("engram-swiftui-macos-v0.4.0/SOURCE_COMMIT", names)

    def test_the_executable_stays_executable(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            bundle = _write_swiftui_bundle(root)
            output = engram_release.archive_swiftui(
                "0.4.0", "macos", bundle, root / "out", COMMIT
            )
            member = "engram-swiftui-macos-v0.4.0/Engram.app/Contents/MacOS/Engram"
            with zipfile.ZipFile(output) as archive:
                mode = archive.getinfo(member).external_attr >> 16
            self.assertTrue(mode & 0o111, f"lost the executable bit: {mode:o}")

    def test_refuses_a_binary_without_the_engine_linked(self) -> None:
        # SwiftUI links the engine statically, so there is no library beside the
        # binary to look for: the engine is inside the executable or every deck
        # operation silently does nothing. This is the check that says so.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            bundle = _write_swiftui_bundle(root, symbols=0)
            with self.assertRaises(ValueError) as caught:
                engram_release.archive_swiftui(
                    "0.4.0", "macos", bundle, root / "out", COMMIT
                )
            self.assertIn("engine did not link", str(caught.exception))

    def test_refuses_a_partially_linked_binary(self) -> None:
        # Below the floor but not zero -- the shape a link that pulled in some
        # objects and not others would take.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            bundle = _write_swiftui_bundle(root, symbols=2)
            with self.assertRaises(ValueError) as caught:
                engram_release.archive_swiftui(
                    "0.4.0", "macos", bundle, root / "out", COMMIT
                )
            self.assertIn("only 2 DEFINED engine symbols", str(caught.exception))

    def test_refuses_a_bundle_with_no_info_plist(self) -> None:
        # Without it macOS does not treat the directory as an application: it
        # opens as a folder rather than launching.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            bundle = _write_swiftui_bundle(root)
            (bundle / "Contents" / "Info.plist").unlink()
            with self.assertRaises(ValueError) as caught:
                engram_release.archive_swiftui(
                    "0.4.0", "macos", bundle, root / "out", COMMIT
                )
            self.assertIn("Info.plist", str(caught.exception))

    def test_refuses_a_bundle_with_no_executable(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            bundle = _write_swiftui_bundle(root)
            (bundle / "Contents" / "MacOS" / "Engram").unlink()
            with self.assertRaises(ValueError) as caught:
                engram_release.archive_swiftui(
                    "0.4.0", "macos", bundle, root / "out", COMMIT
                )
            self.assertIn("must hold exactly", str(caught.exception))

    def test_refuses_something_that_is_not_a_bundle(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            plain = root / "notanapp"
            plain.mkdir()
            with self.assertRaises(ValueError) as caught:
                engram_release.archive_swiftui(
                    "0.4.0", "macos", plain, root / "out", COMMIT
                )
            self.assertIn(".app", str(caught.exception))

    def test_refuses_a_binary_that_only_references_the_engine(self) -> None:
        # The case a byte scan cannot see, and the reason this reads the symbol
        # TABLE. Mach-O stores defined and undefined names identically in the
        # string table, so searching the file for `eg_...` gives the same answer
        # for a binary that CONTAINS the engine and one that merely expects it
        # from a library that will not be there. Confirmed against a pair of
        # compiled fixtures: the byte scan passed both.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            bundle = _write_swiftui_bundle(root, symbols=0, undefined=9)
            with self.assertRaises(ValueError) as caught:
                engram_release.archive_swiftui(
                    "0.4.0", "macos", bundle, root / "out", COMMIT
                )
            message = str(caught.exception)
            self.assertIn("0 DEFINED engine symbols", message)
            self.assertIn("undefined: eg_", message)

    def test_refuses_a_partially_linked_binary_with_undefined_symbols(self) -> None:
        # Enough defined symbols to clear the floor, but some still undefined --
        # the app would load part of the engine and fail on the rest.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            bundle = _write_swiftui_bundle(root, symbols=6, undefined=3)
            with self.assertRaises(ValueError) as caught:
                engram_release.archive_swiftui(
                    "0.4.0", "macos", bundle, root / "out", COMMIT
                )
            self.assertIn("leaves engine symbols undefined", str(caught.exception))

    def test_scans_the_executable_the_plist_names(self) -> None:
        # A sorted glob takes whichever file sorts first, so a second file in
        # `Contents/MacOS` would be verified while the executable macOS actually
        # launches ships unexamined -- and both get published.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            bundle = _write_swiftui_bundle(root, symbols=0)  # the REAL one: empty
            decoy = bundle / "Contents" / "MacOS" / "Assets"  # sorts before Engram
            decoy.write_bytes(_mach_o(ENGINE_SYMBOLS))
            with self.assertRaises(ValueError) as caught:
                engram_release.archive_swiftui(
                    "0.4.0", "macos", bundle, root / "out", COMMIT
                )
            self.assertIn("must hold exactly", str(caught.exception))

    def test_refuses_a_symlink_that_escapes_the_bundle(self) -> None:
        # Staging puts the payload root ABOVE the bundle, so `_zip_tree`'s own
        # containment check would allow a link that leaves the `.app` and lands
        # beside it. Combined with a redirected `Contents/MacOS`, the verified
        # binary would not be in the archive at all.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            bundle = _write_swiftui_bundle(root)
            (root / "outside").mkdir()
            (root / "outside" / "planted").write_text("x\n", encoding="utf-8")
            (bundle / "Contents" / "Resources" / "link").symlink_to("../../../outside")

            with self.assertRaises(ValueError) as caught:
                engram_release.archive_swiftui(
                    "0.4.0", "macos", bundle, root / "out", COMMIT
                )
            self.assertIn("escapes the bundle", str(caught.exception))

    def test_a_link_inside_the_bundle_is_kept(self) -> None:
        # The false-positive direction: a `.app` may legitimately link within
        # itself, and refusing that would break a valid build.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            bundle = _write_swiftui_bundle(root)
            (bundle / "Contents" / "Resources" / "alias").symlink_to("../PkgInfo")

            output = engram_release.archive_swiftui(
                "0.4.0", "macos", bundle, root / "out", COMMIT
            )
            member = (
                "engram-swiftui-macos-v0.4.0/Engram.app/Contents/Resources/alias"
            )
            with zipfile.ZipFile(output) as archive:
                self.assertEqual(archive.read(member), b"../PkgInfo")

    def test_refuses_a_symlinked_macos_directory(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            bundle = _write_swiftui_bundle(root)
            macos = bundle / "Contents" / "MacOS"
            elsewhere = root / "elsewhere"
            elsewhere.mkdir()
            (elsewhere / "Engram").write_bytes(_mach_o(ENGINE_SYMBOLS))
            shutil.rmtree(macos)
            macos.symlink_to(elsewhere)

            with self.assertRaises(ValueError) as caught:
                engram_release.archive_swiftui(
                    "0.4.0", "macos", bundle, root / "out", COMMIT
                )
            self.assertIn("not a real directory", str(caught.exception))

    def test_swiftui_name_is_in_the_declared_set(self) -> None:
        declared = set(engram_release.artifact_names("0.4.0"))
        for platform in engram_release.SWIFTUI_TARGETS:
            self.assertIn(
                engram_release.swiftui_artifact_name("0.4.0", platform), declared
            )

    def test_swiftui_is_macos_only(self) -> None:
        # Not an oversight to be corrected later: it is a SwiftUI app, so there
        # is nowhere else for it to run.
        self.assertEqual(sorted(engram_release.SWIFTUI_TARGETS), ["macos"])
        with self.assertRaises(ValueError):
            engram_release.swiftui_artifact_name("0.4.0", "linux")


ENGINE_FILENAMES = {
    "linux": "libengram_capi.so",
    "macos": "libengram_capi.dylib",
    "windows": "engram_capi.dll",
}


def _write_flutter_bundle(
    root: Path, platform: str, *, engine_dir: str | None = None, empty: bool = False
) -> Path:
    """A stand-in for what `flutter build <platform>` leaves behind.

    `engine_dir` overrides where the library is placed, which is how the
    wrong-layout cases are built -- the whole point being that each platform's
    loader reads a different directory.
    """

    names = {"linux": "bundle", "macos": "engram.app", "windows": "Release"}
    bundle = root / platform / names[platform]
    bundle.mkdir(parents=True)

    if platform == "macos":
        (bundle / "Contents" / "MacOS").mkdir(parents=True)
        (bundle / "Contents" / "MacOS" / "engram").write_bytes(b"\xcf\xfa\xed\xfe")
    else:
        (bundle / "engram").write_bytes(b"\x7fELF")

    where = engine_dir if engine_dir is not None else (
        engram_release.FLUTTER_ENGINE_DIRS[platform]
    )
    target = bundle / where if where else bundle
    target.mkdir(parents=True, exist_ok=True)
    # Real container magic, because the check reads it: ELF, Mach-O, and PE.
    # A fixture with made-up bytes would exercise the filename match and skip
    # the part that distinguishes a library from a `.pdb`.
    magic = {"linux": b"\x7fELF", "macos": b"\xcf\xfa\xed\xfe", "windows": b"MZ"}
    (target / ENGINE_FILENAMES[platform]).write_bytes(
        b"" if empty else magic[platform] + b"\x00rest-of-the-library\x00"
    )
    return bundle


class ArchiveFlutterTests(unittest.TestCase):
    def test_archives_each_platform(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for platform in engram_release.FLUTTER_TARGETS:
                with self.subTest(platform=platform):
                    bundle = _write_flutter_bundle(root, platform)
                    output = engram_release.archive_flutter(
                        "0.4.0", platform, bundle, root / f"out-{platform}", COMMIT
                    )
                    self.assertEqual(
                        output.name, f"engram-flutter-{platform}-v0.4.0.zip"
                    )
                    self.assertIn(
                        output.name, engram_release.artifact_names("0.4.0")
                    )

    def test_refuses_a_bundle_with_no_engine(self) -> None:
        # `flutter build` succeeds whether or not the engine was copied in, so
        # this is a runtime failure behind a green build: the app launches and
        # every deck operation silently does nothing.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            bundle = _write_flutter_bundle(root, "macos")
            next(bundle.rglob("libengram_capi.dylib")).unlink()
            with self.assertRaises(ValueError) as caught:
                engram_release.archive_flutter(
                    "0.4.0", "macos", bundle, root / "out", COMMIT
                )
            self.assertIn("no engram_capi engine", str(caught.exception))

    def test_refuses_an_engine_in_the_wrong_place_for_this_platform(self) -> None:
        # The trap this backend actually has. Flutter puts native libraries in
        # `Contents/Frameworks` on macOS, `lib` on Linux, and beside the
        # executable on Windows. An engine present but in another platform's
        # location passes any "is it in the bundle" check and produces an app
        # whose loader cannot find it -- which is exactly how the Compose
        # backend shipped a distribution with no usable engine.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            bundle = _write_flutter_bundle(root, "macos", engine_dir="lib")
            with self.assertRaises(ValueError) as caught:
                engram_release.archive_flutter(
                    "0.4.0", "macos", bundle, root / "out", COMMIT
                )
            message = str(caught.exception)
            self.assertIn("not where macos looks for it", message)
            self.assertIn("Contents/Frameworks", message)

    def test_each_platform_rejects_the_others_layout(self) -> None:
        # So the layout table cannot be quietly collapsed to one directory.
        others = {
            "linux": "Contents/Frameworks",
            "macos": "lib",
            "windows": "lib",
        }
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for platform, wrong in others.items():
                with self.subTest(platform=platform):
                    bundle = _write_flutter_bundle(
                        root / platform, platform, engine_dir=wrong
                    )
                    with self.assertRaises(ValueError) as caught:
                        engram_release.archive_flutter(
                            "0.4.0", platform, bundle, root / "o", COMMIT
                        )
                    self.assertIn("not where", str(caught.exception))

    def test_windows_wants_the_engine_beside_the_executable(self) -> None:
        # The one platform whose directory is the bundle root, so an empty
        # string in the layout table is meaningful rather than a placeholder.
        self.assertEqual(engram_release.FLUTTER_ENGINE_DIRS["windows"], "")
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            bundle = _write_flutter_bundle(root, "windows")
            self.assertTrue((bundle / "engram_capi.dll").is_file())
            engram_release.archive_flutter(
                "0.4.0", "windows", bundle, root / "out", COMMIT
            )

    def test_refuses_an_empty_engine(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            # An empty file cannot carry container magic, so it is refused as
            # "not a library" rather than "empty" -- the stricter check
            # subsumes the size test rather than reaching it.
            bundle = _write_flutter_bundle(root, "linux", empty=True)
            with self.assertRaises(ValueError) as caught:
                engram_release.archive_flutter(
                    "0.4.0", "linux", bundle, root / "out", COMMIT
                )
            self.assertIn("no engram_capi engine", str(caught.exception))

    def test_a_symlinked_engine_does_not_satisfy_the_check(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            bundle = _write_flutter_bundle(root, "linux")
            engine = bundle / "lib" / "libengram_capi.so"
            real = bundle / "lib" / "other.bin"
            real.write_bytes(b"\x00")
            engine.unlink()
            engine.symlink_to("other.bin")
            with self.assertRaises(ValueError) as caught:
                engram_release.archive_flutter(
                    "0.4.0", "linux", bundle, root / "out", COMMIT
                )
            self.assertIn("engram_capi", str(caught.exception))

    def test_refuses_debug_symbols_named_like_the_engine(self) -> None:
        # A Rust cdylib on Windows emits `engram_capi.dll`,
        # `engram_capi.dll.lib` and `engram_capi.pdb` side by side, so a copy
        # step with a sloppy glob can pick up the debug symbols alone. A check
        # that matched on filename accepted exactly that.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            bundle = _write_flutter_bundle(root, "windows")
            (bundle / "engram_capi.dll").unlink()
            (bundle / "engram_capi.pdb").write_bytes(b"Microsoft C/C++ MSF 7.00")
            with self.assertRaises(ValueError) as caught:
                engram_release.archive_flutter(
                    "0.4.0", "windows", bundle, root / "out", COMMIT
                )
            self.assertIn("no engram_capi engine", str(caught.exception))

    def test_refuses_a_file_with_the_right_name_and_wrong_contents(self) -> None:
        for content, label in [(b"x", "one byte"), (b"not a library\n", "text")]:
            with self.subTest(label=label):
                with tempfile.TemporaryDirectory() as tmp:
                    root = Path(tmp)
                    bundle = _write_flutter_bundle(root, "linux")
                    (bundle / "lib" / "libengram_capi.so").write_bytes(content)
                    with self.assertRaises(ValueError) as caught:
                        engram_release.archive_flutter(
                            "0.4.0", "linux", bundle, root / "out", COMMIT
                        )
                    self.assertIn("no engram_capi engine", str(caught.exception))

    def test_accepts_a_versioned_soname(self) -> None:
        # `Path.stem` strips one suffix, so a real versioned soname
        # (`libengram_capi.so.0.4.0`) failed a stem-based match while
        # `engram_capi.pdb` passed it. Both directions were wrong.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            bundle = _write_flutter_bundle(root, "linux")
            engine = bundle / "lib" / "libengram_capi.so"
            engine.rename(engine.with_name("libengram_capi.so.0.4.0"))
            engram_release.archive_flutter(
                "0.4.0", "linux", bundle, root / "out", COMMIT
            )

    def test_refuses_a_hard_link_that_staging_would_launder(self) -> None:
        # `_zip_tree` refuses hard links -- they reach outside the payload with
        # none of a symlink's tells -- but `shutil.copytree` DEREFERENCES one,
        # writing a fresh file with nlink 1, so that guard can never fire on a
        # staged tree. Verified before the fix: the outside file's bytes came
        # back out of the published archive verbatim.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            bundle = _write_flutter_bundle(root, "linux")
            secret = root / "outside-secret.txt"
            secret.write_text("AUTHORIZATION: basic TOKEN\n", encoding="utf-8")
            try:
                os.link(secret, bundle / "lib" / "innocuous.dat")
            except OSError:
                self.skipTest("filesystem does not support hard links")

            with self.assertRaises(ValueError) as caught:
                engram_release.archive_flutter(
                    "0.4.0", "linux", bundle, root / "out", COMMIT
                )
            self.assertIn("hard link", str(caught.exception))

    def test_swiftui_also_refuses_a_hard_link(self) -> None:
        # Same laundering, same fix: the SwiftUI archiver stages the same way.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            bundle = _write_swiftui_bundle(root)
            secret = root / "outside-secret.txt"
            secret.write_text("AUTHORIZATION: basic TOKEN\n", encoding="utf-8")
            try:
                os.link(secret, bundle / "Contents" / "Resources" / "innocuous.dat")
            except OSError:
                self.skipTest("filesystem does not support hard links")

            with self.assertRaises(ValueError) as caught:
                engram_release.archive_swiftui(
                    "0.4.0", "macos", bundle, root / "out", COMMIT
                )
            self.assertIn("hard link", str(caught.exception))

    def test_flutter_ships_every_desktop_platform(self) -> None:
        self.assertEqual(
            sorted(engram_release.FLUTTER_TARGETS), ["linux", "macos", "windows"]
        )
        self.assertEqual(
            sorted(engram_release.FLUTTER_ENGINE_DIRS),
            sorted(engram_release.FLUTTER_TARGETS),
            "every target needs a declared engine directory",
        )

    def test_rejects_an_unknown_platform(self) -> None:
        with self.assertRaises(ValueError):
            engram_release.flutter_artifact_name("0.4.0", "solaris")


def _write_qt_bundle(
    root: Path, *, engine: bool = True, relocatable: bool = True, signed: bool = True
) -> Path:
    """A stand-in for the Qt `.app` after macdeployqt."""

    bundle = root / "qt" / "Engram.app"
    (bundle / "Contents" / "MacOS").mkdir(parents=True)
    (bundle / "Contents" / "Frameworks").mkdir(parents=True)
    (bundle / "Contents" / "Info.plist").write_bytes(
        plistlib.dumps({"CFBundleExecutable": "Engram", "CFBundlePackageType": "APPL"})
    )
    deps = ["@executable_path/../Frameworks/QtCore.framework/Versions/A/QtCore"]
    if not relocatable:
        deps.append("/opt/homebrew/opt/qtbase/lib/QtGui.framework/Versions/A/QtGui")
    (bundle / "Contents" / "MacOS" / "Engram").write_bytes(
        _mach_o(["main"], dylibs=deps, signed=signed)
    )
    if engine:
        (bundle / "Contents" / "MacOS" / "libengram_capi.dylib").write_bytes(
            _mach_o(["eg_snapshot"], signed=True)
        )
    return bundle


class ArchiveQtTests(unittest.TestCase):
    def test_archives_a_deployed_bundle(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            bundle = _write_qt_bundle(root)
            output = engram_release.archive_qt(
                "0.4.0", "macos", bundle, root / "out", COMMIT
            )
            self.assertEqual(output.name, "engram-qt-macos-v0.4.0.zip")
            self.assertIn(output.name, engram_release.artifact_names("0.4.0"))

    def test_refuses_a_bundle_that_links_qt_by_absolute_path(self) -> None:
        # THE check for a Qt payload. `qt_add_executable` links the frameworks
        # at absolute paths, so an undeployed binary runs perfectly for whoever
        # built it and fails to launch for everyone else -- and nothing about
        # the build says so. Measured against the real binary before
        # macdeployqt ran: 17 dependencies, 8 of them absolute Homebrew paths.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            bundle = _write_qt_bundle(root, relocatable=False)
            with self.assertRaises(ValueError) as caught:
                engram_release.archive_qt(
                    "0.4.0", "macos", bundle, root / "out", COMMIT
                )
            message = str(caught.exception)
            self.assertIn("would not launch on a machine without them", message)
            self.assertIn("/opt/homebrew", message)

    def test_refuses_an_unsigned_bundle(self) -> None:
        # Separate from relocatability, because a bundle can pass that check
        # completely and still not start: macdeployqt INVALIDATES signatures
        # when it rewrites install names, and on arm64 the loader refuses such
        # a binary. Found the hard way -- the dependency check reported zero
        # absolute paths on a bundle that exited instantly with no output.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            bundle = _write_qt_bundle(root, signed=False)
            with self.assertRaises(ValueError) as caught:
                engram_release.archive_qt(
                    "0.4.0", "macos", bundle, root / "out", COMMIT
                )
            self.assertIn("no code signature", str(caught.exception))

    def test_refuses_a_bundle_without_the_engine(self) -> None:
        # Qt does `QDir(appDir).filePath(...)`, and for a bundled app `appDir`
        # is Contents/MacOS -- so the engine belongs beside the executable, not
        # in Frameworks.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            bundle = _write_qt_bundle(root, engine=False)
            with self.assertRaises(ValueError) as caught:
                engram_release.archive_qt(
                    "0.4.0", "macos", bundle, root / "out", COMMIT
                )
            self.assertIn("no engine beside its executable", str(caught.exception))

    def test_an_engine_in_frameworks_does_not_count(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            bundle = _write_qt_bundle(root, engine=False)
            (bundle / "Contents" / "Frameworks" / "libengram_capi.dylib").write_bytes(
                _mach_o(["eg_snapshot"], signed=True)
            )
            with self.assertRaises(ValueError) as caught:
                engram_release.archive_qt(
                    "0.4.0", "macos", bundle, root / "out", COMMIT
                )
            self.assertIn("no engine beside its executable", str(caught.exception))

    def test_qt_declares_only_what_is_verified(self) -> None:
        # macOS only for now: `macdeployqt` and `windeployqt` ship with Qt and
        # Linux has no official equivalent, so each platform is its own piece of
        # work. Declaring more than is built would fail the publish job's
        # set-equality check at the very end of a release.
        self.assertEqual(sorted(engram_release.QT_TARGETS), ["macos"])
        with self.assertRaises(ValueError):
            engram_release.qt_artifact_name("0.4.0", "linux")


class MachOReaderTests(unittest.TestCase):
    def test_reports_absolute_dependencies_and_ignores_system_ones(self) -> None:
        binary = _mach_o(
            ["main"],
            dylibs=[
                "/usr/lib/libSystem.B.dylib",
                "/System/Library/Frameworks/AppKit.framework/AppKit",
                "@executable_path/../Frameworks/QtCore.framework/QtCore",
                "@rpath/libthing.dylib",
                "/opt/homebrew/opt/qtbase/lib/QtGui.framework/QtGui",
            ],
            rpaths=["@loader_path/../Frameworks"],
        )
        self.assertEqual(
            engram_release.non_relocatable_dependencies(binary, depth_in_bundle=2),
            ["/opt/homebrew/opt/qtbase/lib/QtGui.framework/QtGui"],
        )

    def test_parent_hops_are_bounded_by_depth_not_forbidden(self) -> None:
        # `@executable_path/../Frameworks/...` is the STANDARD macdeployqt
        # install name: from `Contents/MacOS` it climbs one level and lands
        # inside the bundle. Treating any `..` as an escape rejected every
        # correctly deployed Qt app -- caught only by running the check against
        # a real bundle rather than against fixtures I had written myself.
        legitimate = _mach_o(
            ["m"],
            dylibs=["@executable_path/../Frameworks/QtCore.framework/QtCore"],
            signed=True,
        )
        self.assertEqual(
            engram_release.non_relocatable_dependencies(legitimate, depth_in_bundle=2),
            [],
        )
        escaping = _mach_o(
            ["m"], dylibs=["@executable_path/../../../outside.dylib"], signed=True
        )
        self.assertEqual(
            len(engram_release.non_relocatable_dependencies(escaping, depth_in_bundle=2)),
            1,
        )

    def test_a_stray_rpath_is_tolerated_when_nothing_resolves_through_it(self) -> None:
        # Measured on the real bundle: 4 binaries carry an outside LC_RPATH and
        # none of them has an `@rpath` dependency, so dyld never consults it.
        # Refusing there would have failed every Qt release over a path that
        # does nothing.
        binary = _mach_o(
            ["m"],
            dylibs=["@executable_path/../Frameworks/x.dylib"],
            rpaths=["/opt/homebrew/Cellar/dbus/1.16.2_1/lib"],
            signed=True,
        )
        self.assertEqual(
            engram_release.non_relocatable_dependencies(binary, depth_in_bundle=2), []
        )

    def test_an_rpath_dependency_with_no_in_bundle_rpath_is_refused(self) -> None:
        # The case that genuinely does not resolve on another machine.
        binary = _mach_o(
            ["m"],
            dylibs=["@rpath/QtCore.framework/QtCore"],
            rpaths=["/opt/homebrew/opt/qtbase/lib"],
            signed=True,
        )
        stray = engram_release.non_relocatable_dependencies(binary, depth_in_bundle=2)
        self.assertEqual(len(stray), 1)
        self.assertIn("no in-bundle LC_RPATH", stray[0])

    def test_refuses_a_malformed_binary_rather_than_hanging(self) -> None:
        # `cmdsize` of zero never advances the cursor and `ncmds` comes from the
        # header, so an unvalidated walk can be stalled by its own input --
        # 50 million iterations were measured on a 64-byte file.
        blob = (
            struct.pack("<IiiIIIII", 0xFEEDFACF, 0x0100000C, 0, 2, 0xFFFFFFFF, 16, 0, 0)
            + struct.pack("<II", 0x0C, 0)
            + b"\x00" * 64
        )
        with self.assertRaises(ValueError):
            engram_release.dylib_dependencies(blob)

    def test_is_code_signed_refuses_malformed_input_rather_than_hanging(self) -> None:
        # `dylib_dependencies` has a second guard (the name-offset check) that
        # happens to catch a zero `cmdsize` too, so mutating the size guard did
        # not fail the suite through that path. `is_code_signed` has no such
        # backstop: with the guard removed, this 64-byte input hung for over six
        # seconds; with it, the refusal is immediate.
        blob = (
            struct.pack("<IiiIIIII", 0xFEEDFACF, 0x0100000C, 0, 2, 0xFFFFFFFF, 16, 0, 0)
            + struct.pack("<II", 0x1, 0)
            + b"\x00" * 64
        )
        with self.assertRaises(ValueError):
            engram_release.is_code_signed(blob)

    def test_refuses_an_implausible_fat_header(self) -> None:
        # A chain of fat headers turned a 2 MB file into 2 GB of RSS.
        blob = struct.pack(">II", 0xCAFEBABE, 0xFFFF) + b"\x00" * 256
        with self.assertRaises(ValueError):
            engram_release.dylib_dependencies(blob)

    def test_a_fat_binary_must_have_every_slice_signed(self) -> None:
        # The earlier form returned True for a fat header followed by zeros --
        # and a Qt build for `arm64;x86_64` IS universal, the normal shape for a
        # public macOS release, so the gate would have stopped asserting
        # anything the moment the build went universal.
        self.assertFalse(
            engram_release.is_code_signed(struct.pack(">II", 0xCAFEBABE, 0) + b"\x00" * 512)
        )
        signed = _fat_mach_o([_mach_o(["m"], signed=True)])
        unsigned = _fat_mach_o([_mach_o(["m"], signed=True), _mach_o(["m"], signed=False)])
        self.assertTrue(engram_release.is_code_signed(signed))
        self.assertFalse(engram_release.is_code_signed(unsigned))

    def test_reads_every_slice_of_a_universal_binary(self) -> None:
        # Omitting this was not hypothetical: `/bin/ls` is universal, so the
        # first control reached for could not be parsed at all.
        thin = _mach_o(["main"], dylibs=["/opt/homebrew/lib/libx.dylib"])
        fat = _fat_mach_o([thin])
        self.assertEqual(
            engram_release.non_relocatable_dependencies(fat, depth_in_bundle=2),
            ["/opt/homebrew/lib/libx.dylib"],
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
