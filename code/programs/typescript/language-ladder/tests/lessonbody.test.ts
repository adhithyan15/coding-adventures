import { describe, expect, it } from "vitest";
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";
import { filmstripBlockIndex as bookFilmstripBlockIndex } from "@coding-adventures/human-language-data/src/figure-targets.ts";
import { parseLesson } from "@coding-adventures/human-language-data/src/parse.ts";
import { filmstripSectionIndex, lessonSections } from "../src/lessonbody.ts";
import { REAL_LESSONS } from "./real-lessons.ts";

describe("lessonSections", () => {
  it("turns authored Markdown into safe readable sections", () => {
    expect(lessonSections(`# Title\n\n## Notice\nRead **سلام**.\n\n- Say [salâm](https://example.test).`)).toEqual([
      {
        title: "Notice",
        blockIndex: 0,
        blocks: [
          { kind: "text", text: "Read سلام." },
          { kind: "text", text: "• Say salâm." },
        ],
      },
    ]);
  });

  it("strips links and images in linear time, even when brackets never close", () => {
    // Answers only: a timing bound would flake on a loaded runner. These inputs
    // took about two seconds each before the bracket classes excluded their
    // own openers.
    const text = (markdown: string) => lessonSections(`## S\n${markdown}`)[0]?.blocks[0];
    expect(text("Say [salâm](u) and ![a cup](f.svg).")).toEqual({ kind: "text", text: "Say salâm and a cup." });
    for (const flood of ["[".repeat(50_000), "![".repeat(25_000), "[a](".repeat(12_500), "![a](".repeat(10_000)]) {
      expect(text(`x ${flood}`)?.kind).toBe("text");
    }
  });

  it("keeps block-boundary knowledge metadata out of learner copy", () => {
    expect(lessonSections(`# Title

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[ES-LEX-HOLA] -->

Say *hola*.`)).toEqual([
      { title: "Guided Practice", blockIndex: 0, blocks: [{ kind: "text", text: "Say hola." }] },
    ]);
  });

  it("keeps compiled activity metadata out of learner copy", () => {
    expect(lessonSections(`# Title

## Wrap-up Recall
<!-- hl-knowledge: introduces=[]; assesses=[ES-GRAMMAR-NOUN-GENDER] -->
<!-- hl-activity: {"id":"ES-G01-count","kind":"text"} -->

Recall the two classes.`)).toEqual([
      { title: "Wrap-up Recall", blockIndex: 0, blocks: [{ kind: "text", text: "Recall the two classes." }] },
    ]);
  });

  it("preserves a standalone canonical figure as structured safe data", () => {
    expect(lessonSections(`# Title

## The word, taken apart

Before.

![Arabic qahwah to Spanish café](figures/ES-C06-cafe-etymology.svg)

After.`)).toEqual([
      {
        title: "The word, taken apart",
        blockIndex: 0,
        blocks: [
          { kind: "text", text: "Before." },
          {
            kind: "image",
            alt: "Arabic qahwah to Spanish café",
            source: "figures/ES-C06-cafe-etymology.svg",
          },
          { kind: "text", text: "After." },
        ],
      },
    ]);
  });
});

