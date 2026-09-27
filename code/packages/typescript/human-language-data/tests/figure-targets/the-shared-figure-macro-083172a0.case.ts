import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { defaultCurriculumRoot } from "../../src/loader.js";

describe("the shared figure macro", () => {
  it("bounds a block figure's height as well as its width", () => {
    // A filmstrip of a tall, narrow letter scaled to the line width alone ran
    // hundreds of points off the page in CI's XeLaTeX build (persian and urdu
    // alef). No test here compiles TeX, so the bound itself is pinned.
    const visual = readFileSync(join(defaultCurriculumRoot(), "_shared", "visual.tex"), "utf8");
    const block = visual.slice(visual.indexOf("\\newcommand{\\hlblockfigure}"));
    const options = /\\includegraphics\[([^\]]*)\]/.exec(block)?.[1] ?? "";
    expect(options).toMatch(/\bwidth=/);
    expect(options).toMatch(/\bheight=/);
    expect(options).toContain("keepaspectratio");
  });
});
