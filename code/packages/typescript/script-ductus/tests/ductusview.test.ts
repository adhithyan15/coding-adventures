import { describe, expect, it } from "vitest";
import { DUCTUS, penPathD, type LetterDuctus } from "../src/strokes";
import {
  ductusFilmstrip,
  ductusFor,
  ductusFrame,
  ductusSteps,
  escapeXml,
  isSafeName,
  segmentEndFractions,
  svgMarkup,
  viewBoxFor,
  wrapCaption,
  type GlyphOutline,
  type SvgNode,
} from "../src/ductusview";
import { tamilOutline } from "./support/font-fixtures";
import { byTag, collect } from "./support/svg-tree";

const MA = DUCTUS["ம"];
const outline = tamilOutline("ம");

describe("ductusFor — only cited letters have a ductus", () => {
  it("finds every authored identity in its own scope", () => {
    for (const letter of Object.values(DUCTUS) as LetterDuctus[]) {
      expect(ductusFor(letter.sequence ?? letter.glyph, letter.script)).toBe(
        letter,
      );
    }
  });

  it("returns undefined for a letter nobody has authored a stroke order for", () => {
    expect(ductusFor("A")).toBeUndefined();
    expect(ductusFor("")).toBeUndefined();
  });

  it("does not mistake inherited Object properties for letters", () => {
    // DUCTUS is a plain object, so `DUCTUS["toString"]` is a FUNCTION, not a
    // letter. A naive lookup would hand that to the renderer and crash.
    expect(ductusFor("toString")).toBeUndefined();
    expect(ductusFor("constructor")).toBeUndefined();
  });
});

describe("segment fractions — where each part ends along its stroke", () => {
  const fractions = segmentEndFractions(MA.strokes[0]);

  it("has one entry per labelled part", () => {
    expect(fractions).toHaveLength(MA.strokes[0].segments.length);
  });

  it("ascends and finishes at the end of the stroke", () => {
    for (let i = 1; i < fractions.length; i++) {
      expect(fractions[i]).toBeGreaterThan(fractions[i - 1]);
    }
    expect(fractions[0]).toBeGreaterThan(0);
    expect(fractions[fractions.length - 1]).toBeCloseTo(1, 10);
  });

  it("a zero-length stroke reports every part already complete", () => {
    const flat = {
      segments: [
        {
          label: "nowhere",
          path: [
            { x: 5, y: 5 },
            { x: 5, y: 5 },
          ],
        },
      ],
    };
    expect(segmentEndFractions(flat)).toEqual([1]);
  });
});

describe("ductusSteps — the frames, in writing order", () => {
  const steps = ductusSteps(MA);

  it("gives one step per labelled part, numbered from 1", () => {
    expect(steps).toHaveLength(5);
    expect(steps.map((s) => s.number)).toEqual([1, 2, 3, 4, 5]);
    expect(steps[0].label).toBe("down the left upright");
    expect(steps[4].label).toBe("down the middle");
  });

  it("marks ம as never lifting the pen", () => {
    expect(steps.every((s) => s.startsAfterLift === false)).toBe(true);
    expect(steps.every((s) => s.strokeIndex === 0)).toBe(true);
  });
});

