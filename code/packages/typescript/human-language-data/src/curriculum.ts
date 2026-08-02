// Pure validation for the structured HL04 language registry and shared spine.

import { CONTENT_TYPES, hasOwn } from "./constants.js";
import { DURATION_THRESHOLD_SECONDS, estimateLessonDuration } from "./report.js";
import type {
  BookCorpus,
  CurriculumSpine,
  Issue,
  LanguageRegistry,
  Taxonomy,
} from "./types.js";
import type { ParsedLesson } from "./parse.js";

const SCHEMA_V2_SKILLS = new Set(["listening", "speaking", "reading", "writing"]);
const SCHEMA_V2_MODES = new Set(["interpretive", "interpersonal", "presentational", "mediation"]);
const SCHEMA_V2_STRANDS = new Set(["meaning-input", "meaning-output", "language-focus", "fluency"]);
const KNOWLEDGE_ATOM = /^[A-Z]{2,}(?:-[A-Z0-9]+)+$/;

function stringValue(value: ParsedLesson["frontmatter"][string] | undefined): string {
  return typeof value === "string" ? value : "";
}

function stringList(value: ParsedLesson["frontmatter"][string] | undefined): string[] {
  if (Array.isArray(value)) return value;
  return typeof value === "string" && value.trim() !== "" ? [value] : [];
}

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
    const schemaVersion = stringValue(lesson.frontmatter.schema_version);
    if (schemaVersion !== "" && schemaVersion !== "1" && schemaVersion !== "2") {
      error("unknown-lesson-schema-version", `${id}: schema_version '${schemaVersion}' is not supported`);
    }
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

  // Schema v1 remains readable during migration. Version 2 is the strict HL04
  // contract used by both books and apps, so every declared field is executable.
  const schema2Lessons = lessons.filter(
    (lesson) => stringValue(lesson.frontmatter.schema_version) === "2",
  );
  const sequences = new Map<string, Map<number, string>>();
  const introducedByLesson = new Map<string, string[]>();
  const atomOwner = new Map<string, Map<string, string>>();
  for (const lesson of schema2Lessons) {
    const id = lesson.realization.lessonId;
    const fm = lesson.frontmatter;
    const spineNode = stringValue(fm.spine_node);
    if (!nodeIds.has(spineNode)) {
      error("schema-v2-unknown-spine-node", `${id}: spine_node '${spineNode}' is not canonical`);
    }

    const sequence = Number(stringValue(fm.sequence));
    if (!Number.isInteger(sequence) || sequence <= 0) {
      error("schema-v2-invalid-sequence", `${id}: sequence must be a positive integer`);
    } else {
      const languageSequences = sequences.get(lesson.language) ?? new Map<number, string>();
      const previous = languageSequences.get(sequence);
      if (previous) {
        error(
          "schema-v2-duplicate-sequence",
          `${id}: sequence ${sequence} is already used by ${previous} in ${lesson.language}`,
        );
      } else {
        languageSequences.set(sequence, id);
      }
      sequences.set(lesson.language, languageSequences);
    }

    const declaredSeconds = Number(stringValue(fm["duration.max_seconds"]));
    if (!Number.isInteger(declaredSeconds) || declaredSeconds < 1 || declaredSeconds >= DURATION_THRESHOLD_SECONDS) {
      error(
        "schema-v2-invalid-duration",
        `${id}: duration.max_seconds must be an integer from 1 through 299`,
      );
    }
    const estimate = estimateLessonDuration(lesson);
    if (estimate.effectiveSeconds >= DURATION_THRESHOLD_SECONDS) {
      error(
        "schema-v2-duration-budget",
        `${id}: effective duration is ${estimate.effectiveSeconds}s ` +
          `(declared ${estimate.declaredSeconds}s, computed ${estimate.computedSeconds}s)`,
      );
    }

    const requiredListFields = [
      "requires.knowledge",
      "introduces.knowledge",
      "practises.knowledge",
      "skills",
      "modes",
      "strands",
    ];
    for (const field of requiredListFields) {
      if (!hasOwn(fm, field) || !Array.isArray(fm[field])) {
        error("schema-v2-missing-list", `${id}: ${field} must be an authored list`);
      }
    }
    for (const field of ["register", "variety"]) {
      if (stringValue(fm[field]).trim() === "") {
        error("schema-v2-missing-metadata", `${id}: ${field} must be explicit`);
      }
    }
    for (const [field, allowed] of [
      ["skills", SCHEMA_V2_SKILLS],
      ["modes", SCHEMA_V2_MODES],
      ["strands", SCHEMA_V2_STRANDS],
    ] as const) {
      const values = stringList(fm[field]);
      if (values.length === 0) {
        error("schema-v2-empty-coverage", `${id}: ${field} must contain at least one value`);
      }
      for (const value of values) {
        if (!allowed.has(value)) {
          error("schema-v2-unknown-coverage", `${id}: ${field} contains unknown value '${value}'`);
        }
      }
    }

    if (lesson.blocks.length === 0) {
      error("schema-v2-empty-blocks", `${id}: body contains no typed level-two blocks`);
    } else {
      if (lesson.blocks[0]?.type !== "warmup") {
        error("schema-v2-first-block", `${id}: first body block must be warmup`);
      }
      if (lesson.blocks.at(-1)?.type !== "recall") {
        error("schema-v2-last-block", `${id}: last body block must be recall`);
      }
    }
    for (const block of lesson.blocks) {
      if (block.type === "unknown") {
        error("schema-v2-unknown-block", `${id}: heading '${block.title}' has no stable block type`);
      }
      if (block.markdown.trim() === "") {
        error("schema-v2-empty-block", `${id}: block '${block.title}' is empty`);
      }
    }

    for (const prerequisite of prerequisites(lesson)) {
      const resolved = lessonById.get(prerequisite);
      if (resolved && resolved.language !== lesson.language) {
        error(
          "schema-v2-cross-language-prerequisite",
          `${id}: prerequisite '${prerequisite}' belongs to ${resolved.language}`,
        );
      }
    }

    const introduced = stringList(fm["introduces.knowledge"]);
    introducedByLesson.set(id, introduced);
    const languageOwners = atomOwner.get(lesson.language) ?? new Map<string, string>();
    for (const field of ["requires.knowledge", "introduces.knowledge", "practises.knowledge"]) {
      for (const atom of stringList(fm[field])) {
        if (!KNOWLEDGE_ATOM.test(atom)) {
          error("schema-v2-invalid-knowledge-atom", `${id}: '${atom}' is not a stable knowledge atom id`);
        }
      }
    }
    for (const atom of introduced) {
      const previous = languageOwners.get(atom);
      if (previous) {
        error(
          "schema-v2-duplicate-knowledge-introduction",
          `${id}: '${atom}' was already introduced by ${previous}`,
        );
      } else {
        languageOwners.set(atom, id);
      }
    }
    atomOwner.set(lesson.language, languageOwners);
  }

  for (const lesson of schema2Lessons) {
    const sequence = Number(stringValue(lesson.frontmatter.sequence));
    for (const prerequisite of prerequisites(lesson)) {
      const resolved = lessonById.get(prerequisite);
      if (!resolved || stringValue(resolved.frontmatter.schema_version) !== "2") continue;
      const prerequisiteSequence = Number(stringValue(resolved.frontmatter.sequence));
      if (Number.isFinite(sequence) && Number.isFinite(prerequisiteSequence) && prerequisiteSequence >= sequence) {
        error(
          "schema-v2-prerequisite-order",
          `${lesson.realization.lessonId}: prerequisite '${prerequisite}' must have an earlier sequence`,
        );
      }
    }
  }

  const prerequisiteKnowledge = (lesson: ParsedLesson): Set<string> => {
    const known = new Set<string>();
    const walked = new Set<string>();
    const walk = (id: string): void => {
      if (walked.has(id)) return;
      walked.add(id);
      for (const atom of introducedByLesson.get(id) ?? []) known.add(atom);
      const resolved = lessonById.get(id);
      if (resolved && resolved.language === lesson.language) {
        for (const prerequisite of prerequisites(resolved)) walk(prerequisite);
      }
    };
    for (const prerequisite of prerequisites(lesson)) walk(prerequisite);
    return known;
  };
  for (const lesson of schema2Lessons) {
    const id = lesson.realization.lessonId;
    const known = prerequisiteKnowledge(lesson);
    for (const atom of stringList(lesson.frontmatter["requires.knowledge"])) {
      if (!known.has(atom)) {
        error(
          "schema-v2-knowledge-not-closed",
          `${id}: required atom '${atom}' is not introduced by a transitive prerequisite`,
        );
      }
    }
    const available = new Set([
      ...known,
      ...stringList(lesson.frontmatter["introduces.knowledge"]),
    ]);
    for (const atom of stringList(lesson.frontmatter["practises.knowledge"])) {
      if (!available.has(atom)) {
        error(
          "schema-v2-practice-before-introduction",
          `${id}: practised atom '${atom}' is not yet available`,
        );
      }
    }
  }

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
