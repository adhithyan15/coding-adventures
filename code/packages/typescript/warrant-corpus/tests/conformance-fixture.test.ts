/**
 * Run every `warrant-corpus-conformance-v1` case against this lane (WD-100).
 *
 * The fixture (`code/specs/fixtures/warrant-corpus-conformance-v1`, VER04i 9.1)
 * holds the behaviours every lane's runner re-discovered in review: strict
 * JSON, canonical output, tagged values, path containment and the label digest.
 * Each block below runs one category through *this lane's own* functions and
 * compares with the outcome stored in the fixture. Failure messages name the
 * case id and never print the input bytes.
 */
import { createHash } from "node:crypto";
import { cpSync, existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, posix, win32 } from "node:path";
import { afterAll, describe, expect, it } from "vitest";
import { relativeInside } from "../src/containment.ts";
import { CorpusError } from "../src/errors.ts";
import { parseJson, renderJson } from "../src/jsonio.ts";
import { manifestLabelDigest } from "../src/labels.ts";
import { StreamHasher, decodeJson, fromHex, parseType } from "../src/values.ts";
import { REPO_ROOT } from "./helpers.ts";


const FIXTURE = join(REPO_ROOT, "code", "specs", "fixtures", "warrant-corpus-conformance-v1");
const CATEGORIES = ["json", "tagged", "containment", "label-digest"] as const;

type Case = Record<string, any>; // eslint-disable-line @typescript-eslint/no-explicit-any

const bytes = (name: string): Uint8Array => new Uint8Array(readFileSync(join(FIXTURE, name)));
// The fixture is read with this lane's own strict reader.
const load = (name: string): Case => parseJson(bytes(name)) as Case;
const sha256 = (data: Uint8Array | string): string => createHash("sha256").update(data).digest("hex");

const index = load("index.json");
const cases: Record<string, Case[]> = Object.fromEntries(
  CATEGORIES.map((category) => [category, load(`${category}.json`)["cases"] as Case[]]),
);

// The fixture family grows by categories (VER04r section 11, [D40]). Every
// category this lane knows must have a row; a row it does not know is accepted
// only when its name starts with `v2-`, and then only digest-checked, never run
// (this runner does not read v2 yet). Rows are ordered: the four v1 rows in
// their present order, then the v2 rows, the known ones in the kit's order.
const V2_ORDER = ["v2-value", "v2-arith", "v2-ir", "v2-domain", "v2-chunk", "v2-fault", "v2-binding"];

/** Validate `dir/index.json`; throws an `Error` with a clear message on any violation, a malformed row included. */
function checkIndex(dir: string): number {
  const read = (name: string): Uint8Array => new Uint8Array(readFileSync(join(dir, name)));
  const document = parseJson(read("index.json")) as Case;
  if (document["schema"] !== "warrant-corpus-conformance-v1") throw new Error("wrong index schema");
  const files = document["files"] as Case[];
  if (!Array.isArray(files) || files.some((row) => typeof row !== "object" || row === null)) {
    throw new Error("malformed files list");
  }
  const names = files.map((row) => row["category"] as string);
  if (names.some((name) => typeof name !== "string")) throw new Error("malformed row: category is not a string");
  if (new Set(names).size !== names.length) throw new Error("duplicate category row");
  if (JSON.stringify(names.slice(0, CATEGORIES.length)) !== JSON.stringify(CATEGORIES)) {
    throw new Error("v1 rows missing or out of order");
  }
  const extra = names.slice(CATEGORIES.length);
  for (const name of extra) if (!name.startsWith("v2-")) throw new Error(`unknown category row ${name}`);
  const known = extra.filter((name) => V2_ORDER.includes(name)).map((name) => V2_ORDER.indexOf(name));
  if (known.some((value, i) => i > 0 && value < (known[i - 1] as number))) throw new Error("v2 rows out of order");
  let total = 0;
  for (const row of files) {
    const category = row["category"] as string;
    const path = row["path"] as string;
    // The category name is safe to splice into a file name: non-empty, no
    // separators and no `..` (known v1 names and `v2-` names were checked above).
    if (category === "" || /[\\/]/.test(category) || category.includes("..")) {
      throw new Error(`${JSON.stringify(category)}: unsafe category name`);
    }
    if (path !== `${category}.json`) throw new Error(`${category}: bad path`);
    if (!existsSync(join(dir, path))) throw new Error(`${category}: file missing`);
    const data = read(path);
    if (sha256(data) !== row["sha256"]) throw new Error(`${category}: sha256 moved`);
    const body = parseJson(data) as Case;
    if (body["category"] !== category) throw new Error(`${category}: wrong category in file`);
    if (!Number.isInteger(row["cases"])) throw new Error(`${category}: cases is not an integer`);
    if (!Array.isArray(body["cases"])) throw new Error(`${category}: file cases is not a list`);
    if ((body["cases"] as Case[]).length !== row["cases"]) throw new Error(`${category}: case count moved`);
    total += row["cases"] as number;
  }
  if (!Number.isInteger(document["total_cases"])) throw new Error("total_cases is not an integer");
  if (document["total_cases"] !== total) throw new Error("total_cases is not the sum of the rows");
  return files.length;
}

