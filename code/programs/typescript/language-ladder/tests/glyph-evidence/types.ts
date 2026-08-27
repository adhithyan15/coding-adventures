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

/** Shape required from every module discovered by the stable eager glob. */
export interface GlyphEvidenceModule {
  readonly default: readonly GlyphEvidence[];
}
