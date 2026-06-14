#!/usr/bin/env python3
"""dict_lint — enforce the standard dictionary on an adj-lang program.

The dictionary (dictionary.json) is the controlled vocabulary SHARED by the input
decomposer and the rulebook/case programs. This linter is the *enforcement*: it
rejects any .adj file that uses a finding term or a hypothesis the dictionary does
not register. That is what guarantees the model's IR and the compiled program
share one vocabulary — a case's `observe csf_glucose(low)` can never silently miss
a rulebook clause because of a naming drift, because both must validate here first.

It validates two things:
  * every compound finding term `functor(value)` — the functor must be a registered
    finding and the value must be in that finding's value_domain; and
  * every hypothesis atom used as a conclusion (after `for`, after `to`, or in a
    `? query`) — it must be a registered hypothesis.

Usage:  python3 dict_lint.py <file.adj> [<file2.adj> ...]
Exit code 0 = all terms known; 1 = at least one unknown term (violations printed).
"""
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))

# adj-lang keywords + trust tiers + annotation heads — bare atoms that are NOT
# hypotheses and must be ignored when we scan for conclusion atoms.
KEYWORDS = {
    "prior", "for", "contributes", "from", "to", "interacts", "when", "and",
    "uncertain", "observe", "source", "locator", "trust",
    "consensus", "authoritative", "empirical", "inferred", "unattributed",
}


def load_dictionary(path=None):
    path = path or os.path.join(HERE, "dictionary.json")
    d = json.load(open(path))
    findings = {f["functor"]: set(f["value_domain"]) for f in d["findings"]}
    hypotheses = {h["term"] for h in d["hypotheses"]}
    return findings, hypotheses


def _strip(text):
    """Remove `% ...` comments and `"..."` string literals so their contents
    (citations, prose) never trip the term scanner."""
    text = re.sub(r'"(?:[^"\\]|\\.)*"', " ", text)        # string literals
    text = re.sub(r"(?m)%.*$", " ", text)                 # line comments
    return text


_COMPOUND = re.compile(r"\b([a-z_][a-z0-9_]*)\s*\(\s*([a-z_][a-z0-9_]*)\s*\)")
# a conclusion atom follows `for`/`to`, or opens a `?` query; it is a bare atom
# (no opening paren after it).
_CONCLUSION = re.compile(r"(?:\bfor\b|\bto\b|^\s*\?)\s+([a-z_][a-z0-9_]*)\b(?!\s*\()", re.M)


def lint(text, findings, hypotheses):
    """Return a list of violation strings (empty == clean)."""
    t = _strip(text)
    violations = []

    for functor, value in _COMPOUND.findall(t):
        if functor not in findings:
            violations.append(f"unknown finding functor: {functor}({value})")
        elif value not in findings[functor]:
            violations.append(
                f"value '{value}' not in domain of {functor} "
                f"(allowed: {sorted(findings[functor])})"
            )

    for atom in _CONCLUSION.findall(t):
        if atom in KEYWORDS:
            continue
        if atom not in hypotheses:
            violations.append(f"unknown hypothesis: {atom}")

    # de-dup, preserve order
    seen, out = set(), []
    for v in violations:
        if v not in seen:
            seen.add(v)
            out.append(v)
    return out


def main(argv):
    findings, hypotheses = load_dictionary()
    bad = 0
    for path in argv:
        text = open(path).read()
        viol = lint(text, findings, hypotheses)
        if viol:
            bad += 1
            print(f"FAIL {path}")
            for v in viol:
                print(f"  - {v}")
        else:
            print(f"ok   {path}")
    return 1 if bad else 0


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("usage: python3 dict_lint.py <file.adj> ...", file=sys.stderr)
        sys.exit(2)
    sys.exit(main(sys.argv[1:]))
