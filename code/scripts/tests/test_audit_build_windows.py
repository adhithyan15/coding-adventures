"""Tests for code/scripts/audit_build_windows.py.

Each test builds a throwaway repository tree with one package in it, so a
class of drift is exercised in isolation: first the shape that must be
reported, then the corrected shape that must not be.  The last test runs the
audit over the real repository and requires zero blocking findings, which is
what keeps the drift from coming back.
"""

from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
SPEC = importlib.util.spec_from_file_location(
    "audit_build_windows", REPO / "code" / "scripts" / "audit_build_windows.py"
)
audit = importlib.util.module_from_spec(SPEC)
# dataclasses look the module up in sys.modules while it executes.
sys.modules["audit_build_windows"] = audit
SPEC.loader.exec_module(audit)


class Tree:
    """A temporary repo root with helpers to add packages."""

    def __init__(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name)

    def close(self) -> None:
        self._tmp.cleanup()

    def package(
        self, language: str, name: str, files: dict[str, str], kind: str = "packages"
    ) -> Path:
        directory = self.root / "code" / kind / language / name
        directory.mkdir(parents=True)
        for filename, content in files.items():
            (directory / filename).write_text(content, encoding="utf-8")
        return directory

    def classes(self) -> list[tuple[str, bool]]:
        return [(f.cls, f.advisory) for f in audit.audit(self.root)]


class AuditTestCase(unittest.TestCase):
    def setUp(self) -> None:
        self.tree = Tree()
        self.addCleanup(self.tree.close)


class BuildFileSelectionTest(AuditTestCase):
    def test_windows_prefers_build_windows_then_build(self) -> None:
        pkg = self.tree.package("go", "p", {"BUILD": "x", "BUILD_mac_and_linux": "y"})
        self.assertEqual(
            audit.effective_build_file(pkg, audit.WINDOWS_CANDIDATES).name, "BUILD"
        )
        (pkg / "BUILD_windows").write_text("z", encoding="utf-8")
        self.assertEqual(
            audit.effective_build_file(pkg, audit.WINDOWS_CANDIDATES).name,
            "BUILD_windows",
        )

    def test_mac_and_linux_file_is_never_a_windows_build(self) -> None:
        pkg = self.tree.package(
            "go", "p", {"BUILD_mac_and_linux": "if [ -f x ]; then y; fi"}
        )
        self.assertIsNone(audit.effective_build_file(pkg, audit.WINDOWS_CANDIDATES))
        self.assertEqual(self.tree.classes(), [])


class VitestDriftTest(AuditTestCase):
    def test_vitest_in_windows_without_dependency_is_reported(self) -> None:
        self.tree.package(
            "typescript",
            "p",
            {
                "package.json": json.dumps(
                    {"scripts": {"test:coverage": "node --test x"}}
                ),
                "BUILD": "npm ci --quiet\nnpm run test:coverage --silent\n",
                "BUILD_windows": "npm ci --quiet\nnpx vitest run --coverage\n",
            },
        )
        classes = self.tree.classes()
        self.assertIn(("A", False), classes)
        # The same drift is also a swapped runner.
        self.assertIn(("D", False), classes)

    def test_vitest_with_dependency_is_fine(self) -> None:
        body = "npm ci --quiet\nnpx vitest run --coverage\n"
        self.tree.package(
            "typescript",
            "p",
            {
                "package.json": json.dumps({"devDependencies": {"vitest": "^3"}}),
                "BUILD": body,
                "BUILD_windows": body,
            },
        )
        self.assertEqual(self.tree.classes(), [])

    def test_matching_node_test_fronts_are_fine(self) -> None:
        body = (
            "npm ci --quiet\nnpm run build --silent\nnpm run test:coverage --silent\n"
        )
        self.tree.package(
            "typescript",
            "p",
            {
                "package.json": json.dumps(
                    {"scripts": {"test:coverage": "node --test x"}}
                ),
                "BUILD": body,
                "BUILD_windows": body,
            },
        )
        self.assertEqual(self.tree.classes(), [])


