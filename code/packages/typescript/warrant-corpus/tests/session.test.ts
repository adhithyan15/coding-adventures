import { beforeAll, describe, expect, it } from "vitest";
import { BindingError, UsageError } from "../src/errors.ts";
import {
  ENV_FAULT,
  ENV_FILTER,
  ENV_OUT,
  checkFilterRules,
  environment,
  execute,
  outcomePassed,
  parseFaultSpec,
  prepare,
  type Outcome,
  type Prepared,
} from "../src/session.ts";
import { BINDING_TEXT, CORPUS_DIR, REPO_ROOT, collect, miniCorpus, newLibrary, realCorpus } from "./helpers.ts";

const corpus = realCorpus();

describe("the whole corpus (572,109 cases)", () => {
  let prepared: Prepared;
  let outcome: Outcome;
  let lines: string[];

  beforeAll(async () => {
    const lib = newLibrary();
    prepared = await prepare({ repoRoot: REPO_ROOT, corpusDir: CORPUS_DIR, bindingPath: lib.binding, packageDir: lib.packageDir });
    const sink = collect();
    outcome = execute(prepared, sink.emit);
    lines = sink.lines;
  }, 600_000);

  it("passes everything, with no `not ok` line", () => {
    expect(outcomePassed(outcome)).toBe(true);
    expect(lines.some((line) => line.startsWith("not ok"))).toBe(false);
    expect(outcome.examples).toHaveLength(12);
    expect(outcome.suites).toHaveLength(9);
    expect(outcome.controls).toHaveLength(8);
  });

  it("agrees with every digest and call count the Rust compiler wrote", () => {
    const byId = new Map(corpus.suites.map((suite) => [suite.id, suite]));
    for (const result of outcome.suites) {
      const suite = byId.get(result.id);
      expect(result.inputsSha256).toBe(suite?.inputsSha256);
      expect(result.expectedSha256).toBe(suite?.expectedSha256);
      expect(result.cases).toBe(suite?.cases);
      expect(result.discarded).toBe(suite?.discarded);
      expect(result.calls).toBe(suite?.expectedCalls);
    }
    expect(outcome.suites.reduce((sum, suite) => sum + suite.cases, 0)).toBe(572_109);
  });

  it("catches every control with a required claim", () => {
    const required = new Map(corpus.controls.map((control) => [control.id, control.requiredClaims]));
    for (const control of outcome.controls) {
      expect(control.reachedExport).toBe(true);
      expect(control.caughtBy.length).toBeGreaterThan(0);
      for (const claim of control.caughtBy) expect(required.get(control.id)).toContain(claim);
    }
  });

  it("builds a complete, sorted manifest bound to this corpus", () => {
    const manifest = outcome.manifest as Record<string, unknown>;
    expect(manifest["schema"]).toBe("warrant-lane-evidence-v1");
    expect(manifest["lane"]).toBe("typescript");
    expect(manifest["corpus"]).toBe("code/specs/fixtures/ct-compare-warrant-v1");
    expect(manifest["corpus_sha256"]).toBe(corpus.corpusSha256);
    expect(manifest["evidence_seed"]).toBe("6287397068022371361");
    expect(manifest["faulted"]).toBe(false);
    const tests = manifest["tests"] as { id: string }[];
    expect(tests).toHaveLength(29);
    const ids = tests.map((row) => row.id);
    expect([...ids].sort()).toEqual(ids);
  });
});

describe("a mini corpus", () => {
  it("is deterministic: two runs give the same manifest", async () => {
    const lib = newLibrary();
    const { root, corpusDir } = miniCorpus();
    const run = async (): Promise<string> => {
      const prepared = await prepare({ repoRoot: root, corpusDir, bindingPath: lib.binding, packageDir: lib.packageDir });
      const manifest = execute(prepared, () => undefined).manifest;
      expect(manifest).not.toBeNull();
      return JSON.stringify(manifest);
    };
    expect(await run()).toBe(await run());
  });

  it("a faulted run fails fast, says why, and writes no manifest", async () => {
    const lib = newLibrary();
    const { root, corpusDir } = miniCorpus();
    const prepared = await prepare({ repoRoot: root, corpusDir, bindingPath: lib.binding, packageDir: lib.packageDir, fault: "invert", faultOp: "eq_fixed", filter: "/exhaustive/" });
    const sink = collect();
    const outcome = execute(prepared, sink.emit);
    expect(outcome.manifest).toBeNull();
    expect(outcome.controls).toEqual([]);
    expect(outcome.examples).toEqual([]);
    expect(outcomePassed(outcome)).toBe(false);
    expect(sink.lines.some((line) => /^not ok .*@\d+ result-mismatch$/.test(line))).toBe(true);
    expect(sink.lines.some((line) => /^not ok warrant\/ct_compare\/exhaustive\/eq_fixed_agrees_w\d+ result-mismatch$/.test(line))).toBe(true);
  });

  it("names a failing case count when more than a few cases fail", async () => {
    const lib = newLibrary({ ctEqFixed: "export const ctEqFixed = () => true;" });
    const { root, corpusDir } = miniCorpus();
    const prepared = await prepare({ repoRoot: root, corpusDir, bindingPath: lib.binding, packageDir: lib.packageDir });
    const sink = collect();
    const outcome = execute(prepared, sink.emit);
    expect(outcome.manifest).toBeNull();
    expect(sink.lines.some((line) => /failing cases in total$/.test(line))).toBe(true);
    expect(sink.lines.some((line) => line.startsWith("not ok") && line.includes("@"))).toBe(true);
    expect(sink.lines.filter((line) => line.startsWith("not ok ") && !line.includes("@")).length).toBeGreaterThan(0);
  });

  it("reports a suite-level failure for a tampered digest", async () => {
    const lib = newLibrary();
    const { root, corpusDir } = miniCorpus((document) => {
      const suite = (document["suites"] as Record<string, unknown>[])[0] as Record<string, unknown>;
      suite["inputs_sha256"] = "0".repeat(64);
    });
    const prepared = await prepare({ repoRoot: root, corpusDir, bindingPath: lib.binding, packageDir: lib.packageDir });
    const sink = collect();
    const outcome = execute(prepared, sink.emit);
    expect(outcomePassed(outcome)).toBe(false);
    expect(sink.lines.some((line) => line.endsWith(" stream-mismatch"))).toBe(true);
    expect(outcome.manifest).toBeNull();
  });

  it("refuses a filter that matches nothing", async () => {
    const lib = newLibrary();
    const { root, corpusDir } = miniCorpus();
    const prepared = await prepare({ repoRoot: root, corpusDir, bindingPath: lib.binding, packageDir: lib.packageDir, fault: "invert", filter: "no such id" });
    expect(() => execute(prepared, () => undefined)).toThrow(UsageError);
  });
});

