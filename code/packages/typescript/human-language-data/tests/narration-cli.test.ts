// Tests for the narration export's filesystem shell.
//
// The contract mirrors `book-cli.ts`, and so do these tests: write it, check it,
// break it, check it again. The property that matters is that **narration cannot
// drift from the lessons without the build noticing** — a stale `.tex` produces a
// book that looks wrong to anyone who opens it, but a stale narration produces a
// voice assistant confidently teaching a lesson that no longer exists, to someone
// driving who cannot check.

import {
  existsSync,
  mkdtempSync,
  mkdirSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  narrationOutputs,
  policyTableWidth,
  runNarrationGeneration,
  safeOutput,
} from "../src/narration-cli.js";

const roots: string[] = [];

/** A one-track curriculum on disk: registry, policy, an HL05 ledger, and two lessons. */
function fixture(
  options: { policy?: Record<string, unknown>; extraLesson?: string } = {},
): string {
  const root = mkdtempSync(join(tmpdir(), "human-language-narration-"));
  roots.push(root);
  mkdirSync(join(root, "core"), { recursive: true });
  mkdirSync(join(root, "test", "lessons"), { recursive: true });

  writeFileSync(
    join(root, "core", "languages.json"),
    `${JSON.stringify({
      version: 1,
      languages: [
        {
          id: "test",
          name: "Testish",
          family: "None",
          script: "latin",
          status: "active",
          bridges: [],
        },
      ],
    })}\n`,
  );
  writeFileSync(
    join(root, "core", "chapter-policy.json"),
    `${JSON.stringify({
      version: 1,
      payoffRepresentativeness: 0.5,
      maxNewAtomsPerLesson: 3,
      maxNewAtomsPerChapter: 12,
      maxLinearisableTableColumns: 3,
      ...(options.policy ?? {}),
    })}\n`,
  );
  writeFileSync(
    join(root, "test", "chapters.json"),
    `${JSON.stringify({
      version: 1,
      language: "test",
      chapters: [
        {
          chapter: 1,
          title: "First Words",
          label: "ch:first",
          canDo: "I can say hello.",
          spineNodes: [],
          payoff: {
            lesson: "TEST-C01-hello",
            kind: "dialogue",
            summary: "hi",
            assesses: [],
          },
        },
      ],
    })}\n`,
  );
  writeFileSync(
    join(root, "test", "lessons", "hello.md"),
    `---
schema_version: 2
id: TEST-C01-hello
sequence: 10
chapter: 1
type: word
headword: hello
gloss: a greeting
concept_tag: GREETING-HELLO
---

# hello — a greeting

## Warm-up

[PAUSE 2s] Say it once.

## Guided Practice

- [YOU SAY: "hello" — HEH-loh]

| word | English |
|---|---|
| hello | a greeting |
`,
  );
  if (options.extraLesson !== undefined) {
    writeFileSync(
      join(root, "test", "lessons", "extra.md"),
      options.extraLesson,
    );
  }
  return root;
}

afterEach(() => {
  vi.restoreAllMocks();
  for (const root of roots.splice(0))
    rmSync(root, { recursive: true, force: true });
});

