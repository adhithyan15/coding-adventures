// ---------------------------------------------------------------------------
// headline-word-gurmukhi.test.ts — a Gurmukhi word: bodies first, ONE headline last
// ---------------------------------------------------------------------------
//
// The cited Gurmukhi letters (GNPS's tracing lesson) draw their own headline
// FIRST. A Gurmukhi WORD is drawn the other way round, by this book's
// convention: every letter's body in reading order, then one headline, last,
// left to right, along the word's printed bar. What is pinned here:
//
//   * every cited Gurmukhi letter either has a headline that comes off its
//     FRONT (one level stroke, labelled exactly, left to right) or is one of
//     the five letters Noto prints with a SPLIT bar (ਅ ਖ ਘ ਪ ਮ), and the font
//     really shows that gap: the split list is measured, not believed;
//   * a single letter's own ductus is untouched: still headline first;
//   * ਪਰ, ਅਮਨ and ਮਨਨ (the corpus words) compose, with exactly the strokes
//     and the one headline described in headline-word.ts;
//   * the footer names the convention as a convention, and the split letters;
//   * the opening "lift, then " is dropped only where nothing came before;
//   * every refusal a Gurmukhi word can meet, each from a word that meets it.
// ---------------------------------------------------------------------------

import { describe, expect, it } from "vitest";

import { ductusFor } from "../src/ductusview";
import { buildFilmstripEntry } from "../src/filmstrip-ledger";
import {
  composeHeadlinePhrase,
  composeHeadlineWord,
  GURMUKHI_HEADLINE_LAST_SOURCE,
  GURMUKHI_LETTER_HEADLINE_LABEL,
  HEADLINE_WORD_SCRIPTS,
  LETTER_HEADLINE,
  LIFT_PREFIX,
  SPLIT_HEADLINE_LETTERS,
  splitHeadline,
  WORD_HEADLINE_LABEL,
} from "../src/headline-word";
import { makeInInk } from "../src/ink";
import { DUCTUS, type LetterDuctus } from "../src/strokes";
import { parsedFont } from "./support/font-fixtures";

const FONT = "NotoSansGurmukhi-Static.ttf";
const font = () => parsedFont(FONT);
const gurmukhi = Object.values(DUCTUS).filter((letter) => letter.script === "gurmukhi");
const compose = (word: string, lookup?: (glyph: string, script: string) => LetterDuctus | undefined) =>
  composeHeadlineWord(word, "gurmukhi", font(), lookup);
const letter = (glyph: string) => ductusFor(glyph, "gurmukhi")!;
const labels = (strokes: LetterDuctus["strokes"]) =>
  strokes.map((stroke) => stroke.segments.map((segment) => segment.label));

/** The runs of ink along one horizontal line through a letter, as [start, end] font units. */
function inkRuns(glyph: string, y: number): Array<[number, number]> {
  const outline = font().glyphFor(glyph)!;
  const inInk = makeInInk(outline.contours);
  const runs: Array<[number, number]> = [];
  let start: number | undefined;
  for (let x = -50; x <= 1100; x++) {
    const on = inInk(x, y);
    if (on && start === undefined) start = x;
    if (!on && start !== undefined) {
      runs.push([start, x - 1]);
      start = undefined;
    }
  }
  return runs;
}

