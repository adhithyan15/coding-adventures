import { describe, expect, it } from "vitest";
import { renderBookChapter, renderInlineMarkdown } from "../src/book.js";
import { parseLesson } from "../src/parse.js";

function source(id: string, sequence: number, word: string): string {
  return `---
schema_version: 2
id: ${id}
spine_node: HELLO
sequence: ${sequence}
chapter: 1
type: word
headword: ${word}
gloss: ${word}
concept_tag: GREETING-HELLO
prerequisites: []
duration:
  max_seconds: 120
requires:
  knowledge: []
introduces:
  knowledge: []
practises:
  knowledge: []
skills: [reading]
modes: [interpretive]
strands: [meaning-input]
register: neutral
variety: general
---

# *${word}* — lesson

## Warm-up

Recall **${word}**.

## Guided Practice

- [YOU SAY: **${word}**]

## Wrap-up Recall

Say *${word}*.
`;
}

const target = {
  language: "test",
  chapter: 1,
  title: "First & safest",
  label: "ch:first",
  output: "test/book/chapters/ch01-first.tex",
};

describe("canonical LaTeX chapter rendering", () => {
  it("renders typed blocks in sequence and embeds the combined source hash", () => {
    const later = parseLesson(source("B", 20, "bye"), "test");
    const earlier = parseLesson(source("A", 10, "hello"), "test");
    const generated = renderBookChapter(target, [later, earlier]);
    expect(generated.lessonIds).toEqual(["A", "B"]);
    expect(generated.tex).toContain(`% canonical-source-hash: ${generated.sourceHash}`);
    expect(generated.tex.indexOf("lesson:A")).toBeLessThan(generated.tex.indexOf("lesson:B"));
    expect(generated.tex).toContain("\\begin{tcolorbox}");
    expect(generated.tex).toContain("\\begin{itemize}\n\\raggedright");
    expect(generated.tex).toContain("\\item {[}YOU SAY: \\textbf{hello}{]}");
  });

  it("keeps arrows out of the PDF bookmark and running-header title", () => {
    const lesson = parseLesson(source("A", 10, "vuestra merced → usted"), "test");
    const generated = renderBookChapter(target, [lesson]);
    expect(generated.tex).toContain("\\section[vuestra merced to usted]");
    expect(generated.tex).toContain("$\\to$");
  });

  it("escapes LaTeX control characters while preserving authored emphasis", () => {
    expect(renderInlineMarkdown("**A&B** costs $5 and uses `x_y`")).toBe(
      "\\textbf{A\\&B} costs \\$5 and uses \\texttt{x\\_y}",
    );
    expect(renderInlineMarkdown("*buen**os*** and ***Como** tú.*")).toBe(
      "\\emph{buen\\textbf{os}} and \\emph{\\textbf{Como} tú.}",
    );
  });

  it("fails closed when a target includes a legacy lesson", () => {
    const legacy = parseLesson(source("A", 10, "hello").replace("schema_version: 2\n", ""), "test");
    expect(() => renderBookChapter(target, [legacy])).toThrow(/schema version 2/);
  });
});
