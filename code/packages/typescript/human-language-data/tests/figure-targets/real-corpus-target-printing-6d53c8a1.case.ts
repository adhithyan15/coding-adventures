import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { realCorpusRoot as root, realCorpusTargetOwners as byLesson, realCorpusTargets as targets, chapterTexFor } from "./real-corpus-fixture.js";

describe("real-corpus figure-target printing", () => {
  it("prints every resolved filmstrip in its chapter", () => {
    // A generated figure that is never placed is a figure no reader sees.
    for (const target of targets) {
      const owner = byLesson.get(target.lessonId);
      expect(owner, target.lessonId).toBeDefined();
      const chapterFile = chapterTexFor(root, owner!);
      const pdf = target.output.split("/").pop()!.replace(/\.svg$/, ".pdf");
      expect(readFileSync(chapterFile, "utf8"), `${target.lessonId} in ${chapterFile}`).toContain(pdf);
    }
  });
});
