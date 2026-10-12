import { resolvedFigureTargets } from "../../src/figure-cli.js";
import type { ScriptFilmstripTarget } from "../../src/figure.js";
import { defaultCurriculumRoot, loadLessons } from "../../src/loader.js";
import type { ParsedLesson } from "../../src/parse.js";

// The shard owner imports five real-corpus groups into one worker. Retaining a
// fresh parsed corpus per group exhausted its 2 GiB heap. All these assertions
// only read the lessons and targets, so share one snapshot within that owner.
// Synthetic lessons still come from fixture.ts and can be mutated independently.
export const realFigureRoot = defaultCurriculumRoot();
export const realFigureLessons: readonly ParsedLesson[] = loadLessons(realFigureRoot);
export const realFigureTargets: readonly ScriptFilmstripTarget[] = resolvedFigureTargets(
  realFigureRoot,
  realFigureLessons,
).filter((target): target is ScriptFilmstripTarget => target.kind === "script-filmstrip");
export const realFigureById: ReadonlyMap<string, ParsedLesson> = new Map(
  realFigureLessons.map((entry) => [entry.realization.lessonId, entry]),
);
export const realFigureDrawn: ReadonlySet<string> = new Set(
  realFigureTargets.map((target) => target.lessonId),
);
