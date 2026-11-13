"""Tests for code/scripts/toolchain_pins.py.

Each test builds a throwaway git repository holding just a mise.toml, a pins
file and the workflow or BUILD text under test, because the checker lists
files with ``git ls-files`` exactly as it does in CI.
"""

from __future__ import annotations

import contextlib
import io
import json
import subprocess
import sys
import tempfile
import textwrap
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS_DIR))

import toolchain_pins as pins

MISE = """
[tools]
go = "1.26.1"
python = "3.13.17"
node = "22.23.5"
rust = "1.99.1"
uv = "0.13.7"
"""

PINS = {
    "schema_version": 1,
    "ci_only": {"elixir": "1.18.4", "erlang": "27.3.4.11"},
    "rust_nightly": "nightly-2026-11-10",
    "bundler": "4.0.24",
    "deviations": [],
}

GOOD_WORKFLOW = """
name: ok
jobs:
  build:
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            python-version: '3.13.17'
    steps:
      - uses: actions/setup-python@abc
        with:
          python-version: ${{ matrix.python-version }}
      - name: Set up Rust
        uses: dtolnay/rust-toolchain@abc # stable
        with:
          toolchain: "1.99.1"
      - uses: astral-sh/setup-uv@abc
        with:
          version: "0.13.7"
      - uses: erlef/setup-beam@abc
        with:
          elixir-version: '1.18.4'
          otp-version: '27.3.4.11'
      - run: |
          cargo +nightly-2026-11-10 miri test
          gem install bundler --version 4.0.24
"""


class RepoCase(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name)
        subprocess.run(["git", "init", "-q"], cwd=self.root, check=True)
        self.write("mise.toml", MISE)
        self.write_pins(PINS)

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def write(self, rel: str, text: str) -> None:
        path = self.root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(textwrap.dedent(text), encoding="utf-8")
        subprocess.run(["git", "add", rel], cwd=self.root, check=True)

    def write_pins(self, data: dict) -> None:
        self.write("code/specs/data/toolchain-pins.json", json.dumps(data))

    def messages(self) -> list[str]:
        return [str(f) for f in pins.check(self.root)]


class MiseTests(RepoCase):
    def test_clean_repository_passes(self) -> None:
        self.write(".github/workflows/ok.yml", GOOD_WORKFLOW)
        self.assertEqual(self.messages(), [])

    def test_floating_mise_values_are_rejected(self) -> None:
        for value in ("latest", "stable", "lts", "3.13"):
            with self.subTest(value=value):
                self.write("mise.toml", f'[tools]\npython = "{value}"\n')
                self.assertTrue(
                    any("mise.toml" in m and "python" in m for m in self.messages())
                )

    def test_two_part_gradle_release_is_exact(self) -> None:
        self.write("mise.toml", MISE + 'gradle = "8.14"\n')
        self.assertEqual(self.messages(), [])

    def test_a_tool_cannot_be_pinned_twice(self) -> None:
        data = dict(PINS, ci_only={"go": "1.26.1"})
        self.write_pins(data)
        self.assertTrue(any("both mise.toml and ci_only" in m for m in self.messages()))


