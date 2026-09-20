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
import type { ExamInventory } from "../src/exam-inventory.js";
import {
  EXAM_INVENTORY_META_OWNER,
  examInventoryOwnerContents,
  readExamInventoryOwnersIfPresent,
} from "../src/exam-inventory-shards.js";

const inventory: ExamInventory = {
  version: 1,
  language: "test",
  level: "A1",
  about: "Fixture inventory",
  source: "Fixture source",
  scope: {
    "communicative-functions": { status: "complete", source: "s", note: "n" },
    grammar: { status: "complete", source: "s", note: "n" },
    "phonology-orthography": { status: "complete", source: "s", note: "n" },
    lexicon: { status: "complete", source: "s", note: "n" },
  },
  probeSemantics: "Every probe atom is required.",
  points: [
    { id: "TEST-A1-ONE", category: "grammar", label: "one", probe: ["ATOM-ONE"] },
    { id: "TEST-A1-TWO", category: "lexicon", label: "two", probe: null, note: "gap" },
  ],
};

const roots: string[] = [];

function temporaryRoot(): { root: string; aggregate: string; directory: string } {
  const root = mkdtempSync(join(tmpdir(), "hl-exam-inventory-shards-"));
  roots.push(root);
  const aggregate = join(root, "core", "exam-inventory-test-a1.json");
  const directory = join(root, "core", "exam-inventory-test-a1.d");
  mkdirSync(directory, { recursive: true });
  return { root, aggregate, directory };
}

function writeOwners(directory: string): void {
  for (const [name, body] of examInventoryOwnerContents(inventory)) {
    writeFileSync(join(directory, name), body, "utf8");
  }
}

function read(aggregate: string): ExamInventory | null {
  return readExamInventoryOwnersIfPresent(aggregate, {
    expectedLanguage: "test",
    expectedLevel: "A1",
  });
}

afterEach(() => {
  for (const root of roots.splice(0)) rmSync(root, { recursive: true, force: true });
});

