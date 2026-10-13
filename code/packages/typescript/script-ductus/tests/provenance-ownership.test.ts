// ---------------------------------------------------------------------------
// provenance-ownership.test.ts — source stories have direct file owners
// ---------------------------------------------------------------------------
//
// Stroke paths are already sharded by writing system. Their provenance prose
// must follow the same ownership boundary: a new source gets one new document,
// not another paragraph in the package README or an entry in a shared index.
// This test discovers the directory exactly as a reader does, so there is no
// registry to update when another owner arrives.
// ---------------------------------------------------------------------------

import {
  cpSync,
  lstatSync,
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
const ownerDirectory = resolve(packageRoot, "docs/stroke-sources");
const packageReadme = resolve(packageRoot, "README.md");
const strictOwnerName = /^[a-z0-9]+(?:-[a-z0-9]+)*\.md$/;

const discoverOwners = (directory: string): string[] =>
  readdirSync(directory)
    .map((name) => {
      expect(name, "provenance owner names are lowercase ASCII slugs").toMatch(
        strictOwnerName,
      );
      const stat = lstatSync(resolve(directory, name));
      expect(stat.isSymbolicLink(), `${name} must not be a symbolic link`).toBe(
        false,
      );
      expect(stat.isFile(), `${name} must be a regular file`).toBe(true);
      const source = readFileSync(resolve(directory, name), "utf8");
      expect(source, `${name} starts with one level-1 heading`).toMatch(
        /^# [^#\n]+\n/,
      );
      return name;
    })
    .sort();

describe("stroke-source provenance ownership", () => {
  it("keeps the migrated source narratives in regular direct-owner files", () => {
    const owners = discoverOwners(ownerDirectory);
    expect(owners).toEqual(
      expect.arrayContaining([
        "arabic-family-poh-db.md",
        "bengali-lipitk.md",
        "gurmukhi-gurmukhi-sikho.md",
        "kannada-chimple.md",
        "latin-grundschrift-ujipenchars2.md",
        "malayalam-jayasree.md",
        "malayalam-moag.md",
        "malayalam-thooval.md",
      ]),
    );
    expect(owners).not.toContain("README.md");
    expect(owners).not.toContain("_meta.md");
  });

  it("keeps owner-specific source sections out of the shared package README", () => {
    const readme = readFileSync(packageReadme, "utf8");
    const citationStart = readme.indexOf(
      "## Stroke order is a citation, not an opinion",
    );
    const usageStart = readme.indexOf("## Usage", citationStart);
    expect(citationStart).toBeGreaterThan(-1);
    expect(usageStart).toBeGreaterThan(citationStart);
    const citationSection = readme.slice(citationStart, usageStart);

    expect(citationSection).toContain("docs/stroke-sources/");
    expect(citationSection).not.toMatch(/^### /m);
    expect(citationSection).not.toMatch(
      /LipiTk|Gurmukhi Sikho|Thooval|University Course and Reference Grammar|Chimple|Jayasree|Grundschrift-App|POH-Db/,
    );
  });

  it("discovers two parallel owners without an index or shared edit", () => {
    const root = mkdtempSync(resolve(tmpdir(), "ductus-provenance-owners-"));
    try {
      const copied = resolve(root, "stroke-sources");
      cpSync(ownerDirectory, copied, { recursive: true });
      const readmeBefore = readFileSync(packageReadme, "utf8");

      writeFileSync(resolve(copied, "alpha-source.md"), "# Alpha source\n\nAlpha.\n");
      writeFileSync(resolve(copied, "beta-source.md"), "# Beta source\n\nBeta.\n");

      expect(discoverOwners(copied)).toEqual(
        expect.arrayContaining(["alpha-source.md", "beta-source.md"]),
      );
      expect(readFileSync(packageReadme, "utf8")).toBe(readmeBefore);
      expect(readdirSync(copied)).not.toEqual(
        expect.arrayContaining(["README.md", "_meta.md", "manifest.json"]),
      );
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });

  it("rejects unsafe names and symbolic-link owners", () => {
    const root = mkdtempSync(resolve(tmpdir(), "ductus-provenance-invalid-"));
    try {
      writeFileSync(resolve(root, "Good.md"), "# Good\n");
      expect(() => discoverOwners(root)).toThrow(/lowercase ASCII slugs/);

      unlinkSync(resolve(root, "Good.md"));
      const target = mkdtempSync(resolve(tmpdir(), "ductus-provenance-target-"));
      try {
        writeFileSync(resolve(target, "source.md"), "# Source\n");
        symlinkSync(resolve(target, "source.md"), resolve(root, "linked.md"));
        expect(() => discoverOwners(root)).toThrow(/must not be a symbolic link/);
      } finally {
        rmSync(target, { recursive: true, force: true });
      }
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });
});
