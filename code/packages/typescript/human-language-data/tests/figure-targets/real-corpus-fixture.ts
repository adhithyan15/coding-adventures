import { readFileSync } from "node:fs";
import { join } from "node:path";
import { resolvedFigureTargets } from "../../src/figure-cli.js";
import type { ScriptFilmstripTarget } from "../../src/figure.js";
import { defaultCurriculumRoot, loadLessons } from "../../src/loader.js";
import type { ParsedLesson } from "../../src/parse.js";

export const realCorpusRoot = defaultCurriculumRoot();
export const realCorpusLessons = loadLessons(realCorpusRoot);
export const realCorpusTargets = resolvedFigureTargets(realCorpusRoot, realCorpusLessons).filter(
  (target): target is ScriptFilmstripTarget => target.kind === "script-filmstrip",
);

export function chapterTexFor(root: string, owner: ParsedLesson): string {
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