/**
 * A temp copy of `base` with extra index rows and files; the caller removes it.
 * Any `v2-*` rows (and their files) the base index already holds are stripped
 * first, so the tests do not depend on how many v2 rows are committed ([D59]).
 */
function scratchFixture(extraRows: Case[], files: Record<string, Buffer>, base: string = FIXTURE): string {
  const scratch = mkdtempSync(join(tmpdir(), "warrant-fixture-"));
  cpSync(base, scratch, { recursive: true });
  const document = JSON.parse(readFileSync(join(scratch, "index.json"), "utf8"));
  for (const row of (document.files as Case[]).filter((r) => String(r["category"]).startsWith("v2-"))) {
    rmSync(join(scratch, row["path"]), { force: true });
    document.files.splice(document.files.indexOf(row), 1);
    document.total_cases -= row["cases"];
  }
  document.files.push(...extraRows);
  document.total_cases += extraRows.reduce((sum, row) => sum + row["cases"], 0);
  writeFileSync(join(scratch, "index.json"), `${JSON.stringify(document, null, 2)}\n`);
  for (const [name, data] of Object.entries(files)) writeFileSync(join(scratch, name), data);
  return scratch;
}

function extraFile(category: string, count = 2): [Case, Buffer] {
  const cases = Array.from({ length: count }, (_, i) => ({ id: `${category}/${i}` }));
  const data = Buffer.from(`${JSON.stringify({ schema: "warrant-corpus-conformance-v1", category, cases }, null, 2)}\n`);
  return [{ cases: count, category, path: `${category}.json`, sha256: sha256(data) }, data];
}

