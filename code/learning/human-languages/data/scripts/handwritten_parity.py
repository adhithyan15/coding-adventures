#!/usr/bin/env python3
"""HL-C134 — what a handwritten chapter would lose by being generated.

THE PROBLEM THIS EXISTS TO PREVENT
-----------------------------------
Chapters 1-5 of all six Indic tracks are hand-written LaTeX, listed in
`core/book-generation.json`'s `handwritten` block and skipped by the generator.
Moving them into the pipeline is the unblocker for almost everything else: it is
why the earliest place a new lesson can go is chapter 6, why Hindi's eleven
writing lessons render only in the answer key, and why the drizzle had to be
placed later than it should be.

The tempting way to do it is to move the entry from `handwritten` to `targets`
and regenerate. That would silently delete prose. The generated chapter is built
from the lesson markdown, and the hand-written `.tex` says MORE than the markdown
does -- it has `sounds` blocks, `cousinweb` etymologies, `grammarlens`
explanations and `cognates` tables that were written straight into the LaTeX and
never went back into the lesson.

Measured per chapter: **88 blocks** across the six tracks. Eighty-eight pieces of
the owner's writing that a one-line config change would have thrown away, with
every gate still green -- because no gate compares the two.

The first run reported 56. That was a per-TRACK aggregate, which lets a surplus
in one chapter cancel a deficit in another -- and chapters are flipped one at a
time, so the cancellation is meaningless. Per chapter, the way the migration will
actually happen, it is 88.

WHAT IS COMPARED, AND WHY IT IS COUNTS AND NOT TEXT
---------------------------------------------------
For each handwritten chapter: how many prose blocks the `.tex` contains, against
how many the same chapter's lessons would produce if generated. The mapping is
the renderer's own, from `parse.ts`:

    \\begin{sounds}       <-  "## Sounds you'll need ..."
    \\begin{cousinweb}    <-  "## The word, taken apart ..."
    \\begin{grammarlens}  <-  "## Grammar lens ..."
    \\begin{culture}      <-  "## Why it's said this way ..."

Counts rather than a text diff, deliberately. The prose was edited on its way
into LaTeX -- rewrapped, re-punctuated, sometimes rephrased -- so a text diff
would report every block as different and say nothing. A count answers the only
question that matters before flipping the switch: *is anything about to
disappear?*

This does NOT check that the surviving prose is the same prose. That check is a
human reading the two chapters side by side, and it is what the migration
tranche has to do per chapter. This is the gate that says which chapters are
safe to flip and which are not yet.

REPORT-ONLY, ON PURPOSE
------------------------
It fails on 88 blocks of pre-existing debt today. Per the HL05 and HL08
precedent, a gate that fails on inherited debt teaches authors to route around
it, so this prints and exits 0. It becomes blocking per track as that track's
prose is carried across -- at which point the number for that track is zero and
staying zero is a real promise.
"""

import os
import re
import sys

from sharded_ledger import load_book_generation

HERE = os.path.dirname(os.path.abspath(__file__))
HL = os.path.normpath(os.path.join(HERE, "..", ".."))

# environment -> the lowercased heading that produces it, per parse.ts
BLOCKS = {
    "sounds": "sounds you'll need",
    "cousinweb": "taken apart",
    "grammarlens": "grammar lens",
    "culture": "why it's said this way",
}

TRACKS = ["tamil", "telugu", "kannada", "malayalam", "hindi", "sanskrit"]


def lessons_by_chapter(track):
    """Lessons grouped by chapter, read from the frontmatter rather than the
    filename -- a lesson id names the chapter it was WRITTEN for, and lessons
    get re-placed."""
    out = {}
    d = os.path.join(HL, track, "lessons")
    for f in sorted(os.listdir(d)):
        if not f.endswith(".md"):
            continue
        text = open(os.path.join(d, f), encoding="utf-8").read()
        m = re.search(r"^chapter: (\d+)", text, re.M)
        if not m:
            continue
        out.setdefault(int(m.group(1)), []).append((f[:-3], text))
    return out


def main():
    config = load_book_generation(HL)
    total = 0
    print("Prose blocks a handwritten chapter holds, against what its lessons would")
    print("produce if it were generated. A positive gap is prose that would be LOST.\n")
    print(f"{'track':<11}{'ch':>3}{'tex':>6}{'md':>5}{'gap':>6}   at risk")
    for track in TRACKS:
        by_chapter = lessons_by_chapter(track)
        for entry in [e for e in config["handwritten"] if e["language"] == track]:
            path = os.path.join(HL, entry["output"])
            if not os.path.exists(path):
                print(f"{track:<11}{entry['chapter']:>3}   .tex missing: {entry['output']}")
                continue
            tex = open(path, encoding="utf-8").read()
            tex_counts = {e: len(re.findall(r"\\begin\{%s\}" % e, tex)) for e in BLOCKS}
            md_counts = dict.fromkeys(BLOCKS, 0)
            for _, body in by_chapter.get(entry["chapter"], []):
                low = body.lower()
                for env, heading in BLOCKS.items():
                    md_counts[env] += low.count(f"## {heading}") or (
                        1 if heading in low else 0)
            gap = {e: tex_counts[e] - md_counts[e] for e in BLOCKS if tex_counts[e] > md_counts[e]}
            n = sum(gap.values())
            total += n
            if n:
                at_risk = ", ".join(f"{e}x{v}" for e, v in gap.items())
                print(f"{track:<11}{entry['chapter']:>3}{sum(tex_counts.values()):>6}"
                      f"{sum(md_counts.values()):>5}{n:>6}   {at_risk}")
    print(f"\n{total} block(s) of hand-written prose would be dropped by generating "
          "these chapters as they stand.")
    print("Report-only: this exits 0 so it can ship against pre-existing debt.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
