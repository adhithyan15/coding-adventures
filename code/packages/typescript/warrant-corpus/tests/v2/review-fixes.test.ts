/**
 * The findings of the CV2-S7a review, one test group each (B1, B2, S1 to S9, the disagreement over `chunk-file`, N1 to N6).
 *
 * Every group states the rule it pins by the section of VER04r it comes from, so that a failure reads as a spec
 * violation and not as an implementation detail.
 */
import { cpSync, existsSync, readFileSync, readdirSync, renameSync, symlinkSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { checkSlimChunks, loadSuite, readCorpusPart } from "../../src/v2/chunks.ts";
import { MAX_IR_WORK_SLIM, MAX_TOTAL_WORK_SLIM, checkWork, loadV2Corpus, runSuite } from "../../src/v2/corpus.ts";
import { KitError } from "../../src/v2/errors.ts";
import * as api from "../../src/v2/index.ts";
import { Program, readOp } from "../../src/v2/ir.ts";
import { runCase, runKit } from "../../src/v2/kit.ts";
import { checkNames, parseType, readTypeTable, showType } from "../../src/v2/types.ts";
import { decodeJson } from "../../src/v2/values.ts";
import { CORPORA, KIT_DIR, kitCase, tempDir } from "./support.ts";

type Json = any; // eslint-disable-line @typescript-eslint/no-explicit-any
const LEVELS = new Set(["ir.core"]);
const clone = <X>(value: X): X => JSON.parse(JSON.stringify(value)) as X;
/** The code of the refusal `run` throws, `none` when it returns. */
const code = (run: () => unknown): string => {
  try {
    run();
  } catch (error) {
    if (error instanceof KitError) return error.code;
    throw error;
  }
  return "none";
};
const message = (run: () => unknown): string => {
  try {
    run();
  } catch (error) {
    return (error as Error).message;
  }
  return "";
};
const NAT = (v: string | number): Json => ["lit", { type: "Nat", value: String(v) }];
const U8 = (v: number): Json => ["lit", { type: "U8", value: String(v) }];
const NAMES = ["constructor", "toString", "valueOf", "hasOwnProperty", "__proto__", "isPrototypeOf"];

// ---------------------------------------------------------------------------
// B2: a name read from input is never looked up in a plain object
// ---------------------------------------------------------------------------

describe("B2: names that Object.prototype also has (section 2.2 rule 4, section 11 `type` and `descriptor`)", () => {
  it("is a type of the kind `type` in v2-value when nothing declares it, never a harness error", () => {
    for (const name of NAMES) {
      expect(runCase("v2-value", { input_hex: "30", type: name, types: {}, requires: ["ir.core"] }), name).toEqual({ outcome: "reject", error: "type" });
    }
    // and a name that is declared is a type like any other
    expect(parseType("toString")).toEqual({ k: "Name", name: "toString" });
  });

  it("accepts a legal error type, a record and a variant with such a name", () => {
    for (const name of NAMES) {
      const types = JSON.parse(`{"${name}": {"kind": "error", "cases": ["BAD"]}}`);
      const table = readTypeTable(types);
      expect(table.errors.has(name), name).toBe(true);
      const ty = parseType(`Result<U8,${name}>`);
      expect(() => checkNames(ty, table)).not.toThrow();
      expect(showType(ty)).toBe(`Result<U8,${name}>`);
      // through the whole value path: an error case of that type decodes and streams
      const out = runCase("v2-value", { input_hex: Buffer.from('{"err":"BAD"}').toString("hex"), type: `Result<U8,${name}>`, types, requires: ["ir.core"] });
      expect(out["outcome"], name).toBe("accept");
      expect(decodeJson({ err: "BAD" }, ty, table)).toEqual({ err: "BAD" });
      const data = readTypeTable(JSON.parse(`{"${name}": {"kind": "record", "fields": [["a", "U8"]]}}`));
      expect(data.data.get(name)).toBe("record");
    }
    // the scalar names stay refused as type names, and so does a bare Object.prototype name that is *not* declared
    expect(() => readTypeTable({ U8: { kind: "error", cases: ["A"] } })).toThrow(/not usable/);
    expect(() => checkNames(parseType("constructor"), readTypeTable({}))).toThrow(/not declared/);
  });

  it("is a `descriptor` refusal for a descriptor kind with such a name, not level-missing", () => {
    for (const name of NAMES) {
      const raw = JSON.parse(`{"d": "${name}"}`);
      expect(runCase("v2-domain", { type: "U8", descriptor: raw, index: 0, requires: ["ir.core"] }), name).toEqual({ error: "descriptor" });
    }
    // the kinds of levels this lane lacks stay level-missing (`tuple` is held since S10f, so it is a plain descriptor refusal here)
    expect(runCase("v2-domain", { type: "U8", descriptor: { d: "variant" }, index: 0, requires: ["ir.core"] })).toEqual({ error: "level-missing" });
    expect(runCase("v2-domain", { type: "U8", descriptor: { d: "tuple" }, index: 0, requires: ["ir.core"] })).toEqual({ error: "descriptor" });
  });
});

// ---------------------------------------------------------------------------
// B1: slim_chunks on the corpus path, and the work meter
// ---------------------------------------------------------------------------

describe("B1: slim_chunks is validated wherever a suite row is read (section 14)", () => {
  const ragged = kitCase("v2-chunk/ragged-short-last-chunk");
  const load = (slim: unknown): string => {
    const c = clone(ragged);
    c.suite.slim_chunks = slim;
    return code(() => loadSuite(readCorpusPart(c.corpus, LEVELS), c.suite));
  };

  it("refuses an empty, non-list, short, unordered, non-integer or oversized value, whatever its entries are", () => {
    for (const bad of [[], 7, "0,16", null, {}, [0], [16], [-1], [0, 16.5], [0, "16"], [0, null], [0, true], [0, 17], [16, 0], [0, 4, 4, 16], [0, 1e21], [0, NaN],
      [0, ...Array.from({ length: 31 }, (_u, i) => i + 1), 16]]) {
      expect(load(bad), JSON.stringify(bad)).toBe("slim-chunks");
    }
    expect(load([0, 16])).toBe("none");
    expect(() => checkSlimChunks([-1], 17)).toThrow(KitError);
  });

  const corpusDir = (): string => {
    const dir = tempDir("warrant-slim-");
    cpSync(CORPORA[0]?.dir as string, dir, { recursive: true });
    return dir;
  };
  const withSlim = (slim: unknown): string => {
    const dir = corpusDir();
    const document = JSON.parse(readFileSync(join(dir, "corpus.json"), "utf8"));
    document.suites[0].slim_chunks = slim;
    writeFileSync(join(dir, "corpus.json"), JSON.stringify(document));
    return dir;
  };

  it("refuses a corpus whose slim set is malformed, so a slim run can never run zero chunks or crash on [-1]", () => {
    const suiteChunks = (JSON.parse(readFileSync(join(CORPORA[0]?.dir as string, "corpus.json"), "utf8")).suites[0] as { chunks: number }).chunks;
    for (const bad of [[], 7, [-1], [0], ["a"], [0, suiteChunks]]) {
      expect(() => loadV2Corpus(withSlim(bad)), JSON.stringify(bad)).toThrow(/slim_chunks/);
    }
    // and the committed corpus runs a non-empty slim set
    const corpus = loadV2Corpus(CORPORA[0]?.dir as string);
    const record = corpus.suites[0] as (typeof corpus.suites)[number];
    expect(runSuite(corpus, record, "slim").chunks).toBeGreaterThan(0);
  });
});

describe("B1: checkWork is part of the run and validates what it adds", () => {
  const corpus = loadV2Corpus(CORPORA[0]?.dir as string);
  const base = (corpus.suites.find((s) => s.state === "live") as { suite: Json }).suite;
  const fake = (suite: Json, controls: Json[] = []): never => ({ ...corpus, suites: [{ id: suite.id, state: "live", suite }], controls }) as never;

  it("is exported from the package index and called by runSuite before anything runs", () => {
    expect(api.checkWork).toBe(checkWork);
    // a corpus whose controls re-run the same suite until the slim total is exceeded is refused by runSuite itself
    const record = corpus.suites.find((s) => s.state === "live") as (typeof corpus.suites)[number];
    const slim = (record.suite?.slimChunks ?? []).length;
    expect(slim).toBeGreaterThan(0);
    const reruns = Math.ceil(MAX_TOTAL_WORK_SLIM / (record.suite?.chunkSize ?? 1)) + 1;
    const crowded = { ...corpus, controls: Array.from({ length: reruns * 2 }, (_u, i) => ({ id: `c${i}`, op: "x", fault: "y", state: "ok", requiredClaims: [record.id] })) };
    expect(() => runSuite(crowded as never, record, "slim")).toThrow(/total work/);
  });

  it("refuses a slim chunk that is not a chunk, an unknown depth and any NaN total, instead of passing a limit", () => {
    for (const bad of [Number.NaN, -1, 0.5, base.chunks, "0", undefined]) {
      expect(() => checkWork(fake({ ...base, slimChunks: [0, bad] }), "slim"), String(bad)).toThrow(/not a chunk/);
    }
    expect(() => checkWork(corpus, "deep" as never)).toThrow(/depth/);
    // a NaN cost can not slip under the limit either
    expect(() => checkWork(fake({ ...base, irCost: Number.NaN }), "slim")).toThrow(/IR work/);
    expect(() => checkWork(fake({ ...base, product: Number.NaN }), "full")).toThrow(/total work/);
  });

  it("counts the re-runs of the controls in the slim total (section 14) and not in a full run", () => {
    const record = corpus.suites.find((s) => s.state === "live") as (typeof corpus.suites)[number];
    const once = checkWork(fake(base), "slim").ordinals;
    const control = { id: "c", op: "x", fault: "y", state: "ok", requiredClaims: [record.id, record.id, "no-such-suite"] };
    expect(checkWork(fake(base, [control]), "slim").ordinals).toBe(once * 3);
    expect(checkWork(fake(base, [{ ...control, state: "not-live" }]), "slim").ordinals).toBe(once);
    expect(checkWork(fake(base, [control]), "full").ordinals).toBe(base.product);
    expect(MAX_IR_WORK_SLIM).toBe(64_000_000_000);
  });
});

// ---------------------------------------------------------------------------
// S1: 64 models of depth 62
// ---------------------------------------------------------------------------

describe("S1: a spec-legal chain of 64 models whose bodies are 62 deep compiles and evaluates (section 14)", () => {
  const chain = (models: number, nots: number): { models: Record<string, unknown>; order: string[] } => {
    const out: Record<string, unknown> = {};
    const order: string[] = [];
    for (let i = 1; i <= models; i += 1) {
      let body: unknown = i === 1 ? ["var", "a"] : ["model_call", `m${i - 1}`, [["var", "a"]]];
      for (let d = 0; d < nots; d += 1) body = ["not", body];
      out[`m${i}`] = { params: [["a", "Bool"]], body };
      order.push(`m${i}`);
    }
    return { models: out, order };
  };

  it("never overflows the stack, in compile or in evaluation, and gives the right value", () => {
    const { models, order } = chain(64, 60);
    const program = new Program({ types: {}, ops: {}, models, modelOrder: order, levels: LEVELS, maxIrCost: 65_536 });
    const compiled = program.compileExpr(["model_call", "m64", [["lit", true]]], [], "pure");
    expect(compiled.cost).toBeGreaterThan(3_000);
    // 64 models x 60 negations = 3,840 negations: an even number, so the value is the argument
    expect(compiled.run([])).toBe(true);
    // each model of the chain is also compiled once, whichever order the calls reach it in
    const odd = chain(63, 61);
    const p2 = new Program({ types: {}, ops: {}, models: odd.models, modelOrder: odd.order, levels: LEVELS, maxIrCost: 65_536 });
    expect(p2.compileExpr(["model_call", "m63", [["lit", true]]], [], "pure").run([])).toBe(false);
    // and through the corpus-part door of a chunk case, as a compile-time refusal would be
    expect(code(() => new Program({ types: {}, ops: {}, models: chain(64, 70).models, modelOrder: chain(64, 70).order, levels: LEVELS, maxIrCost: 65_536 }))).toBe("depth");
  });

  it("reuses a callee compiled for a tainted context: the stand-in path shares the same bodies", () => {
    const { models, order } = chain(9, 5);
    const program = new Program({ types: {}, ops: {}, models, modelOrder: order, levels: LEVELS });
    expect(program.callModel("m9", [true])).toBe(false);
    expect(program.callModel("m9", [false])).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// S2: the static type keeps widths
// ---------------------------------------------------------------------------

describe("S2: expect.type is the static type and keeps byte widths (R51, R73, Appendix A)", () => {
  const ir = (extra: Json): Json => ({ requires: ["ir.core"], levels: ["ir.core"], model_order: [], models: {}, ops: {}, subjects: {}, types: {}, env: [], ...extra });
  const type = (extra: Json): unknown => runCase("v2-ir", ir(extra))["type"];
  const lit = (hex: string): Json => ["lit", { type: "Bytes", value: hex }];

  it("prints Bytes<N> for a var, a bare literal, concat, an if of equal widths and a subject_op at a width", () => {
    expect(type({ ir: ["var", "a"], env: [["a", { type: "Bytes<2>", value: "0102" }]] })).toBe("Bytes<2>");
    expect(type({ ir: ["var", "a"], env: [["a", { type: "Bytes", value: "0102" }]] })).toBe("Bytes");
    expect(type({ ir: lit("0a0b") })).toBe("Bytes<2>");
    expect(type({ ir: ["concat", lit("0a"), lit("0b")] })).toBe("Bytes<2>");
    expect(type({ ir: ["if", ["lit", true], lit("0a0b"), lit("0c0d")] })).toBe("Bytes<2>");
    // different widths have no common width: plain Bytes
    expect(type({ ir: ["if", ["lit", true], lit("0a0b"), lit("0c")] })).toBe("Bytes");
    expect(type({ ir: ["take", lit("0a0b"), NAT(1)] })).toBe("Bytes");
    const op = { params: [["a", "Bytes<N>"]], ret: "Bytes<N>", width_params: ["N"], requires: ["ir.core"] };
    const stand = { m: { params: [["a", "Bytes"]], body: ["var", "a"] } };
    expect(
      type({ ir: ["subject_op", "idn", [lit("0a0b")], [2]], ops: { idn: op }, models: stand, model_order: ["m"], subjects: { idn: "m" } }),
    ).toBe("Bytes<2>");
    expect(() => type({ ir: ["ok", lit("0a0b")] })).toThrow(/undetermined/);
  });
});

// ---------------------------------------------------------------------------
// S3: a fault case's own levels
// ---------------------------------------------------------------------------

describe("S3: v2-fault honours the case's levels, cut down to the runner's (section 11)", () => {
  const base = kitCase("v2-fault/off-by-one-u8");
  it("is level-missing for levels [] and for a levels list without ir.core, and a value when ir.core is there", () => {
    expect(runCase("v2-fault", { ...base, levels: [] })).toEqual({ error: "level-missing" });
    expect(runCase("v2-fault", { ...base, levels: ["ir.fixed"] })).toEqual({ error: "level-missing" });
    expect(runCase("v2-fault", { ...base, levels: ["ir.core"] })).toEqual(base.expect);
    expect(runCase("v2-fault", { ...base, levels: ["ir.core", "ir.fixed"] })).toEqual(base.expect);
    // a runner that lacks the level too
    expect(runCase("v2-fault", base, { levels: new Set() })).toEqual({ error: "level-missing" });
  });

  it("treats a wrong number of args as a broken fixture, not as an outcome", () => {
    expect(() => runCase("v2-fault", { ...base, args: ["1"] })).toThrow(/args/);
    expect(() => runCase("v2-fault", { ...base, args: "12" })).toThrow(/args/);
  });
});

// ---------------------------------------------------------------------------
// S4 and S5: unknown tags in a not-live suite; level-missing outside the closed set of v2-chunk
// ---------------------------------------------------------------------------

describe("S4: an unknown tag cannot hide inside a not-live suite (section 14, R69 (c) and (d))", () => {
  const ragged = kitCase("v2-chunk/ragged-short-last-chunk");
  const load = (edit: (suite: Json) => void): KitError => {
    const c = clone(ragged);
    edit(c.suite);
    try {
      loadSuite(readCorpusPart(c.corpus, LEVELS), c.suite);
    } catch (error) {
      return error as KitError;
    }
    throw new Error("the suite loaded");
  };

  it("refuses the unknown tag of oracle.expected in a suite whose admit needs ir.fixed", () => {
    const error = load((s) => {
      s.admit = ["eq", ["bit_not", U8(1)], U8(254)];
      s.oracle.expected = ["frobnicate"];
    });
    expect(error.code).toBe("corpus");
    expect(error.message).toMatch(/tag/);
    // without the unknown tag the same suite is not-live
    expect(load((s) => (s.admit = ["eq", ["bit_not", U8(1)], U8(254)])).code).toBe("level-missing");
  });

  it("refuses the unknown tag in the body of a suite with a tuple binder", () => {
    const error = load((s) => {
      s.binders[0][1].type = "(U8,U8)";
      s.body = ["frobnicate"];
    });
    expect(error.code).toBe("corpus");
    expect(load((s) => (s.binders[0][1].type = "(U8,U8)")).code).toBe("level-missing");
  });

  it("also refuses a subject_op in admit, a call out of model_order and a tree over 64 deep in a not-live suite", () => {
    const tuple = (s: Json): void => {
      s.binders[0][1].type = "(U8,U8)";
    };
    expect(load((s) => { tuple(s); s.admit = ["subject_op", "rop", []]; }).code).toBe("corpus");
    expect(load((s) => { tuple(s); s.admit = ["model_call", "ghost", []]; }).code).toBe("corpus");
    let deep: Json = ["lit", true];
    for (let i = 0; i < 70; i += 1) deep = ["not", deep];
    expect(load((s) => { tuple(s); s.admit = deep; }).code).toBe("corpus");
  });

  it("is a corpus error in a corpus directory too, not a not-live record", () => {
    const dir = tempDir("warrant-s4-");
    cpSync(CORPORA[1]?.dir as string, dir, { recursive: true });
    const document = JSON.parse(readFileSync(join(dir, "corpus.json"), "utf8"));
    const victim = document.suites.find((s: Json) => s.requires.includes("ir.tuple") || s.requires.includes("ir.fixed"));
    expect(victim).toBeDefined();
    victim.oracle = { ...victim.oracle };
    victim.admit = ["frobnicate"];
    writeFileSync(join(dir, "corpus.json"), JSON.stringify(document));
    expect(() => loadV2Corpus(dir)).toThrow(/tag/);
  });
});

describe("S5: v2-chunk never answers level-missing (R69 (e))", () => {
  it("is `corpus` for a suite whose requires is inside the lane's levels and whose reach is not", () => {
    const c = clone(kitCase("v2-chunk/ragged-short-last-chunk"));
    c.suite.binders[0][1].type = "(U8,U8)";
    c.expect = { error: "corpus" };
    expect(runCase("v2-chunk", c)).toEqual({ error: "corpus" });
    const viaTag = clone(kitCase("v2-chunk/ragged-short-last-chunk"));
    viaTag.suite.admit = ["eq", ["bit_not", U8(1)], U8(254)];
    expect(runCase("v2-chunk", viaTag)).toEqual({ error: "corpus" });
  });
});

// ---------------------------------------------------------------------------
// S6, S7, N3, N4: the corpus directory
// ---------------------------------------------------------------------------

describe("the corpus directory", () => {
  const ct = CORPORA[0]?.dir as string;
  const copy = (): string => {
    const dir = tempDir("warrant-dir-");
    cpSync(ct, dir, { recursive: true });
    return dir;
  };
  const corpusOf = (dir: string): Json => JSON.parse(readFileSync(join(dir, "corpus.json"), "utf8"));
  const write = (dir: string, document: unknown): void => writeFileSync(join(dir, "corpus.json"), JSON.stringify(document));

  it("S6: refuses a symlinked chunks/ directory and a symlinked corpus directory component, not following either", () => {
    const dir = copy();
    const real = join(dir, "chunks-real");
    renameSync(join(dir, "chunks"), real);
    symlinkSync(real, join(dir, "chunks"), "dir");
    expect(() => loadV2Corpus(dir)).toThrow(/symbolic link/);
    // a link that leaves the fixture directory entirely is refused the same way
    const out = copy();
    const elsewhere = tempDir("warrant-elsewhere-");
    cpSync(join(out, "chunks"), join(elsewhere, "chunks"), { recursive: true });
    renameSync(join(out, "chunks"), join(out, "chunks-moved"));
    symlinkSync(join(elsewhere, "chunks"), join(out, "chunks"), "dir");
    expect(() => loadV2Corpus(out)).toThrow(/symbolic link/);
    // the directory the caller hands over may itself be reached through a link: only what is below it is the corpus's
    const holder = tempDir("warrant-holder-");
    symlinkSync(copy(), join(holder, "fixture"), "dir");
    expect(() => loadV2Corpus(join(holder, "fixture"))).not.toThrow();
  });

  it("S6: refuses a chunk file that is a symlink when a suite runs", () => {
    const dir = copy();
    const name = readdirSync(join(dir, "chunks"))[0] as string;
    renameSync(join(dir, "chunks", name), join(dir, `${name}.real`));
    symlinkSync(join(dir, `${name}.real`), join(dir, "chunks", name));
    expect(() => loadV2Corpus(dir)).toThrow(/regular file|symbolic link/);
  });

  it("N3: a missing chunk file is a clean chunk-file refusal and a missing chunks/ directory a corpus one, never ENOENT", () => {
    const dir = copy();
    const corpus = loadV2Corpus(dir);
    const record = corpus.suites.find((s) => s.state === "live") as (typeof corpus.suites)[number];
    const file = readdirSync(join(dir, "chunks")).find((n) => n.startsWith(record.suite?.claim.replaceAll("/", ".") ?? "")) as string;
    renameSync(join(dir, "chunks", file), join(dir, `${file}.away`));
    expect(code(() => runSuite(corpus, record))).toBe("chunk-file");
    const noDir = copy();
    renameSync(join(noDir, "chunks"), join(noDir, "chunks-away"));
    expect(code(() => loadV2Corpus(noDir))).toBe("corpus");
    const noCorpus = copy();
    renameSync(join(noCorpus, "corpus.json"), join(noCorpus, "corpus.away"));
    expect(code(() => loadV2Corpus(noCorpus))).toBe("corpus");
    expect(existsSync(join(noCorpus, "corpus.away"))).toBe(true);
  });

  it("N3: a fault row that does not compile is a corpus error, not the kit's ill-typed", () => {
    const dir = copy();
    const document = corpusOf(dir);
    const row = document.faults.find((f: Json) => f.kind === "post");
    expect(row).toBeDefined();
    row.expr = ["lt", ["var", "result"], ["var", "result"]];
    write(dir, document);
    expect(code(() => loadV2Corpus(dir))).toBe("corpus");
    const forbidden = copy();
    const doc2 = corpusOf(forbidden);
    doc2.faults.find((f: Json) => f.kind === "post").expr = ["subject_op", "x", []];
    write(forbidden, doc2);
    expect(code(() => loadV2Corpus(forbidden))).toBe("corpus");
  });

  it("S7: refuses a duplicated control id and a duplicated fault row", () => {
    const controls = copy();
    const a = corpusOf(controls);
    a.controls.push(clone(a.controls[0]));
    write(controls, a);
    expect(() => loadV2Corpus(controls)).toThrow(/control id appears twice/);
    const faults = copy();
    const b = corpusOf(faults);
    b.faults.push(clone(b.faults[0]));
    write(faults, b);
    expect(() => loadV2Corpus(faults)).toThrow(/fault row appears twice/);
    const claims = copy();
    const c = corpusOf(claims);
    c.controls[0].required_claims = [7];
    write(claims, c);
    expect(() => loadV2Corpus(claims)).toThrow(/required_claims/);
  });

  it("N4: refuses an empty or malformed omitted_examples, a suite with no admitted case, and levels that are not the union of requires", () => {
    for (const [bad, why] of [
      [[], /omitted_examples/],
      ["x", /omitted_examples/],
      [[{ id: "b", reason: "value-over-limit" }, { id: "a", reason: "value-over-limit" }], /sorted/],
      [[{ id: "a", reason: "value-over-limit" }, { id: "a", reason: "not-op-eq-pure" }], /sorted/],
      [[{ id: "a", reason: "because" }], /unknown reason/],
      [[{ id: "a", reason: "value-over-limit", extra: 1 }], /\{id, reason\}/],
      [[5], /\{id, reason\}/],
    ] as [unknown, RegExp][]) {
      const dir = copy();
      write(dir, { ...corpusOf(dir), omitted_examples: bad });
      expect(() => loadV2Corpus(dir), JSON.stringify(bad)).toThrow(why);
    }
    // a well-formed list is accepted by this check (the key set then still needs the rest of the document)
    const fine = copy();
    write(fine, { ...corpusOf(fine), omitted_examples: [{ id: "a", reason: "value-over-limit" }, { id: "b", reason: "not-op-eq-pure" }] });
    expect(() => loadV2Corpus(fine)).not.toThrow();

    const levels = copy();
    write(levels, { ...corpusOf(levels), levels: ["ir.core", "ir.fixed", "ir.tuple"] });
    expect(() => loadV2Corpus(levels)).toThrow(/union/);
    const dup = copy();
    write(dup, { ...corpusOf(dup), levels: ["ir.core", "ir.core"] });
    expect(() => loadV2Corpus(dup)).toThrow(/union/);
    const missing = copy();
    write(missing, { ...corpusOf(missing), levels: [] });
    expect(() => loadV2Corpus(missing)).toThrow(/union/);
  });

  it("N4: a suite that admits no case is refused, in the kit and in a corpus", () => {
    const c = clone(kitCase("v2-chunk/ragged-short-last-chunk"));
    c.suite.cases = 0;
    expect(message(() => loadSuite(readCorpusPart(c.corpus, LEVELS), c.suite))).toMatch(/at least one case/);
    expect(runCase("v2-chunk", c)).toEqual({ error: "corpus" });
  });
});

// ---------------------------------------------------------------------------
// S8, S9, N1, N2
// ---------------------------------------------------------------------------

describe("S8: model_order is a permutation of models (section 1.4, R69 (c) tier 1)", () => {
  const program = (models: Json, order: Json, code_: "corpus" | "ill-typed" = "ill-typed"): Program =>
    new Program({ types: {}, ops: {}, models, modelOrder: order, levels: LEVELS, structuralCode: code_ });

  it("refuses a declared model that model_order leaves out, so an ill-typed body cannot load unseen", () => {
    const models = { good: { params: [], body: ["lit", true] }, bad: { params: [], body: ["add", ["lit", true], U8(1)] } };
    expect(code(() => program(models, ["good"]))).toBe("model-order");
    // listed, the same ill-typed body is found
    expect(code(() => program(models, ["good", "bad"]))).toBe("ill-typed");
    // in a corpus the refusal is `corpus`
    const part = clone(kitCase("v2-chunk/ragged-short-last-chunk").corpus);
    part.models.extra = { params: [], body: ["lit", true] };
    expect(code(() => readCorpusPart(part, LEVELS))).toBe("corpus");
    expect(code(() => program({ ...models, good: { params: [], body: ["lit", true] } }, ["bad", "good"]))).toBe("ill-typed");
  });

  it("types, costs and chain-checks every listed model of the lane's levels", () => {
    const heavy = { a: { params: [], body: ["not", ["lit", true]] } };
    expect(code(() => new Program({ types: {}, ops: {}, models: heavy, modelOrder: ["a"], levels: LEVELS }))).toBe("none");
    const unlisted = { a: { params: [], body: ["lit", true] }, z: { params: [], body: ["bit_not", U8(1)] } };
    expect(code(() => program(unlisted, ["a"]))).toBe("model-order");
  });
});

describe("S9: the oracle's subject and body (section 4.3)", () => {
  const ragged = kitCase("v2-chunk/ragged-short-last-chunk");
  const load = (edit: (suite: Json) => void): KitError => {
    const c = clone(ragged);
    edit(c.suite);
    try {
      loadSuite(readCorpusPart(c.corpus, LEVELS), c.suite);
    } catch (error) {
      return error as KitError;
    }
    throw new Error("the suite loaded");
  };

  it("refuses a subject side whose type does not fit result_type, Bool for a Nat suite included", () => {
    const wrongSubject = (subject: Json): KitError =>
      load((s) => {
        s.oracle.subject = subject;
        s.body = ["eq", subject, s.oracle.expected];
      });
    // a Nat result against an Int-typed subject: the body unifies, the subject does not fit
    expect(wrongSubject(["add", ["var", "n"], ["var", "n"]]).message).toMatch(/subject does not fit result_type/);
    // a Bool subject for a Nat suite is refused (by the body's own typing, and by the subject check)
    expect(wrongSubject(["lit", true]).code).toBe("corpus");
    const alone = load((s) => (s.oracle.subject = ["lit", true]));
    expect(alone.code).toBe("corpus");
  });

  it("refuses a reference suite whose body is not its subject compared with its expected, in either order", () => {
    expect(load((s) => (s.body = ["lit", true])).message).toMatch(/not its subject compared with its expected/);
    expect(load((s) => (s.body = ["ne", s.body[1], s.body[2]])).message).toMatch(/not its subject compared/);
    expect(load((s) => (s.body = ["eq", s.body[1], s.body[1]])).message).toMatch(/not its subject compared/);
    // the swapped order is the same claim (the generator writes both)
    const swapped = clone(ragged);
    swapped.suite.body = ["eq", swapped.suite.body[2], swapped.suite.body[1]];
    expect(() => loadSuite(readCorpusPart(swapped.corpus, LEVELS), swapped.suite)).not.toThrow();
  });
});

describe("N1: a duplicated parameter name is ill-typed; N2: an op's requires names defined levels", () => {
  it("refuses a model, an op and a binder that name a parameter twice", () => {
    const models = { m: { params: [["a", "U8"], ["a", "U8"]], body: ["var", "a"] } };
    expect(code(() => new Program({ types: {}, ops: {}, models, modelOrder: ["m"], levels: LEVELS }))).toBe("ill-typed");
    expect(code(() => readOp("f", { params: [["a", "U8"], ["a", "U8"]], ret: "U8" }))).toBe("ill-typed");
    expect(code(() => readOp("f", { params: [["a", "U8"], ["b", "U8"]], ret: "U8" }))).toBe("none");
    const c = clone(kitCase("v2-chunk/ragged-short-last-chunk"));
    c.suite.binders[1][0] = c.suite.binders[0][0];
    expect(code(() => loadSuite(readCorpusPart(c.corpus, LEVELS), c.suite))).toBe("corpus");
  });

  it("refuses an op whose requires names a level this document does not define, or is not a list of names", () => {
    for (const bad of [["ir.bogus"], "ir.core", [1], ["ir.core", "ir.bogus"]]) {
      expect(code(() => readOp("f", { params: [], ret: "Bool", requires: bad })), JSON.stringify(bad)).toBe("ill-typed");
    }
    expect(readOp("f", { params: [], ret: "Bool", requires: ["ir.fixed"] }).requires).toEqual(["ir.fixed"]);
    const part = clone(kitCase("v2-chunk/ragged-short-last-chunk").corpus);
    part.ops[Object.keys(part.ops)[0] as string].requires = ["ir.bogus"];
    expect(code(() => readCorpusPart(part, LEVELS))).toBe("corpus");
  });
});

describe("N5: a kit row that is broken is a harness error, not a vector", () => {
  const ir = (extra: Json): Json => ({ requires: ["ir.core"], levels: ["ir.core"], model_order: [], models: {}, ops: {}, subjects: {}, types: {}, env: [], ...extra });
  const op = { params: [["a", "U8"]], ret: "U8", requires: ["ir.core"] };
  const stand = (params: Json, body: Json): Json => ({ m: { params, body } });

  it("refuses an undetermined result type (R41, R98 (b))", () => {
    expect(() => runCase("v2-ir", ir({ ir: ["ok", U8(1)] }))).toThrow(/undetermined/);
    expect(() => runCase("v2-ir", ir({ ir: ["err", "BAD"], types: { E: { kind: "error", cases: ["BAD", "WORSE"] } } }))).toThrow(/undetermined/);
    // fixed from the context, it is a vector again
    expect(runCase("v2-ir", ir({ ir: ["eq", ["ok", U8(1)], ["var", "r"]], env: [["r", { type: "Result<U8,E>", value: { ok: "1" } }]], types: { E: { kind: "error", cases: ["BAD"] } } }))).toMatchObject({ value: true });
  });

  it("checks the stand-in of every reached op: present, live, and of the op's signature", () => {
    const run = (extra: Json): Json => runCase("v2-ir", ir({ ir: ["subject_op", "f", [U8(7)]], ops: { f: op }, ...extra }));
    expect(run({ models: stand([["a", "U8"]], ["var", "a"]), model_order: ["m"], subjects: { f: "m" } })).toMatchObject({ value: "7", calls: 1 });
    expect(() => run({})).toThrow(/no stand-in/);
    expect(() => run({ subjects: { f: "ghost" } })).toThrow(/not a live model/);
    expect(() => run({ models: stand([], ["lit", true]), model_order: ["m"], subjects: { f: "m" } })).toThrow(/number of arguments/);
    expect(() => run({ models: stand([["a", "U16"]], ["lit", true]), model_order: ["m"], subjects: { f: "m" } })).toThrow(/parameter differs/);
    expect(() => run({ models: stand([["a", "U8"]], ["lit", true]), model_order: ["m"], subjects: { f: "m" } })).toThrow(/result does not fit/);
    // an op the tree does not reach needs none
    expect(runCase("v2-ir", ir({ ir: U8(1), ops: { f: op } }))).toMatchObject({ value: "1" });
    // a prototype name is not a stand-in
    expect(() => run({ subjects: JSON.parse('{"__proto__": "m"}') })).toThrow(/no stand-in/);
  });
});

describe("N6: a missing level is named by the one rule (D63)", () => {
  it("names ir.list for List<Option<U8>>, ir.data for Option and ir.tuple for a tuple", () => {
    const table = readTypeTable({});
    const named = (text: string): string | undefined => {
      try {
        decodeJson(text.startsWith("(") ? ["1", []] : [], parseType(text), table);
      } catch (error) {
        return (error as KitError).level;
      }
      return undefined;
    };
    expect(named("List<Option<U8>>")).toBe("ir.list");
    expect(named("Option<U8>")).toBe("ir.data");
    // a tuple is held (S10f): its level is named only for what is inside it
    expect(named("(U8,Option<U8>)")).toBe("ir.data");
    expect(named("(U8,List<Option<U8>>)")).toBe("ir.list");
    expect(named("List<(U8,U8)>")).toBe("ir.list");
  });
});

describe("the closed run of the kit with the corrected harness", () => {
  it("runs every case of the six categories and none fails (1,452 of 1,452: S10f runs the 613 ir.fixed and ir.tuple cases S10d added)", () => {
    const report = runKit(KIT_DIR, { sequential: false });
    expect(report.failed).toBe(0);
    expect(report.run).toBe(1452);
  });
});
