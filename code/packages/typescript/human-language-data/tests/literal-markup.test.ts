import { describe, expect, it } from "vitest";
import { measureLiteralMarkup, renderLiteralMarkup } from "../src/literal-markup.js";
import { parseLesson } from "../src/parse.js";
import { loadEverything } from "../src/loader.js";
import { generatedBookOutputs } from "../src/book-cli.js";

const lesson = (body: string) =>
  parseLesson(
    [
      "---",
      "schema_version: 2",
      "id: ES-C01-hola",
      "chapter: 1",
      "type: word",
      "headword: hola",
      "gloss: hello",
      "concept_tag: GREETING-HELLO",
      "---",
      "",
      body,
    ].join("\n"),
    "spanish",
  );

describe("literal markup", () => {
  it("catches the exact defect that shipped to three books", () => {
    const report = measureLiteralMarkup([lesson("> **હા** &nbsp;&nbsp; **ના**")]);
    expect(report.summary.sourceFindings).toBe(2);
    expect(report.findings[0]!.markup).toBe("&nbsp;");
    expect(report.findings[0]!.where).toBe("ES-C01-hola");
  });

  it("catches the same mistake after the LaTeX escaper has been over it", () => {
    // By the time it reaches the book it no longer looks like HTML, which is
    // exactly how it stayed invisible: grepping the .tex for `&nbsp;` finds
    // nothing, because the escaper turned the ampersand into `\&`.
    const report = measureLiteralMarkup(
      [],
      [{ path: "gujarati/book/chapters/ch13.tex", language: "gujarati", text: "\\gu{હા} \\&nbsp; \\gu{ના}" }],
    );
    expect(report.summary.renderedFindings).toBe(1);
    expect(report.findings[0]!.layer).toBe("rendered");
  });

  it("catches numeric entities, which are the same mistake in a different hat", () => {
    const report = measureLiteralMarkup([lesson("a &#160; b &#xA0; c")]);
    expect(report.summary.sourceFindings).toBe(2);
  });

  it("catches bare HTML tags", () => {
    const report = measureLiteralMarkup([lesson("one<br>two<br />three")]);
    expect(report.summary.sourceFindings).toBe(2);
  });

  it("does NOT flag the directive comments, which are the corpus's own syntax", () => {
    // Flagging these would flag every lesson in the corpus, and the gate would be
    // switched off within a day.
    const report = measureLiteralMarkup([
      lesson("<!-- hl-knowledge: introduces=[]; assesses=[] -->\n\ntext"),
    ]);
    expect(report.summary.sourceFindings).toBe(0);
  });

  it("does NOT flag markup that is being quoted rather than emitted", () => {
    // A lesson explaining an entity, or a backlog entry describing this very
    // defect, is quoting. Both code fences and inline spans are exempt.
    const report = measureLiteralMarkup([
      lesson("Write `&nbsp;` and you get literal text.\n\n```\n&amp; <br>\n```\n"),
    ]);
    expect(report.summary.sourceFindings).toBe(0);
  });

  it("reports a line number that survives the exemption blanking", () => {
    // Exempt spans are blanked rather than deleted, so offsets and newlines are
    // preserved and a reported line still points at the right line.
    // Body lines: 1 blank, 2 the comment, 3 and 4 blank, 5 the offender.
    const report = measureLiteralMarkup([lesson("<!-- hl-knowledge: a -->\n\n\nline four &nbsp;")]);
    expect(report.findings).toHaveLength(1);
    expect(report.findings[0]!.line).toBe(5);
  });

  it("finds both matches when one line holds two", () => {
    // A shared /g regex across lines would skip every other match.
    const report = measureLiteralMarkup([lesson("&nbsp;&amp;")]);
    expect(report.findings.map((f) => f.markup)).toEqual(["&nbsp;", "&amp;"]);
  });

  it("renders a clean run as a positive statement rather than silence", () => {
    const text = renderLiteralMarkup(measureLiteralMarkup([lesson("clean")])).join("\n");
    expect(text).toContain("none in 1 lessons");
  });

  it("catches an authoring comment that reached the generated book", () => {
    // Found live: one `<!-- ... -->` from a lesson was typesetting into the
    // shipped Spanish PDF, inside a coloured culture box. The gate was blind to
    // it because it applied MARKDOWN comment semantics to LaTeX output, where
    // `<!--` is not a comment at all.
    const report = measureLiteralMarkup(
      [],
      [{ path: "spanish/book/chapters/ch07.tex", language: "spanish", text: "text\n<!-- a note to authors -->\n" }],
    );
    expect(report.summary.renderedFindings).toBe(2);
  });

  it("cannot be smuggled past on the rendered layer", () => {
    // Markdown exemptions must not apply to LaTeX: in a .tex a backtick is an
    // open quote and `<!-- -->` is ordinary text, so neither may blank a span.
    for (const text of ["<!-- \\&nbsp; -->", "He said `x \\&nbsp; y`", "```\n\\&nbsp;\n```"]) {
      const report = measureLiteralMarkup([], [{ path: "x.tex", language: "x", text }]);
      expect(report.summary.renderedFindings).toBeGreaterThan(0);
    }
  });

  it("does not backtrack catastrophically on a long whitespace run after '<'", () => {
    // `<\s*\/?\s*` split N spaces between two `\s*` in O(N^2) ways: 2,634ms at
    // N=64,000, and reachable from ordinary Markdown because a long inline code
    // span is blanked INTO spaces. Regrouping the slash removes the ambiguity.
    const started = Date.now();
    measureLiteralMarkup([lesson("<" + " ".repeat(64_000) + "end")]);
    expect(Date.now() - started).toBeLessThan(1_000);
  });

  it("strips control characters out of rendered findings", () => {
    // `[^>]*` admits ESC and CR, and the finding is written to a terminal. A
    // finding must not be able to repaint the report that reports it.
    const text = renderLiteralMarkup(
      measureLiteralMarkup([lesson("<br\u001b[2J\u0007>")]),
    ).join("\n");
    expect(text).not.toContain("\u001b");
    expect(text).toContain("?");
  });

  it("THE GATE: the committed corpus carries no literal markup at either layer", () => {
    const { lessons } = loadEverything();
    // The generator's own output, not the committed files: this asks whether the
    // books a reader would get TODAY are clean, which is one step ahead of
    // whatever happens to be checked in.
    const rendered = [...generatedBookOutputs()].map(([path, text]) => ({
      path,
      language: path.split("/")[0] ?? "",
      text,
    }));
    const report = measureLiteralMarkup(lessons, rendered);
    // Named rather than counted: a bare number tells whoever breaks this nothing
    // about which file to open.
    expect(report.findings.map((f) => `${f.where}:${f.line} ${f.markup}`)).toEqual([]);
    expect(rendered.length).toBeGreaterThan(0);
    expect(lessons.length).toBeGreaterThan(0);
  }, 60_000);

  it("proves the corpus gate is not vacuous", () => {
    // The check above passes on an empty input too. This pins that the same
    // measurement, over the same corpus plus one planted line, actually fires.
    const { lessons } = loadEverything();
    const planted = [...lessons, lesson("planted &nbsp; control")];
    expect(measureLiteralMarkup(planted).summary.sourceFindings).toBe(1);
  }, 60_000);
});
