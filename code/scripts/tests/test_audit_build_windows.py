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

    def allow(self, entries: dict[str, dict]) -> None:
        """Write the W6 allowlist for this tree."""
        path = self.root.joinpath(*audit.ALLOWLIST_PATH)
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps({"version": 1, "entries": entries}), encoding="utf-8")

    def findings(self, cls: str) -> list[audit.Finding]:
        return [f for f in audit.audit(self.root) if f.cls == cls]


def entry(category: str = "no-tests-by-design", **overrides: str) -> dict:
    base = {
        "category": category,
        "reason": "nothing to run on this platform for a stated reason",
        "evidence": "BUILD-002 W6 test fixture",
    }
    base.update(overrides)
    return base


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
        # No Windows build at all is a skip like any other: class F (W6).
        self.assertEqual(self.tree.classes(), [("F", False)])


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
        # No runner is swapped (class D stays quiet); the stub itself is class F.
        self.assertEqual(self.tree.classes(), [("F", False)])

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

    def test_venv_bin_after_shell_operators_is_posix(self) -> None:
        for line in ("cd x &&.venv/bin/python -m pytest", "true;.venv/bin/ruff check"):
            with self.subTest(line=line):
                self.assertEqual(audit.posix_construct(line), ".venv/bin/")

    def test_venv_bin_pattern_is_linear_on_long_relative_paths(self) -> None:
        # A crafted BUILD line of many `../` must not stall the audit.
        import time

        line = "../" * 40_000 + "x"
        started = time.perf_counter()
        audit.posix_construct(line)
        self.assertLess(time.perf_counter() - started, 1.0)

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


class WindowsSkipTest(AuditTestCase):
    """Class F: a Windows recipe that runs nothing, unless allowlisted."""

    def skip_of(self, files: dict[str, str], name: str = "p") -> tuple[str, str] | None:
        pkg = self.tree.package("go", name, files)
        return audit.windows_skip_shape(pkg)

    def test_new_echo_only_recipe_blocks(self) -> None:
        self.tree.package(
            "go",
            "p",
            {"BUILD": "go test ./...\n", "BUILD_windows": 'echo "skipping on windows"\n'},
        )
        found = self.tree.findings("F")
        self.assertEqual([(f.package, f.file, f.advisory) for f in found],
                         [("packages/go/p", "BUILD_windows", False)])
        self.assertIn("echo-only", found[0].detail)
        self.assertEqual(audit.main(["--root", str(self.tree.root), "--check"]), 1)

    def test_real_recipe_passes(self) -> None:
        self.tree.package(
            "go",
            "p",
            {"BUILD": "go test ./...\n", "BUILD_windows": "go test ./... -cover\n"},
        )
        self.assertEqual(self.tree.classes(), [])

    def test_recipe_that_echoes_and_then_tests_is_real(self) -> None:
        self.assertIsNone(
            self.skip_of({"BUILD_windows": "echo building\ngo test ./...\n"})
        )

    def test_comments_rem_and_blank_lines_are_not_commands(self) -> None:
        # Only comments around a real command: still a real recipe.
        self.assertIsNone(
            self.skip_of(
                {"BUILD_windows": "# note\nREM note\n@rem note\n:: note\n\ngo test ./...\n"}
            )
        )

    def test_comments_around_echo_do_not_hide_the_skip(self) -> None:
        shape = self.skip_of(
            {"BUILD_windows": "# build-tool: unsupported=X\nREM why\n:: why\necho skipped\n"}
        )
        self.assertIsNotNone(shape)
        self.assertEqual(shape[0], "BUILD_windows")

    def test_comment_only_recipe_is_empty(self) -> None:
        shape = self.skip_of({"BUILD_windows": "# nothing here\nREM nor here\n"})
        self.assertEqual(shape, ("BUILD_windows", "empty recipe (no commands)"))

    def test_other_skip_shapes(self) -> None:
        for line in (
            "@echo off",
            "echo.",
            "exit 0",
            "exit /b 0",
            "exit /b",
            "cmd /c exit 0",
            "cmd.exe /c exit",
            "cmd /c echo skipped",
            "type nul",
            "ver >nul",
            "call echo skipped",
            "goto :eof",
            "if not exist x echo skipped",
            'if "%CI%"=="" echo not in CI',
            "if exist foo (echo have foo)",
            "if errorlevel 1 (echo cabal not found) else (echo fine)",
            "(echo skipped & exit /b 0)",
            "echo a && echo b",
            "if /i not defined X echo no X",
            'powershell -NoProfile -Command "Write-Host skipping"',
            "pwsh -c 'Write-Output skipped'",
        ):
            with self.subTest(line=line):
                self.assertTrue(audit.is_noop_line(line), line)

    def test_two_noop_lines_are_still_a_skip(self) -> None:
        self.assertIsNotNone(self.skip_of({"BUILD_windows": "echo a\nexit /b 0\n"}))
        shape = self.skip_of({"BUILD_windows": "echo a\nexit 0\n"}, name="q")
        self.assertIn("no-op only", shape[1])

    def test_real_commands_are_never_noops(self) -> None:
        for line in (
            "go test ./...",
            "cargo test -p x -- --nocapture",
            "exit 1",
            "exit /b 1",
            "echo y | cargo publish",
            'echo "a && b" && cargo test',
            "if exist x (cargo test) else (echo skipped)",
            "if exist x (echo have) else (cargo test)",
            "echo started & cargo test",
            "where swift >nul 2>nul && swift test || echo not available",
            'powershell -File run.ps1 -Name "Write-Host"',
            "powershell -Command \"Write-Host start; cargo test\"",
            "echoing.exe",
            "exit1.bat",
            '"C:\\Program Files (x86)\\tool.exe" --run',
            "set FOO=1",
            "cd tests",
        ):
            with self.subTest(line=line):
                self.assertFalse(audit.is_noop_line(line), line)

    def test_quoted_separators_do_not_split(self) -> None:
        self.assertEqual(audit.split_segments('echo "a && b" && echo c'), ['echo "a && b"', "echo c"])
        self.assertEqual(audit.split_segments("echo a | more"), ["echo a | more"])

    def test_fallback_build_counts_when_windows_runs_it(self) -> None:
        # No BUILD_windows: Windows runs BUILD, so an echo-only BUILD is a skip.
        self.tree.package("rust", "p", {"BUILD": 'echo "compile-only crate"\n'})
        found = self.tree.findings("F")
        self.assertEqual([f.file for f in found], ["BUILD"])

    def test_package_with_no_windows_build_file_is_a_skip(self) -> None:
        self.tree.package("go", "p", {"BUILD_mac_and_linux": "go test ./...\n"})
        found = self.tree.findings("F")
        self.assertEqual([f.file for f in found], ["(none)"])
        self.assertIn("no Windows build file", found[0].detail)

    def test_starlark_build_is_never_a_skip(self) -> None:
        self.tree.package("go", "p", {"BUILD": 'go_library(name = "p")\n'})
        self.assertEqual(self.tree.findings("F"), [])

    def test_hostile_long_line_is_linear_and_not_a_skip(self) -> None:
        import time

        line = "if " + "a" * 200_000 + " echo x"
        started = time.perf_counter()
        self.assertFalse(audit.is_noop_line(line))
        self.assertLess(time.perf_counter() - started, 1.0)
        started = time.perf_counter()
        audit.is_noop_line("if " + '"' * 1500 + "==" + "\\ " * 20)
        self.assertLess(time.perf_counter() - started, 1.0)

    def test_allowlisted_skip_passes(self) -> None:
        self.tree.package(
            "rust", "p", {"BUILD": "cargo test\n", "BUILD_windows": "echo skip\n"}
        )
        self.tree.allow({"packages/rust/p": entry("no-tests-by-design")})
        self.assertEqual(self.tree.classes(), [])


