/**
 * # Loading and validating a `warrant-corpus-v1` directory
 *
 * The corpus is produced by the Rust compiler (`warrant-grammar`) and is
 * *untrusted input* here. Loading it is therefore a long list of "this must be
 * exactly so", each failing closed:
 *
 * 1. the files are size-capped, regular, strictly parsed (no duplicate keys);
 * 2. every object has exactly the expected keys (a new field is a new profile,
 *    not something to skip);
 * 3. `spec_sha256` is re-derived from the `.warrant` file on disk
 *    (`StaleEvidence` caught at run time, VER04i 6 step 1);
 * 4. each negative control is **re-derived** from the fault registry, the op
 *    signatures and the suites that mention the op, and must equal the corpus's
 *    row -- a corpus cannot quietly omit a control or point it at the wrong
 *    claims;
 * 5. `cases.tfv` is cross-checked, record by record, against the examples in
 *    `corpus.json`: two encodings of the same facts must agree.
 *
 * Nothing here *executes* the library; that is `runner.ts`.
 */

import { join } from "node:path";
import { binderShape } from "./domains.ts";
import { CorpusError, need } from "./errors.ts";
import { FAULTS } from "./faults.ts";
import {
  decodeUtf8,
  isObject,
  MAX_CORPUS_BYTES,
  MAX_TEXT_BYTES,
  parseJson,
  readBytes,
  sha256Hex,
} from "./jsonio.ts";
import { parseTfv } from "./tfv.ts";
import {
  decodeJson,
  decodeTfv,
  parseType,
  valuesEqual,
  type ErrorTypes,
  type OpSig,
} from "./values.ts";

export const PROFILE = "warrant-corpus-v1";

/**
 * Work limits (VER04i section 3; the security review of WD-101). The corpus is
 * untrusted, so the Cartesian product of a suite's binder domains must equal
 * the cases it declares (admitted + discarded) and stay under a cap *before*
 * any case runs. The real ct-compare corpus needs 240,818 at most per suite and
 * 812,927 for one clean pass; with every negative control re-running its target
 * suites the total is about 2.1 million.
 */
export const LIMITS = {
  /** Tuples enumerated by one suite. */
  maxSuiteProduct: 1_000_000,
  /** All suites once, plus every control's re-runs. */
  maxTotalWork: 8_000_000,
  maxBinders: 8,
};

const HEX64 = /^[0-9a-f]{64}$/;
const IDENT = /^[a-z][a-z0-9_]*$/;
const ERROR_NAME = /^[A-Z][A-Za-z0-9_]*$/;
const TOP_KEYS = [
  "schema_version", "profile", "module", "spec", "spec_sha256", "generator",
  "types", "ops", "models", "examples", "suites", "faults", "controls",
]; // prettier-ignore
const SUITE_KEYS = [
  "id", "kind", "claim", "ops", "binders", "admit", "lhs", "rhs", "cases",
  "discarded", "expected_calls", "domain_text", "inputs_sha256",
  "expected_sha256", "result_type",
]; // prettier-ignore

/** One `example` from the spec: call `op` on `args`, expect `expected`. */
export interface Example {
  readonly id: string;
  readonly op: string;
  readonly args: readonly unknown[];
  readonly expected: unknown;
  readonly width: number | null;
}

/** One exhaustive suite: a claim over a finite, enumerable domain. */
export interface Suite {
  readonly id: string;
  readonly claim: string;
  readonly ops: readonly string[];
  /** `(name, domain descriptor)` per binder. */
  readonly binders: readonly (readonly [string, string])[];
  readonly binderTypes: readonly string[];
  readonly admit: unknown;
  readonly lhs: unknown;
  readonly rhs: unknown;
  readonly resultType: string;
  readonly cases: number;
  readonly discarded: number;
  readonly expectedCalls: number;
  readonly domainText: string;
  readonly inputsSha256: string;
  readonly expectedSha256: string;
}

/** One generated negative control (suite kind, op, fault). */
export interface Control {
  readonly id: string;
  readonly fault: string;
  readonly op: string;
  readonly targetKind: string;
  readonly requiredClaims: readonly string[];
}

