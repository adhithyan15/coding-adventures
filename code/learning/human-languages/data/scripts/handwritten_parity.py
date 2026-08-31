#!/usr/bin/env python3
"""HL-C134 — what a handwritten chapter would lose by being generated.

THE PROBLEM THIS EXISTS TO PREVENT
-----------------------------------
69 chapters across 14 tracks are hand-written LaTeX, listed in the
`handwritten` block of the book-generation ledger and skipped by the generator.
Moving them into the pipeline is the unblocker for almost everything else: it is
why the earliest place a new lesson can go is chapter 6, why Hindi's eleven
writing lessons render only in the answer key, and why the drizzle had to be
placed later than it should be. The standing directive is to retire ALL of them,
so this measures all of them.

The tempting way to do it is to move the entry from `handwritten` to `targets`
and regenerate. That would silently delete prose. The generated chapter is built
from the lesson markdown, and the hand-written `.tex` says MORE than the markdown
does -- it has `sounds` blocks, `cousinweb` etymologies, `grammarlens`
explanations and `cognates` tables that were written straight into the LaTeX and
never went back into the lesson.

Measured per chapter: **217 blocks** across 14 tracks. Two hundred and seventeen
pieces of the owner's writing that a one-line config change would have thrown
away, with every gate still green -- because no gate compares the two.

Three earlier numbers were all too small, each for an instructive reason.

The first run reported 56: a per-TRACK aggregate, which lets a surplus in one
chapter cancel a deficit in another -- and chapters are flipped one at a time, so
the cancellation is meaningless. Per chapter it was 88.

But 88 was scoped to a hardcoded list of six Indic tracks, written when those
were the only ones anyone meant to migrate. That excluded 44 of the 69
handwritten chapters, including all 32 French and German ones. Corpus-wide it
was 140. The track list now comes from the ledger and cannot drift again.

And 140 still counted only four environments -- sounds, cousinweb, grammarlens,
culture -- because those are the ones the generator emits. A handwritten chapter
is not limited to what the generator can emit; that is why it is handwritten.
`cognates`, `morphologybox`, `usage`, `scriptstep`, `checkpoint`, `etymology` and
`rootweb` appear in these chapters and in NO generator source path, so all 77 of
them vanish on flip with no heading to carry across. Counting them takes the
figure to 217 -- and moves persian and urdu, whose chapters are built almost
entirely from `usage`/`scriptstep`/`checkpoint`/`rootweb`, from a reported
"nothing would be lost" straight to blocked. The lesson generalises: an allowlist
of what the TARGET format supports is the wrong instrument for measuring what the
SOURCE would lose.

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
It fails on 217 blocks of pre-existing debt today. Per the HL05 and HL08
precedent, a gate that fails on inherited debt teaches authors to route around
it, so a bare run prints and exits 0.

`--check [track ...]` is the blocking form, and it is how the retirement
proceeds: once a track's prose is carried across, wire `--check <track>` into CI
so that track's zero is a promise rather than a snapshot. Four tracks -- italian,
portuguese, punjabi, sanskrit -- already pass it, and a track with no handwritten
chapters left passes trivially, so the gate survives its own success.
"""

import os
import re
import sys

from sharded_ledger import load_book_generation

HERE = os.path.dirname(os.path.abspath(__file__))
HL = os.path.normpath(os.path.join(HERE, "..", ".."))

# environment -> how parse.ts's classifyBlock recognises the heading that
# produces it. `prefix` matches title.startswith, `contains` matches an
# includes. Mirrors classifyBlock exactly, aliases included.
BLOCKS = {
    "sounds": {"prefix": ("sounds you'll need",)},
    "cousinweb": {"contains": ("taken apart",)},
    "grammarlens": {"prefix": ("grammar lens", "the adjective sibling",
                               "its two tags")},
    "culture": {"prefix": ("why it's said this way",)},
}

# Environments the generator can actually produce. Anything else in a
# handwritten chapter is prose the pipeline has no way to emit, so flipping that
# chapter deletes it outright -- there is not even a heading to carry across.
EMITTABLE = set(BLOCKS)

# Structural LaTeX, not prose. Present in generated output too, so their
# appearance in a handwritten chapter says nothing about lost writing.
LAYOUT = {
    "center", "tabular", "tabularx", "itemize", "enumerate", "quote",
    "description", "figure", "table", "minipage", "verbatim", "flushleft",
    "flushright", "small", "footnotesize", "tcolorbox",
}