class SkipAllowlistTest(AuditTestCase):
    """Class G: the allowlist may only shrink and must stay honest."""

    def skip_package(self, name: str = "p", build: str = "cargo test\n") -> None:
        self.tree.package(
            "rust", name, {"BUILD": build, "BUILD_windows": "echo skip\n"}
        )

    def test_stale_entry_blocks(self) -> None:
        self.tree.package(
            "rust", "p", {"BUILD": "cargo test\n", "BUILD_windows": "cargo test\n"}
        )
        self.tree.allow({"packages/rust/p": entry()})
        found = self.tree.findings("G")
        self.assertEqual([f.package for f in found], ["packages/rust/p"])
        self.assertIn("stale entry", found[0].detail)
        self.assertIn("real Windows recipe", found[0].detail)
        self.assertFalse(found[0].advisory)

    def test_entry_for_a_deleted_package_blocks(self) -> None:
        self.tree.allow({"packages/rust/gone": entry()})
        found = self.tree.findings("G")
        self.assertIn("no longer exists", found[0].detail)

    def test_new_entry_beyond_the_cap_blocks(self) -> None:
        self.skip_package("a")
        self.skip_package("b")
        self.tree.allow({"packages/rust/a": entry(), "packages/rust/b": entry()})
        saved = dict(audit.ALLOWLIST_CAPS)
        self.addCleanup(lambda: audit.ALLOWLIST_CAPS.update(saved))
        audit.ALLOWLIST_CAPS["no-tests-by-design"] = 1
        found = self.tree.findings("G")
        self.assertEqual(len(found), 1)
        self.assertIn("cap is 1", found[0].detail)
        # Raising the cap is the explicit, reviewable act that clears it.
        audit.ALLOWLIST_CAPS["no-tests-by-design"] = 2
        self.assertEqual(self.tree.findings("G"), [])

    def test_unlisted_skip_still_blocks_even_with_cap_room(self) -> None:
        self.skip_package("a")
        self.skip_package("b")
        self.tree.allow({"packages/rust/a": entry()})
        self.assertEqual([f.package for f in self.tree.findings("F")], ["packages/rust/b"])

    def test_malformed_entries_block(self) -> None:
        self.skip_package()
        for bad, expect in (
            (entry(category="whatever"), "category must be one of"),
            (entry(reason="short"), "`reason` must be"),
            (entry(evidence=""), "`evidence` must be"),
            ("not an object", "must be an object"),
        ):
            with self.subTest(expect=expect):
                self.tree.allow({"packages/rust/p": bad})
                details = " ".join(f.detail for f in self.tree.findings("G"))
                self.assertIn(expect, details)

    def test_invalid_or_misshapen_allowlist_file_blocks(self) -> None:
        path = self.tree.root.joinpath(*audit.ALLOWLIST_PATH)
        path.parent.mkdir(parents=True)
        path.write_text("{not json", encoding="utf-8")
        self.assertIn("not valid JSON", self.tree.findings("G")[0].detail)
        path.write_text('{"entries": []}', encoding="utf-8")
        self.assertIn("`entries` object", self.tree.findings("G")[0].detail)

    def test_missing_allowlist_is_empty_not_an_error(self) -> None:
        self.assertEqual(audit.load_allowlist(self.tree.root), ({}, []))

    def test_metal_category_must_be_backed_by_the_package(self) -> None:
        self.tree.package(
            "rust",
            "paint-metal",
            {
                "BUILD": 'if [ "$(uname)" = "Darwin" ]; then cargo test; fi\n',
                "BUILD_windows": "echo Metal is macOS only\n",
            },
        )
        self.skip_package("plain")
        self.tree.allow(
            {
                "packages/rust/paint-metal": entry("intrinsic-apple"),
                "packages/rust/plain": entry("intrinsic-apple"),
            }
        )
        found = self.tree.findings("G")
        self.assertEqual([f.package for f in found], ["packages/rust/plain"])
        self.assertIn("nothing in the package names Metal", found[0].detail)

    def test_every_category_has_a_cap(self) -> None:
        self.assertEqual(set(audit.CATEGORIES), set(audit.ALLOWLIST_CAPS))