/** A validated corpus, ready to run. */
export interface Corpus {
  readonly module: string;
  readonly spec: string;
  readonly specSha256: string;
  readonly corpusSha256: string;
  readonly casesSha256: string;
  readonly errors: ErrorTypes;
  readonly ops: ReadonlyMap<string, OpSig>;
  readonly models: Readonly<Record<string, unknown>>;
  readonly examples: readonly Example[];
  readonly suites: readonly Suite[];
  readonly controls: readonly Control[];
}

function exact(obj: unknown, keys: readonly string[], where: string): Record<string, unknown> {
  need(
    isObject(obj) && Object.keys(obj).length === keys.length && keys.every((key) => Object.hasOwn(obj, key)),
    "schema",
    `${where} has unexpected fields`,
  );
  return obj;
}

function text(value: unknown, where: string): string {
  need(typeof value === "string" && value !== "", "schema", `${where} must be a non-empty string`);
  return value;
}

function count(value: unknown, where: string): number {
  need(
    typeof value === "number" && Number.isSafeInteger(value) && value >= 0,
    "schema",
    `${where} must be a count`,
  );
  return value;
}

function digest(value: unknown, where: string): string {
  need(typeof value === "string" && HEX64.test(value), "schema", `${where} must be a lowercase SHA-256`);
  return value;
}

/** Resolve a repository-relative path, refusing anything that escapes. */
function repoPath(root: string, relative: string): string {
  need(
    !relative.includes("\\") &&
      !relative.startsWith("/") &&
      relative.split("/").every((part) => part !== "" && part !== "." && part !== ".."),
    "schema",
    "corpus path must be a normalised repository-relative path",
  );
  return join(root, ...relative.split("/"));
}

function signatures(rawOps: unknown): Map<string, OpSig> {
  need(isObject(rawOps) && Object.keys(rawOps).length > 0, "schema", "ops must be a non-empty object");
  const ops = new Map<string, OpSig>();
  for (const [name, row] of Object.entries(rawOps)) {
    need(IDENT.test(name), "schema", "bad op name");
    need(
      isObject(row) &&
        Object.hasOwn(row, "params") &&
        Object.hasOwn(row, "ret") &&
        Object.keys(row).every((key) => ["params", "ret", "width_params", "fresh"].includes(key)),
      "schema",
      "malformed op signature",
    );
    const fields = row as Record<string, unknown>;
    need(Array.isArray(fields["params"]), "schema", "op params must be a list");
    const params: (readonly [string, ReturnType<typeof parseType>])[] = [];
    for (const pair of fields["params"] as unknown[]) {
      need(
        Array.isArray(pair) && pair.length === 2 && typeof pair[0] === "string" && typeof pair[1] === "string",
        "schema",
        "malformed op parameter",
      );
      params.push([pair[0] as string, parseType(pair[1] as string)]);
    }
    const widths = fields["width_params"] ?? [];
    need(Array.isArray(widths) && widths.every((item) => typeof item === "string"), "schema", "bad widths");
    ops.set(name, {
      name,
      params,
      ret: parseType(text(fields["ret"], "op ret")),
      widthParams: widths as string[],
      fresh: fields["fresh"] === true,
    });
  }
  return ops;
}

function decodeExample(row: unknown, ops: ReadonlyMap<string, OpSig>, errors: ErrorTypes, module: string): Example {
  const fields = exact(row, ["args", "expected", "id", "op", "result_type", "width"], "example");
  const id = text(fields["id"], "example id");
  need(id.startsWith(`warrant/${module}/example/`), "schema", "example id is outside the module");
  const op = text(fields["op"], "example op");
  const sig = ops.get(op);
  need(sig !== undefined, "example", "example names an unknown op");
  const args = fields["args"];
  need(Array.isArray(args) && args.length === sig.params.length, "example", "example arity mismatch");
  need(fields["result_type"] === sig.ret.text, "example", "example result type disagrees");
  const width = fields["width"];
  if (sig.widthParams.length > 0) {
    need(typeof width === "number" && Number.isSafeInteger(width) && width >= 0, "example", "fixed-width example lacks a width");
  } else {
    need(width === null, "example", "unexpected example width");
  }
  const values = (args as unknown[]).map((raw, index) => {
    const ty = (sig.params[index] as readonly [string, ReturnType<typeof parseType>])[1];
    const value = decodeJson(raw, ty, errors);
    if (typeof ty.width === "string") {
      // `Bytes<N>`: the tag keeps the literal parameter name and the actual
      // length must equal the example's static width.
      need((value as Uint8Array).length === width, "example", "example buffer is not N bytes");
    }
    return value;
  });
  return {
    id,
    op,
    args: values,
    expected: decodeJson(fields["expected"], sig.ret, errors),
    width: typeof width === "number" ? width : null,
  };
}

