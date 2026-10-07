// ---------------------------------------------------------------------------
// headline-word.test.ts — a Devanagari word: letters' bodies, then one headline
// ---------------------------------------------------------------------------
//
// What is pinned here:
//
//   * every cited Devanagari letter has a headline that can be taken off on its
//     own, and no sign does;
//   * a composed word keeps each letter's body exactly (moved by the font's
//     advances) and ends with ONE headline over the whole word;
//   * the strip's last frame is that headline, after a lift, and no earlier
//     frame draws a headline;
//   * every refusal reason, each from a word that triggers it.
// ---------------------------------------------------------------------------

import { describe, expect, it } from "vitest";

import { HEADLINE_WORD_SCRIPTS as BOOK_HEADLINE_WORD_SCRIPTS } from "@coding-adventures/human-language-data/src/figure-targets.ts";
import { ductusFor } from "../src/ductusview";
import { buildFilmstripEntry } from "../src/filmstrip-ledger";
import {
  composeHeadlineWord,
  HEADLINE_LAST_SOURCE,
  HEADLINE_WORD_SCRIPTS,
  headlineWordSource,
  LETTER_HEADLINE_LABEL,
  splitHeadline,
  WORD_HEADLINE_LABEL,
} from "../src/headline-word";
import { DUCTUS, type LetterDuctus } from "../src/strokes";
import type { Font } from "../src/truetype";
import { parsedFont } from "./support/font-fixtures";

const font = () => parsedFont("NotoSansDevanagari-Static.ttf");
const devanagari = Object.values(DUCTUS).filter((letter) => letter.script === "devanagari");
const compose = (word: string, lookup?: (glyph: string, script: string) => LetterDuctus | undefined) =>
  composeHeadlineWord(word, "devanagari", font(), lookup);

describe("taking a letter's headline off", () => {
  it("finds one on every cited letter and on no sign", () => {
    const letters = devanagari.filter((letter) => /^\p{L}$/u.test(letter.glyph));
    const signs = devanagari.filter((letter) => /^\p{M}$/u.test(letter.glyph));
    expect(letters).toHaveLength(44);
    expect(signs).toHaveLength(8);
    for (const letter of letters) {
      const split = splitHeadline(letter);
      expect(split, letter.glyph).toBeDefined();
      expect(split!.body).toEqual(letter.strokes.slice(0, -1));
      expect(split!.headline.segments[0].label).toBe(LETTER_HEADLINE_LABEL);
      expect([585, 586]).toContain(split!.y);
      expect(split!.x1).toBeGreaterThan(split!.x0);
    }
    for (const sign of signs) expect(splitHeadline(sign), sign.glyph).toBeUndefined();
  });

  it("refuses a last stroke that is not just the headline", () => {
    const ma = ductusFor("म", "devanagari")!;
    const [body, headline] = ma.strokes;
    const line = headline.segments[0];
    const withLast = (last: (typeof ma.strokes)[number]): LetterDuctus => ({ ...ma, strokes: [body, last] });
    // Joined to another movement.
    expect(splitHeadline(withLast({ segments: [body.segments[0], line] }))).toBeUndefined();
    // Labelled as something else.
    expect(splitHeadline(withLast({ segments: [{ ...line, label: "draw the bar" }] }))).toBeUndefined();
    // Sloped.
    expect(
      splitHeadline(withLast({ segments: [{ ...line, path: [{ x: 0, y: 585 }, { x: 100, y: 600 }] }] })),
    ).toBeUndefined();
    // Right to left.
    expect(splitHeadline(withLast({ segments: [{ ...line, path: [...line.path].reverse() }] }))).toBeUndefined();
    // A single point.
    expect(splitHeadline(withLast({ segments: [{ ...line, path: [line.path[0]] }] }))).toBeUndefined();
    // Nothing left once it is gone.
    expect(splitHeadline({ ...ma, strokes: [headline] })).toBeUndefined();
    expect(splitHeadline({ ...ma, strokes: [] })).toBeUndefined();
  });
});

