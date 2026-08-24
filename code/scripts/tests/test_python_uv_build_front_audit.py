from __future__ import annotations

import contextlib
import io
import json
import sys
import tempfile
import unittest
from pathlib import Path
from typing import Any
from unittest import mock


REPO_ROOT = Path(__file__).resolve().parents[3]
SCRIPTS_DIR = REPO_ROOT / "code" / "scripts"
sys.path.insert(0, str(SCRIPTS_DIR))

import python_uv_build_front_audit as audit  # noqa: E402


EXPECTED_PACKAGES = [
    "bloom-filter",
    "caesar-cipher",
    "directed-graph",
    "fenwick-tree",
    "graph",
    "hash-functions",
    "hash-map",
    "hyperloglog",
    "in-memory-data-store",
    "in-memory-data-store-engine",
    "ls00",
    "radix-tree",
    "resp-protocol",
    "skip-list",
    "tcp-server",
    "tree-set",
    "trie",
]


class FrontParserTests(unittest.TestCase):
    def test_parser_classifies_repeatability_pin_and_named_environment(self) -> None:
        record = audit.parse_front(
            """
            # one command per shell
            uv venv .venv --quiet --no-project
            uv pip install --python .venv -e ../hash-functions --quiet
            uv pip install --python .venv --no-deps -e .[dev] --quiet
            uv run --no-project python -m pytest tests/ -v
            """,
            platform="windows",
        )

        self.assertEqual(record["venv_command"], "uv venv .venv --quiet --no-project")
        self.assertFalse(record["has_clear"])
        self.assertTrue(record["has_no_project"])
        self.assertIsNone(record["python_pin"])
        self.assertEqual(record["test_interpreter"], "uv-run")
        self.assertTrue(record["all_pip_commands_use_named_venv"])
        self.assertFalse(record["quoted_editable"])
        self.assertEqual(record["local_dependencies"], ["hash-functions"])

    def test_parser_rejects_ambiguous_venv_commands(self) -> None:
        with self.assertRaisesRegex(audit.AuditError, "exactly one"):
            audit.parse_front(
                "uv venv .venv --no-project\nuv venv .venv --clear --no-project\n",
                platform="canonical",
            )

    def test_parser_accepts_a_pinned_explicit_front_and_deduplicates_deps(self) -> None:
        record = audit.parse_front(
            """
            uv venv .venv --quiet --no-project --clear --python=3.13
            uv pip install --python=.venv -e ../heap -e ../heap --quiet
            .venv\\Scripts\\python.exe -m pytest tests/ -v
            """,
            platform="windows",
        )

        self.assertTrue(record["has_clear"])
        self.assertEqual(record["python_pin"], "3.13")
        self.assertEqual(record["test_interpreter"], "explicit-venv")
        self.assertEqual(record["local_dependencies"], ["heap"])

    def test_parser_rejects_unknown_platform_and_classifies_other_runner(self) -> None:
        with self.assertRaisesRegex(audit.AuditError, "unsupported platform"):
            audit.parse_front("uv venv .venv", platform="plan9")

        record = audit.parse_front(
            "uv venv .venv --clear\ncustom-test-runner\n", platform="canonical"
        )
        self.assertEqual(record["test_interpreter"], "other")
        self.assertFalse(record["all_pip_commands_use_named_venv"])

    def test_helpers_fail_closed_on_missing_metadata_and_companions(self) -> None:
        with self.assertRaisesRegex(audit.AuditError, "lacks requires-python"):
            audit._requires_python("[project]\nname = 'demo'\n", "demo")

        with tempfile.TemporaryDirectory() as raw_root:
            root = Path(raw_root)
            windows = root / "code" / "packages" / "python" / "demo" / "BUILD_windows"
            windows.parent.mkdir(parents=True)
            windows.write_text("uv venv .venv --no-project\n", encoding="utf-8")
            visible = ["outside/file", windows.relative_to(root).as_posix()]
            with mock.patch.object(audit, "git_visible_paths", return_value=visible):
                with self.assertRaisesRegex(audit.AuditError, "missing BUILD"):
                    audit.build_report(root)

            outside = root / "code" / "packages" / "outside.txt"
            outside.write_text("not a Python package companion", encoding="utf-8")
            with self.assertRaisesRegex(audit.AuditError, "escapes"):
                audit._read_repo_text(root, "code/packages/python/../outside.txt")

        self.assertEqual(
            audit._package_paths(
                ["code/packages/python/../BUILD_windows"], "BUILD_windows"
            ),
            {},
        )


