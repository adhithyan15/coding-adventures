#!/usr/bin/env python3
"""HL-C148 — migrate a track's schema-v1 lessons to schema v2.

WHY THIS IS THE CRITICAL PATH AND NOT BOOKKEEPING
--------------------------------------------------
It was filed as tidying. It is not. `generate:books` refuses a schema-v1 lesson
outright, so chapters 1-5 of all six Indic tracks -- 188 lessons -- cannot be
generated at all. That is why those chapters are hand-written LaTeX, which is in
turn why a new word cannot be placed in the opening of any of these books, why
Hindi's eleven writing lessons render only in the answer key, and why the script
drizzle had to start later than it should.

Every one of those follows from lessons declaring `est_minutes: 1` instead of the
v2 shape.

WHAT IS DERIVED, AND FROM WHERE
--------------------------------
    spine_node        the track's own curriculum.json, which already maps each
                      lesson to the path node it realizes. 163 of the 188 are
                      there; the rest are reported and left alone rather than
                      guessed, because a spine node is a claim about what the
                      lesson is FOR.
    duration          est_minutes x 60, floored at 120s. The declared minute was
                      always a rounded guess; the gate measures the computed
                      length anyway and reports when the two disagree.
    skills/modes/     from the lesson's `type`, following what the v2 lessons in
    strands           these same tracks already declare for each type.
    register/variety  neutral / standard-colloquial, matching every v2 lesson in
                      the six tracks.
    knowledge atoms   ONE per lesson, named for the lesson. See below.

ONE ATOM PER LESSON, WHICH IS A FLOOR AND SAYS SO
--------------------------------------------------
A v2 lesson declares what it introduces, and the gates measure gentleness against
that. Deriving the true atom set from prose is not something a script can do
honestly -- a lesson that teaches a word AND a sound AND a grammatical point has
three, and only a reader can say so.

So this assigns exactly one, named after the lesson, and that is deliberately an
UNDER-count. The direction matters: under-counting makes a chapter look gentler
than it is, so it cannot cause a false alarm, and the atom-budget gates stay
honest for the lessons that were already v2. Splitting these into their real
atoms is authoring work, and it is what the level gate will need before any of
these chapters can claim a rung.

The `hl-knowledge` directives follow the same rule: the first teaching section
introduces the atom, the practice and recall sections assess it, and everything
else claims nothing.
"""

import json
import os
import re
import sys

from sharded_ledger import load_curriculum

HERE = os.path.dirname(os.path.abspath(__file__))
HL = os.path.normpath(os.path.join(HERE, "..", ".."))

# type -> (skills, modes, strands), copied from what the v2 lessons in these six
# tracks already declare rather than invented here.
BY_TYPE = {
    "word": (["listening", "speaking", "reading"],
             ["interpretive", "interpersonal", "presentational"],
             ["meaning-input", "meaning-output"]),
    "phrase": (["listening", "speaking", "reading"],
               ["interpretive", "interpersonal", "presentational"],
               ["meaning-input", "meaning-output"]),
    "writing": (["reading", "writing"],
                ["interpretive", "presentational"],
                ["language-focus"]),
    "grammar": (["listening", "speaking", "reading"],
                ["interpretive", "presentational"],
                ["language-focus"]),
    "etymology": (["reading"], ["interpretive"], ["language-focus"]),
    "culture": (["listening", "reading"], ["interpretive"], ["meaning-input"]),
    "practice": (["listening", "speaking"],
                 ["interpersonal", "presentational"],
                 ["meaning-output", "fluency"]),
}
DEFAULT = BY_TYPE["word"]

# Sections that never introduce and never assess: they are exposition.
PROSE = ("sounds you'll need", "taken apart", "grammar lens", "why it's said this way",
         "roots you now carry", "across the family", "the atoms")
ASSESS = ("guided practice", "your turn", "wrap-up recall", "before you move on")

ATOM_KIND = {"word": "LEX", "phrase": "LEX", "writing": "SCRIPT",
             "grammar": "GRAMMAR", "etymology": "ROOT", "culture": "CULTURE",
             "practice": "LEX"}


def spine_nodes(track):
    doc = load_curriculum(HL, track)
    out = {}
    for node in doc.get("path", []):
        for lid in node.get("lessons", []):
            out[lid] = node["spine_node"]
    return out


