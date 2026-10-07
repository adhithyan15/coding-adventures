import { describe, expect, it } from "vitest";
import { MODELLED_WRITING_STAGES as BOOK_MODELLED_WRITING_STAGES } from "@coding-adventures/human-language-data/src/figure-targets.ts";
import { MODELLED_WRITING_STAGES, filmstripSectionIndex, lessonSections } from "../src/lessonbody.ts";

describe("lessonSections", () => {
  it("turns authored Markdown into safe readable sections", () => {
    expect(lessonSections(`# Title\n\n## Notice\nRead **سلام**.\n\n- Say [salâm](https://example.test).`)).toEqual([
      {
        title: "Notice",
        blocks: [
          { kind: "text", text: "Read سلام." },
          { kind: "text", text: "• Say salâm." },
        ],
      },
    ]);
  });

  it("keeps block-boundary knowledge metadata out of learner copy", () => {
    expect(lessonSections(`# Title

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[ES-LEX-HOLA] -->

Say *hola*.`)).toEqual([
      { title: "Guided Practice", blocks: [{ kind: "text", text: "Say hola." }] },
    ]);
  });

  it("keeps compiled activity metadata out of learner copy", () => {
    expect(lessonSections(`# Title

## Wrap-up Recall
<!-- hl-knowledge: introduces=[]; assesses=[ES-GRAMMAR-NOUN-GENDER] -->
<!-- hl-activity: {"id":"ES-G01-count","kind":"text"} -->

Recall the two classes.`)).toEqual([
      { title: "Wrap-up Recall", blocks: [{ kind: "text", text: "Recall the two classes." }] },
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
      { title: "Guided Practice", writingStage: "guided-copy", blocks: [{ kind: "text", text: "Copy 汉 twice." }] },
    ]);
  });

  it("puts the strip in Writing, else Script, else the first section that shows a model", () => {
    const sections = lessonSections(`# T

## Warm-up
Look.

## Guided Practice — Spaced Return
<!-- hl-writing-stage: dictation-transcription -->

Write it without a model.

## Guided Practice
<!-- hl-writing-stage: delayed-copy -->

Study, cover, write.`);
    expect(filmstripSectionIndex(sections)).toBe(2);
    expect(filmstripSectionIndex([{ title: "Script — the shape", blocks: [] }, ...sections])).toBe(0);
    expect(filmstripSectionIndex(sections.slice(0, 2))).toBe(-1);
    expect(filmstripSectionIndex([{ title: "Guided Practice — decide before writing", blocks: [] }])).toBe(-1);
  });

  it("never puts the strip in a Writing section that is a dictation or a composition", () => {
    // ES-W00-hola-dictation: the strip would sit above "Hear: OH-la".
    const dictation = lessonSections(`# T

## Warm-up
Cover the answer line lower on this page.

## Writing — short dictation
<!-- hl-writing-stage: dictation-transcription -->

Write the Spanish greeting from that sound alone.`);
    expect(filmstripSectionIndex(dictation)).toBe(-1);
    // MR-W03-ba: the Script section shows the letter the dictation covers.
    const marathi = lessonSections(`# T

## Script
> ब

## Writing — heard cue
<!-- hl-writing-stage: dictation-transcription -->

Cover the model.`);
    expect(filmstripSectionIndex(marathi)).toBe(0);
    for (const stage of ["controlled-composition", "connected-composition", "timed-assessment-production"]) {
      expect(filmstripSectionIndex([{ title: "Writing — task", blocks: [], writingStage: stage }])).toBe(-1);
    }
    expect(filmstripSectionIndex([{ title: "Writing — copy", blocks: [], writingStage: "guided-copy" }])).toBe(0);
  });

  it("uses exactly the book's modelled stages", () => {
    expect([...MODELLED_WRITING_STAGES].sort()).toEqual([...BOOK_MODELLED_WRITING_STAGES].sort());
  });
});
