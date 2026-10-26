"""Tests for the Mosaic Journal release helper and its workflow (J5e, #14416).

Two halves:

* the helper, ``code/scripts/journal_release.py``: every check it makes about
  what may ship, run against small payloads built in temporary directories;
* the workflow, ``.github/workflows/release-journal.yml``: its trigger and
  security rules, read as text. Text rather than YAML because the validate job
  that runs this test has only the standard library, and a guard that needs a
  dependency CI lacks would end up skipped. Skipped guards guard nothing.
"""

from __future__ import annotations

import io
import json
import os
import re
import stat
import sys
import tempfile
import unittest
import zipfile
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import journal_release  # noqa: E402

COMMIT = "a" * 40
REPOSITORY_ROOT = Path(__file__).resolve().parents[3]
WORKFLOW = REPOSITORY_ROOT / ".github" / "workflows" / "release-journal.yml"
CI = REPOSITORY_ROOT / ".github" / "workflows" / "ci.yml"
JOURNAL = REPOSITORY_ROOT / "code" / "programs" / "mosaic" / "journal-app"


class TempDirTestCase(unittest.TestCase):
    def setUp(self) -> None:
        self._temp = tempfile.TemporaryDirectory()
        self.root = Path(self._temp.name)

    def tearDown(self) -> None:
        self._temp.cleanup()


# Builders for the small payloads the tests archive. Module-level so every
# test class, including the command-line round trip, builds the same shapes.


def make_changelog(root: Path, heading: str = "## 0.1.0 \u2014 unreleased",
                   shards: dict[str, str] | None = None, version: str = "0.1.0") -> tuple[Path, Path]:
    changelog = root / "CHANGELOG.d"
    changelog.mkdir(exist_ok=True)
    (changelog / "_meta.md").write_text(f"# Changelog\n\n{heading}\n\n", encoding="utf-8")
    if shards is None:
        shards = {
            "00010-FIRST-0123abcd.md": "- **First.** The oldest entry.\n",
            "00020-SECOND-89abcdef.md": "- **Second.** The newest entry.\n  Wrapped line.\n",
        }
    for name, text in shards.items():
        (changelog / name).write_text(text, encoding="utf-8")
    manifest = root / "mosaic-package.toml"
    manifest.write_text(f'[package]\nname = "journal-app"\nversion = "{version}"\n', encoding="utf-8")
    return changelog, manifest


def make_web_bundle(root: Path) -> tuple[Path, Path]:
    dist = root / "dist"
    (dist / "assets").mkdir(parents=True)
    (dist / "index.html").write_text(
        '<script type="module" src="./assets/index-abc.js"></script>', encoding="utf-8"
    )
    (dist / "assets" / "index-abc.js").write_text("fetch('journal_mosaic_app.wasm')", encoding="utf-8")
    (dist / journal_release.WASM_RUNTIME).write_bytes(b"\0asm runtime")
    built = root / "built.wasm"
    built.write_bytes(b"\0asm runtime")
    return dist, built


def make_native_tree(root: Path, backend: str) -> tuple[Path, Path, Path, Path]:
    """A tree shaped like the real one: Qt installs the engine beside the
    binary in bin/; a Compose app image keeps it under lib/app/resources."""

    target = journal_release.NATIVE_TARGETS[backend]
    bundle = root / f"{backend}-bundle"
    (bundle / "bin").mkdir(parents=True)
    executable = bundle / "bin" / target["executable"]
    executable.write_bytes(b"#!/bin/sh\n")
    executable.chmod(0o755)
    runtime_dir = bundle / ("lib/app/resources" if backend == "compose" else "bin")
    runtime_dir.mkdir(parents=True, exist_ok=True)
    runtime = runtime_dir / target["runtime"]
    runtime.write_bytes(b"\x7fELF engine")
    expected = root / f"libjournal_mosaic_app-{backend}.so"
    expected.write_bytes(b"\x7fELF engine")
    return bundle, executable, runtime, expected



# --------------------------------------------------------------------------
# Identity
# --------------------------------------------------------------------------


class ValidateIdentifiersTests(unittest.TestCase):
    def test_accepts_a_matching_release(self) -> None:
        journal_release.validate_identifiers("0.1.0", "journal-v0.1.0", COMMIT)
        journal_release.validate_identifiers("1.2.3-rc.1+build.7", "journal-v1.2.3-rc.1+build.7")

    def test_the_pull_request_dry_run_identity_is_valid(self) -> None:
        # The workflow uses this pair on every pull request.
        journal_release.validate_identifiers("0.0.0-ci", "journal-v0.0.0-ci", COMMIT)

    def test_rejects_versions_that_are_not_strict_semver(self) -> None:
        for version in ("0.1", "01.0.0", "1.2.3.4", "v0.1.0", "", "0.1.0-"):
            with self.subTest(version=version), self.assertRaises(ValueError):
                journal_release.validate_identifiers(version, f"journal-v{version}")

    def test_rejects_a_tag_that_does_not_match(self) -> None:
        for tag in ("journal-0.1.0", "journal-v0.1.1", "task-app-v0.1.0", "journal-v0.1.0 "):
            with self.subTest(tag=tag), self.assertRaises(ValueError):
                journal_release.validate_identifiers("0.1.0", tag)

    def test_rejects_a_commit_that_is_not_a_full_sha(self) -> None:
        for commit in ("abc123", "g" * 40, "a" * 39, "a" * 41):
            with self.subTest(commit=commit), self.assertRaises(ValueError):
                journal_release.validate_identifiers("0.1.0", "journal-v0.1.0", commit)