describe("the drawn frame", () => {
  const frame = ductusFrame(MA, outline, ductusSteps(MA)[2]);

  it("draws the FONT's outline, not a redrawn one", () => {
    const glyphPath = byTag(frame, "path").find(
      (p) => p.attrs.class === "ductus__glyph",
    )!;
    expect(glyphPath.attrs.d).toBe(outline.path);
    // Sanity: that really is a font path — quadratics and closed contours.
    expect(String(glyphPath.attrs.d)).toMatch(/^M/);
    expect(String(glyphPath.attrs.d)).toContain("Z");
  });

  it("emits the pen path straight from penPathD at the step's fraction", () => {
    const step = ductusSteps(MA)[2];
    const pen = byTag(frame, "path").find(
      (p) => p.attrs.class === "ductus__pen",
    )!;
    expect(pen.attrs.d).toBe(penPathD(MA.strokes[0], step.fraction));
    expect(pen.attrs.fill).toBe("none");
  });

  it("puts a pen dot at the end of what has been drawn", () => {
    const dot = byTag(frame, "circle")[0];
    const d = String(
      byTag(frame, "path").find((p) => p.attrs.class === "ductus__pen")!.attrs
        .d,
    );
    const last = d
      .trim()
      .split(/(?=[ML])/)
      .pop()!
      .slice(1)
      .trim()
      .split(/\s+/)
      .map(Number);
    expect(Number(dot.attrs.cx)).toBeCloseTo(last[0], 1);
    expect(Number(dot.attrs.cy)).toBeCloseTo(last[1], 1);
  });

  it("labels the step for a screen reader as well as on screen", () => {
    expect(String(frame.attrs["aria-label"])).toContain("up the right side");
    expect(byTag(frame, "title")[0].text).toContain("up the right side");
    expect(
      byTag(frame, "tspan")
        .map((t) => t.text)
        .join(" "),
    ).toBe("3. up the right side");
  });

  it("carries an intrinsic size matching its viewBox, so it never renders squashed", () => {
    const [, , w, h] = String(frame.attrs.viewBox).split(" ").map(Number);
    expect(Number(frame.attrs.height) / Number(frame.attrs.width)).toBeCloseTo(
      h / w,
      2,
    );
  });
});

describe("captions wrap instead of running off the panel", () => {
  it("breaks on whole words at the width available", () => {
    // 900 units wide at 92-unit text ≈ 18 characters a line.
    expect(wrapCaption("1. down the left upright", 900, 92)).toEqual([
      "1. down the left",
      "upright",
    ]);
    expect(wrapCaption("4. over the top", 900, 92)).toEqual([
      "4. over the top",
    ]);
  });

  it("never chops a word in half, even one too long to fit", () => {
    const lines = wrapCaption("Rrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrr", 200, 92);
    expect(lines).toEqual(["Rrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrr"]);
  });

  it("returns one empty line for empty text rather than no lines at all", () => {
    expect(wrapCaption("   ", 900, 92)).toEqual([""]);
  });

  it("makes the box taller when the captions need two lines", () => {
    const short = viewBoxFor(MA, outline, { captionSize: 20 }).height;
    const tall = viewBoxFor(MA, outline, { captionSize: 200 }).height;
    expect(tall).toBeGreaterThan(short);
  });
});