describe("writing stages and the filmstrip's section", () => {
  it("keeps the writing-stage directive out of learner copy and records it", () => {
    expect(lessonSections(`# Title

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[ZH-SCRIPT-HAN-01] -->
<!-- hl-writing-stage: guided-copy -->

Copy **汉** twice.`)).toEqual([
      { title: "Guided Practice", blockIndex: 0, writingStage: "guided-copy", blocks: [{ kind: "text", text: "Copy 汉 twice." }] },
    ]);
  });

  // `filmstripSectionIndex` takes the authored body and the sections built
  // from it, as main.ts does. This helper keeps the cases below one line each.
  // The book reads a block's writing stage only when the directive comes
  // right after the block's knowledge directive (parse.ts), the shape every
  // corpus lesson has, so each fixture below carries both.
  const stripSection = (body: string): string | undefined => {
    const sections = lessonSections(body);
    return sections[filmstripSectionIndex(body, sections)]?.title;
  };

  it("puts the strip in Writing, else Script, else the first section that shows a model", () => {
    const practice = `# T

## Warm-up
Look.

## Guided Practice — Spaced Return
<!-- hl-knowledge: introduces=[]; assesses=[] -->
<!-- hl-writing-stage: dictation-transcription -->

Write it without a model.

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[] -->
<!-- hl-writing-stage: delayed-copy -->

Study, cover, write.`;
    expect(filmstripSectionIndex(practice, lessonSections(practice))).toBe(2);
    expect(stripSection(practice.replace("## Warm-up", "## Script — the shape"))).toBe("Script — the shape");
    expect(stripSection(practice.replace("delayed-copy", "controlled-composition"))).toBeUndefined();
    expect(stripSection("# T\n\n## Guided Practice — decide before writing\nPick one.")).toBeUndefined();
  });

  it("puts the strip in the Writing section even when a Script section comes first, as the book does", () => {
    // AR-W46-dal's shape: the app used to take whichever of Script and
    // Writing came first, so this strip sat one section higher than in print.
    const arabic = `# T

## Warm-up
Look.

## Script you'll notice: د

> د

## Writing: د
<!-- hl-knowledge: introduces=[]; assesses=[] -->
<!-- hl-writing-stage: observe-trace -->

Trace it twice.`;
    expect(stripSection(arabic)).toBe("Writing: د");
    // With no usable Writing section, the Script section is the home.
    expect(stripSection(arabic.replace("observe-trace", "dictation-transcription"))).toBe("Script you'll notice: د");
  });

  it("never puts the strip in a Writing section that is a dictation or a composition", () => {
    // ES-W00-hola-dictation: the strip would sit above "Hear: OH-la".
    expect(stripSection(`# T

## Warm-up
Cover the answer line lower on this page.

## Writing — short dictation
<!-- hl-knowledge: introduces=[]; assesses=[] -->
<!-- hl-writing-stage: dictation-transcription -->

Write the Spanish greeting from that sound alone.`)).toBeUndefined();
    // MR-W03-ba: the Script section shows the letter the dictation covers.
    expect(stripSection(`# T

## Script
> ब

## Writing — heard cue
<!-- hl-knowledge: introduces=[]; assesses=[] -->
<!-- hl-writing-stage: dictation-transcription -->

Cover the model.`)).toBe("Script");
    for (const stage of ["controlled-composition", "connected-composition", "timed-assessment-production"]) {
      expect(stripSection(`# T\n\n## Writing — task\n<!-- hl-knowledge: introduces=[]; assesses=[] -->\n<!-- hl-writing-stage: ${stage} -->\n\nWrite.`)).toBeUndefined();
    }
    expect(stripSection("# T\n\n## Writing — copy\n<!-- hl-knowledge: introduces=[]; assesses=[] -->\n<!-- hl-writing-stage: guided-copy -->\n\nCopy.")).toBe("Writing — copy");
  });

  it("maps the book's block to the right section past a preamble and an empty heading", () => {
    // The preamble is a section here but no book block; the empty "Script"
    // heading is a book block (index 0) but no section here. The strip is in
    // book block 1, which is section 1 (after "Lesson"), not section 0 or 2.
    const body = `# T

Some words before any heading.

## Script

## Writing: ب
<!-- hl-knowledge: introduces=[]; assesses=[] -->
<!-- hl-writing-stage: guided-copy -->

Copy it.`;
    const sections = lessonSections(body);
    expect(sections.map((section) => [section.title, section.blockIndex])).toEqual([
      ["Lesson", undefined],
      ["Writing: ب", 1],
    ]);
    expect(filmstripSectionIndex(body, sections)).toBe(1);
    // A tab after "##" is a heading to this view but not a book block, so it
    // must not shift the count for the blocks after it.
    const tabbed = "# T\n\n##\tNotes\nA note.\n\n## Writing: ب\nCopy it.";
    expect(lessonSections(tabbed).map((section) => section.blockIndex)).toEqual([undefined, 0]);
    expect(stripSection(tabbed)).toBe("Writing: ب");
  });
});

// ---------------------------------------------------------------------------
// The cross-check: every strip in the corpus is where the printed book puts it
// ---------------------------------------------------------------------------
//
// The app calls the book's placement code, so the RULE cannot drift. What can
// still go wrong is the translation from the book's block to this view's
// section (`blockIndex`), or a lesson the two parsers split differently. So,
// for every lesson whose filmstrip the app shows (a writing lesson with a
// committed `<id>-filmstrip.svg`, exactly the set `generatedFilmstripUrl`
// resolves), check that the section the app picks is the very block the book
// prints the strip in: the same block index from the book's `parseLesson` and
// the same heading text.
describe("the filmstrip's section across the whole corpus", () => {
  it("is the block the book prints the strip in, for every lesson with a strip", () => {
    const root = path.resolve(__dirname, "../../../../learning/human-languages");
    const plain = (title: string): string => lessonSections(`## ${title}\nx`)[0]!.title;
    let checked = 0;
    const mismatches: string[] = [];
    for (const lesson of REAL_LESSONS) {
      if (lesson.type !== "writing") continue;
      if (!existsSync(path.join(root, lesson.language, "book", "figures", `${lesson.id}-filmstrip.svg`))) continue;
      checked += 1;
      const source = readFileSync(path.join(root, lesson.language, "lessons", `${lesson.id}.md`), "utf8");
      const book = parseLesson(source, lesson.language);
      const at = bookFilmstripBlockIndex(book);
      const sections = lessonSections(lesson.body);
      const chosen = sections[filmstripSectionIndex(lesson.body, sections)];
      if (at === -1 || chosen === undefined || chosen.blockIndex !== at || chosen.title !== plain(book.blocks[at]!.title)) {
        mismatches.push(`${lesson.id}: app ${chosen?.title ?? "(none)"}, book ${book.blocks[at]?.title ?? "(none)"}`);
      }
    }
    // ~800 strips today; a corpus that failed to load would pass vacuously.
    expect(checked).toBeGreaterThan(700);
    expect(mismatches).toEqual([]);
  });
});
