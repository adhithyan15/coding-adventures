/**
 * # One lane run, start to finish: prepare, execute, report
 *
 * {@link prepare} is where a run is *refused* (exit 2) -- the corpus loads, the
 * binding parses, and the two are checked against each other before any case
 * runs. {@link execute} is where the library is *judged* (exit 0 or 1).
 *
 * ## Binding totality (VER04i section 5, rule 1)
 *
 * - every op the corpus declares has a binding row, and vice versa;
 * - every error variant declared for an op's `Result` has a row, and every
 *   mapped variant is declared;
 * - an op with a width parameter says how it takes the width, and an op without
 *   one does not claim to.
 *
 * A binding that failed any of these would let some suite quietly skip an
 * operation or an error path.
 */

import { join, relative, resolve, sep } from "node:path";
import { parseBinding, resolveExports, Subject, type Binding, type Exports } from "./binding.ts";
import { loadCorpus, type Corpus } from "./corpus.ts";
import { BindingError, UsageError } from "./errors.ts";
import { FAULTS } from "./faults.ts";
import { readBytes } from "./jsonio.ts";
import { buildManifest } from "./manifest.ts";
import {
  controlPassed,
  runControl,
  runExamples,
  runSuite,
  shuffled,
  suitePassed,
  type ControlResult,
  type ExampleResult,
  type SuiteResult,
} from "./runner.ts";

export const ENV_FAULT = "WARRANT_EVIDENCE_FAULT";
export const ENV_FILTER = "WARRANT_EVIDENCE_FILTER";
export const ENV_OUT = "WARRANT_EVIDENCE_OUT";

/** Where everything is, and what to break. */
export interface RunConfig {
  readonly repoRoot: string;
  readonly corpusDir: string;
  readonly bindingPath: string;
  readonly packageDir: string;
  /** A portable fault name, or `null`. */
  readonly fault?: string | null;
  readonly faultOp?: string | null;
  readonly filter?: string | null;
}

/** A corpus and binding that agree, with the library imported. */
export interface Prepared {
  readonly config: RunConfig;
  readonly corpus: Corpus;
  readonly binding: Binding;
  readonly exports: Exports;
  readonly corpusRelative: string;
}

/** Everything a run learned. */
export interface Outcome {
  examples: ExampleResult[];
  suites: SuiteResult[];
  controls: ControlResult[];
  manifest: Record<string, unknown> | null;
}

/** True when every example, suite and control passed. */
export function outcomePassed(outcome: Outcome): boolean {
  return (
    outcome.examples.every((example) => example.failure === null) &&
    outcome.suites.every(suitePassed) &&
    outcome.controls.every(controlPassed)
  );
}

/**
 * Split `WARRANT_EVIDENCE_FAULT` (`<fault>` or `<fault>:<op>`).
 *
 * `none` and the empty string mean "no fault" -- a *clean* run.
 */
export function parseFaultSpec(text: string | null | undefined): [string | null, string | null] {
  if (text === null || text === undefined || text === "" || text === "none") return [null, null];
  const split = text.indexOf(":");
  const fault = split < 0 ? text : text.slice(0, split);
  const op = split < 0 ? "" : text.slice(split + 1);
  if (!FAULTS.has(fault)) throw new UsageError("unknown-fault", `unknown fault \`${fault}\``);
  return [fault, op === "" ? null : op];
}

/**
 * The refusal rules of VER04i 4.3.
 *
 * A filter on its own, or a filter beside `FAULT=none`, would produce a partial
 * *clean* run that looks like evidence. Refuse it.
 */
export function checkFilterRules(faultText: string | null | undefined, filterText: string | null | undefined): void {
  const noFault = faultText === null || faultText === undefined || faultText === "" || faultText === "none";
  if (filterText && noFault) {
    throw new UsageError("filter-without-fault", "WARRANT_EVIDENCE_FILTER requires a real WARRANT_EVIDENCE_FAULT");
  }
}

/** Load, cross-check and import; throw a `LaneError` to refuse. */
export async function prepare(config: RunConfig): Promise<Prepared> {
  const corpus = loadCorpus(config.repoRoot, config.corpusDir);
  const binding = parseBinding(readBytes(config.bindingPath, 64 * 1024));
  if (binding.module !== corpus.module) {
    throw new BindingError("module", "binding module differs from the corpus module");
  }
  if (binding.lane !== "typescript" || !binding.package.startsWith("typescript/")) {
    throw new BindingError("lane", "this runner only runs the typescript lane");
  }
  const sameOps =
    binding.ops.size === corpus.ops.size && [...binding.ops.keys()].every((name) => corpus.ops.has(name));
  if (!sameOps) throw new BindingError("totality", "binding ops differ from the corpus ops");
  for (const [name, signature] of corpus.ops) {
    const op = binding.ops.get(name) as NonNullable<ReturnType<Binding["ops"]["get"]>>;
    if (signature.widthParams.length > 0 !== (op.width !== "none")) {
      throw new BindingError("width", `op \`${name}\`: width binding disagrees with the spec`);
    }
    const declared =
      signature.ret.kind === "Result" ? (corpus.errors.get(signature.ret.error as string) ?? []) : [];
    const mapped = op.errors.map(([variant]) => variant);
    if (mapped.length !== declared.length || !declared.every((variant) => mapped.includes(variant))) {
      throw new BindingError("totality", `op \`${name}\`: error rows differ from the spec`);
    }
  }
  const exports = await resolveExports(binding, config.packageDir);
  const path = relative(resolve(config.repoRoot), resolve(config.corpusDir));
  if (path === "" || path.startsWith("..")) {
    throw new BindingError("corpus-path", "the corpus must lie inside the repository root");
  }
  return { config, corpus, binding, exports, corpusRelative: path.split(sep).join("/") };
}

