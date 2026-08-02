// Pure validation for the structured HL04 language registry and shared spine.

import { CONTENT_TYPES, hasOwn } from "./constants.js";
import type {
  BookCorpus,
  CurriculumSpine,
  Issue,
  LanguageRegistry,
  Taxonomy,
} from "./types.js";
import type { ParsedLesson } from "./parse.js";

export interface CurriculumValidationInput {
  registry: LanguageRegistry;
  spine: CurriculumSpine;
  taxonomy: Taxonomy;
  lessons: ParsedLesson[];
  books?: BookCorpus;
}

export function validateCurriculum(input: CurriculumValidationInput): Issue[] {
  const { registry, spine, taxonomy, lessons, books } = input;
  const issues: Issue[] = [];
  const error = (code: string, message: string) =>
    issues.push({ level: "error", code, message });
  const warning = (code: string, message: string, lessonId?: string) =>
    issues.push({ level: "warning", code, message, lessonId });

  const languageIds = new Set<string>();
  for (const language of registry.languages) {
    if (languageIds.has(language.id)) {
      error("duplicate-language", `language registry contains '${language.id}' twice`);
    }
    languageIds.add(language.id);
    if (language.name.trim() === "") {
      error("missing-language-name", `${language.id}: language name is empty`);
    }
    if (language.script.trim() === "") {
      error("missing-language-script", `${language.id}: script id is empty`);
    }
  }

  for (const book of books?.books ?? []) {
    if (!languageIds.has(book.language)) {
      error("unregistered-book-language", `${book.language}: LaTeX book is absent from core/languages.json`);
    }
    const chapterNumbers = new Set<number>();
    for (const chapter of book.chapters) {
      if (chapterNumbers.has(chapter.chapter)) {
        error("duplicate-book-chapter", `${book.language}: book chapter ${chapter.chapter} occurs twice`);
      }
      chapterNumbers.add(chapter.chapter);
      const hasLesson = lessons.some(
        (lesson) => lesson.language === book.language && lesson.realization.chapter === chapter.chapter,
      );
      if (!hasLesson) {
        error(
          "book-chapter-without-lessons",
          `${chapter.source}: no chapter ${chapter.chapter} Markdown lessons exist`,
        );
      }
    }
  }

  const lessonLanguages = new Set(lessons.map((lesson) => lesson.language));
  for (const language of lessonLanguages) {
    if (!languageIds.has(language)) {
      error("unregistered-language", `${language}: lesson track is absent from core/languages.json`);
    }
  }
  for (const language of registry.languages) {
    if (language.status === "active" && !lessonLanguages.has(language.id)) {
      error("active-language-without-lessons", `${language.id}: active language has no lessons`);
    }
  }

  const lessonById = new Map<string, ParsedLesson>();
  for (const lesson of lessons) {
    const id = lesson.realization.lessonId;
    if (lessonById.has(id)) error("duplicate-lesson-id", `lesson id '${id}' occurs twice`);
    else lessonById.set(id, lesson);
    const estimate = Number(lesson.frontmatter.est_minutes);
    if (Number.isFinite(estimate) && estimate > 5) {
      warning("long-micro-lesson", `${id}: estimated at ${estimate} minutes; new spine lessons should be at most 5`, id);
    }
    if (lesson.body.trim() === "") warning("empty-lesson-body", `${id}: lesson has no authored body`, id);
  }

  const prerequisites = (lesson: ParsedLesson): string[] => {
    const value = lesson.frontmatter.prerequisites;
    return Array.isArray(value) ? value : typeof value === "string" && value !== "" ? [value] : [];
  };
  for (const lesson of lessons) {
    for (const prerequisite of prerequisites(lesson)) {
      if (!lessonById.has(prerequisite)) {
        error(
          "unknown-lesson-prerequisite",
          `${lesson.realization.lessonId}: unknown prerequisite '${prerequisite}'`,
        );
      }
    }
  }
  const visitingLessons = new Set<string>();
  const visitedLessons = new Set<string>();
  const visitLesson = (id: string): void => {
    if (visitedLessons.has(id)) return;
    if (visitingLessons.has(id)) {
      error("lesson-prerequisite-cycle", `lesson prerequisites contain a cycle through '${id}'`);
      return;
    }
    visitingLessons.add(id);
    const lesson = lessonById.get(id);
    if (lesson) for (const prerequisite of prerequisites(lesson)) visitLesson(prerequisite);
    visitingLessons.delete(id);
    visitedLessons.add(id);
  };
  for (const id of lessonById.keys()) visitLesson(id);

  const nodeIds = new Set<string>();
  const conceptOwner = new Map<string, string>();
  for (const node of spine.nodes) {
    if (nodeIds.has(node.id)) error("duplicate-spine-node", `spine node '${node.id}' is duplicated`);
    nodeIds.add(node.id);
    if (node.canDo.trim() === "") error("missing-can-do", `${node.id}: canDo is empty`);
    if (!spine.stages.includes(node.stage)) {
      error("unknown-spine-stage", `${node.id}: stage '${node.stage}' is not declared`);
    }
    for (const concept of node.concepts) {
      if (!hasOwn(taxonomy.concepts, concept)) {
        error("unknown-spine-concept", `${node.id}: concept '${concept}' is not canonical`);
      }
      const previous = conceptOwner.get(concept);
      if (previous) {
        error("duplicate-spine-concept", `concept '${concept}' occurs in ${previous} and ${node.id}`);
      } else {
        conceptOwner.set(concept, node.id);
      }
    }
  }

  for (const node of spine.nodes) {
    for (const prerequisite of node.prerequisites) {
      if (!nodeIds.has(prerequisite)) {
        error("unknown-spine-prerequisite", `${node.id}: unknown prerequisite '${prerequisite}'`);
      }
    }
  }

  const byId = new Map(spine.nodes.map((node) => [node.id, node]));
  const visiting = new Set<string>();
  const visited = new Set<string>();
  const visit = (id: string): void => {
    if (visited.has(id)) return;
    if (visiting.has(id)) {
      error("spine-cycle", `shared spine contains a cycle through '${id}'`);
      return;
    }
    visiting.add(id);
    for (const prerequisite of byId.get(id)?.prerequisites ?? []) visit(prerequisite);
    visiting.delete(id);
    visited.add(id);
  };
  for (const id of nodeIds) visit(id);

  const spineConcepts = new Set(conceptOwner.keys());
  for (const language of registry.languages) {
    const hasSpineRealization = lessons.some(
      (lesson) =>
        lesson.language === language.id &&
        CONTENT_TYPES.has(lesson.realization.type) &&
        spineConcepts.has(lesson.realization.concept),
    );
    if (!hasSpineRealization) {
      error("language-without-spine-realization", `${language.id}: no lesson realizes the shared spine`);
    }
  }

  return issues;
}