function decodeSuite(row: unknown, ops: ReadonlyMap<string, OpSig>, module: string): Suite {
  const fields = exact(row, SUITE_KEYS, "suite");
  const claim = text(fields["claim"], "suite claim");
  const id = text(fields["id"], "suite id");
  need(
    id === `warrant/${module}/exhaustive/${claim}` && fields["kind"] === "exhaustive",
    "schema",
    "suite id does not follow the claim",
  );
  const suiteOps = fields["ops"];
  need(
    Array.isArray(suiteOps) &&
      suiteOps.length > 0 &&
      suiteOps.every((op) => typeof op === "string" && ops.has(op)),
    "schema",
    "suite names an unknown op",
  );
  const binders = fields["binders"];
  need(
    Array.isArray(binders) && binders.length > 0 && binders.length <= LIMITS.maxBinders,
    "schema",
    "suite needs between 1 and MAX_BINDERS binders",
  );
  const names: string[] = [];
  const descriptors: string[] = [];
  const types: string[] = [];
  let product = 1;
  for (const pair of binders as unknown[]) {
    need(Array.isArray(pair) && pair.length === 2 && typeof pair[0] === "string", "schema", "bad binder");
    const shape = binderShape((pair as unknown[])[1]);
    product *= shape.size;
    // The product is a double; refuse before it can stop being exact.
    need(product <= Number.MAX_SAFE_INTEGER, "work", "suite exceeds the per-suite work cap");
    need(!names.includes((pair as string[])[0] as string), "schema", "duplicate binder name");
    names.push((pair as string[])[0] as string);
    descriptors.push(shape.name);
    types.push(shape.type);
  }
  parseType(text(fields["result_type"], "suite result type"));
  const cases = count(fields["cases"], "cases");
  const discarded = count(fields["discarded"], "discarded");
  need(cases > 0, "schema", "a suite must admit at least one case (no vacuous suites)");
  need(
    product === cases + discarded,
    "work",
    "declared cases + discarded differ from the binder domains' product",
  );
  need(product <= LIMITS.maxSuiteProduct, "work", "suite exceeds the per-suite work cap");
  return {
    id,
    claim,
    ops: suiteOps as string[],
    binders: names.map((name, index) => [name, descriptors[index] as string] as const),
    binderTypes: types,
    admit: fields["admit"],
    lhs: fields["lhs"],
    rhs: fields["rhs"],
    resultType: fields["result_type"] as string,
    cases,
    discarded,
    expectedCalls: count(fields["expected_calls"], "expected_calls"),
    domainText: text(fields["domain_text"], "domain_text"),
    inputsSha256: digest(fields["inputs_sha256"], "inputs_sha256"),
    expectedSha256: digest(fields["expected_sha256"], "expected_sha256"),
  };
}

