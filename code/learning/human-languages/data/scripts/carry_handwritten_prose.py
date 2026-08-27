#!/usr/bin/env python3
"""HL-C134 — carry the prose out of a handwritten chapter and into its lessons.

`handwritten_parity.py` measures what generating a handwritten chapter would
delete. This is the tool that stops it being deleted: it reads the hand-written
`.tex`, finds the blocks each lesson's markdown does not have, converts them back
to markdown, and inserts them in the right place.

WHAT MAKES THIS SAFE TO AUTOMATE, AND WHAT DOES NOT
----------------------------------------------------
Safe: FINDING the blocks and knowing which lesson each belongs to. A `\\section`
is one lesson, its `\\label{lesson:X}` names it, and the environments inside it
are unambiguous.

Not safe, and therefore not attempted: deciding whether the result reads well.
LaTeX prose is not markdown prose -- it is line-wrapped for a book column, it
uses `---` where markdown wants an em dash, and its tables are `tabular`. This
converts the mechanical parts and leaves the block where a human can see it. The
parity check then says nothing was lost; a reader says whether it landed well.

THE TWO MATCHING BUGS THIS FILE IS WRITTEN AROUND
--------------------------------------------------
`\\section` takes an optional argument -- `\\section[short]{long}` -- used when a
title is too long for the running head. Splitting on `\\section{` silently merges
that lesson into the previous one, which is how the first attempt at this
reported Tamil's வணக்கம் as holding eight prose blocks: it had swallowed நன்றி.

And a label is matched against the lesson id's TAIL, exactly, never with
`endswith`. This corpus has already been bitten by that: the book label `naam`
matched `HI-W04-ra-sa-mera-naam`, pairing a lesson with someone else's prose.
"""

import os
import re

from sharded_ledger import load_book_generation
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
HL = os.path.normpath(os.path.join(HERE, "..", ".."))

# environment -> (markdown heading, where it goes relative to other sections)
# environment -> (heading to WRITE, fragment to DETECT).
#
# The two differ, and conflating them duplicates prose. `parse.ts` maps any
# heading containing "taken apart" to an etymology block, so a lesson that
# already says "The phrase, taken apart" has one -- but a check for the exact
# string "The word, taken apart" does not find it, and the carry adds a second.
# That is what happened to ML-C03-sukhamaano on the first pass: parity reported
# the chapter as safe, because parity matched the fragment, while the carry
# matched the full heading. Detection is on the fragment, always.
BLOCKS = {
    "sounds": ("Sounds you'll need", "sounds you'll need"),
    "cousinweb": ("The word, taken apart", "taken apart"),
    "grammarlens": ("Grammar lens", "grammar lens"),
    "culture": ("Why it's said this way", "why it's said this way"),
}


def sections(tex):
    """Split a chapter into (label, body) per lesson.

    Handles `\\section[short]{long}` as well as `\\section{...}`. Getting this
    wrong does not error -- it merges two lessons and silently attributes one
    lesson's prose to another.
    """
    out = []
    marks = [m.start() for m in re.finditer(r"\\section\s*[\[{]", tex)]
    for i, start in enumerate(marks):
        end = marks[i + 1] if i + 1 < len(marks) else len(tex)
        body = tex[start:end]
        label = re.search(r"\\label\{lesson:([^}]+)\}", body)
        out.append((label.group(1) if label else None, body))
    return out


def blocks_in(body):
    """Every prose block in one section, as (environment, inner text)."""
    found = []
    for env in BLOCKS:
        for m in re.finditer(r"\\begin\{%s\}(.*?)\\end\{%s\}" % (env, env), body, re.S):
            found.append((env, m.group(1).strip()))
    return found