class ManifestVersionTests(TempDirTestCase):
    def test_reads_the_package_version(self) -> None:
        manifest = self.root / "mosaic-package.toml"
        manifest.write_text('[package]\nname = "journal-app"\nversion = "0.3.1"\n', encoding="utf-8")
        self.assertEqual(journal_release.manifest_version(manifest), "0.3.1")

    def test_refuses_a_missing_or_loose_version(self) -> None:
        for body in ("[package]\nname = 'x'\n", "[package]\nversion = '0.1'\n", "version = '0.1.0'\n", "[[["):
            manifest = self.root / "mosaic-package.toml"
            manifest.write_text(body, encoding="utf-8")
            with self.subTest(body=body), self.assertRaises(ValueError):
                journal_release.manifest_version(manifest)

    def test_refuses_a_missing_file(self) -> None:
        with self.assertRaises(ValueError):
            journal_release.manifest_version(self.root / "absent.toml")

    def test_the_real_manifest_has_a_strict_version(self) -> None:
        journal_release.manifest_version(JOURNAL / "mosaic-package.toml")


# --------------------------------------------------------------------------
# Changelog
# --------------------------------------------------------------------------


class ChangelogTests(TempDirTestCase):
    def make(self, heading: str = "## 0.1.0 \u2014 unreleased", shards: dict[str, str] | None = None,
             version: str = "0.1.0") -> tuple[Path, Path]:
        return make_changelog(self.root, heading, shards, version)

    def test_an_unreleased_section_passes_a_dry_run(self) -> None:
        changelog, manifest = self.make()
        self.assertEqual(journal_release.validate_changelog(changelog, manifest), "0.1.0")

    def test_an_unreleased_section_cannot_be_published(self) -> None:
        changelog, manifest = self.make()
        with self.assertRaisesRegex(ValueError, "still 'unreleased'"):
            journal_release.validate_changelog(changelog, manifest, "0.1.0")

    def test_a_dated_section_publishes_its_own_version_only(self) -> None:
        changelog, manifest = self.make("## 0.1.0 — 2026-11-02")
        self.assertEqual(journal_release.validate_changelog(changelog, manifest, "0.1.0"), "0.1.0")
        with self.assertRaisesRegex(ValueError, "not the release"):
            journal_release.validate_changelog(changelog, manifest, "0.2.0")

    def test_the_section_must_match_the_manifest(self) -> None:
        changelog, manifest = self.make("## 0.2.0 — unreleased", version="0.1.0")
        with self.assertRaisesRegex(ValueError, "declares '0.1.0'"):
            journal_release.validate_changelog(changelog, manifest)

    def test_malformed_headings_are_refused(self) -> None:
        for heading in (
            "## Unreleased",
            "## [0.1.0] - 2026-11-02",
            "## 0.1.0 - unreleased",  # a hyphen, not the em dash
            "## 0.1 — unreleased",
            "## 0.1.0 — 2026-02-30",  # not a real date
            "## 0.1.0 — soon",
        ):
            changelog, manifest = self.make(heading)
            with self.subTest(heading=heading), self.assertRaises(ValueError):
                journal_release.validate_changelog(changelog, manifest)

    def test_exactly_one_section(self) -> None:
        changelog, manifest = self.make("## 0.2.0 — unreleased\n\n## 0.1.0 — 2026-11-02")
        with self.assertRaisesRegex(ValueError, "exactly one"):
            journal_release.validate_changelog(changelog, manifest)
        (changelog / "_meta.md").write_text("# Changelog\n", encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "found 0"):
            journal_release.validate_changelog(changelog, manifest)

    def test_a_missing_entry_point_is_refused(self) -> None:
        changelog, manifest = self.make()
        (changelog / "_meta.md").unlink()
        with self.assertRaisesRegex(ValueError, "no entry point"):
            journal_release.validate_changelog(changelog, manifest)

    def test_shards_must_be_well_named_bullets(self) -> None:
        cases = {
            "1-SHORT-0123abcd.md": "- ok\n",
            "00010-lower-0123abcd.md": "- ok\n",
            "00010-SLUG-0123ABCD.md": "- ok\n",
            "00010-SLUG.md": "- ok\n",
            "notes.txt": "- ok\n",
            "00010-SLUG-0123abcd.md": "### Heading\n",
        }
        for name, text in cases.items():
            with self.subTest(name=name):
                self._temp.cleanup()
                self._temp = tempfile.TemporaryDirectory()
                self.root = Path(self._temp.name)
                changelog, manifest = self.make(shards={name: text})
                with self.assertRaises(ValueError):
                    journal_release.validate_changelog(changelog, manifest)

    def test_a_directory_in_place_of_a_shard_is_refused(self) -> None:
        changelog, manifest = self.make()
        (changelog / "00030-DIR-0123abcd.md").mkdir()
        with self.assertRaises(ValueError):
            journal_release.validate_changelog(changelog, manifest)

    def test_no_entries_is_refused(self) -> None:
        changelog, manifest = self.make(shards={})
        with self.assertRaisesRegex(ValueError, "no entries"):
            journal_release.validate_changelog(changelog, manifest)
        with self.assertRaisesRegex(ValueError, "does not exist"):
            journal_release.changelog_entries(self.root / "absent")

    def test_entries_read_newest_first(self) -> None:
        changelog, _ = self.make()
        entries = journal_release.changelog_entries(changelog)
        self.assertEqual(len(entries), 2)
        self.assertTrue(entries[0].startswith("- **Second.**"))
        self.assertTrue(entries[1].startswith("- **First.**"))

    def test_the_real_changelog_passes_a_dry_run(self) -> None:
        # The same check the validate job makes on every pull request, so a
        # malformed shard fails here first, with this test's name on it.
        version = journal_release.validate_changelog(
            JOURNAL / "CHANGELOG.d", JOURNAL / "mosaic-package.toml"
        )
        self.assertEqual(version, journal_release.manifest_version(JOURNAL / "mosaic-package.toml"))


