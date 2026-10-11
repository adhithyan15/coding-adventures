// The Latin script's first strips: print letters stand apart, so a word of
// cited letters is drawn letter by letter, a list of marks ("¿ ¡") is drawn
// mark by mark, and ñ is one precomposed letter with a ductus of its own.
// What is refused, and why: a word with punctuation inside it (¿ and ? are not
// base letters); ñ typed as n plus a combining tilde (Latin has no written-order
// table); and every lesson with a letter the ledger has no ductus for (œ ...;
// the ledger, not this module, says which letters are drawn). The a was refused
// too until the strips moved to an outline that prints the one-storey a every
// source teaches (LatinPrint-Subset.ttf, from SIL's Andika).
import { describe, expect, it } from "vitest";
import {
  DERIVED_FILMSTRIP_SCRIPTS,
  filmstripCandidates,
  filmstripImageMarkdown,
  MAX_SEQUENCE_PIECES,
  SEPARATE_LETTER_SCRIPTS,
  WRITTEN_SIGN_SIDES,
  withDerivedFilmstrips,
  writingLetterOf,
  writingSequenceOf,
} from "../../src/figure-targets.js";
import { lesson } from "./fixture.js";

/** The 31 glyphs the Latin owner cited first (this case predates v m R and the analogy letters). */
const CITED = new Set([..."bceghilnorsuwßñG¿¡adpqtyHáéíóúü"]);
const hasDuctus = (script: string, glyph: string) => script === "latin" && CITED.has(glyph);

describe("Latin print letters", () => {
  it("switches on all six Latin-script tracks, and lets Latin words stand apart", () => {
    for (const track of ["spanish", "german", "french", "italian", "portuguese", "latin"]) {
      expect(DERIVED_FILMSTRIP_SCRIPTS[track], track).toBe("latin");
    }
    expect(SEPARATE_LETTER_SCRIPTS.has("latin")).toBe(true);
    expect(WRITTEN_SIGN_SIDES.latin).toBeUndefined();
  });

  it("reads a word letter by letter, capitals and ß included", () => {
    expect(writingSequenceOf(lesson("GE-W1", { language: "german", headword: "weil" }), "latin")).toEqual([
      "w", "e", "i", "l",
    ]);
    expect(
      writingSequenceOf(lesson("GE-W2", { language: "german", headword: "Großschreibung" }), "latin"),
    ).toEqual([..."Großschreibung"]);
  });

  it("offers no strip for a sequence longer than MAX_SEQUENCE_PIECES", () => {
    // writingSequenceOf still reads the word; filmstripCandidates drops it,
    // because a 14-piece strip shrinks past legibility in the book.
    expect(MAX_SEQUENCE_PIECES).toBe(10);
    const long = lesson("GE-W2", { language: "german", headword: "Großschreibung" });
    const short = lesson("GE-W1", { language: "german", headword: "weil" });
    expect(filmstripCandidates([long, short]).map((target) => target.lessonId)).toEqual(["GE-W1"]);
  });

  it("keeps a precomposed ñ whole, and refuses n typed with a combining tilde", () => {
    expect(writingLetterOf(lesson("ES-W1", { language: "spanish", headword: "ñ" }))).toBe("ñ");
    expect(writingSequenceOf(lesson("ES-W2", { language: "spanish", headword: "año" }), "latin")).toEqual([
      "a", "ñ", "o",
    ]);
    const decomposed = "año";
    expect(decomposed.normalize("NFC")).toBe("año");
    expect(writingSequenceOf(lesson("ES-W3", { language: "spanish", headword: decomposed }), "latin")).toBeUndefined();
  });

  it("draws a list of opening marks, but no word with punctuation inside it", () => {
    expect(writingSequenceOf(lesson("ES-W4", { language: "spanish", headword: "¿ ¡" }), "latin")).toEqual([
      "¿", "¡",
    ]);
    expect(writingSequenceOf(lesson("ES-W5", { language: "spanish", headword: "¿cómo?" }), "latin")).toBeUndefined();
  });

  it("draws a candidate only when every letter is cited, the one-storey a included", () => {
    const candidates = filmstripCandidates([
      lesson("ES-W6", { language: "spanish", headword: "hola" }),
      lesson("ES-W7", { language: "spanish", headword: "á é í ó ú" }),
      lesson("GE-W8", { language: "german", headword: "Hallo" }),
      lesson("GE-W9", { language: "german", headword: "ä ö ü" }),
      lesson("FR-W1", { language: "french", headword: "salut" }),
      lesson("FR-W2", { language: "french", headword: "é è ê" }),
      lesson("FR-W3", { language: "french", headword: "ç" }),
    ]);
    expect(candidates.map((candidate) => candidate.lessonId)).toEqual([
      "ES-W6", "ES-W7", "FR-W1", "FR-W2", "FR-W3", "GE-W8", "GE-W9",
    ]);
    // This fixture's ledger has no ductus for ä ö, è ê or ç, so those lessons print no strip.
    const drawn = withDerivedFilmstrips([], candidates, hasDuctus);
    expect(drawn.map((target) => (target.kind === "script-filmstrip" ? target.lessonId : ""))).toEqual([
      "ES-W6",
      "ES-W7",
      "FR-W1",
      "GE-W8",
    ]);
  });

  it("captions a word by its spelling and a list of marks by its letters", () => {
    const [weil, marks] = filmstripCandidates([
      lesson("GE-W1", { language: "german", headword: "weil" }),
      lesson("GE-W2", { language: "german", headword: "¿ ¡" }),
    ]);
    expect(filmstripImageMarkdown(weil!)).toBe(
      "![How weil is written, letter by letter, stroke by stroke](figures/GE-W1-filmstrip.svg)",
    );
    expect(filmstripImageMarkdown(marks!)).toBe(
      "![How these letters are written, one after another, stroke by stroke: ¿, ¡](figures/GE-W2-filmstrip.svg)",
    );
  });
});
