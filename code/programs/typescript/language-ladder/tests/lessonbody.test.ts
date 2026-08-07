import { describe, expect, it } from "vitest";
import { lessonSections } from "../src/lessonbody.ts";

describe("lessonSections", () => {
  it("turns authored Markdown into safe readable sections", () => {
    expect(lessonSections(`# Title\n\n## Notice\nRead **سلام**.\n\n- Say [salâm](https://example.test).`)).toEqual([
      { title: "Notice", blocks: ["Read سلام.", "• Say salâm."] },
    ]);
  });

  it("keeps block-boundary knowledge metadata out of learner copy", () => {
    expect(lessonSections(`# Title

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[ES-LEX-HOLA] -->

Say *hola*.`)).toEqual([
      { title: "Guided Practice", blocks: ["Say hola."] },
    ]);
  });

  it("keeps compiled activity metadata out of learner copy", () => {
    expect(lessonSections(`# Title

## Wrap-up Recall
<!-- hl-knowledge: introduces=[]; assesses=[ES-GRAMMAR-NOUN-GENDER] -->
<!-- hl-activity: {"id":"ES-G01-count","kind":"text"} -->

Recall the two classes.`)).toEqual([
      { title: "Wrap-up Recall", blocks: ["Recall the two classes."] },
    ]);
  });
});
