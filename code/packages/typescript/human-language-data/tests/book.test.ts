import { describe, expect, it } from "vitest";
import {
  bookBlockTitle,
  bookVoice,
  renderBookChapter,
  renderBookGlossary,
  renderInlineMarkdown,
  renderReferenceAppendix,
} from "../src/book.js";
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

[PAUSE 2s] Recall **${word}**.

## Guided Practice

[PAUSE 1s]
- [YOU SAY: **${word}**]
- [YOU SAY: it again]

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
    expect(generated.tex).toContain("\\item \\textbf{hello}");
  });

  it("prints no build-process blurb under the chapter title", () => {
    const generated = renderBookChapter(target, [parseLesson(source("A", 10, "hello"), "test")]);
    expect(generated.tex).not.toContain("canonical micro-lessons");
    expect(generated.tex).not.toContain("Language Ladder");
    expect(generated.tex).toContain("\\label{ch:first}\n\n\\section[hello]");
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

  it("renders a canonical Markdown reference as structured book back matter", () => {
    const generated = renderReferenceAppendix(
      {
        language: "test",
        title: "Pronunciation & Script Reference",
        source: "test/pronunciation-reference.md",
        output: "test/book/chapters/appendix-pronunciation.tex",
        unicodeScript: "Devanagari",
        scriptCommand: "dv",
      },
      `# Test — Pronunciation Reference

A [repository note](../data/script.json) and an [external source](https://example.test/ref).

## Read देवनागरी

1. Start with **दे**.
   Keep the continuation with the same step.
2. Then read **वन**.

### Sound ids

| id | anchor |
|---|---|
| \`first\` | **दे** |
`,
    );

    expect(generated).toContain("% GENERATED FILE. Edit test/pronunciation-reference.md");
    expect(generated).toContain("\\chapter*{Pronunciation \\& Script Reference}");
    expect(generated).toContain("A repository note and an \\href{https://example.test/ref}");
    expect(generated).toContain("\\section*{Read \\dv{देवनागरी}}");
    expect(generated).toContain("\\begin{enumerate}\n\\raggedright");
    expect(generated).toContain(
      "\\item Start with \\textbf{\\dv{दे}}. Keep the continuation with the same step.",
    );
    expect(generated).toContain("\\subsection*{Sound ids}");
    expect(generated).toContain(
      "\\item \\begin{minipage}[t]{\\linewidth}\n  \\textbf{id:} \\texttt{first}",
    );
    expect(generated).toContain("\\par \\textbf{anchor:} \\textbf{\\dv{दे}}");
    expect(generated).not.toContain("# Test");
  });

  it("renders a deduplicated, romanization-sorted glossary from content lessons", () => {
    const devanagari = (id: string, sequence: number, headword: string, romanization: string, gloss: string) =>
      parseLesson(
        source(id, sequence, headword).replace(
          `gloss: ${headword}`,
          `gloss: ${gloss}\nromanization: ${romanization}`,
        ),
        "test",
        "devanagari",
      );
    const two = devanagari("A", 10, "दो", "do", "two");
    const repeatedTwo = parseLesson(
      source("B", 20, "दो")
        .replace("chapter: 1", "chapter: 2")
        .replace("gloss: दो", "gloss: two\nromanization: do"),
      "test",
      "devanagari",
    );
    const ten = devanagari("C", 30, "दस", "das", "ten");
    const practice = parseLesson(
      source("D", 40, "drill").replace("type: word", "type: practice"),
      "test",
    );
    const generated = renderBookGlossary(
      {
        language: "test",
        output: "test/book/chapters/appendix-glossary.tex",
        unicodeScript: "Devanagari",
        scriptCommand: "dv",
      },
      [two, repeatedTwo, ten, practice],
    );

    expect(generated).toContain("% canonical-entries: 2");
    expect(generated).toContain("\\textbf{\\dv{दस}}\\enspace\\emph{das}");
    expect(generated).toContain("\\textbf{\\dv{दो}}\\enspace\\emph{do}");
    expect(generated.indexOf("\\dv{दस}")).toBeLessThan(generated.indexOf("\\dv{दो}"));
    expect(generated).toContain("Introduced in Chapters 1 and 2.");
    expect(generated).not.toContain("drill");
  });

  it("rejects glossary entries without a valid introduction chapter", () => {
    const lesson = parseLesson(
      source("A", 10, "hello").replace("chapter: 1\n", "chapter: 0\n"),
      "test",
    );
    expect(() =>
      renderBookGlossary(
        { language: "test", output: "test/book/chapters/appendix-glossary.tex" },
        [lesson],
      ),
    ).toThrow(/require a chapter/);
  });

  it("keeps indented Markdown quote continuations in one LaTeX quote", () => {
    const lesson = parseLesson(
      source("A", 10, "hello").replace(
        "- [YOU SAY: **hello**]\n- [YOU SAY: it again]",
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
      "Keep \\texttt{\"code\"}, " +
        "\\href{https://example.test/\\%22raw\\%22}" +
        "{\\textquotedblleft{}label\\textquotedblright{}}, and " +
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

  it("keeps external citations and unlinks repository-relative destinations", () => {
    expect(renderInlineMarkdown("[source](https://example.test/a_b?q=x&y=z#frag)")).toBe(
      "\\href{https://example.test/a\\_b?q=x\\&y=z\\#frag}{source}",
    );
    expect(renderInlineMarkdown("[draft](https://example.test/$value~draft)")).toBe(
      "\\href{https://example.test/\\%24value\\%7Edraft}{draft}",
    );
    // The label survives; the unreachable destination does not.
    expect(renderInlineMarkdown("Read [the next lesson](./TEST-C01-next.md).")).toBe(
      "Read the next lesson.",
    );
    expect(renderInlineMarkdown("→ [reference](../pronunciation-reference.md)")).toBe(
      "$\\to$ reference",
    );
    expect(renderInlineMarkdown("[**bien** / bueno](./ES-C01-bien.md) — the adjective")).toBe(
      "\\textbf{bien} / bueno — the adjective",
    );
  });

  it("fails closed for empty destinations and unsupported protocols", () => {
    expect(() => renderInlineMarkdown("[mail](mailto:hello@example.test)")).toThrow(
      /unsupported Markdown link protocol 'mailto:'/,
    );
    expect(() => renderInlineMarkdown("[empty]()")).toThrow(/must not be empty/);
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

describe("the book voice", () => {
  it("deletes pause cues, keeping whatever prose shared the line", () => {
    expect(bookVoice("[PAUSE 2s] You already know **por favor**.")).toBe(
      "You already know **por favor**.",
    );
    expect(bookVoice("[PAUSE 1s each] Two beats.")).toBe("Two beats.");
    // A line that was nothing but a cue leaves no trace at all.
    expect(bookVoice("[PAUSE 1s]\n- [YOU SAY: hola]")).toBe("- *Say it:* hola");
    expect(bookVoice("First.\n\n[PAUSE 3s]\n\nSecond.")).toBe("First.\n\n\nSecond.");
  });

  it("says repetition in prose instead of printing the bracket", () => {
    expect(bookVoice('[REPEAT x2] "Ça va ? — Ça va."')).toBe(
      '*Twice through:* "Ça va ? — Ça va."',
    );
    expect(bookVoice("[REPEAT x3] Run the exchange.")).toBe(
      "*3 times through:* Run the exchange.",
    );
    expect(bookVoice("[PAUSE 2s] [REPEAT x2] Both cues.")).toBe("*Twice through:* Both cues.");
  });

  it("gives a run of same-verb prompts one lead-in instead of three labels", () => {
    expect(
      bookVoice(
        '- [YOU SAY: "siento" — I feel]\n- [YOU SAY: the cousin word — "perdón"]\n' +
          "- [YOU SAY: both together]",
      ),
    ).toBe(
      'Say these aloud:\n\n- "siento" — I feel\n- the cousin word — "perdón"\n- both together',
    );
    expect(bookVoice("- [YOU WRITE: क]\n- [YOU WRITE: त]")).toBe(
      "Write these out:\n\n- क\n- त",
    );
  });

  it("labels each prompt when a list mixes cue kinds or stands alone", () => {
    expect(bookVoice("- [YOU SAY: नमस्ते]\n- [YOU WRITE: नमस्ते]")).toBe(
      "- *Say it:* नमस्ते\n- *Write it:* नमस्ते",
    );
    expect(bookVoice("- [YOU SAY: alone]")).toBe("- *Say it:* alone");
    expect(bookVoice("- plain bullet\n- [YOU SAY: a prompt]")).toBe(
      "- plain bullet\n- *Say it:* a prompt",
    );
    // A verb with no plural lead-in keeps the per-bullet label.
    expect(bookVoice("- [YOU POINT: the alif]\n- [YOU POINT: the bāʾ]")).toBe(
      "- *Point to:* the alif\n- *Point to:* the bāʾ",
    );
    // A verb nobody has voiced yet still prints as English, never as a bracket.
    expect(bookVoice("- [YOU WHISTLE: the tune]")).toBe("- *Whistle:* the tune");
    // Including the one lesson that writes a whole phrase where a verb goes.
    expect(bookVoice("- [YOU CHOOSE BY CONTEXT: clock → hour]")).toBe(
      "- *Choose by context:* clock → hour",
    );
  });

  it("keeps writing and tracing prompts as printed exercises", () => {
    // Writing is teaching content, not audio scaffolding: it must survive.
    expect(bookVoice("- [YOU TRACE: the shirorekhā first]\n- [YOU TRACE: then the body]")).toBe(
      "Trace these:\n\n- the shirorekhā first\n- then the body",
    );
    expect(bookVoice("- [YOU READ: क, त, न]\n- [YOU READ: back to front]")).toBe(
      "Read these aloud:\n\n- क, त, न\n- back to front",
    );
  });

  it("handles brackets inside a prompt and wrapped bullets", () => {
    expect(bookVoice('- [YOU SAY: "mā ismuka?" — "what [is] your name?"]')).toBe(
      '- *Say it:* "mā ismuka?" — "what [is] your name?"',
    );
    expect(bookVoice('- [YOU SAY: "al-" and three English words —\n  al-gebra, al-cohol]')).toBe(
      '- *Say it:* "al-" and three English words — al-gebra, al-cohol',
    );
  });

  it("leaves ordinary lists and prose untouched", () => {
    const plain = "- **lo** = it\n- **siento** = I feel\n\nSo it is literally *I feel it*.";
    expect(bookVoice(plain)).toBe(plain);
  });

  it("keeps a blank line so a lead-in never welds onto the paragraph above", () => {
    expect(bookVoice("Now the drill.\n- [YOU SAY: uno]\n- [YOU SAY: dos]")).toBe(
      "Now the drill.\n\nSay these aloud:\n\n- uno\n- dos",
    );
  });

  it("prints book headings for the internal block labels, and only those", () => {
    expect(bookBlockTitle("Guided Practice")).toBe("Your turn");
    expect(bookBlockTitle("You'll want to know first")).toBe("What to know first");
    expect(bookBlockTitle("You'll want to know")).toBe("What to know first");
    // Authored headings that already read like a book are left alone.
    expect(bookBlockTitle("You'll want to know — The famous mātrā")).toBe(
      "You'll want to know — The famous mātrā",
    );
    expect(bookBlockTitle("Grammar Lens: the verb goes last")).toBe(
      "Grammar Lens: the verb goes last",
    );
    // A qualified label loses the label and keeps the qualifier.
    expect(bookBlockTitle("Guided Practice: conjugate on command")).toBe(
      "Your turn: conjugate on command",
    );
  });

  it("titles the warm-up and recall blocks as a book would", () => {
    const generated = renderBookChapter(target, [parseLesson(source("A", 10, "hello"), "test")]);
    expect(generated.tex).not.toContain("Warm-up");
    expect(generated.tex).not.toContain("Wrap-up");
    expect(generated.tex).not.toContain("Guided Practice");
    expect(generated.tex).toContain("\\begin{quote}\nRecall \\textbf{hello}.\n\\end{quote}");
    expect(generated.tex).toContain("title={Before you move on}]");
    expect(generated.tex).toContain("\\subsection*{Your turn}");
  });

  it("prints no delivery cue anywhere in a generated chapter", () => {
    const generated = renderBookChapter(target, [parseLesson(source("A", 10, "hello"), "test")]);
    expect(generated.tex).not.toMatch(/PAUSE/);
    expect(generated.tex).not.toMatch(/YOU SAY/);
    expect(generated.tex).not.toMatch(/REPEAT x/);
    expect(generated.tex).toContain("Say these aloud:");
  });
});
