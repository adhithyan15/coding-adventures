/**
 * A mock's declaration says what must be taught; it is not proof that the
 * book did so. Resolve only against the level-filtered, parsed lesson corpus
 * and against anchored reads of the form's actual stimuli. In particular, an
 * issue closing or a manifest being added cannot make a requirement pass.
 */
import { existsSync } from "node:fs";
import { join } from "node:path";

import type { AssessmentMockBookPrerequisite } from "./assessment.js";
import { lessonsUpToLevel, type CefrLevel } from "./levels.js";
import { loadCurriculumSpine, loadTrackLanguageCurriculum, loadTrackLessons } from "./loader.js";
import { readMockEvidence } from "./mock-evidence-read.js";
import type { ParsedLesson } from "./parse.js";

const GURMUKHI_TOKEN = /[\u0A00-\u0A7F]+/gu;

/** Only exact script tokens count: a related lemma or substring is not taught. */
export function gurmukhiTokens(text: string): Set<string> {
  return new Set(text.match(GURMUKHI_TOKEN) ?? []);
}

function introducedAndPractised(lessons: readonly ParsedLesson[], atom: string): boolean {
  return lessons.some((lesson) => {
    const introduced = lesson.frontmatter["introduces.knowledge"];
    if (!Array.isArray(introduced) || !introduced.includes(atom)) return false;
    const teachingBlock = lesson.blocks.some((block) =>
      block.knowledge?.introduces.includes(atom) && block.markdown.trim().length > 0);
    const productiveTask = lesson.blocks.some((block) =>
      block.activities?.some((activity) =>
        activity.assesses.includes(atom) && activity.prompt.trim().length > 0
        && activity.answer.trim().length > 0));
    return teachingBlock && productiveTask;
  });
}

/**
 * One track is loaded lazily and only when it declares book prerequisites.
 * The level filter is the same one used to construct an A1/A2 book: a later
 * chapter cannot retroactively make an earlier paper fair. A missing
 * curriculum mapping leaves the requirement unresolved, not optimistically
 * taught.
 */
export function unresolvedMockBookPrerequisites(
  root: string, language: string, level: CefrLevel, mockId: string,
  prerequisites: readonly AssessmentMockBookPrerequisite[],
): string[] {
  if (prerequisites.length === 0) return [];
  let eligible: ParsedLesson[] | undefined;
  const lessons = (): ParsedLesson[] => {
    if (eligible !== undefined) return eligible;
    const curriculum = loadTrackLanguageCurriculum(language, root);
    eligible = curriculum === undefined ? [] : lessonsUpToLevel(
      loadTrackLessons(language, root), [curriculum], loadCurriculumSpine(root), level,
    );
    return eligible;
  };
  const missing: string[] = [];
  for (const requirement of prerequisites) {
    let satisfied = false;
    if (requirement.kind === "lesson-knowledge") {
      satisfied = requirement.knowledgeIds.every((atom) => introducedAndPractised(lessons(), atom));
    } else {
      // A missing draft remains blocked. If a path is present but has become
      // a symlink or is swapped after this probe, the anchored reader fails
      // closed; it never follows the path out of this track.
      const present = requirement.sources.every((path) => existsSync(join(root, language, path)));
      if (present) {
        const source = readMockEvidence(root, language, requirement.sources, []);
        const tested = gurmukhiTokens(requirement.sources.map((path) => source.get(path) ?? "").join("\n"));
        const taught = gurmukhiTokens(lessons().map((lesson) => [
          lesson.realization.headword,
          ...lesson.blocks.flatMap((block) => [
            block.markdown,
            ...(block.activities ?? []).flatMap((activity) => [activity.prompt, activity.answer]),
          ]),
        ].join("\n")).join("\n"));
        satisfied = tested.size > 0 && [...tested].every((token) => taught.has(token));
      }
    }
    if (!satisfied) missing.push(`book-teaching/${language}/${level}/${mockId}/${requirement.id}`);
  }
  return missing;
}
