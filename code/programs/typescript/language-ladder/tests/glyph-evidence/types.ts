/**
 * The shared, once-loaded inputs available to independently owned evidence.
 *
 * Keeping this context explicit is more than type plumbing: evidence modules
 * are not Vitest roots and cannot accidentally create their own corpus setup.
 */
export interface GlyphEvidenceContext {
  readonly SCRIPTS: typeof import("@coding-adventures/script-ductus").SCRIPTS;
  readonly isSyllabary: typeof import("../../src/syllabary").isSyllabary;
  readonly buildSyllableMatrix: typeof import("../../src/matrix").buildSyllableMatrix;
}

/** One deterministic test case owned by a script or shared inventory. */
export interface GlyphEvidence {
  readonly suite: string;
  readonly suiteOrder: number;
  readonly caseOrder: number;
  readonly name: string;
  readonly verify: (context: GlyphEvidenceContext) => void;
}

/** Evidence annotated with its discovered module for deterministic tie-breaking. */
export interface LocatedGlyphEvidence extends GlyphEvidence {
  readonly modulePath: string;
}

/** Reject metadata that could make JavaScript's numeric comparator fail open. */
export function assertValidGlyphEvidenceRanks(entry: LocatedGlyphEvidence): void {
  for (const [label, rank] of [
    ["suiteOrder", entry.suiteOrder],
    ["caseOrder", entry.caseOrder],
  ] as const) {
    if (!Number.isSafeInteger(rank) || rank <= 0) {
      throw new Error(
        `glyph evidence '${entry.name}' in ${entry.modulePath} has invalid ${label} ${String(rank)}; expected a positive safe integer`,
      );
    }
  }
}

/** Preserve historical order while allowing parallel additions to share a rank. */
export function compareGlyphEvidence(
  left: LocatedGlyphEvidence,
  right: LocatedGlyphEvidence,
): number {
  return (
    left.suiteOrder - right.suiteOrder ||
    (left.suite < right.suite ? -1 : left.suite > right.suite ? 1 : 0) ||
    left.caseOrder - right.caseOrder ||
    (left.modulePath < right.modulePath ? -1 : left.modulePath > right.modulePath ? 1 : 0) ||
    (left.name < right.name ? -1 : left.name > right.name ? 1 : 0)
  );
}

/** Shape required from every module discovered by the stable eager glob. */
export interface GlyphEvidenceModule {
  readonly default: readonly GlyphEvidence[];
}
