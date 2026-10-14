import type { ScriptFilmstripTarget } from "../../src/figure.js";
import type { ParsedLesson } from "../../src/parse.js";
import { realCorpusLessons, realCorpusRoot, realCorpusTargets } from "./real-corpus-fixture.js";

// The shard owner imports five real-corpus groups into one worker. Retaining a
// fresh parsed corpus per group exhausted its 2 GiB heap. All these assertions
// only read the lessons and targets, so share one snapshot within that owner.
// The per-script real-corpus cases load theirs through real-corpus-fixture.ts in
// the same worker, so these names alias that snapshot instead of parsing a
// second copy of the corpus.
// Synthetic lessons still come from fixture.ts and can be mutated independently.
export const realFigureRoot = realCorpusRoot;
export const realFigureLessons: readonly ParsedLesson[] = realCorpusLessons;
export const realFigureTargets: readonly ScriptFilmstripTarget[] = realCorpusTargets;
export const realFigureById: ReadonlyMap<string, ParsedLesson> = new Map(
  realFigureLessons.map((entry) => [entry.realization.lessonId, entry]),
);
export const realFigureDrawn: ReadonlySet<string> = new Set(
  realFigureTargets.map((target) => target.lessonId),
);
