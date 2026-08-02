import { describe, expect, it } from "vitest";
import { lessonSections } from "../src/lessonbody.ts";

describe("lessonSections", () => {
  it("turns authored Markdown into safe readable sections", () => {
    expect(lessonSections(`# Title\n\n## Notice\nRead **سلام**.\n\n- Say [salâm](https://example.test).`)).toEqual([
      { title: "Notice", blocks: ["Read سلام.", "• Say salâm."] },
    ]);
  });
});