class Perl5LibTest(AuditTestCase):
    def test_set_perl5lib_is_reported_in_both_quoting_styles(self) -> None:
        self.tree.package(
            "perl",
            "a",
            {
                "BUILD": "prove -l t/\n",
                "BUILD_windows": "set PERL5LIB=..\\x\\lib;%PERL5LIB% && prove -l -v t\n",
            },
        )
        self.tree.package(
            "perl",
            "b",
            {
                "BUILD": "prove -l t/\n",
                "BUILD_windows": 'set "PERL5LIB=..\\x\\lib" && prove -l -v t\n',
            },
        )
        self.assertEqual([c for c, _ in self.tree.classes()], ["B", "B"])

    def test_dash_i_form_is_fine(self) -> None:
        self.tree.package(
            "perl",
            "a",
            {"BUILD": "prove -l t/\n", "BUILD_windows": "prove -I..\\x\\lib -l -v t\n"},
        )
        self.assertEqual(self.tree.classes(), [])


class RubyStandardrbTest(AuditTestCase):
    BUILD = "bundle install --quiet\nbundle exec standardrb --no-fix lib/\nbundle exec rspec\n"

    def test_missing_build_windows_is_reported(self) -> None:
        self.tree.package("ruby", "p", {"BUILD": self.BUILD})
        self.assertEqual(self.tree.classes(), [("C", False)])

    def test_build_windows_without_exemption_is_reported(self) -> None:
        self.tree.package(
            "ruby", "p", {"BUILD": self.BUILD, "BUILD_windows": self.BUILD}
        )
        self.assertEqual(self.tree.classes(), [("C", False)])

    def test_exemption_is_fine(self) -> None:
        windows = self.BUILD.replace("--no-fix ", "--no-fix --except Layout/EndOfLine ")
        self.tree.package("ruby", "p", {"BUILD": self.BUILD, "BUILD_windows": windows})
        self.assertEqual(self.tree.classes(), [])

    def test_other_languages_are_not_checked(self) -> None:
        self.tree.package("python", "p", {"BUILD": "standardrb lib/\n"})
        self.assertEqual(self.tree.classes(), [])


class RunnerMismatchTest(AuditTestCase):
    def test_skip_stub_is_not_a_mismatch(self) -> None:
        self.tree.package(
            "perl",
            "p",
            {
                "BUILD": "prove -l t/\n",
                "BUILD_windows": "echo Perl testing is not supported on Windows - skipping\n",
            },
        )
        self.assertEqual(self.tree.classes(), [])

    def test_dropped_runner_is_advisory(self) -> None:
        self.tree.package(
            "go",
            "p",
            {
                "BUILD": "cargo test\ngo test ./...\n",
                "BUILD_windows": "go test ./...\n",
            },
        )
        self.assertEqual(self.tree.classes(), [("D", True)])

    def test_swapped_runner_blocks(self) -> None:
        self.tree.package(
            "python",
            "p",
            {
                "BUILD": "python -m pytest\n",
                "BUILD_windows": "python -m unittest discover\n",
            },
        )
        self.assertEqual(self.tree.classes(), [("D", False)])

    def test_perl_prove_and_direct_perl_tests_share_a_runner(self) -> None:
        self.tree.package(
            "perl",
            "p",
            {"BUILD": "prove -l t/\n", "BUILD_windows": "perl -Ilib t/00-load.t\n"},
        )
        self.assertEqual(self.tree.classes(), [])