# --------------------------------------------------------------------------
# Payload names and reports
# --------------------------------------------------------------------------


class ArtifactNamesTests(unittest.TestCase):
    def test_the_complete_payload_set(self) -> None:
        self.assertEqual(
            journal_release.artifact_names("0.1.0"),
            [
                "journal-compose-linux-v0.1.0.zip",
                "journal-qt-linux-v0.1.0.zip",
                "journal-web-v0.1.0.zip",
            ],
        )

    def test_names_refuse_bad_input(self) -> None:
        with self.assertRaises(ValueError):
            journal_release.artifact_names("0.1")
        with self.assertRaisesRegex(ValueError, "unsupported"):
            journal_release.native_artifact_name("0.1.0", "swiftui")

    def test_only_launched_linux_hosts_ship(self) -> None:
        # SwiftUI only compiles in CI; Flutter desktop and XAML have no Journal
        # lane. Adding one here without a launching lane is the mistake this
        # pins against (spec: "Release (J5e)", Payloads).
        self.assertEqual(sorted(journal_release.NATIVE_TARGETS), ["compose", "qt"])
        for backend, target in journal_release.NATIVE_TARGETS.items():
            self.assertEqual(target["label"], f"{backend}-linux")


class NativeReportTests(TempDirTestCase):
    def write(self, report: object) -> Path:
        path = self.root / "mosaic-degradations.json"
        path.write_text(json.dumps(report), encoding="utf-8")
        return path

    def test_a_clean_report_passes(self) -> None:
        journal_release.check_native_report(
            self.write({"nativeComplete": True, "degradations": [], "replacedGeneratedFiles": []})
        )

    def test_any_fallback_is_refused(self) -> None:
        for report in (
            {"nativeComplete": False, "degradations": [], "replacedGeneratedFiles": []},
            {"nativeComplete": True, "degradations": [{"code": "x"}], "replacedGeneratedFiles": []},
            {"nativeComplete": True, "degradations": [], "replacedGeneratedFiles": ["Main.kt"]},
            {"nativeComplete": True, "degradations": []},
            {"nativeComplete": "true", "degradations": [], "replacedGeneratedFiles": []},
            [],
        ):
            with self.subTest(report=report), self.assertRaises(ValueError):
                journal_release.check_native_report(self.write(report))

    def test_unreadable_reports_are_refused(self) -> None:
        with self.assertRaises(ValueError):
            journal_release.check_native_report(self.root / "absent.json")
        bad = self.root / "bad.json"
        bad.write_text("{", encoding="utf-8")
        with self.assertRaises(ValueError):
            journal_release.check_native_report(bad)


# --------------------------------------------------------------------------
# Archives
# --------------------------------------------------------------------------