def to_markdown(text):
    """LaTeX prose to markdown, for the constructs these chapters actually use.

    Deliberately narrow. Anything not handled is left alone and shows up as
    LaTeX in the lesson, which is visible and fixable -- unlike a clever
    conversion that quietly changes what a sentence says.
    """
    t = text
    # ORDER MATTERS, and getting it wrong leaves raw LaTeX on the page rather
    # than erroring. The inner accent macros go first: `\emph{va-\d{n}ak-kam}`
    # has braces inside its argument, so an `\emph\{([^{}]*)\}` pattern skips
    # it entirely and the reader gets `\emph{va-ṇak-kam}` in their lesson.
    # Accent macros, in both their braced and bare forms. The bare form is the
    # one that gets missed: `\=a` has no braces, so a `\\=\{([a-z])\}` pattern
    # walks straight past it and the reader is shown `\=aṇ` in their lesson.
    ACCENTS = {"d": "\u0323", "=": "\u0304", ".": "\u0307", "u": "\u0306",
               "'": "\u0301", "~": "\u0303", "c": "\u0327"}
    for mac, comb in ACCENTS.items():
        t = re.sub(r"\\%s\{([a-zA-Z])\}" % re.escape(mac),
                   lambda m, c=comb: m.group(1) + c, t)
        t = re.sub(r"\\%s([a-zA-Z])" % re.escape(mac),
                   lambda m, c=comb: m.group(1) + c, t)
    # Script-font wrappers, named explicitly. The first version matched
    # `\\t[a-z]{1,2}` and so handled \ta and \te but silently left \ml and \kn.
    t = re.sub(r"\\(ta|te|kn|ml|dv|sk|hi|ar|bn|gu|pa|mr|ur|fa|ru|he|zh|ja)\{([^{}]*)\}",
               r"\2", t)
    t = t.replace("$+$", "+").replace("$=$", "=").replace("$\\to$", "\u2192")
    # Then the wrappers, repeatedly, so a nesting one level deeper still resolves.
    for _ in range(3):
        t = re.sub(r"\\emph\{([^{}]*)\}", r"*\1*", t)
        t = re.sub(r"\\textbf\{([^{}]*)\}", r"**\1**", t)
    t = t.replace("``", "\u201c").replace("''", "\u201d")
    t = re.sub(r"(?<!-)---(?!-)", "\u2014", t)
    t = re.sub(r"\\label\{[^}]*\}", "", t)
    t = re.sub(r"\n{3,}", "\n\n", t)
    return t.strip()


def carry(track, chapter, apply=False):
    config = load_book_generation(HL)
    entry = next((e for e in config["handwritten"]
                  if e["language"] == track and e["chapter"] == chapter), None)
    if entry is None:
        print(f"{track} ch{chapter} is not a handwritten chapter")
        return 0

    tex = open(os.path.join(HL, entry["output"]), encoding="utf-8").read()
    lessons = {}
    d = os.path.join(HL, track, "lessons")
    for f in sorted(os.listdir(d)):
        if not f.endswith(".md"):
            continue
        text = open(os.path.join(d, f), encoding="utf-8").read()
        m = re.search(r"^chapter: (\d+)", text, re.M)
        i = re.search(r"^id: (\S+)", text, re.M)
        if m and int(m.group(1)) == chapter and i:
            lessons[i.group(1)] = (os.path.join(d, f), text)

    carried = 0
    for label, body in sections(tex):
        if label is None:
            continue
        # Match on the id's TAIL, exactly. Never endswith: `naam` would match
        # `HI-W04-ra-sa-mera-naam` and steal another lesson's prose.
        # A label is either the lesson's whole id (`TA-W01-curves-va-ka`) or just
        # its tail (`vanakkam`). Both forms appear in these chapters. Compared
        # exactly against each, never with endswith -- the book label `naam` once
        # matched `HI-W04-ra-sa-mera-naam` and gave one lesson another's prose.
        target = None
        for lid, (path, text) in lessons.items():
            tail = lid.split("-", 2)[-1] if lid.count("-") >= 2 else lid
            # A recap section is labelled for the CHAPTER rather than the lesson
            # -- `ta-greetings-practice` against a lesson id of `TA-C01-practice`
            # -- so it needs its own rule. Narrow and explicit on purpose: both
            # sides must end in the same recap word, which is not the same thing
            # as a substring match and cannot pair two unrelated lessons.
            recap = (label.rsplit("-", 1)[-1] == tail
                     and tail in ("practice", "recap", "review"))
            if label == lid or label == tail or recap:
                target = (lid, path, text)
                break
        if target is None:
            print(f"  ch{chapter} label '{label}' -> no lesson (recap/practice section)")
            continue
        lid, path, text = target
        low = text.lower()
        for env, inner in blocks_in(body):
            heading, fragment = BLOCKS[env]
            if fragment in low:
                continue
            md = to_markdown(inner)
            section = f"\n## {heading}\n<!-- hl-knowledge: introduces=[]; assesses=[] -->\n\n{md}\n"
            # Before Guided Practice if there is one, else at the end: the prose
            # blocks are all exposition and belong ahead of the doing.
            anchor = re.search(r"\n## (Guided Practice|Your turn|Wrap-up Recall)", text)
            text = (text[:anchor.start()] + section + text[anchor.start():]) if anchor else text + section
            lessons[lid] = (path, text)
            carried += 1
            print(f"  {lid:<28} + {heading}")
            if apply:
                open(path, "w", encoding="utf-8").write(text)
    return carried


if __name__ == "__main__":
    apply = "--apply" in sys.argv
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    track = args[0] if args else "tamil"
    chapters = [int(a) for a in args[1:]] or [1, 2, 3, 4, 5]
    total = 0
    for ch in chapters:
        print(f"== {track} chapter {ch}")
        total += carry(track, ch, apply)
    print(f"\n{total} block(s) {'carried' if apply else 'would be carried'}")
    if not apply:
        print("Dry run. Re-run with --apply to write.")
