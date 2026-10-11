// ---------------------------------------------------------------------------
// headline-word.test.ts — a Devanagari word: letters' bodies, then one headline
// ---------------------------------------------------------------------------
//
// What is pinned here:
//
//   * every cited Devanagari letter has a headline that can be taken off on its
//     own, and so does the ā sign (its piece of headline), and no other sign;
//   * ā joins a word only straight after a consonant: its stem after the
//     consonant's body, its piece of headline inside the word's one headline,
//     and its place cited from its mark record;
//   * a phrase is composed word by word, each word with its own headline, and
//     one failing word refuses the phrase (the height cap on phrases is
//     measured in filmstrip-ledger.test.ts, under the ledger's budget);
//   * a composed word keeps each letter's body exactly (moved by the font's
//     advances) and ends with ONE headline over the whole word;
//   * the strip's last frame is that headline, after a lift, and no earlier
//     frame draws a headline;
//   * every refusal reason, each from a word that triggers it.
// ---------------------------------------------------------------------------

import { describe, expect, it } from "vitest";

import {
  HEADLINE_WORD_SCRIPTS as BOOK_HEADLINE_WORD_SCRIPTS,
  HEADLINE_WORD_SIGNS as BOOK_HEADLINE_WORD_SIGNS,
} from "@coding-adventures/human-language-data/src/figure-targets.ts";
import devanagariData from "../../../../learning/human-languages/data/scripts/devanagari.json";
import { ductusFor } from "../src/ductusview";
import { buildFilmstripEntry } from "../src/filmstrip-ledger";
import {
  composeHeadlinePhrase,
  composeHeadlineWord,
  HEADLINE_LAST_SOURCE,
  HEADLINE_WORD_CONSONANTS,
  HEADLINE_WORD_SCRIPTS,
  HEADLINE_WORD_SIGNS,
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
  it("finds one on every cited letter, on the ā sign, and on no other sign", () => {
    const letters = devanagari.filter((letter) => /^\p{L}$/u.test(letter.glyph));
    const aa = devanagari.filter((letter) => letter.glyph === "\u093E");
    const signs = devanagari.filter((letter) => /^\p{M}$/u.test(letter.glyph) && letter.glyph !== "\u093E");
    expect(letters).toHaveLength(44);
    expect(aa).toHaveLength(1);
    // ु ू े ं ़ ् ृ ँ, and ी ो ः: the last three leave the piece of headline
    // Noto prints on them undrawn (it is the word's headline), so none of the
    // eleven ends in a headline stroke to take off.
    expect(signs).toHaveLength(11);
    // ā's last stroke is the piece of headline Noto prints on the sign.
    const split = splitHeadline(aa[0])!;
    expect(split.body).toEqual(aa[0].strokes.slice(0, 1));
    expect([split.y, split.x0, split.x1]).toEqual([585, 5, 268]);
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
  // One case per word, not one loop over all nine. Each composition traces
  // every stroke against the printed word's ink, about a quarter of a second
  // on an idle machine; nine in one case took over two seconds, and on a loaded
  // runner that crossed the 5s per-test budget while every word still composed.
  // Split, each case costs one word, and a failure names its word in the title.
  it.each(["नमक", "कमल", "थम", "अब", "घर", "जल", "बस", "इस", "एक"])(
    "composes %s, whose letters' printed headlines join",
    (word) => {
      const result = compose(word);
      expect(result.ok ? "ok" : result.reason, word).toBe("ok");
    },
  );
});

describe("every refusal", () => {
  it("names the script, the length, a sign and an uncited letter", () => {
    expect(composeHeadlineWord("மம", "tamil", font())).toEqual({
      ok: false,
      reason: "tamil words are not composed with a shared headline",
    });
    expect(compose("म")).toEqual({ ok: false, reason: "a word needs two or more characters" });
    expect(compose("मि")).toEqual({
      ok: false,
      reason: "ि is neither a single base letter nor a sign with a cited place in a word",
    });
    expect(compose("क़म")).toEqual({
      ok: false,
      reason: "क़ is neither a single base letter nor a sign with a cited place in a word",
    });
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

describe("composing नाम: the ā sign between two letters", () => {
  const result = compose("नाम");
  if (!result.ok) throw new Error(result.reason);
  const na = ductusFor("न", "devanagari")!;
  const aa = ductusFor("ा", "devanagari")!;
  const ma = ductusFor("म", "devanagari")!;
  const naAdvance = font().advanceFor("न")!;
  const aaAdvance = font().advanceFor("ा")!;

  it("draws न's body, then ā's stem, then म's body, then ONE headline", () => {
    expect([naAdvance, aaAdvance]).toEqual([555, 259]);
    const strokes = result.ductus.strokes;
    expect(strokes).toHaveLength(na.strokes.length - 1 + 1 + ma.strokes.length - 1 + 1);
    expect(strokes.slice(0, 2)).toEqual(na.strokes.slice(0, 2));
    // The stem, moved right by न's advance, and nothing else of the sign.
    expect(strokes[2].segments.map((segment) => segment.label)).toEqual(["draw the stem straight down"]);
    expect(strokes[2].segments[0].path).toEqual(
      aa.strokes[0].segments[0].path.map((point) => ({ x: point.x + naAdvance, y: point.y })),
    );
    expect(strokes[3].segments[0].path[0]).toEqual({
      x: ma.strokes[0].segments[0].path[0].x + naAdvance + aaAdvance,
      y: ma.strokes[0].segments[0].path[0].y,
    });
    const headline = strokes[strokes.length - 1].segments;
    expect(headline.map((segment) => segment.label)).toEqual([WORD_HEADLINE_LABEL]);
    // From न's headline start to म's headline end, across the sign's piece.
    expect(headline[0].path[0]).toEqual({ x: 5, y: 585 });
    expect(headline[0].path.at(-1)).toEqual({ x: naAdvance + aaAdvance + 610, y: 585 });
  });

  it("prints exactly one headline, last, and the stem after a lift", () => {
    const entry = buildFilmstripEntry(result.ductus, result.outline, "_fonts/NotoSansDevanagari-Static.ttf");
    const labels = entry.frames.map((frame) => frame.label);
    expect(labels.filter((label) => /shirorekha/.test(label))).toEqual([WORD_HEADLINE_LABEL]);
    expect(labels.at(-1)).toBe(WORD_HEADLINE_LABEL);
    expect(entry.frames[2]).toMatchObject({ label: "draw the stem straight down", startsAfterLift: true });
    expect(entry.summary).toBe("5 strokes · 4 pen lifts · 7 movements");
  });

  it("names the sign by position and cites its place, from the mark record", () => {
    const place = (devanagariData.marks as Array<{ mark: string; compositionSource?: unknown }>).find(
      (mark) => mark.mark === "ा",
    )!.compositionSource as { citation: string; url: string };
    expect(HEADLINE_WORD_SIGNS.devanagari!.get("ा")).toEqual(place);
    expect(place.citation).toContain("Devanagari आ stroke order.svg");
    expect(result.ductus.source.citation).toBe(
      `letter 1: ${na.source.citation}; sign 2: ${aa.source.citation}; letter 3: ${ma.source.citation}; ` +
        `the place of sign 2, after its consonant's body: ${place.citation}; ` +
        `the shared headline, drawn last: ${HEADLINE_LAST_SOURCE.citation}`,
    );
    expect(result.ductus.source.variation).toContain(`<${place.url}>`);
    expect(result.ductus.source.variation).toContain("at medium confidence");
    expect(result.ductus.source.variation).toContain(
      "the sign's own piece of headline becomes part of the word's one headline",
    );
  });

  it("groups two ā signs under one citation and one place", () => {
    const source = headlineWordSource(["न", "ा", "म", "ा"].map((glyph) => ductusFor(glyph, "devanagari")!));
    expect(source.citation).toContain("signs 2 and 4: ");
    expect(source.citation).toContain("the place of signs 2 and 4, each after its consonant's body: ");
  });

  it("composes ā after a consonant, at the end of a word too", () => {
    for (const word of ["सा", "मा", "नामा", "राम", "कमला"]) {
      const composed = compose(word);
      expect(composed.ok ? "ok" : composed.reason, word).toBe("ok");
    }
  });

  it("refuses ā anywhere but straight after a consonant", () => {
    // After a vowel letter it is no written syllable (after अ the font prints a
    // dotted circle); first, or after another sign, it has no consonant at all.
    for (const word of ["अाम", "ाम", "नाा", "इा"]) {
      expect(compose(word), word).toEqual({ ok: false, reason: "ा must follow a consonant" });
    }
    expect(HEADLINE_WORD_CONSONANTS.devanagari!.test("क")).toBe(true);
    expect(HEADLINE_WORD_CONSONANTS.devanagari!.test("ह")).toBe(true);
    expect(HEADLINE_WORD_CONSONANTS.devanagari!.test("अ")).toBe(false);
  });

  it("keeps refusing every other sign", () => {
    for (const sign of ["े", "ो", "ी", "ं", "ु", "ि", "ः"]) {
      expect(compose(`म${sign}`), sign).toEqual({
        ok: false,
        reason: `${sign} is neither a single base letter nor a sign with a cited place in a word`,
      });
    }
  });
});

describe("composing a phrase, word by word", () => {
  const phrase = (text: string, lookup?: (glyph: string, script: string) => LetterDuctus | undefined) =>
    composeHeadlinePhrase(text, "devanagari", font(), lookup);

  it("composes each word of मम नाम on its own, each with its own headline", () => {
    const result = phrase("मम नाम");
    if (!result.ok) throw new Error(result.reason);
    expect(result.words.map((word) => word.glyph)).toEqual(["मम", "नाम"]);
    for (const word of result.words) {
      const alone = compose(word.glyph);
      if (!alone.ok) throw new Error(alone.reason);
      expect(word.ductus).toEqual(alone.ductus);
      expect(word.outline).toEqual(alone.outline);
      expect(word.ductus.strokes.at(-1)!.segments[0].label).toBe(WORD_HEADLINE_LABEL);
    }
  });

  it("draws a one-letter word as that letter's own strip", () => {
    const result = phrase("न मम");
    if (!result.ok) throw new Error(result.reason);
    expect(result.words[0].glyph).toBe("न");
    expect(result.words[0].ductus).toBe(ductusFor("न", "devanagari"));
    expect(result.words[0].outline.path).toBe(font().glyphFor("न")!.path);
  });

  it("refuses the whole phrase when one word fails, naming the word", () => {
    expect(phrase("मम मथ")).toEqual({
      ok: false,
      reason:
        "word 2 (मथ): the shared headline is only 88.2% on the printed word's ink (the default tolerance is over 97%)",
    });
    expect(phrase("नाम: मीरा")).toEqual({
      ok: false,
      reason: "word 1 (नाम:): : is neither a single base letter nor a sign with a cited place in a word",
    });
    expect(phrase("ि मम")).toEqual({ ok: false, reason: "word 1 (ि): ि is not a cited base letter" });
    expect(phrase("ङ मम")).toEqual({ ok: false, reason: "word 1 (ङ): ङ is not a cited base letter" });
  });

  it("takes words separated by single spaces and nothing else", () => {
    expect(phrase("मम")).toEqual({ ok: false, reason: "a phrase needs two or more words" });
    for (const text of ["मम  नाम", " मम नाम", "मम नाम ", "मम नाम क", "मम\tनाम क"]) {
      expect(phrase(text), JSON.stringify(text)).toEqual({
        ok: false,
        reason: "a phrase's words are separated by single spaces and nothing else",
      });
    }
    expect(composeHeadlinePhrase("மம நம", "tamil", font())).toEqual({
      ok: false,
      reason: "tamil words are not composed with a shared headline",
    });
  });
});

describe("the book and this package agree", () => {
  it("name the same headline-word scripts", () => {
    expect([...HEADLINE_WORD_SCRIPTS]).toEqual(Object.keys(BOOK_HEADLINE_WORD_SCRIPTS));
  });

  it("name the same signs a word may hold", () => {
    expect(Object.keys(HEADLINE_WORD_SIGNS)).toEqual(Object.keys(BOOK_HEADLINE_WORD_SIGNS));
    for (const [script, signs] of Object.entries(HEADLINE_WORD_SIGNS)) {
      expect([...signs.keys()]).toEqual([...BOOK_HEADLINE_WORD_SIGNS[script]!]);
    }
  });
});
