#!/usr/bin/env python3
"""Generate the FNT02 glyph-outline oracle from fontTools.

Outline extraction is transcription: `loca` offsets in one of two formats,
flags with a run-length encoding, and coordinate deltas whose width and sign
are carried by *other* bits in those flags. Every one of those is a place where
a wrong stride or an inverted bit produces something that still looks like a
glyph. A self-consistent test cannot catch it, because a hand-built fixture and
a hand-written parser share their author's misunderstanding and agree perfectly.

So the expected values here come from **fontTools**, an independent
implementation, reading real shipping fonts. Our parser never contributes a
number to this file.

## What is compared

`glyph.getCoordinates(glyf)` returns the *decoded and flattened* points: deltas
already accumulated, composites already resolved into their components' points.
That is exactly the stage our parser reaches before it emits drawing commands,
so the two are directly comparable with no interpretation on either side.

Two levels are emitted:

* `glyphs` -- a sample of glyphs recorded in full (bounds, contour ends, every
  point). When something breaks, this says *what* differs.
* `digests` -- one FNV-1a digest per glyph, for **every** glyph in the font.
  This is the coverage: 2,926 glyphs in Inter alone, too many to write out, but
  a digest per glyph still fails if any single one of them decodes wrongly.

FNV-1a rather than SHA-256 so that both sides can compute it in a few lines
with no dependency; this guards against mistakes, not adversaries.

## Regenerating

    python3 code/scripts/generate_glyph_oracle.py

Requires `fonttools` (pip install fonttools). Rerun only when adding a font or
a sampled glyph -- a diff in this file otherwise means behaviour changed.
"""

from __future__ import annotations

import pathlib
import sys

try:
    from fontTools.misc.roundTools import otRound
    from fontTools.ttLib import TTFont
except ImportError:  # pragma: no cover - developer environment issue
    sys.exit("fonttools is required: pip install fonttools")

REPO = pathlib.Path(__file__).resolve().parents[2]
OUT = REPO / "code/packages/rust/glyph-parser/tests/fixtures/oracle.txt"

# Chosen to cover both `loca` formats and the scripts Engram actually ships
# decks for. A Latin-only fixture would leave the short-offset branch and the
# heavily-composite Indic fonts untested.
FONTS = [
    ("inter", "code/fixtures/fonts/Inter-Regular.ttf"),
    ("noto-tamil", "code/learning/human-languages/_fonts/NotoSansTamil-Static.ttf"),
    ("noto-jp", "code/learning/human-languages/_fonts/NotoSansJP-Subset.ttf"),
]

FNV_OFFSET = 0xCBF29CE484222325
FNV_PRIME = 0x100000001B3
MASK = 0xFFFFFFFFFFFFFFFF


def fnv1a(text: str) -> str:
    """FNV-1a over the canonical encoding. Mirrored exactly in Rust."""
    h = FNV_OFFSET
    for byte in text.encode("utf-8"):
        h = ((h ^ byte) * FNV_PRIME) & MASK
    return f"{h:016x}"


def canonical(record: dict) -> str:
    """The one string both implementations agree to hash.

    Kept deliberately dull and explicit: any cleverness here is a second thing
    that has to be reimplemented identically in Rust.
    """
    if record["empty"]:
        return "EMPTY"
    bounds = ",".join(str(v) for v in record["bounds"])
    ends = ",".join(str(v) for v in record["end_pts"])
    points = ";".join(f"{x},{y},{on}" for x, y, on in record["points"])
    return f"{bounds}|{ends}|{points}"


def glyph_record(glyf, name: str) -> dict:
    glyph = glyf[name]

    # numberOfContours == 0 is a legitimate glyph with no outline (space).
    # It has no coordinates and, in fontTools, no bounding box attributes.
    if glyph.numberOfContours == 0:
        return {"empty": True, "bounds": [], "end_pts": [], "points": []}

    coords, end_pts, flags = glyph.getCoordinates(glyf)
    return {
        "empty": False,
        "bounds": [glyph.xMin, glyph.yMin, glyph.xMax, glyph.yMax],
        "end_pts": [int(e) for e in end_pts],
        # Bit 0 of each flag is ON_CURVE_POINT. The other bits describe how the
        # point was *encoded* and are meaningless once decoded, so they are not
        # part of the comparison.
        # otRound, NOT int(): a scaled composite yields fractional coordinates
        # (150 * 0.25 = 37.5), and int() truncates toward zero while every real
        # consumer -- fontTools included -- rounds. Truncating here would have
        # made a correct parser look wrong by one unit on exactly the glyphs
        # this fixture exists to cover.
        "points": [
            [otRound(x), otRound(y), int(f) & 1] for (x, y), f in zip(coords, flags)
        ],
    }