class ArchiveWebTests(TempDirTestCase):
    def bundle(self) -> tuple[Path, Path]:
        return make_web_bundle(self.root)

    def test_archives_a_complete_bundle(self) -> None:
        dist, built = self.bundle()
        output = journal_release.archive_web("0.1.0", COMMIT, dist, built, self.root / "out")
        self.assertEqual(output.name, "journal-web-v0.1.0.zip")
        with zipfile.ZipFile(output) as archive:
            names = set(archive.namelist())
            self.assertIn("journal-web-v0.1.0/index.html", names)
            self.assertIn("journal-web-v0.1.0/journal_mosaic_app.wasm", names)
            self.assertEqual(archive.read("journal-web-v0.1.0/SOURCE_COMMIT"), f"{COMMIT}\n".encode())
            self.assertTrue(all(name.startswith("journal-web-v0.1.0/") for name in names))

    def test_an_incomplete_bundle_is_refused(self) -> None:
        for missing in ("index.html", journal_release.WASM_RUNTIME):
            with self.subTest(missing=missing):
                dist, built = self.bundle()
                (dist / missing).unlink()
                with self.assertRaisesRegex(ValueError, "incomplete"):
                    journal_release.archive_web("0.1.0", COMMIT, dist, built, self.root / "out")
                self._reset()

    def _reset(self) -> None:
        self._temp.cleanup()
        self._temp = tempfile.TemporaryDirectory()
        self.root = Path(self._temp.name)

    def test_a_bundle_without_assets_is_refused(self) -> None:
        dist, built = self.bundle()
        for path in (dist / "assets").iterdir():
            path.unlink()
        (dist / "assets").rmdir()
        with self.assertRaisesRegex(ValueError, "assets"):
            journal_release.archive_web("0.1.0", COMMIT, dist, built, self.root / "out")

    def test_an_empty_or_stale_runtime_is_refused(self) -> None:
        dist, built = self.bundle()
        (dist / journal_release.WASM_RUNTIME).write_bytes(b"")
        with self.assertRaisesRegex(ValueError, "empty"):
            journal_release.archive_web("0.1.0", COMMIT, dist, built, self.root / "out")
        (dist / journal_release.WASM_RUNTIME).write_bytes(b"\0asm older runtime")
        with self.assertRaisesRegex(ValueError, "not the runtime just built"):
            journal_release.archive_web("0.1.0", COMMIT, dist, built, self.root / "out")
        with self.assertRaisesRegex(ValueError, "not the runtime just built"):
            journal_release.archive_web("0.1.0", COMMIT, dist, self.root / "absent.wasm", self.root / "out")

    def test_a_root_absolute_entry_point_is_refused(self) -> None:
        dist, built = self.bundle()
        (dist / "index.html").write_text('<script src="/assets/index-abc.js"></script>', encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "domain root"):
            journal_release.archive_web("0.1.0", COMMIT, dist, built, self.root / "out")
        self.assertFalse((self.root / "out" / "journal-web-v0.1.0.zip").exists())

    def test_bad_identifiers_are_refused_before_anything_is_written(self) -> None:
        dist, built = self.bundle()
        with self.assertRaises(ValueError):
            journal_release.archive_web("0.1", COMMIT, dist, built, self.root / "out")
        with self.assertRaises(ValueError):
            journal_release.archive_web("0.1.0", "abc", dist, built, self.root / "out")
        self.assertFalse((self.root / "out").exists())