// ---------------------------------------------------------------------------
// The flip. This is the invariant the whole module exists to guarantee: the
// glyph and the pen path are both in font units (y-up), SVG is y-down, and they
// are flipped together exactly once — so they cannot end up disagreeing.
// ---------------------------------------------------------------------------
describe("one shared y-flip", () => {
  const frame = ductusFrame(MA, outline, ductusSteps(MA)[4]);

  it("uses exactly one scale(1,-1) group", () => {
    const flips = collect(frame, (n) =>
      String(n.attrs.transform ?? "").includes("scale(1,-1)"),
    );
    expect(flips).toHaveLength(1);
  });

  it("puts the glyph, every pen path and the pen dot inside that one group", () => {
    const flip = collect(frame, (n) =>
      String(n.attrs.transform ?? "").includes("scale(1,-1)"),
    )[0];
    expect(byTag(flip, "path").length).toBe(byTag(frame, "path").length);
    expect(byTag(flip, "circle").length).toBe(byTag(frame, "circle").length);
    expect(byTag(flip, "path").length).toBeGreaterThan(1); // glyph + pen
  });

  it("keeps text OUT of the flip, because mirrored text is unreadable", () => {
    const flip = collect(frame, (n) =>
      String(n.attrs.transform ?? "").includes("scale(1,-1)"),
    )[0];
    expect(byTag(flip, "text")).toHaveLength(0);
    expect(byTag(frame, "text")).toHaveLength(1);
  });

  it("negates the vertical range in the viewBox, as the flip requires", () => {
    const box = viewBoxFor(MA, outline);
    const b = outline.bounds;
    // Top of the letter (largest font y) becomes the SMALLEST svg y.
    expect(box.minY).toBeLessThan(0);
    expect(box.minY).toBeCloseTo(-(b.y1 + 70), 5);
    // The box is wide enough for the ink, and taller by the caption band.
    expect(box.width).toBeGreaterThanOrEqual(b.x1 - b.x0);
    expect(box.height).toBeGreaterThan(b.y1 - b.y0);
  });

  it("keeps the whole flipped glyph inside the viewBox", () => {
    const box = viewBoxFor(MA, outline);
    const b = outline.bounds;
    // Flip every corner of the glyph box and check containment.
    for (const x of [b.x0, b.x1]) {
      for (const y of [b.y0, b.y1]) {
        expect(x).toBeGreaterThanOrEqual(box.minX);
        expect(x).toBeLessThanOrEqual(box.minX + box.width);
        expect(-y).toBeGreaterThanOrEqual(box.minY);
        expect(-y).toBeLessThanOrEqual(box.minY + box.height);
      }
    }
  });

  it("CONTROL: an unflipped viewBox would cut the letter off entirely", () => {
    // If someone "simplified" the box to raw font coordinates, the flipped
    // glyph would sit at negative y and fall completely outside it. This is the
    // failure the negation prevents; assert it really is a failure.
    const b = outline.bounds;
    const naiveTop = b.y0;
    expect(-b.y1).toBeLessThan(naiveTop);
  });
});

describe("the build-up advances", () => {
  const strip = ductusFilmstrip(MA, outline);

  it("has one frame per step", () => {
    expect(strip.frames).toHaveLength(strip.steps.length);
    expect(strip.frames).toHaveLength(5);
  });

  it("draws strictly more of the stroke in each successive frame", () => {
    const drawn = strip.frames.map(
      (f) =>
        String(
          byTag(f, "path").find((p) => p.attrs.class === "ductus__pen")!.attrs
            .d,
        ).length,
    );
    for (let i = 1; i < drawn.length; i++) {
      expect(drawn[i]).toBeGreaterThan(drawn[i - 1]);
    }
  });

  it("the last frame is the complete stroke", () => {
    const last = strip.frames[strip.frames.length - 1];
    const pen = byTag(last, "path").find(
      (p) => p.attrs.class === "ductus__pen",
    )!;
    expect(pen.attrs.d).toBe(penPathD(MA.strokes[0], 1));
  });

  it("says in words how many strokes and lifts there are", () => {
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 5 movements");
  });
});