class PosixFallbackTest(AuditTestCase):
    def test_posix_syntax_in_fallback_build_is_reported(self) -> None:
        self.tree.package(
            "lua", "p", {"BUILD": "luarocks show x >/dev/null 2>&1 || echo no\n"}
        )
        self.assertEqual(self.tree.classes(), [("E", False)])

    def test_each_construct_is_recognised(self) -> None:
        for line in (
            "if [ -f x ]; then y; fi",
            "command -v perl",
            "X=$(pwd)",
            "echo ${HOME}",
            ".venv/bin/python -m pytest tests/ -v",
            ".venv/bin/ruff check src/",
            "../../../packages/python/conduit/.venv/bin/python -m pytest",
        ):
            with self.subTest(line=line):
                self.assertTrue(audit.posix_construct(line))

    def test_windows_venv_scripts_path_is_not_posix(self) -> None:
        self.assertEqual(audit.posix_construct(r".venv\Scripts\python -m pytest"), "")
        self.assertEqual(audit.posix_construct("uv venv .venv --quiet --no-project"), "")

    def test_python_build_without_build_windows_is_reported(self) -> None:
        self.tree.package(
            "python",
            "p",
            {"BUILD": "uv venv --quiet --clear\n.venv/bin/python -m pytest tests/ -v\n"},
        )
        self.assertEqual(self.tree.classes(), [("E", False)])

    def test_double_quoted_payload_is_not_read(self) -> None:
        self.assertEqual(
            audit.posix_construct('python -c "import os; print(os.devnull)"'), ""
        )
        self.assertEqual(audit.posix_construct('bash -c "command -v x >/dev/null"'), "")

    def test_build_windows_present_means_no_fallback(self) -> None:
        self.tree.package(
            "lua",
            "p",
            {
                "BUILD": "luarocks show x >/dev/null 2>&1\n",
                "BUILD_windows": "luarocks show x 1>nul 2>nul\n",
            },
        )
        self.assertEqual(self.tree.classes(), [])

    def test_known_exceptions_are_advisory(self) -> None:
        for rel in audit.KNOWN_POSIX_FALLBACKS:
            kind, language, name = rel.split("/")
            self.tree.package(language, name, {"BUILD": "x >/dev/null\n"}, kind)
        classes = self.tree.classes()
        self.assertEqual(len(classes), len(audit.KNOWN_POSIX_FALLBACKS))
        self.assertTrue(all(advisory for _, advisory in classes))

    def test_starlark_build_is_skipped(self) -> None:
        self.tree.package(
            "go", "p", {"BUILD": 'go_library(name = "p", srcs = ["a/*.go"])\n'}
        )
        self.assertEqual(self.tree.classes(), [])


class DriverTest(AuditTestCase):
    def test_comments_and_rem_lines_are_ignored(self) -> None:
        self.tree.package(
            "perl",
            "p",
            {
                "BUILD": "prove -l t/\n",
                "BUILD_windows": "# set PERL5LIB=x\nREM set PERL5LIB=y\nprove -l t\n",
            },
        )
        self.assertEqual(self.tree.classes(), [])

    def test_check_mode_exit_codes(self) -> None:
        self.tree.package(
            "perl",
            "p",
            {
                "BUILD": "prove -l t/\n",
                "BUILD_windows": "set PERL5LIB=..\\x\\lib && prove -l t\n",
            },
        )
        self.assertEqual(audit.main(["--root", str(self.tree.root)]), 0)
        self.assertEqual(audit.main(["--root", str(self.tree.root), "--check"]), 1)

    def test_empty_tree_is_clean(self) -> None:
        self.assertEqual(audit.main(["--root", str(self.tree.root), "--check"]), 0)


class RepositoryTest(unittest.TestCase):
    def test_real_repository_has_no_blocking_drift(self) -> None:
        blocking = [f for f in audit.audit(REPO) if not f.advisory]
        self.assertEqual(blocking, [], audit.render(blocking))


if __name__ == "__main__":
    unittest.main()


class NodeTestMatcherTests(unittest.TestCase):
    """`node [flags] --test` is matched by a linear token scan (CodeQL py/redos)."""

    def test_recognises_node_test_forms(self) -> None:
        for line in (
            "node --test",
            "node --test test/x.test.js",
            "node --experimental-strip-types --test x",
            "C:\\tools\\node.exe --experimental-vm-modules --test",
            "/usr/bin/node --test",
        ):
            with self.subTest(line=line):
                self.assertTrue(audit._is_node_test(line))

    def test_rejects_non_node_test_forms(self) -> None:
        for line in (
            "node script.js --test",
            "nodemon --test",
            "npx vitest run",
            "node",
            "echo --test",
        ):
            with self.subTest(line=line):
                self.assertFalse(audit._is_node_test(line))

    def test_hostile_line_is_linear(self) -> None:
        import time

        line = "node " + "-!\t-" * 50_000
        start = time.perf_counter()
        self.assertFalse(audit._is_node_test(line))
        self.assertLess(time.perf_counter() - start, 1.0)
