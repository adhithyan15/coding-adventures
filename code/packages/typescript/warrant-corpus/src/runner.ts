/**
 * # Executing the corpus against the real library
 *
 * A run has three movements, in this order (VER04i section 6):
 *
 * 1. **Examples** -- the spec's hand-written `example` lines, one case each.
 * 2. **Exhaustive suites** -- every claim over its whole finite domain. The
 *    inputs are *enumerated here*, the expected answers come from interpreting
 *    the corpus's model IR here, and both streams are hashed. The hashes must
 *    equal the ones the Rust compiler computed. That is the cross-check that
 *    makes in-lane enumeration safe: a lane that skips a case, reorders the
 *    domain, or evaluates a model differently produces a different digest.
 * 3. **Negative controls** -- the clean pass proved the library agrees with the
 *    model; this proves the *runner* can disagree. One op at a time is broken
 *    (`faults.ts`) and the suites that mention it must fail.
 *
 * ## What a suite checks, per case
 *
 *     actual   = subject side   (calls the REAL export through the binding)
 *     expected = reference side (pure model, interpreted from the corpus IR)
 *     ok  iff  actual is exactly the declared type  and  actual equals expected
 *
 * The "exactly the declared type" half happens inside `Subject.call`, at the
 * boundary, so a wrong type is reported where it occurred (`type-mismatch`).
 * The expected value never depends on the library, so a wrong library cannot
 * change the oracle (VER04i 2.5). A failure prints the suite id and case
 * *index* only -- never the input bytes (VER00 section 7.3); the index plus the
 * corpus reproduces the case in any lane.
 */

import { Subject, SubjectFailure, type Binding, type Exports } from "./binding.ts";
import { LIMITS, type Corpus, type Suite } from "./corpus.ts";
import { binderShape, domainValues, product } from "./domains.ts";
import {
  CALL_COUNT_MISMATCH,
  COUNT_MISMATCH,
  MISSING_ERROR,
  RESULT_MISMATCH,
  STREAM_MISMATCH,
  need,
} from "./errors.ts";
import { FAULTS } from "./faults.ts";
import { Compiler } from "./ir.ts";
import { StreamHasher, parseType, streamPrefix, valuesEqual } from "./values.ts";

/** Failing cases named per suite (the rest are counted). */
export const MAX_REPORTED_CASES = 10;

/** The outcome of one exhaustive suite. */
export interface SuiteResult {
  id: string;
  cases: number;
  discarded: number;
  calls: number;
  inputsSha256: string;
  expectedSha256: string;
  failures: number;
  failedCases: [number, string][];
  suiteFailures: string[];
}

/** True when no case failed and every count and digest matched. */
export function suitePassed(result: SuiteResult): boolean {
  return result.failures === 0 && result.suiteFailures.length === 0;
}

function containsSubject(node: unknown): boolean {
  if (!Array.isArray(node)) return false;
  if (node.length > 0 && node[0] === "subject_op") return true;
  return node.some(containsSubject);
}

/** The failure id for one case, or `null` when it passes. */
export function classify(actual: unknown, expected: unknown): string | null {
  if (valuesEqual(actual, expected)) return null;
  const expectingError = Array.isArray(expected) && expected[0] === "err";
  const returnedError = Array.isArray(actual) && actual[0] === "err";
  return expectingError && !returnedError ? MISSING_ERROR : RESULT_MISMATCH;
}

