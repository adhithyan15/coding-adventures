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


def snapshot(root: Path) -> dict[Path, bytes]:
    return {
        path: path.read_bytes() for path in sorted(root.rglob("*")) if path.is_file()
    }


class SyncBoundaryTests(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.root = Path(self._tmp.name)
        copy_working_set(self.root)
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


class DiscoveryTests(unittest.TestCase):
    """``expected_repository_source_exact_roots`` on a tiny, real directory tree."""

    def test_finds_rust_starlark_consumer_and_typescript_workspace_root(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
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


if __name__ == "__main__":
    unittest.main()