describe("composing मम", () => {
  const result = compose("मम");
  if (!result.ok) throw new Error(result.reason);
  const ma = ductusFor("म", "devanagari")!;
  const advance = font().advanceFor("म")!;

  it("keeps each letter's body, moved right by the advances before it", () => {
    expect(advance).toBe(598);
    expect(result.ductus.glyph).toBe("मम");
    expect(result.ductus.strokes).toHaveLength(3);
    expect(result.ductus.strokes[0]).toEqual(ma.strokes[0]);
    expect(result.ductus.strokes[1].segments.map((segment) => segment.label)).toEqual(
      ma.strokes[0].segments.map((segment) => segment.label),
    );
    expect(result.ductus.strokes[1].segments[0].path).toEqual(
      ma.strokes[0].segments[0].path.map((point) => ({ x: point.x + advance, y: point.y })),
    );
  });

  it("ends with ONE headline over the whole word, left to right", () => {
    const last = result.ductus.strokes[2];
    expect(last.segments).toHaveLength(1);
    expect(last.segments[0].label).toBe(WORD_HEADLINE_LABEL);
    const path = last.segments[0].path;
    expect(path[0]).toEqual({ x: 5, y: 585 });
    expect(path[path.length - 1]).toEqual({ x: advance + 610, y: 585 });
    expect(path.every((point) => point.y === 585)).toBe(true);
    for (let i = 1; i < path.length; i++) expect(path[i].x).toBeGreaterThan(path[i - 1].x);
  });

  it("lays the outline out the same way", () => {
    const glyph = font().glyphFor("म")!;
    const single = glyph.contours.flat();
    const x0 = Math.min(...single.map((point) => point.x));
    const x1 = Math.max(...single.map((point) => point.x));
    expect(result.outline.bounds.x0).toBe(x0);
    expect(result.outline.bounds.x1).toBe(x1 + advance);
  });

  it("prints the headline as the last frame, after a lift, and no headline before it", () => {
    const entry = buildFilmstripEntry(result.ductus, result.outline, "_fonts/NotoSansDevanagari-Static.ttf");
    const labels = entry.frames.map((frame) => frame.label);
    expect(labels).toEqual([
      ...ma.strokes[0].segments.map((segment) => segment.label),
      ...ma.strokes[0].segments.map((segment) => segment.label),
      WORD_HEADLINE_LABEL,
    ]);
    expect(labels.filter((label) => /shirorekha/.test(label))).toEqual([WORD_HEADLINE_LABEL]);
    expect(entry.frames[entry.frames.length - 1].startsAfterLift).toBe(true);
    // The second letter starts after a lift too.
    expect(entry.frames[3].startsAfterLift).toBe(true);
    expect(entry.summary).toBe("3 strokes · 2 pen lifts · 7 movements");
    expect(entry.penLifts).toBe(2);
  });

  it("cites the letters by position and the headline-last counts", () => {
    const source = result.ductus.source;
    expect(source.citation).toBe(
      `letters 1 and 2: ${ma.source.citation}; the shared headline, drawn last: ${HEADLINE_LAST_SOURCE.citation}`,
    );
    expect(source.citation).toContain("82% of 2,706");
    expect(source.url).toBe("https://lipitk.sourceforge.net/lipi-reco.htm");
    expect(source.variation).toContain(`<${ma.source.url}>`);
    expect(source.variation).toContain("some writers draw the headline first");
    expect(source.variation).toContain("no reachable source records native writers' headline timing in whole words");
  });
});

describe("the source of a word", () => {
  it("groups letters that share a source, in the order they first appear", () => {
    const ka = ductusFor("क", "devanagari")!;
    const ma = ductusFor("म", "devanagari")!;
    const la = ductusFor("ल", "devanagari")!;
    const source = headlineWordSource([ma, ka, ma, la]);
    expect(source.citation.startsWith(`letters 1 and 3: ${ma.source.citation}; letter 2: ${ka.source.citation}; letter 4: `)).toBe(true);
    expect(headlineWordSource([ma, ma, ma]).citation.startsWith("letters 1, 2 and 3: ")).toBe(true);
  });
});

