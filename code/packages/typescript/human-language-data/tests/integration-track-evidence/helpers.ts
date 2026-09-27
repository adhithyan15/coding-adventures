import type { loadEverything } from "../../src/loader.js";
import type { buildCurriculumGapReport } from "../../src/report.js";

export type IntegrationTrackContext = ReturnType<typeof loadEverything> & {
  readonly curriculumGapReport: ReturnType<typeof buildCurriculumGapReport>;
};

export interface IntegrationTrackEvidence {
  readonly id: string;
  assert(context: IntegrationTrackContext): void;
}

export interface IntegrationTrackEvidenceModule {
  readonly integrationTrackEvidence: IntegrationTrackEvidence;
}
