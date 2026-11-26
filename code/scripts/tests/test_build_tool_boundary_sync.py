"""Tests for ``build_tool_conformance.py sync-boundary``.

Each test builds a miniature repository in a temporary directory: copies of the
handful of real files the generator reads or rewrites (the boundary fixture,
its schema and registry, the two projections, the digest pins and the case
fixtures), and nothing else.  Discovery is replaced with a function returning
chosen roots, so a test can say "a crate appeared" without creating one.  The
discovery function itself is tested separately against a tiny real tree, and
against the whole repository by the corpus test.
"""

from __future__ import annotations

import io
import json
import os
import re
import shutil
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS_DIR))

import build_tool_boundary_sync as sync
import build_tool_conformance as runner

REAL = runner.REPO_ROOT
RUST_BOUNDARY = "starlark-rust-library-rule-consumers"
TS_BOUNDARY = "typescript-workspace-configuration"


def copy_working_set(destination: Path) -> None:
    """Copy exactly the files ``sync-boundary`` reads or rewrites."""

    paths = [
        sync.FIXTURE_PATH,
        sync.SCHEMA_PATH,
        sync.REGISTRY_PATH,
        sync.PYTHON_TEST_PATH,
        *(projection.path for projection in sync.PROJECTIONS),
        *(path for path, _pattern in sync.DIGEST_PINS),
    ]
    paths += [
        path.relative_to(REAL)
        for path in (REAL / sync.CASES_DIR).glob("*.json")
        if "boundary_sha256" in path.read_text(encoding="utf-8")
    ]
    for relative in set(paths):
        target = destination / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(REAL / relative, target)


def load_boundary(root: Path) -> dict:
    return json.loads((root / sync.FIXTURE_PATH).read_text(encoding="utf-8"))


def roots_of(root: Path, boundary_id: str) -> list[str]:
    document = load_boundary(root)
    return next(b for b in document["boundaries"] if b["id"] == boundary_id)[
        "applies_to"
    ]["exact_roots"]


def snapshot(root: Path) -> dict[Path, bytes | None]:
    result: dict[Path, bytes | None] = {}
    for path in sorted(root.rglob("*")):
        try:
            if path.is_file():
                result[path] = path.read_bytes()
        except PermissionError:  # an unreadable file is part of some scenarios
            result[path] = None
    return result


