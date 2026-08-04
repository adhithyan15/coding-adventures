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

  it("renders Markdown tables as width-aware LaTeX tables", () => {
    const lesson = parseLesson(
      source("A", 10, "hablar").replace(
        "## Guided Practice",
        "## Grammar Lens: singular forms\n\n| person | form |\n|---|---|\n| I | **hablo** |\n\n## Guided Practice",
      ),
      "test",
    );
    const generated = renderBookChapter(target, [lesson]);
    expect(generated.tex).toContain("\\noindent\n\\begin{tabularx}{\\linewidth}");
    expect(generated.tex).toContain("\\begin{tabularx}{\\linewidth}");
    expect(generated.tex).toContain("\\textbf{person} & \\textbf{form} \\\\");
    expect(generated.tex).toContain("I & \\textbf{hablo} \\\\");
  });

  it("keeps indented Markdown quote continuations in one LaTeX quote", () => {
    const lesson = parseLesson(
      source("A", 10, "hello").replace(
        "- [YOU SAY: **hello**]",
        '> **hello** — "a greeting\n  that continues."',
      ),
      "test",
    );
    const generated = renderBookChapter(target, [lesson]);
    expect(generated.tex).toContain(
      "\\begin{quote}\n\\textbf{hello} — \\textquotedblleft{}a greeting that " +
        "continues.\\textquotedblright{}\n\\end{quote}",
    );
  });

  it("escapes LaTeX control characters while preserving authored emphasis", () => {
    expect(renderInlineMarkdown("**A&B** costs $5 and uses `x_y`")).toBe(
      "\\textbf{A\\&B} costs \\$5 and uses \\texttt{x\\_y}",
    );
    expect(renderInlineMarkdown("favor ≈ fah-BOR")).toBe("favor $\\approx$ fah-BOR");
    expect(renderInlineMarkdown("hṓrā ↔ h₁rewdʰ")).toBe(
      "h\\'{\\={o}}rā $\\leftrightarrow$ h\\textsubscript{1}rewd\\textsuperscript{h}",
    );
    expect(renderInlineMarkdown("trī́ṇi · catvā́ri · ph₂tḗr · nókʷts ≠ ḱwon- · hããⁿ")).toBe(
      "tr\\'{\\={\\i}}ṇi · catv\\'{\\={a}}ri · ph\\textsubscript{2}t\\'{\\={e}}r · nók\\textsuperscript{w}ts $\\neq$ \\'{k}won- · hãã\\textsuperscript{n}",
    );
    expect(renderInlineMarkdown("*buen**os*** and ***Como** tú.*")).toBe(
      "\\emph{buen\\textbf{os}} and \\emph{\\textbf{Como} tú.}",
    );
    expect(renderInlineMarkdown("**\\*parabolāvit**")).toBe(
      "\\textbf{*parabolāvit}",
    );
  });

  it("typesets paired straight prose quotes without changing literal text", () => {
    expect(renderInlineMarkdown('Say "hello" and "**goodbye**".')).toBe(
      "Say \\textquotedblleft{}hello\\textquotedblright{} and " +
        "\\textquotedblleft{}\\textbf{goodbye}\\textquotedblright{}.",
    );
    expect(
      renderInlineMarkdown(
        'Keep `"code"`, ["label"](https://example.test/"raw"), and \\"literal\\".',
      ),
    ).toBe(
      "Keep \\texttt{\"code\"}, \\textquotedblleft{}label\\textquotedblright{}, and " +
        '\"literal\".',
    );
    expect(renderInlineMarkdown('Keep a 5" mark before "paired" prose.')).toBe(
      'Keep a 5" mark before \\textquotedblleft{}paired\\textquotedblright{} prose.',
    );
    expect(renderInlineMarkdown('Read "a saying of "you are worthy.""')).toBe(
      "Read \\textquotedblleft{}a saying of \\textquotedblleft{}you are worthy." +
        "\\textquotedblright{}\\textquotedblright{}",
    );
    expect(renderInlineMarkdown('Say *"...and again."*')).toBe(
      "Say \\emph{\\textquotedblleft{}...and again.\\textquotedblright{}}",
    );
    expect(renderInlineMarkdown('Use "**mother of ___**".')).toBe(
      "Use \\textquotedblleft{}\\textbf{mother of \\_\\_\\_}\\textquotedblright{}.",
    );
    expect(renderInlineMarkdown('An unmatched " mark stays literal.')).toBe(
      'An unmatched " mark stays literal.',
    );
    expect(renderInlineMarkdown('Existing “curly quotes” stay unchanged.')).toBe(
      'Existing “curly quotes” stay unchanged.',
    );
  });

  it("wraps configured Unicode-script runs in the book's dedicated font command", () => {
    const options = { unicodeScript: "Devanagari", scriptCommand: "mr" };
    expect(renderInlineMarkdown("**दोन** and पाच.", options)).toBe(
      "\\textbf{\\mr{दोन}} and \\mr{पाच}.",
    );
  });

  it("wraps comparisons across multiple configured writing systems", () => {
    const options = [
      { unicodeScript: "Telugu", scriptCommand: "te" },
      { unicodeScript: "Tamil", scriptCommand: "ta" },
      { unicodeScript: "Kannada", scriptCommand: "ka" },
    ];
    expect(renderInlineMarkdown("తెలుగు / தமிழ் / ಕನ್ನಡ", options)).toBe(
      "\\te{తెలుగు} / \\ta{தமிழ்} / \\ka{ಕನ್ನಡ}",
    );
  });

  it("uses authored romanization for a non-Latin section bookmark", () => {
    const lesson = parseLesson(
      source("A", 10, "दोन").replace("gloss: दोन", "gloss: two\nromanization: don"),
      "test",
    );
    const generated = renderBookChapter(
      { ...target, unicodeScript: "Devanagari", scriptCommand: "mr" },
      [lesson],
    );
    expect(generated.tex).toContain("\\section[don]{\\emph{\\mr{दोन}} — lesson}");
  });

  it("requires both script-rendering options when either is configured", () => {
    const lesson = parseLesson(source("A", 10, "hello"), "test");
    expect(() => renderBookChapter({ ...target, unicodeScript: "Devanagari" }, [lesson])).toThrow(
      /unicodeScript and scriptCommand must be declared together/,
    );
    expect(() => renderBookChapter({ ...target, inlineScripts: [] }, [lesson])).toThrow(
      /inlineScripts must not be empty/,
    );
    expect(() =>
      renderBookChapter(
        {
          ...target,
          unicodeScript: "Devanagari",
          scriptCommand: "mr",
          inlineScripts: [{ unicodeScript: "Tamil", scriptCommand: "ta" }],
        },
        [lesson],
      ),
    ).toThrow(/inlineScripts cannot be combined/);
  });

  it("fails closed when a target includes a legacy lesson", () => {
    const legacy = parseLesson(source("A", 10, "hello").replace("schema_version: 2\n", ""), "test");
    expect(() => renderBookChapter(target, [legacy])).toThrow(/schema version 2/);
  });
});
