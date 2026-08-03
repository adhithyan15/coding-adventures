import catalogDocument from "../../../../specs/fixtures/reference-validation-v1/catalog.json";

const TRACK_IDS = [
  "foundation",
  "spatial",
  "sequence",
  "attention",
  "representation",
  "structured",
  "deep-training",
  "autograd",
  "compilation",
] as const;
const TRACK_BY_ORDER: Readonly<Record<number, ReferenceTrackId>> = {
  3: "foundation", 4: "foundation",
  5: "spatial", 6: "spatial", 7: "spatial", 8: "spatial",
  9: "sequence", 10: "sequence", 11: "sequence",
  12: "attention", 13: "attention", 14: "attention", 15: "attention",
  16: "representation", 17: "representation", 18: "representation", 19: "representation",
  20: "structured", 21: "structured", 22: "structured",
  23: "deep-training", 24: "deep-training", 25: "deep-training",
  26: "autograd", 27: "autograd", 28: "autograd",
  29: "compilation", 30: "compilation", 31: "compilation", 32: "compilation",
};
const FAMILY_ID = /^[a-z][a-z0-9-]{0,79}$/;
const VALIDATOR_PATH = /^code\/scripts\/validate_[a-z0-9_]+_labs\.py$/;

export type ReferenceTrackId = typeof TRACK_IDS[number];

export interface ReferenceFamily {
  readonly order: number;
  readonly id: string;
  readonly title: string;
  readonly track: ReferenceTrackId;
  readonly spec: string;
  readonly fixtureRoot: string;
  readonly validator: string;
  readonly labCount: number;
  readonly oracle: string;
}

export interface ReferenceHandCheck {
  readonly equation: "absolute_error = |recomputed - stored|";
  readonly stored: number;
  readonly recomputed: number;
  readonly absoluteTolerance: number;
  readonly absoluteError: number;
  readonly passes: boolean;
}

export interface ReferenceCatalog {
  readonly id: "neural-reference-catalog";
  readonly title: string;
  readonly question: string;
  readonly command: "python code/scripts/validate_reference_fixture_catalog.py";
  readonly steps: readonly string[];
  readonly handCheck: ReferenceHandCheck;
  readonly families: readonly ReferenceFamily[];
}

export interface ReferenceCatalogTrace {
  readonly catalog: ReferenceCatalog;
  readonly family: ReferenceFamily;
  readonly familyCount: number;
  readonly labCount: number;
  readonly trackCount: number;
  readonly recomputedError: number;
  readonly passes: boolean;
}

function object(value: unknown, keys: readonly string[], context: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw new Error(`${context}: expected object`);
  const record = value as Record<string, unknown>;
  const actual = Object.keys(record).sort();
  const expected = [...keys].sort();
  if (actual.length !== expected.length || actual.some((key, index) => key !== expected[index])) throw new Error(`${context}: unexpected keys`);
  return record;
}

function text(value: unknown, context: string, maximum = 512): string {
  if (typeof value !== "string" || value.trim().length === 0 || value.length > maximum || /[\u0000-\u0008\u000b\u000c\u000e-\u001f]/.test(value)) throw new Error(`${context}: invalid text`);
  return value;
}

function number(value: unknown, context: string): number {
  if (typeof value !== "number" || !Number.isFinite(value)) throw new Error(`${context}: expected finite number`);
  return value;
}

function integer(value: unknown, minimum: number, maximum: number, context: string): number {
  const result = number(value, context);
  if (!Number.isInteger(result) || result < minimum || result > maximum) throw new Error(`${context}: invalid integer`);
  return result;
}

function parseHandCheck(value: unknown): ReferenceHandCheck {
  const check = object(value, ["equation", "stored", "recomputed", "absolute_tolerance", "absolute_error", "passes"], "hand check");
  if (check.equation !== "absolute_error = |recomputed - stored|") throw new Error("hand check: wrong equation");
  const stored = number(check.stored, "hand check stored");
  const recomputed = number(check.recomputed, "hand check recomputed");
  const absoluteTolerance = number(check.absolute_tolerance, "hand check tolerance");
  const absoluteError = number(check.absolute_error, "hand check error");
  const expectedRecomputed = 0.1 + 0.05;
  const expectedError = Math.abs(expectedRecomputed - stored);
  if (recomputed !== expectedRecomputed || absoluteError !== expectedError || absoluteTolerance <= 0) throw new Error("hand check: dishonest arithmetic");
  const passes = check.passes;
  if (typeof passes !== "boolean" || passes !== (expectedError <= absoluteTolerance)) throw new Error("hand check: dishonest pass result");
  return { equation: check.equation, stored, recomputed, absoluteTolerance, absoluteError, passes };
}