def atom_for(lid, ltype):
    """`TA-C01-vanakkam` -> `TA-LEX-C01-VANAKKAM-01`, the shape the v2 lessons
    in these tracks already use."""
    parts = lid.split("-")
    prefix, chapter = parts[0], parts[1]
    slug = "-".join(parts[2:]).upper().replace("_", "-")
    slug = re.sub(r"[^A-Z0-9-]", "", slug) or "MAIN"
    return f"{prefix}-{ATOM_KIND.get(ltype, 'LEX')}-{chapter}-{slug}-01"


def migrate(track, apply=False):
    nodes = spine_nodes(track)
    d = os.path.join(HL, track, "lessons")
    done = skipped = 0
    for f in sorted(os.listdir(d)):
        if not f.endswith(".md"):
            continue
        path = os.path.join(d, f)
        text = open(path, encoding="utf-8").read()
        if re.search(r"^schema_version: 2", text, re.M):
            continue
        end = text.find("\n---", 3)
        fm, body = text[4:end], text[end + 4:]
        lid = re.search(r"^id: (\S+)", fm, re.M).group(1)
        ltype = (re.search(r"^type: (\S+)", fm, re.M) or [None, "word"])[1] \
            if re.search(r"^type: (\S+)", fm, re.M) else "word"
        node = nodes.get(lid)
        if node is None:
            print(f"  {lid:<30} SKIPPED — no spine node in curriculum.json")
            skipped += 1
            continue

        minutes = re.search(r"^est_minutes: (\d+)", fm, re.M)
        seconds = max(120, int(minutes.group(1)) * 60) if minutes else 180
        atom = atom_for(lid, ltype)
        skills, modes, strands = BY_TYPE.get(ltype, DEFAULT)
        lst = lambda xs: "[" + ", ".join(xs) + "]"

        fm = re.sub(r"^est_minutes: \d+\n", "", fm, flags=re.M)
        fm = f"schema_version: 2\n" + fm.lstrip("\n")
        if "spine_node:" not in fm:
            fm = re.sub(r"^(id: .*)$", r"\1\nspine_node: " + node, fm, count=1, flags=re.M)
        addition = (f"duration:\n  max_seconds: {seconds}\n"
                    f"requires:\n  knowledge: []\n"
                    f"introduces:\n  knowledge: [{atom}]\n"
                    f"practises:\n  knowledge: [{atom}]\n"
                    f"skills: {lst(skills)}\nmodes: {lst(modes)}\nstrands: {lst(strands)}\n"
                    f"register: neutral\nvariety: standard-colloquial\n")
        fm = fm.rstrip("\n") + "\n" + addition

        # Directives: the first non-warm-up, non-prose section introduces; the
        # practice and recall sections assess; exposition claims nothing.
        out, introduced = [], False
        for line in body.split("\n"):
            out.append(line)
            m = re.match(r"^## (.+)$", line)
            if not m:
                continue
            low = m.group(1).lower()
            if any(p in low for p in PROSE) or low.startswith("warm-up"):
                out.append("<!-- hl-knowledge: introduces=[]; assesses=[] -->")
            elif any(a in low for a in ASSESS):
                out.append(f"<!-- hl-knowledge: introduces=[]; assesses=[{atom}] -->")
            elif not introduced:
                out.append(f"<!-- hl-knowledge: introduces=[{atom}]; assesses=[] -->")
                introduced = True
            else:
                out.append(f"<!-- hl-knowledge: introduces=[]; assesses=[{atom}] -->")
        body = "\n".join(out)
        if not introduced:
            # No teaching section: put the introduction on the warm-up rather
            # than leaving the atom introduced nowhere, which fails the gate.
            body = body.replace("<!-- hl-knowledge: introduces=[]; assesses=[] -->",
                                f"<!-- hl-knowledge: introduces=[{atom}]; assesses=[] -->", 1)

        result = "---\n" + fm + "---\n" + body
        assert "\x00" not in result, lid
        if apply:
            open(path, "w", encoding="utf-8").write(result)
        done += 1
    return done, skipped


if __name__ == "__main__":
    apply = "--apply" in sys.argv
    tracks = [a for a in sys.argv[1:] if not a.startswith("--")] or ["tamil"]
    for t in tracks:
        print(f"== {t}")
        done, skipped = migrate(t, apply)
        print(f"   {done} migrated, {skipped} skipped for a hand-chosen spine node")
    if not apply:
        print("\nDry run. Re-run with --apply to write.")
