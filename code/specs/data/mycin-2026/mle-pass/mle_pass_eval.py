"""MLE-PASS multi-hop recall harness — score two-hop board questions answered purely on
the CPU from grounded edges, with ZERO model calls.

The reasoning step past single-hop recall: a board question like "leukocoria points to a
disease caused by a mutation in which gene?" needs TWO grounded hops —
  hop 1  clue → disease     (an organ-system recall library: ophtho / neuro / collagen / …)
  hop 2  disease → gene     (the genetics library)
joined on the shared disease. We express the join as an adj-lang **rule body** and let the
engine's SLD resolver do it:

    import "<hop1 library>"
    import "genetics-edges.adj"
    rule {
        head: clue_to_gene($X, $G)
        when: <hop1 relation>($X, $D), gene_defect($D, $G)
    }
    ? clue_to_gene(<clue>, $G)

The engine returns the `$G` binding and — crucially — the citing clause of EACH hop, so a
correct answer is defended by both spans (multi-hop byte-provenance). The harness reads the
binding, maps it to the printed gene option, and scores correct / abstain / wrong. It never
asks a model anything; all knowledge lives in the imported grounded libraries.

Defensibility metric: `multihop_coverage` = fraction of CORRECT answers that cite BOTH hops
(≥2 authoritative citing clauses). That is the number a future grounding PR moves, and the
proof that the answer is a genuine two-hop derivation, not a one-edge coincidence.

Import-root note: adj-lang forbids `..` in import paths (an import cannot escape its file's
directory), so a query that imports the recall libraries must sit BESIDE them. The harness
therefore assembles each run in a temp directory: it copies in the two needed `*-edges.adj`
files and writes the query there with sibling imports, runs the CLI, then discards the temp
dir — leaving the shipped recall/ library untouched.
"""
from __future__ import annotations

import json
import os
import shutil
import subprocess
import tempfile
from dataclasses import dataclass
from pathlib import Path

HERE = Path(__file__).resolve().parent
RECALL = HERE.parent / "recall"
RUST = HERE.parents[3] / "packages" / "rust"


def _find_cli() -> Path | None:
    """Locate the adj-lang-cli binary (env override, then debug/release build dirs)."""
    override = os.environ.get("ADJ_LANG_CLI")
    if override and Path(override).exists():
        return Path(override)
    for cand in (RUST / "target" / "debug" / "adj-lang-cli",
                 RUST / "target" / "release" / "adj-lang-cli"):
        if cand.exists():
            return cand
    return None


_CLI = _find_cli()


def build_query(item: dict) -> str:
    """The two-hop join program for one item: import both libraries, define the chaining
    rule (hop1 relation × gene_defect, joined on the disease), and ask the binding query."""
    return (
        f'import "{item["hop1_lib"]}"\n'
        f'import "{item["hop2_lib"]}"\n'
        "rule {\n"
        "    head: clue_to_gene($X, $G)\n"
        f'    when: {item["hop1_relation"]}($X, $D), {item["hop2_relation"]}($D, $G)\n'
        "}\n"
        f'? clue_to_gene({item["clue"]}, $G)\n'
    )


@dataclass
class RunResult:
    binding: str | None       # the gene the engine bound (or None if it abstained)
    citations: int            # number of citing clauses (≥2 ⇒ both hops defended)


def run_item(item: dict, cli: Path | None = None) -> RunResult:
    """Run one item's two-hop query through the engine in an isolated temp dir."""
    cli = cli or _CLI
    if cli is None:
        raise RuntimeError("adj-lang-cli not built (cargo build -p adj-lang-cli)")
    with tempfile.TemporaryDirectory() as td:
        tdp = Path(td)
        # Copy the two grounded libraries beside the query so sibling imports resolve.
        # Each must be a BARE filename (no path separators / parent refs) — a structural
        # guard so a library name can only ever name a file directly inside recall/, never
        # path-traverse out of it, regardless of where items.json comes from.
        for lib in (item["hop1_lib"], item["hop2_lib"]):
            if "/" in lib or "\\" in lib or ".." in lib:
                raise ValueError(f"library name must be a bare filename, got {lib!r}")
            shutil.copy(RECALL / lib, tdp / lib)
        query = tdp / "query.adj"
        query.write_text(build_query(item))
        out = subprocess.run([str(cli), str(query)], capture_output=True, text=True)
    doc = json.loads(out.stdout)
    recall = doc.get("recall") or []
    if not recall or not recall[0].get("answers"):
        return RunResult(binding=None, citations=0)
    ans = recall[0]["answers"][0]
    return RunResult(binding=ans["bindings"].get("G"), citations=len(ans.get("citations", [])))


def letter_for(item: dict, gene: str | None) -> str | None:
    """Map a bound gene to its printed option letter, or None if it is not an option."""
    if gene is None:
        return None
    for letter, value in item["options"].items():
        if value == gene:
            return letter
    return None


def score(items: list[dict], cli: Path | None = None) -> dict:
    """Run every item; return per-item outcomes and the aggregate scoreboard."""
    results, correct, abstained, wrong, both_hops = [], 0, 0, 0, 0
    for item in items:
        r = run_item(item, cli=cli)
        picked = letter_for(item, r.binding)
        if picked is None:
            outcome = "abstained"
            abstained += 1
        elif picked == item["gold_letter"]:
            outcome = "correct"
            correct += 1
            if r.citations >= 2:
                both_hops += 1
        else:
            outcome = "wrong"
            wrong += 1
        results.append({
            "id": item["id"], "outcome": outcome, "picked": picked,
            "binding": r.binding, "citations": r.citations,
        })
    total = len(items)
    return {
        "total": total,
        "correct": correct,
        "abstained": abstained,
        "wrong": wrong,
        # defensibility: of the correct answers, how many cite BOTH grounded hops.
        "multihop_coverage": (both_hops / correct) if correct else 0.0,
        "results": results,
    }


def load_items(path: Path | None = None) -> list[dict]:
    path = path or (HERE / "items.json")
    return json.loads(path.read_text())["items"]


if __name__ == "__main__":
    if _CLI is None:
        raise SystemExit("adj-lang-cli not built — run: cargo build -p adj-lang-cli")
    board = score(load_items())
    print(json.dumps({k: v for k, v in board.items() if k != "results"}, indent=2))
    for r in board["results"]:
        print(f"  {r['id']}: {r['outcome']:9} picked={r['picked']} "
              f"binding={r['binding']} hops_cited={r['citations']}")