describe("a Gurmukhi letter's own headline", () => {
  it("is a headline-word script whose letters draw their headline first", () => {
    expect(HEADLINE_WORD_SCRIPTS.has("gurmukhi")).toBe(true);
    expect(LETTER_HEADLINE.gurmukhi).toEqual({ at: "first", label: GURMUKHI_LETTER_HEADLINE_LABEL });
    expect(LETTER_HEADLINE.devanagari!.at).toBe("last");
  });

  it("comes off the front of every cited letter but the five whose printed bar is split", () => {
    expect(gurmukhi).toHaveLength(27);
    const split = SPLIT_HEADLINE_LETTERS.gurmukhi!;
    expect([...split]).toEqual(["ਅ", "ਖ", "ਘ", "ਪ", "ਮ"]);
    for (const cited of gurmukhi) {
      const taken = splitHeadline(cited);
      if (split.has(cited.glyph)) {
        // No stroke of a split letter is a headline: its bar pieces open its
        // body strokes, as GNPS draws them.
        expect(taken, cited.glyph).toBeUndefined();
        expect(labels(cited.strokes).flat(), cited.glyph).not.toContain(GURMUKHI_LETTER_HEADLINE_LABEL);
        continue;
      }
      expect(taken, cited.glyph).toBeDefined();
      expect(taken!.headline).toBe(cited.strokes[0]);
      expect(taken!.body).toEqual(cited.strokes.slice(1));
      expect(taken!.y, cited.glyph).toBe(586);
      expect(taken!.x0, cited.glyph).toBe(15);
      // The body that followed the headline opens with a lift.
      expect(taken!.body[0].segments[0].label.startsWith(LIFT_PREFIX), cited.glyph).toBe(true);
    }
  });

  it("is split, in the bundled font, exactly where the list says", () => {
    // Along the headline's height the printed bar is one run of ink for every
    // letter with a separate headline stroke, and two or more for the five.
    for (const cited of gurmukhi) {
      const runs = inkRuns(cited.glyph, 586);
      if (SPLIT_HEADLINE_LETTERS.gurmukhi!.has(cited.glyph)) {
        expect(runs.length, cited.glyph).toBeGreaterThanOrEqual(2);
        // The gap is wide: at least 180 units of blank paper above the body.
        expect(runs[1][0] - runs[0][1], cited.glyph).toBeGreaterThan(180);
      } else {
        expect(runs, cited.glyph).toHaveLength(1);
      }
    }
  });

  it("stays first in a letter's own strip: a single letter keeps the cited order", () => {
    for (const glyph of ["ਰ", "ਨ"]) {
      expect(letter(glyph).strokes[0].segments.map((segment) => segment.label)).toEqual([
        GURMUKHI_LETTER_HEADLINE_LABEL,
      ]);
    }
  });

  it("refuses a front stroke that is not just the headline", () => {
    const ra = letter("ਰ");
    const [headline, ...body] = ra.strokes;
    const line = headline.segments[0];
    const withFirst = (first: (typeof ra.strokes)[number]): LetterDuctus => ({ ...ra, strokes: [first, ...body] });
    expect(splitHeadline(withFirst({ segments: [line, body[0].segments[0]] }))).toBeUndefined();
    expect(splitHeadline(withFirst({ segments: [{ ...line, label: "draw the bar" }] }))).toBeUndefined();
    expect(splitHeadline(withFirst({ segments: [{ ...line, path: [...line.path].reverse() }] }))).toBeUndefined();
    expect(splitHeadline({ ...ra, strokes: [headline] })).toBeUndefined();
    // Read with the Devanagari rule, the same letter has no headline at its end.
    expect(splitHeadline(ra, LETTER_HEADLINE.devanagari)).toBeUndefined();
  });
});

