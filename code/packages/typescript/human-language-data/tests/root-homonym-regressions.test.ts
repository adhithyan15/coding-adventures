// Regression pins for the cross-track homonyms split in #17565. The cousins
// join uses exact roots: strings, so reusing one slug for two etymons would
// silently claim that unrelated words share an origin.
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { buildCousinIndex, cousinsFor, rootSlugs } from "../src/cousins.js";
import { defaultCurriculumRoot } from "../src/loader.js";
import { parseLesson } from "../src/parse.js";

function realLesson(language: string, id: string) {
  const path = join(defaultCurriculumRoot(), language, "lessons", `${id}.md`);
  return parseLesson(readFileSync(path, "utf8"), language);
}

const pinned = [
  ["sanskrit", "SA-C19-nine", ["sanskrit-nava-nine"]],
  ["gujarati", "GU-C39-nav", ["sanskrit-nava-nine"]],
  ["sanskrit", "SA-C15-new", ["sanskrit-nava-new"]],
  ["gujarati", "GU-C73-new", ["sanskrit-nava-new"]],
  ["sanskrit", "SA-C30-garland", ["sanskrit-mala-garland"]],
  ["hindi", "HI-C51-garland", ["sanskrit-mala-garland"]],
  ["telugu", "TE-C288-malabaddhakam", ["sanskrit-mala-dirt", "sanskrit-baddha"]],
  ["sanskrit", "SA-C44-yellow", ["sanskrit-pita-yellow"]],
  ["hindi", "HI-C24-pila-history", ["sanskrit-pitala", "sanskrit-pita-yellow"]],
  ["bengali", "BN-C14-kalo", ["sanskrit-kala-black"]],
  ["hindi", "HI-C11-kaalaa-safed", ["sanskrit-kala-black", "safed-persian"]],
  ["kannada", "KA-C305-kalavida", ["sanskrit-kala-art"]],
  ["malayalam", "ML-C258-caturam", ["sanskrit-catura-square"]],
  ["tamil", "TA-C240-caturamana", ["sanskrit-caturasra"]],
  ["french", "FR-C131-libre", ["liber-free-latin"]],
  ["latin", "LA-C71-liberi", ["liberi-latin", "liber-free-latin"]],
  ["spanish", "ES-C43-libro", ["liber-latin"]],
] as const;

describe("real-corpus homonym splits", () => {
  it.each(pinned)("pins all roots for %s %s", (language, id, slugs) => {
    // Pin the complete list: a correct slug alongside a wrong homonym would
    // still create a false cousin edge, and a mere toContain would miss it.
    expect(rootSlugs(realLesson(language, id))).toEqual(slugs);
  });

  it("pairs nine with nine and new with new, never nine with new", () => {
    const nine = realLesson("sanskrit", "SA-C19-nine");
    const newWord = realLesson("sanskrit", "SA-C15-new");
    const index = buildCousinIndex([
      nine,
      newWord,
      realLesson("gujarati", "GU-C39-nav"),
      realLesson("gujarati", "GU-C73-new"),
    ]);
    expect(cousinsFor(index, nine, ["gujarati"]).map((entry) => entry.lessonId)).toEqual(["GU-C39-nav"]);
    expect(cousinsFor(index, newWord, ["gujarati"]).map((entry) => entry.lessonId)).toEqual(["GU-C73-new"]);
  });

  it("pairs garlands without pulling in dirt", () => {
    const garland = realLesson("sanskrit", "SA-C30-garland");
    const index = buildCousinIndex([
      garland,
      realLesson("hindi", "HI-C51-garland"),
      realLesson("telugu", "TE-C288-malabaddhakam"),
    ]);
    expect(cousinsFor(index, garland, ["hindi", "telugu"]).map((entry) => entry.lessonId)).toEqual([
      "HI-C51-garland",
    ]);
  });

  it("keeps yellow on the yellow etymon, not the drunk homonym", () => {
    for (const [language, id] of [
      ["sanskrit", "SA-C44-yellow"],
      ["hindi", "HI-C24-pila-history"],
    ] as const) {
      const roots = rootSlugs(realLesson(language, id));
      expect(roots).toContain("sanskrit-pita-yellow");
      expect(roots).not.toContain("sanskrit-pita-drunk");
    }
  });

  it("pairs black with black, not with art", () => {
    const black = realLesson("bengali", "BN-C14-kalo");
    const index = buildCousinIndex([
      black,
      realLesson("hindi", "HI-C11-kaalaa-safed"),
      realLesson("kannada", "KA-C305-kalavida"),
    ]);
    expect(cousinsFor(index, black, ["hindi", "kannada"]).map((entry) => entry.lessonId)).toEqual([
      "HI-C11-kaalaa-safed",
    ]);
  });

  it("pairs free with free, not with book", () => {
    const free = realLesson("french", "FR-C131-libre");
    const index = buildCousinIndex([
      free,
      realLesson("latin", "LA-C71-liberi"),
      realLesson("spanish", "ES-C43-libro"),
    ]);
    expect(cousinsFor(index, free, ["latin", "spanish"]).map((entry) => entry.lessonId)).toEqual([
      "LA-C71-liberi",
    ]);
  });
});
