import { readFileSync } from "node:fs";
import { expect, it } from "vitest";
import {
  loadCurriculumSpine,
  loadTrackLanguageCurriculum,
  loadTrackLessons,
} from "../../../src/loader.js";
import { lessonsUpToLevel } from "../../../src/levels.js";

interface BoundaryLesson {
  id: string;
  introduces: string[];
  forms: string[];
}

interface A2Boundary {
  version: number;
  language: string;
  level: string;
  maxRegularChapter: number;
  firstExcludedChapter: number;
  grammarLessons: BoundaryLesson[];
  functionalLessons: BoundaryLesson[];
}

const trackRoot = new URL(
  "../../../../../../learning/human-languages/marathi/",
  import.meta.url,
);

it("owns Marathi A2 past and future grammar before the chapter-275 source gate", () => {
  const boundary = JSON.parse(readFileSync(
    new URL("assessment-boundaries/a2.json", trackRoot),
    "utf8",
  )) as A2Boundary;

  expect(Object.keys(boundary).sort()).toEqual([
    "firstExcludedChapter",
    "functionalLessons",
    "grammarLessons",
    "language",
    "level",
    "maxRegularChapter",
    "version",
  ]);
  expect(boundary).toMatchObject({
    version: 1,
    language: "marathi",
    level: "A2",
    maxRegularChapter: 275,
    firstExcludedChapter: 276,
  });
  expect(boundary.grammarLessons.map(({ id }) => id)).toEqual([
    "MR-C159-gelo",
    "MR-C159-tyane-kela",
    "MR-C159-hota",
    "MR-C160-jain",
    "MR-C160-janar-ahe",
  ]);
  expect(boundary.functionalLessons.map(({ id }) => id)).toEqual([
    "MR-C32-nako",
    "MR-C34-kaaran",
    "MR-C111-adhi",
    "MR-C111-nantar",
    "MR-C171-mag",
    "MR-C171-sevti",
  ]);

  const lessons = loadTrackLessons("marathi");
  const curriculum = loadTrackLanguageCurriculum("marathi");
  expect(curriculum, "Marathi curriculum must load").toBeDefined();
  const lessonById = new Map(lessons.map((lesson) => [
    lesson.realization.lessonId,
    lesson,
  ]));
  const requiredA2Extensions = curriculum?.extensions.filter(
    (extension) => extension.stage === "A2" && extension.kind === "required",
  ) ?? [];
  expect(requiredA2Extensions.length).toBeGreaterThan(0);
  for (const extension of requiredA2Extensions) {
    expect(extension.lessons.length, `${extension.id} must own lessons`).toBeGreaterThan(0);
    for (const lessonId of extension.lessons) {
      const extensionLesson = lessonById.get(lessonId);
      expect(extensionLesson, `${extension.id} must resolve ${lessonId}`).toBeDefined();
      expect(
        Number(extensionLesson?.frontmatter.chapter),
        `${extension.id} must not require A2 material after chapter 275`,
      ).toBeLessThanOrEqual(boundary.maxRegularChapter);
    }
  }
  expect(curriculum?.extensions.find(({ id }) => id === "MR-EXT-276-PAST")?.stage)
    .toBe("B1");
  expect(curriculum?.extensions.find(({ id }) => id === "MR-EXT-277-FUTURE")?.stage)
    .toBe("B1");
  const a2LessonIds = new Set(
    lessonsUpToLevel(
      lessons,
      curriculum ? [curriculum] : [],
      loadCurriculumSpine(),
      "A2",
    ).map((lesson) => lesson.realization.lessonId),
  );
  for (const declaration of boundary.grammarLessons) {
    expect(a2LessonIds, `${declaration.id} must remain in the A2 level slice`)
      .toContain(declaration.id);
  }
  for (const extensionId of ["MR-EXT-276-PAST", "MR-EXT-277-FUTURE"]) {
    const extension = curriculum?.extensions.find(({ id }) => id === extensionId);
    expect(extension, `${extensionId} must load`).toBeDefined();
    for (const lessonId of extension?.lessons ?? []) {
      expect(a2LessonIds, `${lessonId} must be excluded from the A2 level slice`)
        .not.toContain(lessonId);
    }
  }
  const exactPriorAtoms = new Map<string, { required: string[]; forbidden: string[] }>([
    ["MR-C159-tyane-kela", {
      required: ["MR-LEX-C89-FOOD89-02"],
      forbidden: ["MR-LEX-C89-FOOD89-01"],
    }],
    ["MR-C159-hota", {
      required: [
        "MR-LEX-MI",
        "MR-LEX-C96-NATURE96-05",
        "MR-LEX-MOTHA",
      ],
      forbidden: ["MR-LEX-C96-NATURE96-02"],
    }],
  ]);
  const declaredAtoms = new Set<string>();
  for (const declaration of [
    ...boundary.grammarLessons,
    ...boundary.functionalLessons,
  ]) {
    expect(Object.keys(declaration).sort()).toEqual(["forms", "id", "introduces"]);
    expect(declaration.introduces.length).toBeGreaterThan(0);
    expect(declaration.forms.length).toBeGreaterThan(0);

    const name = declaration.id.match(/^MR-C(\d{2,3})-[a-z0-9-]+$/u);
    expect(name, `${declaration.id} must be a regular numbered lesson`).not.toBeNull();
    const filenameChapter = Number(name?.[1]);
    expect(filenameChapter).toBeLessThanOrEqual(boundary.maxRegularChapter);
    expect(filenameChapter).toBeLessThan(boundary.firstExcludedChapter);

    const lesson = lessons.find(
      (candidate) => candidate.realization.lessonId === declaration.id,
    );
    expect(lesson, `${declaration.id} must load`).toBeDefined();
    expect(Number(lesson?.frontmatter.chapter)).toBe(filenameChapter);
    expect(Number(lesson?.frontmatter["duration.max_seconds"]))
      .toBeLessThanOrEqual(300);
    if (boundary.grammarLessons.includes(declaration)) {
      expect(lesson?.frontmatter.spine_node).toBe(
        filenameChapter === 159
          ? "SPINE-TALK-ABOUT-PAST"
          : "SPINE-TALK-ABOUT-FUTURE",
      );
    }
    expect(lesson?.frontmatter.introduces_idioms).toEqual([]);
    expect(lesson?.frontmatter.introduces_senses).toEqual([]);
    expect(lesson?.frontmatter.introduces_culture_claims).toEqual([]);
    expect(lesson?.frontmatter["introduces.knowledge"])
      .toEqual(declaration.introduces);

    const authored = readFileSync(
      new URL(`lessons/${declaration.id}.md`, trackRoot),
      "utf8",
    );
    if (boundary.grammarLessons.includes(declaration)) {
      expect(authored).toContain("<!-- hl-activity:");
    }
    const priorAtoms = exactPriorAtoms.get(declaration.id);
    for (const atom of priorAtoms?.required ?? []) {
      expect(lesson?.frontmatter["requires.knowledge"], `${declaration.id} requires ${atom}`)
        .toContain(atom);
    }
    for (const atom of priorAtoms?.forbidden ?? []) {
      expect(lesson?.frontmatter["requires.knowledge"], `${declaration.id} must not bind ${atom}`)
        .not.toContain(atom);
      expect(authored, `${declaration.id} must not assess ${atom}`).not.toContain(atom);
    }
    for (const form of declaration.forms) {
      expect(authored, `${declaration.id} must visibly teach ${form}`).toContain(form);
    }

    for (const atom of declaration.introduces) {
      expect(declaredAtoms.has(atom), `${atom} is declared twice`).toBe(false);
      declaredAtoms.add(atom);
      const owners = lessons.filter((candidate) =>
        candidate.frontmatter["introduces.knowledge"].includes(atom),
      );
      expect(owners.map((owner) => owner.realization.lessonId)).toEqual([
        declaration.id,
      ]);
    }
  }

  expect(declaredAtoms.size).toBe(16);
});