describe("composing ਪਰ", () => {
  const result = compose("ਪਰ");
  if (!result.ok) throw new Error(result.reason);
  const pa = letter("ਪ");
  const ra = letter("ਰ");
  const advance = font().advanceFor("ਪ")!;

  it("draws ਪ whole, then ਰ's body without its headline, moved by ਪ's advance", () => {
    expect(advance).toBe(593);
    const strokes = result.ductus.strokes;
    expect(strokes).toHaveLength(pa.strokes.length + ra.strokes.length - 1 + 1);
    expect(strokes.slice(0, 2)).toEqual(pa.strokes);
    expect(labels(strokes.slice(2, -1))).toEqual(labels(ra.strokes.slice(1)));
    expect(strokes[2].segments[0].path).toEqual(
      ra.strokes[1].segments[0].path.map((point) => ({ x: point.x + advance, y: point.y })),
    );
  });

  it("ends with ONE headline, along the unbroken bar, never across ਪ's gap", () => {
    const headline = result.ductus.strokes.at(-1)!.segments;
    expect(headline.map((segment) => segment.label)).toEqual([WORD_HEADLINE_LABEL]);
    const path = headline[0].path;
    // From ਰ's own headline start to its end: ਪ's bar pieces are its own strokes'.
    expect(path[0]).toEqual({ x: advance + 15, y: 586 });
    expect(path.at(-1)).toEqual({ x: advance + 561, y: 586 });
    expect(path.every((point) => point.y === 586)).toBe(true);
    // The gap in ਪ's printed bar (x 160 to 422) lies wholly left of it.
    expect(inkRuns("ਪ", 586)[0][1]).toBeLessThan(path[0].x);
  });

  it("prints the one headline last, after a lift, and no headline before it", () => {
    const entry = buildFilmstripEntry(result.ductus, result.outline, `_fonts/${FONT}`);
    const frames = entry.frames.map((frame) => frame.label);
    expect(frames.filter((label) => /headline|shirorekha/.test(label))).toEqual([WORD_HEADLINE_LABEL]);
    expect(frames.at(-1)).toBe(WORD_HEADLINE_LABEL);
    expect(entry.frames.at(-1)!.startsAfterLift).toBe(true);
    expect(entry.summary).toBe("4 strokes · 3 pen lifts · 8 movements");
  });

  it("names GNPS for each letter and the convention, as a convention, for the headline", () => {
    const source = result.ductus.source;
    expect(source.citation).toBe(
      `letter 1: ${pa.source.citation}; letter 2: ${ra.source.citation}; ` +
        `the shared headline, drawn last: ${GURMUKHI_HEADLINE_LAST_SOURCE.citation}`,
    );
    expect(GURMUKHI_HEADLINE_LAST_SOURCE.citation).toContain("by this book's convention for words");
    expect(GURMUKHI_HEADLINE_LAST_SOURCE.citation).toContain("not from a Gurmukhi recording");
    expect(GURMUKHI_HEADLINE_LAST_SOURCE.citation).toContain("GNPS's own letters draw the headline first");
    expect(source.url).toMatch(/^https:\/\//);
    const variation = source.variation!;
    expect(variation).toContain(`<${pa.source.url}>`);
    expect(variation).toContain("GNPS's tracing lesson draws every letter's headline FIRST");
    expect(variation).toContain("The word-level order is not separately sourced");
    expect(variation).toContain("Letter 1 prints its headline split");
    expect(variation).toContain("the shared headline does not cross the gap");
    // Nothing of Devanagari's majority wording is claimed for Gurmukhi.
    expect(variation).not.toContain("most native writers use");
  });
});

describe("the corpus words", () => {
  it.each([
    // word, the letters taken whole, the letters the one headline runs over
    ["ਪਰ", 1, 1],
    ["ਅਮਨ", 2, 1],
    ["ਮਨਨ", 1, 2],
  ])("composes %s", (word, whole, under) => {
    const result = compose(word);
    if (!result.ok) throw new Error(result.reason);
    const characters = [...word];
    const split = characters.filter((glyph) => SPLIT_HEADLINE_LETTERS.gurmukhi!.has(glyph));
    expect(split).toHaveLength(whole);
    expect(characters.length - split.length).toBe(under);
    const expected = characters.reduce(
      (total, glyph) => total + letter(glyph).strokes.length - (split.includes(glyph) ? 0 : 1),
      1,
    );
    expect(result.ductus.strokes).toHaveLength(expected);
    expect(result.ductus.glyph).toBe(word);
  });

  it("draws ਮਨਨ's headline over both ਨ, from the first ਨ's headline to the second's end", () => {
    const result = compose("ਮਨਨ");
    if (!result.ok) throw new Error(result.reason);
    const ma = font().advanceFor("ਮ")!;
    const na = font().advanceFor("ਨ")!;
    const path = result.ductus.strokes.at(-1)!.segments[0].path;
    expect(path[0]).toEqual({ x: ma + 15, y: 586 });
    expect(path.at(-1)).toEqual({ x: ma + na + 630, y: 586 });
  });
});

describe("the word's first movement", () => {
  it("drops 'lift, then ' when a body opens the word, and nowhere else", () => {
    const result = compose("ਰਨ");
    if (!result.ok) throw new Error(result.reason);
    const ra = letter("ਰ");
    const opening = ra.strokes[1].segments[0].label;
    expect(opening).toBe("lift, then come down the stem");
    expect(result.ductus.strokes[0].segments[0].label).toBe("come down the stem");
    expect(result.ductus.strokes[0].segments[0].path).toEqual(ra.strokes[1].segments[0].path);
    // The rest of the first stroke, and ਨ's own opening lift, are kept.
    expect(labels([result.ductus.strokes[0]])[0].slice(1)).toEqual(labels([ra.strokes[1]])[0].slice(1));
    expect(result.ductus.strokes[1].segments[0].label).toBe("lift, then come down the stem");
    // The cited letter itself is unchanged.
    expect(ra.strokes[1].segments[0].label).toBe(opening);
  });

  it("leaves a split letter's opening alone: it never said lift", () => {
    const result = compose("ਮਨਨ");
    if (!result.ok) throw new Error(result.reason);
    expect(result.ductus.strokes[0]).toEqual(letter("ਮ").strokes[0]);
  });
});

describe("every Gurmukhi refusal", () => {
  it("refuses a split letter between two letters whose headline is shared", () => {
    // One straight headline from the first ਨ to the last would cross ਪ's gap.
    expect(compose("ਨਪਨ")).toEqual({
      ok: false,
      reason: "the shared headline is only 86.1% on the printed word's ink (the default tolerance is over 97%)",
    });
  });

  it("refuses a word of split letters only: it has no headline to share", () => {
    expect(compose("ਪਮ")).toEqual({
      ok: false,
      reason: "no letter of the word has a headline stroke of its own to share",
    });
  });

  it("refuses every sign, a precomposed nukta letter and an uncited letter", () => {
    for (const sign of ["ਿ", "ਾ", "ੀ", "ੇ", "ੰ", "ਂ", "ੱ", "੍", "਼"]) {
      expect(compose(`ਨ${sign}`), sign).toEqual({
        ok: false,
        reason: `${sign} is neither a single base letter nor a sign with a cited place in a word`,
      });
    }
    expect(compose("\u0A36\u0A30")).toEqual({
      ok: false,
      reason: "\u0A36 is neither a single base letter nor a sign with a cited place in a word",
    });
    expect(compose("ਉਹ")).toEqual({ ok: false, reason: "ਉ has no cited ductus" });
  });

  it("refuses a letter whose headline is not a stroke of its own, or sits at another height", () => {
    const ra = letter("ਰ");
    const noHeadline = (glyph: string, script: string) =>
      glyph === "ਰ" ? { ...ra, strokes: ra.strokes.slice(1) } : ductusFor(glyph, script);
    expect(compose("ਨਰ", noHeadline)).toEqual({ ok: false, reason: "ਰ has no separate headline stroke" });
    const lowered = (glyph: string, script: string) => {
      if (glyph !== "ਰ") return ductusFor(glyph, script);
      const [first, ...rest] = ra.strokes;
      const segment = first.segments[0];
      return {
        ...ra,
        strokes: [{ segments: [{ ...segment, path: segment.path.map((point) => ({ ...point, y: point.y - 10 })) }] }, ...rest],
      };
    };
    expect(compose("ਨਰ", lowered)).toEqual({ ok: false, reason: "ਰ's headline is not at the height of the first letter's" });
  });
});

describe("a Gurmukhi phrase", () => {
  it("is composed word by word, a one-letter word keeping its own headline-first strip", () => {
    const result = composeHeadlinePhrase("ਨ ਮਨਨ", "gurmukhi", font());
    if (!result.ok) throw new Error(result.reason);
    expect(result.words.map((word) => word.glyph)).toEqual(["ਨ", "ਮਨਨ"]);
    expect(result.words[0].ductus).toBe(letter("ਨ"));
    expect(result.words[1].ductus.strokes.at(-1)!.segments[0].label).toBe(WORD_HEADLINE_LABEL);
  });
});
