"""MLE-PASS multi-hop recall harness — score two-hop board questions answered purely on
the CPU from grounded edges, with ZERO model calls.

The reasoning step past single-hop recall: a board question like "leukocoria points to a
disease caused by a mutation in which gene?" needs TWO grounded hops —
  hop 1  clue → middle entity   (an organ-system recall library: ophtho / neuro / micro / …)
  hop 2  middle entity → answer (gene / inheritance pattern / Gram stain / morphology / …)
joined on the shared middle entity. We express the join as an adj-lang **rule body** and let
the engine's SLD resolver do it:

    import "<hop1 library>"
    import "<hop2 library>"
    rule {
        head: clue_to_answer($X, $A)
        when: <hop1 relation>($X, $D), <hop2 relation>($D, $A)
    }
    ? clue_to_answer(<clue>, $A)

The engine returns the `$A` binding and — crucially — the citing clause of EACH hop, so a
correct answer is defended by both spans (multi-hop byte-provenance). The harness reads the
binding, maps it to the printed option, and scores correct / abstain / wrong. It never asks a
model anything; all knowledge lives in the imported grounded libraries. Hop 1 may run in
reverse (`"hop1_reverse"`) when the clue is the relation's second argument — e.g. microbiology
`causes(organism, disease)`, where the *disease* is the clue and the *organism* is the middle
entity whose Gram stain / morphology is the answer (the original MYCIN organism-ID chain).

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
    rule (hop1 relation joined to hop2 relation on the shared middle entity `$D`), and ask the
    binding query. The answer variable is `$A` (it may be a gene, an inheritance pattern, a
    Gram stain, …, depending on the hop2 relation).

    Hop-1 direction. By default the clue is hop1's FIRST argument and the middle entity its
    second: `rel1($X, $D)` (e.g. `eye_finding_indicates(clue, disease)`). Some chains run the
    other way: the clue is hop1's SECOND argument and the middle entity its first — e.g.
    `causes(organism, disease)`, where the clue is the *disease* and the middle entity is the
    *organism* to be found. Setting `"hop1_reverse": true` emits `rel1($D, $X)` for that case.
    Either way the join variable `$D` is what hop2 consumes (`rel2($D, $A)`), so the engine's
    SLD resolver does the join regardless of argument order — relations are bidirectional.

    Imports are de-duplicated: when both hops draw on the same library (e.g. microbiology's
    `causes` and `gram_stain` both live in `micro-edges.adj`) it is imported once.
    """
    libs = [item["hop1_lib"]]
    if item["hop2_lib"] not in libs:
        libs.append(item["hop2_lib"])
    imports = "".join(f'import "{lib}"\n' for lib in libs)
    hop1 = (f'{item["hop1_relation"]}($D, $X)' if item.get("hop1_reverse")
            else f'{item["hop1_relation"]}($X, $D)')
    return (
        imports
        + "rule {\n"
        "    head: clue_to_answer($X, $A)\n"
        f'    when: {hop1}, {item["hop2_relation"]}($D, $A)\n'
        "}\n"
        f'? clue_to_answer({item["clue"]}, $A)\n'
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
        # Copy the grounded libraries beside the query so sibling imports resolve (deduped —
        # a chain whose two hops share one library copies it once). Each must be a BARE
        # filename (no path separators / parent refs) — a structural guard so a library name
        # can only ever name a file directly inside recall/, never path-traverse out of it,
        # regardless of where items.json comes from.
        for lib in dict.fromkeys((item["hop1_lib"], item["hop2_lib"])):
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
    return RunResult(binding=ans["bindings"].get("A"), citations=len(ans.get("citations", [])))


def letter_for(item: dict, gene: str | None) -> str | None:
    """Map a bound gene to its printed option letter, or None if it is not an option."""
    if gene is None:
        return None
    for letter, value in item["options"].items():
        if value == gene:
            return letter
    return None


def score(items: list[dict], cli: Path | None = None) -> dict:
    """Run every item; return per-item outcomes and the aggregate scoreboard.

    Two item kinds:
      • answerable — gold the printed option matching the engine's binding; binding must
        match `gold_letter` and (for defensibility) cite both hops;
      • abstention (`expect_abstain`) — the chain is ungrounded, so the engine MUST bind
        nothing: abstaining is CORRECT, binding any option is WRONG (a fabrication).
    """
    results, correct, abstained_ok, wrong, both_hops, answerable_correct = [], 0, 0, 0, 0, 0
    for item in items:
        r = run_item(item, cli=cli)
        picked = letter_for(item, r.binding)
        if item.get("expect_abstain"):
            # abstaining (no binding) is the correct, non-fabricating answer.
            if r.binding is None:
                outcome = "correct"
                correct += 1
                abstained_ok += 1
            else:
                outcome = "wrong"  # fabricated an answer for an ungrounded chain
                wrong += 1
        elif picked is None:
            outcome = "abstained"  # an answerable item the engine failed to resolve
            wrong += 1             # counts against us (it was answerable)
        elif picked == item["gold_letter"]:
            outcome = "correct"
            correct += 1
            answerable_correct += 1
            if r.citations >= 2:
                both_hops += 1
        else:
            outcome = "wrong"
            wrong += 1
        results.append({
            "id": item["id"], "outcome": outcome, "picked": picked,
            "binding": r.binding, "citations": r.citations,
            "expect_abstain": bool(item.get("expect_abstain")),
        })
    return {
        "total": len(items),
        "correct": correct,
        "abstained_correctly": abstained_ok,
        "wrong": wrong,
        # defensibility: of the correct ANSWERABLE items, how many cite BOTH grounded hops.
        "multihop_coverage": (both_hops / answerable_correct) if answerable_correct else 0.0,
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
