import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { LIMITS, listControlRows, loadCorpus } from "../src/corpus.ts";
import { CorpusError } from "../src/errors.ts";
import { CORPUS_DIR, REPO_ROOT, SPEC_PATH, copyCorpus, miniCorpus, realCorpus, tempDir } from "./helpers.ts";

type Doc = Record<string, any>; // eslint-disable-line @typescript-eslint/no-explicit-any

const corpus = realCorpus();

function rejects(code: string | null, tamper: { corpus?: (d: Doc) => void; tfv?: (t: string) => string; spec?: (b: Buffer) => Buffer }): void {
  const { root, corpusDir } = copyCorpus(tamper);
  try {
    loadCorpus(root, corpusDir);
  } catch (error) {
    expect(error).toBeInstanceOf(CorpusError);
    if (code !== null) expect((error as CorpusError).code).toBe(code);
    return;
  }
  throw new Error("the tampered corpus was accepted");
}

describe("the real corpus", () => {
  it("loads, with the Rust compiler's shape", () => {
    expect(corpus.module).toBe("ct_compare");
    expect([corpus.examples.length, corpus.suites.length, corpus.controls.length]).toEqual([12, 9, 8]);
    expect(corpus.suites.reduce((sum, suite) => sum + suite.cases, 0)).toBe(572_109);
    expect(corpus.errors.get("SelectError")).toEqual(["CT_SELECT_LENGTH_MISMATCH"]);
    expect(corpus.ops.get("eq_fixed")?.widthParams).toEqual(["N"]);
    expect(corpus.ops.get("select_bytes")?.fresh).toBe(true);
    expect(corpus.corpusSha256).toMatch(/^[0-9a-f]{64}$/);
    expect(corpus.casesSha256).toMatch(/^[0-9a-f]{64}$/);
    expect(REPO_ROOT.length).toBeGreaterThan(0);
  });

  it("lists its control rows from corpus.json alone", () => {
    const rows = listControlRows(CORPUS_DIR);
    expect(rows).toHaveLength(8);
    expect(rows[0]).toBe("ignore-last-byte:eq");
  });

  it("refuses a controls field that is not a list of objects", () => {
    const dir = tempDir();
    writeFileSync(join(dir, "corpus.json"), '{"controls": 3}');
    expect(() => listControlRows(dir)).toThrow(CorpusError);
    writeFileSync(join(dir, "corpus.json"), '{"controls": [3]}');
    expect(() => listControlRows(dir)).toThrow(CorpusError);
  });

  it("loads an untouched copy", () => {
    const { root, corpusDir } = copyCorpus();
    expect(loadCorpus(root, corpusDir).module).toBe("ct_compare");
  });

  it("points at a spec that is on disk", () => {
    expect(SPEC_PATH.endsWith("ct-compare.warrant")).toBe(true);
  });
});

describe("tampered input files", () => {
  it("a changed spec is stale evidence", () => {
    rejects("stale-evidence", { spec: (bytes) => Buffer.concat([bytes, Buffer.from("\n# edit\n")]) });
  });

  it("refuses unreadable files", () => {
    const { root, corpusDir } = copyCorpus();
    rmSync(join(corpusDir, "cases.tfv"));
    expect(() => loadCorpus(root, corpusDir)).toThrow(/io/);
    mkdirSync(join(corpusDir, "cases.tfv"));
    expect(() => loadCorpus(root, corpusDir)).toThrow(/regular/);
  });

  it("refuses malformed JSON", () => {
    const { root, corpusDir } = copyCorpus();
    for (const bytes of [Buffer.from("{"), Buffer.from('{"a": 1, "a": 2}'), Buffer.from("NaN"), Buffer.from([0xff])]) {
      writeFileSync(join(corpusDir, "corpus.json"), bytes);
      expect(() => loadCorpus(root, corpusDir)).toThrow(CorpusError);
    }
  });

  it("turns a field of the wrong shape into a refusal, not a crash", () => {
    rejects("schema", { corpus: (d) => (d["ops"]["eq"]["params"] = [null]) });
    rejects(null, { corpus: (d) => (d["suites"][0]["binders"][0] = null) });
  });
});

