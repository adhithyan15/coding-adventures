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
    // 30 one-letter lessons, 29 letters. வ has both TA-S01-va and the guided
    // copy TA-W00, while a letter lesson without cited ductus remains an
    // undrawn candidate.
    const tamil = targets.filter((target) => target.lessonId.startsWith("TA-"));
    const single = tamil.filter((target) => target.letters === undefined);
    expect(single).toHaveLength(30);
    expect(new Set(single.map((target) => target.glyph)).size).toBe(29);
  });

  it("draws every Tamil letter-list lesson whose letters are all cited, and no Tamil word yet", () => {
    // Four lessons list letters ("வ, க"). Every Tamil WORD headword so far
    // carries a vowel sign or pulli, and marks are never composed, so none is
    // drawn — `writingSequenceOf` refuses them before the ledger is asked.
    const sequences = targets.filter(
      (target) => target.lessonId.startsWith("TA-") && target.letters !== undefined,
    );
    expect(
      Object.fromEntries(sequences.map((target) => [target.lessonId, target.letters!.join(" ")])),
    ).toEqual({
      "TA-W01-abugida-va-ka": "வ க",
      "TA-W02-ma-retroflex-na": "ம ண",
      "TA-W02-three-ns": "ந ன ண",
      "TA-W04-vowel-signs-nandri": "ந ன ற",
    });
  });

  it("composes words only in scripts whose letters stand apart", () => {
    // The three Japanese words are the corpus's only word sequences today.
    // Devanagari (मम), the Arabic family (سلام) and Cyrillic (привет) have
    // fully cited words that are deliberately NOT drawn: see
    // SEPARATE_LETTER_SCRIPTS for why each would draw something false.
    const words = targets.filter(
      (target) => target.letters !== undefined && target.letters.join("") === target.glyph,
    );
    expect(words.map((target) => target.lessonId).sort()).toEqual([
      "JA-W01-hai-read",
      "JA-W01-konnichiwa-read",
      "JA-W08-sayounara-read",
    ]);
    const lessonIds = new Set(targets.map((target) => target.lessonId));
    for (const id of ["SA-W03-mama-guided-copy", "UR-W04-joining", "RU-W05-privet-guided-copy"]) {
      expect(lessonIds.has(id), id).toBe(false);
    }
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
