import { mkdirSync, symlinkSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { beforeAll, describe, expect, it } from "vitest";
import { CorpusError } from "../src/errors.ts";
import { sha256Hex } from "../src/jsonio.ts";
import { deriveLabels, domainLabel, NOTE } from "../src/labels.ts";
import {
  MAX_SOURCE_DEPTH,
  MAX_SOURCE_FILE_BYTES,
  frameSources,
  readSourceTree,
  runnerDigest,
  sourceDigest,
  validateManifest,
} from "../src/manifest.ts";
import { execute, prepare } from "../src/session.ts";
import { CORPUS_DIR, REPO_ROOT, newLibrary, realCorpus, tempDir } from "./helpers.ts";

const corpus = realCorpus();
const enc = (text: string): Uint8Array => new TextEncoder().encode(text);
const u64 = (n: number): number[] => [n, 0, 0, 0, 0, 0, 0, 0];

describe("warrant-src-v1 framing", () => {
  it("has the specified layout, built by hand", () => {
    const framed = frameSources([["b.ts", enc("22")], ["a.ts", enc("1")]]);
    expect(Array.from(framed)).toEqual([
      ...Array.from(enc("warrant-src-v1\0")),
      ...u64(2),
      ...u64(4), ...Array.from(enc("a.ts")), ...u64(1), ...Array.from(enc("1")),
      ...u64(4), ...Array.from(enc("b.ts")), ...u64(2), ...Array.from(enc("22")),
    ]); // prettier-ignore
    expect(Array.from(frameSources([]))).toEqual([...Array.from(enc("warrant-src-v1\0")), ...u64(0)]);
  });

  it("cannot confuse a path with content", () => {
    expect(sha256Hex(frameSources([["a.tsb", enc("")]]))).not.toBe(sha256Hex(frameSources([["a.ts", enc("b")]])));
    expect(sha256Hex(frameSources([["a.ts", enc("x")], ["b.ts", enc("")]]))).not.toBe(sha256Hex(frameSources([["a.ts", enc("")], ["b.ts", enc("x")]])));
  });
});

function tree(files: Record<string, string>): string {
  const root = tempDir();
  for (const [path, text] of Object.entries(files)) {
    mkdirSync(dirname(join(root, path)), { recursive: true });
    writeFileSync(join(root, path), text);
  }
  return root;
}

describe("sourceDigest", () => {
  it("selects production TypeScript only", () => {
    const base = { "index.ts": "a", "sub/util.ts": "b" };
    const root = tree({
      ...base,
      "index.test.ts": "test",
      "sub/util.test.ts": "test",
      "dist/index.ts": "built",
      "node_modules/x/index.ts": "foreign",
      "readme.md": "doc",
      "data.json": "{}",
      "script.js": "js",
    });
    expect(readSourceTree(root).map(([path]) => path)).toEqual(["index.ts", "sub/util.ts"]);
    expect(sourceDigest(root)).toBe(sourceDigest(tree(base)));
    expect(sourceDigest(tree({ ...base, "index.ts": "changed" }))).not.toBe(sourceDigest(root));
  });

  it("sorts by UTF-8 bytes, so file-system order never matters", () => {
    const one = tree({ "b.ts": "1", "a.ts": "2", "é.ts": "3" });
    expect(readSourceTree(one).map(([path]) => path)).toEqual(["a.ts", "b.ts", "é.ts"]);
  });

  it("refuses a root that is not a directory", () => {
    expect(() => sourceDigest(join(tempDir(), "missing"))).toThrow(/not a directory/);
    expect(() => sourceDigest(join(tree({ "a.ts": "x" }), "a.ts"))).toThrow(/not a directory/);
  });

  it("refuses an over-deep tree and an oversized file", () => {
    const deep = Array.from({ length: MAX_SOURCE_DEPTH + 2 }, (_x, i) => `d${i}`).join("/");
    expect(() => readSourceTree(tree({ [`${deep}/x.ts`]: "x" }))).toThrow(/too deeply/);
    const big = tempDir();
    writeFileSync(join(big, "big.ts"), Buffer.alloc(MAX_SOURCE_FILE_BYTES + 1));
    expect(() => readSourceTree(big)).toThrow(/larger than the limit/);
  });

  // A symlink needs a privilege Windows grants only to administrators or to
  // developer mode, so creating one there would fail for the wrong reason.
  it.skipIf(process.platform === "win32")("refuses a symlink, even to a file inside the tree", () => {
    const root = tree({ "a.ts": "x" });
    symlinkSync(join(root, "a.ts"), join(root, "b.ts"));
    expect(() => readSourceTree(root)).toThrow(/is a symlink/);
    const dirLink = tree({ "d/a.ts": "x" });
    symlinkSync(join(dirLink, "d"), join(dirLink, "e"));
    expect(() => readSourceTree(dirLink)).toThrow(/is a symlink/);
  });

  // Only Linux lets a file name be arbitrary bytes: Windows names are UTF-16 and
  // macOS rejects an invalid UTF-8 name outright.
  it.skipIf(process.platform !== "linux")("refuses a file name that is not UTF-8", () => {
    const root = tempDir();
    writeFileSync(Buffer.concat([Buffer.from(`${root}/`), Buffer.from([0x61, 0xff, 0x2e, 0x74, 0x73])]), "x");
    expect(() => readSourceTree(root)).toThrow(/non-UTF-8/);
  });

  it("digests the runner's own source tree", () => {
    const here = join(dirname(fileURLToPath(import.meta.url)), "..", "src");
    expect(runnerDigest()).toBe(sourceDigest(here));
    expect(runnerDigest()).toMatch(/^[0-9a-f]{64}$/);
    expect(readSourceTree(here).map(([path]) => path)).toContain("manifest.ts");
    expect(readSourceTree(here).some(([path]) => path.endsWith(".test.ts"))).toBe(false);
  });
});

describe("domain labels", () => {
  it("labels each descriptor from what it enumerates", () => {
    for (const [domain, label] of [
      ["bool", "full"], ["u8_all", "full"], ["bytes_n(0)", "full"], ["bytes_n(1)", "full"],
      ["bytes_n(2)", "bounded-set"], ["bytes_n(16)", "bounded-set"], ["u64_edge", "bounded-set"],
      ["bytes_d1", "bounded-set"], ["nat_0_64", "bounded-set"],
    ] as const) {
      expect(domainLabel(domain)).toBe(label);
    }
    expect(() => domainLabel("sampled")).toThrow(CorpusError);
  });

  it("splits the real corpus into 65,537 full and 506,572 bounded-set cases", () => {
    const labels = deriveLabels(corpus, "typescript/ct-compare", "ct_compare", "0".repeat(64));
    expect(labels["cases_by_label"]).toEqual({ "bounded-set": 506_572, full: 65_537 });
    expect(labels["note"]).toBe(NOTE);
    const suites = labels["suites"] as { id: string; label: string }[];
    expect(suites.filter((suite) => suite.label === "full")).toHaveLength(2);
    expect(suites.filter((suite) => suite.label === "bounded-set")).toHaveLength(7);
    expect(suites.map((suite) => suite.id)).toEqual([...suites.map((suite) => suite.id)].sort());
  });

  it("is as weak as the weakest binder", () => {
    const first = corpus.suites[0];
    if (!first) throw new Error("no suite");
    const mixed = { ...corpus, suites: [{ ...first, binders: [["a", "bool"], ["b", "bytes_d1"]] as const }] };
    const labels = deriveLabels(mixed, "p/q", "m", "1".repeat(64));
    expect((labels["suites"] as { label: string }[])[0]?.label).toBe("bounded-set");
  });
});

describe("validateManifest", () => {
  let manifest: Record<string, any>; // eslint-disable-line @typescript-eslint/no-explicit-any
  const RELATIVE = "code/specs/fixtures/ct-compare-warrant-v1";

  beforeAll(async () => {
    const lib = newLibrary();
    const prepared = await prepare({ repoRoot: REPO_ROOT, corpusDir: CORPUS_DIR, bindingPath: lib.binding, packageDir: lib.packageDir });
    manifest = execute(prepared, () => undefined).manifest as Record<string, unknown>;
  }, 600_000);

  const bytesOf = (document: unknown): Uint8Array => enc(JSON.stringify(document));
  const copy = (): Record<string, any> => JSON.parse(JSON.stringify(manifest)); // eslint-disable-line @typescript-eslint/no-explicit-any
  const registry = (rows: unknown[]): Uint8Array => bytesOf({ consumers: rows });
  const row = (status: string, extra: Record<string, unknown> = {}): Record<string, unknown> => ({
    lane: "typescript", package: "typescript/ct-compare", module: "ct_compare", status, ...extra,
  }); // prettier-ignore

  it("accepts a real manifest", () => {
    expect(validateManifest(bytesOf(manifest), corpus, RELATIVE)["lane"]).toBe("typescript");
  });

  it("joins a registry: exactly one row, the right module, active only if asked", () => {
    validateManifest(bytesOf(manifest), corpus, RELATIVE, registry([row("planned")]));
    validateManifest(bytesOf(manifest), corpus, RELATIVE, registry([row("active")]), true);
    const refuse = (rows: unknown[], active = false): void => {
      expect(() => validateManifest(bytesOf(manifest), corpus, RELATIVE, registry(rows), active)).toThrow(CorpusError);
    };
    refuse([row("planned")], true);
    refuse([]);
    refuse([row("planned"), row("planned")]);
    refuse([row("planned", { module: "other" })]);
    refuse([{ ...row("planned"), lane: "python", package: "python/ct-compare" }]);
    expect(() => validateManifest(bytesOf(manifest), corpus, RELATIVE, bytesOf({ consumers: 3 }))).toThrow(CorpusError);
    expect(() => validateManifest(bytesOf(manifest), corpus, RELATIVE, bytesOf([]))).toThrow(CorpusError);
  });

  it("rejects every kind of disagreement", () => {
    const mutations: [string, (m: Record<string, any>) => void][] = [ // eslint-disable-line @typescript-eslint/no-explicit-any
      ["extra field", (m) => (m["extra"] = 1)],
      ["missing field", (m) => delete m["tests"]],
      ["schema", (m) => (m["schema"] = "warrant-lane-evidence-v2")],
      ["lane", (m) => (m["lane"] = "Type-Script")],
      ["lane/package", (m) => (m["package"] = "python/ct-compare")],
      ["package shape", (m) => (m["package"] = "typescript")],
      ["package name", (m) => (m["package"] = "typescript/CT")],
      ["module", (m) => (m["module"] = "other")],
      ["spec", (m) => (m["spec"] = "other.warrant")],
      ["spec digest", (m) => (m["spec_sha256"] = "0".repeat(64))],
      ["corpus path", (m) => (m["corpus"] = "elsewhere")],
      ["corpus digest", (m) => (m["corpus_sha256"] = "0".repeat(64))],
      ["cases digest", (m) => (m["cases_sha256"] = "0".repeat(64))],
      ["runner name", (m) => (m["runner"]["name"] = "other")],
      ["runner lane", (m) => (m["runner"]["lane"] = "python")],
      ["runner digest", (m) => (m["runner"]["src_sha256"] = "x")],
      ["runner shape", (m) => (m["runner"]["extra"] = 1)],
      ["binding digest", (m) => (m["binding_sha256"] = "x")],
      ["source digest", (m) => (m["source_sha256"] = "X".repeat(64))],
      ["toolchain", (m) => (m["toolchain"]["version"] = 22)],
      ["seed text", (m) => (m["evidence_seed"] = "0x10")],
      ["seed number", (m) => (m["evidence_seed"] = 5)],
      ["seed range", (m) => (m["evidence_seed"] = "18446744073709551616")],
      ["faulted", (m) => (m["faulted"] = true)],
      ["row count", (m) => m["tests"].pop()],
      ["tests type", (m) => (m["tests"] = {})],
      ["row without id", (m) => (m["tests"][0] = 5)],
      ["duplicate id", (m) => (m["tests"][1]["id"] = m["tests"][0]["id"])],
      ["unsorted", (m) => m["tests"].reverse()],
      ["foreign id", (m) => (m["tests"][0]["id"] = "a")],
      ["row cases", (m) => (m["tests"].find((r: any) => r["kind"] === "exhaustive")["cases"] += 1)], // eslint-disable-line @typescript-eslint/no-explicit-any
      ["row digest", (m) => (m["tests"].find((r: any) => r["kind"] === "exhaustive")["inputs_sha256"] = "0".repeat(64))], // eslint-disable-line @typescript-eslint/no-explicit-any
      ["row verdict", (m) => (m["tests"][0]["verdict"] = "fail")],
      ["row layer", (m) => (m["tests"][0]["layer"] = "native")],
      ["row field", (m) => (m["tests"][0]["extra"] = 1)],
      ["example op", (m) => (m["tests"].find((r: any) => r["kind"] === "example")["op"] = "other")], // eslint-disable-line @typescript-eslint/no-explicit-any
      ["control not caught", (m) => (m["tests"].find((r: any) => r["kind"] === "negctl")["caught_by"] = [])], // eslint-disable-line @typescript-eslint/no-explicit-any
      ["control foreign claim", (m) => (m["tests"].find((r: any) => r["kind"] === "negctl")["caught_by"] = ["other"])], // eslint-disable-line @typescript-eslint/no-explicit-any
      ["control duplicate claim", (m) => { const r = m["tests"].find((x: any) => x["kind"] === "negctl"); r["caught_by"] = [r["caught_by"][0], r["caught_by"][0]]; }], // eslint-disable-line @typescript-eslint/no-explicit-any
      ["control fault", (m) => (m["tests"].find((r: any) => r["kind"] === "negctl")["fault"] = "invert")], // eslint-disable-line @typescript-eslint/no-explicit-any
    ];
    for (const [name, mutate] of mutations) {
      const changed = copy();
      mutate(changed);
      expect(() => validateManifest(bytesOf(changed), corpus, RELATIVE), name).toThrow(CorpusError);
    }
  });

  it("rejects malformed JSON", () => {
    expect(() => validateManifest(enc("{"), corpus, RELATIVE)).toThrow(CorpusError);
    expect(() => validateManifest(enc("[]"), corpus, RELATIVE)).toThrow(CorpusError);
  });
});