describe("the closed top-level shape", () => {
  it("rejects every deviation", () => {
    for (const change of [
      (d: Doc) => (d["extra"] = 1),
      (d: Doc) => delete d["controls"],
      (d: Doc) => (d["profile"] = "warrant-corpus-v2"),
      (d: Doc) => (d["schema_version"] = 2),
      (d: Doc) => (d["module"] = "Bad-Module"),
      (d: Doc) => (d["spec"] = "../escape.warrant"),
      (d: Doc) => (d["spec"] = "a\\b"),
      (d: Doc) => (d["spec"] = "/etc/passwd"),
      (d: Doc) => (d["spec"] = "a//b"),
      (d: Doc) => (d["spec_sha256"] = "nothex"),
      (d: Doc) => (d["types"] = []),
      (d: Doc) => (d["types"] = { lower: ["X"] }),
      (d: Doc) => (d["types"] = { Good: [1] }),
      (d: Doc) => (d["models"] = []),
      (d: Doc) => (d["examples"] = {}),
      (d: Doc) => (d["suites"] = []),
      (d: Doc) => (d["controls"] = {}),
    ]) {
      rejects("schema", { corpus: change });
    }
  });
});

describe("operation signatures", () => {
  it("are closed", () => {
    for (const change of [
      (d: Doc) => (d["ops"] = {}),
      (d: Doc) => (d["ops"]["eq"]["extra"] = 1),
      (d: Doc) => (d["ops"]["eq"]["params"] = [["a"]]),
      (d: Doc) => (d["ops"]["eq"]["params"] = "a"),
      (d: Doc) => (d["ops"]["eq"]["ret"] = "Float"),
      (d: Doc) => (d["ops"]["eq"]["width_params"] = "N"),
      (d: Doc) => (d["ops"]["BadName"] = d["ops"]["eq"]),
      (d: Doc) => (d["ops"]["eq"] = []),
    ]) {
      rejects(null, { corpus: change });
    }
  });
});

describe("examples", () => {
  it("are checked", () => {
    rejects("schema", { corpus: (d) => (d["examples"][0]["extra"] = 1) });
    rejects("schema", { corpus: (d) => (d["examples"][0]["id"] = "other/id") });
    rejects("example", { corpus: (d) => (d["examples"][0]["op"] = "ghost") });
    rejects("example", { corpus: (d) => (d["examples"][0]["args"] = []) });
    rejects("example", { corpus: (d) => (d["examples"][0]["result_type"] = "U8") });
    rejects("example", { corpus: (d) => (d["examples"][0]["width"] = 3) });
    rejects("example", { corpus: (d) => (d["examples"][4]["width"] = null) });
    rejects("example", { corpus: (d) => (d["examples"][4]["width"] = 9) });
    rejects("schema", { corpus: (d) => d["examples"].push(d["examples"][0]) });
  });
});

describe("suites", () => {
  it("are checked", () => {
    const suite = (d: Doc): Doc => d["suites"][0];
    rejects("schema", { corpus: (d) => (suite(d)["extra"] = 1) });
    rejects("schema", { corpus: (d) => (suite(d)["id"] = "warrant/ct_compare/exhaustive/other") });
    rejects("schema", { corpus: (d) => (suite(d)["ops"] = ["ghost"]) });
    rejects("schema", { corpus: (d) => (suite(d)["ops"] = []) });
    rejects("schema", { corpus: (d) => (suite(d)["binders"] = []) });
    rejects("schema", { corpus: (d) => (suite(d)["binders"] = [["a"]]) });
    rejects("schema", { corpus: (d) => (suite(d)["binders"] = Array.from({ length: 9 }, (_x, i) => [`b${i}`, { domain: "bool" }])) });
    rejects("schema", { corpus: (d) => (suite(d)["binders"] = [["a", { domain: "bool" }], ["a", { domain: "bool" }]]) });
    rejects("unsupported-domain", { corpus: (d) => (suite(d)["binders"] = [["a", { domain: "nope" }]]) });
    rejects("unsupported-type", { corpus: (d) => (suite(d)["result_type"] = "List<U8>") });
    rejects("schema", { corpus: (d) => (suite(d)["cases"] = -1) });
    rejects(null, { corpus: (d) => (suite(d)["cases"] = 1.5) });
    rejects("schema", { corpus: (d) => (suite(d)["inputs_sha256"] = "short") });
    rejects("schema", { corpus: (d) => (suite(d)["domain_text"] = "") });
    rejects("schema", { corpus: (d) => d["suites"].push(suite(d)) });
  });
});