describe("the narration export's filesystem shell", () => {
  it("refuses to write through a symlinked hash-owner directory or file", () => {
    const root = fixture();
    const outside = mkdtempSync(
      join(tmpdir(), "human-language-narration-outside-"),
    );
    roots.push(outside);
    mkdirSync(join(root, "core", "generated-narration-hashes"), {
      recursive: true,
    });
    symlinkSync(
      outside,
      join(root, "core", "generated-narration-hashes", "test.d"),
    );
    expect(() => runNarrationGeneration(["--write"], root)).toThrow(
      /real directory/,
    );

    rmSync(join(root, "core", "generated-narration-hashes", "test.d"));
    expect(runNarrationGeneration(["--write"], root)).toBe(0);
    const owner = join(
      root,
      "core",
      "generated-narration-hashes",
      "test.d",
      "0001.json",
    );
    rmSync(owner);
    const victim = join(outside, "victim.json");
    writeFileSync(victim, "unchanged\n");
    symlinkSync(victim, owner);
    vi.spyOn(process.stderr, "write").mockImplementation(() => true);
    expect(runNarrationGeneration(["--check"], root)).toBe(1);
    expect(() => runNarrationGeneration(["--write"], root)).toThrow(
      /real regular file/,
    );
    expect(readFileSync(victim, "utf8")).toBe("unchanged\n");
  });

  it("writes a script, a structured view, and a hash manifest per chapter", () => {
    const root = fixture();
    vi.spyOn(process.stdout, "write").mockImplementation(() => true);

    expect(runNarrationGeneration(["--write"], root)).toBe(0);
    const text = join(root, "test", "narration", "ch01.txt");
    const json = join(root, "test", "narration", "ch01.json");
    const manifest = join(
      root,
      "core",
      "generated-narration-hashes",
      "test.d",
      "_meta.json",
    );
    const chapterOwner = join(
      root,
      "core",
      "generated-narration-hashes",
      "test.d",
      "0001.json",
    );
    expect(existsSync(text)).toBe(true);

    const script = readFileSync(text, "utf8");
    expect(script).toContain("Testish, chapter 1: First Words.");
    expect(script).toContain("[pause 2 seconds]");
    expect(script).toContain('[your turn — say: "hello" — HEH-loh]');
    expect(script).toContain("hello means a greeting.");

    const structured = JSON.parse(readFileSync(json, "utf8")) as {
      version: number;
      lessons: Array<{
        id: string;
        blocks: Array<{ segments: Array<{ kind: string }> }>;
      }>;
    };
    expect(structured.version).toBe(1);
    expect(structured.lessons[0]?.id).toBe("TEST-C01-hello");
    const kinds = structured.lessons[0]?.blocks.flatMap((block) =>
      block.segments.map((segment) => segment.kind),
    );
    expect(kinds).toEqual(["pause", "speech", "prompt", "table"]);

    const recorded = JSON.parse(readFileSync(manifest, "utf8")) as {
      algorithm: string;
      maxLinearisableTableColumns: number;
    };
    const recordedChapter = JSON.parse(readFileSync(chapterOwner, "utf8")) as {
      sourceHash: string;
      textHash: string;
      drivablePrefix: number;
    };
    expect(recorded.algorithm).toBe("fnv1a64");
    expect(recorded.maxLinearisableTableColumns).toBe(3);
    expect(recordedChapter.sourceHash).toMatch(/^fnv1a64:[0-9a-f]{16}$/);
    expect(recordedChapter.drivablePrefix).toBe(1);

    expect(runNarrationGeneration(["--check"], root)).toBe(0);
  });

  it("--check fails when a generated file is edited, missing, or stale", () => {
    const root = fixture();
    vi.spyOn(process.stdout, "write").mockImplementation(() => true);
    vi.spyOn(process.stderr, "write").mockImplementation(() => true);
    expect(runNarrationGeneration(["--write"], root)).toBe(0);

    const text = join(root, "test", "narration", "ch01.txt");
    writeFileSync(text, "hand-edited\n");
    expect(runNarrationGeneration(["--check"], root)).toBe(1);
    expect(process.stderr.write).toHaveBeenCalledWith(
      "test/narration/ch01.txt: generated narration is missing or stale\n",
    );

    rmSync(text);
    expect(runNarrationGeneration(["--check"], root)).toBe(1);
  });

  it("--check catches a lesson edited after the export was written", () => {
    // This is the drift the hash manifest exists for: the lesson changes, nobody
    // re-runs the exporter, and the voice assistant keeps teaching the old words.
    const root = fixture();
    vi.spyOn(process.stdout, "write").mockImplementation(() => true);
    vi.spyOn(process.stderr, "write").mockImplementation(() => true);
    expect(runNarrationGeneration(["--write"], root)).toBe(0);
    expect(runNarrationGeneration(["--check"], root)).toBe(0);

    const source = join(root, "test", "lessons", "hello.md");
    writeFileSync(
      source,
      readFileSync(source, "utf8").replace("Say it once.", "Say it twice."),
    );
    expect(runNarrationGeneration(["--check"], root)).toBe(1);

    const manifest = join(
      root,
      "core",
      "generated-narration-hashes",
      "test.d",
      "0001.json",
    );
    const stale = readFileSync(manifest, "utf8");
    expect(runNarrationGeneration(["--write"], root)).toBe(0);
    const fresh = readFileSync(manifest, "utf8");
    expect(fresh).not.toBe(stale);
    expect(runNarrationGeneration(["--check"], root)).toBe(0);
  });

  it("refuses any output path that escapes the curriculum root", () => {
    const root = fixture();
    expect(() => safeOutput(root, "../escape.txt")).toThrow(
      /unsafe generated narration output/,
    );
    expect(() => safeOutput(root, "/etc/passwd")).toThrow(
      /unsafe generated narration output/,
    );
    expect(() => safeOutput(root, "test/narration/../../../out.json")).toThrow(
      /unsafe generated narration output/,
    );
    // …and any extension we did not intend to write.
    expect(() => safeOutput(root, "test/narration/ch01.sh")).toThrow(
      /unsafe generated narration output/,
    );
    expect(safeOutput(root, "test/narration/ch01.txt")).toBe(
      join(root, "test", "narration", "ch01.txt"),
    );
  });

  it("validates the authored table width rather than trusting it", () => {
    // The policy file is authored JSON. A negative or fractional width would silently
    // reshape every lesson's modality, so it is rejected loudly instead.
    for (const bad of [-1, 2.5, 99, "3", null]) {
      const root = fixture({ policy: { maxLinearisableTableColumns: bad } });
      expect(() => policyTableWidth(root)).toThrow(
        /maxLinearisableTableColumns/,
      );
    }
    const missing = fixture({
      policy: { maxLinearisableTableColumns: undefined },
    });
    writeFileSync(
      join(missing, "core", "chapter-policy.json"),
      `${JSON.stringify({ version: 1, payoffRepresentativeness: 0.5, maxNewAtomsPerLesson: 3, maxNewAtomsPerChapter: 12 })}\n`,
    );
    expect(policyTableWidth(missing)).toBe(3);
  });

  it("reports a bad policy as an error instead of writing a wrong export", () => {
    const root = fixture({ policy: { maxLinearisableTableColumns: -1 } });
    vi.spyOn(process.stderr, "write").mockImplementation(() => true);
    expect(runNarrationGeneration(["--write"], root)).toBe(2);
    expect(existsSync(join(root, "test", "narration", "ch01.txt"))).toBe(false);
  });

  it("threads the policy width through to what gets exported", () => {
    const wide = `---
schema_version: 2
id: TEST-C02-wide
sequence: 10
chapter: 2
type: word
headword: paradigm
gloss: a grid
---

# paradigm — a grid

## Grammar Lens

| yo | tú | él |
|---|---|---|
| soy | eres | es |
`;
    const narrow = narrationOutputs(fixture({ extraLesson: wide })).get(
      "test/narration/ch02.txt",
    );
    expect(narrow).toContain("yo: soy. tú: eres. él: es.");

    const strict = narrationOutputs(
      fixture({
        policy: { maxLinearisableTableColumns: 2 },
        extraLesson: wide,
      }),
    ).get("test/narration/ch02.txt");
    expect(strict).toContain("There is a table here I cannot read to you");
    expect(strict).toContain("Before we start: this one needs your eyes");
  });

  it("rejects anything but a single --check or --write", () => {
    const root = fixture();
    vi.spyOn(process.stderr, "write").mockImplementation(() => true);
    for (const args of [[], ["--write", "--check"], ["--wat"]]) {
      expect(runNarrationGeneration(args, root)).toBe(2);
    }
    expect(process.stderr.write).toHaveBeenCalledWith(
      "usage: narration-cli (--check | --write)\n",
    );
  });

  it("keeps the manifest deterministic across runs", () => {
    const root = fixture();
    const first = narrationOutputs(root).get(
      "core/generated-narration-hashes/test.d/0001.json",
    );
    const second = narrationOutputs(root).get(
      "core/generated-narration-hashes/test.d/0001.json",
    );
    expect(first).toBe(second);
    expect(first?.endsWith("\n")).toBe(true);
  });
});

describe("the committed corpus export", () => {
  // The gate itself. If this fails, someone edited a lesson without re-running
  // `npm run generate:narration`, and the committed script no longer matches it.
  it("is in sync with the lessons it came from", () => {
    vi.spyOn(process.stderr, "write").mockImplementation(() => true);
    expect(runNarrationGeneration(["--check"])).toBe(0);
  });
});