/** Re-derive the controls the corpus *must* contain (VER04i 4.1). */
function deriveControls(
  module: string,
  faults: unknown,
  suites: readonly Suite[],
  ops: ReadonlyMap<string, OpSig>,
): Control[] {
  need(Array.isArray(faults) && faults.length > 0, "schema", "faults must be a non-empty list");
  const derived: Control[] = [];
  const seen = new Set<string>();
  for (const row of faults as unknown[]) {
    const fields = exact(row, ["applies", "fault", "op"], "fault");
    const name = text(fields["fault"], "fault");
    const op = text(fields["op"], "fault op");
    const fault = FAULTS.get(name);
    need(fault !== undefined, "fault", "corpus names a fault the lane does not implement");
    const sig = ops.get(op);
    need(sig !== undefined && fault.applies(sig), "fault", "fault does not apply to op");
    need(fields["applies"] === "input" || fields["applies"] === "output", "fault", "bad fault stage");
    need(!seen.has(`${name}\u0000${op}`), "fault", "duplicate fault row");
    seen.add(`${name}\u0000${op}`);
    const required = suites.filter((suite) => suite.ops.includes(op)).map((suite) => suite.id);
    need(required.length > 0, "fault", "no claim can catch a registered fault");
    derived.push({
      id: `warrant/${module}/negctl/exhaustive/${op}/${name}`,
      fault: name,
      op,
      targetKind: "exhaustive",
      requiredClaims: required,
    });
  }
  // The compiler lists controls sorted by id; so must the corpus.
  return derived.sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));
}

/** Cross-check `cases.tfv` against `corpus.json`'s examples. */
function checkTfv(source: string, examples: readonly Example[], ops: ReadonlyMap<string, OpSig>, errors: ErrorTypes): void {
  const records = parseTfv(source);
  need(records.length === examples.length, "tfv", "TFV record count differs from the examples");
  records.forEach(([recordId, fields], index) => {
    const example = examples[index] as Example;
    need(recordId === example.id, "tfv", "TFV example order or identity differs");
    const sig = ops.get(example.op) as OpSig;
    const expectedKeys = new Set(["op", "expect", ...sig.params.map(([name]) => name)]);
    if (sig.widthParams.length > 0) expectedKeys.add("width");
    need(
      fields.size === expectedKeys.size && [...fields.keys()].every((key) => expectedKeys.has(key)),
      "tfv",
      "TFV fields disagree with the signature",
    );
    const op = fields.get("op");
    need(op !== undefined && op[0] === "text" && op[1] === example.op, "tfv", "TFV op differs from the example");
    sig.params.forEach(([name, ty], position) => {
      const value = decodeTfv(fields.get(name) as never, ty, errors);
      need(valuesEqual(value, example.args[position]), "tfv", "TFV argument differs");
    });
    if (sig.widthParams.length > 0) {
      const width = fields.get("width");
      need(width !== undefined && width[0] === "int" && Number(width[1]) === example.width, "tfv", "TFV width differs");
    }
    need(
      valuesEqual(decodeTfv(fields.get("expect") as never, sig.ret, errors), example.expected),
      "tfv",
      "TFV expected value differs",
    );
  });
}

/**
 * Read, validate and decode the corpus in `corpusDir`.
 *
 * Every malformation is a {@link CorpusError} (exit status 2). The explicit
 * checks cover the shapes we thought of; the guard below turns any shape we did
 * not (an `undefined` where an object belongs) into the same refusal instead of
 * a `TypeError` that would exit like a verdict failure.
 */
export function loadCorpus(repoRoot: string, corpusDir: string): Corpus {
  try {
    return loadCorpusUnguarded(repoRoot, corpusDir);
  } catch (error) {
    if (error instanceof TypeError || error instanceof RangeError || error instanceof SyntaxError) {
      throw new CorpusError("schema", "corpus has a malformed field type");
    }
    throw error;
  }
}