class SyncBoundaryTests(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.root = Path(self._tmp.name)
        copy_working_set(self.root)
        # A missing scan root is an error, so the miniature repo has them all.
        for scan_root in sync.SCAN_ROOTS:
            (self.root / scan_root).mkdir(parents=True, exist_ok=True)
        # What discovery "sees": by default exactly what the fixture says.
        self.discovered = {
            RUST_BOUNDARY: roots_of(self.root, RUST_BOUNDARY),
            TS_BOUNDARY: roots_of(self.root, TS_BOUNDARY),
        }

    def invoke(self, *arguments: str) -> tuple[int, str, str]:
        out, err = io.StringIO(), io.StringIO()
        with redirect_stdout(out), redirect_stderr(err):
            code = sync.run(
                list(arguments),
                self.root,
                discover=lambda _root: {k: list(v) for k, v in self.discovered.items()},
            )
        return code, out.getvalue(), err.getvalue()

    def pinned_digests(self) -> set[str]:
        found = set()
        for path in snapshot(self.root):
            found.update(
                re.findall(
                    r'"boundary_sha256": "([0-9a-f]{64})"',
                    path.read_text(encoding="utf-8") if path.suffix == ".json" else "",
                )
            )
        return found

    def test_in_sync_tree_passes_check_and_changes_nothing(self) -> None:
        before = snapshot(self.root)
        code, out, _err = self.invoke("--check")
        self.assertEqual(code, 0)
        self.assertIn("in sync", out)
        self.assertEqual(snapshot(self.root), before)

    def test_missing_root_is_added_everywhere(self) -> None:
        new_root = "code/packages/rust/aaa-new-crate"
        self.discovered[RUST_BOUNDARY] = sorted(
            [*self.discovered[RUST_BOUNDARY], new_root],
            key=lambda path: path.encode("utf-8"),
        )
        old_summary = runner._validate_repository_source_input_boundary(
            load_boundary(self.root),
            runner.load_document(self.root / sync.SCHEMA_PATH),
            runner.load_document(self.root / sync.REGISTRY_PATH),
        )
        code, _out, _err = self.invoke()
        self.assertEqual(code, 0)

        self.assertIn(new_root, roots_of(self.root, RUST_BOUNDARY))
        new_digest = runner.repository_source_input_boundary_digest(
            load_boundary(self.root)
        )
        self.assertEqual(self.pinned_digests(), {new_digest})
        for projection in sync.PROJECTIONS:
            self.assertIn(
                f'"{new_root}"', (self.root / projection.path).read_text("utf-8")
            )
        for path, _pattern in sync.DIGEST_PINS:
            self.assertIn(new_digest, (self.root / path).read_text("utf-8"), path)
        test_text = (self.root / sync.PYTHON_TEST_PATH).read_text("utf-8")
        self.assertIn(f'"scope_count": {old_summary["scope_count"] + 1},', test_text)
        self.assertIn(
            f'"authorization_count": {old_summary["authorization_count"] + 1},',
            test_text,
        )
        # The projection is the fixture, byte for byte.
        csharp = (self.root / sync.PROJECTIONS[0].path).read_text("utf-8")
        self.assertIn(
            (self.root / sync.FIXTURE_PATH).read_text("utf-8").rstrip("\n"), csharp
        )

    def test_removed_root_is_dropped(self) -> None:
        dropped = self.discovered[TS_BOUNDARY].pop()
        old_summary = runner._validate_repository_source_input_boundary(
            load_boundary(self.root),
            runner.load_document(self.root / sync.SCHEMA_PATH),
            runner.load_document(self.root / sync.REGISTRY_PATH),
        )
        code, _out, _err = self.invoke()
        self.assertEqual(code, 0)
        self.assertNotIn(dropped, roots_of(self.root, TS_BOUNDARY))
        for projection in sync.PROJECTIONS:
            self.assertNotIn(
                f'"{dropped}"', (self.root / projection.path).read_text("utf-8")
            )
        self.assertIn(
            f'"scope_count": {old_summary["scope_count"] - 1},',
            (self.root / sync.PYTHON_TEST_PATH).read_text("utf-8"),
        )

    def test_second_run_is_idempotent(self) -> None:
        self.discovered[RUST_BOUNDARY].append("code/packages/rust/zzz-new-crate")
        self.assertEqual(self.invoke()[0], 0)
        after_first = snapshot(self.root)
        code, out, _err = self.invoke()
        self.assertEqual(code, 0)
        self.assertIn("nothing to do", out)
        self.assertEqual(snapshot(self.root), after_first)

    def test_check_fails_when_out_of_sync_and_names_the_fix(self) -> None:
        self.discovered[RUST_BOUNDARY].append("code/packages/rust/zzz-new-crate")
        before = snapshot(self.root)
        code, _out, err = self.invoke("--check")
        self.assertEqual(code, 1)
        self.assertIn(sync.SYNC_COMMAND, err)
        self.assertIn("+ " + RUST_BOUNDARY + ": code/packages/rust/zzz-new-crate", err)
        self.assertIn("boundary digest:", err)
        self.assertEqual(snapshot(self.root), before, "--check must not write")
        # ... and passes again once synchronised.
        self.assertEqual(self.invoke()[0], 0)
        self.assertEqual(self.invoke("--check")[0], 0)

    def assert_refuses_without_writing(self, expected_fragment: str) -> None:
        before = snapshot(self.root)
        code, _out, err = self.invoke()
        self.assertEqual(code, 2, err)
        self.assertIn(expected_fragment, err)
        self.assertEqual(snapshot(self.root), before)

    def test_unrecognised_digest_pin_format_is_an_error(self) -> None:
        self.discovered[RUST_BOUNDARY].append("code/packages/rust/zzz-new-crate")
        go_path = self.root / sync.DIGEST_PINS[0][0]
        text = go_path.read_text("utf-8").replace("digest != ", "digest == ")
        go_path.write_text(text, encoding="utf-8")
        self.assert_refuses_without_writing("pin format is not recognised")

    def test_missing_pin_file_is_an_error(self) -> None:
        (self.root / sync.DIGEST_PINS[1][0]).unlink()
        self.assert_refuses_without_writing("cannot read")

    def test_unrecognised_projection_format_is_an_error(self) -> None:
        path = self.root / sync.PROJECTIONS[1].path
        path.write_text(
            path.read_text("utf-8").replace("embeddedJSON", "somethingElse"),
            encoding="utf-8",
        )
        self.assert_refuses_without_writing("projection format is not recognised")

    def test_unrecognised_count_pin_format_is_an_error(self) -> None:
        path = self.root / sync.PYTHON_TEST_PATH
        path.write_text(
            path.read_text("utf-8").replace('"scope_count": ', '"scopes": ', 1),
            encoding="utf-8",
        )
        self.assert_refuses_without_writing("file format is not recognised")

    def test_malformed_case_pin_is_an_error(self) -> None:
        case = next((self.root / sync.CASES_DIR).glob("*.json"))
        case.write_text(
            case.read_text("utf-8").replace(
                '"boundary_sha256": "', '"boundary_sha256": "not-hex-', 1
            ),
            encoding="utf-8",
        )
        self.assert_refuses_without_writing("recognised format")

    def test_non_canonical_fixture_is_an_error(self) -> None:
        path = self.root / sync.FIXTURE_PATH
        path.write_text(path.read_text("utf-8").replace("  ", "    ", 1))
        self.assert_refuses_without_writing("not in canonical form")

    def test_emptied_boundary_is_an_error_not_a_silent_write(self) -> None:
        self.discovered[RUST_BOUNDARY] = []
        self.assert_refuses_without_writing("synchronised boundary is invalid")

    def test_digest_pinned_in_unknown_file_is_an_error(self) -> None:
        self.discovered[RUST_BOUNDARY].append("code/packages/rust/zzz-new-crate")
        digest = runner.repository_source_input_boundary_digest(
            load_boundary(self.root)
        )
        stray = self.root / "code/programs/ocaml/build-tool/test/pin.ml"
        stray.parent.mkdir(parents=True)
        stray.write_text(f'let d = "{digest}"\n', encoding="utf-8")
        self.assert_refuses_without_writing("pinned in files this tool does not")

    # ---- fail-closed stale-digest safety net ------------------------------

    def add_root(self) -> None:
        self.discovered[RUST_BOUNDARY].append("code/packages/rust/zzz-new-crate")

    def test_second_stale_copy_in_a_registered_file_is_an_error(self) -> None:
        self.add_root()
        old = runner.repository_source_input_boundary_digest(load_boundary(self.root))
        go_path = self.root / sync.DIGEST_PINS[0][0]
        go_path.write_text(
            go_path.read_text("utf-8") + f'\n// stale copy: "{old}"\n',
            encoding="utf-8",
        )
        self.assert_refuses_without_writing("still contains the stale boundary digest")

    def test_second_stale_copy_under_another_case_key_is_an_error(self) -> None:
        self.add_root()
        old = runner.repository_source_input_boundary_digest(load_boundary(self.root))
        case = next((self.root / sync.CASES_DIR).glob("*.json"))
        case.write_text(
            case.read_text("utf-8").replace(
                '"boundary_sha256"', f'"other_digest": "{old}", "boundary_sha256"', 1
            ),
            encoding="utf-8",
        )
        self.assert_refuses_without_writing("still contains the stale boundary digest")

    def test_scan_has_no_suffix_allowlist_and_skips_binary_and_narrative(self) -> None:
        self.add_root()
        old = runner.repository_source_input_boundary_digest(load_boundary(self.root))
        programs = self.root / "code/programs/zz"
        programs.mkdir(parents=True)
        (programs / "pin.unheardof").write_text(old, encoding="utf-8")
        self.assert_refuses_without_writing("pin.unheardof")
        (programs / "pin.unheardof").unlink()

        # A binary file that happens to contain the bytes is not a pin site.
        (programs / "blob.bin").write_bytes(b"\0" + old.encode("ascii"))
        # The dated narrative copy is deliberately preserved.
        narrative = self.root / next(iter(sync.NARRATIVE_EXCLUSIONS))
        narrative.parent.mkdir(parents=True, exist_ok=True)
        narrative.write_text(f"validated {old}\n", encoding="utf-8")
        self.assertEqual(self.invoke()[0], 0)
        self.assertIn(old, narrative.read_text("utf-8"))
        # .github is scanned too.
        self.assertIn(".github", sync.SCAN_ROOTS)

    @unittest.skipIf(hasattr(os, "geteuid") and os.geteuid() == 0, "root ignores modes")
    def test_unreadable_file_during_scan_is_an_error(self) -> None:
        self.add_root()
        stray = self.root / "code/programs/zz/secret.txt"
        stray.parent.mkdir(parents=True)
        stray.write_text("x", encoding="utf-8")
        stray.chmod(0)
        self.addCleanup(stray.chmod, 0o644)
        self.assert_refuses_without_writing("cannot be read while scanning")

    @unittest.skipIf(hasattr(os, "geteuid") and os.geteuid() == 0, "root ignores modes")
    def test_unreadable_directory_during_scan_is_an_error(self) -> None:
        self.add_root()
        directory = self.root / "code/programs/zz"
        directory.mkdir(parents=True)
        directory.chmod(0)
        self.addCleanup(directory.chmod, 0o755)
        self.assert_refuses_without_writing("cannot scan a directory")

    def test_symlinked_target_is_refused_and_not_written_through(self) -> None:
        self.add_root()
        victim = self.root.parent / f"{self.root.name}-victim.txt"
        victim.write_text("precious", encoding="utf-8")
        self.addCleanup(victim.unlink)
        pin = self.root / sync.DIGEST_PINS[1][0]
        pin.unlink()
        pin.symlink_to(victim)
        before = snapshot(self.root)
        code, _out, err = self.invoke()
        self.assertEqual(code, 2)
        self.assertIn("symlink", err)
        self.assertEqual(victim.read_text("utf-8"), "precious")
        self.assertTrue(pin.is_symlink())
        self.assertEqual(snapshot(self.root), before)

    def test_target_resolving_outside_the_repository_is_refused(self) -> None:
        outside = self.root.parent / f"{self.root.name}-outside"
        outside.mkdir()
        self.addCleanup(shutil.rmtree, outside)
        (outside / "Hasher.swift").write_text("x", encoding="utf-8")
        parent = self.root / sync.DIGEST_PINS[3][0].parent
        shutil.rmtree(parent)
        parent.parent.mkdir(parents=True, exist_ok=True)
        parent.symlink_to(outside)
        code, _out, err = self.invoke()
        self.assertEqual(code, 2)
        self.assertIn("outside the repository", err)

    def test_write_is_atomic_and_leaves_no_temporary_files(self) -> None:
        self.add_root()
        self.assertEqual(self.invoke()[0], 0)
        leftovers = [p for p in self.root.rglob(".*") if p.is_file()]
        self.assertEqual(leftovers, [])
        mode = (self.root / sync.FIXTURE_PATH).stat().st_mode & 0o777
        self.assertEqual(mode, (REAL / sync.FIXTURE_PATH).stat().st_mode & 0o777)

    def test_apply_replaces_a_swapped_in_symlink_instead_of_following_it(self) -> None:
        self.add_root()
        plan = sync.plan(self.root, discover=lambda _r: dict(self.discovered))
        victim = self.root.parent / f"{self.root.name}-victim2.txt"
        victim.write_text("precious", encoding="utf-8")
        self.addCleanup(victim.unlink)
        target = self.root / sync.FIXTURE_PATH
        target.unlink()
        target.symlink_to(victim)
        with self.assertRaises(sync.SyncError):
            sync.apply(plan)
        self.assertEqual(victim.read_text("utf-8"), "precious")


class RealCorpusTests(unittest.TestCase):
    def test_every_case_fixture_pins_the_current_digest(self) -> None:
        """No case carries a deliberately wrong digest.

        ``sync-boundary`` rewrites every ``boundary_sha256`` in the cases.  If a
        negative case ever needs a wrong digest, exclude it from ``CASE_PIN``
        explicitly and change this test; making that decision visible is the
        point.
        """

        digest = runner.repository_source_input_boundary_digest(
            runner.load_document(REAL / sync.FIXTURE_PATH)
        )
        seen = 0
        for path in sorted((REAL / sync.CASES_DIR).glob("*.json")):
            text = path.read_text(encoding="utf-8")
            for value in re.findall(r'"boundary_sha256": "([^"]*)"', text):
                seen += 1
                self.assertEqual(value, digest, path.name)
        self.assertGreater(seen, 0)


class DiscoveryTests(unittest.TestCase):
    """``expected_repository_source_exact_roots`` on a tiny, real directory tree."""

    def test_finds_rust_starlark_consumer_and_typescript_workspace_root(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            (root / "code/programs").mkdir(parents=True)
            (root / "code/packages/python").mkdir(parents=True)
            (root / "code/packages/python/pyproject.toml").write_text(
                "[tool.uv.workspace]\nmembers = []\n[tool.uv.sources]\n",
                encoding="utf-8",
            )
            crate = root / "code/packages/rust/zz-demo"
            crate.mkdir(parents=True)
            (crate / "BUILD").write_text(
                'load("code/packages/starlark/library-rules/rust_library.star", "x")\n',
                encoding="utf-8",
            )
            ts = root / "code/packages/typescript/zz-demo"
            ts.mkdir(parents=True)
            (ts / "BUILD").write_text("npm install\n", encoding="utf-8")
            (ts / "tsconfig.json").write_text(
                '{"extends": "../tsconfig.base.json"}', encoding="utf-8"
            )
            # A TypeScript package that does not extend the base is not a root.
            other = root / "code/packages/typescript/zz-other"
            other.mkdir()
            (other / "BUILD").write_text("npm install\n", encoding="utf-8")
            (other / "tsconfig.json").write_text("{}", encoding="utf-8")

            roots = runner.expected_repository_source_exact_roots(root)

        self.assertEqual(roots[RUST_BOUNDARY], ["code/packages/rust/zz-demo"])
        self.assertEqual(roots[TS_BOUNDARY], ["code/packages/typescript/zz-demo"])
        self.assertEqual(roots["starlark-go-library-rule-consumers"], [])

    def make_tree(self, root: Path, members: str = "") -> Path:
        (root / "code/packages/python").mkdir(parents=True)
        (root / "code/packages/python/pyproject.toml").write_text(
            f"[tool.uv.workspace]\nmembers = [{members}]\n[tool.uv.sources]\n",
            encoding="utf-8",
        )
        (root / "code/programs").mkdir(parents=True)
        return root

    def test_workspace_member_escaping_the_python_tree_is_rejected(self) -> None:
        for member in ("../../programs/x", "/etc", "a/../../b"):
            with self.subTest(member=member), tempfile.TemporaryDirectory() as raw:
                root = self.make_tree(Path(raw), f'"{member}"')
                with self.assertRaises(runner.ConformanceError) as raised:
                    runner.expected_repository_source_exact_roots(root)
                self.assertEqual(
                    raised.exception.code, "BOUNDARY_DISCOVERY_MEMBER_ESCAPES"
                )

    def test_oversized_discovery_input_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            root = self.make_tree(Path(raw))
            big = root / "code/packages/rust/big"
            big.mkdir(parents=True)
            (big / "BUILD").write_bytes(b"#" * (1024 * 1024 + 1))
            with self.assertRaises(runner.ConformanceError) as raised:
                runner.expected_repository_source_exact_roots(root)
            self.assertEqual(raised.exception.code, "BOUNDARY_DISCOVERY_FILE_TOO_LARGE")

    def test_symlinked_build_and_tsconfig_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            root = self.make_tree(Path(raw))
            real = root / "real.txt"
            real.write_text("x", encoding="utf-8")
            pkg = root / "code/packages/rust/linked"
            pkg.mkdir(parents=True)
            (pkg / "BUILD").symlink_to(real)
            with self.assertRaises(runner.ConformanceError) as raised:
                runner.expected_repository_source_exact_roots(root)
            self.assertEqual(raised.exception.code, "BOUNDARY_DISCOVERY_SYMLINK")
        with tempfile.TemporaryDirectory() as raw:
            root = self.make_tree(Path(raw))
            pkg = root / "code/packages/typescript/linked"
            pkg.mkdir(parents=True)
            (pkg / "BUILD").write_text("npm install\n", encoding="utf-8")
            (root / "real.json").write_text("{}", encoding="utf-8")
            (pkg / "tsconfig.json").symlink_to(root / "real.json")
            with self.assertRaises(runner.ConformanceError) as raised:
                runner.expected_repository_source_exact_roots(root)
            self.assertEqual(raised.exception.code, "BOUNDARY_DISCOVERY_SYMLINK")

    @unittest.skipIf(hasattr(os, "geteuid") and os.geteuid() == 0, "root ignores modes")
    def test_unreadable_directory_fails_instead_of_being_omitted(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            root = self.make_tree(Path(raw))
            hidden = root / "code/packages/rust/hidden"
            hidden.mkdir(parents=True)
            hidden.chmod(0)
            try:
                with self.assertRaises(runner.ConformanceError) as raised:
                    runner.expected_repository_source_exact_roots(root)
            finally:
                hidden.chmod(0o755)
            self.assertEqual(raised.exception.code, "BOUNDARY_DISCOVERY_UNREADABLE")

    def test_generated_directories_are_pruned(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            root = self.make_tree(Path(raw))
            for name in ("node_modules", "target", ".venv", "dist", "bin"):
                vendored = root / "code/packages/rust/pkg" / name / "inner"
                vendored.mkdir(parents=True)
                (vendored / "BUILD").write_text(
                    'load("code/packages/starlark/library-rules/rust_library.star")\n',
                    encoding="utf-8",
                )
            roots = runner.expected_repository_source_exact_roots(root)
        self.assertEqual(roots[RUST_BOUNDARY], [])


if __name__ == "__main__":
    unittest.main()
