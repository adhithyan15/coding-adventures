import { existsSync, readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import {
  BACKMATTER_TEX,
  FRONTMATTER_TEX,
  chapterInputsFor,
  inputArgument,
  renderBookTex,
  splitBookTex,
} from "../src/book-tex.js";
import { generatedBookOutputs } from "../src/book-cli.js";
import { defaultCurriculumRoot } from "../src/loader.js";

const ROOT = defaultCurriculumRoot();
const config = JSON.parse(
  readFileSync(join(ROOT, "core", "book-generation.json"), "utf8"),
) as Parameters<typeof chapterInputsFor>[0];

/** Every track that has a book. */
const tracks = readdirSync(ROOT, { withFileTypes: true })
  .filter((entry) => entry.isDirectory() && existsSync(join(ROOT, entry.name, "book", "book.tex")))
  .map((entry) => entry.name)
  .sort();

describe("the generated book.tex", () => {
  it("reproduces every track's committed book.tex byte for byte", () => {
    // The fidelity requirement, and the whole proof of this change. If the
    // generator cannot reproduce what is on disk, it is not a generator for
    // this book — it is a proposal to rewrite 23 of them.
    const outputs = generatedBookOutputs(ROOT);
    let checked = 0;
    for (const track of tracks) {
      const relative = `${track}/book/book.tex`;
      const generated = outputs.get(relative);
      expect(generated, `${relative} is not generated`).toBeDefined();
      expect(generated, relative).toBe(readFileSync(join(ROOT, track, "book", "book.tex"), "utf8"));
      checked += 1;
    }
    expect(checked).toBe(tracks.length);
  });

  it("gives every track both authored halves", () => {
    for (const track of tracks) {
      expect(existsSync(join(ROOT, track, "book", FRONTMATTER_TEX)), track).toBe(true);
      expect(existsSync(join(ROOT, track, "book", BACKMATTER_TEX)), track).toBe(true);
    }
  });

  it("keeps the authored halves out of the generated set", () => {
    // They are edited by hand. A generator that also emitted them would
    // overwrite the edit on the next `--write`.
    const outputs = generatedBookOutputs(ROOT);
    for (const track of tracks) {
      expect(outputs.has(`${track}/book/${FRONTMATTER_TEX}`)).toBe(false);
      expect(outputs.has(`${track}/book/${BACKMATTER_TEX}`)).toBe(false);
    }
  });

  it("includes handwritten chapters, not only generated ones", () => {
    // `targets` alone silently drops every handwritten chapter from the book —
    // the same invisible failure this generator exists to prevent. French has
    // 16 handwritten chapters and would lose all of them.
    const handwritten = (config.handwritten ?? []).filter((entry) => entry.language === "french");
    expect(handwritten.length).toBeGreaterThan(0);
    const inputs = chapterInputsFor(config, "french");
    for (const entry of handwritten) {
      expect(inputs).toContain(`\\input{${inputArgument(entry.output)}}`);
    }
  });

  it("orders chapters by number, not by output path", () => {
    for (const track of tracks) {
      const numbers = [
        ...config.targets.filter((e) => e.language === track),
        ...(config.handwritten ?? []).filter((e) => e.language === track),
      ]
        .sort((a, b) => a.chapter - b.chapter)
        .map((e) => e.chapter);
      expect(numbers, track).toEqual([...numbers].sort((a, b) => a - b));
      expect(new Set(numbers).size, `${track} has a duplicate chapter number`).toBe(numbers.length);
    }
  });
});

describe("splitBookTex", () => {
  it("round-trips a file whose chapter block has no blank lines", () => {
    const tex = [
      "\\documentclass{book}",
      "\\begin{document}",
      "\\mainmatter",
      "",
      "\\input{chapters/ch01-a}",
      "\\input{chapters/ch02-b}",
      "",
      "\\backmatter",
      "\\end{document}",
      "",
    ].join("\n");
    const split = splitBookTex(tex);
    expect(split.chapterInputs).toEqual(["\\input{chapters/ch01-a}", "\\input{chapters/ch02-b}"]);
    expect(renderBookTex(split.frontmatter, split.chapterInputs, split.backmatter)).toBe(tex);
  });

  it("refuses a directive interleaved among the chapters", () => {
    // That is authored ordering the ledgers cannot express, and dropping it
    // would silently remove content from the book.
    const tex = [
      "\\mainmatter",
      "\\input{chapters/ch01-a}",
      "\\clearpage",
      "\\input{chapters/ch02-b}",
      "\\backmatter",
      "",
    ].join("\n");
    expect(() => splitBookTex(tex)).toThrow(/sits inside the chapter block/);
  });

  it("refuses a file with no chapter inputs at all", () => {
    expect(() => splitBookTex("\\documentclass{book}\n")).toThrow(/no \\input\{chapters/);
  });
});

describe("chapterInputsFor", () => {
  it("refuses a chapter number declared twice", () => {
    // Two entries for one chapter means one .tex overwrites the other and the
    // book prints whichever won.
    const clash = {
      targets: [
        { language: "toy", chapter: 1, output: "toy/book/chapters/ch01-a.tex" },
        { language: "toy", chapter: 1, output: "toy/book/chapters/ch01-b.tex" },
      ],
    };
    expect(() => chapterInputsFor(clash, "toy")).toThrow(/declared twice/);
  });

  it("strips the track and book prefix from the input argument", () => {
    expect(inputArgument("spanish/book/chapters/ch01-first-words.tex")).toBe(
      "chapters/ch01-first-words",
    );
  });

  it("refuses an output that would inject LaTeX", () => {
    // `safeOutput` checks PATH CONTAINMENT — relative, no `..`, ends in `.tex`.
    // It says nothing about TeX metacharacters, and `}`, `{`, `\` and space are
    // all legal in a filename, so they survive `resolve()` untouched. Each of
    // these passes containment and would render a working chapter followed by an
    // arbitrary file read or a shell escape — and would not look broken.
    for (const hostile of [
      "spanish/book/chapters/ch01-a} \\input{/etc/passwd} \\iffalse{.tex",
      "spanish/book/chapters/ch01-a}\\immediate\\write18{id}\\relax\\iffalse{.tex",
      "spanish/book/chapters/../../../etc/passwd.tex",
      "spanish/book/notchapters/ch01-a.tex",
    ]) {
      expect(() => inputArgument(hostile), hostile).toThrow(/unsafe chapter input argument/);
    }
  });

  it("accepts every output the real corpus actually uses", () => {
    // The allowlist is only worth having if it does not also reject the corpus.
    for (const entry of [...config.targets, ...(config.handwritten ?? [])]) {
      expect(() => inputArgument(entry.output), entry.output).not.toThrow();
    }
  });
});
