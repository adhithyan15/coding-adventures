import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import {
  GENERATED_BOOK_HASH_DIR,
  generatedBookHashOwnerContents,
  generatedNarrationHashOwnerContents,
  listGeneratedBookHashOwnerLanguages,
  prepareGeneratedHashOwnerWrite,
  readGeneratedBookHashManifest,
  readGeneratedNarrationHashManifest,
  type GeneratedBookHashManifest,
  type GeneratedNarrationHashManifest,
} from "../src/generated-hash-shards.js";
import { loadGeneratedBookChapterRefs } from "../src/track-progress-cli.js";

const roots: string[] = [];

function fixture(): string {
  const root = mkdtempSync(join(tmpdir(), "generated-hash-shards-"));
  roots.push(root);
  mkdirSync(join(root, "core", "generated-book-hashes"), { recursive: true });
  mkdirSync(join(root, "core", "generated-narration-hashes"), {
    recursive: true,
  });
  return root;
}

const bookManifest = (): GeneratedBookHashManifest => ({
  version: 1,
  algorithm: "fnv1a64",
  chapters: [
    {
      language: "test",
      chapter: 2,
      sourceHash: "fnv1a64:0123456789abcdef",
      lessonIds: ["TEST-C02-one"],
      tex: "test/book/chapters/ch02.tex",
    },
  ],
});

const narrationManifest = (): GeneratedNarrationHashManifest => ({
  version: 1,
  algorithm: "fnv1a64",
  maxLinearisableTableColumns: 3,
  chapters: [
    {
      language: "test",
      chapter: 2,
      sourceHash: "fnv1a64:0123456789abcdef",
      lessonIds: ["TEST-C02-one"],
      voiceLessons: 1,
      drivablePrefix: 1,
      text: "test/narration/ch02.txt",
      json: "test/narration/ch02.json",
      textHash: "fnv1a64:1111111111111111",
      jsonHash: "fnv1a64:2222222222222222",
    },
  ],
  findings: [
    {
      code: "eyes-required",
      lessonId: "TEST-C02-one",
      language: "test",
      message: "Look at the page.",
    },
  ],
});

function writeOwners(root: string, outputs: ReadonlyMap<string, string>): void {
  for (const [relative, content] of outputs) {
    const path = join(root, relative);
    mkdirSync(join(path, ".."), { recursive: true });
    writeFileSync(path, content);
  }
}

afterEach(() => {
  for (const root of roots.splice(0))
    rmSync(root, { recursive: true, force: true });
});

