"""Create disposable canonical and contradictory DB01 fixture seeds."""

from __future__ import annotations

import copy
import json
import sys
from pathlib import Path


def write_seeds(canonical_path: Path, destination: Path) -> None:
    canonical = json.loads(canonical_path.read_text(encoding="utf-8"))
    cases = {"canonical.json": canonical}

    wrong_read = copy.deepcopy(canonical)
    read = wrong_read["operations"][-2]
    assert read["op"] == "read" and read["result"] == "0000"
    read["result"] = "ffff"
    cases["wrong-late-read.json"] = wrong_read

    wrong_error = copy.deepcopy(canonical)
    erase = wrong_error["operations"][-1]
    assert erase["op"] == "erase_block" and erase["result"] is None
    del erase["result"]
    erase["error"] = {
        "type": "ProgramError",
        "message": "page has already been programmed since erase",
    }
    cases["program-error-on-erase.json"] = wrong_error

    wrong_count = copy.deepcopy(canonical)
    counts = wrong_count["states_after_operations"][-1]["erase_counts"]
    assert counts == [1, 1]
    counts[1] = 0
    cases["wrong-final-erase-count.json"] = wrong_count

    unknown = copy.deepcopy(canonical)
    unknown["operations"][-1]["op"] = "read_cache"
    cases["unknown-operation.json"] = unknown

    destination.mkdir(parents=True, exist_ok=True)
    for name, document in cases.items():
        (destination / name).write_text(
            json.dumps(document, ensure_ascii=False, separators=(",", ":")) + "\n",
            encoding="utf-8",
        )


if __name__ == "__main__":
    if len(sys.argv) != 3:
        raise SystemExit("usage: generate_fixture_fuzz_seeds.py CANONICAL_JSON DESTINATION")
    write_seeds(Path(sys.argv[1]), Path(sys.argv[2]))
