"""Tests for elixir_locks.py.  No Elixir needed; fixtures are temp git repos."""

from __future__ import annotations

import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import elixir_locks as el  # noqa: E402

SHA_A = "a" * 64
SHA_B = "b" * 64


def mix_exs(app: str, deps: str) -> str:
    return (
        "defmodule M.MixProject do\n  use Mix.Project\n"
        f"  def project, do: [app: :{app}, deps: deps()]\n"
        f"  defp deps do\n    [\n{deps}\n    ]\n  end\nend\n"
    )


def lock_line(name: str, version: str, outer: str | None = SHA_B) -> str:
    tail = f', "{outer}"' if outer else ""
    return (
        f'  "{name}": {{:hex, :{name}, "{version}", "{SHA_A}", [:mix], [], '
        f'"hexpm"{tail}}},\n'
    )


class RequirementTests(unittest.TestCase):
    def test_operators(self) -> None:
        cases = [
            ("~> 1.4", "1.4.5", True),
            ("~> 1.4", "1.3.9", False),
            ("~> 1.4", "2.0.0", False),
            ("~> 1.4.2", "1.4.9", True),
            ("~> 1.4.2", "1.5.0", False),
            ("== 1.0.0", "1.0.0", True),
            ("== 1.0.0", "1.0.1", False),
            ("1.0.0", "1.0.0", True),
            ("!= 1.0.0", "1.0.0", False),
            (">= 1.0.0 and < 2.0.0", "1.9.9", True),
            (">= 1.0.0 and < 2.0.0", "2.0.0", False),
            ("~> 1.0 or ~> 2.0", "2.3.0", True),
            ("~> 1.0 or ~> 2.0", "3.0.0", False),
            ("> 1.0", "1.0.0", False),
            ("<= 1.0", "1.0.0", True),
        ]
        for requirement, version, expected in cases:
            with self.subTest(requirement=requirement, version=version):
                self.assertEqual(el.satisfies(requirement, version), expected)

    def test_bad_input(self) -> None:
        with self.assertRaises(ValueError):
            el.satisfies("~> x", "1.0.0")
        with self.assertRaises(ValueError):
            el.parse_version("1.2.3.4")


class LockParseTests(unittest.TestCase):
    def test_complete_entry(self) -> None:
        entries, problems = el.parse_lock("%{\n" + lock_line("jason", "1.4.5") + "}\n")
        self.assertEqual(problems, [])
        self.assertEqual(entries["jason"].version, "1.4.5")

    def test_missing_outer_checksum(self) -> None:
        _, problems = el.parse_lock(lock_line("jason", "1.4.5", outer=None))
        self.assertTrue(any("outer" in p for p in problems))

    def test_garbage_entry(self) -> None:
        _, problems = el.parse_lock('  "x": {:hex, broken\n')
        self.assertTrue(any("unparseable" in p for p in problems))


class RepoTests(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.root = Path(self._tmp.name).resolve()
        self.git("init", "-q")
        self.git("config", "user.email", "t@example.com")
        self.git("config", "user.name", "t")

    def git(self, *args: str) -> None:
        subprocess.run(["git", *args], cwd=self.root, check=True, capture_output=True)

    def write(self, rel: str, text: str) -> None:
        path = self.root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")

    def commit(self) -> None:
        self.git("add", "-A")
        self.git("commit", "-q", "-m", "x")

    def layout(self, *, lock: bool = True, version: str = "1.4.5") -> None:
        """app -> lib (path) -> jason (test-only in lib2, real in lib)."""
        self.write("app/mix.exs", mix_exs("app", '{:lib, path: "../lib"}, {:lib2, path: "../lib2"}'))
        self.write("lib/mix.exs", mix_exs("lib", '{:jason, "~> 1.4"}'))
        self.write("lib2/mix.exs", mix_exs("lib2", '{:other, "~> 9.0", only: :test}'))
        if lock:
            self.write("app/mix.lock", "%{\n" + lock_line("jason", version) + "}\n")

    def test_roots_ignore_test_only_deps_of_path_deps(self) -> None:
        self.layout()
        self.commit()
        found = el.find_roots(self.root, el.load_projects(self.root))
        names = {p.name: sorted(r) for p, r in found.items()}
        # lib2 is a root for its own test-only dep, but app does not inherit it.
        self.assertEqual(names, {"app": ["jason"], "lib": ["jason"], "lib2": ["other"]})

    def test_passes_when_locked(self) -> None:
        self.layout()
        self.write("lib/mix.lock", "%{\n" + lock_line("jason", "1.4.5") + "}\n")
        self.write("lib2/mix.lock", "%{\n" + lock_line("other", "9.1.0") + "}\n")
        self.commit()
        self.assertEqual(el.run_check(self.root), [])

    def test_missing_lock(self) -> None:
        self.layout(lock=False)
        self.commit()
        problems = el.run_check(self.root)
        self.assertTrue(any("no tracked mix.lock" in p for p in problems))

    def test_version_not_satisfying(self) -> None:
        self.layout(version="2.0.0")
        self.write("lib/mix.lock", "%{\n" + lock_line("jason", "2.0.0") + "}\n")
        self.commit()
        self.assertTrue(any("does not satisfy" in p for p in el.run_check(self.root)))

    def test_missing_entry(self) -> None:
        self.layout()
        self.write("app/mix.lock", "%{\n" + lock_line("other", "1.0.0") + "}\n")
        self.write("lib/mix.lock", "%{\n" + lock_line("jason", "1.4.5") + "}\n")
        self.commit()
        self.assertTrue(any("missing entry" in p for p in el.run_check(self.root)))

    def test_disagreeing_locks(self) -> None:
        self.layout()
        self.write("lib/mix.lock", "%{\n" + lock_line("jason", "1.4.4") + "}\n")
        self.commit()
        self.assertTrue(any("disagree" in p for p in el.run_check(self.root)))

    def test_checksum_problem_reported(self) -> None:
        self.layout()
        self.write("app/mix.lock", "%{\n" + lock_line("jason", "1.4.5", outer=None) + "}\n")
        self.write("lib/mix.lock", "%{\n" + lock_line("jason", "1.4.5") + "}\n")
        self.commit()
        self.assertTrue(any("checksum" in p for p in el.run_check(self.root)))

    def test_changed_since_limits_scope(self) -> None:
        self.layout(lock=False)
        self.write("lib/mix.lock", "%{\n" + lock_line("jason", "1.4.5") + "}\n")
        self.commit()
        self.git("branch", "base")
        self.write("lib2/note.txt", "hi")  # lib2 is only in app's closure
        self.commit()
        self.assertTrue(el.run_check(self.root, "base"))  # app is affected, unlocked
        self.git("reset", "-q", "--hard", "base")
        self.write("unrelated/readme.txt", "hi")
        self.commit()
        self.assertEqual(el.run_check(self.root, "base"), [])

    def test_main_exit_codes(self) -> None:
        self.layout(lock=False)
        self.commit()
        self.assertEqual(el.main(["--root", str(self.root), "check"]), 1)
        self.assertEqual(el.main(["--root", str(self.root), "roots"]), 0)


if __name__ == "__main__":
    unittest.main()
