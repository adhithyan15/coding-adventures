/**
 * Shared scaffolding for the v2 tests: the committed kit and corpora, a
 * scratch copy of the kit that a test may tamper with, and a tiny builder for
 * `Program`s.
 */

import { createHash } from "node:crypto";
import { cpSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterAll } from "vitest";
import { REPO_ROOT } from "../helpers.ts";

export const KIT_DIR = join(REPO_ROOT, "code", "specs", "fixtures", "warrant-corpus-conformance-v1");
export const CORPORA = ["ct-compare", "x25519", "lzw", "wasm-leb128"].map((name) => ({
  name,
  dir: join(REPO_ROOT, "code", "specs", "fixtures", `${name}-warrant-v2`),
}));

type Json = any; // eslint-disable-line @typescript-eslint/no-explicit-any

const scratch: string[] = [];
afterAll(() => {
  for (const path of scratch.splice(0)) rmSync(path, { recursive: true, force: true });
});

/** A fresh temp directory removed when the test file finishes. */
export function tempDir(prefix = "warrant-v2-"): string {
  const path = mkdtempSync(join(tmpdir(), prefix));
  scratch.push(path);
  return path;
}

/** The committed kit's category file, parsed. */
export function kitFile(category: string): { cases: Json[] } {
  return JSON.parse(readFileSync(join(KIT_DIR, `${category}.json`), "utf8"));
}

/** The case with `id` from its category file. */
export function kitCase(id: string): Json {
  const category = id.split("/")[0] as string;
  const found = kitFile(category).cases.find((c) => c.id === id);
  if (found === undefined) throw new Error(`no kit case ${id}`);
  return found;
}

/**
 * A scratch copy of the kit whose category file `category` is rewritten by
 * `edit` (which may change the `cases` array); the index row is re-sealed so
 * the copy is internally consistent.  Returns the directory.
 */
export function tamperedKit(category: string, edit: (cases: Json[]) => void): string {
  const dir = tempDir("warrant-kit-");
  cpSync(KIT_DIR, dir, { recursive: true });
  const body = JSON.parse(readFileSync(join(dir, `${category}.json`), "utf8"));
  edit(body.cases);
  const text = `${JSON.stringify(body, null, 2)}\n`;
  writeFileSync(join(dir, `${category}.json`), text);
  const index = JSON.parse(readFileSync(join(dir, "index.json"), "utf8"));
  const row = index.files.find((f: Json) => f.category === category);
  index.total_cases += body.cases.length - row.cases;
  row.cases = body.cases.length;
  row.sha256 = createHash("sha256").update(text).digest("hex");
  writeFileSync(join(dir, "index.json"), `${JSON.stringify(index, null, 2)}\n`);
  return dir;
}

/** A kit copy with one extra category row (a made-up file of `count` cases). */
export function kitWithExtraCategory(category: string, count: number, requires: string[] = ["ir.core"]): string {
  const dir = tempDir("warrant-kit-");
  cpSync(KIT_DIR, dir, { recursive: true });
  const cases = Array.from({ length: count }, (_unused, i) => ({ id: `${category}/${i}`, summary: "x", requires, expect: {} }));
  const text = `${JSON.stringify({ schema: "warrant-corpus-conformance-v1", category, cases }, null, 2)}\n`;
  writeFileSync(join(dir, `${category}.json`), text);
  const index = JSON.parse(readFileSync(join(dir, "index.json"), "utf8"));
  index.files.push({ cases: count, category, path: `${category}.json`, sha256: createHash("sha256").update(text).digest("hex") });
  index.total_cases += count;
  writeFileSync(join(dir, "index.json"), `${JSON.stringify(index, null, 2)}\n`);
  return dir;
}