class ConditionalSkipTest(AuditTestCase):
    """Class H: a real recipe that quietly passes when the toolchain is absent."""

    def test_probe_with_echo_skipping_fallback_is_advisory(self) -> None:
        self.tree.package(
            "swift",
            "p",
            {
                "BUILD": "swift test\n",
                "BUILD_windows": "where swift >nul 2>nul && swift test || echo not available - skipping\n",
            },
        )
        self.assertEqual(self.tree.classes(), [("H", True)])
        self.assertEqual(audit.main(["--root", str(self.tree.root), "--check"]), 0)

    def test_if_else_form_is_detected(self) -> None:
        self.tree.package(
            "haskell",
            "p",
            {
                "BUILD": "cabal test all\n",
                "BUILD_windows": "if errorlevel 1 (echo cabal not found -- skipping) else (cabal test all)\n",
            },
        )
        self.assertEqual(self.tree.classes(), [("H", True)])

    def test_plain_real_recipe_and_plain_skip_are_not_h(self) -> None:
        self.tree.package("go", "a", {"BUILD": "x\n", "BUILD_windows": "go test ./...\n"})
        self.tree.package("go", "b", {"BUILD": "x\n", "BUILD_windows": "echo skipping\n"})
        self.assertEqual([c for c, _ in self.tree.classes()], ["F"])

    def test_render_summarises_h_unless_verbose(self) -> None:
        self.tree.package(
            "swift",
            "p",
            {"BUILD": "x\n", "BUILD_windows": "where swift && swift test || echo no - skipping\n"},
        )
        findings = audit.audit(self.tree.root)
        self.assertIn("by language: swift: 1", audit.render(findings, verbose=False))
        self.assertIn("packages/swift/p/BUILD_windows", audit.render(findings, verbose=True))


class RepositoryAllowlistTest(unittest.TestCase):
    """The committed allowlist, against the real tree."""

    def setUp(self) -> None:
        self.entries, problems = audit.load_allowlist(REPO)
        self.assertEqual(problems, [])

    def test_allowlist_matches_the_caps_exactly(self) -> None:
        # Equality, not <=: a stale cap is slack that would let an entry in
        # unreviewed, so lowering the list means lowering the cap with it.
        counts: dict[str, int] = {}
        for entry_ in self.entries.values():
            counts[entry_["category"]] = counts.get(entry_["category"], 0) + 1
        self.assertEqual(counts, audit.ALLOWLIST_CAPS)

    def test_every_metal_package_is_apple_intrinsic(self) -> None:
        for rel, entry_ in self.entries.items():
            if "metal" in rel.lower() or "objc" in rel.lower():
                self.assertEqual(entry_["category"], "intrinsic-apple", rel)

    def test_real_repository_has_no_unlisted_windows_skip(self) -> None:
        offenders = [f for f in audit.audit(REPO) if f.cls in ("F", "G")]
        self.assertEqual(offenders, [], audit.render(offenders))


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