class ArchiveNativeTests(TempDirTestCase):
    def tree(self, backend: str) -> tuple[Path, Path, Path, Path]:
        return make_native_tree(self.root, backend)

    def test_archives_each_shipping_backend(self) -> None:
        for backend in journal_release.NATIVE_TARGETS:
            with self.subTest(backend=backend):
                bundle, executable, runtime, expected = self.tree(backend)
                output = journal_release.archive_native(
                    "0.1.0", COMMIT, backend, bundle, executable, runtime, expected, self.root / "out"
                )
                root = f"journal-{backend}-linux-v0.1.0"
                self.assertEqual(output.name, f"{root}.zip")
                with zipfile.ZipFile(output) as archive:
                    exe = archive.getinfo(f"{root}/bin/{executable.name}")
                    # The executable bit survives, or the app extracts unlaunchable.
                    self.assertTrue((exe.external_attr >> 16) & stat.S_IXUSR)
                    install = archive.read(f"{root}/INSTALL.txt").decode()
                    self.assertIn(f"{root}/bin/{executable.name}", install)
                    self.assertIn("not signed", install)
                    self.assertIn("MOSAIC_APP_STATE_PATH", install)
                    self.assertEqual(archive.read(f"{root}/SOURCE_COMMIT"), f"{COMMIT}\n".encode())

    def test_the_bundled_engine_must_be_the_release_build(self) -> None:
        bundle, executable, runtime, expected = self.tree("qt")
        expected.write_bytes(b"\x7fELF another engine")
        with self.assertRaisesRegex(ValueError, "not the release build"):
            journal_release.archive_native(
                "0.1.0", COMMIT, "qt", bundle, executable, runtime, expected, self.root / "out"
            )
        with self.assertRaisesRegex(ValueError, "must both exist"):
            journal_release.archive_native(
                "0.1.0", COMMIT, "qt", bundle, executable, runtime, self.root / "absent", self.root / "out"
            )

    def test_identity_is_checked(self) -> None:
        bundle, executable, runtime, expected = self.tree("qt")
        other = bundle / "bin" / "TaskApp"
        other.write_bytes(b"#!/bin/sh\n")
        other.chmod(0o755)
        with self.assertRaisesRegex(ValueError, "must be named 'JournalApp'"):
            journal_release.archive_native(
                "0.1.0", COMMIT, "qt", bundle, other, runtime, expected, self.root / "out"
            )
        engine = bundle / "bin" / "libtask_mosaic_app.so"
        engine.write_bytes(b"\x7fELF engine")
        with self.assertRaisesRegex(ValueError, "must be named 'libmosaic_app.so'"):
            journal_release.archive_native(
                "0.1.0", COMMIT, "qt", bundle, executable, engine, expected, self.root / "out"
            )
        with self.assertRaisesRegex(ValueError, "unsupported"):
            journal_release.archive_native(
                "0.1.0", COMMIT, "xaml", bundle, executable, runtime, expected, self.root / "out"
            )

    def test_a_non_executable_app_is_refused(self) -> None:
        bundle, executable, runtime, expected = self.tree("compose")
        executable.chmod(0o644)
        with self.assertRaisesRegex(ValueError, "not an executable"):
            journal_release.archive_native(
                "0.1.0", COMMIT, "compose", bundle, executable, runtime, expected, self.root / "out"
            )

    def test_paths_outside_the_bundle_are_refused(self) -> None:
        bundle, executable, runtime, expected = self.tree("qt")
        outside = self.root / "JournalApp"
        outside.write_bytes(b"#!/bin/sh\n")
        outside.chmod(0o755)
        with self.assertRaisesRegex(ValueError, "inside the bundle"):
            journal_release.archive_native(
                "0.1.0", COMMIT, "qt", bundle, outside, runtime, expected, self.root / "out"
            )
        with self.assertRaisesRegex(ValueError, "inside the bundle"):
            journal_release.archive_native(
                "0.1.0", COMMIT, "qt", bundle, executable, expected, expected, self.root / "out"
            )
        with self.assertRaisesRegex(ValueError, "does not exist"):
            journal_release.archive_native(
                "0.1.0", COMMIT, "qt", bundle, bundle / "bin" / "absent", runtime, expected, self.root / "out"
            )
        with self.assertRaisesRegex(ValueError, "does not exist"):
            journal_release.archive_native(
                "0.1.0", COMMIT, "qt", self.root / "absent", executable, runtime, expected, self.root / "out"
            )

    @unittest.skipIf(os.name == "nt", "symlinks need privileges on Windows")
    def test_a_symlink_escaping_the_bundle_is_refused(self) -> None:
        # The shared archiver's guarantee, checked through this helper so a
        # change on either side of the import is caught here.
        bundle, executable, runtime, expected = self.tree("qt")
        (bundle / "leak").symlink_to(self.root)
        with self.assertRaises(ValueError):
            journal_release.archive_native(
                "0.1.0", COMMIT, "qt", bundle, executable, runtime, expected, self.root / "out"
            )
        self.assertFalse((self.root / "out" / "journal-qt-linux-v0.1.0.zip").exists())

    def test_an_existing_install_file_is_not_overwritten(self) -> None:
        bundle, executable, runtime, expected = self.tree("qt")
        (bundle / "INSTALL.txt").write_text("theirs", encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "already has an INSTALL.txt"):
            journal_release.archive_native(
                "0.1.0", COMMIT, "qt", bundle, executable, runtime, expected, self.root / "out"
            )
        self.assertEqual((bundle / "INSTALL.txt").read_text(encoding="utf-8"), "theirs")


class ChecksumTests(TempDirTestCase):
    def assets(self) -> Path:
        assets = self.root / "release-assets"
        assets.mkdir()
        for name in journal_release.artifact_names("0.1.0"):
            (assets / name).write_bytes(name.encode())
        return assets

    def test_writes_sha256sum_lines_for_exactly_the_declared_set(self) -> None:
        assets = self.assets()
        output = journal_release.write_checksums("0.1.0", assets)
        lines = output.read_text(encoding="utf-8").splitlines()
        self.assertEqual([line.split("  ")[1] for line in lines], journal_release.artifact_names("0.1.0"))
        for line in lines:
            self.assertRegex(line, r"^[0-9a-f]{64}  journal-[a-z-]+-v0\.1\.0\.zip$")

    def test_a_missing_or_extra_payload_is_refused(self) -> None:
        assets = self.assets()
        (assets / "journal-qt-linux-v0.1.0.zip").unlink()
        with self.assertRaisesRegex(ValueError, "missing \\['journal-qt-linux-v0.1.0.zip'\\]"):
            journal_release.write_checksums("0.1.0", assets)
        (assets / "journal-qt-linux-v0.1.0.zip").write_bytes(b"x")
        (assets / "journal-swiftui-macos-v0.1.0.zip").write_bytes(b"x")
        with self.assertRaisesRegex(ValueError, "unexpected"):
            journal_release.write_checksums("0.1.0", assets)
        with self.assertRaisesRegex(ValueError, "does not exist"):
            journal_release.write_checksums("0.1.0", self.root / "absent")

    def test_a_payload_that_is_not_a_regular_file_is_refused(self) -> None:
        assets = self.assets()
        (assets / "journal-web-v0.1.0.zip").unlink()
        (assets / "journal-web-v0.1.0.zip").mkdir()
        with self.assertRaisesRegex(ValueError, "not a regular file"):
            journal_release.write_checksums("0.1.0", assets)