def sample_ids(glyf, order: list[str]) -> list[int]:
    """A spread that hits every structural case, deterministically."""
    composite, simple, empty = [], [], []
    for i, name in enumerate(order):
        glyph = glyf[name]
        if glyph.numberOfContours == 0:
            empty.append(i)
        elif glyph.isComposite():
            composite.append(i)
        else:
            simple.append(i)

    chosen = set(composite[:12]) | set(simple[:12]) | set(empty[:4])
    # Plus a spread across the id range, so a `loca` bug that only shows up at
    # large offsets is not sampled past.
    count = len(order)
    chosen |= {i * count // 12 for i in range(12)}
    chosen |= {0, count - 1}
    return sorted(i for i in chosen if i < count)


def build_synthetic_font(path: pathlib.Path) -> None:
    """Write a font exercising the composite features no shipping font here has.

    Every real font in this repository -- Inter and thirteen Noto faces, 4,149
    glyphs between them -- uses only *plain offset* composites. Not one has a
    scale, a 2x2 transform, a nested composite, or anchor-point placement. So
    those branches of the parser would be written, shipped, and never executed.

    fontTools writes this font; our parser reads it. The expectations still come
    from fontTools reading back the saved bytes, so this is a genuine
    cross-implementation check rather than a fixture agreeing with itself -- and
    reading back the *saved file* matters, because it is the compiled bytes our
    parser sees, not the in-memory objects.
    """
    from fontTools.fontBuilder import FontBuilder
    from fontTools.ttLib.tables._g_l_y_f import Glyph, GlyphComponent
    from fontTools.pens.ttGlyphPen import TTGlyphPen

    fb = FontBuilder(1000, isTTF=True)
    order = [
        ".notdef",     # required at id 0
        "box",         # a plain simple glyph, the shared component
        "curved",      # a simple glyph WITH off-curve points
        "oddneg",      # odd NEGATIVE coordinates, for the rounding tie case
        "offset",      # composite: plain translation (the case real fonts use)
        "scaled",      # composite: WE_HAVE_A_SCALE
        "xyscaled",    # composite: WE_HAVE_AN_X_AND_Y_SCALE
        "twobytwo",    # composite: WE_HAVE_A_TWO_BY_TWO (a rotation)
        "halved",      # composite: 0.5 scale onto odd negative coordinates
        "nested",      # composite whose component is itself a composite
        "anchored",    # composite placed by point matching, not by offset
        "blank",       # no outline at all
    ]
    fb.setupGlyphOrder(order)
    fb.setupCharacterMap({ord("A") + i: name for i, name in enumerate(order[1:])})

    def simple(pen_ops) -> Glyph:
        pen = TTGlyphPen(None)
        pen_ops(pen)
        return pen.glyph()

    def box(pen):
        pen.moveTo((100, 100))
        pen.lineTo((500, 100))
        pen.lineTo((500, 400))
        pen.lineTo((100, 400))
        pen.closePath()

    def oddneg(pen):
        # Halving these lands on .5 for both signs: -75 -> -37.5 and 25 -> 12.5.
        # Rounding half away from zero gives -38, rounding half up gives -37,
        # and the two agree on the positive tie -- so only a negative tie
        # distinguishes them.
        pen.moveTo((-75, -25))
        pen.lineTo((25, -75))
        pen.lineTo((-25, 75))
        pen.closePath()

    def curved(pen):
        # Two consecutive off-curve points, so the implied midpoint that
        # TrueType omits is present in the compiled bytes.
        pen.moveTo((0, 0))
        pen.qCurveTo((150, 300), (300, 0))
        pen.qCurveTo((450, -300), (600, 0), (700, 200))
        pen.closePath()

    def composite(components) -> Glyph:
        glyph = Glyph()
        glyph.numberOfContours = -1
        glyph.components = []
        for spec in components:
            component = GlyphComponent()
            component.glyphName = spec["glyph"]
            component.flags = 0
            if "points" in spec:
                component.firstPt, component.secondPt = spec["points"]
            else:
                component.x, component.y = spec["at"]
            component.transform = spec.get("transform", [[1, 0], [0, 1]])
            glyph.components.append(component)
        return glyph

    # A 30-degree rotation: not axis-aligned, so a transposed matrix (b and c
    # swapped) produces different numbers rather than the same ones.
    cos30, sin30 = 0.8660254, 0.5

    glyphs = {
        ".notdef": simple(box),
        "box": simple(box),
        "curved": simple(curved),
        "oddneg": simple(oddneg),
        "offset": composite([{"glyph": "box", "at": (250, 125)}]),
        "scaled": composite(
            [{"glyph": "box", "at": (40, 60), "transform": [[0.5, 0], [0, 0.5]]}]
        ),
        "xyscaled": composite(
            [{"glyph": "curved", "at": (10, 20), "transform": [[0.25, 0], [0, 1.5]]}]
        ),
        "twobytwo": composite(
            [
                {
                    "glyph": "curved",
                    "at": (5, 7),
                    "transform": [[cos30, sin30], [-sin30, cos30]],
                }
            ]
        ),
        # Two components, one of which is itself a composite -- so resolving
        # this one requires recursing, and the second component's contour ends
        # must be shifted past the first component's points.
        "halved": composite(
            [{"glyph": "oddneg", "at": (0, 0), "transform": [[0.5, 0], [0, 0.5]]}]
        ),
        "nested": composite(
            [
                {"glyph": "scaled", "at": (0, 0)},
                {"glyph": "box", "at": (600, 0)},
            ]
        ),
        "anchored": composite(
            [
                {"glyph": "box", "at": (0, 0)},
                # Align point 2 of what is placed so far with point 0 of `box`.
                {"glyph": "box", "points": (2, 0)},
            ]
        ),
        "blank": simple(lambda pen: None),
    }
    fb.setupGlyf(glyphs)

    advances = {name: 800 for name in order}
    fb.setupHorizontalMetrics({name: (advances[name], 0) for name in order})
    fb.setupHorizontalHeader(ascent=800, descent=-200)
    fb.setupNameTable({"familyName": "FNT02 Synthetic", "styleName": "Regular"})
    fb.setupOS2()
    fb.setupPost()
    path.parent.mkdir(parents=True, exist_ok=True)
    fb.save(str(path))


def main() -> None:
    synthetic = OUT.parent / "synthetic.ttf"
    build_synthetic_font(synthetic)
    fonts = FONTS + [
        ("synthetic", str(synthetic.relative_to(REPO))),
    ]

    lines = [
        "# FNT02 glyph oracle -- generated by code/scripts/generate_glyph_oracle.py",
        "# Every value here was produced by fontTools, never by our parser.",
        "#",
        "#   FONT <key> <path> <numGlyphs> <indexToLocFormat> <unitsPerEm>",
        "#   D <fnv1a>                      one per glyph, in glyph-id order",
        "#   G <id> EMPTY",
        "#   G <id> <xMin> <yMin> <xMax> <yMax> <endPts> <x,y,onCurve;...>",
        "#",
        "# A line format rather than JSON so the Rust test can read it with",
        "# split_whitespace() and stay dependency-free, like the crate it tests.",
    ]

    for key, rel in fonts:
        path = REPO / rel
        if not path.exists():
            sys.exit(f"missing font: {path}")

        font = TTFont(str(path))
        glyf = font["glyf"]
        order = font.getGlyphOrder()
        head = font["head"]

        lines.append(
            f"FONT {key} {rel} {font['maxp'].numGlyphs} "
            f"{head.indexToLocFormat} {head.unitsPerEm}"
        )

        records = [glyph_record(glyf, name) for name in order]
        for record in records:
            lines.append(f"D {fnv1a(canonical(record))}")

        for gid in sample_ids(glyf, order):
            record = records[gid]
            if record["empty"]:
                lines.append(f"G {gid} EMPTY")
                continue
            bounds = " ".join(str(v) for v in record["bounds"])
            ends = ",".join(str(v) for v in record["end_pts"])
            points = ";".join(f"{x},{y},{on}" for x, y, on in record["points"])
            lines.append(f"G {gid} {bounds} {ends} {points}")

        composites = sum(1 for n in order if glyf[n].isComposite())
        print(
            f"  {key:12} {len(order):5} glyphs "
            f"({composites} composite)  locaFormat={head.indexToLocFormat}"
        )

    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text("\n".join(lines) + "\n")
    print(f"wrote {OUT.relative_to(REPO)} ({OUT.stat().st_size // 1024} KiB)")


if __name__ == "__main__":
    main()
