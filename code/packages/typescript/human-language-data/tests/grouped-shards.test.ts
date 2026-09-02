import {
  existsSync,
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
import { defaultCurriculumRoot, loadLanguageRegistry } from "../src/loader.js";

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

// ---------------------------------------------------------------------------
// Why these expectations are DERIVED and not pinned literals
// ---------------------------------------------------------------------------
//
// These tests used to compare the live corpus against frozen numbers: a byte
// length, a SHA-256 of the whole reconstructed ledger, and the literals
// 1_118 / 69 / 1_187 / 23. Every one is a CORPUS-WIDE total, so adding a single
// chapter in a single language rewrote five lines here — see #13609, which moved
// 1_117→1_118, 1_186→1_187, the byte length, and the digest for one Spanish
// chapter.
//
// That made the file a global write-lock. Two branches adding a chapter in two
// unrelated languages edit the same five lines, so they conflict by
// construction; worse, whichever merges second carries a digest computed before
// the other landed, so a green PR breaks main on merge. Human-language work was
// therefore serialised to one in-flight chapter at a time.
//
// Replacing a literal with a derivation is only safe if the derivation comes
// from somewhere that does NOT already agree by construction. That rules out
// the obvious candidate: `core/generated-book-hashes/` looks like a second
// opinion, but `book-cli` builds it by iterating `config.targets` straight out
// of this very ledger (`readBookGenerationOwners` → `for (const configuredTarget
// of config.targets)` → `BOOK_HASH_MANIFEST_DIR`). Comparing the ledger to it
// asserts f(X) == X: add a bogus target, regenerate — which CI forces — and both
// sides move together while the test stays green. That is weaker than the
// literal it would replace, so this file does not make that comparison.
//
// `<track>/chapters.d/` IS independent. It is authored per track by whoever
// writes the chapter, and the book pipeline only checks one direction
// (`requireChapterCapability` demands a capability for every ledger entry, never
// the reverse), so a chapter can exist there without a ledger entry. Comparing
// the two as SETS is strictly stronger than the old count: a literal only
// catches a change in size, while a set comparison also catches a swap, a
// rename, or a chapter moved between languages that preserves the total.
//
// ---------------------------------------------------------------------------
// Why the handwritten count is STILL a literal
// ---------------------------------------------------------------------------
//
// Not every pinned number here was part of the write-lock, and `69` was not.
// Across the last 300 commits it moved exactly once — when owner sharding was
// introduced — while the targets/combined literals moved with ordinary chapter
// work. Authoring a chapter never changes it. It is stable because nothing
// routine flips a chapter between the two halves.
//
// It is also the ONLY thing that can see that flip. `shard-cli` never mentions
// `handwritten`; its cross-ledger check compares `targets` against the
// generated hash tree, which is built from `targets`. Deriving the handwritten
// set from the authored `.tex` does not work either, tempting as it looks: the
// `% GENERATED FILE.` stamp is itself a function of `targets`, so moving a
// chapter and regenerating overwrites the authored file with a stamped one —
// the witness is destroyed by the very act being tested, and both sides move
// together again.
//
// `data/scripts/handwritten_parity.py` records what that flip costs: the prose
// living only in the hand-written LaTeX — `sounds` blocks, `cousinweb`
// etymologies, `grammarlens` explanations, `cognates` tables — is not in the
// lesson markdown, so regeneration silently deletes it. Measured at 88 blocks
// across the six Indic tracks, "with every gate still green".
//
// So this literal stays. A number that only moves when a person deliberately
// changes the thing it measures is a tripwire, not a maintenance tax.
//
// 69 -> 67: the tripwire firing as designed. Punjabi's Chapters 4 and 5 were the
// track's last hand-written .tex, and they are now generated from their lessons.
// Nothing was silently dropped in the flip: `handwritten_parity.py` reported
// Punjabi under NOTHING WOULD BE LOST before it, and now reports the track as
// having nothing hand-written left at all.

/**
 * Authored-chapter identities according to each track's own `chapters.d` — the
 * independent opinion this file measures the ledger against.
 */
function authoredChapterIdentities(): Set<string> {
  const identities = new Set<string>();
  for (const language of loadLanguageRegistry(root).languages) {
    // The registry is repo-controlled, but every other reader here validates
    // ids before joining them onto a path, so this one does too.
    expect(language.id).toMatch(/^[a-z][a-z0-9-]*$/);
    const directory = join(root, language.id, "chapters.d");
    // A registered track with no chapter owners is a corpus fault, not a
    // reason to quietly contribute zero.
    expect(existsSync(directory), `${language.id}: chapters.d is missing`).toBe(
      true,
    );
    for (const file of readdirSync(directory)) {
      if (file === "_meta.json" || !file.endsWith(".json")) continue;
      identities.add(`${language.id}/${file.slice(0, -".json".length)}`);
    }
  }
  return identities;
}

describe("the chapter-owned real book-generation ledger", () => {
  it("re-serializes each owner file to its own canonical bytes", () => {
    // What survives from the pinned digest: every owner file on disk must equal
    // the bytes the projector would write for it, so raw formatting drift —
    // indentation, key order, a missing trailing newline — still fails. This
    // holds at any corpus size, so it never needs rewriting when a chapter
    // lands. It deliberately does NOT claim to detect changed chapter CONTENT;
    // that belongs to the per-chapter hashes `check:books` verifies, not here.
    const bytes = unshardContents(root, BOOK_GENERATION_PLAN);
    const directory = join(root, BOOK_GENERATION_DIRECTORY);
    const reconstructed = bookGenerationOwnerContents(
      JSON.parse(bytes) as BookGenerationDocument,
    );
    let compared = 0;
    for (const [name, body] of reconstructed) {
      expect(readFileSync(join(directory, name), "utf8")).toBe(body);
      compared += 1;
    }
    // Both directions: no owner on disk is left unaccounted for.
    expect(compared).toBe(
      BOOK_GENERATION_SECTION_DIRECTORIES.reduce(
        (sum, section) => sum + readdirSync(join(directory, section)).length,
        1, // _meta.json
      ),
    );
  });

  it("has the measured stable owner counts", () => {
    const directory = join(root, BOOK_GENERATION_DIRECTORY);
    const tracks = loadLanguageRegistry(root).languages.length;
    // Genuinely fixed shapes — these do not move when a chapter is authored.
    expect(readdirSync(join(directory, "script-sets.d"))).toHaveLength(8);
    expect(readdirSync(join(directory, "reference-appendices.d"))).toHaveLength(6);
    // One per registered track, derived from the registry rather than retyped.
    expect(readdirSync(join(directory, "glossaries.d"))).toHaveLength(tracks);
    expect(readdirSync(join(directory, "answer-keys.d"))).toHaveLength(tracks);
    expect(readdirSync(join(directory, "indexes.d"))).toHaveLength(tracks);
    // The handwritten count STAYS PINNED, and deliberately so — see the note
    // above on why this particular literal is not part of the write-lock.
    // 67 -> 64: french chapters 3, 4 and 5 were retired into `targets.d`, the
    // first three of French's sixteen handwritten chapters to be generated from
    // their lessons. This literal is the ONLY place a retirement shows up,
    // which is the point of pinning it.
    expect(readdirSync(join(directory, "handwritten.d"))).toHaveLength(64);
    // The total is chapter-scaled, so it is proved against the independently
    // authored `chapters.d` instead.
    expect(
      readdirSync(join(directory, "targets.d")).length +
        readdirSync(join(directory, "handwritten.d")).length,
    ).toBe(authoredChapterIdentities().size);
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

  it("covers exactly the chapters the tracks actually author", () => {
    const identities = bookGenerationIdentitySets(
      readBookGenerationOwners(root).document,
    );
    // The load-bearing assertion: every chapter a track authored has a ledger
    // entry and vice versa, compared as SETS in both directions so a swap, a
    // rename, or a chapter moved between languages fails even though the total
    // survives. `chapters.d` is authored independently of this ledger, so this
    // is a real second opinion rather than a projection of the tree under test.
    expect([...identities.combined].sort()).toEqual(
      [...authoredChapterIdentities()].sort(),
    );
    // The split, pinned. A chapter moved from `handwritten` to `targets` keeps
    // the COMBINED set identical, so only this literal sees the flip.
    // 67 -> 64 for the french chapter 3, 4 and 5 retirement.
    expect(identities.handwritten.size).toBe(64);
    expect(identities.languages.size).toBe(
      loadLanguageRegistry(root).languages.length,
    );
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
