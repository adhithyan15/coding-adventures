// HL47 — writing-system figure policy has direct source and evidence owners.

import { lstatSync, readFileSync, readdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const sourceOwnerDirectory = resolve(packageRoot, "src/figure-target-policies");
const evidenceOwnerDirectory = resolve(packageRoot, "tests/figure-targets");
const safeOwnerName = /^[a-z][a-z0-9-]*\.ts$/;
const reservedOwnerNames = new Set([
  "all.ts",
  "index.ts",
  "manifest.ts",
  "policies.ts",
  "registry.ts",
]);

function directOwners(directory: string): string[] {
  const root = lstatSync(directory);
  expect(root.isSymbolicLink(), `${directory} is not a symlink`).toBe(false);
  expect(root.isDirectory(), `${directory} is a directory`).toBe(true);
  return readdirSync(directory)
    .map((name) => {
      expect(name, `${name} is a safe direct-owner filename`).toMatch(safeOwnerName);
      expect(reservedOwnerNames.has(name), `${name} is not a shared aggregate`).toBe(false);
      const stat = lstatSync(resolve(directory, name));
      expect(stat.isSymbolicLink(), `${name} is not a symlink`).toBe(false);
      expect(stat.isFile(), `${name} is a regular file`).toBe(true);
      return name;
    })
    .sort();
}

describe("figure-target policy ownership", () => {
  it("keeps writing-system policy in direct source owners", () => {
    expect(directOwners(sourceOwnerDirectory)).toEqual(
      expect.arrayContaining([
        "arabic-family.ts",
        "bengali.ts",
        "chinese.ts",
        "cyrillic.ts",
        "devanagari.ts",
        "gujarati.ts",
        "gurmukhi.ts",
        "japanese.ts",
        "kannada.ts",
        "latin.ts",
        "malayalam.ts",
        "tamil.ts",
        "telugu.ts",
      ]),
    );
  });

  it("keeps the compatibility facade free of policy and citations", () => {
    const facade = readFileSync(resolve(packageRoot, "src/figure-targets.ts"), "utf8");
    expect(facade).not.toMatch(/https:\/\//);
    expect(facade).not.toMatch(/Script=(?:Devanagari|Gurmukhi)/);
    expect(facade).not.toMatch(/Noto Sans|LipiTk|Jayasree|KanoAI|Grundschrift/);
    expect(facade.split(/\r?\n/).length).toBeLessThanOrEqual(25);
  });

  it("keeps the stable core independent of writing-system facts", () => {
    const core = readFileSync(resolve(packageRoot, "src/figure-target-core.ts"), "utf8");
    expect(core).not.toMatch(/https:\/\//);
    expect(core).not.toMatch(/(?:tamil|gujarati|devanagari|gurmukhi|malayalam|bengali)/i);
    expect(core).not.toMatch(/Noto Sans|LipiTk|Jayasree|KanoAI|Grundschrift/);
  });

  it("uses one stable composition boundary rather than a second policy copy", () => {
    const composition = readFileSync(resolve(packageRoot, "src/figure-target-policy.ts"), "utf8");
    const owners = directOwners(sourceOwnerDirectory);
    for (const owner of owners) {
      expect(composition, `${owner} has one stable import`).toContain(
        `./figure-target-policies/${owner.slice(0, -3)}.js`,
      );
    }
    expect(composition).not.toMatch(/https:\/\//);
    expect(composition).not.toMatch(/Noto Sans|LipiTk|Jayasree|KanoAI|Grundschrift/);
    expect(composition).not.toMatch(/readonly \["/);
  });

  it("retires the shared real-corpus aggregate in favour of evidence shards", () => {
    expect(() =>
      lstatSync(resolve(evidenceOwnerDirectory, "the-real-corpus-1d0d0f63.case.ts")),
    ).toThrow();
    expect(readdirSync(evidenceOwnerDirectory)).toEqual(
      expect.arrayContaining([
        "real-corpus-bengali-telugu-figure-targets-e3f520d1.case.ts",
        "real-corpus-devanagari-figure-targets-803c533e.case.ts",
        "real-corpus-gujarati-figure-targets-b0baaa53.case.ts",
        "real-corpus-latin-figure-targets-28d94b6f.case.ts",
        "real-corpus-tamil-figure-targets-1987c5d4.case.ts",
        "real-corpus-target-closure-d27405ec.case.ts",
        "real-corpus-target-printing-6d53c8a1.case.ts",
      ]),
    );
  });
});
