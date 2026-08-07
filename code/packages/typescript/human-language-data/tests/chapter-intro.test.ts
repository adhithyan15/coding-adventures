// HL09 §8 — every chapter opens by saying what the reader will be able to do.

import { describe, expect, it } from "vitest";
import { generatedBookOutputs } from "../src/book-cli.js";
import { loadTrackChapters } from "../src/loader.js";
import { renderBookChapter } from "../src/book.js";
import { loadLessons } from "../src/loader.js";

// `.tex` only: generatedBookOutputs also carries the generated hashes JSON, which is
// not a chapter and has no opening.
const generated = [...generatedBookOutputs().entries()]
  .filter(([path]) => path.endsWith(".tex"))
  .map(([, tex]) => tex);

describe("the chapter opening", () => {
  it("is present on every generated chapter that has a capability", () => {
    // 288 of 407 chapters used to open on a bare title: \chapter{}, \label{}, then
    // straight into the first lesson. Nothing told the reader why they were here.
    //
    // The count is asserted as a FLOOR, not an equality: main lands new generated
    // targets regularly, and pinning the exact number just breaks this test on
    // somebody else's unrelated chapter.
    expect(generated.length).toBeGreaterThanOrEqual(311);

    // A chapter with no HL05 capability gets no opening rather than an invented one,
    // so the gap is capability debt the gap report already counts — not a rendering
    // bug. Asserted by NAME so it shrinks visibly instead of hiding behind a number.
    const chapters = loadTrackChapters();
    const hasCapability = (language: string, chapter: number) =>
      Boolean(
        chapters
          .find((t) => t.language === language)
          ?.chapters.find((c) => c.chapter === chapter)?.canDo,
      );
    const withoutOpening = [...generatedBookOutputs().entries()]
      .filter(([path]) => path.endsWith(".tex"))
      .filter(([, tex]) => !tex.includes("\\begin{chapteropening}"))
      .map(([path]) => path);

    for (const path of withoutOpening) {
      const match = /^([^/]+)\/book\/chapters\/ch(\d+)-/.exec(path);
      expect(match).not.toBeNull();
      expect(hasCapability(match![1]!, Number(match![2]))).toBe(false);
    }
    // Russian chapter 3 is the whole of the debt today.
    expect(withoutOpening).toEqual(["russian/book/chapters/ch03-first-verbs.tex"]);
  });

  it("never points at another track of this course", () => {
    // The book is a standalone artifact and English is its only requirement, so an
    // intro may not lean on a track the reader may not have. Etymology is NOT this:
    // "negro inherited from Latin" names a source language, which is the point of
    // the book. "the Hindi track shows" names a book the reader does not have.
    const dangling =
      /(the\s+\w+\s+track|other\s+tracks?|this\s+course|as\s+in\s+the\s+\w+\s+(?:track|book))/i;
    const offenders: string[] = [];
    for (const tex of generated) {
      const opening = /\\begin\{chapteropening\}([\s\S]*?)\\end\{chapteropening\}/.exec(tex);
      if (opening && dangling.test(opening[1]!)) offenders.push(opening[1]!.slice(0, 80));
    }
    expect(offenders).toEqual([]);
  });

  it("still names source languages, because that is the book's signature", () => {
    // The guard above must not have been bought by stripping etymology.
    const withEtymology = generated.filter((tex) => {
      const opening = /\\begin\{chapteropening\}([\s\S]*?)\\end\{chapteropening\}/.exec(tex);
      return opening ? /\b(Latin|Sanskrit|Arabic|Greek|Germanic)\b/.test(opening[1]!) : false;
    });
    expect(withEtymology.length).toBeGreaterThan(50);
  });

  it("is omitted, not faked, when a chapter has no capability", () => {
    // A chapter with no HL05 entry gets no opening rather than an invented one.
    const lessons = loadLessons();
    const spanish = lessons.find((l) => l.language === "spanish")!;
    const target = {
      language: "spanish",
      chapter: spanish.realization.chapter,
      title: "T",
      label: "ch:t",
      output: "spanish/book/chapters/ch99-t.tex",
    };
    const withoutCapability = renderBookChapter(target, lessons);
    expect(withoutCapability.tex).not.toContain("chapteropening");
  });

  it("keeps the reconstruction asterisk, which marks a form as UNATTESTED", () => {
    // `renderInlineMarkdown` reads a bare `*` as an italic opener and DELETES it, so
    // `PIE *ne` printed as `PIE ` with the rest italicised — turning a reconstructed
    // form into an attested one, a false claim in the part of the book that exists
    // for etymology. The ledger escapes it as `\\*`; this proves it survives.
    const withPie = generated.filter((tex) => {
      const opening = /\\begin\{chapteropening\}([\s\S]*?)\\end\{chapteropening\}/.exec(tex);
      return opening ? /PIE\s+\*/.test(opening[1]!) : false;
    });
    expect(withPie.length).toBeGreaterThan(0);
    // And no opening may contain a DANGLING emphasis that swallowed one.
    for (const tex of generated) {
      const opening = /\\begin\{chapteropening\}([\s\S]*?)\\end\{chapteropening\}/.exec(tex);
      if (!opening) continue;
      expect(opening[1]).not.toMatch(/PIE\s+\\emph\{/);
    }
  });

  it("never explains to the reader how the chapter was built", () => {
    // Four books printed "Chapter 17 has no terminal practice lesson, so the payoff
    // is the last lesson by sequence (4 of 12 atoms, below the 0.5 floor)" under the
    // title. That sentence is addressed to the gap report. Books do not describe
    // their own build system — the same rule that got the old blurb removed.
    const tooling = /terminal practice|payoff is the last lesson|below the .*floor|atoms\)/i;
    const offenders: string[] = [];
    for (const tex of generated) {
      const opening = /\\begin\{chapteropening\}([\s\S]*?)\\end\{chapteropening\}/.exec(tex);
      if (opening && tooling.test(opening[1]!)) offenders.push(opening[1]!.slice(0, 70));
    }
    expect(offenders).toEqual([]);
  });

  it("quotes canDo verbatim, so the book and the ledger cannot disagree", () => {
    const spanishCh1 = generated.find((tex) => tex.includes("\\label{ch:first-words}"))!;
    expect(spanishCh1).toContain(
      "I can greet someone in Spanish, say how I am, and wish them a good day.",
    );
  });
});