def handwritten_tracks(config):
    """Every track that still has a handwritten chapter, read from the ledger.

    This used to be a hardcoded list of the six Indic tracks, written when those
    were the only ones anybody intended to migrate. That silently excluded 44 of
    the 69 handwritten chapters -- including all 32 French and German ones, the
    two largest holdings -- so the headline "N blocks at risk" was never a
    corpus-wide number. Retiring handwritten books entirely means every track has
    to be measured, so the list comes from the ledger and cannot drift again.
    """
    return sorted({e["language"] for e in config["handwritten"]})


def md_block_counts(body):
    """Blocks the renderer would build from one lesson, counted its way.

    parse.ts splits on a line that -- after trimStart -- begins with "## " and
    not "### ", then classifies the TITLE. The previous version here counted
    `low.count(f"## {heading}") or (1 if heading in low else 0)` over the whole
    lowercased file, which counts a passing mention in prose as a block. For
    `cousinweb` the key is the bare substring "taken apart" while the real
    heading is "## The word, taken apart", so the count arm was always 0 and the
    mention arm always fired: FR-C02-je-mappelle scored an etymology block for
    the sentence "each already taken apart on its own". Overcounting the
    markdown side SHRINKS the gap, which on this gate means wrongly reporting a
    chapter safe to retire.
    """
    counts = dict.fromkeys(BLOCKS, 0)
    for line in body.splitlines():
        stripped = line.lstrip()
        if not stripped.startswith("## ") or stripped.startswith("### "):
            continue
        title = stripped[3:].strip().lower()
        for env, rule in BLOCKS.items():
            if any(title.startswith(p) for p in rule.get("prefix", ())) or any(
                    c in title for c in rule.get("contains", ())):
                counts[env] += 1
                break
    return counts


def unportable_blocks(tex):
    """Prose environments in the .tex the generator cannot emit at all.

    `cognates`, `morphologybox`, `usage`, `scriptstep`, `checkpoint`,
    `etymology` and `rootweb` appear in handwritten chapters and in NO generator
    source path. They were invisible to the four-environment allowlist, so
    persian and urdu -- whose chapters are built almost entirely from them --
    reported zero blocks and passed --check, while holding 8 and 9 blocks of
    exactly the writing this script exists to protect.
    """
    found = {}
    for env in re.findall(r"\\begin\{([a-z]+)\}", tex):
        if env in EMITTABLE or env in LAYOUT:
            continue
        found[env] = found.get(env, 0) + 1
    return found


def lessons_by_chapter(track):
    """Lessons grouped by chapter, read from the frontmatter rather than the
    filename -- a lesson id names the chapter it was WRITTEN for, and lessons
    get re-placed."""
    out = {}
    d = os.path.join(HL, track, "lessons")
    if not os.path.isdir(d):
        return out
    for f in sorted(os.listdir(d)):
        if not f.endswith(".md"):
            continue
        text = open(os.path.join(d, f), encoding="utf-8").read()
        m = re.search(r"^chapter: (\d+)", text, re.M)
        if not m:
            continue
        out.setdefault(int(m.group(1)), []).append((f[:-3], text))
    return out


