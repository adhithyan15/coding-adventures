// ---------------------------------------------------------------------------
// feature-document-ownership.test.ts — feature contracts have direct owners
// ---------------------------------------------------------------------------
//
// Package-wide guidance belongs in the README. Independently changing feature
// contracts belong in filesystem-discovered documents, with no replacement
// index for parallel authors to edit.
// ---------------------------------------------------------------------------

import {
  cpSync,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  symlinkSync,
  unlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const ownerDirectory = resolve(packageRoot, "docs/feature-contracts");
const packageReadme = resolve(packageRoot, "README.md");
const strictOwnerName = /^[a-z0-9]+(?:-[a-z0-9]+)*\.md$/;
const reservedOwnerNames = new Set(["index.md", "manifest.md", "readme.md"]);
const openingH1 = /^ {0,3}#(?!#)[\t ]+\S.*(?:\n|$)/;

const markdownStructure = (source: string): { balancedFences: boolean; h1s: number } => {
  let fence: { character: "`" | "~"; length: number } | undefined;
  let h1s = 0;

  for (const line of source.split(/\r?\n/)) {
    if (fence !== undefined) {
      const close = line.match(/^ {0,3}(`+|~+)[\t ]*$/);
      if (
        close !== null &&
        close[1]![0] === fence.character &&
        close[1]!.length >= fence.length
      ) {
        fence = undefined;
      }
      continue;
    }

    const open = line.match(/^ {0,3}(`{3,}|~{3,})(.*)$/);
    if (open !== null) {
      const marker = open[1]!;
      const character = marker[0] as "`" | "~";
      if (character === "~" || !open[2]!.includes("`")) {
        fence = { character, length: marker.length };
        continue;
      }
    }

    if (/^ {0,3}#(?!#)(?:[\t ]+|$)/.test(line)) h1s += 1;
  }

  return { balancedFences: fence === undefined, h1s };
};

const discoverOwners = (directory: string): string[] => {
  const rootStat = lstatSync(directory);
  if (rootStat.isSymbolicLink()) {
    throw new Error("feature-contract owner directory must not be a symbolic link");
  }
  if (!rootStat.isDirectory()) {
    throw new Error("feature-contract owner root must be a directory");
  }

  return readdirSync(directory)
    .map((name) => {
      if (!strictOwnerName.test(name)) {
        throw new Error(`${name}: feature-contract owner names are lowercase ASCII slugs`);
      }
      if (reservedOwnerNames.has(name)) {
        throw new Error(`${name}: feature-contract owners must not create a shared index`);
      }

      const path = resolve(directory, name);
      const stat = lstatSync(path);
      if (stat.isSymbolicLink()) {
        throw new Error(`${name}: feature-contract owner must not be a symbolic link`);
      }
      if (!stat.isFile()) {
        throw new Error(`${name}: feature-contract owner must be a regular file`);
      }

      const source = readFileSync(path, "utf8");
      expect(source, `${name} starts with one level-1 heading`).toMatch(openingH1);
      const structure = markdownStructure(source);
      expect(structure.balancedFences, `${name} has balanced fenced code blocks`).toBe(
        true,
      );
      expect(structure.h1s, `${name} has exactly one CommonMark level-1 heading`).toBe(1);
      return name;
    })
    .sort();
};

describe("Human Language Data feature-document ownership", () => {
  it("keeps migrated contracts in regular direct-owner files", () => {
    expect(discoverOwners(ownerDirectory)).toEqual(
      expect.arrayContaining([
        "assessment-contract-artifacts.md",
        "chapter-capabilities.md",
        "continuity.md",
        "generated-books.md",
        "generated-figures.md",
        "modality-manifest.md",
        "script-filmstrip-tamil.md",
        "track-progress.md",
      ]),
    );
  });

  it("keeps feature-specific sections out of the shared package README", () => {
    const readme = readFileSync(packageReadme, "utf8");
    const featureStart = readme.indexOf("## Feature contracts");
    const architectureStart = readme.indexOf("## Architecture", featureStart);
    expect(featureStart).toBeGreaterThan(-1);
    expect(architectureStart).toBeGreaterThan(featureStart);

    const featureSection = readme.slice(featureStart, architectureStart);
    expect(featureSection).toContain("docs/feature-contracts/");
    expect(featureSection).not.toMatch(/^### /m);
    expect(featureSection).not.toMatch(
      /modality manifest|filmstrip|exam inventories|chapter capabilities|duration estimator/i,
    );
  });

  it("discovers two parallel owners without an index or shared edit", () => {
    const root = mkdtempSync(resolve(tmpdir(), "human-language-feature-owners-"));
    try {
      const copied = resolve(root, "feature-contracts");
      cpSync(ownerDirectory, copied, { recursive: true });
      const readmeBefore = readFileSync(packageReadme, "utf8");

      writeFileSync(resolve(copied, "alpha-contract.md"), "# Alpha contract\n\nAlpha.\n");
      writeFileSync(resolve(copied, "beta-contract.md"), "# Beta contract\n\nBeta.\n");

      expect(discoverOwners(copied)).toEqual(
        expect.arrayContaining(["alpha-contract.md", "beta-contract.md"]),
      );
      expect(readFileSync(packageReadme, "utf8")).toBe(readmeBefore);
      expect(readdirSync(copied)).not.toEqual(
        expect.arrayContaining([
          "README.md",
          "_meta.md",
          "index.md",
          "manifest.json",
          "manifest.md",
        ]),
      );
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });

  it("rejects unsafe names, reserved indexes and non-regular children", () => {
    const root = mkdtempSync(resolve(tmpdir(), "human-language-feature-invalid-"));
    try {
      writeFileSync(resolve(root, "Good.md"), "# Good\n");
      expect(() => discoverOwners(root)).toThrow(/lowercase ASCII slugs/);
      unlinkSync(resolve(root, "Good.md"));

      for (const reserved of reservedOwnerNames) {
        writeFileSync(resolve(root, reserved), "# Shared index\n");
        expect(() => discoverOwners(root)).toThrow(/must not create a shared index/);
        unlinkSync(resolve(root, reserved));
      }

      mkdirSync(resolve(root, "nested.md"));
      expect(() => discoverOwners(root)).toThrow(/must be a regular file/);
      rmSync(resolve(root, "nested.md"), { recursive: true });

    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });

  it("rejects a symbolic-link child", (ctx) => {
    const root = mkdtempSync(resolve(tmpdir(), "human-language-feature-linked-child-"));
    try {
      const target = resolve(root, "target");
      writeFileSync(target, "# Target\n");
      try {
        symlinkSync(target, resolve(root, "linked.md"), "file");
      } catch (error) {
        if (process.platform !== "win32" || (error as NodeJS.ErrnoException).code !== "EPERM") throw error;
        ctx.skip();
        return;
      }
      expect(() => discoverOwners(root)).toThrow(/must not be a symbolic link/);
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });

  it("rejects a symbolic-link root and a non-directory root", () => {
    const parent = mkdtempSync(resolve(tmpdir(), "human-language-feature-root-"));
    try {
      const target = resolve(parent, "target");
      mkdirSync(target);
      writeFileSync(resolve(target, "source.md"), "# Source\n");
      const linkedRoot = resolve(parent, "linked-root");
      symlinkSync(target, linkedRoot, process.platform === "win32" ? "junction" : "dir");
      expect(() => discoverOwners(linkedRoot)).toThrow(
        /owner directory must not be a symbolic link/,
      );

      const fileRoot = resolve(parent, "file-root");
      writeFileSync(fileRoot, "# Not a directory\n");
      expect(() => discoverOwners(fileRoot)).toThrow(/owner root must be a directory/);
    } finally {
      rmSync(parent, { recursive: true, force: true });
    }
  });

  it("parses fenced examples and rejects malformed Markdown structure", () => {
    const root = mkdtempSync(resolve(tmpdir(), "human-language-feature-heading-"));
    try {
      writeFileSync(resolve(root, "missing.md"), "## Missing H1\n");
      expect(() => discoverOwners(root)).toThrow();
      unlinkSync(resolve(root, "missing.md"));

      writeFileSync(resolve(root, "duplicate.md"), "# First #\n\n# Second\n");
      expect(() => discoverOwners(root)).toThrow();
      unlinkSync(resolve(root, "duplicate.md"));

      writeFileSync(
        resolve(root, "balanced.md"),
        "# Owner\n\n```markdown\n# Example, not an owner\n```\n",
      );
      expect(discoverOwners(root)).toEqual(["balanced.md"]);
      unlinkSync(resolve(root, "balanced.md"));

      writeFileSync(resolve(root, "unbalanced.md"), "# Owner\n\n```markdown\n# Example\n");
      expect(() => discoverOwners(root)).toThrow(/balanced fenced code blocks/);
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });
});
