// Devanagari vowel signs and other marks get a filmstrip only where a lesson
// teaches the sign BY ITSELF. Their cited source — native writers' pen traces
// in HP Labs India's LipiTk Devanagari recognizer — was written one sign at a
// time, with no consonant and no headline, so it says how the sign is drawn
// but not when it is drawn against its consonant or the shared headline.
// So Devanagari has no row in the composer's written-order table: a sign on a
// consonant ("कि", "कु") is never drawn letter by letter, and the mark records
// claim no written order beyond the nukta's existing carrier-first convention
// and ā's. ā's place (after its consonant's body, before the headline) is
// cited to the cited आ, which draws the same bar that way; it is used only
// inside a word composed with one shared headline (devanagari-words-share-
// one-headline).
import { describe, expect, it } from "vitest";
import {
  filmstripCandidates,
  FUSED_SIGN_PAIRS,
  withDerivedFilmstrips,
  WRITTEN_SIGN_SIDES,
  writingLetterOf,
  writingSequenceOf,
  writtenPiecesOf,
} from "../../src/figure-targets.js";
import { defaultCurriculumRoot, loadScripts } from "../../src/loader.js";
import { lesson } from "./fixture.js";

const LIPITK = "https://lipitk.sourceforge.net/lipi-reco.htm";
const hindi = (id: string, headword: string) => lesson(id, { language: "hindi", headword });
/** The nine signs that carry a cited ductus, plus two letters. */
const CITED = new Set(["ा", "ु", "ू", "े", "ं", "़", "्", "ृ", "ँ", "क", "म"]);
const hasDuctus = (script: string, glyph: string) => script === "devanagari" && CITED.has(glyph);

describe("Devanagari signs drawn alone", () => {
  it("gives Devanagari no written-order row, so no sign is placed against a consonant", () => {
    expect(WRITTEN_SIGN_SIDES.devanagari).toBeUndefined();
    expect(FUSED_SIGN_PAIRS.devanagari).toBeUndefined();
    for (const grapheme of ["कि", "कु", "के", "कं", "क्", "कृ", "कँ", "क़", "ु", "े"]) {
      expect(writtenPiecesOf(grapheme, "devanagari"), grapheme).toBeUndefined();
    }
  });

  it("takes a sign taught alone as one glyph, never as a sequence", () => {
    for (const sign of ["ा", "ु", "ू", "े", "ं", "़", "्", "ृ", "ँ"]) {
      expect(writingSequenceOf(hindi("HI-1", sign), "devanagari"), sign).toBeUndefined();
      expect(writingLetterOf(hindi("HI-1", sign)), sign).toBe(sign);
    }
  });

  it("draws the bare sign and refuses the same sign on a consonant, or in a word", () => {
    const candidates = filmstripCandidates([
      hindi("HI-W1", "ु"),
      hindi("HI-W2", "कु"),
      hindi("HI-W3", "मेरा"),
      hindi("HI-W4", "कि"),
    ]);
    // A consonant-plus-sign headword is still one candidate glyph ("कु"), but
    // no ductus is cited for that pair, so it is refused; a word is not even
    // a candidate.
    expect(candidates.map((target) => [target.lessonId, target.glyph, target.letters])).toEqual([
      ["HI-W1", "ु", undefined],
      ["HI-W2", "कु", undefined],
      ["HI-W4", "कि", undefined],
    ]);
    expect(withDerivedFilmstrips([], candidates, hasDuctus).map((target) => target.lessonId)).toEqual([
      "HI-W1",
    ]);
  });

  it("cites a ductus on exactly the nine signs, and a place only for ā and the nukta", () => {
    const script = loadScripts(defaultCurriculumRoot()).devanagari!;
    const cited = (script.marks ?? []).filter((mark) => mark.strokeOrderSource !== undefined);
    expect(cited.map((mark) => mark.mark)).toEqual(["ा", "ु", "ू", "े", "ं", "़", "्", "ृ", "ँ"]);
    for (const mark of cited) {
      expect(mark.strokeOrderSource?.url, mark.mark).toBe(LIPITK);
      expect(mark.strokeOrderSource?.variation, mark.mark).toMatch(
        mark.mark === "ा"
          ? /written by itself, with no consonant beside it.*that place is cited separately \(compositionSource\), and it is the one Devanagari sign a composed word may hold/
          : /written by itself, with no consonant beside it.*composes no Devanagari word from it/,
      );
    }
    // The nukta carries the carrier-first convention cited to Unicode before
    // the traces were; ā carries its place after its consonant's body and
    // before the headline, cited to the cited आ, at medium confidence.
    const ordered = (script.marks ?? []).filter((mark) => mark.compositionOrder !== undefined);
    expect(ordered.map((mark) => mark.mark)).toEqual(["ा", "़"]);
    const [aa, nukta] = ordered;
    expect(aa!.compositionSource?.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Devanagari_%E0%A4%86_stroke_order.svg",
    );
    expect(aa!.compositionSource?.variation).toContain("at medium confidence");
    expect(aa!.compositionOrder).toEqual([
      "write the consonant's body first, without its headline",
      "draw the ā stem straight down, to the right of the consonant",
      "draw the headline last, across the consonant and the sign",
    ]);
    expect(nukta!.compositionSource?.url).toMatch(/^https:\/\/www\.unicode\.org\//);
  });
});