class NotesTests(TempDirTestCase):
    def test_notes_carry_the_dated_entries_newest_first_and_the_payloads(self) -> None:
        changelog, manifest = make_changelog(self.root, "## 0.1.0 \u2014 2026-11-02")
        notes = journal_release.render_notes("0.1.0", COMMIT.upper(), changelog, manifest)
        self.assertLess(notes.index("**Second.**"), notes.index("**First.**"))
        for name in journal_release.artifact_names("0.1.0"):
            self.assertIn(f"`{name}`", notes)
        self.assertIn("SHA256SUMS", notes)
        self.assertIn(f"Built from {COMMIT}.", notes)
        self.assertNotIn("unreleased", notes)

    def test_notes_refuse_an_unreleased_changelog(self) -> None:
        changelog, manifest = make_changelog(self.root)
        with self.assertRaises(ValueError):
            journal_release.render_notes("0.1.0", COMMIT, changelog, manifest)


class CommandLineTests(TempDirTestCase):
    def run_main(self, *argv: str) -> tuple[int, str, str]:
        out, err = io.StringIO(), io.StringIO()
        with redirect_stdout(out), redirect_stderr(err):
            status = journal_release.main(list(argv))
        return status, out.getvalue(), err.getvalue()

    def test_validate_prints_the_identity(self) -> None:
        status, out, _ = self.run_main(
            "validate", "--version", "0.1.0", "--tag", "journal-v0.1.0", "--commit", COMMIT
        )
        self.assertEqual(status, 0)
        self.assertIn("tag=journal-v0.1.0", out)

    def test_a_refusal_is_one_line_and_exit_one(self) -> None:
        status, _, err = self.run_main("validate", "--version", "0.1", "--tag", "journal-v0.1")
        self.assertEqual(status, 1)
        self.assertTrue(err.startswith("journal_release: version is not strict SemVer"))

    def test_artifact_names_and_changelog_commands(self) -> None:
        status, out, _ = self.run_main("artifact-names", "--version", "0.1.0")
        self.assertEqual((status, out.split()), (0, journal_release.artifact_names("0.1.0")))
        status, out, _ = self.run_main(
            "validate-changelog",
            "--changelog-dir", str(JOURNAL / "CHANGELOG.d"),
            "--manifest", str(JOURNAL / "mosaic-package.toml"),
        )
        self.assertEqual(status, 0, out)

    def test_every_file_command_round_trips(self) -> None:
        report = self.root / "report.json"
        report.write_text('{"nativeComplete": true, "degradations": [], "replacedGeneratedFiles": []}')
        self.assertEqual(self.run_main("check-native-report", "--report", str(report))[0], 0)

        dist, built = make_web_bundle(self.root)
        out_dir = self.root / "assets"
        self.assertEqual(self.run_main(
            "archive-web", "--version", "0.1.0", "--commit", COMMIT, "--source", str(dist),
            "--expected-wasm", str(built), "--output-dir", str(out_dir),
        )[0], 0)

        for backend in journal_release.NATIVE_TARGETS:
            bundle, executable, runtime, expected = make_native_tree(self.root, backend)
            self.assertEqual(self.run_main(
                "archive-native", "--version", "0.1.0", "--commit", COMMIT, "--backend", backend,
                "--source", str(bundle), "--executable", str(executable), "--runtime", str(runtime),
                "--expected-runtime", str(expected), "--output-dir", str(out_dir),
            )[0], 0)
        self.assertEqual(
            self.run_main("write-checksums", "--version", "0.1.0", "--assets-dir", str(out_dir))[0], 0
        )

        changelog, manifest = make_changelog(self.root, "## 0.1.0 — 2026-11-02")
        notes = self.root / "notes.md"
        self.assertEqual(self.run_main(
            "write-notes", "--version", "0.1.0", "--commit", COMMIT, "--changelog-dir", str(changelog),
            "--manifest", str(manifest), "--output", str(notes),
        )[0], 0)
        self.assertIn("Journal 0.1.0", notes.read_text(encoding="utf-8"))


# --------------------------------------------------------------------------
# The workflow
# --------------------------------------------------------------------------

USES = re.compile(r"^\s*(?:-\s+)?uses:\s*(?P<ref>[^\s#]+)(?P<rest>.*)$")
RUN = re.compile(r"^(?P<indent>\s*)(?P<dash>-\s+)?run:\s*(?P<rest>.*)$")
CHECKOUT = re.compile(r"^(?P<indent>\s*)(?P<dash>-\s+)?uses:\s*[\"']?actions/checkout@", re.IGNORECASE)


