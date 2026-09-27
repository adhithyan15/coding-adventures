import { expect } from "vitest";
import { lessonsUpToLevel } from "../../src/levels.js";
import { buildCurriculumGapReport } from "../../src/report.js";
import type { IntegrationTrackEvidence } from "./helpers.js";

export const integrationTrackEvidence: IntegrationTrackEvidence = {
  id: "spanish",
  assert({ lessons, curricula, spine, registry, books }): void {
    const a1Ids = new Set(
      lessonsUpToLevel(lessons, curricula, spine, "A1")
        .filter((lesson) => lesson.language === "spanish")
        .map((lesson) => lesson.realization.lessonId),
    );
    const expectedA1 = [
      "ES-C401-seguir", "ES-C401-cambiar", "ES-C401-avenida", "ES-C401-recto",
      "ES-C401-route-recall-1", "ES-C401-route-recall-2",
      "ES-C406-cliente", "ES-C406-documento", "ES-C406-dato", "ES-C406-impreso",
      "ES-C406-rellenar", "ES-C406-documents-recall-1", "ES-C406-documents-recall-2",
      "ES-C407-medio", "ES-C407-debajo", "ES-C407-fuera", "ES-C407-lado",
      "ES-C407-location-recall-1", "ES-C407-location-recall-2",
    ];
    expect(expectedA1.filter((id) => !a1Ids.has(id))).toEqual([]);

    const report = buildCurriculumGapReport({ registry, lessons, books });
    const pilot = lessons.filter(
      (lesson) => lesson.language === "spanish" &&
        lesson.realization.chapter >= 1 && lesson.realization.chapter <= 4,
    );
    expect(pilot).toHaveLength(24);
    expect(pilot.every((lesson) => lesson.frontmatter.schema_version === "2")).toBe(true);
    expect(report.duration.violations.filter(
      (lesson) => lesson.language === "spanish" && (lesson.chapter ?? 0) <= 3,
    )).toEqual([]);
    expect(report.prerequisites.laterChapterWithoutPrerequisites.filter(
      (lesson) => lesson.language === "spanish" && (lesson.chapter ?? 0) <= 3,
    )).toEqual([]);
  },
};
