import { describe, expect, it } from "vitest";
import {
  bookBlockTitle,
  bookVoice,
  renderBookAnswerKey,
  renderBookChapter,
  renderBookGlossary,
  renderBookIndex,
  renderInlineMarkdown,
  renderReferenceAppendix,
} from "../src/book.js";
import { parseLesson } from "../src/parse.js";
import type { ChapterCapability } from "../src/types.js";

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

function capability(chapter: number, title: string, label: string): ChapterCapability {
  return {
    chapter,
    title,
    label,
    canDo: `I can use chapter ${chapter}.`,
    spineNodes: [],
    payoff: {
      lesson: `TEST-C${chapter}-payoff`,
      kind: "task",
      summary: `Use chapter ${chapter}.`,
      assesses: [],
    },
  };
}

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

  it("renders compiled activities as linked review questions and answers", () => {
    const withActivity = (
      id: string,
      sequence: number,
      word: string,
      activityId: string,
      answer: string,
    ) =>
      parseLesson(
        source(id, sequence, word).replace(
          `## Wrap-up Recall\n\nSay *${word}*.`,
          `## Wrap-up Recall
<!-- hl-knowledge: introduces=[]; assesses=[TEST-RECALL] -->
<!-- hl-activity: {"id":"${activityId}","kind":"text","assesses":["TEST-RECALL"],"prompt":"Type ${word}.","answer":"${answer}","accepted":["${word}-variant"],"feedback":{"correct":"Right.","incorrect":"Try again."},"response_seconds":8} -->

Say *${word}*.`,
        ),
        "test",
      );
    const later = withActivity("B", 20, "bye", "TEST-bye", "goodbye");
    const earlier = withActivity("A", 10, "hello", "TEST-hello", "hello");
    const generated = renderBookAnswerKey(
      { language: "test", output: "test/book/chapters/appendix-answer-key.tex" },
      [later, earlier],
    );

    expect(generated).toContain("% canonical-activities: 2");
    expect(generated).toContain("\\chapter*{Review Questions}");
    expect(generated).toContain("\\hypertarget{review-TEST-hello}{\\textbf{1.1}}");
    expect(generated).toContain("\\hypertarget{review-TEST-bye}{\\textbf{1.2}}");
    expect(generated.indexOf("Type hello.")).toBeLessThan(generated.indexOf("Type bye."));
    expect(generated).toContain("\\chapter*{Answer Key}");
    expect(generated).toContain("\\hyperlink{review-TEST-hello}{\\textbf{1.1}}");
    expect(generated).toContain("\\textbf{Answer:} hello");
    expect(generated).toContain("\\textbf{Also accepted:} hello-variant");
  });

  it("uses the configured book font for target-script prompts and answers", () => {
    const lesson = parseLesson(
      source("A", 10, "namaste").replace(
        "## Wrap-up Recall\n\nSay *namaste*.",
        `## Wrap-up Recall
<!-- hl-knowledge: introduces=[]; assesses=[TEST-RECALL] -->
<!-- hl-activity: {"id":"TEST-namaste","kind":"text","assesses":["TEST-RECALL"],"prompt":"Type नमस्ते.","answer":"नमस्ते","accepted":["namaste"],"feedback":{"correct":"Right.","incorrect":"Try again."},"response_seconds":8} -->

Say namaste.`,
      ),
      "test",
      "devanagari",
    );
    const generated = renderBookAnswerKey(
      {
        language: "test",
        output: "test/book/chapters/appendix-answer-key.tex",
        unicodeScript: "Devanagari",
        scriptCommand: "dv",
      },
      [lesson],
    );
    expect(generated).toContain("Type \\dv{नमस्ते}.");
    expect(generated).toContain("\\textbf{Answer:} \\dv{नमस्ते}");
  });

  it("rejects empty answer keys and duplicate activity ids", () => {
    const plain = parseLesson(source("A", 10, "hello"), "test");
    const answerTarget = {
      language: "test",
      output: "test/book/chapters/appendix-answer-key.tex",
    };
    expect(() => renderBookAnswerKey(answerTarget, [plain])).toThrow(
      /no compiled lesson activities/,
    );

    const activity = source("A", 10, "hello").replace(
      "## Wrap-up Recall\n\nSay *hello*.",
      `## Wrap-up Recall
<!-- hl-knowledge: introduces=[]; assesses=[TEST-RECALL] -->
<!-- hl-activity: {"id":"TEST-same","kind":"text","assesses":["TEST-RECALL"],"prompt":"Type hello.","answer":"hello","accepted":[],"feedback":{"correct":"Right.","incorrect":"Try again."},"response_seconds":8} -->

Say hello.`,
    );
    const first = parseLesson(activity, "test");
    const second = parseLesson(
      activity.replace("id: A", "id: B").replace("sequence: 10", "sequence: 20"),
      "test",
    );
    expect(() => renderBookAnswerKey(answerTarget, [first, second])).toThrow(
      /duplicate activity id 'TEST-same'/,
    );
  });

  it("renders a deduplicated English-first subject index with typed facets and chapter links", () => {
    const first = parseLesson(
      source("A", 10, "नमस्ते")
        .replace("gloss: नमस्ते", "gloss: greeting\nromanization: namaste")
        .replace(
          "## Guided Practice",
          `## Sounds you'll need

Hear the greeting.

## Script: the greeting

Read the greeting.

## Writing: the greeting

Write the greeting.

## Grammar Lens: formality

Notice the form.

## The word, taken apart

Trace the history.

## Why it's said this way

Use it politely.

## Guided Practice`,
        ),
      "test",
      "devanagari",
    );
    const repeated = parseLesson(
      source("B", 20, "नमस्ते")
        .replace("chapter: 1", "chapter: 2")
        .replace("gloss: नमस्ते", "gloss: greeting\nromanization: namaste"),
      "test",
      "devanagari",
    );
    const grammar = parseLesson(
      source("C", 30, "Agreement").replace("type: word", "type: grammar"),
      "test",
    );
    const practice = parseLesson(
      source("D", 40, "Drill").replace("type: word", "type: practice"),
      "test",
    );
    const generated = renderBookIndex(
      {
        language: "test",
        output: "test/book/chapters/appendix-index.tex",
        unicodeScript: "Devanagari",
        scriptCommand: "dv",
      },
      [practice, grammar, repeated, first],
      [
        capability(2, "Polite speech", "ch:polite"),
        capability(1, "Introductions & Greetings", "ch:greetings"),
      ],
    );

    expect(generated).toContain("% canonical-index-candidates: 5");
    expect(generated).toContain("% canonical-index-entries: 4");
    expect(generated).toContain("\\chapter*{Index}");
    expect(generated).toContain("\\section*{G}");
    expect(generated).toContain("\\textbf{greeting}\\enspace\\emph{\\dv{नमस्ते}}\\enspace(namaste)");
    expect(generated).toContain(
      "explicit focus: pronunciation, script, writing, grammar, etymology, usage and culture",
    );
    expect(generated).toContain(
      "\\hyperref[ch:greetings]{Chapter~1, p.~\\pageref*{ch:greetings}}; " +
        "\\hyperref[ch:polite]{Chapter~2, p.~\\pageref*{ch:polite}}",
    );
    expect(generated).toContain("grammar topic");
    expect(generated).toContain("chapter topic");
    expect(generated).not.toContain("Drill");
  });

  it("rejects index entries whose chapter is absent from the capability ledger", () => {
    const lesson = parseLesson(
      source("A", 10, "hello").replace("chapter: 1", "chapter: 2"),
      "test",
    );
    expect(() =>
      renderBookIndex(
        { language: "test", output: "test/book/chapters/appendix-index.tex" },
        [lesson],
        [capability(1, "Greetings", "ch:greetings")],
      ),
    ).toThrow(/chapter 2 is not in the capability ledger/);
    expect(() =>
      renderBookIndex(
        { language: "test", output: "test/book/chapters/appendix-index.tex" },
        [lesson],
        [],
      ),
    ).toThrow(/no canonical chapter capabilities/);
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
