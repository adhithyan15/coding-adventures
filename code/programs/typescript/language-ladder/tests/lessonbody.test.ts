import { describe, expect, it } from "vitest";
import { lessonSections } from "../src/lessonbody.ts";

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
