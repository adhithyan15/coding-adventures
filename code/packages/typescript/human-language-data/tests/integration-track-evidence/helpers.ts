import type { loadEverything } from "../../src/loader.js";

export type IntegrationTrackContext = ReturnType<typeof loadEverything>;

export interface IntegrationTrackEvidence {
  readonly id: string;
  assert(context: IntegrationTrackContext): void;
}

export interface IntegrationTrackEvidenceModule {
  readonly integrationTrackEvidence: IntegrationTrackEvidence;
}
