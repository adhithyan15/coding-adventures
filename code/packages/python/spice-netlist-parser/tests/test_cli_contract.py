import json
import subprocess
import sys
from pathlib import Path

from spice_netlist_parser import CLI_RESULT_SCHEMA_VERSION, run_netlist_json


def _cli_case() -> dict[str, object]:
    corpus_path = Path(__file__).resolve().parents[4] / "grammars/spice/berkeley-v1-cli-corpus.json"
    corpus = json.loads(corpus_path.read_text(encoding="utf-8"))
    assert corpus["schemaVersion"] == 1
    assert corpus["suite"] == "berkeley-v1-cli"
    return corpus["cases"][0]


def test_run_netlist_json_uses_the_shared_berkeley_cli_contract() -> None:
    case = _cli_case()
    payload = json.loads(run_netlist_json(case["deck"]))

    assert payload["schemaVersion"] == CLI_RESULT_SCHEMA_VERSION
    assert payload["title"] == case["expected"]["title"]
    assert [analysis["kind"] for analysis in payload["analyses"]] == case["expected"]["analysisKinds"]
    assert [len(analysis["records"]) for analysis in payload["analyses"]] == case["expected"][
        "recordCounts"
    ]


def test_module_cli_runs_a_deck_file_as_json(tmp_path: Path) -> None:
    case = _cli_case()
    deck_path = tmp_path / "core.cir"
    deck_path.write_text(case["deck"], encoding="utf-8")

    completed = subprocess.run(
        [sys.executable, "-m", "spice_netlist_parser.cli", "run", "--json", str(deck_path)],
        check=False,
        capture_output=True,
        text=True,
    )

    assert completed.returncode == 0, completed.stderr
    assert json.loads(completed.stdout)["title"] == case["expected"]["title"]
