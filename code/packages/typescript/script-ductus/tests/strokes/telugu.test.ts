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
import { registerStrokeHonestyTests } from "../support/stroke-honesty";

const TELUGU_A = DUCTUS[ductusKey("telugu", "అ")];
const TELUGU_AA = DUCTUS[ductusKey("telugu", "ఆ")];
const TELUGU_I = DUCTUS[ductusKey("telugu", "ఇ")];
const TELUGU_U = DUCTUS[ductusKey("telugu", "ఉ")];
const TELUGU_E = DUCTUS[ductusKey("telugu", "ఎ")];
const TELUGU_EE = DUCTUS[ductusKey("telugu", "ఏ")];

const OWNER_SCRIPTS = new Set(["telugu"]);
const letters = (Object.values(DUCTUS) as LetterDuctus[]).filter((letter) =>
  OWNER_SCRIPTS.has(letter.script),
);

describe("handwriting ductus", () => {
  registerStrokeHonestyTests(letters, { అ: 0.96 });

  beforeAll(() => {
    expect(verifiedLetterFont("అ", TELUGU_A.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("ఆ", TELUGU_AA.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("ఇ", TELUGU_I.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("ఉ", TELUGU_U.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("ఎ", TELUGU_E.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
    expect(verifiedLetterFont("ఏ", TELUGU_EE.source.url)).toBe(
      "_fonts/NotoSansTelugu-Static.ttf",
    );
  });

  it("Telugu అ groups four source-verified movements into two pen-down runs", () => {
    expect(penLifts(TELUGU_A)).toBe(1);
    expect(TELUGU_A.strokes).toHaveLength(2);
    expect(
      TELUGU_A.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      ["turn around the left lobe", "sweep around the broad lower bowl"],
      ["turn around the right lobe", "return left along the inner bar"],
    ]);
  });

  it("Telugu ఆ keeps its two source-verified components in separate pen-down runs", () => {
    expect(penLifts(TELUGU_AA)).toBe(1);
    expect(TELUGU_AA.strokes).toHaveLength(2);
    expect(
      TELUGU_AA.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "turn around the hooked left lobe and sweep through the broad lower bowl",
      ],
      [
        "turn around the rounded right lobe and return left along the inner bar",
      ],
    ]);
  });

  it("Telugu ఇ keeps its three source-verified components in separate pen-down runs", () => {
    expect(penLifts(TELUGU_I)).toBe(2);
    expect(TELUGU_I.strokes).toHaveLength(3);
    expect(
      TELUGU_I.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      ["turn around the broad outer bowl"],
      ["form the compact upper-left lobe"],
      ["form the angled upper-right shoulder"],
    ]);
  });

  it("Telugu ఉ groups five source-verified movements into three pen-down runs", () => {
    expect(penLifts(TELUGU_U)).toBe(2);
    expect(TELUGU_U.strokes).toHaveLength(3);
    expect(
      TELUGU_U.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "sweep left across the rounded upper arch",
        "continue down and around the broad lower bowl",
        "curl upward around the rounded right lobe without lifting",
      ],
      ["lift and draw the inner horizontal bar from left to right"],
      ["lift again and draw the short upper headstroke downward"],
    ]);
  });

  it("Telugu ఎ groups three source-verified movements into two pen-down runs", () => {
    expect(penLifts(TELUGU_E)).toBe(1);
    expect(TELUGU_E.strokes).toHaveLength(2);
    expect(
      TELUGU_E.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "turn down and left around the compact lower loop",
        "continue around its base and return to the central junction",
      ],
      ["restart at the junction and sweep up through the broad outer arch"],
    ]);
  });

  it("Telugu ఏ groups four source-verified movements into three pen-down runs", () => {
    expect(penLifts(TELUGU_EE)).toBe(2);
    expect(TELUGU_EE.strokes).toHaveLength(3);
    expect(
      TELUGU_EE.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "turn down and left around the compact lower loop",
        "continue around its base and return to the central junction",
      ],
      [
        "restart at the lower-right tail and sweep up through the broad outer arch",
      ],
      ["restart below the upper-left hook and sweep upward to its tip"],
    ]);
  });
});
