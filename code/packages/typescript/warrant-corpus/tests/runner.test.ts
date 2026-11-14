import { describe, expect, it } from "vitest";
import { Subject, parseBinding, resolveExports } from "../src/binding.ts";
import type { Corpus, Suite } from "../src/corpus.ts";
import { CorpusError } from "../src/errors.ts";
import { FAULTS } from "../src/faults.ts";
import {
  MAX_REPORTED_CASES,
  classify,
  controlPassed,
  runControl,
  runExamples,
  runSuite,
  shuffled,
  suitePassed,
} from "../src/runner.ts";
import { BINDING_TEXT, newLibrary, realCorpus } from "./helpers.ts";

const corpus = realCorpus();
const binding = parseBinding(new TextEncoder().encode(BINDING_TEXT));

async function exportsFor(overrides: Record<string, string>) {
  const lib = newLibrary(overrides);
  return resolveExports(binding, lib.packageDir);
}
async function subjectFor(overrides: Record<string, string> = {}, fault: string | null = null, faultOp: string | null = null): Promise<Subject> {
  return new Subject(binding, await exportsFor(overrides), corpus.ops, corpus.errors, fault === null ? null : (FAULTS.get(fault) ?? null), faultOp);
}
const suiteNamed = (claim: string): Suite => {
  const found = corpus.suites.find((suite) => suite.claim === claim);
  if (!found) throw new Error(`no suite ${claim}`);
  return found;
};
const W2 = suiteNamed("eq_fixed_agrees_w2");
const W1 = suiteNamed("eq_fixed_agrees_w1");
const withSuite = (suite: Suite, change: Partial<Suite>): Suite => ({ ...suite, ...change });