def main(argv=None):
    argv = list(sys.argv[1:] if argv is None else argv)
    check = "--check" in argv
    if check:
        argv.remove("--check")
    bad_flags = sorted(a for a in argv if a.startswith("-"))
    if bad_flags:
        print(f"unknown option(s): {', '.join(bad_flags)}")
        print("usage: handwritten_parity.py [--check] [track ...]")
        return 2
    wanted = set(argv)

    config = load_book_generation(HL)
    registered = {e["language"] for e in config["targets"]} | set(
        handwritten_tracks(config))
    tracks = handwritten_tracks(config)
    if wanted:
        unknown = wanted - registered
        if unknown:
            print(f"unknown track(s): {', '.join(sorted(unknown))}")
            return 2
        # A requested track with no handwritten chapters left is the GOAL state,
        # not an error. Wiring `--check <track>` into CI per the workflow above
        # used to go red at the exact moment that track was successfully
        # retired, which would teach everyone to remove the gate.
        retired = sorted(wanted - set(tracks))
        if retired:
            print(f"already retired, nothing handwritten remains: "
                  f"{', '.join(retired)}")
        tracks = [t for t in tracks if t in wanted]
        if not tracks:
            return 0

    total = 0
    per_track = {}
    tex_blocks = {}
    unmeasured = {}
    chapters = 0
    print("Prose blocks a handwritten chapter holds, against what its lessons would")
    print("produce if it were generated. A positive gap is prose that would be LOST.\n")
    print(f"{'track':<11}{'ch':>3}{'tex':>6}{'md':>5}{'gap':>6}   at risk")
    for track in tracks:
        by_chapter = lessons_by_chapter(track)
        for entry in [e for e in config["handwritten"] if e["language"] == track]:
            chapters += 1
            path = os.path.join(HL, entry["output"])
            if not os.path.isfile(path):
                # NEVER a quiet skip. Before, this `continue`d without touching
                # per_track, so the track fell into the "clean" bucket and
                # --check exited 0 -- a renamed chapter or a drifted ledger path
                # turned straight into a green gate for chapters nobody read.
                print(f"{track:<11}{entry['chapter']:>3}   UNMEASURED: .tex missing "
                      f"({entry['output']})")
                unmeasured.setdefault(track, []).append(entry["chapter"])
                continue
            with open(path, encoding="utf-8") as fh:
                tex = fh.read()
            tex_counts = {e: len(re.findall(r"\\begin\{%s\}" % e, tex)) for e in BLOCKS}
            md_counts = dict.fromkeys(BLOCKS, 0)
            for _, body in by_chapter.get(entry["chapter"], []):
                for env, value in md_block_counts(body).items():
                    md_counts[env] += value
            gap = {e: tex_counts[e] - md_counts[e] for e in BLOCKS if tex_counts[e] > md_counts[e]}
            stranded = unportable_blocks(tex)
            n = sum(gap.values()) + sum(stranded.values())
            total += n
            per_track[track] = per_track.get(track, 0) + n
            tex_blocks[track] = tex_blocks.get(track, 0) + sum(
                tex_counts.values()) + sum(stranded.values())
            if n:
                at_risk = ", ".join(
                    f"{e}x{v}" for e, v in list(gap.items()) + sorted(stranded.items()))
                print(f"{track:<11}{entry['chapter']:>3}{sum(tex_counts.values()):>6}"
                      f"{sum(md_counts.values()):>5}{n:>6}   {at_risk}")

    # An unmeasured track is NOT a clean one.
    clean = [t for t in tracks
             if not per_track.get(t) and t not in unmeasured]
    print(f"\n{chapters} handwritten chapter(s) across {len(tracks)} track(s) examined.")
    print(f"{total} block(s) of hand-written prose would be dropped by generating "
          "these chapters as they stand.")
    if clean:
        carried = [t for t in clean if tex_blocks.get(t)]
        empty = [t for t in clean if not tex_blocks.get(t)]
        print(f"\nNOTHING WOULD BE LOST ({len(clean)}):")
        if carried:
            print(f"  prose already carried into the lessons: {', '.join(carried)}")
        if empty:
            print(f"  no prose blocks in the .tex at all: {', '.join(empty)}")
            print("    ^ trivially safe on THIS measure, which is a weaker claim. A")
            print("      chapter with no prose blocks may still be handwritten for a")
            print("      reason this script cannot see -- a custom table, a figure, an")
            print("      ordering the generator cannot produce. Read it before flipping.")
        print("  Still needs a side-by-side read before the flip: this measures")
        print("  only whether a prose BLOCK disappears, never whether the")
        print("  surviving prose says the same thing.")
    if unmeasured:
        print("UNMEASURED ({}): {}".format(
            len(unmeasured),
            ", ".join(f"{t} (ch {', '.join(str(c) for c in v)})"
                      for t, v in sorted(unmeasured.items()))))
        print("  -- the .tex could not be read, so nothing is known about these. "
              "Fix the path before flipping anything in these tracks.")
    dirty = sorted(t for t in tracks if per_track.get(t))
    if dirty:
        print(f"BLOCKED ({len(dirty)}): " + ", ".join(
            f"{t} ({per_track[t]})" for t in dirty))
        print("  -- carry the missing prose into the lessons first.")

    if check:
        # Blocking mode, per track. Once a track's prose is carried across, wire
        # `--check <track>` into CI so its zero is a promise and not a snapshot.
        if unmeasured:
            print(f"\nFAIL: {sum(len(v) for v in unmeasured.values())} chapter(s) "
                  f"could not be measured: {', '.join(sorted(unmeasured))}")
            return 1
        if total:
            print(f"\nFAIL: {total} block(s) at risk in {', '.join(dirty)}")
            return 1
        print("\nOK: nothing would be lost by generating these chapters.")
        return 0
    print("Report-only without --check: exits 0 so it can ship against "
          "pre-existing debt.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
