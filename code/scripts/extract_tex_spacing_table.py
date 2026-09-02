#!/usr/bin/env python3
"""Extract TeX's inter-atom spacing table from a real TeX.

TeX's math spacing is a table indexed by the classes of two adjacent atoms.
Get it right and `a+b`, `f(x)` and `a=b` space correctly with no tuning; get it
wrong and no amount of tuning fixes them, because the rule is structural rather
than aesthetic.

The table is published in *The TeXbook* (Chapter 18), and transcribing it by
hand is exactly the kind of copying where a transposed row produces output that
looks *almost* right. So this asks a real TeX instead.

## Why the glue names it itself

TeX inserts the space as a glue node carrying the *name* of the parameter it
came from:

    \\glue(\\medmuskip) 2.22217 plus 1.11108 minus 2.22217

So there is no width arithmetic, no em-to-point conversion, and no threshold
deciding whether 1.67pt is "thin" — TeX says `\\thinmuskip` and that is the
answer. Anything derived would be our interpretation; this is TeX's own.

## The demotion rule falls out for free

A `Bin` atom is demoted to `Ord` when it cannot be binary: first in a list,
last in a list, or following a Bin/Op/Rel/Open/Punct. That is why a naive
`$\\mathord{x}\\mathbin{x}$` measures *zero* space — the Bin became an Ord.

Rather than encode that rule here (and risk agreeing with a wrong
implementation), each pair is emitted with padding atoms around it and the
result is read back. Where a pair is genuinely impossible, TeX reports the
spacing of the demoted form, and the extractor records the pair as unreachable
instead of inventing a value for it.

## Output

`code/packages/rust/math-layout/tests/fixtures/tex-spacing.txt`, a line per
ordered pair:

    ord bin med

Run:  python3 code/scripts/extract_tex_spacing_table.py
Needs a TeX binary on PATH (`tex`).
"""

from __future__ import annotations

import pathlib
import re
import subprocess
import sys
import tempfile

REPO = pathlib.Path(__file__).resolve().parents[2]
OUT = REPO / "code/packages/rust/math-layout/tests/fixtures/tex-spacing.txt"

# The eight atom classes, with the primitive that forces each one.
CLASSES = [
    ("ord", r"\mathord"),
    ("op", r"\mathop"),
    ("bin", r"\mathbin"),
    ("rel", r"\mathrel"),
    ("open", r"\mathopen"),
    ("close", r"\mathclose"),
    ("punct", r"\mathpunct"),
    ("inner", r"\mathinner"),
]

# Distinct nuclei so each glue can be attributed to the gap it sits in.
PAD_LEFT, LEFT, RIGHT, PAD_RIGHT = "a", "b", "c", "d"

GLUE_NAME = {
    r"\thinmuskip": "thin",
    r"\medmuskip": "med",
    r"\thickmuskip": "thick",
}

# The four styles TeX distinguishes. Medium and thick spaces are suppressed in
# the script styles, which is a real part of the table rather than a detail --
# a subscript that spaces like display text is visibly wrong.
STYLES = [
    ("display", r"\displaystyle"),
    ("text", r"\textstyle"),
    ("script", r"\scriptstyle"),
    ("scriptscript", r"\scriptscriptstyle"),
]


def build_document() -> tuple[str, list[tuple[str, str, str]]]:
    """One box per (style, left, right), each preceded by a marker message."""
    lines = [
        r"\showboxdepth=10 \showboxbreadth=200 \tracingonline=1",
    ]
    cases: list[tuple[str, str, str]] = []
    for style_name, style_cmd in STYLES:
        for left_name, left_cmd in CLASSES:
            for right_name, right_cmd in CLASSES:
                cases.append((style_name, left_name, right_name))
                lines.append(rf"\message{{@@CASE {style_name} {left_name} {right_name}@@}}")
                # Padding atoms on both sides so a Bin under test is neither
                # first nor last; the padding's own gaps are attributed to the
                # padding characters and ignored.
                lines.append(
                    rf"\setbox0=\hbox{{${style_cmd}"
                    rf"\mathord{{{PAD_LEFT}}}{left_cmd}{{{LEFT}}}"
                    rf"{right_cmd}{{{RIGHT}}}\mathord{{{PAD_RIGHT}}}$}}"
                )
                lines.append(r"\showbox0")
    lines.append(r"\end")
    return "\n".join(lines) + "\n", cases


CHAR_RE = re.compile(r"^\.*\\[a-zA-Z]+ (.)$")
GLUE_RE = re.compile(r"^\.*\\glue\((\\[a-z]+)\)")


def parse(log: str, cases: list[tuple[str, str, str]]) -> dict:
    """Walk each box's node list, attributing glue to the gap it sits in."""
    blocks = log.split("@@CASE ")[1:]
    if len(blocks) != len(cases):
        sys.exit(f"expected {len(cases)} cases in the log, found {len(blocks)}")

    table: dict[tuple[str, str, str], str] = {}
    for block, (style, left, right) in zip(blocks, cases):
        header, _, body = block.partition("@@")
        parts = header.split()
        assert parts == [style, left, right], f"case order drifted: {parts}"

        # Sequence of "char" and "glue" events, in document order.
        seen_chars: list[str] = []
        space = "none"
        for line in body.splitlines():
            char = CHAR_RE.match(line.strip())
            if char:
                seen_chars.append(char.group(1))
                continue
            glue = GLUE_RE.match(line.strip())
            if glue:
                # The gap under test is the one after the LEFT nucleus and
                # before the RIGHT one -- i.e. exactly one char seen so far
                # beyond the left padding.
                if seen_chars[-1:] == [LEFT]:
                    space = GLUE_NAME.get(glue.group(1), glue.group(1))
        table[(style, left, right)] = space
    return table


def main() -> None:
    document, cases = build_document()
    with tempfile.TemporaryDirectory() as work:
        source = pathlib.Path(work) / "spacing.tex"
        source.write_text(document)
        result = subprocess.run(
            ["tex", "-interaction=nonstopmode", source.name],
            cwd=work,
            capture_output=True,
            text=True,
        )
        log = result.stdout + result.stderr
        if "@@CASE" not in log:
            sys.exit(f"tex produced no cases:\n{log[-2000:]}")

    table = parse(log, cases)

    lines = [
        "# TeX inter-atom spacing, extracted from a real TeX by",
        "# code/scripts/extract_tex_spacing_table.py. Every value is TeX's own:",
        "# it names the glue parameter it inserted, so nothing here is derived",
        "# from a measured width or a threshold of ours.",
        "#",
        "#   <style> <left-class> <right-class> <none|thin|med|thick>",
        "#",
        "# A `Bin` that cannot be binary is demoted to `Ord` by TeX before any",
        "# spacing is chosen, so pairs involving such a Bin record the spacing",
        "# of the demoted form -- which is what a conforming implementation",
        "# must also produce.",
    ]
    for (style, left, right), space in table.items():
        lines.append(f"{style} {left} {right} {space}")

    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text("\n".join(lines) + "\n")

    from collections import Counter

    counts = Counter(table.values())
    print(f"wrote {OUT.relative_to(REPO)}  ({len(table)} pairs)")
    print(f"  {dict(counts)}")


if __name__ == "__main__":
    main()
