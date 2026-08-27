import { beforeAll, describe, expect, it } from "vitest";
import { SCRIPTS, verifiedLetterFont } from "../../src/scriptdata";
import {
  DUCTUS,
  ductusFor,
  ductusKey,
  joinGaps,
  penLifts,
  penPath,
  penPathD,
  penTip,
  type LetterDuctus,
  type Point,
} from "../../src/strokes";
import {
  distanceToPath,
  fontForDuctus,
  fractionOnInk,
  inkPoints,
  makeInInk,
  registerStrokeHonestyTests,
} from "../support/stroke-honesty";

const TAMIL_U = DUCTUS["உ"];
const TAMIL_UU = DUCTUS["ஊ"];
const TAMIL_NGA = DUCTUS["ங"];
const TAMIL_NYA = DUCTUS["ஞ"];

const OWNER_SCRIPTS = new Set(["tamil"]);
const letters = (Object.values(DUCTUS) as LetterDuctus[]).filter((letter) =>
  OWNER_SCRIPTS.has(letter.script),
);

describe("handwriting ductus", () => {
  registerStrokeHonestyTests(letters, { ஊ: 0.9 });

  it("CONTROL: a Tamil stroke pushed off its glyph fails the on-ink check", () => {
    const reference = DUCTUS["ம"];
    const inInk = makeInInk(fontForDuctus(reference).glyphFor("ம")!.contours);
    const shifted = penPath(reference.strokes[0]).map((point) => ({
      x: point.x + 400,
      y: point.y,
    }));
    expect(fractionOnInk(shifted, inInk)).toBeLessThan(0.9);
  });

  it("CONTROL: dropping the Tamil arch leaves much of the glyph untraced", () => {
    const reference = DUCTUS["ம"];
    const ink = fontForDuctus(reference).glyphFor("ம")!;
    const points = inkPoints(ink.contours);
    const onlyFirstTwo = {
      segments: reference.strokes[0].segments.slice(0, 2),
    };
    const path = penPath(onlyFirstTwo);
    const strayed = points.filter(([x, y]) => distanceToPath(x, y, path) > 130);
    expect(strayed.length / points.length).toBeGreaterThan(0.1);
  });

  beforeAll(() => {
    expect(verifiedLetterFont("எ", DUCTUS["எ"].source.url)).toBe(
      "_fonts/NotoSansTamil-Static.ttf",
    );
    expect(verifiedLetterFont("ம", DUCTUS["ம"].source.url)).toBe(
      "_fonts/NotoSansTamil-Static.ttf",
    );
  });

  it("ம is written without lifting the pen (one stroke)", () => {
    expect(penLifts(DUCTUS["ம"])).toBe(0);
    expect(DUCTUS["ம"].strokes).toHaveLength(1);
  });

  it("Tamil உ keeps all three Frame 16 movements in one run", () => {
    expect(penLifts(TAMIL_U)).toBe(0);
    expect(TAMIL_U.strokes).toHaveLength(1);
    expect(TAMIL_U.strokes[0].segments.map((segment) => segment.label)).toEqual(
      [
        "sweep outward around the compact upper spiral",
        "descend through the broad outer curve and turn left onto the baseline",
        "carry the long baseline straight to the right",
      ],
    );
  });

  it("Tamil ஊ writes familiar உ before the three-run ள overlay", () => {
    expect(penLifts(TAMIL_UU)).toBe(3);
    expect(TAMIL_UU.strokes).toHaveLength(4);
    expect(TAMIL_UU.strokes.map((stroke) => stroke.segments.length)).toEqual([
      3, 3, 2, 1,
    ]);
    expect(TAMIL_UU.source.citation).toMatch(
      /Module 17.*ஊ.*Frames 17, 16, and 12.*pp\. 195–196/i,
    );
    expect(TAMIL_UU.source.variation).toMatch(
      /write உ first.*then write ள over it.*Frame 16.*three movements joined.*Frame 12.*six movements.*three pen-down runs.*four-run learner order.*Noto Sans Tamil/i,
    );
  });

  it("ஒ's two-run order traces to Module 14 and Appendix I Frame 14", () => {
    const source = DUCTUS["ஒ"].source;
    expect(source.url).toContain("module-14");
    expect(source.citation).toMatch(
      /Module 14.*ஒ.*Appendix I.*Frame 14.*p\. 195/i,
    );
    expect(source.variation).toMatch(
      /short o.*three movements.*left loop.*large right loop.*joined.*separate lower bowl.*one lift.*two-run.*Noto Sans Tamil.*varies by school/i,
    );
    expect(penLifts(DUCTUS["ஒ"])).toBe(1);
  });

  it("Tamil ங keeps Frame 2's detached upright and joined body separate", () => {
    expect(penLifts(TAMIL_NGA)).toBe(1);
    expect(TAMIL_NGA.strokes).toHaveLength(2);
    expect(
      TAMIL_NGA.strokes[1].segments.map((segment) => segment.label),
    ).toEqual([
      "climb the tall left body",
      "carry the top bar right and return inward",
      "descend into the rounded inner turn",
      "carry the low bar to the right",
      "return left and finish up the inner stem",
    ]);
  });

  it("Tamil ஞ groups Frame 8's eight movements into four runs", () => {
    expect(penLifts(TAMIL_NYA)).toBe(3);
    expect(TAMIL_NYA.strokes).toHaveLength(4);
    expect(TAMIL_NYA.strokes.map((stroke) => stroke.segments.length)).toEqual([
      2, 1, 2, 3,
    ]);
    expect(TAMIL_NYA.source.citation).toMatch(
      /Tamil Script Learners Manual.*Frame 8.*ஞ.*p\. 194/i,
    );
    expect(TAMIL_NYA.source.variation).toMatch(
      /1–2.*left inner loop.*3.*top bar.*4–5.*central descent.*6–8.*outer bowl.*four-run order.*Noto Sans Tamil/i,
    );
  });

  it("ப descends, crosses the bottom, and rises without lifting", () => {
    expect(penLifts(DUCTUS["ப"])).toBe(0);
    expect(DUCTUS["ப"].strokes).toHaveLength(1);
    expect(
      DUCTUS["ப"].strokes[0].segments.map((segment) => segment.label),
    ).toEqual([
      "down the left upright",
      "along the bottom",
      "up the right upright",
    ]);
  });

  it("ட descends and turns along its foot without lifting", () => {
    expect(penLifts(DUCTUS["ட"])).toBe(0);
    expect(DUCTUS["ட"].strokes).toHaveLength(1);
    expect(
      DUCTUS["ட"].strokes[0].segments.map((segment) => segment.label),
    ).toEqual(["down the left upright", "along the long rightward foot"]);
  });

  it("த groups seven movements into four source-verified pen-down runs", () => {
    expect(penLifts(DUCTUS["த"])).toBe(3);
    expect(DUCTUS["த"].strokes).toHaveLength(4);
    expect(
      DUCTUS["த"].strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      ["climb the short left upright", "carry the top bar to the right"],
      [
        "carry the short upper bar right",
        "curve down around the broad right bowl",
      ],
      [
        "turn around the compact left loop",
        "curl back to the central crossing",
      ],
      ["sweep the low tail left"],
    ]);
  });

  it("Tamil எ keeps its six-movement body separate from the right upright", () => {
    expect(penLifts(DUCTUS["எ"])).toBe(1);
    expect(DUCTUS["எ"].strokes).toHaveLength(2);
    expect(
      DUCTUS["எ"].strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "climb the outer left side",
        "carry the top bar to the right",
        "retrace left and drop the inner upright",
        "turn left into the inner spiral",
        "sweep around the broad outer curve",
        "carry the lower foot right",
      ],
      ["draw the separate right upright up"],
    ]);
  });

  it("Tamil ழ groups six movements into three source-verified pen-down runs", () => {
    expect(penLifts(DUCTUS["ழ"])).toBe(2);
    expect(DUCTUS["ழ"].strokes).toHaveLength(3);
    expect(
      DUCTUS["ழ"].strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "climb the outer left upright",
        "retrace down the left upright",
        "carry the low crossbar right",
      ],
      [
        "retrace left into the inner upright",
        "descend and sweep around the broad right bowl",
      ],
      ["turn through the detached lower hook"],
    ]);
  });

  it("ய joins its hook, retraced center, base, and right upright without lifting", () => {
    expect(penLifts(DUCTUS["ய"])).toBe(0);
    expect(DUCTUS["ய"].strokes).toHaveLength(1);
    expect(
      DUCTUS["ய"].strokes[0].segments.map((segment) => segment.label),
    ).toEqual([
      "down the left upright",
      "around the curved foot into the center",
      "up the central upright",
      "retrace down the central upright",
      "along the bottom",
      "up the right upright",
    ]);
  });

  it("ர writes its uprights and cap before joining the angular tail", () => {
    expect(penLifts(DUCTUS["ர"])).toBe(2);
    expect(DUCTUS["ர"].strokes).toHaveLength(3);
    expect(DUCTUS["ர"].strokes.map((stroke) => stroke.segments.length)).toEqual(
      [1, 1, 2],
    );
    expect(
      DUCTUS["ர"].strokes[2].segments.map((segment) => segment.label),
    ).toEqual(["down the central upright", "around the short angular tail"]);
  });

  it("ச joins its upper frame before lifting for the lower-left bowl", () => {
    expect(penLifts(DUCTUS["ச"])).toBe(1);
    expect(DUCTUS["ச"].strokes).toHaveLength(2);
    expect(DUCTUS["ச"].strokes.map((stroke) => stroke.segments.length)).toEqual(
      [3, 1],
    );
    expect(
      DUCTUS["ச"].strokes[0].segments.map((segment) => segment.label),
    ).toEqual([
      "climb the left upright",
      "carry the top bar to the right",
      "drop the inner upright and carry right",
    ]);
    expect(DUCTUS["ச"].strokes[1].segments[0].label).toBe(
      "turn around and close the lower-left bowl",
    );
  });

  it("அ lifts once before its separate right upright (two strokes)", () => {
    expect(penLifts(DUCTUS["அ"])).toBe(1);
    expect(DUCTUS["அ"].strokes).toHaveLength(2);
  });

  it("ஆ lifts once, then joins its upright and long-vowel loop", () => {
    expect(penLifts(DUCTUS["ஆ"])).toBe(1);
    expect(DUCTUS["ஆ"].strokes).toHaveLength(2);
    expect(DUCTUS["ஆ"].strokes[1].segments).toHaveLength(2);
  });

  it("இ lifts once before its joined outer climb and arch", () => {
    expect(penLifts(DUCTUS["இ"])).toBe(1);
    expect(DUCTUS["இ"].strokes).toHaveLength(2);
    expect(DUCTUS["இ"].strokes[0].segments).toHaveLength(5);
    expect(DUCTUS["இ"].strokes[1].segments).toHaveLength(2);
  });

  it("க lifts between its upper frame and two lower bowls", () => {
    expect(penLifts(DUCTUS["க"])).toBe(2);
    expect(DUCTUS["க"].strokes).toHaveLength(3);
    expect(DUCTUS["க"].strokes.map((stroke) => stroke.segments.length)).toEqual(
      [3, 2, 1],
    );
  });

  it("வ joins all five movements without lifting the pen", () => {
    expect(penLifts(DUCTUS["வ"])).toBe(0);
    expect(DUCTUS["வ"].strokes).toHaveLength(1);
    expect(DUCTUS["வ"].strokes[0].segments).toHaveLength(5);
  });

  it("ல joins all four movements without lifting the pen", () => {
    expect(penLifts(DUCTUS["ல"])).toBe(0);
    expect(DUCTUS["ல"].strokes).toHaveLength(1);
    expect(DUCTUS["ல"].strokes[0].segments).toHaveLength(4);
  });

  it("ள lifts between its three pen-down runs", () => {
    expect(penLifts(DUCTUS["ள"])).toBe(2);
    expect(DUCTUS["ள"].strokes).toHaveLength(3);
    expect(DUCTUS["ள"].strokes.map((stroke) => stroke.segments.length)).toEqual(
      [3, 2, 1],
    );
  });

  it("ற lifts between its three pen-down runs", () => {
    expect(penLifts(DUCTUS["ற"])).toBe(2);
    expect(DUCTUS["ற"].strokes).toHaveLength(3);
    expect(DUCTUS["ற"].strokes.map((stroke) => stroke.segments.length)).toEqual(
      [2, 1, 2],
    );
  });

  it("ன joins its first five movements before the right upright", () => {
    expect(penLifts(DUCTUS["ன"])).toBe(1);
    expect(DUCTUS["ன"].strokes).toHaveLength(2);
    expect(DUCTUS["ன"].strokes.map((stroke) => stroke.segments.length)).toEqual(
      [5, 1],
    );
  });

  it("ண joins its first six movements before the right upright", () => {
    expect(penLifts(DUCTUS["ண"])).toBe(1);
    expect(DUCTUS["ண"].strokes).toHaveLength(2);
    expect(DUCTUS["ண"].strokes.map((stroke) => stroke.segments.length)).toEqual(
      [6, 1],
    );
  });

  it("ந groups Frame 5's six movements into three pen-down runs", () => {
    expect(penLifts(DUCTUS["ந"])).toBe(2);
    expect(DUCTUS["ந"].strokes).toHaveLength(3);
    expect(DUCTUS["ந"].strokes.map((stroke) => stroke.segments.length)).toEqual(
      [2, 2, 2],
    );
  });

  it("ம's stroke order traces to the UT Austin primer, and records Tamil's variation", () => {
    const src = DUCTUS["ம"].source;
    expect(src.url).toContain("tamilscript");
    expect(src.citation).toMatch(/Appendix I|Frame 1/);
    expect(
      src.variation,
      "must not present one order as the only order",
    ).toMatch(/variation|no single/i);
  });

  it("ப's continuous order traces to Frame 1 of the UT Austin primer", () => {
    const src = DUCTUS["ப"].source;
    expect(src.url).toContain("tamilscript/category/3-moduals/module-01");
    expect(src.citation).toMatch(/Tamil Script Learners Manual.*Frame 1.*ப/i);
    expect(src.variation).toMatch(
      /left-to-right.*top-to-bottom.*varies by school.*continuous order.*Noto Sans Tamil/i,
    );
  });

  it("த's four-run order traces to Frame 3 of the UT Austin primer", () => {
    const src = DUCTUS["த"].source;
    expect(src.url).toContain(
      "tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(src.citation).toMatch(/Appendix I.*Frame 3.*த.*p\. 192/i);
    expect(src.variation).toMatch(
      /Module 3 identifies.*dental stop.*final Frame 3 row.*four separate pen-down runs.*1–2.*upper frame.*3–4.*right bowl.*5–6.*left loop.*movement 7.*leftward tail.*varies by school.*four-run order.*Noto Sans Tamil/i,
    );
  });

  it("ட's continuous order traces to Frame 1 of the UT Austin primer", () => {
    const src = DUCTUS["ட"].source;
    expect(src.url).toContain(
      "tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(src.citation).toMatch(/Appendix I.*Frame 1.*ட.*p\. 190/i);
    expect(src.variation).toMatch(
      /left descent.*rightward foot.*two joined movements.*Module 1 identifies.*top-to-bottom.*left-to-right.*varies by school.*continuous order.*Noto Sans Tamil/i,
    );
  });

  it("ய's six-movement continuous order traces to Appendix I Frame 1", () => {
    const src = DUCTUS["ய"].source;
    expect(src.url).toContain(
      "tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(src.citation).toMatch(/Appendix I.*Frame 1.*ய.*p\. 190/i);
    expect(src.variation).toMatch(
      /six joined movements.*down the left.*central upright.*across the bottom.*up the right.*varies by school.*continuous order.*Noto Sans Tamil/i,
    );
  });

  it("ர's three-run order traces to Frame 3 of the UT Austin primer", () => {
    const src = DUCTUS["ர"].source;
    expect(src.url).toContain(
      "tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(src.citation).toMatch(/Appendix I.*Frame 3.*ர/i);
    expect(src.variation).toMatch(
      /three-movement ஈ frame.*angular short fourth movement.*varies by school.*three-run order.*Noto Sans Tamil/i,
    );
  });

  it("ச's two-run order traces to Frame 3 of the UT Austin primer", () => {
    const src = DUCTUS["ச"].source;
    expect(src.url).toContain(
      "tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(src.citation).toMatch(/Appendix I.*Frame 3.*ச.*p\. 191/i);
    expect(src.variation).toMatch(
      /three joined upper-frame movements.*separate fourth movement.*lower-left bowl.*varies by school.*two-run order.*Noto Sans Tamil/i,
    );
  });

  it("அ's stroke order traces to Frame 4 of the same primer", () => {
    const src = DUCTUS["அ"].source;
    expect(src.url).toContain("tamilscript");
    expect(src.citation).toMatch(/Appendix I.*Frame 4.*அ/);
    expect(
      src.variation,
      "must not present one order as the only order",
    ).toMatch(/variation|no single/i);
  });

  it("ஆ's stroke order traces to the next row of Frame 4", () => {
    const src = DUCTUS["ஆ"].source;
    expect(src.url).toContain("tamilscript");
    expect(src.citation).toMatch(/Appendix I.*Frame 4.*ஆ/);
    expect(
      src.variation,
      "must not present one order as the only order",
    ).toMatch(/variation|no single/i);
  });

  it("இ's stroke order traces to Frame 4's third row", () => {
    const src = DUCTUS["இ"].source;
    expect(src.url).toContain("tamilscript");
    expect(src.citation).toMatch(/Appendix I.*Frame 4.*இ/);
    expect(
      src.variation,
      "must not present one order as the only order",
    ).toMatch(/variation|no single/i);
  });

  it("க's stroke order traces to Frame 3's final row", () => {
    const src = DUCTUS["க"].source;
    expect(src.url).toContain("tamilscript");
    expect(src.citation).toMatch(/Appendix I.*Frame 3.*க/);
    expect(
      src.variation,
      "must not present one order as the only order",
    ).toMatch(/variation|no single/i);
  });

  it("ங's two-run order traces to Appendix I Frame 2", () => {
    const src = DUCTUS["ங"].source;
    expect(src.url).toContain(
      "tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(src.citation).toMatch(/Appendix I.*Frame 2.*ங.*p\. 191/i);
    expect(src.variation).toMatch(
      /detached descending upright.*five joined movements.*detached upright on the right.*two-run order/i,
    );
  });

  it("ஞ's four-run order traces to Appendix I Frame 8", () => {
    const src = DUCTUS["ஞ"].source;
    expect(src.url).toContain(
      "tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(src.citation).toMatch(/Appendix I.*Frame 8.*ஞ.*p\. 194/i);
    expect(src.variation).toMatch(
      /eight movements.*1–2.*left inner loop.*3.*top bar.*4–5.*central descent.*6–8.*outer bowl.*varies by school.*four-run order.*Noto Sans Tamil/i,
    );
  });

  it("ஊ's compositional order traces to Module 17 and its familiar components", () => {
    const src = DUCTUS["ஊ"].source;
    expect(src.url).toBe("https://sites.la.utexas.edu/tamilscript/frame-17/92");
    expect(src.citation).toMatch(
      /Module 17.*ஊ.*Frames 17, 16, and 12.*pp\. 195–196/i,
    );
    expect(src.variation).toMatch(
      /long ū.*write உ first.*then write ள over it.*four-run learner order.*Noto Sans Tamil.*varies by school/i,
    );
  });

  it("வ's stroke order traces to Frame 9's first row", () => {
    const src = DUCTUS["வ"].source;
    expect(src.url).toContain("tamilscript");
    expect(src.citation).toMatch(/Appendix I.*Frame 9.*வ/);
    expect(
      src.variation,
      "must not present one order as the only order",
    ).toMatch(/variation|no single/i);
  });

  it("ல's stroke order traces to Frame 9's second row", () => {
    const src = DUCTUS["ல"].source;
    expect(src.url).toContain("tamilscript");
    expect(src.citation).toMatch(/Appendix I.*Frame 9.*ல/);
    expect(
      src.variation,
      "must not present one order as the only order",
    ).toMatch(/variation|no single/i);
  });

  it("ள's three-run order traces to Frame 12", () => {
    const src = DUCTUS["ள"].source;
    expect(src.url).toContain("tamilscript");
    expect(src.citation).toMatch(/Appendix I.*Frame 12.*ள.*p\. 195/);
    expect(src.variation).toMatch(
      /Module 12.*retroflex lateral.*six movements.*three pen-down runs/i,
    );
    expect(
      src.variation,
      "must not present one order as the only order",
    ).toMatch(/variation|one attested/i);
  });

  it("ற's stroke order traces to Frame 10", () => {
    const src = DUCTUS["ற"].source;
    expect(src.url).toContain("tamilscript");
    expect(src.citation).toMatch(/Appendix I.*Frame 10.*ற/);
    expect(
      src.variation,
      "must not present one order as the only order",
    ).toMatch(/variation|no single/i);
  });

  it("ன's stroke order traces to Frame 13's first row", () => {
    const src = DUCTUS["ன"].source;
    expect(src.url).toContain("tamilscript");
    expect(src.citation).toMatch(/Appendix I.*Frame 13.*ன/);
    expect(
      src.variation,
      "must not present one order as the only order",
    ).toMatch(/variation|no single/i);
  });

  it("ண's stroke order traces to Frame 13's adjacent row", () => {
    const src = DUCTUS["ண"].source;
    expect(src.url).toContain("tamilscript");
    expect(src.citation).toMatch(/Appendix I.*Frame 13.*ண/);
    expect(
      src.variation,
      "must not present one order as the only order",
    ).toMatch(/variation|no single/i);
  });

  it("ந's three-run stroke order traces to Frame 5's first row", () => {
    const src = DUCTUS["ந"].source;
    expect(src.url).toContain("tamilscript");
    expect(src.citation).toMatch(/Appendix I.*Frame 5.*ந.*p\. 193/);
    expect(src.variation).toMatch(
      /Module 5.*dental nasal.*six movements.*three pen-down runs.*1.?2.*3.?4.*5.?6/i,
    );
    expect(
      src.variation,
      "must not present one order as the only order",
    ).toMatch(/varies|one attested/i);
  });

  it("எ's two-run stroke order traces to Frame 5's second row", () => {
    const src = DUCTUS["எ"].source;
    expect(src.url).toContain(
      "tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(src.citation).toMatch(/Appendix I.*Frame 5.*எ.*p\. 193/);
    expect(src.variation).toMatch(
      /first six movements.*connected body.*upward right upright.*movement 7.*one lift.*varies by school.*two-run order.*Noto Sans Tamil/i,
    );
  });

  it("ழ's three-run stroke order traces to Appendix I Frame 7", () => {
    const src = DUCTUS["ழ"].source;
    expect(src.url).toContain(
      "tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(src.citation).toMatch(/Appendix I.*Frame 7.*ழ.*p\. 193/i);
    expect(src.variation).toMatch(
      /six movements.*three pen-down runs.*1–3.*left body and bar.*4–5.*inner upright and broad right bowl.*movement 6.*detached lower hook.*Noto Sans Tamil.*low crossbar.*varies by school.*three-run order/i,
    );
  });
});