describe("generated chapter hash shard loader", () => {
  it("enumerates only strict top-level .d owners without reading chapter bytes", () => {
    const root = fixture();
    const base = join(root, "core", "generated-book-hashes");
    mkdirSync(join(base, "zulu.d"));
    mkdirSync(join(base, "alpha.d"));
    writeFileSync(
      join(base, "zulu.d", "malformed chapter bytes.json"),
      "not json\n",
    );
    expect(listGeneratedBookHashOwnerLanguages(root)).toEqual([
      "alpha",
      "zulu",
    ]);

    writeFileSync(join(base, "legacy.json"), "{}\n");
    expect(() => listGeneratedBookHashOwnerLanguages(root)).toThrow(
      /forbidden flat monolith/,
    );
    rmSync(join(base, "legacy.json"));

    writeFileSync(join(base, "notes.txt"), "unexpected\n");
    expect(() => listGeneratedBookHashOwnerLanguages(root)).toThrow(
      /malformed name or type/,
    );
    rmSync(join(base, "notes.txt"));

    const outside = fixture();
    symlinkSync(outside, join(base, "linked.d"));
    expect(() => listGeneratedBookHashOwnerLanguages(root)).toThrow(
      /symbolic link/,
    );
  });

  it("round-trips book and narration owners into the legacy aggregate schema", () => {
    const root = fixture();
    writeOwners(root, generatedBookHashOwnerContents("test", bookManifest()));
    writeOwners(
      root,
      generatedNarrationHashOwnerContents("test", narrationManifest()),
    );

    const book = readGeneratedBookHashManifest(
      join(root, "core", "generated-book-hashes", "test.json"),
    );
    expect(book.manifest).toEqual(bookManifest());
    expect(book.sourcePaths.map((path) => path.split("/").at(-1))).toEqual([
      "0002.json",
      "_meta.json",
    ]);
    expect(book.sharded).toBe(true);

    const narration = readGeneratedNarrationHashManifest(
      join(root, "core", "generated-narration-hashes", "test.json"),
    );
    expect(narration.manifest).toEqual(narrationManifest());
  });

  it("binds the four-digit filename to the owned chapter and rejects duplicates", () => {
    const root = fixture();
    const outputs = generatedBookHashOwnerContents("test", bookManifest());
    writeOwners(root, outputs);
    const directory = join(root, "core", "generated-book-hashes", "test.d");
    const owner = JSON.parse(
      readFileSync(join(directory, "0002.json"), "utf8"),
    );
    owner.chapter = 3;
    writeFileSync(join(directory, "0002.json"), `${JSON.stringify(owner)}\n`);
    expect(() =>
      readGeneratedBookHashManifest(
        join(root, "core", "generated-book-hashes", "test.json"),
      ),
    ).toThrow(/must own test chapter 2/);

    owner.chapter = 2;
    writeFileSync(join(directory, "0002.json"), `${JSON.stringify(owner)}\n`);
    writeFileSync(join(directory, "2.json"), `${JSON.stringify(owner)}\n`);
    expect(() =>
      readGeneratedBookHashManifest(
        join(root, "core", "generated-book-hashes", "test.json"),
      ),
    ).toThrow(/malformed name/);
  });

  it("rejects symlinked, non-regular, extra, and resurrected owners", () => {
    const root = fixture();
    writeOwners(root, generatedBookHashOwnerContents("test", bookManifest()));
    const directory = join(root, "core", "generated-book-hashes", "test.d");
    mkdirSync(join(directory, "0003.json"));
    expect(() =>
      readGeneratedBookHashManifest(
        join(root, "core", "generated-book-hashes", "test.json"),
      ),
    ).toThrow(/regular file/);
    rmSync(join(directory, "0003.json"), { recursive: true });

    symlinkSync(join(directory, "0002.json"), join(directory, "0003.json"));
    expect(() =>
      readGeneratedBookHashManifest(
        join(root, "core", "generated-book-hashes", "test.json"),
      ),
    ).toThrow(/regular file/);
    rmSync(join(directory, "0003.json"));

    writeFileSync(join(directory, "notes.json"), "{}\n");
    expect(() =>
      readGeneratedBookHashManifest(
        join(root, "core", "generated-book-hashes", "test.json"),
      ),
    ).toThrow(/malformed name/);
    rmSync(join(directory, "notes.json"));

    writeFileSync(
      join(root, "core", "generated-book-hashes", "test.json"),
      "{}\n",
    );
    expect(() =>
      readGeneratedBookHashManifest(
        join(root, "core", "generated-book-hashes", "test.json"),
      ),
    ).toThrow(/resurrected monolith/);
  });

  it("rejects dangerous JSON keys and traversal-shaped payload paths", () => {
    const root = fixture();
    writeOwners(root, generatedBookHashOwnerContents("test", bookManifest()));
    const owner = join(
      root,
      "core",
      "generated-book-hashes",
      "test.d",
      "0002.json",
    );
    writeFileSync(owner, '{"__proto__":{"polluted":true}}\n');
    expect(() =>
      readGeneratedBookHashManifest(
        join(root, "core", "generated-book-hashes", "test.json"),
      ),
    ).toThrow(/must not carry '__proto__'/);

    const payload = bookManifest().chapters[0]!;
    writeFileSync(
      owner,
      `${JSON.stringify({ ...payload, tex: "../escape.tex" })}\n`,
    );
    expect(() =>
      readGeneratedBookHashManifest(
        join(root, "core", "generated-book-hashes", "test.json"),
      ),
    ).toThrow(/unsafe/);
  });

  it("rejects duplicate narration chapters and unassigned or multiply-owned findings", () => {
    const duplicate = narrationManifest();
    duplicate.chapters.push({ ...duplicate.chapters[0]! });
    expect(() =>
      generatedNarrationHashOwnerContents("test", duplicate),
    ).toThrow(/duplicates chapter 2/);

    const unassigned = narrationManifest();
    unassigned.findings[0] = {
      ...unassigned.findings[0]!,
      lessonId: "TEST-C99-none",
    };
    expect(() =>
      generatedNarrationHashOwnerContents("test", unassigned),
    ).toThrow(/has no chapter owner/);

    const multiplyOwned = narrationManifest();
    multiplyOwned.chapters.push({
      ...multiplyOwned.chapters[0]!,
      chapter: 3,
      text: "test/narration/ch03.txt",
      json: "test/narration/ch03.json",
    });
    expect(() =>
      generatedNarrationHashOwnerContents("test", multiplyOwned),
    ).toThrow(/owned by both chapter 2 and chapter 3/);
  });

  it("binds findings to their chapter owner and canonical identity order", () => {
    const root = fixture();
    const manifest = narrationManifest();
    manifest.findings = [
      { ...manifest.findings[0]!, code: "z-last" },
      { ...manifest.findings[0]!, code: "a-first" },
    ];
    manifest.chapters.push({
      ...manifest.chapters[0]!,
      chapter: 3,
      lessonIds: ["TEST-C03-two"],
      text: "test/narration/ch03.txt",
      json: "test/narration/ch03.json",
    });
    writeOwners(root, generatedNarrationHashOwnerContents("test", manifest));
    const directory = join(
      root,
      "core",
      "generated-narration-hashes",
      "test.d",
    );
    const firstPath = join(directory, "0002.json");
    const secondPath = join(directory, "0003.json");
    const first = JSON.parse(readFileSync(firstPath, "utf8"));
    const second = JSON.parse(readFileSync(secondPath, "utf8"));
    expect(
      first.findings.map((finding: { code: string }) => finding.code),
    ).toEqual(["a-first", "z-last"]);

    first.findings.reverse();
    writeFileSync(firstPath, `${JSON.stringify(first)}\n`);
    expect(() =>
      readGeneratedNarrationHashManifest(
        join(root, "core", "generated-narration-hashes", "test.json"),
      ),
    ).toThrow(/sorted by lessonId then code/);

    first.findings.reverse();
    second.findings.push(first.findings.shift());
    writeFileSync(firstPath, `${JSON.stringify(first)}\n`);
    writeFileSync(secondPath, `${JSON.stringify(second)}\n`);
    expect(() =>
      readGeneratedNarrationHashManifest(
        join(root, "core", "generated-narration-hashes", "test.json"),
      ),
    ).toThrow(/not owned by this chapter/);
  });

  it("refuses symlinked intermediate directories and write targets", () => {
    const root = fixture();
    const outside = fixture();
    rmSync(join(root, "core"), { recursive: true });
    symlinkSync(join(outside, "core"), join(root, "core"));
    expect(() =>
      readGeneratedBookHashManifest(
        join(root, "core", "generated-book-hashes", "test.json"),
      ),
    ).toThrow(/symbolic link/);

    const clean = fixture();
    const ownerDirectory = join(
      clean,
      "core",
      "generated-book-hashes",
      "test.d",
    );
    symlinkSync(join(outside, "core", "generated-book-hashes"), ownerDirectory);
    expect(() =>
      prepareGeneratedHashOwnerWrite(
        clean,
        GENERATED_BOOK_HASH_DIR,
        "core/generated-book-hashes/test.d/0002.json",
      ),
    ).toThrow(/real directory/);
  });

  it("progress rejects an unregistered owner but allows a registered track with no owner", () => {
    const root = fixture();
    const ghost = bookManifest();
    ghost.chapters[0] = { ...ghost.chapters[0]!, language: "ghost" };
    writeOwners(root, generatedBookHashOwnerContents("ghost", ghost));
    expect(() => loadGeneratedBookChapterRefs(root, new Set(["test"]))).toThrow(
      /not a registered language/,
    );

    rmSync(join(root, "core", "generated-book-hashes", "ghost.d"), {
      recursive: true,
    });
    expect(loadGeneratedBookChapterRefs(root, new Set(["test"]))).toEqual([]);
  });
});
