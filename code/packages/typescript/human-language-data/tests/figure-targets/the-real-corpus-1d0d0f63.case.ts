// HL-C443 — a single-letter writing lesson gets its filmstrip without anyone
// declaring it, and every filmstrip the curriculum resolves is actually PRINTED.
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { resolvedFigureTargets } from "../../src/figure-cli.js";
import type { ScriptFilmstripTarget } from "../../src/figure.js";
import { defaultCurriculumRoot, loadLessons } from "../../src/loader.js";
import type { ParsedLesson } from "../../src/parse.js";
import { loadFilmstripTargetCountPins } from "../filmstrip-target-count-pins.js";

describe("the real corpus", () => {
  const root = defaultCurriculumRoot();
  const lessons = loadLessons(root);
  const targets = resolvedFigureTargets(root, lessons).filter(
    (target): target is ScriptFilmstripTarget => target.kind === "script-filmstrip",
  );

  it("draws a filmstrip for every Tamil letter lesson whose letter has a cited ductus", () => {
    // 30 lessons, 29 letters. வ has both TA-S01-va and the guided copy TA-W00,
    // while a letter lesson without cited ductus remains an undrawn candidate.
    const tamil = targets.filter((target) => target.lessonId.startsWith("TA-"));
    expect(tamil).toHaveLength(30);
    expect(new Set(tamil.map((target) => target.glyph)).size).toBe(29);
  });

  it("draws every switched-on track exactly the letters its ductus cites", () => {
    const counts: Record<string, number> = {};
    for (const target of targets) {
      const prefix = target.lessonId.split("-")[0]!;
      counts[prefix] = (counts[prefix] ?? 0) + 1;
    }
    const pins = loadFilmstripTargetCountPins(
      join(dirname(fileURLToPath(import.meta.url)), "..", "filmstrip-target-counts"),
    );
    const byTrack = Object.fromEntries(
      Object.entries(counts).map(([prefix, count]) => [
        targets.find((target) => target.lessonId.startsWith(`${prefix}-`))!.output.split("/", 1)[0]!,
        count,
      ]),
    );
    expect(byTrack).toEqual(pins);
  });

  it("prints every resolved filmstrip in its chapter", () => {
    // A generated figure that is never placed is a figure no reader sees.
    const byLesson = new Map(lessons.map((entry) => [entry.realization.lessonId, entry]));
    for (const target of targets) {
      const owner = byLesson.get(target.lessonId);
      expect(owner, target.lessonId).toBeDefined();
      const chapterFile = chapterTexFor(root, owner!);
      const pdf = target.output.split("/").pop()!.replace(/\.svg$/, ".pdf");
      expect(readFileSync(chapterFile, "utf8"), `${target.lessonId} in ${chapterFile}`).toContain(pdf);
    }
  });
});

function chapterTexFor(root: string, owner: ParsedLesson): string {
  const targets = JSON.parse(
    readFileSync(
      join(
        root,
        "core",
        "book-generation.d",
        "targets.d",
        `${owner.language}-${String(owner.realization.chapter).padStart(4, "0")}.json`,
      ),
      "utf8",
    ),
  ) as { output: string };
  return join(root, targets.output);
}
