#!/usr/bin/env python3
"""Live decompose->solve demonstration for the adj-lang constraint sublanguage
(ADJ constraints track D2).

The ONE model touchpoint per case decomposes a messy-prose word problem into an
adj-lang program. The deterministic CPU engine (`adj-lang-cli`) then SOLVES it.
The model never computes the answer; every answer is the engine's, at zero
answer-time model calls.

Zero third-party deps: the model is called over Ollama's HTTP `/api/generate`
endpoint with `urllib` only. Run with `python3 run.py [model]` (default
`llama3.1:8b`). Re-runs the model and overwrites `results/`.
"""

import json
import os
import re
import subprocess
import sys
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
RESULTS = os.path.join(HERE, "results")
CASES = os.path.join(HERE, "cases", "cases.json")
# adj-lang-cli debug binary, relative to this file.
CLI = os.path.normpath(
    os.path.join(HERE, "..", "..", "..", "packages", "rust", "target", "debug", "adj-lang-cli")
)
OLLAMA = "http://localhost:11434/api/generate"
# Case ids become filenames and a CLI argument, so constrain them to the
# lowercase-snake convention the grammar already uses — no `/`, `..`, or leading
# `-` can reach a path or the subprocess arg vector (defence in depth: cases.json
# is a committed fixture, but the SPEC invites editing it).
CID_RE = re.compile(r"^[a-z0-9_]+$")

# The decompose-only system instruction: the adj-lang CONSTRAINT grammar, the
# decompose-only discipline, and the rule that the model must NOT state an answer.
GRAMMAR = """\
You translate a word problem into a tiny constraint language called adj-lang.
You DO NOT solve it. You DO NOT compute or state any numeric answer. A separate
deterministic engine solves the program you write. Output ONLY the program.

The language has exactly these statements (one per line):

  symbol <name> : scalar          # declare an unknown (lowercase snake_case name)
  constrain <expr> <op> <expr>    # an (in)equality the unknowns must satisfy
  solve for { <name>, <name> }    # ask for the value(s) of the unknown(s)
  check                           # ask only whether the constraints are satisfiable
  minimize <expr>                 # find the unknowns minimizing <expr>
  maximize <expr>                 # find the unknowns maximizing <expr>

  <op>   is one of:  <=  >=  <  >  =  !=
  <expr> is arithmetic over symbols and numbers: +  -  *  /  and ( ).
         Always put spaces around operators:  3 * x + 2 * y   (NOT 3*x).

Rules:
- Declare every unknown with `symbol` before using it.
- Use ONE of `solve for {...}` / `check` / `minimize <expr>` / `maximize <expr>`,
  matching what the problem asks ("how many / what value" -> solve for;
  "is it possible / feasible" -> check; "most / maximize" -> maximize;
  "least / minimum / cheapest" -> minimize).
- Encode EVERY limit in the problem as a `constrain` line, including non-negativity
  (a count or amount that cannot go below zero -> `constrain <name> >= 0`).
- Do NOT write the answer. Do NOT add comments or prose. Output only the program.
"""


def ollama(model, prompt):
    """Call Ollama /api/generate, temperature 0, return the response text."""
    body = json.dumps(
        {
            "model": model,
            "prompt": prompt,
            "stream": False,
            "options": {"temperature": 0, "seed": 7},
        }
    ).encode()
    req = urllib.request.Request(OLLAMA, data=body, headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=180) as r:
        return json.loads(r.read())["response"]


def extract_program(text):
    """Pull the adj-lang program out of the model's reply: strip ``` fences and
    keep only lines that start with a known statement keyword."""
    lines = []
    for raw in text.splitlines():
        line = raw.strip()
        if line.startswith("```") or not line:
            continue
        kw = line.split(" ", 1)[0]
        if kw in ("symbol", "constrain", "solve", "check", "minimize", "maximize"):
            lines.append(line)
    return "\n".join(lines) + "\n"


def run_cli(adj_path):
    """Run adj-lang-cli on a program file; return (ok, parsed_json_or_error)."""
    out = subprocess.run([CLI, adj_path], capture_output=True, text=True)
    try:
        return out.returncode == 0, json.loads(out.stdout)
    except json.JSONDecodeError:
        return False, {"error": out.stdout or out.stderr}


def score(case, engine):
    """Compare the ENGINE output to the case GOLD. Returns (passed, detail)."""
    gold = case["gold"]
    if "optimize_value" in gold:
        opt = engine.get("optimize", {})
        if opt.get("outcome") != "optimal":
            return False, f"expected optimal, got {opt.get('outcome')}"
        got = opt.get("value")
        return abs(got - gold["optimize_value"]) < 1e-6, f"value {got} vs {gold['optimize_value']}"
    if "solve_value" in gold:
        sol = engine.get("solve", {})
        if sol.get("outcome") not in ("solved", "solved_roots"):
            return False, f"expected solved, got {sol.get('outcome')}"
        want = gold["solve_value"]
        for a in sol.get("assignments", []):
            if abs(a["value"] - want["value"]) < 1e-6:
                return True, f"{a['name']}={a['value']} matches {want['value']}"
        return False, f"no assignment == {want['value']} in {sol.get('assignments')}"
    if "check_outcome" in gold:
        got = engine.get("check", {}).get("outcome")
        return got == gold["check_outcome"], f"check {got} vs {gold['check_outcome']}"
    return False, "unscoreable gold"


def main():
    model = sys.argv[1] if len(sys.argv) > 1 else "llama3.1:8b"
    cases = json.load(open(CASES))["cases"]
    os.makedirs(RESULTS, exist_ok=True)

    decompose_calls = 0
    answer_time_calls = 0  # the engine never calls a model
    rows = []
    for case in cases:
        cid = case["id"]
        if not CID_RE.match(cid):
            raise ValueError(f"invalid case id {cid!r}: must match ^[a-z0-9_]+$")
        prompt = GRAMMAR + "\n\nWORD PROBLEM:\n" + case["prose"] + "\n\nadj-lang program:\n"
        reply = ollama(model, prompt)
        decompose_calls += 1
        program = extract_program(reply)
        adj_path = os.path.join(RESULTS, cid + ".adj")
        open(adj_path, "w").write(program)

        ok, engine = run_cli(adj_path)
        json.dump(engine, open(os.path.join(RESULTS, cid + ".json"), "w"), indent=2)
        passed, detail = (False, "did not compile") if not ok else score(case, engine)
        rows.append(
            {"id": cid, "solver_path": case["solver_path"], "compiled": ok,
             "passed": passed, "detail": detail}
        )
        print(f"[{'PASS' if passed else 'fail'}] {cid:22s} {detail}")

    summary = {
        "model": model,
        "n_cases": len(cases),
        "decompose_calls": decompose_calls,
        "answer_time_model_calls": answer_time_calls,
        "passed": sum(r["passed"] for r in rows),
        "cases": rows,
    }
    json.dump(summary, open(os.path.join(RESULTS, "summary.json"), "w"), indent=2)
    print(
        f"\n{summary['passed']}/{summary['n_cases']} solved by the engine | "
        f"decompose calls = {decompose_calls} | answer-time model calls = {answer_time_calls}"
    )


if __name__ == "__main__":
    main()
