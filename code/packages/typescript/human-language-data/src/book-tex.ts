// book-tex — assemble `<track>/book/book.tex` instead of maintaining it by hand.
//
// ---------------------------------------------------------------------------
// Why this exists
// ---------------------------------------------------------------------------
//
// `book.tex` was the last hand-maintained link in the lesson -> chapter -> book
// chain. Every other artefact below it is already generated: the chapter `.tex`
// files, the modality definitions, the glossary, the answer key, the index, the
// narration, the hashes. Only the `\input{}` list that stitches them together
// was typed by a person, once per chapter, forever.
//
// That cost two things.
//
// FIRST, it conflicted. Every content tranche appends one `\input` line to the
// end of the same list, so every pair of tranches collides on the same few
// lines — the HL21 problem exactly, in a file that is not even JSON.
//
// SECOND, and worse, it was forgettable. A chapter whose `\input` line is
// missing simply does not appear in the book, and nothing fails: the chapter
// `.tex` is generated, committed, hash-checked and never read. That has already
// happened once.
//
// ---------------------------------------------------------------------------
// Split by ORIGIN, not by size
// ---------------------------------------------------------------------------
//
// `book.tex` is two different documents glued together, and the split that
// matters is not "top half / bottom half" but "who writes it":
//
//   <track>/book/frontmatter.tex   AUTHORED. \documentclass, \input{preamble},
//                                  the titlepage, the CC BY-SA notice, the
//                                  preface, \tableofcontents, \mainmatter, and
//                                  \input{chapter-modalities}. Per-track,
//                                  genuinely written, and no tranche touches it.
//
//   ...the chapter \input list...  DERIVED. One line per chapter, reconstructible
//                                  from `core/book-generation.json` alone.
//
//   <track>/book/backmatter.tex    AUTHORED. \backmatter, the appendix inputs in
//                                  their authored order, \end{document}.
//
//   <track>/book/book.tex          GENERATED = the three concatenated.
//
// Sharding the `\input` list would have been the wrong tool. It is not content
// anybody should be editing; it is a projection of a ledger that already exists.
// Generating it removes the conflict AND the forgettable step, where sharding
// would only have removed the conflict.
//
// The concatenation is literal rather than `\input{frontmatter}`, because the
// front matter contains `\documentclass` and `\begin{document}`, neither of
// which can appear inside an `\input`. So `frontmatter.tex` and
// `backmatter.tex` are FRAGMENTS, not standalone documents — they will not
// compile alone, and are not meant to.
//
// ---------------------------------------------------------------------------
// Where the chapter list comes from
// ---------------------------------------------------------------------------
//
// `book-generation.json` has two arrays of chapters, and BOTH are needed:
//
//   targets[]      the generated chapters
//   handwritten[]  chapters whose .tex is authored rather than rendered
//
// They interleave by chapter number — French's handwritten chapters are 1..16
// and its generated ones 17..33, but Kannada's alternate — so the list is the
// union, ordered by `chapter`. Using `targets` alone silently drops every
// handwritten chapter from the book, which is the same invisible failure this
// module exists to prevent.
//
// This was checked against all 23 tracks before being relied on: the union,
// ordered by chapter number, reproduces the authored `\input` list exactly —
// same set, same order, no interleaved directives, no duplicates, for every
// track. See HL21 section 6.

/**
 * The three fields of a chapter entry this module needs.
 *
 * Structural rather than importing `BookGenerationConfig`, which is private to
 * `book-cli.ts`. Both `targets[]` and `handwritten[]` satisfy it, which is the
 * point: they are different kinds of chapter and identical as far as the
 * `\input` list is concerned.
 */
export interface BookChapterEntry {
  readonly language: string;
  readonly chapter: number;
  readonly output: string;
}

/** Just the parts of `book-generation.json` that decide the chapter list. */
export interface BookChapterLedger {
  readonly targets: readonly BookChapterEntry[];
  readonly handwritten?: readonly BookChapterEntry[];
}

/** The authored fragment before the chapter list. */
export const FRONTMATTER_TEX = "frontmatter.tex";
/** The authored fragment after it. */
export const BACKMATTER_TEX = "backmatter.tex";

/**
 * What a chapter's `\input{}` argument is allowed to look like.
 *
 * An ALLOWLIST, and it has to be, because there is no reliable way to escape a
 * filename inside a TeX `\input`. Every legitimate value across all 23 tracks
 * already matches this shape, so the list costs nothing and closes the hole.
 */
const SAFE_INPUT_ARGUMENT = /^chapters\/[A-Za-z0-9][A-Za-z0-9._-]*$/;

