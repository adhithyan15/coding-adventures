// ---------------------------------------------------------------------------
// nasal-macron.test.ts — the long nasal vowel has to reach the page.
//
// WHAT WAS WRONG
// Seven tracks romanize a LONG NASAL vowel as a macron vowel plus U+0303
// COMBINING TILDE — "gā̃v", "nahī̃", "tū̃" — 346 times in the generated books
// and 1,837 times across the whole corpus, the rest of which is lesson
// frontmatter and narration script that is never typeset.
// Latin Modern SET THAT AS THE SHORT NASAL VOWEL: the macron was dropped, so
// "gā̃v" printed "gãv" and a reader of the PDF could not tell a long nasal vowel
// from a short one anywhere in seven books.
//
// WHY NOTHING CAUGHT IT
// Every gate in this package reads bytes, and the bytes were right. The lesson
// wrote U+0101 U+0303, the generator emitted U+0101 U+0303, the hash ledger
// agreed, `glyph-coverage` found both characters in the font's cmap, and
// XeLaTeX reported no missing character — because nothing was missing. What
// failed was MARK STACKING: Latin Modern has no anchor for a second above-mark
// on a base that already carries one, and the shaper drops the first rather
// than raising the second. The only witness is the rendered page.
//
// WHAT THIS TEST CAN AND CANNOT DO
// It cannot compile a book; that is `code/scripts/check-book-compile.sh`, which
// is deliberately outside `vitest`. What it CAN do is make sure the repair is
// present wherever the corpus needs it — which is the half that rots. The
// tracks are DERIVED from the book text, not listed here: a track that starts
// using the sequence tomorrow is required to carry the fix tomorrow, and a list
// would have to be remembered.
// ---------------------------------------------------------------------------
import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { loadBookFonts } from "../src/loader.js";

/** A macron vowel immediately followed by U+0303. The thing that did not set. */
const LONG_NASAL = /[āīū]̃/u;
const SHARED_INPUT = "_shared/nasal-macron.tex";

const SHARED_SOURCE = new URL(
  "../../../../learning/human-languages/_shared/nasal-macron.tex",
  import.meta.url,
);

describe("the long nasal vowel", () => {
  const books = loadBookFonts();

  it("finds books at all, so an empty walk cannot satisfy the gate below", () => {
    expect(books.length).toBeGreaterThan(20);
    expect(books.some((book) => book.files.length > 0)).toBe(true);
  });

  it("gives EVERY track that writes one the preamble that can set it", () => {
    // Derived twice over: which tracks need it comes from the generated book
    // text, and whether they have it comes from the preamble. Neither side is a
    // list somebody has to keep, so a track cannot fall out of the set by being
    // forgotten — only by no longer writing the sequence.
    const needs = books
      .filter((book) => book.files.some((file) => LONG_NASAL.test(file.text)))
      .map((book) => book.language)
      .sort();
    // The set as measured today, so the test also reports when the convention
    // spreads or retreats. Seven Indo-Aryan tracks; no other family uses it.
    expect(needs).toEqual([
      "bengali", "gujarati", "hindi", "marathi", "marwadi", "punjabi", "urdu",
    ]);
    const missing = needs.filter(
      (language) => !books.find((book) => book.language === language)!.preamble.includes(SHARED_INPUT),
    );
    expect(missing, "tracks writing a long nasal vowel with no preamble to set it").toEqual([]);
  }, 60_000);

  it("inputs it AFTER hyperref, which is the dependency it actually has", () => {
    // The first draft of this test asserted ordering against `visual.tex`,
    // reasoning that graphicx was needed and that visual.tex follows hyperref in
    // every preamble. Both halves were wrong: `\raisebox` is a kernel command,
    // not graphicx, and asserting a proxy means a track that moves visual.tex
    // above hyperref keeps this green while the build breaks. The real
    // dependency is `\pdfstringdefDisableCommands`, which is hyperref's.
    for (const book of books) {
      const shared = book.preamble.indexOf(SHARED_INPUT);
      if (shared === -1) continue;
      const hyperref = book.preamble.indexOf("{hyperref}");
      expect(hyperref, `${book.language} inputs the fix but never loads hyperref`).toBeGreaterThan(-1);
      expect(hyperref, `${book.language} inputs the fix before hyperref`).toBeLessThan(shared);
    }
  });

  it("IS THE LAST WORD on those characters, because a later one wins silently", () => {
    // The finding of this change's security review, and the only way the fix
    // can be undone with no evidence anywhere. `newunicodechar`'s Unicode branch
    // has NO redefinition warning — the "Redefining Unicode character" message
    // exists only in its 8-bit branch — so a later `\newunicodechar{ā}` simply
    // wins. The rendered page then goes back to printing the short vowel, byte
    // for byte identical to the unfixed bug, with no error and no warning.
    //
    // The tripwire is already in the tree: Marwadi's preamble declares
    // `\newunicodechar{ā}{\={a}}` before the input, and reversing those two
    // lines is enough. So the ordering is asserted rather than trusted.
    for (const book of books) {
      const shared = book.preamble.indexOf(SHARED_INPUT);
      if (shared === -1) continue;
      const after = book.preamble.slice(shared);
      const clobbers = [...after.matchAll(/\\newunicodechar\{(.)\}/gu)]
        .map((match) => match[1]!)
        .filter((character) => ["\u0101", "\u012B", "\u016B", "\u0303"].includes(character));
      expect(
        clobbers,
        `${book.language} redefines a character the shared fix owns, AFTER inputting it`,
      ).toEqual([]);
    }
  });

  it("keeps the mark as U+0303 itself, which is what the text layer carries", () => {
    // Two earlier drafts of the shared file failed here in ways the page did
    // not show. A scaled `\textasciitilde` looked right and put an ASCII tilde
    // in the PDF's text layer, so `pdftotext` read "sāp" and the nasalization
    // vanished from every copy, search and screen reader. Centring that tilde in
    // a box the width of the base then displaced it by a whole letter in the
    // Punjabi table of contents, because a combining mark's ink sits LEFT of its
    // origin. Setting U+0303 directly after the vowel fixes both at once, and
    // this pins the choice so a later "simplification" has to argue with it.
    // Comments STRIPPED before asserting. The file's own prose names both of
    // the rejected constructions in order to explain them, and a naive
    // `not.toContain` reads the explanation as the code — a test that fails on
    // a file for saying why it does not do the thing it does not do.
    const source = readFileSync(SHARED_SOURCE, "utf8")
      .split("\n")
      .filter((line) => !line.trimStart().startsWith("%"))
      .join("\n");
    expect(source).toContain('\\char"0303');
    expect(source).not.toContain("textasciitilde");
    expect(source).not.toContain("\\llap");
    // All three macron vowels the corpus nasalizes, not just the one that was
    // reported. ī and ū are 991 of the 1,840 occurrences between them.
    for (const vowel of ["ā", "ī", "ū"]) {
      expect(source, `${vowel} has no lookahead`).toContain(`\\newunicodechar{${vowel}}`);
    }
    // And the bare mark still reaches the font, because a combining tilde that
    // follows something OTHER than a macron vowel already set correctly — h̃ and
    // ɛ̃ both occur — and this file must not break what worked.
    expect(source).toContain("\\newcommand{\\hlcombiningtilde}{\\char\"0303\\relax}");
  });
});