describe("runSuite", () => {
  it("a correct library matches every digest, count and call count", async () => {
    const subject = await subjectFor();
    const result = runSuite(corpus, W2, subject, false);
    expect(suitePassed(result)).toBe(true);
    expect([result.cases, result.discarded, result.calls]).toEqual([256, 0, 256]);
    expect(result.inputsSha256).toBe(W2.inputsSha256);
    expect(result.expectedSha256).toBe(W2.expectedSha256);
  });

  it("a wrong library fails every case with result-mismatch", async () => {
    const subject = await subjectFor({ ctEqFixed: "export const ctEqFixed = () => true;" });
    const result = runSuite(corpus, W2, subject, false);
    expect(suitePassed(result)).toBe(false);
    expect(result.failures).toBe(240); // 256 pairs, 16 of them genuinely equal
    expect(result.failedCases).toHaveLength(MAX_REPORTED_CASES);
    expect(result.failedCases.every(([, id]) => id === "result-mismatch")).toBe(true);
    // The digests do not depend on the library, so they still agree.
    expect(result.suiteFailures).toEqual([]);
  });

  it("names a single wrong case by its index", async () => {
    const subject = await subjectFor({ ctEqFixed: "export const ctEqFixed = (a: Uint8Array, b: Uint8Array) => (a[0] === 0x80 && a[1] === 0x80 && b[0] === 0x80 && b[1] === 0x80 ? false : a[0] === b[0] && a[1] === b[1]);" });
    const result = runSuite(corpus, W2, subject, false);
    expect(result.failures).toBe(1);
    // alphabet {00,01,80,ff}: "8080" is value 10 on each side, so pair (10, 10).
    expect(result.failedCases).toEqual([[10 * 16 + 10, "result-mismatch"]]);
  });

  it("fail-fast stops at the first failure and skips the digests", async () => {
    const subject = await subjectFor({ ctEqFixed: "export const ctEqFixed = () => true;" });
    const result = runSuite(corpus, W2, subject, true);
    expect(result.failures).toBe(1);
    expect(result.cases).toBe(2);
    expect(result.inputsSha256).toBe("");
    expect(result.suiteFailures).toEqual([]);
  });

  it("classifies a missing error, a wrong type and an unexpected error separately", async () => {
    const reject = suiteNamed("select_rejects_mismatch");
    const missing = await subjectFor({ ctSelectBytes: "export const ctSelectBytes = (a: Uint8Array) => a.slice();" });
    expect(runSuite(corpus, reject, missing, true).failedCases[0]?.[1]).toBe("missing-error");
    const wrongType = await subjectFor({ ctSelectBytes: "export const ctSelectBytes = (a: Uint8Array) => Buffer.from(a);" });
    expect(runSuite(corpus, suiteNamed("select_picks"), wrongType, true).failedCases[0]?.[1]).toBe("type-mismatch");
    const unexpected = await subjectFor({ ctSelectBytes: "export const ctSelectBytes = () => { throw new TypeError('x'); };" });
    expect(runSuite(corpus, reject, unexpected, true).failedCases[0]?.[1]).toBe("unexpected-error");
  });

  it("reports tampered counts, digests and call counts as suite-level failures", async () => {
    const run = async (change: Partial<Suite>): Promise<string[]> =>
      runSuite(corpus, withSuite(W2, change), await subjectFor(), false).suiteFailures;
    expect(await run({ inputsSha256: "0".repeat(64) })).toEqual(["stream-mismatch"]);
    expect(await run({ expectedSha256: "0".repeat(64) })).toEqual(["stream-mismatch"]);
    expect(await run({ expectedCalls: 255 })).toEqual(["call-count-mismatch"]);
    // A claim that discards what the corpus said it admits changes both counts.
    const fewer = withSuite(W2, { admit: ["lt", ["len", ["var", "a"]], ["lit", 0]] as unknown });
    const result = runSuite(corpus, withSuite(fewer, { cases: 256, discarded: 0 }), await subjectFor(), false);
    expect(result.suiteFailures).toContain("count-mismatch");
  });

  it("refuses a lying product before enumerating anything", async () => {
    const subject = await subjectFor();
    expect(() => runSuite(corpus, withSuite(W2, { cases: 1 }), subject, false)).toThrow(/declared/);
    expect(subject.totalCalls(["eq_fixed"])).toBe(0);
    const huge = withSuite(W2, { binders: Array.from({ length: 8 }, (_x, i) => [`b${i}`, "bytes_n(4096)"] as const) });
    expect(() => runSuite(corpus, huge, subject, false)).toThrow(/work/);
  });

  it("refuses IR problems before any case runs", async () => {
    for (const change of [
      { lhs: ["lit", true] as unknown, rhs: ["lit", true] as unknown },
      { lhs: ["subject_op", "eq_fixed", [["var", "a"], ["var", "b"]], [2]] as unknown, rhs: ["subject_op", "eq", [["var", "a"], ["var", "b"]]] as unknown },
      { admit: ["bogus"] as unknown },
      { rhs: ["var", "nope"] as unknown },
    ]) {
      const subject = await subjectFor();
      expect(() => runSuite(corpus, withSuite(W2, change), subject, false)).toThrow(CorpusError);
      expect(subject.totalCalls(["eq_fixed", "eq"])).toBe(0);
    }
  });

  it("a model may not call the subject", async () => {
    const tampered: Corpus = { ...corpus, models: { ...corpus.models, same: { params: ["a", "b"], body: ["subject_op", "eq", [["var", "a"], ["var", "b"]]] } } };
    const lhs = ["model_call", "same", [["var", "a"], ["var", "b"]]] as unknown;
    const subject = await subjectFor();
    expect(() => runSuite(tampered, withSuite(W2, { rhs: lhs }), subject, false)).toThrow(/forbidden/);
  });

  it("runs a suite whose subject side is the right-hand side", async () => {
    const swapped = withSuite(W1, { lhs: W1.rhs, rhs: W1.lhs });
    const result = runSuite(corpus, swapped, await subjectFor(), false);
    expect(suitePassed(result)).toBe(true);
  });

  it("counts discarded cases and rethrows what is not a verdict", async () => {
    const picks = suiteNamed("select_picks");
    const result = runSuite(corpus, picks, await subjectFor(), false);
    expect([result.cases, result.discarded]).toEqual([139_818, 101_000]);
    expect(suitePassed(result)).toBe(true);
    const wide = withSuite(W2, { rhs: ["lit", true] as unknown, lhs: ["subject_op", "eq_fixed", [["var", "a"], ["var", "b"]], [3]] as unknown });
    expect(() => runSuite(corpus, wide, new Subject(binding, new Map(), corpus.ops, corpus.errors), false)).toThrow(CorpusError);
  });
});

