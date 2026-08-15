// Pure validation for the structured HL04 language registry and shared spine.

import { CONTENT_TYPES, REALIZING_TYPES, hasOwn } from "./constants.js";
import { DURATION_THRESHOLD_SECONDS, estimateLessonDuration } from "./report.js";
import { activityContractErrors } from "./activity.js";
import type {
  BookCorpus,
  CurriculumSpine,
  Issue,
  LanguageCurriculum,
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
  curricula?: LanguageCurriculum[];
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

  if (input.curricula) {
    const curriculaByLanguage = new Map<string, LanguageCurriculum>();
    for (const curriculum of input.curricula) {
      if (curriculum.version !== 1) {
        error(
          "unsupported-language-curriculum-version",
          `${curriculum.language}: curriculum.json version must be 1, got '${curriculum.version}'`,
        );
      }
      if (curriculaByLanguage.has(curriculum.language)) {
        error(
          "duplicate-language-curriculum",
          `${curriculum.language}: more than one curriculum.json was loaded`,
        );
      } else {
        curriculaByLanguage.set(curriculum.language, curriculum);
      }
      if (!languageIds.has(curriculum.language)) {
        error(
          "unregistered-language-curriculum",
          `${curriculum.language}: curriculum.json has no language registry entry`,
        );
      }
    }
    for (const language of registry.languages) {
      if (!curriculaByLanguage.has(language.id)) {
        error("missing-language-curriculum", `${language.id}: curriculum.json is missing`);
      }
    }

    const allowedExtensionKinds = new Set([
      "required",
      "supporting",
      "reference",
      "not-applicable",
    ]);
    const lessonLanguage = new Map(
      lessons.map((lesson) => [lesson.realization.lessonId, lesson.language]),
    );

    for (const curriculum of input.curricula) {
      if (!languageIds.has(curriculum.language)) continue;
      const segmentById = new Map<string, LanguageCurriculum["path"][number]>();
      const lessonPlacement = new Map<string, { node: string; position: number }>();
      let position = 0;

      for (const segment of curriculum.path ?? []) {
        if (segmentById.has(segment.id)) {
          error(
            "duplicate-curriculum-segment",
            `${curriculum.language}: path segment '${segment.id}' occurs twice`,
          );
        } else {
          segmentById.set(segment.id, segment);
        }
        if (!nodeIds.has(segment.spine_node)) {
          error(
            "unknown-curriculum-spine-node",
            `${curriculum.language}: ${segment.id} uses unknown node '${segment.spine_node}'`,
          );
        }
        if (!Array.isArray(segment.lessons) || segment.lessons.length === 0) {
          error(
            "empty-curriculum-segment",
            `${curriculum.language}: ${segment.id} must contain at least one lesson`,
          );
        }
        for (const lessonId of segment.lessons ?? []) {
          const owner = lessonPlacement.get(lessonId);
          if (owner) {
            error(
              "duplicate-curriculum-lesson",
              `${curriculum.language}: ${lessonId} occurs in both ${owner.node} and ${segment.id}`,
            );
            continue;
          }
          if (!lessonById.has(lessonId)) {
            error(
              "unknown-curriculum-lesson",
              `${curriculum.language}: ${segment.id} names unknown lesson '${lessonId}'`,
            );
          } else if (lessonLanguage.get(lessonId) !== curriculum.language) {
            error(
              "cross-language-curriculum-lesson",
              `${curriculum.language}: ${segment.id} includes ${lessonId} from ${lessonLanguage.get(lessonId)}`,
            );
          }
          lessonPlacement.set(lessonId, { node: segment.spine_node, position });
          position += 1;
        }
      }

      for (const node of spine.nodes) {
        const realization = curriculum.spine?.[node.id];
        if (!realization) {
          error(
            "missing-curriculum-spine-node",
            `${curriculum.language}: spine map omits '${node.id}'`,
          );
          continue;
        }
        const actualSegments = (curriculum.path ?? [])
          .filter((segment) => segment.spine_node === node.id)
          .map((segment) => segment.id);
        if (JSON.stringify(realization.segments) !== JSON.stringify(actualSegments)) {
          error(
            "curriculum-segment-ledger-drift",
            `${curriculum.language}: ${node.id} segment ledger does not match the authored path`,
          );
        }
        const realizedConcepts = new Set(
          lessons
            .filter(
              (lesson) =>
                lesson.language === curriculum.language &&
                REALIZING_TYPES.has(lesson.realization.type),
            )
            .map((lesson) => lesson.realization.concept),
        );
        // A track may declare that one of its own tags satisfies a spine concept.
        // The spine names concepts language-neutrally (TENSE-BACKSHIFT); a track
        // names its lessons in its own terms (ES-REPORT-BACKSHIFT). Aliases let
        // the second answer the first WITHOUT retagging the lesson and throwing
        // away the specific name, which carries real information (HL-C169).
        // `hasOwn` cuts the prototype chain, and Array.isArray closes the
        // malformed-value case. Both matter because validateCurriculum is a
        // PUBLIC export: a consumer validating a curriculum it did not author
        // should get an Issue back, not an uncaught TypeError from `.some`.
        const aliasesRaw: unknown = curriculum.conceptAliases;
        const aliases: Record<string, unknown> =
          aliasesRaw && typeof aliasesRaw === "object"
            ? (aliasesRaw as Record<string, unknown>)
            : {};
        const aliasesFor = (concept: string): string[] => {
          if (!hasOwn(aliases, concept)) return [];
          const value = aliases[concept];
          return Array.isArray(value)
            ? value.filter((tag): tag is string => typeof tag === "string")
            : [];
        };
        const conceptIsRealized = (concept: string) =>
          realizedConcepts.has(concept) ||
          aliasesFor(concept).some((tag) => realizedConcepts.has(tag));
        const expectedOmits = node.concepts.filter((concept) => !conceptIsRealized(concept));
        if (JSON.stringify(realization.omits) !== JSON.stringify(expectedOmits)) {
          error(
            "curriculum-omission-ledger-drift",
            `${curriculum.language}: ${node.id} omissions must be ${expectedOmits.join(", ") || "empty"}`,
          );
        }
        const expectedRelocations = Object.fromEntries(
          lessons
            .filter(
              (lesson) =>
                lesson.language === curriculum.language &&
                CONTENT_TYPES.has(lesson.realization.type) &&
                node.concepts.includes(lesson.realization.concept) &&
                stringValue(lesson.frontmatter.spine_node) !== "" &&
                stringValue(lesson.frontmatter.spine_node) !== node.id &&
                nodeIds.has(stringValue(lesson.frontmatter.spine_node)),
            )
            .map((lesson) => [
              lesson.realization.concept,
              stringValue(lesson.frontmatter.spine_node),
            ]),
        );
        if (JSON.stringify(realization.relocates) !== JSON.stringify(expectedRelocations)) {
          error(
            "curriculum-relocation-ledger-drift",
            `${curriculum.language}: ${node.id} relocation ledger does not match lesson metadata`,
          );
        }
      }
      for (const nodeId of Object.keys(curriculum.spine ?? {})) {
        if (!nodeIds.has(nodeId)) {
          error(
            "unknown-curriculum-spine-ledger",
            `${curriculum.language}: spine map contains unknown node '${nodeId}'`,
          );
        }
      }

      for (const lesson of lessons.filter((item) => item.language === curriculum.language)) {
        const lessonId = lesson.realization.lessonId;
        const canonicalOwner = conceptOwner.get(lesson.realization.concept);
        const explicitNode = stringValue(lesson.frontmatter.spine_node);
        const placement = lessonPlacement.get(lessonId);
        if (canonicalOwner && CONTENT_TYPES.has(lesson.realization.type)) {
          const expectedNode = explicitNode !== "" && nodeIds.has(explicitNode)
            ? explicitNode
            : canonicalOwner;
          if (!placement) {
            error(
              "unmapped-shared-realization",
              `${curriculum.language}: ${lessonId} realizes ${canonicalOwner} but is absent from the local path`,
            );
          } else if (placement.node !== expectedNode) {
            error(
              "misplaced-shared-realization",
              `${curriculum.language}: ${lessonId} belongs to ${expectedNode}, not ${placement.node}`,
            );
          }
        }
        if (explicitNode !== "" && nodeIds.has(explicitNode)) {
          if (!placement) {
            error(
              "unmapped-schema-v2-lesson",
              `${curriculum.language}: ${lessonId} declares ${explicitNode} but is absent from the local path`,
            );
          } else if (placement.node !== explicitNode) {
            error(
              "misplaced-schema-v2-lesson",
              `${curriculum.language}: ${lessonId} declares ${explicitNode}, not ${placement.node}`,
            );
          }
        }
      }

      for (const [lessonId, placement] of lessonPlacement) {
        const lesson = lessonById.get(lessonId);
        if (!lesson) continue;
        for (const prerequisite of prerequisites(lesson)) {
          const required = lessonPlacement.get(prerequisite);
          if (!required) {
            error(
              "curriculum-prerequisite-omitted",
              `${curriculum.language}: ${lessonId} requires ${prerequisite}, which is absent from the local path`,
            );
          } else if (required.position >= placement.position) {
            error(
              "curriculum-prerequisite-order",
              `${curriculum.language}: ${prerequisite} must precede ${lessonId} in curriculum.json`,
            );
          }
        }
      }

      const extensionById = new Map<string, LanguageCurriculum["extensions"][number]>();
      for (const extension of curriculum.extensions ?? []) {
        if (extensionById.has(extension.id)) {
          error(
            "duplicate-curriculum-extension",
            `${curriculum.language}: extension '${extension.id}' occurs twice`,
          );
        } else {
          extensionById.set(extension.id, extension);
        }
        if (!allowedExtensionKinds.has(extension.kind)) {
          error(
            "unknown-curriculum-extension-kind",
            `${curriculum.language}: ${extension.id} has unknown kind '${extension.kind}'`,
          );
        }
        if (!spine.stages.includes(extension.stage)) {
          error(
            "unknown-curriculum-extension-stage",
            `${curriculum.language}: ${extension.id} uses undeclared stage '${extension.stage}'`,
          );
        }
        if (extension.canDo.trim() === "") {
          error(
            "missing-curriculum-extension-can-do",
            `${curriculum.language}: ${extension.id} has an empty canDo`,
          );
        }
        for (const prerequisite of extension.prerequisites ?? []) {
          if (!nodeIds.has(prerequisite) && !(curriculum.extensions ?? []).some((item) => item.id === prerequisite)) {
            error(
              "unknown-curriculum-extension-prerequisite",
              `${curriculum.language}: ${extension.id} requires unknown '${prerequisite}'`,
            );
          }
        }
      }

      const attachment = new Map<string, { segment: string; relation: string }>();
      const extensionLessonCount = new Map<string, number>();
      for (const segment of curriculum.path ?? []) {
        const segmentLessons = new Set(segment.lessons ?? []);
        for (const relation of ["before", "inline", "after"] as const) {
          for (const extensionId of segment[relation] ?? []) {
            const previous = attachment.get(extensionId);
            if (previous) {
              error(
                "duplicate-curriculum-extension-attachment",
                `${curriculum.language}: ${extensionId} is attached to both ${previous.segment} and ${segment.id}`,
              );
              continue;
            }
            attachment.set(extensionId, { segment: segment.id, relation });
            const extension = extensionById.get(extensionId);
            if (!extension) {
              error(
                "unknown-curriculum-extension",
                `${curriculum.language}: ${segment.id} attaches unknown extension '${extensionId}'`,
              );
              continue;
            }
            for (const lessonId of extension.lessons ?? []) {
              if (!segmentLessons.has(lessonId)) {
                error(
                  "curriculum-extension-outside-segment",
                  `${curriculum.language}: ${extensionId} uses ${lessonId} outside ${segment.id}`,
                );
              }
              extensionLessonCount.set(lessonId, (extensionLessonCount.get(lessonId) ?? 0) + 1);
            }
          }
        }
      }
      for (const extensionId of extensionById.keys()) {
        if (!attachment.has(extensionId)) {
          error(
            "unattached-curriculum-extension",
            `${curriculum.language}: ${extensionId} is not attached to a path segment`,
          );
        }
      }
      for (const [lessonId, placement] of lessonPlacement) {
        const lesson = lessonById.get(lessonId);
        if (!lesson) continue;
        const isSharedContent =
          CONTENT_TYPES.has(lesson.realization.type) &&
          conceptOwner.get(lesson.realization.concept) === placement.node;
        const count = extensionLessonCount.get(lessonId) ?? 0;
        if (!isSharedContent && count === 0) {
          error(
            "unclassified-curriculum-extension-lesson",
            `${curriculum.language}: ${lessonId} is local support but belongs to no extension node`,
          );
        } else if (!isSharedContent && count > 1) {
          error(
            "duplicate-curriculum-extension-lesson",
            `${curriculum.language}: ${lessonId} belongs to ${count} extension nodes`,
          );
        }
      }
    }
  }

  // Schema v1 remains readable during migration. Version 2 is the strict HL04
  // contract used by both books and apps, so every declared field is executable.
  const schema2Lessons = lessons.filter(
    (lesson) => stringValue(lesson.frontmatter.schema_version) === "2",
  );
  for (const lesson of lessons) {
    if (stringValue(lesson.frontmatter.schema_version) === "2") continue;
    if (lesson.blocks.some((block) =>
      (block.activities?.length ?? 0) > 0 || (block.activityDirectiveErrors?.length ?? 0) > 0
    )) {
      error(
        "activity-requires-schema-v2",
        `${lesson.realization.lessonId}: hl-activity directives require schema_version 2`,
      );
    }
  }
  const sequences = new Map<string, Map<number, string>>();
  const introducedByLesson = new Map<string, string[]>();
  const atomOwner = new Map<string, Map<string, string>>();
  const activityOwner = new Map<string, string>();
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
      for (const directiveError of block.activityDirectiveErrors ?? []) {
        error(
          "schema-v2-invalid-activity-directive",
          `${id}: block '${block.title}' ${directiveError}`,
        );
      }
      for (const activity of block.activities ?? []) {
        if (!activity.id.startsWith(`${id}-`)) {
          error(
            "schema-v2-activity-id-prefix",
            `${id}: activity '${activity.id}' must begin with '${id}-'`,
          );
        }
        const previousOwner = activityOwner.get(activity.id);
        if (previousOwner) {
          error(
            "schema-v2-duplicate-activity-id",
            `${id}: activity '${activity.id}' is already authored by ${previousOwner}`,
          );
        } else {
          activityOwner.set(activity.id, id);
        }
        for (const contractError of activityContractErrors(activity)) {
          error(
            "schema-v2-invalid-activity-contract",
            `${id}: activity '${activity.id}' ${contractError}`,
          );
        }
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

    const lessonIntroduces = new Set(stringList(lesson.frontmatter["introduces.knowledge"]));
    const lessonPractises = new Set(stringList(lesson.frontmatter["practises.knowledge"]));
    const blockFrontier = new Set(known);
    const introducedInBlocks = new Set<string>();
    const assessedInBlocks = new Set<string>();
    for (const block of lesson.blocks) {
      if (block.knowledgeDirectiveError) {
        error(
          "schema-v2-invalid-block-knowledge",
          `${id}: block '${block.title}' ${block.knowledgeDirectiveError}`,
        );
        continue;
      }
      if (!block.knowledge) {
        error(
          "schema-v2-missing-block-knowledge",
          `${id}: block '${block.title}' must declare block-boundary knowledge`,
        );
        continue;
      }
      for (const atom of [...block.knowledge.introduces, ...block.knowledge.assesses]) {
        if (!KNOWLEDGE_ATOM.test(atom)) {
          error(
            "schema-v2-invalid-block-knowledge-atom",
            `${id}: block '${block.title}' contains invalid knowledge atom '${atom}'`,
          );
        }
      }
      for (const atom of block.knowledge.assesses) {
        assessedInBlocks.add(atom);
        if (!lessonPractises.has(atom)) {
          error(
            "schema-v2-block-undeclared-assessment",
            `${id}: block '${block.title}' assesses '${atom}' without declaring it in practises.knowledge`,
          );
        }
        if (!blockFrontier.has(atom)) {
          error(
            "schema-v2-block-knowledge-not-closed",
            `${id}: block '${block.title}' assesses '${atom}' before it is available`,
          );
        }
      }
      const blockAssessments = new Set(block.knowledge.assesses);
      for (const activity of block.activities ?? []) {
        for (const atom of activity.assesses) {
          if (!KNOWLEDGE_ATOM.test(atom)) {
            error(
              "schema-v2-invalid-activity-knowledge-atom",
              `${id}: activity '${activity.id}' contains invalid knowledge atom '${atom}'`,
            );
          }
          if (!blockAssessments.has(atom)) {
            error(
              "schema-v2-activity-assessment-outside-block",
              `${id}: activity '${activity.id}' assesses '${atom}' outside block '${block.title}'`,
            );
          }
        }
      }
      if (
        (block.type === "guided-production" || block.type === "recall") &&
        block.knowledge.assesses.length === 0
      ) {
        error(
          "schema-v2-empty-block-assessment",
          `${id}: ${block.type} block '${block.title}' must declare assessed knowledge`,
        );
      }
      for (const atom of block.knowledge.introduces) {
        if (!lessonIntroduces.has(atom)) {
          error(
            "schema-v2-block-undeclared-introduction",
            `${id}: block '${block.title}' introduces '${atom}' without declaring it in introduces.knowledge`,
          );
        }
        if (introducedInBlocks.has(atom)) {
          error(
            "schema-v2-duplicate-block-introduction",
            `${id}: '${atom}' is introduced by more than one body block`,
          );
        }
        introducedInBlocks.add(atom);
        blockFrontier.add(atom);
      }
    }
    for (const atom of lessonIntroduces) {
      if (!introducedInBlocks.has(atom)) {
        error(
          "schema-v2-block-introduction-missing",
          `${id}: introduced atom '${atom}' has no body-block introduction`,
        );
      }
    }
    for (const atom of lessonPractises) {
      if (!assessedInBlocks.has(atom)) {
        error(
          "schema-v2-block-assessment-missing",
          `${id}: practised atom '${atom}' is not assessed by any body block`,
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
