/**
 * Shared test scaffolding: a reference library, bindings and tampered corpora.
 *
 * The runner must work with *any* TypeScript library, so these tests do not
 * import `@coding-adventures/ct-compare` (the runner has no dependency on a leaf
 * library). Instead each test writes a small reference library to a temporary
 * directory, with optional one-line mutations, and binds it. The real corpus
 * (emitted by the Rust compiler) is used unmodified wherever possible: its
 * digests come from outside this package, which is what makes agreeing with
 * them meaningful.
 *
 * Every fixture is copied as BYTES. Reading a file as text and writing it back
 * is a newline-translating round trip on some platforms; bytes are not.
 */

import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { afterAll } from "vitest";
import { loadCorpus, type Corpus } from "../src/corpus.ts";

const HERE = dirname(fileURLToPath(import.meta.url));
export const REPO_ROOT = resolve(HERE, "..", "..", "..", "..", "..");
export const CORPUS_DIR = join(REPO_ROOT, "code", "specs", "fixtures", "ct-compare-warrant-v1");
export const SPEC_PATH = join(REPO_ROOT, "code", "specs", "ct-compare.warrant");
export const REGISTRY_PATH = join(CORPUS_DIR, "consumers.json");

const scratch: string[] = [];

/** A fresh scratch directory, removed when the test file finishes. */
export function tempDir(): string {
  const path = mkdtempSync(join(tmpdir(), "warrant-corpus-"));
  scratch.push(path);
  return path;
}

afterAll(() => {
  for (const path of scratch.splice(0)) rmSync(path, { recursive: true, force: true });
});

let counter = 0;
export function unique(): number {
  counter += 1;
  return counter;
}

export const CORRECT: Record<string, string> = {
  ctEq: "export function ctEq(a: Uint8Array, b: Uint8Array): boolean {\n  return a.length === b.length && a.every((x, i) => x === b[i]);\n}\n",
  ctEqFixed:
    "export function ctEqFixed(a: Uint8Array, b: Uint8Array): boolean {\n  return a.length === b.length && a.every((x, i) => x === b[i]);\n}\n",
  ctSelectBytes: [
    "export function ctSelectBytes(a: Uint8Array, b: Uint8Array, choice: boolean): Uint8Array {",
    "  if (a.length !== b.length) throw new RangeError('length mismatch');",
    "  return (choice ? a : b).slice();",
    "}",
    "",
  ].join("\n"),
  ctEqU64: "export function ctEqU64(a: bigint, b: bigint): boolean {\n  return a === b;\n}\n",
};

export const BINDING_TEXT = `lane typescript
package typescript/ct-compare
module ct_compare
import src/index.ts
source src
seed 0x5741_5252_414e_5421
values Bool=boolean Bytes=Uint8Array Nat=bigint U64=bigint
op eq export=ctEq
op eq_fixed export=ctEqFixed width=implicit
op select_bytes export=ctSelectBytes errors=CT_SELECT_LENGTH_MISMATCH:RangeError
op eq_u64 export=ctEqU64
`;

export interface Library {
  packageDir: string;
  binding: string;
}

/** Write a reference library (optionally with replaced exports); return where it is. */
export function newLibrary(overrides: Record<string, string> = {}, bindingText: string = BINDING_TEXT): Library {
  const packageDir = join(tempDir(), `ctref${unique()}`);
  mkdirSync(join(packageDir, "src"), { recursive: true });
  const functions = { ...CORRECT, ...overrides };
  writeFileSync(join(packageDir, "src", "index.ts"), Object.values(functions).join("\n"));
  const binding = join(packageDir, "evidence.bindings");
  writeFileSync(binding, bindingText);
  return { packageDir, binding };
}

type Json = Record<string, unknown>;

/** Copy the real corpus into a scratch repository, optionally tampering with it. */
export function copyCorpus(
  options: {
    corpus?: (document: Json) => void;
    tfv?: (text: string) => string;
    spec?: (bytes: Buffer) => Buffer;
  } = {},
): { root: string; corpusDir: string } {
  const root = join(tempDir(), `repo${unique()}`);
  const corpusDir = join(root, "code", "specs", "fixtures", "ct-compare-warrant-v1");
  mkdirSync(corpusDir, { recursive: true });
  const document = JSON.parse(readFileSync(join(CORPUS_DIR, "corpus.json")).toString("utf-8")) as Json;
  options.corpus?.(document);
  writeFileSync(join(corpusDir, "corpus.json"), Buffer.from(JSON.stringify(document, null, 1), "utf-8"));
  const tfv = readFileSync(join(CORPUS_DIR, "cases.tfv")).toString("utf-8");
  writeFileSync(join(corpusDir, "cases.tfv"), Buffer.from(options.tfv ? options.tfv(tfv) : tfv, "utf-8"));
  const spec = readFileSync(SPEC_PATH);
  writeFileSync(join(root, "code", "specs", "ct-compare.warrant"), options.spec ? options.spec(spec) : spec);
  writeFileSync(join(corpusDir, "consumers.json"), readFileSync(REGISTRY_PATH));
  return { root, corpusDir };
}

const KEEP = ["eq_fixed_agrees_w2", "eq_fixed_agrees_w4"];

/**
 * A consistent two-suite slice of the real corpus (512 cases in all).
 *
 * Whole-corpus runs take seconds (and more under coverage), so tests of
 * command-line and manifest *plumbing* use this slice; one test runs the full
 * corpus and checks its digests against the Rust compiler's.
 */
export function miniCorpus(tweak?: (document: Json) => void): { root: string; corpusDir: string } {
  return copyCorpus({
    corpus: (document) => {
      document["suites"] = (document["suites"] as Json[]).filter((s) => KEEP.includes(s["claim"] as string));
      document["examples"] = (document["examples"] as Json[]).filter((e) => e["op"] === "eq_fixed");
      document["faults"] = (document["faults"] as Json[]).filter((f) => f["op"] === "eq_fixed");
      document["controls"] = (document["controls"] as Json[]).filter((c) => c["op"] === "eq_fixed");
      for (const control of document["controls"] as Json[]) {
        control["required_claims"] = KEEP.map((claim) => `warrant/ct_compare/exhaustive/${claim}`);
      }
      tweak?.(document);
    },
    tfv: (text) => {
      const lines = text.split("\n");
      const kept = lines.slice(1).filter((line) => line.includes("Kop S65715f6669786564"));
      return `${[lines[0], ...kept].join("\n")}\n`;
    },
  });
}

/** The real, Rust-emitted corpus, validated once per test file. */
export function realCorpus(): Corpus {
  return loadCorpus(REPO_ROOT, CORPUS_DIR);
}

/** Collect emitted lines. */
export function collect(): { lines: string[]; emit: (line: string) => void } {
  const lines: string[] = [];
  return { lines, emit: (line) => lines.push(line) };
}
