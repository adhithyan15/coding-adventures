#!/usr/bin/env python3
"""ADJ81 — end-to-end small-model deployment: extract facts -> emit program -> run library.

Tests Adhithya's bet: a 0.5B that does ONLY fact extraction can drive a pre-compiled
rulebook ("library") to the correct answer, with reasoning done deterministically on CPU.

Two arms:
  A) FRAMEWORK-EMITS: 0.5B slot-extracts the SCHEMA facts (single natural questions);
     the FRAMEWORK templates a program that imports the library + calls it; execute.
  B) MODEL-WRITES:    0.5B is asked to WRITE the whole program freehand; try to run it.

Hypothesis: A is correct & robust (model's only job = extraction, its proven strength,
ADJ78); B fails (freehand code-gen is the rigid-schema task that choked the 0.5B, ADJ77).
"""
import json
import os
import re
import subprocess
import sys
import tempfile
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
GEN = "http://127.0.0.1:11434/api/generate"

CASES = [
    {"id": "C1", "passage": "Jordan is a part-time employee who was hired in March 2022.", "expect": 12},
    {"id": "C2", "passage": "Alex works full-time and was hired in 2022.", "expect": 20},
    {"id": "C3", "passage": "Sam is a part-time employee, hired back in 2018.", "expect": 20},
    {"id": "C4", "passage": "Priya joined as a part-time staff member in 2021.", "expect": 12},
    {"id": "C5", "passage": "Morgan has been a full-time employee since 2015.", "expect": 20},
]


def gen(model, prompt, npred=60, timeout=120):
    body = json.dumps({"model": model, "prompt": prompt, "stream": False,
                       "options": {"temperature": 0, "seed": 0, "num_predict": npred}}).encode()
    req = urllib.request.Request(GEN, data=body, headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=timeout) as r:
        return json.loads(r.read())["response"]


def extract_facts(model, passage):
    """Single natural question per SCHEMA slot. Categorical slots use COPY-THE-PHRASE
    (the 0.5B's strength + byte-anchorable), NOT forced either/or (which biases the
    0.5B to the default option -- see ADJ81 debug)."""
    et_out = gen(model, f"Passage: {passage}\n\nCopy the exact words from the passage that describe "
                        "the person's employment status (such as full-time or part-time).")
    el = et_out.lower()
    et = "part_time" if "part" in el else ("full_time" if "full" in el else None)
    hy_out = gen(model, f"Passage: {passage}\n\nWhat year was the person hired? Answer with a 4-digit year only.")
    m = re.search(r"\b(19|20)\d\d\b", hy_out)
    hy = int(m.group()) if m else None
    return {"employment_type": et, "hire_year": hy}


def arm_framework_emits(model, case):
    facts = extract_facts(model, case["passage"])
    if facts["hire_year"] is None:
        return None, facts, "extraction failed (no year)"
    # FRAMEWORK deterministically emits the program from the extracted facts
    program = (f"import sys; sys.path.insert(0, {HERE!r})\n"
               f"from leave_library import leave_days\n"
               f"print(leave_days(employment_type={facts['employment_type']!r}, hire_year={facts['hire_year']}))\n")
    with tempfile.NamedTemporaryFile("w", suffix=".py", delete=False) as f:
        f.write(program); path = f.name
    try:
        out = subprocess.run([sys.executable, path], capture_output=True, text=True, timeout=30)
        ans = out.stdout.strip()
        return (int(ans) if ans.lstrip("-").isdigit() else None), facts, out.stderr.strip()[:80]
    finally:
        os.unlink(path)


def arm_model_writes(model, case):
    prompt = (f"You have a Python module leave_library with: def leave_days(employment_type, hire_year)\n"
              f"where employment_type is 'full_time' or 'part_time'.\n"
              f"Case: {case['passage']}\n\n"
              f"Write a complete Python program that imports leave_days from leave_library and prints "
              f"the result for this case. Output ONLY code, no explanation.")
    code = gen(model, prompt, npred=200)
    code = re.sub(r"^```(python)?|```$", "", code.strip(), flags=re.M).strip()
    # ensure the library is importable
    code = f"import sys; sys.path.insert(0, {HERE!r})\n" + code
    with tempfile.NamedTemporaryFile("w", suffix=".py", delete=False) as f:
        f.write(code); path = f.name
    try:
        out = subprocess.run([sys.executable, path], capture_output=True, text=True, timeout=30)
        nums = re.findall(r"-?\d+", out.stdout)
        return (int(nums[-1]) if nums else None), code[:120], out.stderr.strip().splitlines()[-1][:90] if out.stderr else ""
    finally:
        os.unlink(path)


def main():
    model = sys.argv[1] if len(sys.argv) > 1 else "qwen2.5:0.5b"
    a_ok = b_ok = 0
    print(f"model={model}\n")
    for c in CASES:
        a_ans, facts, a_err = arm_framework_emits(model, c)
        b_ans, b_code, b_err = arm_model_writes(model, c)
        a_correct = a_ans == c["expect"]; b_correct = b_ans == c["expect"]
        a_ok += a_correct; b_ok += b_correct
        print(f"{c['id']} expect={c['expect']:2}  A(framework-emits)={a_ans} {'OK' if a_correct else 'X'} "
              f"facts={facts}   B(model-writes)={b_ans} {'OK' if b_correct else 'X'} {('err:'+b_err) if b_err else ''}")
    n = len(CASES)
    print(f"\nArm A (framework emits program from 0.5B-extracted facts): {a_ok}/{n} correct")
    print(f"Arm B (0.5B writes the program freehand):                   {b_ok}/{n} correct")


if __name__ == "__main__":
    main()