function loadCorpusUnguarded(repoRoot: string, corpusDir: string): Corpus {
  const corpusBytes = readBytes(join(corpusDir, "corpus.json"), MAX_CORPUS_BYTES);
  const casesBytes = readBytes(join(corpusDir, "cases.tfv"), MAX_TEXT_BYTES);
  const root = exact(parseJson(corpusBytes), TOP_KEYS, "corpus");
  need(root["schema_version"] === 1 && root["profile"] === PROFILE, "schema", "wrong profile");
  const module = text(root["module"], "module");
  need(IDENT.test(module), "schema", "module must be an identifier");

  const spec = text(root["spec"], "spec");
  const specBytes = readBytes(repoPath(repoRoot, spec), MAX_TEXT_BYTES);
  const specSha256 = digest(root["spec_sha256"], "spec_sha256");
  need(sha256Hex(specBytes) === specSha256, "stale-evidence", "spec_sha256 does not match the Warrant spec on disk");

  const rawTypes = root["types"];
  need(isObject(rawTypes), "schema", "types must be an object");
  const errors = new Map<string, string[]>();
  for (const [name, variants] of Object.entries(rawTypes)) {
    need(
      ERROR_NAME.test(name) && Array.isArray(variants) && variants.every((v) => typeof v === "string"),
      "schema",
      "malformed error type",
    );
    errors.set(name, [...(variants as string[])]);
  }
  const ops = signatures(root["ops"]);
  const rawModels = root["models"];
  need(isObject(rawModels), "schema", "models must be an object");

  const rawExamples = root["examples"];
  need(Array.isArray(rawExamples), "schema", "examples must be a list");
  const examples = (rawExamples as unknown[]).map((row) => decodeExample(row, ops, errors, module));
  need(new Set(examples.map((e) => e.id)).size === examples.length, "schema", "duplicate example id");

  const rawSuites = root["suites"];
  need(Array.isArray(rawSuites) && rawSuites.length > 0, "schema", "suites must be a list");
  const suites = (rawSuites as unknown[]).map((row) => decodeSuite(row, ops, module));
  need(new Set(suites.map((s) => s.id)).size === suites.length, "schema", "duplicate suite id");

  const controls = deriveControls(module, root["faults"], suites, ops);
  const rawControls = root["controls"];
  need(Array.isArray(rawControls), "schema", "controls must be a list");
  for (const row of rawControls as unknown[]) {
    need(
      isObject(row) &&
        Array.isArray(row["required_claims"]) &&
        row["required_claims"].every((claim) => typeof claim === "string"),
      "schema",
      "control required_claims must be a list of strings",
    );
  }
  const actual = (rawControls as unknown[]).map((row) => {
    const fields = exact(row, ["fault", "id", "op", "required_claims", "target_kind"], "control");
    return {
      id: fields["id"],
      fault: fields["fault"],
      op: fields["op"],
      targetKind: fields["target_kind"],
      requiredClaims: fields["required_claims"],
    };
  });
  need(
    actual.length === controls.length &&
      actual.every((row, index) => {
        const want = controls[index] as Control;
        const claims = row.requiredClaims as string[];
        return (
          row.id === want.id &&
          row.fault === want.fault &&
          row.op === want.op &&
          row.targetKind === want.targetKind &&
          claims.length === want.requiredClaims.length &&
          claims.every((claim, at) => claim === want.requiredClaims[at])
        );
      }),
    "control",
    "controls differ from those derived from the faults",
  );

  const byId = new Map(suites.map((suite) => [suite.id, suite.cases + suite.discarded]));
  let total = [...byId.values()].reduce((sum, size) => sum + size, 0);
  for (const control of controls) {
    for (const claim of control.requiredClaims) total += byId.get(claim) as number;
  }
  need(total <= LIMITS.maxTotalWork, "work", "total work (suites plus control re-runs) exceeds the work cap");
  checkTfv(decodeUtf8(casesBytes, "tfv"), examples, ops, errors);
  return {
    module,
    spec,
    specSha256,
    corpusSha256: sha256Hex(corpusBytes),
    casesSha256: sha256Hex(casesBytes),
    errors,
    ops,
    models: rawModels,
    examples,
    suites,
    controls,
  };
}

/**
 * `<fault>:<op>` for each control, read from `corpus.json` alone.
 *
 * Used by `list-faults`; it does not need the spec or the library.
 */
export function listControlRows(corpusDir: string): string[] {
  const root = parseJson(readBytes(join(corpusDir, "corpus.json"), MAX_CORPUS_BYTES));
  need(isObject(root) && Array.isArray(root["controls"]), "schema", "no controls");
  return (root["controls"] as unknown[]).map((control) => {
    need(isObject(control), "schema", "bad control");
    return `${String(control["fault"])}:${String(control["op"])}`;
  });
}