/**
 * `spanish/book/chapters/ch01-first-words.tex` -> `chapters/ch01-first-words`.
 *
 * VALIDATED, not merely transformed, and this is a real hole rather than a
 * theoretical one. `output` comes from authored JSON, and the only guard it
 * previously passed was `safeOutput`, which checks PATH CONTAINMENT: relative,
 * no `..` escape, ends in `.tex`. Path containment says nothing about TeX
 * metacharacters, and `}`, `{`, `\` and space are all legal in a filename, so
 * they survive `resolve()` untouched. An `output` of
 *
 *     spanish/book/chapters/ch01-a} \input{/etc/passwd} \iffalse{.tex
 *
 * passes containment and renders as
 *
 *     \input{chapters/ch01-a} \input{/etc/passwd} \iffalse{}
 *
 * — a working chapter followed by an arbitrary file read, which does not even
 * look broken. `\immediate\write18{...}` is available the same way. That would
 * be latent in a committed file, and this same change adds a compile gate that
 * feeds it to `latexmk -xelatex`.
 *
 * The repo already treats `book-generation.json` paths as untrusted; that is the
 * whole reason `safeOutput` and `manifest-path.ts` exist. This is simply a
 * second sink for the same input, needing a different kind of check.
 */
export function inputArgument(output: string): string {
  const argument = output.replace(/^[^/]+\/book\//, "").replace(/\.tex$/, "");
  if (!SAFE_INPUT_ARGUMENT.test(argument)) {
    throw new Error(
      `unsafe chapter input argument '${argument}' derived from output '${output}'`,
    );
  }
  return argument;
}

/**
 * Every chapter of one track, ordered as the book prints them.
 *
 * Generated and handwritten chapters merged and sorted by chapter number. The
 * sort is on the NUMBER, not the output path: `ch10-...` sorts before `ch2-...`
 * as a string, and while today's paths happen to be zero-padded, relying on
 * that would make the ordering depend on a naming convention nobody has
 * promised to keep.
 */
export function chapterInputsFor(config: BookChapterLedger, language: string): string[] {
  const chapters = [
    ...config.targets.filter((entry) => entry.language === language),
    ...(config.handwritten ?? []).filter((entry) => entry.language === language),
  ].sort((left, right) => left.chapter - right.chapter);

  const seen = new Set<number>();
  for (const entry of chapters) {
    if (seen.has(entry.chapter)) {
      // Two entries for one chapter number means one `.tex` overwrites the
      // other and the book prints whichever won. Refuse rather than pick.
      throw new Error(
        `${language}: chapter ${entry.chapter} is declared twice in book-generation.json`,
      );
    }
    seen.add(entry.chapter);
  }
  return chapters.map((entry) => `\\input{${inputArgument(entry.output)}}`);
}

/**
 * The whole file: authored front, derived middle, authored back.
 *
 * The fragments are used verbatim, including their trailing newlines. That is
 * what makes the split reversible — `frontmatter + chapters + backmatter` is
 * exactly the original file when the original had no blank lines inside its
 * chapter block, which is how the fragments are cut in the first place.
 */
export function renderBookTex(
  frontmatter: string,
  chapterInputs: string[],
  backmatter: string,
): string {
  return `${frontmatter}${chapterInputs.map((line) => `${line}\n`).join("")}${backmatter}`;
}

/** Where a track's chapter `\input` block starts and ends. */
export interface BookTexSplit {
  frontmatter: string;
  chapterInputs: string[];
  backmatter: string;
}

/**
 * Cut an existing hand-maintained `book.tex` into its three parts.
 *
 * Used once per track by the migration, and by the tests that prove the split
 * is faithful. The cut points are the first and last `\input{chapters/ch...}`
 * lines: everything before the first is front matter, everything after the last
 * is back matter.
 *
 * Blank lines INSIDE the chapter block are dropped, and that is the one place
 * this is not byte-preserving. See HL21 section 6 — 15 of 23 tracks carry such
 * blanks, in at least four mutually inconsistent patterns, and they are
 * semantically inert in LaTeX.
 */
export function splitBookTex(tex: string): BookTexSplit {
  const lines = tex.split("\n");
  const isChapterInput = (line: string) => /^\\input\{chapters\/ch[^}]*\}\s*$/.test(line);
  const first = lines.findIndex(isChapterInput);
  if (first < 0) throw new Error("book.tex has no \\input{chapters/ch...} lines to split on");
  let last = first;
  for (let i = lines.length - 1; i >= first; i -= 1) {
    if (isChapterInput(lines[i]!)) {
      last = i;
      break;
    }
  }

  const chapterInputs: string[] = [];
  for (let i = first; i <= last; i += 1) {
    const line = lines[i]!;
    if (line.trim() === "") continue;
    if (!isChapterInput(line)) {
      // A directive interleaved among the chapters is authored ordering the
      // ledgers cannot express, and silently dropping it would remove content
      // from the book. Refuse; a human decides where it belongs.
      throw new Error(
        `book.tex line ${i + 1} sits inside the chapter block but is not a chapter input: ${line}`,
      );
    }
    chapterInputs.push(line.trimEnd());
  }

  return {
    frontmatter: lines.slice(0, first).map((line) => `${line}\n`).join(""),
    chapterInputs,
    backmatter: lines.slice(last + 1).join("\n"),
  };
}
