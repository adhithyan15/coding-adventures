import { createHash } from "node:crypto";
import {
  mkdtempSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import {
  BOOK_GENERATION_DIRECTORY,
  BOOK_GENERATION_SECTION_DIRECTORIES,
  assertBookGenerationIdentitySets,
  bookGenerationIdentitySets,
  bookGenerationOwnerContents,
  readBookGenerationOwners,
  type BookGenerationDocument,
} from "../src/book-generation-shards.js";
import {
  BOOK_GENERATION_PLAN,
  SHARD_PLANS,
  runShardCli,
  shardContents,
  shardLedger,
  unshardContents,
} from "../src/shard-cli.js";
import { defaultCurriculumRoot } from "../src/loader.js";

const root = defaultCurriculumRoot();
const temporary: string[] = [];
const serialize = (value: unknown) => `${JSON.stringify(value, null, 2)}\n`;

afterEach(() => {
  for (const path of temporary.splice(0))
    rmSync(path, { recursive: true, force: true });
});

function smallDocument(): BookGenerationDocument {
  return {
    version: 1,
    sourceBaseUrl: "https://example.test/curriculum",
    scriptSets: { "test-main": [{ unicodeScript: "Latin" }] },
    referenceAppendices: [],
    glossaries: [],
    answerKeys: [],
    indexes: [],
    targets: [
      {
        language: "test",
        chapter: 1,
        output: "test/book/chapters/ch01-first.tex",
        scriptSet: "test-main",
      },
    ],
    handwritten: [],
  };
}

function writeOwners(sandbox: string, document = smallDocument()): void {
  const directory = join(sandbox, BOOK_GENERATION_DIRECTORY);
  for (const section of BOOK_GENERATION_SECTION_DIRECTORIES)
    mkdirSync(join(directory, section), { recursive: true });
  for (const [name, body] of bookGenerationOwnerContents(document)) {
    writeFileSync(join(directory, name), body);
  }
}

function sandbox(): string {
  const path = mkdtempSync(join(tmpdir(), "book-generation-owners-"));
  temporary.push(path);
  return path;
}

describe("the chapter-owned real book-generation ledger", () => {
  it("reconstructs the exact fresh-main canonical bytes", () => {
    const bytes = unshardContents(root, BOOK_GENERATION_PLAN);
    expect(Buffer.byteLength(bytes)).toBe(188_833);
    expect(createHash("sha256").update(bytes).digest("hex")).toBe(
      "906e7078b79012af1d98f42f424bb35e981d6196ddea8878c5c1970d90a7faab",
    );
  });

  it("has the measured stable owner counts", () => {
    const directory = join(root, BOOK_GENERATION_DIRECTORY);
    expect(readdirSync(join(directory, "script-sets.d"))).toHaveLength(8);
    expect(readdirSync(join(directory, "reference-appendices.d"))).toHaveLength(6);
    expect(readdirSync(join(directory, "glossaries.d"))).toHaveLength(23);
    expect(readdirSync(join(directory, "answer-keys.d"))).toHaveLength(23);
    expect(readdirSync(join(directory, "indexes.d"))).toHaveLength(23);
    expect(readdirSync(join(directory, "targets.d"))).toHaveLength(1_090);
    expect(readdirSync(join(directory, "handwritten.d"))).toHaveLength(69);
  });

  it("uses no routine flat language aggregates", () => {
    const entries = readdirSync(join(root, BOOK_GENERATION_DIRECTORY)).sort();
    expect(entries).toEqual(
      ["_meta.json", ...BOOK_GENERATION_SECTION_DIRECTORIES].sort(),
    );
    expect(entries.filter((entry) => entry.endsWith(".json"))).toEqual([
      "_meta.json",
    ]);
  });

  it("is enabled and passes the full independent-set check", () => {
    expect(SHARD_PLANS).toContain(BOOK_GENERATION_PLAN);
    expect(runShardCli(["--check", "core/book-generation.json"])).toBe(0);
  });

  it("uses the same language/chapter identity convention as hash owners", () => {
    const identities = bookGenerationIdentitySets(
      readBookGenerationOwners(root).document,
    );
    expect(identities.targets.size).toBe(1_090);
    expect(identities.handwritten.size).toBe(69);
    expect(identities.combined.size).toBe(1_159);
    expect(identities.languages.size).toBe(23);
  });
});

describe("book-generation owner projection", () => {
  it("splits directly to stable chapter and metadata owners", () => {
    const contents = shardContents(
      smallDocument() as unknown as Record<string, unknown>,
      BOOK_GENERATION_PLAN,
    );
    expect([...contents.keys()].sort()).toEqual(
      [
        "_meta.json",
        "script-sets.d/0010-test-main.json",
        "targets.d/test-0001.json",
      ].sort(),
    );
  });

  it("detects a clean owner deletion against an independent expected set", () => {
    const document = smallDocument();
    const expected = bookGenerationIdentitySets(document).targets;
    document.targets = [];
    expect(() =>
      assertBookGenerationIdentitySets(document, { targets: expected }),
    ).toThrow(/missing \[test\/0001\]/);
  });

  it("detects a clean backmatter-owner deletion against an independent set", () => {
    const document = smallDocument();
    document.glossaries = [{
      language: "test",
      output: "test/book/chapters/appendix-glossary.tex",
    }];
    const expected = bookGenerationIdentitySets(document).glossaries;
    document.glossaries = [];
    expect(() =>
      assertBookGenerationIdentitySets(document, { glossaries: expected }),
    ).toThrow(/missing \[test\/appendix-glossary\]/);
  });

  it("rejects an output owned by a different language tree", () => {
    const document = smallDocument();
    document.targets[0].output = "other/book/chapters/ch01-first.tex";
    expect(() => bookGenerationOwnerContents(document)).toThrow(
      /must stay within the 'test' track/,
    );
  });

  it.each([
    "test/../other/book/chapters/ch01-first.tex",
    "test\\book\\chapters\\ch01-first.tex",
    "test//book/chapters/ch01-first.tex",
  ])("rejects non-canonical owner output %s", (output) => {
    const document = smallDocument();
    document.targets[0].output = output;
    expect(() => bookGenerationOwnerContents(document)).toThrow(/unsafe/);
  });

  it("migrates the legacy grouped shape exactly once", () => {
    const path = sandbox();
    const directory = join(path, BOOK_GENERATION_DIRECTORY);
    mkdirSync(directory, { recursive: true });
    const document = smallDocument();
    writeFileSync(
      join(directory, "_meta.json"),
      serialize({
        version: 1,
        sourceBaseUrl: document.sourceBaseUrl,
        scriptSets: document.scriptSets,
      }),
    );
    writeFileSync(
      join(directory, "test.json"),
      serialize({
        referenceAppendices: [],
        glossaries: [],
        answerKeys: [],
        indexes: [],
        targets: document.targets,
        handwritten: [],
      }),
    );
    expect(shardLedger(path, BOOK_GENERATION_PLAN)).toHaveLength(3);
    expect(readBookGenerationOwners(path).document).toEqual(document);
    expect(() => shardLedger(path, BOOK_GENERATION_PLAN)).toThrow(
      /already uses chapter-owned/,
    );
  });

  it("refuses a resurrected aggregate without touching canonical owners", () => {
    const path = sandbox();
    writeOwners(path);
    const owner = join(path, BOOK_GENERATION_DIRECTORY, "targets.d", "test-0001.json");
    const before = readFileSync(owner, "utf8");
    mkdirSync(join(path, "core"), { recursive: true });
    writeFileSync(
      join(path, "core", "book-generation.json"),
      serialize({ ...smallDocument(), targets: [] }),
    );
    expect(() => shardLedger(path, BOOK_GENERATION_PLAN)).toThrow(
      /already uses chapter-owned/,
    );
    expect(readFileSync(owner, "utf8")).toBe(before);
  });

  it("imports an explicitly supplied monolith through a staged owner tree", () => {
    const path = sandbox();
    mkdirSync(join(path, "core"), { recursive: true });
    writeFileSync(
      join(path, "core", "book-generation.json"),
      serialize(smallDocument()),
    );
    expect(shardLedger(path, BOOK_GENERATION_PLAN)).toHaveLength(3);
    expect(readBookGenerationOwners(path).document).toEqual(smallDocument());
  });
});

describe("strict owner tree", () => {
  it("rejects a filename/record identity mismatch", () => {
    const path = sandbox();
    writeOwners(path);
    const owner = join(path, BOOK_GENERATION_DIRECTORY, "targets.d", "test-0001.json");
    const value = JSON.parse(readFileSync(owner, "utf8"));
    value.chapter = 2;
    writeFileSync(owner, serialize(value));
    expect(() => readBookGenerationOwners(path)).toThrow(/does not match its record identity/);
  });

  it("rejects unexpected nested entries", () => {
    const path = sandbox();
    writeOwners(path);
    mkdirSync(join(path, BOOK_GENERATION_DIRECTORY, "targets.d", "nested"));
    expect(() => readBookGenerationOwners(path)).toThrow(/malformed name/);
  });

  it("rejects a symlinked section", () => {
    const path = sandbox();
    writeOwners(path);
    const section = join(path, BOOK_GENERATION_DIRECTORY, "targets.d");
    rmSync(section, { recursive: true });
    symlinkSync(join(path, BOOK_GENERATION_DIRECTORY, "handwritten.d"), section);
    expect(() => readBookGenerationOwners(path)).toThrow(/symbolic link|real directory/);
  });

  it("rejects a resurrected monolith", () => {
    const path = sandbox();
    writeOwners(path);
    writeFileSync(join(path, "core", "book-generation.json"), "{}\n");
    expect(() => readBookGenerationOwners(path)).toThrow(/resurrected monolith/);
  });
});