function parseFamily(value: unknown, index: number): ReferenceFamily {
  const family = object(value, ["order", "id", "title", "track", "spec", "fixture_root", "validator", "lab_count", "oracle"], `families[${index}]`);
  const order = integer(family.order, 3, 32, `families[${index}].order`);
  const id = text(family.id, `families[${index}].id`, 80);
  const title = text(family.title, `families[${index}].title`, 120);
  const track = family.track;
  const spec = text(family.spec, `families[${index}].spec`, 240);
  const fixtureRoot = text(family.fixture_root, `families[${index}].fixture_root`, 240);
  const validator = text(family.validator, `families[${index}].validator`, 240);
  const labCount = integer(family.lab_count, 1, 100, `families[${index}].lab_count`);
  const oracle = text(family.oracle, `families[${index}].oracle`, 80);
  if (!FAMILY_ID.test(id) || !TRACK_IDS.includes(track as ReferenceTrackId) || track !== TRACK_BY_ORDER[order]) throw new Error(`families[${index}]: invalid identity or track`);
  if (!spec.startsWith(`code/specs/NN${String(order).padStart(2, "0")}-`) || !spec.endsWith(".md")) throw new Error(`families[${index}]: spec order mismatch`);
  if (fixtureRoot !== `code/specs/fixtures/${id}-v1` || !VALIDATOR_PATH.test(validator)) throw new Error(`families[${index}]: invalid fixture or validator path`);
  return { order, id, title, track: track as ReferenceTrackId, spec, fixtureRoot, validator, labCount, oracle };
}

export function parseReferenceCatalog(value: unknown): ReferenceCatalog {
  const catalog = object(value, ["schema_version", "id", "title", "question", "protocol", "families"], "catalog");
  if (catalog.schema_version !== 1 || catalog.id !== "neural-reference-catalog") throw new Error("catalog: wrong identity");
  const protocol = object(catalog.protocol, ["command", "success_exit_code", "steps", "hand_check"], "protocol");
  if (protocol.command !== "python code/scripts/validate_reference_fixture_catalog.py" || protocol.success_exit_code !== 0) throw new Error("protocol: wrong command contract");
  if (!Array.isArray(protocol.steps) || protocol.steps.length !== 4) throw new Error("protocol: expected four steps");
  const steps = protocol.steps.map((step, index) => text(step, `protocol.steps[${index}]`, 200));
  if (!Array.isArray(catalog.families) || catalog.families.length !== 30) throw new Error("catalog: expected 30 families");
  const families = catalog.families.map(parseFamily);
  const ids = new Set(families.map((family) => family.id));
  const specs = new Set(families.map((family) => family.spec));
  const fixtures = new Set(families.map((family) => family.fixtureRoot));
  const validators = new Set(families.map((family) => family.validator));
  if (families.some((family, index) => family.order !== index + 3) || ids.size !== 30 || specs.size !== 30 || fixtures.size !== 30 || validators.size !== 30) throw new Error("catalog: incomplete or duplicate roster");
  if (families.reduce((sum, family) => sum + family.labCount, 0) !== 33) throw new Error("catalog: expected 33 lab documents");
  return {
    id: catalog.id,
    title: text(catalog.title, "catalog.title", 160),
    question: text(catalog.question, "catalog.question", 240),
    command: protocol.command,
    steps,
    handCheck: parseHandCheck(protocol.hand_check),
    families,
  };
}

export const referenceCatalog = parseReferenceCatalog(catalogDocument);

export function traceReferenceValidation(familyId = "neural-learning"): ReferenceCatalogTrace {
  const family = referenceCatalog.families.find((candidate) => candidate.id === familyId);
  if (!family) throw new Error(`unknown reference family: ${familyId}`);
  const recomputedError = Math.abs(referenceCatalog.handCheck.recomputed - referenceCatalog.handCheck.stored);
  return {
    catalog: referenceCatalog,
    family,
    familyCount: referenceCatalog.families.length,
    labCount: referenceCatalog.families.reduce((sum, candidate) => sum + candidate.labCount, 0),
    trackCount: new Set(referenceCatalog.families.map((candidate) => candidate.track)).size,
    recomputedError,
    passes: recomputedError <= referenceCatalog.handCheck.absoluteTolerance,
  };
}