describe("bounded work", () => {
  const original = { ...LIMITS };
  afterEach(() => Object.assign(LIMITS, original));
  const target = "eq_fixed_agrees_w2";
  const row = (d: Doc): Doc => d["suites"].find((s: Doc) => s["claim"] === target);

  it("a product that disagrees with cases + discarded is refused", () => {
    for (const [field, value] of [["cases", 255], ["discarded", 1], ["cases", 1_000_000_000]] as const) {
      rejects("work", { corpus: (d) => (row(d)[field] = value) });
    }
  });

  it("a suite that admits no case is refused", () => {
    rejects("schema", { corpus: (d) => { row(d)["cases"] = 0; row(d)["discarded"] = 256; } });
  });

  it("a suite over the per-suite cap is refused", () => {
    const { root, corpusDir } = miniCorpus();
    LIMITS.maxSuiteProduct = 100;
    expect(() => loadCorpus(root, corpusDir)).toThrow(/work/);
  });

  it("a product too large to be exact is refused before it is multiplied further", () => {
    rejects("work", {
      corpus: (d) => (row(d)["binders"] = Array.from({ length: 8 }, (_x, i) => [`b${i}`, { domain: "bytes_n(4096)" }])),
    });
  });

  it("total work including control re-runs is capped", () => {
    const { root, corpusDir } = miniCorpus();
    LIMITS.maxTotalWork = 1000;
    expect(() => loadCorpus(root, corpusDir)).toThrow(/total work/);
  });
});

describe("controls are re-derived from the faults", () => {
  it("rejects every deviation", () => {
    rejects("control", { corpus: (d) => d["controls"].pop() });
    rejects("control", { corpus: (d) => (d["controls"][0]["required_claims"] = []) });
    rejects("control", { corpus: (d) => d["controls"].find((c: Doc) => c["op"] === "eq_fixed")["required_claims"].reverse() });
    rejects("control", { corpus: (d) => (d["controls"][0]["fault"] = "invert") });
    rejects("control", { corpus: (d) => d["controls"].reverse() });
    rejects("schema", { corpus: (d) => (d["controls"][0]["extra"] = 1) });
    rejects("schema", { corpus: (d) => (d["controls"][0]["required_claims"] = [1]) });
    rejects("schema", { corpus: (d) => (d["faults"] = []) });
    rejects("schema", { corpus: (d) => (d["faults"][0]["extra"] = 1) });
    rejects("fault", { corpus: (d) => (d["faults"][0]["fault"] = "no-such-fault") });
    rejects("fault", { corpus: (d) => (d["faults"][0]["op"] = "ghost") });
    rejects("fault", { corpus: (d) => (d["faults"][0]["fault"] = "swap-choice") });
    rejects("fault", { corpus: (d) => (d["faults"][0]["applies"] = "sideways") });
    rejects("fault", { corpus: (d) => d["faults"].push({ ...d["faults"][0] }) });
  });

  it("refuses a fault no claim can catch", () => {
    rejects("fault", {
      corpus: (d) => {
        for (const suite of d["suites"]) if (suite["ops"][0] === "eq_u64") suite["ops"] = ["eq"];
      },
    });
  });
});

describe("cases.tfv must agree with the examples", () => {
  it("rejects every deviation", () => {
    for (const change of [
      (t: string) => t.replace("Kexpect T", "Kexpect F"),
      (t: string) => t.split("\n").slice(0, -2).join("\n") + "\n",
      (t: string) => t.replace("eq_empty", "eq_other"),
      (t: string) => t.replace("Kop S6571", "Kop S6572"),
      (t: string) => t.replace("Ka X00 Kb X00", "Ka X01 Kb X00"),
      (t: string) => t.replace("Kwidth I0", "Kwidth I1"),
      (t: string) => t.replace("R4 Kop S6571 Ka X Kb X ", "R4 Kop S6571 Ka X Kc X "),
      (t: string) => t.replace("Kop S6571", "Kop N"),
      (t: string) => t.replace("Kexpect T", "Kexpect I1"),
      (t: string) => t.replace(" Kwidth I0", ""),
    ]) {
      rejects(null, { tfv: change });
    }
  });

  it("refuses a non-UTF-8 cases file", () => {
    const { root, corpusDir } = copyCorpus();
    writeFileSync(join(corpusDir, "cases.tfv"), Buffer.from([0x74, 0xff]));
    expect(() => loadCorpus(root, corpusDir)).toThrow(CorpusError);
  });
});

describe("fixtures are copied as bytes", () => {
  it("keeps LF line ends exactly, on every platform", () => {
    const { corpusDir } = copyCorpus();
    const original = readFileSync(join(CORPUS_DIR, "cases.tfv"));
    const copied = readFileSync(join(corpusDir, "cases.tfv"));
    expect(copied.equals(original)).toBe(true);
    expect(copied.includes(0x0d)).toBe(false);
    expect(readFileSync(join(corpusDir, "consumers.json")).equals(readFileSync(join(CORPUS_DIR, "consumers.json")))).toBe(true);
    expect(readFileSync(join(corpusDir, "..", "..", "ct-compare.warrant")).equals(readFileSync(SPEC_PATH))).toBe(true);
  });

  it("refuses a CRLF cases.tfv instead of tolerating it", () => {
    rejects(null, { tfv: (text) => text.replaceAll("\n", "\r\n") });
  });
});