class WorkflowTests(RepoCase):
    def test_mismatch_with_mise_is_reported_with_its_line(self) -> None:
        self.write(".github/workflows/a.yml", """
            jobs:
              x:
                steps:
                  - uses: actions/setup-go@abc
                    with:
                      go-version: '1.26.2'
            """)
        [message] = self.messages()
        self.assertIn(".github/workflows/a.yml:7:", message)
        self.assertIn("differs from go = '1.26.1'", message)

    def test_range_versions_are_rejected(self) -> None:
        self.write(".github/workflows/a.yml", """
            jobs:
              x:
                steps:
                  - uses: actions/setup-node@abc
                    with:
                      node-version: 22
                  - uses: actions/setup-node@abc
                    with:
                      node-version: '22.23'
            """)
        major, minor = self.messages()
        self.assertIn("not an exact version", major)
        self.assertIn("missing the patch component", minor)

    def test_matrix_literals_are_checked(self) -> None:
        self.write(".github/workflows/a.yml", """
            jobs:
              x:
                strategy:
                  matrix:
                    include:
                      - python-version: '3.12'
            """)
        self.assertEqual(len(self.messages()), 1)

    def test_a_reasoned_deviation_is_accepted_and_a_stale_one_is_not(self) -> None:
        self.write(".github/workflows/a.yml", """
            jobs:
              x:
                steps:
                  - uses: actions/setup-node@abc
                    with:
                      node-version: "20.20.2"
            """)
        deviation = {"file": ".github/workflows/a.yml", "input": "node-version",
                     "value": "20.20.2", "reason": "deliberately old"}
        stale = dict(deviation, value="18.0.0")
        self.write_pins(dict(PINS, deviations=[deviation]))
        self.assertEqual(self.messages(), [])
        self.write_pins(dict(PINS, deviations=[deviation, stale]))
        [message] = self.messages()
        self.assertIn("matches nothing", message)

    def test_a_deviation_needs_a_reason(self) -> None:
        self.write(".github/workflows/a.yml", """
            jobs:
              x:
                steps:
                  - uses: actions/setup-node@abc
                    with:
                      node-version: "20.20.2"
            """)
        deviation = {"file": ".github/workflows/a.yml", "input": "node-version",
                     "value": "20.20.2", "reason": ""}
        self.write_pins(dict(PINS, deviations=[deviation]))
        self.assertTrue(any("needs a reason" in m for m in self.messages()))

    def test_rust_toolchain_and_uv_need_an_explicit_version(self) -> None:
        self.write(".github/workflows/a.yml", """
            jobs:
              x:
                steps:
                  - uses: dtolnay/rust-toolchain@abc # stable
                  - name: uv
                    uses: astral-sh/setup-uv@abc
                    with:
                      enable-cache: true
            """)
        messages = self.messages()
        self.assertEqual(len(messages), 2)
        self.assertTrue(any("without `toolchain:`" in m for m in messages))
        self.assertTrue(any("without `version:`" in m for m in messages))

    def test_version_key_outside_setup_uv_is_not_a_toolchain(self) -> None:
        self.write(".github/workflows/a.yml", """
            jobs:
              x:
                outputs:
                  version: latest
                steps:
                  - uses: some/other-action@abc
                    with:
                      version: latest
            """)
        self.assertEqual(self.messages(), [])

    def test_uses_after_with_still_names_the_step(self) -> None:
        self.write(".github/workflows/a.yml", """
            jobs:
              x:
                steps:
                  - with:
                      version: "latest"
                    uses: astral-sh/setup-uv@abc
            """)
        self.assertTrue(any("'latest'" in m for m in self.messages()))

    def test_undated_nightly_toolchain_input_is_rejected(self) -> None:
        self.write(".github/workflows/a.yml", """
            jobs:
              x:
                steps:
                  - uses: dtolnay/rust-toolchain@abc
                    with:
                      toolchain: nightly
            """)
        self.assertTrue(any("nightly" in m for m in self.messages()))

    def test_rust_version_env_is_checked(self) -> None:
        self.write(".github/workflows/a.yml", """
            env:
              RUST_VERSION: "1.98"
            """)
        self.assertTrue(any("RUST_VERSION" in m for m in self.messages()))


class ShellTests(RepoCase):
    def test_floating_shell_commands_are_rejected_everywhere(self) -> None:
        cases = {
            "code/packages/rust/a/BUILD": "rustup run stable cargo build\n",
            "code/packages/rust/a/BUILD_windows": "cargo +stable test\n",
            "code/scripts/miri.sh": "cargo +nightly miri test\n",
            ".github/workflows/a.yml": "jobs:\n  x:\n    steps:\n      - run: rustup toolchain install stable\n",
            "code/packages/ruby/a/BUILD": "gem install bundler\n",
        }
        for rel, text in cases.items():
            with self.subTest(rel=rel):
                self.write(rel, text)
                self.assertTrue(any(m.startswith(rel + ":") for m in self.messages()))
                (self.root / rel).unlink()
                subprocess.run(["git", "rm", "-q", "--cached", rel], cwd=self.root, check=True)

    def test_comments_and_pinned_forms_are_allowed(self) -> None:
        self.write("code/packages/rust/a/BUILD",
                   "# rustup run stable cargo build used to be here\n"
                   "cargo +nightly-2026-11-10 miri test\n"
                   "gem install bundler -v 4.0.24\n")
        self.write("code/packages/rust/a/BUILD_windows",
                   "REM rustup run stable cargo build\n:: cargo +stable test\n")
        self.assertEqual(self.messages(), [])


class RepositoryTests(unittest.TestCase):
    def test_the_repository_itself_is_pinned(self) -> None:
        findings = pins.check(pins.REPO_ROOT)
        self.assertEqual([str(f) for f in findings], [])

    def test_main_reports_success_and_failure(self) -> None:
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(pins.main(["check"]), 0)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            (root / "mise.toml").write_text('[tools]\ngo = "latest"\n', encoding="utf-8")
            data = root / "code/specs/data"
            data.mkdir(parents=True)
            (data / "toolchain-pins.json").write_text(json.dumps(PINS), encoding="utf-8")
            with contextlib.redirect_stderr(io.StringIO()):
                self.assertEqual(pins.main(["check", "--root", str(root)]), 1)


if __name__ == "__main__":
    unittest.main()
