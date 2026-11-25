/**
 * The four committed v2 corpora, read by this lane at `ir.core` (VER04r
 * sections 4.1, 9.1, 14; [D64], [D65]).
 *
 * A lane at `ir.core` still runs the `ir.core` suites of a corpus that also
 * holds suites of other levels, records the rest `not-live` by what they
 * reach, compiles every `ir.core` fault row, and writes no row for a model or
 * an op.
 */
import { cpSync, readFileSync, readdirSync, symlinkSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { parseTfv } from "../../src/tfv.ts";
import { KitError } from "../../src/v2/errors.ts";
import { MAX_IR_WORK_SLIM, MAX_TOTAL_WORK_SLIM, checkWork, chunkFileName, loadV2Corpus, runSuite } from "../../src/v2/corpus.ts";
import { type OpSig, readOp } from "../../src/v2/ir.ts";
import { parseType, readTypeTable, typeLevels } from "../../src/v2/types.ts";
import { decodeJson, decodeTfv } from "../../src/v2/values.ts";
import { CORPORA, tempDir } from "./support.ts";

// name: [live suites, not-live suites, ok controls, not-live controls, cases of the live suites]
const EXPECTED: Record<string, [number, number, number, number, number]> = {
  "ct-compare": [9, 0, 7, 1, 572_109],
  x25519: [6, 2, 2, 3, 10_500],
  lzw: [4, 17, 2, 3, 121_168],
  "wasm-leb128": [4, 40, 2, 18, 2_000],
};

describe.each(CORPORA)("$name-warrant-v2 at ir.core", ({ name, dir }) => {
  const corpus = loadV2Corpus(dir);
  const [live, notLive, ok, notLiveControls, cases] = EXPECTED[name] as [number, number, number, number, number];

  it("classifies suites and controls by what they reach", () => {
    expect(corpus.suites.filter((s) => s.state === "live")).toHaveLength(live);
    expect(corpus.suites.filter((s) => s.state === "not-live")).toHaveLength(notLive);
    expect(corpus.controls.filter((c) => c.state === "ok")).toHaveLength(ok);
    expect(corpus.controls.filter((c) => c.state === "not-live")).toHaveLength(notLiveControls);
    // not_live counts suites plus controls (section 4.1).
    expect(corpus.notLive).toBe(notLive + notLiveControls);
  });

  it("reports every not-live unit with the level it lacks and no row for a model or an op [D65]", () => {
    for (const record of corpus.suites.filter((s) => s.state === "not-live")) expect(record.level).toMatch(/^ir\.(fixed|tuple|data)$/);
    for (const control of corpus.controls.filter((c) => c.state === "not-live")) expect(control.level).toMatch(/^ir\.(fixed|tuple|data)$/);
    // The loaded corpus has suites and controls; a model or an op is never a unit of the report.
    expect(Object.keys(corpus).sort()).toEqual(["controls", "dir", "levels", "module", "notLive", "part", "suites"]);
  });

  it("reproduces every chunk's cases, calls and digests of every live suite at full depth", () => {
    let total = 0;
    for (const record of corpus.suites.filter((s) => s.state === "live")) {
      const run = runSuite(corpus, record, "full");
      expect(run.chunks).toBe(record.suite?.chunks);
      expect(run.cases).toBe(record.suite?.cases);
      expect(run.calls).toBe(record.suite?.expectedCalls);
      total += run.cases;
    }
    expect(total).toBe(cases);
  }, 120_000);

  it("runs only the corpus-chosen slim chunks at slim depth", () => {
    for (const record of corpus.suites.filter((s) => s.state === "live")) {
      const run = runSuite(corpus, record, "slim");
      expect(run.chunks).toBe(record.suite?.slimChunks.length);
    }
  }, 120_000);

  it("compiles every ir.core fault row and records the others not-live with a level", () => {
    for (const control of corpus.controls) {
      if (control.state === "ok") expect(control.compiled?.fault).toBe(control.fault);
      else expect(control.compiled).toBeUndefined();
    }
  });

  it("refuses a suite it classified not-live", () => {
    const record = corpus.suites.find((s) => s.state === "not-live");
    if (record !== undefined) expect(() => runSuite(corpus, record)).toThrow(KitError);
  });

  it("reads cases.tfv examples to the same values as the JSON examples of corpus.json", () => {
    const raw = JSON.parse(readFileSync(join(dir, "corpus.json"), "utf8"));
    const records = new Map(parseTfv(readFileSync(join(dir, "cases.tfv"), "utf8")));
    const table = readTypeTable(raw.types);
    let compared = 0;
    for (const example of raw.examples as { id: string; op: string; args: unknown[]; expected: unknown; result_type: string }[]) {
      const record = records.get(example.id);
      const op: OpSig = readOp(example.op, raw.ops[example.op]);
      if (record === undefined) continue;
      // An example whose types need a level this lane lacks is not read (ir.tuple and ir.data are not implemented).
      const needed = new Set<string>();
      [...op.params.map((p) => p.ty), parseType(example.result_type)].forEach((ty) => typeLevels(ty, needed));
      if (corpus.part.program.lacks(needed) !== undefined) continue;
      op.params.forEach((param, i) => {
        const field = record.get(param.name);
        expect(field, `${example.id}.${param.name}`).toBeDefined();
        if (param.ty.k === "Bytes" && typeof param.ty.n === "string") return;
        const fromTfv = decodeTfv(field as never, param.ty, table);
        const fromJson = decodeJson(example.args[i], param.ty, table);
        expect(fromTfv).toEqual(fromJson);
      });
      const expectField = record.get("expect");
      const resultType = parseType(example.result_type);
      expect(decodeTfv(expectField as never, resultType, table)).toEqual(decodeJson(example.expected, resultType, table));
      compared += 1;
    }
    expect(compared > 0 || name !== "ct-compare").toBe(true);
  });
});

describe("loading a hostile corpus directory", () => {
  const copy = (): string => {
    const target = tempDir("warrant-corpus-v2-");
    const source = (CORPORA.find((c) => c.name === "ct-compare") as { dir: string }).dir;
    cpSync(source, target, { recursive: true });
    return target;
  };
  const corpusOf = (dir: string): Record<string, unknown> => JSON.parse(readFileSync(join(dir, "corpus.json"), "utf8"));
  const write = (dir: string, document: unknown): void => writeFileSync(join(dir, "corpus.json"), JSON.stringify(document));

  it("derives the chunk file name from the claim, refusing a Windows device name and a name over 133 bytes", () => {
    expect(chunkFileName("eq_agrees")).toBe("eq_agrees.txt");
    expect(chunkFileName("op/post")).toBe("op.post.txt");
    for (const claim of ["con", "CON/post", "op/NUL", "aux", "com1", "LPT9/x", "x/prn"]) expect(() => chunkFileName(claim), claim).toThrow(/device name/);
    expect(chunkFileName("connect")).toBe("connect.txt");
    expect(chunkFileName("com10")).toBe("com10.txt");
    expect(() => chunkFileName(`${"a".repeat(64)}/${"b".repeat(64)}`)).not.toThrow();
    expect(() => chunkFileName(`${"a".repeat(64)}/${"b".repeat(65)}`)).toThrow(/133/);
  });

  it("refuses a level name this document does not define, in levels and in a suite's requires", () => {
    const named = copy();
    write(named, { ...corpusOf(named), levels: ["ir.core", "ir.fancy"] });
    expect(() => loadV2Corpus(named)).toThrow(/levels names/);
    const notList = copy();
    write(notList, { ...corpusOf(notList), levels: "ir.core" });
    expect(() => loadV2Corpus(notList)).toThrow(/levels names/);
    const suite = copy();
    const document = corpusOf(suite) as { suites: Record<string, unknown>[] };
    document.suites[0] = { ...document.suites[0], requires: ["ir.fancy"] };
    write(suite, document);
    expect(() => loadV2Corpus(suite)).toThrow(/requires names/);
    const fault = copy();
    const withFault = corpusOf(fault) as { faults: Record<string, unknown>[] };
    withFault.faults[0] = { ...withFault.faults[0], requires: ["ir.fancy"] };
    write(fault, withFault);
    expect(() => loadV2Corpus(fault)).toThrow(/levels this document defines/);
  });

  it("bounds the total work before the first case (section 14)", () => {
    const corpus = loadV2Corpus(copy());
    const full = checkWork(corpus, "full");
    const slim = checkWork(corpus, "slim");
    expect(full.ordinals).toBe(corpus.suites.reduce((sum, s) => sum + (s.suite?.product ?? 0), 0));
    // A slim run executes each suite's slim chunk set once, plus once more for every suite an ok control re-runs (section 14).
    const slimOf = (s: (typeof corpus.suites)[number]): number =>
      (s.suite?.slimChunks ?? []).reduce((sum, c) => sum + Math.min(s.suite?.chunkSize ?? 0, (s.suite?.product ?? 0) - c * (s.suite?.chunkSize ?? 0)), 0);
    const live = corpus.suites.filter((s) => s.state === "live");
    const direct = live.reduce((sum, s) => sum + slimOf(s), 0);
    const rerun = corpus.controls
      .filter((c) => c.state === "ok")
      .flatMap((c) => c.requiredClaims)
      .reduce((sum, id) => sum + slimOf(live.find((s) => s.id === id) ?? { id, state: "live" }), 0);
    expect(rerun).toBeGreaterThan(0);
    expect(slim.ordinals).toBe(direct + rerun);
    expect(full.irWork).toBeGreaterThan(0);
    const heavy = { ...corpus, suites: [{ id: "x", state: "live" as const, suite: { ...(corpus.suites[0] as { suite: object }).suite, product: 2 ** 33, chunkSize: 65536, slimChunks: [0], irCost: 1 } }] };
    expect(() => checkWork(heavy as never, "full")).toThrow(/total work/);
    const costly = { ...corpus, suites: [{ id: "x", state: "live" as const, suite: { ...(corpus.suites[0] as { suite: object }).suite, product: 2 ** 28, chunkSize: 65536, slimChunks: [0, 1], irCost: 2 ** 40 } }] };
    expect(() => checkWork(costly as never, "slim")).toThrow(/IR work/);
    expect(MAX_TOTAL_WORK_SLIM).toBe(32_000_000);
    expect(MAX_IR_WORK_SLIM).toBe(64_000_000_000);
  });

  it("refuses an unknown key, a wrong profile and a stale profile digest", () => {
    const withKey = copy();
    write(withKey, { ...corpusOf(withKey), surprise: 1 });
    expect(() => loadV2Corpus(withKey)).toThrow(/unknown key/);

    const wrongProfile = copy();
    write(wrongProfile, { ...corpusOf(wrongProfile), profile: "warrant-corpus-v1" });
    expect(() => loadV2Corpus(wrongProfile)).toThrow(/not a warrant-corpus-v2/);

    const stale = copy();
    writeFileSync(join(stale, "corpus.profile"), `${readFileSync(join(stale, "corpus.profile"), "utf8")}fault eq invert\n`);
    expect(() => loadV2Corpus(stale)).toThrow(/profile_sha256/);

    const notObject = copy();
    write(notObject, []);
    expect(() => loadV2Corpus(notObject)).toThrow(/object/);
  });

  it("refuses an undeclared file in chunks/ and a symbolic link", () => {
    const extra = copy();
    writeFileSync(join(extra, "chunks", "stray.txt"), "x\n");
    expect(() => loadV2Corpus(extra)).toThrow(/undeclared file/);

    const linked = copy();
    const claim = readdirSync(join(linked, "chunks"))[0] as string;
    try {
      symlinkSync(join(linked, "cases.tfv"), join(linked, "chunks", `${claim}.link`));
    } catch {
      return; // a platform that cannot create symbolic links has nothing to refuse
    }
    expect(() => loadV2Corpus(linked)).toThrow();
  });

  it("refuses a tampered chunk file when the suite is run", () => {
    const dir = copy();
    const path = join(dir, "chunks", "eq_agrees.txt");
    const text = readFileSync(path, "utf8");
    writeFileSync(path, text.replace(/[0-9a-f]/, (c) => (c === "0" ? "1" : "0")));
    const corpus = loadV2Corpus(dir);
    const record = corpus.suites.find((s) => s.suite?.claim === "eq_agrees");
    expect(() => runSuite(corpus, record as never)).toThrow(/chunk-file/);
  });

  it("refuses a duplicate suite id and two claims with one chunk file name ignoring case", () => {
    const duplicate = copy();
    const document = corpusOf(duplicate) as { suites: Record<string, unknown>[] };
    document.suites[1] = { ...document.suites[1], id: document.suites[0]?.["id"] };
    write(duplicate, document);
    expect(() => loadV2Corpus(duplicate)).toThrow(/appears twice/);

    const folded = copy();
    const second = corpusOf(folded) as { suites: Record<string, unknown>[] };
    second.suites[1] = { ...second.suites[1], claim: String(second.suites[0]?.["claim"]).toUpperCase(), id: "warrant/x/exhaustive/other" };
    write(folded, second);
    expect(() => loadV2Corpus(folded)).toThrow(/ignoring case/);
  });

  it("treats a row whose requires disagrees with what it reaches as a corpus error (R69 e), and the opposite as not-live", () => {
    const dir = copy();
    const document = corpusOf(dir) as { suites: Record<string, unknown>[] };
    // eq_u64_agrees really is ir.core: declaring it ir.fixed makes it not-live, never a corpus error.
    const row = document.suites.find((s) => s["claim"] === "eq_u64_agrees") as Record<string, unknown>;
    row["requires"] = ["ir.core", "ir.fixed"];
    write(dir, document);
    const corpus = loadV2Corpus(dir);
    expect(corpus.suites.find((s) => s.id === row["id"])).toMatchObject({ state: "not-live", level: "ir.fixed" });

    // A row that says ir.core but reaches ir.fixed is wrong: the generator said the lane could run it.
    const wrong = copy();
    const other = corpusOf(wrong) as { suites: Record<string, unknown>[] };
    const victim = other.suites.find((s) => s["claim"] === "eq_agrees") as Record<string, unknown>;
    victim["body"] = ["eq", ["bit_not", ["lit", { type: "U8", value: "1" }]], ["lit", { type: "U8", value: "254" }]];
    write(wrong, other);
    expect(() => loadV2Corpus(wrong)).toThrow(/disagrees/);
  });

  it("refuses a chunk file that is consistent but is not what the suite's expressions enumerate", () => {
    const dir = copy();
    const document = corpusOf(dir) as { suites: Record<string, any>[] }; // eslint-disable-line @typescript-eslint/no-explicit-any
    const original = loadV2Corpus(dir);
    const victim = original.suites.find((s) => s.suite?.claim === "eq_agrees")?.suite;
    const row = document.suites.find((s) => s["claim"] === "eq_agrees") as Record<string, any>; // eslint-disable-line @typescript-eslint/no-explicit-any
    // an expected expression of cost 1 (a literal) in place of the model call: the row and file are untouched
    const was = JSON.stringify(row["oracle"]["expected"]);
    row["oracle"]["expected"] = ["lit", true];
    // the body *is* subject == expected (section 4.3), so it names the new expected too, and its cost counts twice
    row["body"] = row["body"].map((side: unknown) => (JSON.stringify(side) === was ? ["lit", true] : side));
    row["ir_cost"] -= 2 * ((victim?.expected?.cost ?? 1) - 1);
    write(dir, document);
    const corpus = loadV2Corpus(dir);
    expect(() => runSuite(corpus, corpus.suites.find((s) => s.suite?.claim === "eq_agrees") as never)).toThrow(/chunk differs/);
    expect(() => runSuite(corpus, corpus.suites.find((s) => s.suite?.claim === "eq_agrees") as never, "slim")).toThrow(/chunk differs/);
  });

  it("refuses an unchunked v1 digest that the walk does not reproduce", () => {
    const dir = copy();
    const document = corpusOf(dir) as { suites: Record<string, unknown>[] };
    const row = document.suites.find((s) => s["claim"] === "eq_agrees") as Record<string, unknown>;
    row["v1_inputs_sha256"] = "0".repeat(64);
    write(dir, document);
    const corpus = loadV2Corpus(dir);
    expect(() => runSuite(corpus, corpus.suites.find((s) => s.suite?.claim === "eq_agrees") as never)).toThrow(/unchunked/);
  });

  it("refuses a fault row for an op that does not exist and a control with no fault", () => {
    const noOp = copy();
    const a = corpusOf(noOp) as { faults: Record<string, unknown>[] };
    a.faults[0] = { ...a.faults[0], op: "nope" };
    write(noOp, a);
    expect(() => loadV2Corpus(noOp)).toThrow(/not in ops/);

    const noFault = copy();
    const b = corpusOf(noFault) as { faults: unknown[] };
    b.faults.pop();
    write(noFault, b);
    expect(() => loadV2Corpus(noFault)).toThrow(/no fault row/);

    const malformed = copy();
    const c = corpusOf(malformed) as { faults: unknown[]; controls: unknown[] };
    c.faults[0] = 7;
    write(malformed, c);
    expect(() => loadV2Corpus(malformed)).toThrow(/names its op/);
  });
});
