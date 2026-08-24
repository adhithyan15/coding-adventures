from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[3]
PYTHON_PACKAGES_ROOT = REPO_ROOT / "code" / "packages" / "python"
SCRIPTS_ROOT = REPO_ROOT / "code" / "scripts"
sys.path.insert(0, str(SCRIPTS_ROOT))

import python_uv_build_front_audit as uv_audit  # noqa: E402


CANONICAL_RECIPE = [
    "uv venv .venv --quiet --no-project --clear --python 3.13",
    "uv pip install --python .venv -e ../graph -e ../directed-graph --quiet",
    "uv pip install --python .venv -e .[dev] --quiet",
    ".venv/bin/python -m ruff check src tests",
    ".venv/bin/python -m ruff format --check src tests",
    ".venv/bin/python -m mypy --strict src tests",
    ".venv/bin/python -m pytest tests/ -v",
]

WINDOWS_RECIPE = [
    "uv venv .venv --quiet --no-project --clear --python 3.13",
    "uv pip install --python .venv ../graph ../directed-graph --quiet",
    "uv pip install --python .venv -e .[dev] --quiet",
    r".venv\Scripts\python.exe -m ruff check src tests",
    r".venv\Scripts\python.exe -m ruff format --check src tests",
    r".venv\Scripts\python.exe -m mypy --strict src tests",
    r".venv\Scripts\python.exe -m pytest tests/ -v",
]


class PythonMarkovChainBuildFrontTests(unittest.TestCase):
    def test_fronts_use_the_complete_pinned_recipe(self) -> None:
        self.assertEqual(self._recipe("BUILD"), CANONICAL_RECIPE)
        self.assertEqual(self._recipe("BUILD_windows"), WINDOWS_RECIPE)

    def test_state_preserves_the_merged_graph_prerequisite(self) -> None:
        state = json.loads(
            (REPO_ROOT / ".claude" / "package-parity-loop-state.json").read_text(
                encoding="utf-8"
            )
        )
        by_id = {item["id"]: item for item in state["items"]}

        self.assertEqual(
            by_id["python-markov-chain-build-front-python313"]["depends_on"],
            ["python-graph-build-front-idempotence"],
        )
        self.assertEqual(
            by_id["python-graph-build-front-idempotence"]["status"], "merged"
        )

    def test_markov_repair_does_not_claim_a_uv_audit_reduction(self) -> None:
        report = uv_audit.build_report(REPO_ROOT)
        packages = [row["package"] for row in report["fronts"]]

        self.assertNotIn("markov-chain", packages)
        self.assertEqual(report["summary"]["non_idempotent_fronts"], 5)

    @staticmethod
    def _recipe(name: str) -> list[str]:
        return [
            line
            for line in (PYTHON_PACKAGES_ROOT / "markov-chain" / name)
            .read_text(encoding="utf-8")
            .splitlines()
            if line and not line.startswith("#")
        ]


if __name__ == "__main__":
    unittest.main()