describe("words that compose", () => {
  it("composes words of letters whose printed headlines join", () => {
    for (const word of ["नमक", "कमल", "थम", "अब", "घर", "जल", "बस", "इस", "एक"]) {
      const result = compose(word);
      expect(result.ok ? "ok" : result.reason, word).toBe("ok");
    }
  });
});

describe("every refusal", () => {
  it("names the script, the length, a sign and an uncited letter", () => {
    expect(composeHeadlineWord("மம", "tamil", font())).toEqual({
      ok: false,
      reason: "tamil words are not composed with a shared headline",
    });
    expect(compose("म")).toEqual({ ok: false, reason: "a word needs two or more letters" });
    expect(compose("मि")).toEqual({ ok: false, reason: "ि is not a single base letter" });
    expect(compose("क़म")).toEqual({ ok: false, reason: "क़ is not a single base letter" });
    expect(compose("ङम")).toEqual({ ok: false, reason: "ङ has no cited ductus" });
  });

  it("refuses a word whose one headline would cross blank paper", () => {
    // थ's own headline starts at its stem (x 394), not at its left edge.
    expect(compose("मथ")).toEqual({
      ok: false,
      reason: "the shared headline is only 88.2% on the printed word's ink (the default tolerance is over 97%)",
    });
    expect(compose("मअ").ok).toBe(false);
  });

  it("refuses a letter that does not meet the default tolerances in a word either", () => {
    // ख's own test allows 95% on ink; a word is held to the default.
    expect(compose("खम")).toEqual({
      ok: false,
      reason: "stroke 2 is only 95.1% on the printed word's ink (the default tolerance is over 97%)",
    });
  });

  it("refuses a letter with no separate headline, or one at another height", () => {
    const ma = ductusFor("म", "devanagari")!;
    const noHeadline = (glyph: string, script: string) =>
      glyph === "न" ? { ...ma, glyph: "न", strokes: ma.strokes.slice(0, 1) } : ductusFor(glyph, script);
    expect(compose("मन", noHeadline)).toEqual({ ok: false, reason: "न has no separate headline stroke" });
    const lowered = (glyph: string, script: string) => {
      const letter = ductusFor(glyph, script)!;
      if (glyph !== "न") return letter;
      const last = letter.strokes[letter.strokes.length - 1];
      const segment = last.segments[0];
      return {
        ...letter,
        strokes: [
          ...letter.strokes.slice(0, -1),
          { segments: [{ ...segment, path: segment.path.map((point) => ({ ...point, y: point.y - 10 })) }] },
        ],
      };
    };
    expect(compose("मन", lowered)).toEqual({
      ok: false,
      reason: "न's headline is not at the height of the first letter's",
    });
  });

  it("refuses a word the font cannot place, or that leaves ink untraced", () => {
    const real = font();
    const noAdvance: Font = { ...real, advanceFor: () => undefined };
    expect(composeHeadlineWord("मम", "devanagari", noAdvance)).toEqual({
      ok: false,
      reason: "the font has no outline or advance for म",
    });
    // A म whose body was never drawn: its headline still fits, but most of the
    // printed word is never traced.
    const bodyless = (glyph: string, script: string) => {
      const letter = ductusFor(glyph, script)!;
      const headline = letter.strokes[letter.strokes.length - 1];
      const line = headline.segments[0].path;
      return {
        ...letter,
        strokes: [{ segments: [{ label: "touch the headline", path: [line[0], line[1]] }] }, headline],
      };
    };
    const result = compose("मम", bodyless);
    expect(result.ok).toBe(false);
    expect(result.ok ? "" : result.reason).toMatch(/% of the printed word is never traced \(the default tolerance is under 2%\)$/);
  });
});

describe("the book and this package agree", () => {
  it("name the same headline-word scripts", () => {
    expect([...HEADLINE_WORD_SCRIPTS]).toEqual(Object.keys(BOOK_HEADLINE_WORD_SCRIPTS));
  });
});