describe("exam-inventory direct point owners", () => {
  it("folds canonical metadata and one self-binding owner per ordered point", () => {
    const { aggregate, directory } = temporaryRoot();
    writeOwners(directory);

    expect([...examInventoryOwnerContents(inventory).keys()]).toEqual([
      EXAM_INVENTORY_META_OWNER,
      "0010-TEST-A1-ONE.json",
      "0020-TEST-A1-TWO.json",
    ]);
    expect(read(aggregate)).toEqual(inventory);
  });

  it("uses the metadata identity manifest to reject missing, extra, and reordered owners", () => {
    const missing = temporaryRoot();
    writeOwners(missing.directory);
    rmSync(join(missing.directory, "0020-TEST-A1-TWO.json"));
    expect(() => read(missing.aggregate)).toThrow(/missing: TEST-A1-TWO/);

    const extra = temporaryRoot();
    writeOwners(extra.directory);
    writeFileSync(
      join(extra.directory, "0030-TEST-A1-THREE.json"),
      `${JSON.stringify({
        id: "TEST-A1-THREE",
        category: "grammar",
        label: "three",
        probe: null,
      }, null, 2)}\n`,
    );
    expect(() => read(extra.aggregate)).toThrow(/extra: TEST-A1-THREE/);

    const reordered = temporaryRoot();
    writeOwners(reordered.directory);
    const one = join(reordered.directory, "0010-TEST-A1-ONE.json");
    const two = join(reordered.directory, "0020-TEST-A1-TWO.json");
    const oneBody = examInventoryOwnerContents(inventory).get("0010-TEST-A1-ONE.json")!;
    const twoBody = examInventoryOwnerContents(inventory).get("0020-TEST-A1-TWO.json")!;
    rmSync(one);
    rmSync(two);
    writeFileSync(join(reordered.directory, "0010-TEST-A1-TWO.json"), twoBody);
    writeFileSync(join(reordered.directory, "0020-TEST-A1-ONE.json"), oneBody);
    expect(() => read(reordered.aggregate)).toThrow(/order does not match/);
  });

  it("rejects filename/body mismatches and noncanonical bytes", () => {
    const mismatch = temporaryRoot();
    writeOwners(mismatch.directory);
    writeFileSync(
      join(mismatch.directory, "0010-TEST-A1-ONE.json"),
      examInventoryOwnerContents(inventory).get("0020-TEST-A1-TWO.json")!,
    );
    expect(() => read(mismatch.aggregate)).toThrow(/carries point id 'TEST-A1-TWO'/);

    const noncanonical = temporaryRoot();
    writeOwners(noncanonical.directory);
    writeFileSync(
      join(noncanonical.directory, "0010-TEST-A1-ONE.json"),
      JSON.stringify(inventory.points[0]),
    );
    expect(() => read(noncanonical.aggregate)).toThrow(/not canonical/);
  });

  it("rejects unsafe and repeated completeness identities", () => {
    const unsafe = temporaryRoot();
    writeOwners(unsafe.directory);
    const metaPath = join(unsafe.directory, EXAM_INVENTORY_META_OWNER);
    const meta = JSON.parse(
      examInventoryOwnerContents(inventory).get(EXAM_INVENTORY_META_OWNER)!,
    ) as Record<string, unknown>;
    meta.pointIds = ["../POINT"];
    writeFileSync(metaPath, `${JSON.stringify(meta, null, 2)}\n`);
    expect(() => read(unsafe.aggregate)).toThrow(/unsafe point id/);

    const repeated = temporaryRoot();
    writeOwners(repeated.directory);
    const repeatedMeta = JSON.parse(
      examInventoryOwnerContents(inventory).get(EXAM_INVENTORY_META_OWNER)!,
    ) as Record<string, unknown>;
    repeatedMeta.pointIds = ["TEST-A1-ONE", "TEST-A1-ONE"];
    writeFileSync(
      join(repeated.directory, EXAM_INVENTORY_META_OWNER),
      `${JSON.stringify(repeatedMeta, null, 2)}\n`,
    );
    expect(() => read(repeated.aggregate)).toThrow(/repeats point id/);
  });

  it("rejects nesting, symbolic links, non-regular owners, and aggregate resurrection", () => {
    const nested = temporaryRoot();
    writeOwners(nested.directory);
    mkdirSync(join(nested.directory, "nested"));
    expect(() => read(nested.aggregate)).toThrow(/real direct-child regular file/);

    const linked = temporaryRoot();
    writeOwners(linked.directory);
    rmSync(join(linked.directory, "0010-TEST-A1-ONE.json"));
    symlinkSync(
      join(linked.directory, "0020-TEST-A1-TWO.json"),
      join(linked.directory, "0010-TEST-A1-ONE.json"),
    );
    expect(() => read(linked.aggregate)).toThrow(/real direct-child regular file/);

    const resurrected = temporaryRoot();
    writeOwners(resurrected.directory);
    writeFileSync(resurrected.aggregate, `${JSON.stringify(inventory, null, 2)}\n`);
    expect(() => read(resurrected.aggregate)).toThrow(/present beside canonical/);
  });

  it("rejects case-fold-colliding owner names on case-sensitive filesystems", () => {
    const collision = temporaryRoot();
    writeOwners(collision.directory);
    writeFileSync(join(collision.directory, "0010-test-a1-one.JSON"), "{}\n");
    const variants = readdirSync(collision.directory).filter(
      (name) => name.toLowerCase() === "0010-test-a1-one.json",
    );
    if (variants.length === 2) {
      expect(() => read(collision.aggregate)).toThrow(/case-fold collision/);
    }
  });

  it("returns null only when no owner directory exists", () => {
    const { aggregate, directory } = temporaryRoot();
    rmSync(directory, { recursive: true });
    expect(read(aggregate)).toBeNull();
  });
});
