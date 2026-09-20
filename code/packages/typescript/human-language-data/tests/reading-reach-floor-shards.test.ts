import {
  mkdirSync,
  mkdtempSync,
  readdirSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import {
  READING_REACH_FLOOR_META_OWNER,
  READING_REACH_FLOOR_OWNER_DIRECTORY,
  readReadingReachFloorOwners,
  readingReachFloorOwnerContents,
} from "../src/reading-reach-floor-shards.js";

const inventories = [
  { language: "spanish", level: "pre-A1" as const },
  { language: "spanish", level: "A1" as const },
  { language: "tamil", level: "pre-A1" as const },
];

const registry = {
  version: 1 as const,
  about: "the legacy wording is deliberately replaced by canonical metadata",
  floors: {
    "spanish/pre-A1": 2,
    "tamil/pre-A1": 3,
  },
};

const roots: string[] = [];

function temporaryRoot(): string {
  const root = mkdtempSync(join(tmpdir(), "hl-reading-reach-floor-"));
  roots.push(root);
  mkdirSync(join(root, READING_REACH_FLOOR_OWNER_DIRECTORY), { recursive: true });
  return root;
}

function writeOwners(root: string): void {
  for (const [name, body] of readingReachFloorOwnerContents(registry, inventories)) {
    writeFileSync(join(root, READING_REACH_FLOOR_OWNER_DIRECTORY, name), body, "utf8");
  }
}

afterEach(() => {
  for (const root of roots.splice(0)) rmSync(root, { recursive: true, force: true });
});

describe("reading-reach floor direct owners", () => {
  it("owns every task-shape identity while preserving absent-entry floor zero", () => {
    const root = temporaryRoot();
    writeOwners(root);

    expect([...readingReachFloorOwnerContents(registry, inventories).keys()]).toEqual([
      READING_REACH_FLOOR_META_OWNER,
      "spanish--a1.json",
      "spanish--pre-a1.json",
      "tamil--pre-a1.json",
    ]);
    const loaded = readReadingReachFloorOwners(root, { expectedInventories: inventories });
    expect(Object.entries(loaded.floors)).toEqual([
      ["spanish/pre-A1", 2],
      ["tamil/pre-A1", 3],
    ]);
    expect(loaded.floors["spanish/A1"] ?? 0).toBe(0);
  });

  it("checks exact task-shape ownership before opening bytes", () => {
    const root = temporaryRoot();
    writeOwners(root);
    rmSync(join(root, READING_REACH_FLOOR_OWNER_DIRECTORY, "tamil--pre-a1.json"));
    writeFileSync(
      join(root, READING_REACH_FLOOR_OWNER_DIRECTORY, "spanish--pre-a1.json"),
      "not json\n",
    );

    expect(() =>
      readReadingReachFloorOwners(root, { expectedInventories: inventories }),
    ).toThrow(/missing: tamil--pre-a1\.json/);

    writeFileSync(
      join(root, READING_REACH_FLOOR_OWNER_DIRECTORY, "ghost--a1.json"),
      "{}\n",
    );
    expect(() =>
      readReadingReachFloorOwners(root, {
        expectedInventories: inventories.slice(0, 2),
      }),
    ).toThrow(/extra: ghost--a1\.json/);
  });

  it("rejects identity mismatches, malformed floors, and noncanonical bytes", () => {
    const mismatch = temporaryRoot();
    writeOwners(mismatch);
    writeFileSync(
      join(mismatch, READING_REACH_FLOOR_OWNER_DIRECTORY, "spanish--a1.json"),
      `${JSON.stringify({ language: "tamil", level: "A1", floor: 0 }, null, 2)}\n`,
    );
    expect(() =>
      readReadingReachFloorOwners(mismatch, { expectedInventories: inventories }),
    ).toThrow(/carries 'tamil\/A1'/);

    const negative = temporaryRoot();
    writeOwners(negative);
    writeFileSync(
      join(negative, READING_REACH_FLOOR_OWNER_DIRECTORY, "spanish--a1.json"),
      `${JSON.stringify({ language: "spanish", level: "A1", floor: -1 }, null, 2)}\n`,
    );
    expect(() =>
      readReadingReachFloorOwners(negative, { expectedInventories: inventories }),
    ).toThrow(/non-negative integer/);

    const noncanonical = temporaryRoot();
    writeOwners(noncanonical);
    writeFileSync(
      join(noncanonical, READING_REACH_FLOOR_OWNER_DIRECTORY, "spanish--a1.json"),
      JSON.stringify({ language: "spanish", level: "A1", floor: 0 }),
    );
    expect(() =>
      readReadingReachFloorOwners(noncanonical, { expectedInventories: inventories }),
    ).toThrow(/not canonical/);
  });

  it("rejects nesting, symlinks, case-fold collisions, and resurrected aggregates", () => {
    const nested = temporaryRoot();
    writeOwners(nested);
    mkdirSync(join(nested, READING_REACH_FLOOR_OWNER_DIRECTORY, "nested"));
    expect(() =>
      readReadingReachFloorOwners(nested, { expectedInventories: inventories }),
    ).toThrow(/real direct-child regular file/);

    const linked = temporaryRoot();
    writeOwners(linked);
    rmSync(join(linked, READING_REACH_FLOOR_OWNER_DIRECTORY, "spanish--a1.json"));
    symlinkSync(
      join(linked, READING_REACH_FLOOR_OWNER_DIRECTORY, "spanish--pre-a1.json"),
      join(linked, READING_REACH_FLOOR_OWNER_DIRECTORY, "spanish--a1.json"),
    );
    expect(() =>
      readReadingReachFloorOwners(linked, { expectedInventories: inventories }),
    ).toThrow(/real direct-child regular file/);

    const collision = temporaryRoot();
    writeOwners(collision);
    writeFileSync(
      join(collision, READING_REACH_FLOOR_OWNER_DIRECTORY, "SPANISH--A1.JSON"),
      "{}\n",
    );
    // A case-insensitive checkout overwrites the lowercase file; Linux keeps
    // both and exercises the collision refusal.
    const caseVariants = readdirSync(join(collision, READING_REACH_FLOOR_OWNER_DIRECTORY))
      .filter((name) => name.toLowerCase() === "spanish--a1.json");
    if (caseVariants.length === 2) {
      expect(() =>
        readReadingReachFloorOwners(collision, { expectedInventories: inventories }),
      ).toThrow(/case-fold collision/);
    }

    const resurrected = temporaryRoot();
    writeOwners(resurrected);
    writeFileSync(join(resurrected, "core", "reading-reach-floor.json"), "{}\n");
    expect(() =>
      readReadingReachFloorOwners(resurrected, { expectedInventories: inventories }),
    ).toThrow(/present beside canonical/);
  });

  it("rejects unsafe, duplicated, and invalid expected identities", () => {
    const root = temporaryRoot();
    writeOwners(root);

    expect(() =>
      readReadingReachFloorOwners(root, {
        expectedInventories: [{ language: "../spanish", level: "A1" }],
      }),
    ).toThrow(/unsafe/);
    expect(() =>
      readReadingReachFloorOwners(root, {
        expectedInventories: [inventories[0]!, inventories[0]!],
      }),
    ).toThrow(/repeat/);
    expect(() =>
      readReadingReachFloorOwners(root, {
        expectedInventories: [{ language: "spanish", level: "A0" as never }],
      }),
    ).toThrow(/invalid/);
  });
});