describe("runExamples", () => {
  it("runs every example, or only the selected ones", async () => {
    const all = runExamples(corpus, await subjectFor(), () => true);
    expect(all).toHaveLength(12);
    expect(all.every((example) => example.failure === null)).toBe(true);
    const some = runExamples(corpus, await subjectFor(), (id) => id.endsWith("eq_empty"));
    expect(some.map((example) => example.op)).toEqual(["eq"]);
  });

  it("reports a failing example and a library error", async () => {
    const wrong = runExamples(corpus, await subjectFor({ ctEq: "export const ctEq = () => true;" }), (id) => id.includes("/eq_"));
    expect(wrong.some((example) => example.failure === "result-mismatch")).toBe(true);
    const throwing = runExamples(corpus, await subjectFor({ ctEq: "export const ctEq = () => { throw new Error('x'); };" }), (id) => id.endsWith("eq_empty"));
    expect(throwing[0]?.failure).toBe("unexpected-error");
  });
});

describe("classify", () => {
  it("separates a missing error from a mismatch", () => {
    expect(classify(true, true)).toBeNull();
    expect(classify(["ok", Uint8Array.of(1)], ["err", "E"])).toBe("missing-error");
    expect(classify(["err", "F"], ["err", "E"])).toBe("result-mismatch");
    expect(classify(false, true)).toBe("result-mismatch");
  });
});

describe("negative controls", () => {
  const indexOf = (fault: string, op: string): number => corpus.controls.findIndex((control) => control.fault === fault && control.op === op);

  it("catches a real fault, and the export was reached", async () => {
    const result = runControl(corpus, binding, await exportsFor({}), indexOf("invert", "eq_fixed"));
    expect(controlPassed(result)).toBe(true);
    expect(result.reachedExport).toBe(true);
    expect(result.caughtBy.length).toBeGreaterThan(0);
  });

  it("fails a control that nothing notices", async () => {
    const control = corpus.controls[indexOf("invert", "eq")] as Corpus["controls"][number];
    const tampered: Corpus = { ...corpus, controls: [{ ...control, requiredClaims: [W1.id] }] };
    const result = runControl(tampered, binding, await exportsFor({}), 0);
    expect(controlPassed(result)).toBe(false);
    expect(result.caughtBy).toEqual([]);
    expect(result.reachedExport).toBe(false);
  });

  it("a control must reach the export", () => {
    expect(controlPassed({ id: "x", fault: "f", op: "o", targetKind: "k", caughtBy: ["a"], reachedExport: false })).toBe(false);
  });
});

describe("shuffled", () => {
  it("is a deterministic permutation, equal to the Python lane's order", () => {
    const seed = 0x5741_5252_414e_5421n;
    const order = shuffled(corpus.suites, seed).map((suite) => suite.claim);
    expect(order).toEqual([
      "eq_fixed_agrees_w16", "eq_u64_agrees", "eq_fixed_agrees_w2", "select_rejects_mismatch", "eq_fixed_agrees_w4",
      "eq_fixed_agrees_w0", "select_picks", "eq_agrees", "eq_fixed_agrees_w1",
    ]); // prettier-ignore
    expect([...order].sort()).toEqual(corpus.suites.map((suite) => suite.claim).sort());
    expect(shuffled(corpus.suites, seed).map((s) => s.id)).toEqual(shuffled(corpus.suites, seed).map((s) => s.id));
    expect(shuffled([], seed)).toEqual([]);
    expect(shuffled([1], seed)).toEqual([1]);
  });
});