def _indent(line: str) -> int:
    return len(line) - len(line.lstrip())


def run_blocks(text: str) -> list[tuple[int, str]]:
    """(line number, script) for every `run:` in a workflow.

    A block scalar (`run: |`) owns every following line that is blank or
    indented deeper than the `run:` key itself.
    """

    lines = text.splitlines()
    blocks = []
    for number, line in enumerate(lines):
        match = RUN.match(line)
        if not match:
            continue
        rest = match["rest"].strip()
        if not rest.startswith(("|", ">")):
            blocks.append((number + 1, rest))
            continue
        key_indent = len(match["indent"]) + len(match["dash"] or "")
        body = []
        for following in lines[number + 1:]:
            if following.strip() and _indent(following) <= key_indent:
                break
            body.append(following)
        blocks.append((number + 1, "\n".join(body)))
    return blocks


def step_block(lines: list[str], number: int, match: re.Match[str]) -> str:
    key_indent = len(match["indent"]) + len(match["dash"] or "")
    block = [lines[number]]
    for following in lines[number + 1:]:
        stripped = following.strip()
        if stripped and (
            _indent(following) < key_indent
            or (_indent(following) == key_indent - 2 and stripped.startswith("- "))
        ):
            break
        block.append(following)
    return "\n".join(block)


def jobs(text: str) -> dict[str, str]:
    """Each top-level job's text, keyed by its id."""

    body = text.split("\njobs:\n", 1)[1]
    found: dict[str, str] = {}
    current = None
    for line in body.splitlines():
        match = re.match(r"^  ([A-Za-z0-9_-]+):\s*$", line)
        if match:
            current = match[1]
            found[current] = ""
        elif current is not None:
            found[current] += line + "\n"
    return found


class WorkflowTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.text = WORKFLOW.read_text(encoding="utf-8")
        cls.lines = cls.text.splitlines()
        cls.code = "\n".join(line for line in cls.lines if not line.lstrip().startswith("#"))
        cls.jobs = jobs(cls.text)

    def test_triggers_are_dispatch_and_pull_request_only(self) -> None:
        on = self.code.split("\non:\n", 1)[1].split("\npermissions:", 1)[0]
        self.assertEqual(re.findall(r"^  ([a-z_]+):", on, re.MULTILINE), ["pull_request", "workflow_dispatch"])
        # The retired Electron lane published on any `journal-v*` tag push.
        self.assertNotIn("tags:", on)
        self.assertIn("version:", on)
        self.assertIn("tag:", on)

    def test_the_legacy_electron_app_is_not_built(self) -> None:
        self.assertNotIn("programs/typescript", self.code)
        self.assertNotIn("electron", self.code.lower())

    def test_the_pull_request_run_covers_the_lane_and_what_it_imports(self) -> None:
        on = self.code.split("\non:\n", 1)[1].split("workflow_dispatch:", 1)[0]
        for path in (
            ".github/workflows/release-journal.yml",
            "code/scripts/journal_release.py",
            "code/scripts/tests/test_journal_release.py",
            "code/scripts/engram_release.py",
            "code/scripts/verify_relocatable_bundle.py",
            "code/programs/mosaic/journal-app/**",
            "code/packages/rust/journal-mosaic-app/**",
        ):
            self.assertIn(f"- {path}\n", on, path)
        # A standalone lane listing ci.yml would run on every CI edit.
        self.assertNotIn("ci.yml", on)

    def test_default_permissions_are_read_only(self) -> None:
        top = self.code.split("\njobs:\n", 1)[0]
        self.assertIn("\npermissions:\n  contents: read\n", top)
        self.assertEqual(top.count("permissions:"), 1)

    def test_only_the_dispatch_only_publish_job_can_write(self) -> None:
        self.assertEqual(sorted(self.jobs), ["build-native", "build-web", "publish", "validate"])
        for name, body in self.jobs.items():
            if name == "publish":
                continue
            self.assertNotIn("permissions:", body, name)
        publish = self.jobs["publish"]
        self.assertIn("    if: github.event_name == 'workflow_dispatch'\n", publish)
        self.assertIn("    permissions:\n      contents: write\n    steps:", publish)
        for scope in ("id-token", "attestations", "packages:", "pages:", "environment:"):
            self.assertNotIn(scope, self.code, scope)
        self.assertIn("needs: [validate, build-web, build-native]", publish)

    def test_every_action_is_pinned_to_a_commented_sha(self) -> None:
        uses = [USES.match(line) for line in self.lines]
        uses = [match for match in uses if match]
        self.assertGreater(len(uses), 10)
        for match in uses:
            with self.subTest(ref=match["ref"]):
                self.assertRegex(match["ref"], r"^[A-Za-z0-9_.-]+(/[A-Za-z0-9_.-]+)+@[0-9a-f]{40}$")
                self.assertRegex(match["rest"], r"^\s+#\s*\S+")

    def test_pins_reuse_the_shas_already_in_the_repository(self) -> None:
        # A second SHA for the same action is a second thing to review and
        # bump. Each pin here must already be used by another workflow,
        # except setup-java, which no workflow pinned before this one.
        others = "\n".join(
            path.read_text(encoding="utf-8")
            for path in sorted(WORKFLOW.parent.glob("*.yml"))
            if path != WORKFLOW
        )
        for match in filter(None, (USES.match(line) for line in self.lines)):
            if match["ref"].startswith("actions/setup-java@"):
                continue
            with self.subTest(ref=match["ref"]):
                self.assertIn(match["ref"], others)

    def test_every_checkout_drops_its_token(self) -> None:
        checkouts = 0
        for number, line in enumerate(self.lines):
            match = CHECKOUT.match(line)
            if match:
                checkouts += 1
                self.assertRegex(
                    step_block(self.lines, number, match),
                    r"(?m)^\s+persist-credentials:\s*false\s*$",
                    f"checkout on line {number + 1}",
                )
        self.assertEqual(checkouts, 4)

    def test_no_expression_is_interpolated_into_a_shell_script(self) -> None:
        blocks = run_blocks(self.text)
        self.assertGreater(len(blocks), 10)
        for number, script in blocks:
            with self.subTest(line=number):
                self.assertNotIn("${{", script)

    def test_no_shell_gate_is_a_bare_negated_grep(self) -> None:
        # `set -e` ignores a command inverted with `!`, so `! grep` gates
        # nothing (lessons.d: a-bare-grep-never-fails-a-set-e-bash-step).
        for number, script in run_blocks(self.text):
            for line in script.splitlines():
                with self.subTest(line=number):
                    self.assertFalse(line.strip().startswith("! "), line)

    def test_every_script_fails_fast(self) -> None:
        for number, script in run_blocks(self.text):
            if "\n" in script.strip():
                with self.subTest(line=number):
                    self.assertTrue(script.strip().startswith("set -euo pipefail"), script[:80])

    def test_the_validate_job_runs_this_test(self) -> None:
        self.assertIn("-p 'test_journal_release.py'", self.jobs["validate"])

    def test_launches_use_the_xvfb_helper_and_a_scratch_state_file(self) -> None:
        self.assertIsNone(re.search(r"(?<![-\w])xvfb-run(?![-\w.])", self.code))
        self.assertIn('bash "$GITHUB_WORKSPACE/code/scripts/run-under-xvfb.sh"', self.code)
        launch = self.jobs["build-native"].split("- name: Launch the extracted payload", 1)[1]
        self.assertEqual(launch.count('MOSAIC_APP_STATE_PATH="$state"'), 2)
        self.assertIn('state="$state_dir/mosaic-state.v1.json"', launch)
        self.assertEqual(launch.count("env -u MOSAIC_APP_LIBRARY"), 2)

    def test_the_native_matrix_is_the_shipping_set(self) -> None:
        backends = re.findall(r"^\s+- backend: (\S+)$", self.jobs["build-native"], re.MULTILINE)
        self.assertEqual(sorted(backends), sorted(journal_release.NATIVE_TARGETS))
        # The launch step rebuilds the archive name from the backend.
        self.assertIn('label="$BACKEND-linux"', self.jobs["build-native"])

    def test_toolchains_match_the_ci_lanes_that_prove_journal(self) -> None:
        ci = CI.read_text(encoding="utf-8")
        for pin in ("6.8.3", "temurin", "java-version: '21'"):
            self.assertIn(pin, ci, pin)
        self.assertIn('version: "6.8.3"', self.code)
        self.assertIn('java-version: "21"', self.code)

    def test_publication_rechecks_the_tag_and_targets_the_validated_commit(self) -> None:
        publish = self.jobs["publish"]
        self.assertIn("git ls-remote --exit-code --tags origin", publish)
        # The tag is created atomically at the validated commit, read back,
        # and only then released with --verify-tag. `--target` is gone: for a
        # tag that appeared in the meantime it would be silently ignored.
        self.assertIn('-f ref="refs/tags/$RELEASE_TAG" -f sha="$RELEASE_COMMIT"', publish)
        self.assertIn('[[ "$tagged_commit" != "$RELEASE_COMMIT" ]]', publish)
        self.assertIn("--verify-tag", publish)
        self.assertNotIn("--target", publish.rsplit("gh release create", 1)[1])
        self.assertIn("--latest=false", publish)
        self.assertIn("write-checksums", publish)
        self.assertIn("sha256sum --check SHA256SUMS", publish)
        self.assertIn("pattern: journal-*-release", publish)

    def test_concurrency_serializes_publication_only(self) -> None:
        self.assertIn("format('release-journal-pr-{0}'", self.code)
        self.assertIn("'release-journal-publish'", self.code)
        self.assertIn("cancel-in-progress: false", self.code)


if __name__ == "__main__":
    unittest.main()
