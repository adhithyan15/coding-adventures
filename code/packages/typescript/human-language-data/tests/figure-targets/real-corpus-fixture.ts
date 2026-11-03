import { readFileSync } from "node:fs";
import { join } from "node:path";
import { resolvedFigureTargets } from "../../src/figure-cli.js";
import type { ScriptFilmstripTarget } from "../../src/figure.js";
import { filmstripCandidates } from "../../src/figure-targets.js";
import { defaultCurriculumRoot, forEachTrackLessons } from "../../src/loader.js";
import type { ParsedLesson } from "../../src/parse.js";
import { STRIP_PROSE, STROKE_ORDER_DISCLAIMER } from "./strip-prose.js";

// The real corpus as the figure-target cases see it: read ONCE for the whole
// suite, one track at a time, keeping only what a case asserts on.
//
// Why not just `loadLessons()`. This suite used to hold five whole copies of the
// parsed corpus at once: this fixture loaded one, and three cases each loaded
// another (one of them twice) inside a `describe`, where it stayed reachable
// for the life of the worker. At ~62,000 lessons that peaked at ~3.9 GB RSS and
// killed a fork worker on the 2 GB macOS heap (PR #17737). The cases never
// needed the whole corpus. They need:
//
//   - the resolved filmstrip targets,
//   - the parsed lessons that OWN a target (to place and print its strip),
//   - which lesson ids exist at all, and
//   - which lessons' bodies carry the two strip-related phrases.
//
// All four are small, and all four can be read off one track at a time, so at
// most one track's parsed lessons are alive during the scan:
//
//   arabic  -> candidates, owners, ids, phrase hits -> dropped
//   bengali -> candidates, owners, ids, phrase hits -> dropped
//   ...
//
// The targets are exactly what `resolvedFigureTargets(root, loadLessons(root))`
// returns. That function reads lessons only through `filmstripCandidates`, which
// judges each lesson on its own and sorts its result by lesson id, so handing it
// just the lessons that yield a candidate (plus the owners of declared targets,
// which yield none of their own) produces the same candidates and therefore the
// same targets.

export const realCorpusRoot = defaultCurriculumRoot();

// `figure-generation.json`'s declared targets. With no lessons there are no
// derived candidates, and `resolvedFigureTargets` returns the declared list as is.
const declaredOwners = new Set(
  resolvedFigureTargets(realCorpusRoot, []).map((target) => target.lessonId),
);

const ownerLessons: ParsedLesson[] = [];
const lessonIds = new Set<string>();
const disclaiming = new Set<string>();
const stripProse = new Set<string>();
forEachTrackLessons(realCorpusRoot, (_language, lessons) => {
  const candidates = new Set(filmstripCandidates(lessons).map((target) => target.lessonId));
  for (const lesson of lessons) {
    const id = lesson.realization.lessonId;
    lessonIds.add(id);
    if (STROKE_ORDER_DISCLAIMER.test(lesson.body)) disclaiming.add(id);
    if (STRIP_PROSE.test(lesson.body)) stripProse.add(id);
    if (candidates.has(id) || declaredOwners.has(id)) ownerLessons.push(lesson);
  }
});

export const realCorpusTargets = resolvedFigureTargets(realCorpusRoot, ownerLessons).filter(
  (target): target is ScriptFilmstripTarget => target.kind === "script-filmstrip",
);

/** Every lesson that could own a figure target, by id: a superset of the drawn ones. */
export const realCorpusTargetOwners: ReadonlyMap<string, ParsedLesson> = new Map(
  ownerLessons.map((lesson) => [lesson.realization.lessonId, lesson]),
);

/** Every lesson id in the real corpus. */
export const realCorpusLessonIds: ReadonlySet<string> = lessonIds;

/** The ids of the lessons whose body matches `STROKE_ORDER_DISCLAIMER`, in corpus order. */
export const realCorpusDisclaimingLessons: ReadonlySet<string> = disclaiming;

/** The ids of the lessons whose body matches `STRIP_PROSE`, in corpus order. */
export const realCorpusStripProseLessons: ReadonlySet<string> = stripProse;

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
