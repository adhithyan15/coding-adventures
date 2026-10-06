// Fused consonant + sign pairs are refused by the filmstrip composer: drawing
// the consonant and then the sign would draw a word nobody writes (Tamil டி),
// or a shape the printed font does not show (Gujarati નુ, whose ન grows a
// stem). That refusal is a claim about the script or the font, so every pair
// must stand on a cited source. `FUSED_SIGN_PAIRS` is built from
// `FUSED_SIGN_PAIR_SOURCES` alone; this file holds each source's citation to
// the pairs it carries, in both directions:
//
//   * every pair's base letter and sign are named in its citation, so a pair
//     slipped into a source that does not show it fails;
//   * every letter of the script a citation names has at least one pair under
//     it, so dropping a letter's pairs without editing the citation fails;
//   * each source's pair count is pinned, so dropping one sign of a letter
//     (ખુ but not ખૂ) fails too.
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { FUSED_SIGN_PAIR_SOURCES, FUSED_SIGN_PAIRS } from "../../src/figure-targets.js";
import { defaultCurriculumRoot } from "../../src/loader.js";

/** The Unicode block each table's pairs and cited letters come from. */
const BLOCKS: Readonly<Record<string, RegExp>> = {
  tamil: /[஀-௿]/u,
  gujarati: /[઀-૿]/u,
};

/** Letters (not signs) of `script` that a citation names. */
const citedLetters = (citation: string, script: string): string[] =>
  [...citation].filter((ch) => BLOCKS[script]!.test(ch) && /\p{L}/u.test(ch));

describe("every fused sign pair stands on a cited source", () => {
  it("builds FUSED_SIGN_PAIRS from the cited sources and nothing else", () => {
    expect(Object.keys(FUSED_SIGN_PAIRS)).toEqual(Object.keys(FUSED_SIGN_PAIR_SOURCES));
    for (const [script, sources] of Object.entries(FUSED_SIGN_PAIR_SOURCES)) {
      const cited = new Set(sources.flatMap((source) => source.pairs));
      expect([...FUSED_SIGN_PAIRS[script]!].sort(), script).toEqual([...cited].sort());
    }
  });

  it("gives every source a citation, an HTTPS URL and pairs of one letter and one sign", () => {
    for (const [script, sources] of Object.entries(FUSED_SIGN_PAIR_SOURCES)) {
      expect(sources.length, script).toBeGreaterThan(0);
      for (const source of sources) {
        expect(source.citation.trim().length, source.citation).toBeGreaterThan(40);
        expect(source.url, source.citation).toMatch(/^https:\/\/\S+$/);
        expect(source.pairs.length, source.citation).toBeGreaterThan(0);
        expect(new Set(source.pairs).size, source.citation).toBe(source.pairs.length);
        for (const pair of source.pairs) {
          expect(pair.normalize("NFD"), pair).toBe(pair);
          const [base, sign, ...rest] = [...pair];
          expect(rest, pair).toEqual([]);
          expect(base, pair).toMatch(/^\p{L}$/u);
          expect(sign, pair).toMatch(/^\p{M}$/u);
          expect(BLOCKS[script]!.test(base!) && BLOCKS[script]!.test(sign!), pair).toBe(true);
        }
      }
    }
  });

  it("names every pair's letter and sign in its own citation", () => {
    for (const sources of Object.values(FUSED_SIGN_PAIR_SOURCES)) {
      for (const source of sources) {
        for (const pair of source.pairs) {
          const [base, sign] = [...pair];
          expect(source.citation, `${pair}: letter ${base}`).toContain(base!);
          expect(source.citation, `${pair}: sign ${sign}`).toContain(sign!);
        }
      }
    }
  });

  it("has a pair for every letter a citation names", () => {
    for (const [script, sources] of Object.entries(FUSED_SIGN_PAIR_SOURCES)) {
      for (const source of sources) {
        const bases = new Set(source.pairs.map((pair) => [...pair][0]));
        for (const letter of citedLetters(source.citation, script)) {
          expect(bases.has(letter), `${letter} in "${source.citation}"`).toBe(true);
        }
      }
    }
  });

  it("pins each source's pairs to the source the code comment names", () => {
    const tamil = FUSED_SIGN_PAIR_SOURCES.tamil!;
    expect(tamil.map((source) => [source.pairs.length, source.url])).toEqual([
      [3, "https://www.unicode.org/versions/Unicode17.0.0/core-spec/chapter-12/"],
    ]);
    expect(tamil[0]!.citation).toMatch(
      /^The Unicode Standard, Version 17\.0, §12\.6\.3 Tamil Ligatures, Ligatures with Vowel i and Figure 12-21: /,
    );
    expect(tamil[0]!.pairs).toEqual(["டி", "டீ", "லீ"]);

    const gujarati = FUSED_SIGN_PAIR_SOURCES.gujarati!;
    // 22 stem-form consonants x (ુ, ૂ); રુ રૂ ણુ; જ and ૹ x (ા ી ો ૌ).
    expect(gujarati.map((source) => source.pairs.length)).toEqual([44, 3, 8]);
    expect(gujarati.map((source) => source.citation.match(/GSUB '(\w+)'/)?.[1])).toEqual([
      "blws",
      "blws",
      "psts",
    ]);
  });

  it("cites the Gujarati font that is actually bundled", () => {
    // The GSUB lookups were read from this file. If the font is replaced, its
    // version string changes and the citations must be read again.
    const fonts = join(defaultCurriculumRoot(), "_fonts");
    const font = readFileSync(join(fonts, "NotoSansGujarati-Static.ttf"));
    for (const source of FUSED_SIGN_PAIR_SOURCES.gujarati!) {
      expect(source.citation).toContain(
        "Noto Sans Gujarati Version 2.106, bundled as learning/human-languages/_fonts/NotoSansGujarati-Static.ttf",
      );
    }
    // The OpenType name table stores the version string as UTF-16BE.
    const version = Buffer.from("Version 2.106", "utf16le").swap16();
    expect(font.includes(version)).toBe(true);
  });
});