/** Run `check` against a scratch fixture and always remove the scratch copy. */
function withScratch<T>(rows: Case[], files: Record<string, Buffer>, check: (dir: string) => T, base: string = FIXTURE): T {
  const dir = scratchFixture(rows, files, base);
  try {
    return check(dir);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

/**
 * The index-rule tests, run against `base`: the committed fixture, and again
 * (below) against a simulated committed index that already holds v2 rows.
 */
function indexRuleTests(base: string): void {
  const scratch = <T>(rows: Case[], files: Record<string, Buffer>, check: (dir: string) => T): T =>
    withScratch(rows, files, check, base);
  const noRows = withScratch([], {}, (dir) => checkIndex(dir), base);

  it("accepts a digest-checked v2-x row without running it", () => {
    const [row, data] = extraFile("v2-x");
    expect(noRows).toBe(CATEGORIES.length);
    expect(scratch([row], { "v2-x.json": data }, checkIndex)).toBe(noRows + 1);
  });

  it("rejects an unknown row that is not v2-", () => {
    const [row, data] = extraFile("extra");
    expect(() => scratch([row], { "extra.json": data }, checkIndex)).toThrow(/unknown category/);
  });

  it("rejects a v2 row with a wrong digest", () => {
    const [row, data] = extraFile("v2-x");
    row["sha256"] = "0".repeat(64);
    expect(() => scratch([row], { "v2-x.json": data }, checkIndex)).toThrow(/sha256/);
  });

  it("rejects a v2 row with a wrong case count", () => {
    const [row, data] = extraFile("v2-x");
    row["cases"] = 3;
    expect(() => scratch([row], { "v2-x.json": data }, checkIndex)).toThrow(/case count/);
  });

  it("rejects a row whose path is not <category>.json", () => {
    const [row, data] = extraFile("v2-x");
    row["path"] = "v2-y.json";
    expect(() => scratch([row], { "v2-x.json": data, "v2-y.json": data }, checkIndex)).toThrow(/bad path/);
  });

  it("rejects a boolean case count", () => {
    const [row, data] = extraFile("v2-x", 1);
    row["cases"] = true;
    expect(() => scratch([row], { "v2-x.json": data }, checkIndex)).toThrow(/not an integer/);
  });

  it("rejects a v2 row with a missing file", () => {
    const [row] = extraFile("v2-x");
    expect(() => scratch([row], {}, checkIndex)).toThrow(/file missing/);
  });

  it("rejects a missing v1 row, removed by name", () => {
    const dir = scratchFixture([], {}, base);
    try {
      const document = JSON.parse(readFileSync(join(dir, "index.json"), "utf8"));
      document.files = document.files.filter((row: Case) => row["category"] !== CATEGORIES[CATEGORIES.length - 1]);
      writeFileSync(join(dir, "index.json"), JSON.stringify(document));
      expect(() => checkIndex(dir)).toThrow(/v1 rows/);
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  it("rejects v2 rows out of the kit's order", () => {
    const [value, valueData] = extraFile("v2-value");
    const [arith, arithData] = extraFile("v2-arith");
    const files = { "v2-value.json": valueData, "v2-arith.json": arithData };
    expect(scratch([value, arith], files, checkIndex)).toBe(noRows + 2);
    expect(() => scratch([arith, value], files, checkIndex)).toThrow(/out of order/);
  });
}

describe("fixture index", () => {
  it("matches the files on disk", () => {
    expect(checkIndex(FIXTURE)).toBeGreaterThanOrEqual(CATEGORIES.length);
    const rows = new Map<string, Case>((index["files"] as Case[]).map((row) => [row["category"], row]));
    for (const category of CATEGORIES) {
      expect(cases[category]?.length, `${category}: case count moved`).toBe(rows.get(category)?.["cases"]);
    }
  });

  indexRuleTests(FIXTURE);
});

// S5-1b ([D59]): the same tests, against a simulated committed index that
// already holds v2-value and v2-arith rows, as it will once the real v2 rows land.
describe("fixture index with v2 rows already committed", () => {
  const [value, valueData] = extraFile("v2-value", 3);
  const [arith, arithData] = extraFile("v2-arith", 5);
  const committed = scratchFixture([value, arith], { "v2-value.json": valueData, "v2-arith.json": arithData });

  afterAll(() => rmSync(committed, { recursive: true, force: true }));

  it("is itself a valid index with the two v2 rows", () => {
    expect(checkIndex(committed)).toBe(CATEGORIES.length + 2);
  });

  indexRuleTests(committed);
});

describe("json", () => {
  for (const item of cases["json"] as Case[]) {
    it(item["id"], () => {
      const data = fromHex(item["input_hex"]);
      const expected = item["expect"];
      if (expected["outcome"] === "reject") {
        let code: string | undefined;
        try {
          parseJson(data);
        } catch (error) {
          expect(error).toBeInstanceOf(CorpusError);
          code = (error as CorpusError).code;
        }
        expect(code, item["id"]).toBe(expected["error"]);
        return;
      }
      const text = renderJson(parseJson(data));
      expect(sha256(text), item["id"]).toBe(expected["canonical_sha256"]);
      if ("canonical" in expected) expect(text, item["id"]).toBe(expected["canonical"]);
    });
  }
});

describe("tagged", () => {
  for (const item of cases["tagged"] as Case[]) {
    it(item["id"], () => {
      const expected = item["expect"];
      const errors = new Map<string, string[]>(Object.entries(item["errors"] ?? {}));
      let ty!: ReturnType<typeof parseType>;
      let value: unknown;
      try {
        ty = parseType(item["type"]);
        value = decodeJson(parseJson(fromHex(item["input_hex"])), ty, errors);
      } catch (error) {
        expect(expected["outcome"], item["id"]).toBe("reject");
        expect((error as CorpusError).code, item["id"]).toBe(expected["error"]);
        return;
      }
      expect(expected["outcome"], item["id"]).toBe("accept");
      // The hasher is the lane's stream encoder: an empty prefix leaves the
      // digest of exactly the encoded value bytes.
      const hasher = new StreamHasher(new Uint8Array(0));
      hasher.write(value, ty);
      expect(hasher.hexdigest(), item["id"]).toBe(expected["stream_sha256"]);
    });
  }
});

describe("containment", () => {
  for (const item of cases["containment"] as Case[]) {
    it(item["id"], () => {
      const mod = item["flavor"] === "win32" ? win32 : posix;
      const verdict = relativeInside(item["root"], item["child"], mod);
      expect(verdict !== null, item["id"]).toBe(item["expect"]["inside"]);
      expect(verdict === "", item["id"]).toBe(item["expect"]["same"]);
    });
  }
});

describe("label-digest", () => {
  const digests = new Map<string, string>(
    (cases["label-digest"] as Case[]).map((item) => [item["id"], manifestLabelDigest(item["manifest"])]),
  );
  for (const item of cases["label-digest"] as Case[]) {
    it(item["id"], () => {
      expect(digests.get(item["id"])).toBe(item["sha256"]);
      if ("same_as" in item) expect(digests.get(item["id"])).toBe(digests.get(item["same_as"]));
      if ("differs_from" in item) expect(digests.get(item["id"])).not.toBe(digests.get(item["differs_from"]));
    });
  }
});
