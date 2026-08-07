// HL-C50 — the book is a standalone artifact. See HL09 §8.

import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { defaultCurriculumRoot, loadBookCorpus } from "../src/loader.js";

const BOOKS = loadBookCorpus().books;

// Chapters AND the book shell. `loadBookCorpus` records `entrypoint` as a path and
// never reads it, so a test over `book.chapters` alone passes green while the TITLE
// PAGE prints a repo path — which is exactly what Japanese and Chinese were doing,
// more prominently than any chapter clause.
const CHAPTERS = [
  ...BOOKS.flatMap((book) =>
    book.chapters.map((chapter) => ({ source: chapter.source, tex: chapter.tex })),
  ),
  ...BOOKS.map((book) => ({
    source: book.entrypoint,
    tex: readFileSync(join(defaultCurriculumRoot(), book.entrypoint), "utf8"),
  })),
];

describe("a reader holding the PDF", () => {
  it("is never sent to a file in the git repository", () => {
    // 99 chapters ended a paragraph with "Practice lessons: lessons/AR-C01-*.md".
    // That sentence is addressed to somebody with a checkout. `book-cli.ts` already
    // states the principle — "a reader holding the PDF cannot follow a link into a
    // Git repository" — and drops relative links for exactly this reason; the
    // handwritten chapters had never been held to it.
    const offenders = CHAPTERS.filter(({ tex }) =>
      /lessons\/[A-Z]{2}-C\d|(?:Practice|Companion)\s+(?:practice\s+)?lessons?:?/.test(tex),
    ).map(({ source }) => source);
    expect(offenders).toEqual([]);
  });

  it("is never pointed at a path in the curriculum tree at all", () => {
    // Broader than the line that prompted this: any `lessons/`, `curriculum.json`,
    // `chapters.json` or `.md` path printed to a reader is the same mistake.
    const offenders = CHAPTERS.filter(({ tex }) =>
      /\\(?:texttt|nolinkurl|path)\{[^}]*(?:lessons\/|\.md|chapters\.json|curriculum\.json)[^}]*\}/.test(
        tex,
      ),
    ).map(({ source }) => source);
    expect(offenders).toEqual([]);
  });
});