/** Run one exhaustive suite; see the module comment for the checks. */
export function runSuite(corpus: Corpus, suite: Suite, subject: Subject, failFast: boolean): SuiteResult {
  const names = suite.binders.map(([name]) => name);
  // Refuse before running anything: the work this suite will do must be the
  // work it declared (the corpus is untrusted; see LIMITS.maxSuiteProduct).
  // The shapes are counted without building the domains.
  let size = 1;
  for (const [, descriptor] of suite.binders) {
    size *= binderShape({ domain: descriptor }).size;
    need(size <= Number.MAX_SAFE_INTEGER, "work", "suite exceeds the per-suite work cap");
  }
  need(
    size === suite.cases + suite.discarded && size <= LIMITS.maxSuiteProduct,
    "work",
    "declared cases + discarded differ from the binder domains' product",
  );
  const domains = suite.binders.map(([, descriptor]) => domainValues(descriptor).values);
  const binderTypes = suite.binderTypes.map(parseType);
  const resultType = parseType(suite.resultType);

  const pure = new Compiler(corpus.models, null, corpus.ops);
  const hooked = new Compiler(corpus.models, (op, args, width) => subject.call(op, args, width), corpus.ops);
  const admit = pure.compile(suite.admit, names);
  const lhsSubject = containsSubject(suite.lhs);
  const rhsSubject = containsSubject(suite.rhs);
  need(lhsSubject !== rhsSubject, "unsupported-ir", "exactly one side of a claim may call the subject");
  const [subjectSide, referenceSide] = lhsSubject ? [suite.lhs, suite.rhs] : [suite.rhs, suite.lhs];
  const runSubject = hooked.compile(subjectSide, names);
  const runReference = pure.compile(referenceSide, names);

  const prefix = streamPrefix(suite.id, suite.binderTypes, suite.resultType);
  const inputsHash = new StreamHasher(prefix);
  const expectedHash = new StreamHasher(prefix);

  const result: SuiteResult = {
    id: suite.id,
    cases: 0,
    discarded: 0,
    calls: 0,
    inputsSha256: "",
    expectedSha256: "",
    failures: 0,
    failedCases: [],
    suiteFailures: [],
  };
  const callsBefore = subject.totalCalls(suite.ops);
  let index = 0;
  let stopped = false;
  product(domains, (env) => {
    if (admit(env) !== true) {
      result.discarded += 1;
      return true;
    }
    const expected = runReference(env);
    env.forEach((value, position) => inputsHash.write(value, binderTypes[position] as never));
    expectedHash.write(expected, resultType);
    let failure: string | null;
    try {
      failure = classify(runSubject(env), expected);
    } catch (error) {
      if (!(error instanceof SubjectFailure)) throw error;
      failure = error.failureId;
    }
    index += 1;
    if (failure === null) return true;
    result.failures += 1;
    if (result.failedCases.length < MAX_REPORTED_CASES) result.failedCases.push([index - 1, failure]);
    if (failFast) {
      stopped = true;
      return false;
    }
    return true;
  });

  result.cases = index;
  result.calls = subject.totalCalls(suite.ops) - callsBefore;
  if (stopped) {
    // A fail-fast run never saw the whole enumeration, so its counts and
    // digests are not comparable; the failing cases already decide it.
    return result;
  }
  result.inputsSha256 = inputsHash.hexdigest();
  result.expectedSha256 = expectedHash.hexdigest();
  if (result.cases !== suite.cases || result.discarded !== suite.discarded) {
    result.suiteFailures.push(COUNT_MISMATCH);
  }
  if (result.inputsSha256 !== suite.inputsSha256 || result.expectedSha256 !== suite.expectedSha256) {
    result.suiteFailures.push(STREAM_MISMATCH);
  }
  if (result.calls !== suite.expectedCalls) result.suiteFailures.push(CALL_COUNT_MISMATCH);
  return result;
}

/** The outcome of one example case. */
export interface ExampleResult {
  id: string;
  op: string;
  failure: string | null;
}

/**
 * Run each example once.
 *
 * `cases.tfv` was cross-checked equal to these decoded examples when the corpus
 * loaded (`loadCorpus`), so executing the decoded form executes the TFV records.
 */
export function runExamples(corpus: Corpus, subject: Subject, only: (id: string) => boolean): ExampleResult[] {
  const results: ExampleResult[] = [];
  for (const example of corpus.examples) {
    if (!only(example.id)) continue;
    let failure: string | null;
    try {
      failure = classify(subject.call(example.op, [...example.args], example.width ?? undefined), example.expected);
    } catch (error) {
      if (!(error instanceof SubjectFailure)) throw error;
      failure = error.failureId;
    }
    results.push({ id: example.id, op: example.op, failure });
  }
  return results;
}

/** The outcome of one negative control. */
export interface ControlResult {
  id: string;
  fault: string;
  op: string;
  targetKind: string;
  caughtBy: string[];
  reachedExport: boolean;
}

/** A control passes iff a required claim failed *and* the export was called. */
export function controlPassed(result: ControlResult): boolean {
  return result.caughtBy.length > 0 && result.reachedExport;
}

/** Break one op, rerun exactly the control's target suites, record who noticed. */
export function runControl(corpus: Corpus, binding: Binding, exports: Exports, controlIndex: number): ControlResult {
  const control = corpus.controls[controlIndex] as Corpus["controls"][number];
  const subject = new Subject(binding, exports, corpus.ops, corpus.errors, FAULTS.get(control.fault) ?? null, control.op);
  const byId = new Map(corpus.suites.map((suite) => [suite.id, suite]));
  const caught: string[] = [];
  for (const suiteId of control.requiredClaims) {
    if (runSuite(corpus, byId.get(suiteId) as Suite, subject, true).failures > 0) caught.push(suiteId);
  }
  return {
    id: control.id,
    fault: control.fault,
    op: control.op,
    targetKind: control.targetKind,
    caughtBy: caught,
    reachedExport: subject.callsTo(control.op) > 0,
  };
}

const MASK = (1n << 64n) - 1n;

/**
 * Order suites by a seeded Fisher-Yates shuffle (SplitMix64).
 *
 * Suite order is the only thing the evidence seed controls (VER04i 6): case
 * order is canonical. Shuffling stops a suite from silently depending on state
 * left behind by the one before it. The seed is a `bigint` and every step is
 * masked to 64 bits, so the order is the same one the Python lane draws.
 */
export function shuffled<T>(items: readonly T[], seed: bigint): T[] {
  let state = seed;
  const nextU64 = (): bigint => {
    state = (state + 0x9e3779b97f4a7c15n) & MASK;
    let z = state;
    z = ((z ^ (z >> 30n)) * 0xbf58476d1ce4e5b9n) & MASK;
    z = ((z ^ (z >> 27n)) * 0x94d049bb133111ebn) & MASK;
    return z ^ (z >> 31n);
  };
  const order = [...items];
  for (let position = order.length - 1; position > 0; position -= 1) {
    const other = Number(nextU64() % BigInt(position + 1));
    [order[position], order[other]] = [order[other] as T, order[position] as T];
  }
  return order;
}
