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
import { readFileSync } from "node:fs";
import { join, posix, win32 } from "node:path";
import { describe, expect, it } from "vitest";
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

describe("fixture index", () => {
  it("matches the files on disk", () => {
    expect(index["schema"]).toBe("warrant-corpus-conformance-v1");
    const rows = new Map<string, Case>((index["files"] as Case[]).map((row) => [row["category"], row]));
    expect([...rows.keys()].sort()).toEqual([...CATEGORIES].sort());
    for (const [category, row] of rows) {
      expect(sha256(bytes(row["path"]))).toBe(row["sha256"]);
      expect(cases[category]?.length, `${category}: case count moved`).toBe(row["cases"]);
    }
    expect(index["total_cases"]).toBe(Object.values(cases).reduce((sum, list) => sum + list.length, 0));
  });
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
      const ty = parseType(item["type"]);
      let value: unknown;
      try {
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