describe("prepare refuses a binding that does not match the corpus", () => {
  const refused = async (text: string, code?: string): Promise<void> => {
    const lib = newLibrary({}, text);
    try {
      await prepare({ repoRoot: REPO_ROOT, corpusDir: CORPUS_DIR, bindingPath: lib.binding, packageDir: lib.packageDir });
    } catch (error) {
      expect(error).toBeInstanceOf(BindingError);
      if (code) expect((error as BindingError).code).toBe(code);
      return;
    }
    throw new Error("the binding was accepted");
  };
  const edit = (from: string, to: string): string => {
    expect(BINDING_TEXT).toContain(from);
    return BINDING_TEXT.replace(from, to);
  };

  it("checks module, lane, totality, widths and error rows", async () => {
    await refused(edit("module ct_compare", "module other"), "module");
    await refused(edit("lane typescript", "lane python"), "lane");
    await refused(edit("package typescript/ct-compare", "package python/ct-compare"), "lane");
    await refused(edit("op eq_u64 export=ctEqU64\n", ""), "totality");
    await refused(`${BINDING_TEXT}op extra export=ctEq\n`, "totality");
    await refused(edit("width=implicit", ""), "width");
    await refused(edit("op eq export=ctEq", "op eq export=ctEq width=implicit"), "width");
    await refused(edit(" errors=CT_SELECT_LENGTH_MISMATCH:RangeError", ""), "totality");
    await refused(edit("CT_SELECT_LENGTH_MISMATCH:RangeError", "CT_SELECT_LENGTH_MISMATCH:RangeError,OTHER_VARIANT:TypeError"), "totality");
    await refused(edit("export=ctEq\n", "export=ctMissing\n"), "export");
  });

  it("refuses a corpus outside the repository root", async () => {
    const lib = newLibrary();
    await expect(
      prepare({ repoRoot: REPO_ROOT, corpusDir: REPO_ROOT, bindingPath: lib.binding, packageDir: lib.packageDir }),
    ).rejects.toThrow();
    const { corpusDir } = miniCorpus();
    await expect(
      prepare({ repoRoot: REPO_ROOT, corpusDir, bindingPath: lib.binding, packageDir: lib.packageDir }),
    ).rejects.toThrow(/inside the repository root/);
  });
});

describe("fault and filter rules", () => {
  it("parses the fault spec", () => {
    expect(parseFaultSpec(undefined)).toEqual([null, null]);
    expect(parseFaultSpec(null)).toEqual([null, null]);
    expect(parseFaultSpec("")).toEqual([null, null]);
    expect(parseFaultSpec("none")).toEqual([null, null]);
    expect(parseFaultSpec("invert")).toEqual(["invert", null]);
    expect(parseFaultSpec("invert:eq")).toEqual(["invert", "eq"]);
    expect(parseFaultSpec("invert:")).toEqual(["invert", null]);
    expect(() => parseFaultSpec("bogus:eq")).toThrow(UsageError);
    expect(() => parseFaultSpec("toString")).toThrow(UsageError);
  });

  it("refuses a filter without a real fault", () => {
    expect(() => checkFilterRules(undefined, "x")).toThrow(UsageError);
    expect(() => checkFilterRules("none", "x")).toThrow(UsageError);
    expect(() => checkFilterRules("", "x")).toThrow(UsageError);
    expect(() => checkFilterRules(null, "x")).toThrow(UsageError);
    checkFilterRules("invert", "x");
    checkFilterRules(undefined, undefined);
    checkFilterRules("none", null);
  });

  it("reads the three variables", () => {
    expect(environment({ [ENV_FAULT]: "invert", [ENV_FILTER]: "x", [ENV_OUT]: "o" })).toEqual(["invert", "x", "o"]);
    expect(environment({ [ENV_FILTER]: "", [ENV_OUT]: "" })).toEqual([undefined, null, null]);
    expect(environment()).toHaveLength(3);
  });
});