function newSubject(prepared: Prepared): Subject {
  const { config, corpus, binding, exports } = prepared;
  const fault = config.fault ? (FAULTS.get(config.fault) ?? null) : null;
  return new Subject(binding, exports, corpus.ops, corpus.errors, fault, config.faultOp ?? null);
}

/** Run the corpus. `emit` receives each TAP-style output line. */
export function execute(prepared: Prepared, emit: (line: string) => void): Outcome {
  const { config, corpus, binding } = prepared;
  const faulted = Boolean(config.fault);
  const needle = config.filter ?? null;
  const selected = (id: string): boolean => needle === null || id.includes(needle);

  const allIds = [...corpus.examples.map((e) => e.id), ...corpus.suites.map((s) => s.id)];
  if (needle !== null && !allIds.some(selected)) {
    throw new UsageError("empty-filter", "the filter matched no example or suite");
  }

  const outcome: Outcome = { examples: [], suites: [], controls: [], manifest: null };
  const subject = newSubject(prepared);

  for (const example of runExamples(corpus, subject, selected)) {
    outcome.examples.push(example);
    emit(example.failure === null ? `ok ${example.id}` : `not ok ${example.id} ${example.failure}`);
  }

  const todo = corpus.suites.filter((suite) => selected(suite.id));
  for (const suite of shuffled(todo, binding.seed)) {
    const result = runSuite(corpus, suite, subject, faulted);
    outcome.suites.push(result);
    for (const [index, failure] of result.failedCases) emit(`not ok ${suite.id}@${index} ${failure}`);
    if (result.failures > result.failedCases.length) {
      emit(`# ${suite.id}: ${result.failures} failing cases in total`);
    }
    for (const failure of result.suiteFailures) emit(`not ok ${suite.id} ${failure}`);
    if (suitePassed(result)) {
      emit(`ok ${suite.id}`);
      emit(`# ${suite.id}: ${result.cases} cases, ${result.discarded} discarded, ${result.calls} calls`);
    } else if (result.failures > 0) {
      emit(`not ok ${suite.id} ${(result.failedCases[0] as [number, string])[1]}`);
    }
  }

  if (!faulted && needle === null) {
    for (let index = 0; index < corpus.controls.length; index += 1) {
      const control = runControl(corpus, binding, prepared.exports, index);
      outcome.controls.push(control);
      emit(controlPassed(control) ? `ok ${control.id}` : `not ok ${control.id} uncaught-fault`);
    }
    if (outcomePassed(outcome)) {
      outcome.manifest = buildManifest(
        corpus,
        binding,
        prepared.corpusRelative,
        resolve(join(config.packageDir, ...binding.source.split("/"))),
        rows(prepared, outcome),
      );
    }
  }
  return outcome;
}

function rows(prepared: Prepared, outcome: Outcome): Record<string, unknown>[] {
  const result: Record<string, unknown>[] = outcome.examples.map((example) => ({
    id: example.id,
    layer: "portable",
    kind: "example",
    op: example.op,
    cases: 1,
    verdict: "pass",
  }));
  const byId = new Map(prepared.corpus.suites.map((suite) => [suite.id, suite]));
  for (const suiteResult of outcome.suites) {
    const suite = byId.get(suiteResult.id) as NonNullable<ReturnType<typeof byId.get>>;
    result.push({
      id: suite.id,
      layer: "portable",
      kind: "exhaustive",
      op: suite.ops[0],
      claim: suite.claim,
      cases: suiteResult.cases,
      discarded: suiteResult.discarded,
      calls: suiteResult.calls,
      inputs_sha256: suiteResult.inputsSha256,
      expected_sha256: suiteResult.expectedSha256,
      domain: suite.domainText,
      verdict: "pass",
    });
  }
  for (const control of outcome.controls) {
    result.push({
      id: control.id,
      layer: "portable",
      kind: "negctl",
      fault: control.fault,
      op: control.op,
      target: control.targetKind,
      caught_by: [...control.caughtBy],
      verdict: "pass",
    });
  }
  return result;
}

/** `[fault, filter, out]` from the `WARRANT_EVIDENCE_*` variables. */
export function environment(env: NodeJS.ProcessEnv = process.env): [string | undefined, string | null, string | null] {
  return [env[ENV_FAULT], env[ENV_FILTER] || null, env[ENV_OUT] || null];
}