// ---------------------------------------------------------------------------
// Generic multi-stroke edge cases still use a synthetic ductus so the test can
// vary stroke counts independently of curriculum data. Nothing in this fixture
// is ever shown to a learner, and no letter enters DUCTUS without a citation.
// ---------------------------------------------------------------------------
describe("a letter written in more than one stroke", () => {
  const twoStroke: LetterDuctus = {
    script: "test",
    glyph: "✚",
    strokes: [
      {
        segments: [
          {
            label: "the upright",
            path: [
              { x: 100, y: 0 },
              { x: 100, y: 400 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "the crossbar, left half",
            path: [
              { x: 0, y: 200 },
              { x: 100, y: 200 },
            ],
          },
          {
            label: "the crossbar, right half",
            path: [
              { x: 100, y: 200 },
              { x: 200, y: 200 },
            ],
          },
        ],
      },
    ],
    source: {
      citation: "test fixture, not curriculum data",
      url: "https://example.invalid/fixture",
    },
  };
  const fakeOutline: GlyphOutline = {
    path: "M0 0L1 1Z",
    bounds: { x0: 0, y0: 0, x1: 200, y1: 400 },
  };

  it("renders every frame without throwing", () => {
    expect(() => ductusFilmstrip(twoStroke, fakeOutline)).not.toThrow();
    const strip = ductusFilmstrip(twoStroke, fakeOutline);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 3 movements");
  });

  it("flags the frame where the hand leaves the paper", () => {
    const steps = ductusSteps(twoStroke);
    expect(steps.map((s) => s.startsAfterLift)).toEqual([false, true, false]);
  });

  it("keeps already-finished strokes on screen, in a settled colour", () => {
    const strip = ductusFilmstrip(twoStroke, fakeOutline);
    const first = byTag(strip.frames[0], "path").filter(
      (p) => p.attrs.class === "ductus__done",
    );
    const later = byTag(strip.frames[2], "path").filter(
      (p) => p.attrs.class === "ductus__done",
    );
    expect(first).toHaveLength(0); // nothing finished yet
    expect(later).toHaveLength(1); // the upright, done, behind the crossbar
    expect(later[0].attrs.d).toBe(penPathD(twoStroke.strokes[0], 1));
  });

  it("counts lifts and movements in plain English", () => {
    const strip = (n: number) =>
      ductusFilmstrip(
        {
          script: "test",
          glyph: "?",
          strokes: Array.from({ length: n }, (_, i) => ({
            segments: [
              {
                label: `part ${i}`,
                path: [
                  { x: 0, y: 0 },
                  { x: 10, y: 0 },
                ],
              },
            ],
          })),
          source: twoStroke.source,
        },
        fakeOutline,
      ).summary;
    expect(strip(1)).toBe("one unbroken stroke · 1 movement");
    expect(strip(3)).toBe("3 strokes · 2 pen lifts · 3 movements");
  });

  it("honours caller-supplied sizes and colours", () => {
    const strip = ductusFilmstrip(twoStroke, fakeOutline, {
      padding: 0,
      penColor: "#ff0000",
    });
    const pen = byTag(strip.frames[0], "path").find(
      (p) => p.attrs.class === "ductus__pen",
    )!;
    expect(pen.attrs.stroke).toBe("#ff0000");
    // With no padding the box hugs the ink exactly sideways, and is taller than
    // the letter by exactly the caption band.
    const box = viewBoxFor(twoStroke, fakeOutline, { padding: 0 });
    expect(box.minX).toBe(0);
    expect(box.minY).toBe(-400);
    expect(box.width).toBe(200);
    expect(box.height).toBeGreaterThan(400);
  });

  it("gives every frame of a letter the SAME box, so the strip reads as one picture", () => {
    const boxes = ductusFilmstrip(twoStroke, fakeOutline).frames.map(
      (f) => f.attrs.viewBox,
    );
    expect(new Set(boxes).size).toBe(1);
  });

  it("draws nothing rather than an empty path when a part has no points", () => {
    const hollow: LetterDuctus = {
      script: "test",
      glyph: "␀",
      strokes: [{ segments: [{ label: "nothing at all", path: [] }] }],
      source: twoStroke.source,
    };
    const frame = ductusFrame(hollow, fakeOutline, ductusSteps(hollow)[0]);
    // Only the glyph outline survives; no zero-length pen path is emitted.
    expect(byTag(frame, "path")).toHaveLength(1);
    expect(byTag(frame, "path")[0].attrs.class).toBe("ductus__glyph");
    // The box still falls back to the glyph's own bounds when the pen has none.
    expect(viewBoxFor(hollow, fakeOutline).width).toBeGreaterThan(200);
  });

  it("survives degenerate input rather than emitting a zero-size picture", () => {
    const empty: LetterDuctus = {
      script: "test",
      glyph: "␣",
      strokes: [],
      source: twoStroke.source,
    };
    const nowhere: GlyphOutline = {
      path: "",
      bounds: { x0: 0, y0: 0, x1: 0, y1: 0 },
    };
    const strip = ductusFilmstrip(empty, nowhere);
    expect(strip.frames).toHaveLength(0);
    expect(strip.penLifts).toBe(0);
    const box = viewBoxFor(empty, nowhere);
    expect(box.width).toBeGreaterThan(0);
    expect(box.height).toBeGreaterThan(0);
  });
});

describe("serialising to markup", () => {
  it("produces well-formed SVG with the path data intact", () => {
    const svg = svgMarkup(ductusFrame(MA, outline, ductusSteps(MA)[0]));
    expect(svg.startsWith("<svg ")).toBe(true);
    expect(svg.endsWith("</svg>")).toBe(true);
    expect(svg).toContain('xmlns="http://www.w3.org/2000/svg"');
    expect(svg).toContain(`d="${outline.path}"`);
    expect(svg).toContain('transform="scale(1,-1)"');
    // A DOM parser is the real test of well-formedness.
    const doc = new DOMParser().parseFromString(svg, "image/svg+xml");
    expect(doc.querySelector("parsererror")).toBeNull();
    expect(doc.querySelectorAll("path").length).toBeGreaterThan(1);
  });

  it("escapes every XML metacharacter", () => {
    expect(escapeXml(`<&>"'`)).toBe("&lt;&amp;&gt;&quot;&apos;");
    expect(escapeXml("plain")).toBe("plain");
  });

  it("cannot be escaped out of an attribute by a hostile label", () => {
    // A label is authored today, but escaping is a property of the writer, not
    // of the data. Feed it markup and check none survives as markup.
    const nasty: LetterDuctus = {
      script: "test",
      glyph: "x",
      strokes: [
        {
          segments: [
            {
              label: `"><script>alert(1)</script>`,
              path: [
                { x: 0, y: 0 },
                { x: 10, y: 10 },
              ],
            },
          ],
        },
      ],
      source: {
        citation: "test fixture",
        url: "https://example.invalid/fixture",
      },
    };
    const svg = svgMarkup(
      ductusFrame(
        nasty,
        { path: "M0 0Z", bounds: { x0: 0, y0: 0, x1: 10, y1: 10 } },
        ductusSteps(nasty)[0],
      ),
    );
    expect(svg).not.toContain("<script>");
    expect(svg).toContain("&lt;script&gt;");
    expect(svg).toContain("&quot;");
    const doc = new DOMParser().parseFromString(svg, "image/svg+xml");
    expect(doc.querySelector("parsererror")).toBeNull();
    expect(doc.querySelector("script")).toBeNull();
  });

  it("self-closes childless elements", () => {
    expect(svgMarkup({ tag: "circle", attrs: { r: 3 } })).toBe(
      '<circle r="3"/>',
    );
  });

  // An attribute NAME cannot be escaped — there is nowhere to put the entity —
  // so a name is either legal or dropped. Every name this module emits is a
  // literal, but `SvgNode` is public and the serialiser is meant to be reused.
  it("drops attribute names that are not legal XML names", () => {
    expect(isSafeName("stroke-width")).toBe(true);
    expect(isSafeName("xlink:href")).toBe(true);
    expect(isSafeName(`x" onload="alert(1)`)).toBe(false);
    expect(isSafeName("2bad")).toBe(false);
    const svg = svgMarkup({
      tag: "rect",
      attrs: { [`x" onload="alert(1)`]: "1", width: 4 },
    });
    expect(svg).toBe('<rect width="4"/>');
  });

  it("refuses event-handler attributes outright, prefix and all", () => {
    // `onload` is a perfectly legal XML name AND a script. Reject the prefix
    // rather than chase a list of handler names that keeps growing.
    expect(isSafeName("onload")).toBe(false);
    expect(isSafeName("OnClick")).toBe(false);
    expect(
      svgMarkup({ tag: "svg", attrs: { onload: "alert(1)", onclick: "x" } }),
    ).toBe("<svg/>");
  });

  it("neutralises a hostile tag name rather than emitting it", () => {
    expect(svgMarkup({ tag: "svg><script", attrs: {} })).toBe("<g/>");
  });
});