class RepositoryAuditTests(unittest.TestCase):
    report: dict[str, Any]
    by_package: dict[str, dict[str, Any]]

    @classmethod
    def setUpClass(cls) -> None:
        cls.report = audit.build_report(REPO_ROOT)
        cls.by_package = {row["package"]: row for row in cls.report["fronts"]}

    def test_report_finds_the_exact_non_idempotent_corpus(self) -> None:
        packages = [row["package"] for row in self.report["fronts"]]
        self.assertEqual(packages, sorted(packages))
        self.assertTrue(set(packages) <= set(EXPECTED_PACKAGES))
        self.assertEqual(self.report["schema_version"], 1)
        self.assertEqual(self.report["python_package_count"], 481)
        summary = self.report["summary"]
        self.assertEqual(summary["non_idempotent_fronts"], len(packages))
        self.assertEqual(
            summary["fronts_missing_canonical_clear"],
            sum(not row["canonical"]["has_clear"] for row in self.report["fronts"]),
        )
        self.assertEqual(
            summary["fronts_missing_windows_clear"],
            sum(not row["windows"]["has_clear"] for row in self.report["fronts"]),
        )
        self.assertEqual(
            summary["fronts_with_local_dependencies"],
            sum(
                bool(row["windows"]["local_dependencies"])
                for row in self.report["fronts"]
            ),
        )

    def test_report_preserves_dependency_order_and_platform_symmetry(self) -> None:
        expected_dependencies = {
            "bloom-filter": ["hash-functions"],
            "directed-graph": ["graph"],
            "hash-map": ["hash-functions"],
            "hyperloglog": ["hash-functions"],
            "in-memory-data-store-engine": [
                "hash-functions",
                "hyperloglog",
                "in-memory-data-store-protocol",
            ],
            "in-memory-data-store": [
                "hash-functions",
                "hyperloglog",
                "in-memory-data-store-protocol",
                "resp-protocol",
                "in-memory-data-store-engine",
            ],
            "ls00": ["json-rpc"],
            "radix-tree": ["trie"],
            "tcp-server": ["resp-protocol"],
        }
        actual = {
            package: row["windows"]["local_dependencies"]
            for package, row in self.by_package.items()
            if row["windows"]["local_dependencies"]
        }
        expected = {
            package: dependencies
            for package, dependencies in expected_dependencies.items()
            if package in self.by_package
        }

        self.assertEqual(actual, expected)
        self.assertTrue(
            all(row["local_dependency_symmetric"] for row in self.report["fronts"])
        )

    def test_report_distinguishes_generated_and_legacy_failures(self) -> None:
        caesar = self.by_package["caesar-cipher"]
        self.assertEqual(
            caesar["issues"],
            [
                "canonical-implicit-test-interpreter",
                "canonical-missing-clear",
                "canonical-missing-python-pin",
                "windows-implicit-test-interpreter",
                "windows-missing-clear",
                "windows-missing-python-pin",
            ],
        )

        ls00 = self.by_package["ls00"]
        self.assertEqual(ls00["canonical"]["test_interpreter"], "explicit-venv")
        self.assertIn("canonical-missing-no-project", ls00["issues"])
        self.assertIn("canonical-pip-without-named-venv", ls00["issues"])
        self.assertIn("windows-missing-no-project", ls00["issues"])
        self.assertIn("windows-pip-without-named-venv", ls00["issues"])

        quoted = [
            row["package"]
            for row in self.report["fronts"]
            if "windows-quoted-editable" in row["issues"]
        ]
        self.assertEqual(
            quoted, ["in-memory-data-store", "in-memory-data-store-engine"]
        )

    def test_dependency_components_are_deterministic(self) -> None:
        components_by_package = {
            row["package"]: tuple(row["dependency_component"])
            for row in self.report["fronts"]
        }
        components = set(components_by_package.values())
        self.assertEqual(
            self.report["summary"]["dependency_components"], len(components)
        )
        for package, component in components_by_package.items():
            with self.subTest(package=package):
                self.assertEqual(component, tuple(sorted(component)))
                self.assertIn(package, component)
                self.assertTrue(
                    all(
                        components_by_package[member] == component
                        for member in component
                    )
                )

        corpus = set(components_by_package)
        for row in self.report["fronts"]:
            for dependency in row["windows"]["local_dependencies"]:
                if dependency in corpus:
                    self.assertEqual(
                        components_by_package[row["package"]],
                        components_by_package[dependency],
                    )

    def test_backfill_fixture_covers_every_front_once_and_matches_state(self) -> None:
        fixture_path = (
            REPO_ROOT
            / "code"
            / "specs"
            / "fixtures"
            / "python-uv-build-front-idempotence"
            / "backfills.json"
        )
        fixture = json.loads(fixture_path.read_text(encoding="utf-8"))
        assigned = [
            package for owner in fixture["owners"] for package in owner["packages"]
        ]
        self.assertEqual(sorted(assigned), EXPECTED_PACKAGES)
        self.assertEqual(len(assigned), len(set(assigned)))

        state = json.loads(
            (REPO_ROOT / ".claude" / "package-parity-loop-state.json").read_text(
                encoding="utf-8"
            )
        )
        state_by_id = {item["id"]: item for item in state["items"]}
        for owner in fixture["owners"]:
            with self.subTest(owner=owner["id"]):
                item = state_by_id[owner["id"]]
                self.assertIn(
                    item["status"],
                    {"pending", "in-progress", "pr-open", "merged"},
                )
                self.assertEqual(item["depends_on"], owner["depends_on"])

    def test_runtime_observations_cover_every_front_without_host_paths(self) -> None:
        observation_path = (
            REPO_ROOT
            / "code"
            / "specs"
            / "fixtures"
            / "python-uv-build-front-idempotence"
            / "observations.json"
        )
        observations = json.loads(observation_path.read_text(encoding="utf-8"))
        observed = [
            package
            for receipt in observations["receipts"]
            for package in receipt["packages"]
        ]

        self.assertEqual(sorted(observed), EXPECTED_PACKAGES)
        self.assertEqual(len(observed), len(set(observed)))
        self.assertEqual(observations["platform"], "windows")
        self.assertEqual(observations["uv_version"], "0.11.28")
        self.assertNotIn("C:\\\\", json.dumps(observations))
        for receipt in observations["receipts"]:
            self.assertEqual(
                receipt["second_run"],
                {
                    "failure_command_index": 1,
                    "exit_code": 2,
                    "diagnostic": "existing-environment",
                    "interpreter": receipt["first_run"]["interpreter"],
                },
            )

    def test_markdown_is_a_rendering_of_the_same_report(self) -> None:
        markdown = audit.render_markdown(self.report)
        self.assertIn(
            f"| Non-idempotent fronts | {len(self.report['fronts'])} |", markdown
        )
        self.assertIn("| `ls00` | `>=3.11` |", markdown)
        self.assertIn("`windows-missing-no-project`", markdown)

    def test_cli_renders_both_supported_formats(self) -> None:
        for output_format, expected in (
            ("json", '"schema_version": 1'),
            ("markdown", "# Python uv BUILD-front idempotence audit"),
        ):
            with self.subTest(output_format=output_format):
                stdout = io.StringIO()
                argv = ["python_uv_build_front_audit.py", "--format", output_format]
                with (
                    mock.patch.object(sys, "argv", argv),
                    mock.patch.object(audit, "build_report", return_value=self.report),
                    contextlib.redirect_stdout(stdout),
                ):
                    self.assertEqual(audit.main(), 0)
                self.assertIn(expected, stdout.getvalue())


if __name__ == "__main__":
    unittest.main()
