import type { loadEverything } from "../../src/loader.js";
import type { buildCurriculumGapReport } from "../../src/report.js";

/**
 * The real curriculum plus ONE gap report built from `{ registry, lessons, books }`.
 * Every evidence module used to build that same report itself, all inside a single
 * test, which put three whole-corpus builds under one 30s budget. The report is
 * shared read-only input: modules filter it and never change it.
 */
export type IntegrationTrackContext = ReturnType<typeof loadEverything> & {
  readonly gapReport: ReturnType<typeof buildCurriculumGapReport>;
};

export interface IntegrationTrackEvidence {
  readonly id: string;
  assert(context: IntegrationTrackContext): void;
}

export interface IntegrationTrackEvidenceModule {
  readonly integrationTrackEvidence: IntegrationTrackEvidence;
}
