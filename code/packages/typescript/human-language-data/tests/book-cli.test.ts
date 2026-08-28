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
import { join } from "node:path";
import { tmpdir } from "node:os";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  assertHandwrittenLessonCoverage,
  generatedBookOutputs,
  handwrittenBookChapters,
  loadBookGenerationConfig,
  runBookGeneration,
} from "../src/book-cli.js";
import { defaultCurriculumRoot, loadTrackChapters } from "../src/loader.js";

const roots: string[] = [];

function fixture(output = "test/book/chapters/ch01-first.tex"): string {
  const root = mkdtempSync(join(tmpdir(), "human-language-book-"));
  roots.push(root);
  mkdirSync(join(root, "core"), { recursive: true });
  mkdirSync(join(root, "test", "lessons"), { recursive: true });
  writeFileSync(
    join(root, "core", "book-generation.json"),
    `${JSON.stringify({
      version: 1,
      sourceBaseUrl: "https://example.test/curriculum",
      targets: [{ language: "test", chapter: 1, output }],
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
          title: "Hello",
          label: "ch:hello",
          canDo: "I can say hello.",
          spineNodes: ["HELLO"],
          payoff: {
            lesson: "TEST-C01-hello",
            kind: "task",
            summary: "Say hello.",
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
spine_node: HELLO
sequence: 10
chapter: 1
type: word
headword: hello
gloss: hello
concept_tag: GREETING-HELLO
duration:
  max_seconds: 120
---

# hello

## Warm-up

Say hello.

## Wrap-up Recall

Read the [curriculum guide](../guide.md) and the
[Wiktionary entry](https://example.test/wiki/hello), then say hello again.
`,
  );
  return root;
}

afterEach(() => {
  vi.restoreAllMocks();
  for (const root of roots.splice(0))
    rmSync(root, { recursive: true, force: true });
});

describe("canonical book generator filesystem shell", () => {
  it("refuses to write through a symlinked hash-owner directory or file", () => {
    const root = fixture();
    const outside = mkdtempSync(join(tmpdir(), "human-language-book-outside-"));
    roots.push(outside);
    mkdirSync(join(root, "core", "generated-book-hashes"), { recursive: true });
    symlinkSync(outside, join(root, "core", "generated-book-hashes", "test.d"));
    expect(() => runBookGeneration(["--write"], root)).toThrow(
      /real directory/,
    );

    rmSync(join(root, "core", "generated-book-hashes", "test.d"));
    expect(runBookGeneration(["--write"], root)).toBe(0);
    const owner = join(
      root,
      "core",
      "generated-book-hashes",
      "test.d",
      "0001.json",
    );
    rmSync(owner);
    const victim = join(outside, "victim.json");
    writeFileSync(victim, "unchanged\n");
    symlinkSync(victim, owner);
    vi.spyOn(process.stderr, "write").mockImplementation(() => true);
    expect(runBookGeneration(["--check"], root)).toBe(1);
    expect(() => runBookGeneration(["--write"], root)).toThrow(
      /real regular file/,
    );
    expect(readFileSync(victim, "utf8")).toBe("unchanged\n");
  });

  it("writes and checks the generated chapter plus source-hash manifest", () => {
    const root = fixture();
    vi.spyOn(process.stdout, "write").mockImplementation(() => true);
    vi.spyOn(process.stderr, "write").mockImplementation(() => true);

    expect(runBookGeneration(["--write"], root)).toBe(0);
    const chapter = join(root, "test", "book", "chapters", "ch01-first.tex");
    const manifest = join(
      root,
      "core",
      "generated-book-hashes",
      "test.d",
      "_meta.json",
    );
    const chapterOwner = join(
      root,
      "core",
      "generated-book-hashes",
      "test.d",
      "0001.json",
    );
    const modalities = join(root, "test", "book", "chapter-modalities.tex");
    expect(existsSync(chapter)).toBe(true);
    expect(existsSync(modalities)).toBe(true);
    expect(readFileSync(manifest, "utf8")).toContain('"algorithm": "fnv1a64"');
    expect(readFileSync(chapterOwner, "utf8")).toContain('"chapter": 1');
    expect(readFileSync(modalities, "utf8")).toContain(
      "\\textbf{Hands-free start:} all 1 lesson.",
    );
    // sourceBaseUrl no longer feeds the book: a repository-relative link keeps
    // its label and loses its destination, while a real citation stays a link.
    const generated = readFileSync(chapter, "utf8");
    expect(generated).toContain("\\chapter{Hello}");
    expect(generated).toContain("\\label{ch:hello}");
    expect(generated).not.toContain("example.test/curriculum");
    expect(generated).toContain("Read the curriculum guide and the");
    expect(generated).toContain(
      "\\href{https://example.test/wiki/hello}{Wiktionary entry}",
    );
    expect(runBookGeneration(["--check"], root)).toBe(0);

    writeFileSync(chapter, "stale\n");
    expect(runBookGeneration(["--check"], root)).toBe(1);
    expect(process.stderr.write).toHaveBeenCalledWith(
      "test/book/chapters/ch01-first.tex: generated output is missing or stale\n",
    );
  });

  it("rejects outputs outside the curriculum root", () => {
    const root = fixture("../../escape.tex");
    expect(() => generatedBookOutputs(root)).toThrow(
      /unsafe generated book output/,
    );
  });

  it("resolves reusable script sets for cross-script comparison chapters", () => {
    const root = fixture();
    const lesson = join(root, "test", "lessons", "hello.md");
    // Narrowed from a blanket replaceAll: that also rewrote `id: TEST-C01-hello`
    // into a non-ASCII id, which was incidental to this test and is now refused
    // by the parser. Only the headword, gloss and body are the subject here.
    writeFileSync(
      lesson,
      readFileSync(lesson, "utf8")
        .replaceAll("headword: hello", "headword: తెలుగు தமிழ்")
        .replaceAll("gloss: hello", "gloss: తెలుగు தமிழ்")
        .replaceAll("# hello", "# తెలుగు தமிழ்"),
    );
    writeFileSync(
      join(root, "core", "book-generation.json"),
      `${JSON.stringify({
        version: 1,
        sourceBaseUrl: "https://example.test/curriculum/",
        scriptSets: {
          comparisons: [
            { unicodeScript: "Telugu", scriptCommand: "te" },
            { unicodeScript: "Tamil", scriptCommand: "ta" },
          ],
        },
        targets: [
          {
            language: "test",
            chapter: 1,
            output: "test/book/chapters/ch01-first.tex",
            scriptSet: "comparisons",
          },
        ],
      })}\n`,
    );

    const chapter = generatedBookOutputs(root).get(
      "test/book/chapters/ch01-first.tex",
    );
    expect(chapter).toContain("\\te{తెలుగు} \\ta{தமிழ்}");
  });

  it("writes and byte-checks configured canonical reference appendices", () => {
    const root = fixture();
    const configPath = join(root, "core", "book-generation.json");
    const config = JSON.parse(readFileSync(configPath, "utf8")) as Record<
      string,
      unknown
    >;
    writeFileSync(
      join(root, "test", "pronunciation-reference.md"),
      `# Test reference

## The script

- Read **తెలుగు**.
`,
    );
    writeFileSync(
      configPath,
      `${JSON.stringify({
        ...config,
        referenceAppendices: [
          {
            language: "test",
            title: "Pronunciation Reference",
            source: "test/pronunciation-reference.md",
            output: "test/book/chapters/appendix-pronunciation.tex",
            unicodeScript: "Telugu",
            scriptCommand: "te",
          },
        ],
      })}\n`,
    );

    vi.spyOn(process.stdout, "write").mockImplementation(() => true);
    vi.spyOn(process.stderr, "write").mockImplementation(() => true);
    expect(runBookGeneration(["--write"], root)).toBe(0);
    const appendix = join(
      root,
      "test",
      "book",
      "chapters",
      "appendix-pronunciation.tex",
    );
    expect(readFileSync(appendix, "utf8")).toContain("\\textbf{\\te{తెలుగు}}");
    expect(runBookGeneration(["--check"], root)).toBe(0);

    writeFileSync(appendix, "stale\n");
    expect(runBookGeneration(["--check"], root)).toBe(1);
    expect(process.stderr.write).toHaveBeenCalledWith(
      "test/book/chapters/appendix-pronunciation.tex: generated output is missing or stale\n",
    );
  });

  it("writes and byte-checks configured canonical glossaries", () => {
    const root = fixture();
    const configPath = join(root, "core", "book-generation.json");
    const config = JSON.parse(readFileSync(configPath, "utf8")) as Record<
      string,
      unknown
    >;
    writeFileSync(
      configPath,
      `${JSON.stringify({
        ...config,
        glossaries: [
          {
            language: "test",
            output: "test/book/chapters/appendix-glossary.tex",
          },
        ],
      })}\n`,
    );

    vi.spyOn(process.stdout, "write").mockImplementation(() => true);
    vi.spyOn(process.stderr, "write").mockImplementation(() => true);
    expect(runBookGeneration(["--write"], root)).toBe(0);
    const glossary = join(
      root,
      "test",
      "book",
      "chapters",
      "appendix-glossary.tex",
    );
    expect(readFileSync(glossary, "utf8")).toContain("\\textbf{hello}");
    expect(runBookGeneration(["--check"], root)).toBe(0);

    writeFileSync(glossary, "stale\n");
    expect(runBookGeneration(["--check"], root)).toBe(1);
    expect(process.stderr.write).toHaveBeenCalledWith(
      "test/book/chapters/appendix-glossary.tex: generated output is missing or stale\n",
    );
  });

  it("writes and byte-checks configured canonical answer keys", () => {
    const root = fixture();
    const lessonPath = join(root, "test", "lessons", "hello.md");
    writeFileSync(
      lessonPath,
      readFileSync(lessonPath, "utf8").replace(
        "## Wrap-up Recall\n\nRead",
        `## Wrap-up Recall
<!-- hl-knowledge: introduces=[]; assesses=[TEST-HELLO] -->
<!-- hl-activity: {"id":"TEST-C01-hello-recall","kind":"text","assesses":["TEST-HELLO"],"prompt":"Type the greeting.","answer":"hello","accepted":["hi"],"feedback":{"correct":"Right.","incorrect":"Try again."},"response_seconds":8} -->

Read`,
      ),
    );
    const configPath = join(root, "core", "book-generation.json");
    const config = JSON.parse(readFileSync(configPath, "utf8")) as Record<
      string,
      unknown
    >;
    writeFileSync(
      configPath,
      `${JSON.stringify({
        ...config,
        answerKeys: [
          {
            language: "test",
            output: "test/book/chapters/appendix-answer-key.tex",
          },
        ],
      })}\n`,
    );

    vi.spyOn(process.stdout, "write").mockImplementation(() => true);
    vi.spyOn(process.stderr, "write").mockImplementation(() => true);
    expect(runBookGeneration(["--write"], root)).toBe(0);
    const answerKey = join(
      root,
      "test",
      "book",
      "chapters",
      "appendix-answer-key.tex",
    );
    const generated = readFileSync(answerKey, "utf8");
    expect(generated).toContain("\\chapter*{Review Questions}");
    expect(generated).toContain("Type the greeting.");
    expect(generated).toContain("\\textbf{Answer:} hello");
    expect(runBookGeneration(["--check"], root)).toBe(0);

    writeFileSync(answerKey, "stale\n");
    expect(runBookGeneration(["--check"], root)).toBe(1);
    expect(process.stderr.write).toHaveBeenCalledWith(
      "test/book/chapters/appendix-answer-key.tex: generated output is missing or stale\n",
    );
  });

  it("writes and byte-checks configured canonical subject indexes", () => {
    const root = fixture();
    writeFileSync(
      join(root, "test", "chapters.json"),
      `${JSON.stringify({
        version: 1,
        language: "test",
        chapters: [
          {
            chapter: 1,
            title: "Greetings",
            label: "ch:hello",
            canDo: "I can greet someone.",
            spineNodes: ["HELLO"],
            payoff: {
              lesson: "TEST-C01-hello",
              kind: "task",
              summary: "Greet someone.",
              assesses: [],
            },
          },
        ],
      })}\n`,
    );
    const configPath = join(root, "core", "book-generation.json");
    const config = JSON.parse(readFileSync(configPath, "utf8")) as Record<
      string,
      unknown
    >;
    writeFileSync(
      configPath,
      `${JSON.stringify({
        ...config,
        indexes: [
          {
            language: "test",
            output: "test/book/chapters/appendix-index.tex",
          },
        ],
      })}\n`,
    );

    vi.spyOn(process.stdout, "write").mockImplementation(() => true);
    vi.spyOn(process.stderr, "write").mockImplementation(() => true);
    expect(runBookGeneration(["--write"], root)).toBe(0);
    const index = join(root, "test", "book", "chapters", "appendix-index.tex");
    const generated = readFileSync(index, "utf8");
    expect(generated).toContain("\\chapter*{Index}");
    expect(generated).toContain("\\textbf{hello}");
    expect(generated).toContain(
      "\\hyperref[ch:hello]{Chapter~1, p.~\\pageref*{ch:hello}}",
    );
    expect(runBookGeneration(["--check"], root)).toBe(0);

    writeFileSync(index, "stale\n");
    expect(runBookGeneration(["--check"], root)).toBe(1);
    expect(process.stderr.write).toHaveBeenCalledWith(
      "test/book/chapters/appendix-index.tex: generated output is missing or stale\n",
    );
  });

  it("rejects reference sources outside the curriculum root", () => {
    const root = fixture();
    const configPath = join(root, "core", "book-generation.json");
    const config = JSON.parse(readFileSync(configPath, "utf8")) as Record<
      string,
      unknown
    >;
    writeFileSync(
      configPath,
      `${JSON.stringify({
        ...config,
        referenceAppendices: [
          {
            language: "test",
            title: "Pronunciation Reference",
            source: "../escape.md",
            output: "test/book/chapters/appendix-pronunciation.tex",
          },
        ],
      })}\n`,
    );
    expect(() => generatedBookOutputs(root)).toThrow(
      /unsafe generated book source/,
    );
  });

  it("fails closed on an unknown reusable script set", () => {
    const root = fixture();
    const config = join(root, "core", "book-generation.json");
    writeFileSync(
      config,
      readFileSync(config, "utf8").replace(
        '"output":"test/book/chapters/ch01-first.tex"',
        '"output":"test/book/chapters/ch01-first.tex","scriptSet":"missing"',
      ),
    );
    expect(() => generatedBookOutputs(root)).toThrow(
      /unknown scriptSet 'missing'/,
    );
  });

  it("rejects an empty generation config and unsupported CLI modes", () => {
    const root = fixture();
    writeFileSync(
      join(root, "core", "book-generation.json"),
      `${JSON.stringify({ version: 1, targets: [] })}\n`,
    );
    expect(() => generatedBookOutputs(root)).toThrow(/at least one target/);

    vi.spyOn(process.stderr, "write").mockImplementation(() => true);
    expect(runBookGeneration([], root)).toBe(2);
    expect(process.stderr.write).toHaveBeenCalledWith(
      "usage: book-cli (--check | --write)\n",
    );
  });

  it("requires a canonical HTTP(S) source base URL", () => {
    const root = fixture();
    const config = join(root, "core", "book-generation.json");
    writeFileSync(
      config,
      readFileSync(config, "utf8").replace(
        '"sourceBaseUrl":"https://example.test/curriculum"',
        '"sourceBaseUrl":"../curriculum"',
      ),
    );
    expect(() => generatedBookOutputs(root)).toThrow(
      /must declare an HTTP\(S\) sourceBaseUrl/,
    );
  });

  it("rejects legacy title or label ownership in the generation config", () => {
    const root = fixture();
    const config = join(root, "core", "book-generation.json");
    writeFileSync(
      config,
      readFileSync(config, "utf8").replace(
        '"chapter":1',
        '"chapter":1,"title":"Duplicate"',
      ),
    );
    expect(() => generatedBookOutputs(root)).toThrow(
      /must derive title and label from chapters\.json/,
    );
  });

  it("fails closed when a declared chapter has no canonical metadata", () => {
    const root = fixture();
    rmSync(join(root, "test", "chapters.json"));
    expect(() => generatedBookOutputs(root)).toThrow(
      /book-generation\.json declaration has no chapters\.json capability/,
    );
  });
});

describe("handwritten schema-v2 lesson coverage", () => {
  function configure(
    root: string,
    declaration: Record<string, unknown>,
  ): string {
    const configPath = join(root, "core", "book-generation.json");
    const config = JSON.parse(readFileSync(configPath, "utf8"));
    config.handwritten = [
      {
        language: "test",
        chapter: 1,
        output: "test/book/chapters/ch01-handwritten.tex",
        ...declaration,
      },
    ];
    writeFileSync(configPath, `${JSON.stringify(config)}\n`);
    const chapter = join(
      root,
      "test",
      "book",
      "chapters",
      "ch01-handwritten.tex",
    );
    mkdirSync(join(root, "test", "book", "chapters"), { recursive: true });
    return chapter;
  }

  it("fails closed on a new schema-v2 lesson with no coverage declaration", () => {
    const root = fixture();
    configure(root, {});
    expect(() => assertHandwrittenLessonCoverage(root)).toThrow(
      /canonical schema-v2 lesson\(s\) missing from handwritten coverage ledger: TEST-C01-hello/,
    );
  });

  it("permits only issue-owned omission debt", () => {
    const root = fixture();
    configure(root, { omittedLessonIds: ["TEST-C01-hello"] });
    expect(() => assertHandwrittenLessonCoverage(root)).toThrow(
      /positive omissionIssue/,
    );

    configure(root, {
      omittedLessonIds: ["TEST-C01-hello"],
      omissionIssue: 13117,
    });
    expect(() => assertHandwrittenLessonCoverage(root)).not.toThrow();
  });

  it("requires learner-visible marker and label evidence for an embedded lesson", () => {
    const root = fixture();
    const chapter = configure(root, { embeddedLessonIds: ["TEST-C01-hello"] });
    writeFileSync(chapter, "\\chapter{Hello}\n");
    expect(() => assertHandwrittenLessonCoverage(root)).toThrow(
      /canonical-insertion marker/,
    );

    writeFileSync(
      chapter,
      "% canonical-insertion: TEST-C01-hello\n\\chapter{Hello}\n",
    );
    expect(() => assertHandwrittenLessonCoverage(root)).toThrow(
      /lacks \\label\{lesson:TEST-C01-hello\}/,
    );

    writeFileSync(
      chapter,
      "% canonical-insertion: TEST-C01-hello\n\\chapter{Hello}\n\\label{lesson:TEST-C01-hello}\n",
    );
    expect(() => assertHandwrittenLessonCoverage(root)).not.toThrow();
  });

  it("rejects stale or cross-chapter lesson ids in the ledger", () => {
    const root = fixture();
    configure(root, {
      omittedLessonIds: ["TEST-C99-not-here"],
      omissionIssue: 13117,
    });
    expect(() => assertHandwrittenLessonCoverage(root)).toThrow(
      /is not a canonical schema-v2 lesson in this chapter/,
    );
  });
});

// ---------------------------------------------------------------------------
// The hand-written half of the books.
//
// Roughly a third of the committed chapters were authored by hand before the
// generator existed. They must be *described* by the manifest (so their titles and
// labels are checkable) without being *produced* by it (so `--write` never overwrites
// authored prose with generated text). These tests pin both halves of that sentence.
// ---------------------------------------------------------------------------
describe("hand-written chapters", () => {
  const root = defaultCurriculumRoot();
  const handwritten = handwrittenBookChapters(root);
  const config = loadBookGenerationConfig(root) as {
    targets: { language: string; chapter: number; output: string }[];
    handwritten?: Array<{
      language: string;
      chapter: number;
      output: string;
      embeddedLessonIds?: string[];
      omittedLessonIds?: string[];
      omissionIssue?: number;
    }>;
  };

  /** Pull the printed title out of `\chapter{...}`, honouring nested braces. */
  function chapterTitle(tex: string): string | undefined {
    const start = tex.match(/\\chapter\s*(?:\[[^\]]*\])?\s*\{/);
    if (start?.index === undefined) return undefined;
    let depth = 0;
    for (let i = start.index + start[0].length - 1; i < tex.length; i += 1) {
      const character = tex[i];
      if (character === "\\") {
        i += 1;
        continue;
      }
      if (character === "{") depth += 1;
      else if (character === "}") {
        depth -= 1;
        if (depth === 0)
          return tex.slice(start.index + start[0].length, i).trim();
      }
    }
    return undefined;
  }

  it("is never generated", () => {
    // THE load-bearing assertion. `generatedBookOutputs` is what `--write` writes, so a
    // hand-written path appearing in it means that chapter is one command away from being
    // replaced by generated text. Adding these chapters to `targets[]` instead of
    // `handwritten[]` would fail exactly here.
    const generated = generatedBookOutputs(root);
    for (const entry of handwritten) {
      expect(
        generated.has(entry.output),
        `${entry.output} is hand-written but the generator claims it`,
      ).toBe(false);
    }
  });

  it("keeps dependency-linked issue 13117 omission debt fully retired", () => {
    const debt = (config.handwritten ?? []).filter(
      (entry) => (entry.omittedLessonIds?.length ?? 0) > 0,
    );
    expect(debt).toEqual([]);
  });

  it("keeps the generator's claim honest against the committed files themselves", () => {
    // The test above can only police chapters that are *in* `handwritten[]`. The mistake
    // it cannot see is the promotion: moving a hand-written chapter into `targets[]`, at
    // which point it leaves the list and the check stops applying to it.
    //
    // So check the files instead of the lists. Every chapter the generator produces opens
    // with a "% GENERATED FILE." banner, and no hand-authored chapter does. A committed
    // .tex without that banner appearing in `targets[]` therefore means the generator has
    // laid claim to prose a human wrote — regardless of what either list says.
    const generated = generatedBookOutputs(root);
    for (const [relative] of generated) {
      if (!relative.endsWith(".tex")) continue;
      // `book.tex` is the one generated .tex that is a COMPOSITE rather than a
      // rendering (HL21 section 6): its bytes are the authored `frontmatter.tex`,
      // then the derived `\input` list, then the authored `backmatter.tex`. So it
      // legitimately opens with `\documentclass`, which is a human's prose, and a
      // "% GENERATED FILE." banner would either have to be prepended to all 23
      // committed books or be written into the authored half where it would be a
      // lie. The invariant this test protects — the generator must not silently
      // own hand-written prose — is enforced for it instead by
      // `tests/book-tex.test.ts`, which asserts the two authored halves exist,
      // are NOT in the generated set, and reassemble the committed file byte for
      // byte.
      if (relative.endsWith("/book/book.tex")) continue;
      const path = join(root, relative);
      if (!existsSync(path)) continue;
      expect(
        readFileSync(path, "utf8").startsWith("% GENERATED FILE."),
        `${relative} is a generation target but the committed file is hand-authored`,
      ).toBe(true);
    }
  });

  it("refuses a hand-written output that escapes the curriculum root", () => {
    // Same containment rule the generation targets obey. These paths are only read today,
    // but reading is still an file open, and the guard belongs at the boundary.
    const sandbox = fixture();
    const config = join(sandbox, "core", "book-generation.json");
    writeFileSync(
      config,
      `${JSON.stringify({
        ...JSON.parse(readFileSync(config, "utf8")),
        handwritten: [
          {
            language: "test",
            chapter: 9,
            output: "../../../etc/passwd.tex",
          },
        ],
      })}\n`,
    );
    expect(() => handwrittenBookChapters(sandbox)).toThrow(
      /unsafe generated book output/,
    );
  });

  it("never lists the same chapter as both generated and hand-written", () => {
    for (const entry of handwritten) {
      const clash = config.targets.find(
        (t) => t.language === entry.language && t.chapter === entry.chapter,
      );
      expect(
        clash,
        `${entry.language} ch${entry.chapter} is in both lists`,
      ).toBeUndefined();
    }
  });

  it("derives the title and label that the .tex actually declares", () => {
    // The capability ledger owns both fields: re-read the file and compare. This is what
    // makes the ledger cross-check in chapters.test.ts mean anything.
    for (const entry of handwritten) {
      const path = join(root, entry.output);
      expect(existsSync(path), `${entry.output} is missing`).toBe(true);
      const tex = readFileSync(path, "utf8");
      expect(chapterTitle(tex), `${entry.output} title`).toBe(entry.title);
      expect(tex, `${entry.output} label`).toContain(`\\label{${entry.label}}`);
    }
  });

  it("keeps title and label ownership out of every manifest chapter declaration", () => {
    const raw = loadBookGenerationConfig(root) as {
      targets: Record<string, unknown>[];
      handwritten?: Record<string, unknown>[];
    };
    for (const entry of [...raw.targets, ...(raw.handwritten ?? [])]) {
      expect(entry).not.toHaveProperty("title");
      expect(entry).not.toHaveProperty("label");
    }
  });

  it("accounts for every committed chapter file", () => {
    // Scanning the filesystem rather than trusting the manifest is what keeps this
    // self-maintaining: a newly hand-written chapter cannot quietly escape the checks
    // above by simply not being listed.
    const known = new Set([
      ...handwritten.map((h) => h.output),
      ...config.targets.map((t) => t.output),
    ]);
    for (const language of readdirSync(root, { withFileTypes: true })) {
      if (!language.isDirectory()) continue;
      const chapters = join(root, language.name, "book", "chapters");
      if (!existsSync(chapters)) continue;
      for (const file of readdirSync(chapters)) {
        // Appendices carry no chapter number and are outside this accounting.
        if (!/^ch\d+.*\.tex$/.test(file)) continue;
        const relative = `${language.name}/book/chapters/${file}`;
        expect(
          known.has(relative),
          `${relative} is in neither targets[] nor handwritten[]`,
        ).toBe(true);
      }
    }
  });
});

describe("pronunciation reference coverage", () => {
  const root = defaultCurriculumRoot();

  it("includes pronunciation back matter in every registered book", () => {
    const registry = JSON.parse(
      readFileSync(join(root, "core", "languages.json"), "utf8"),
    ) as { languages: Array<{ id: string }> };
    expect(registry.languages.length).toBeGreaterThan(0);
    for (const { id } of registry.languages) {
      const book = join(root, id, "book", "book.tex");
      const appendix = join(
        root,
        id,
        "book",
        "chapters",
        "appendix-pronunciation.tex",
      );
      expect(existsSync(book), `${id} book`).toBe(true);
      expect(readFileSync(book, "utf8"), `${id} book input`).toContain(
        "\\input{chapters/appendix-pronunciation}",
      );
      expect(existsSync(appendix), `${id} pronunciation appendix`).toBe(true);
    }
  });

  it("generates and byte-gates the five references that were missing", () => {
    const outputs = generatedBookOutputs(root);
    for (const language of [
      "chinese",
      "japanese",
      "persian",
      "russian",
      "urdu",
    ]) {
      const relative = `${language}/book/chapters/appendix-pronunciation.tex`;
      expect(outputs.get(relative), relative).toMatch(/^% GENERATED FILE\./);
    }
  });
});

describe("glossary coverage", () => {
  const root = defaultCurriculumRoot();

  it("generates, byte-gates, and includes a glossary in every registered book", () => {
    const registry = JSON.parse(
      readFileSync(join(root, "core", "languages.json"), "utf8"),
    ) as { languages: Array<{ id: string }> };
    const outputs = generatedBookOutputs(root);
    expect(registry.languages.length).toBeGreaterThan(0);
    for (const { id } of registry.languages) {
      const relative = `${id}/book/chapters/appendix-glossary.tex`;
      expect(outputs.get(relative), relative).toMatch(/^% GENERATED FILE\./);
      expect(existsSync(join(root, relative)), `${id} glossary`).toBe(true);
      expect(
        readFileSync(join(root, id, "book", "book.tex"), "utf8"),
        `${id} book input`,
      ).toContain("\\input{chapters/appendix-glossary}");
    }
  });
});

describe("answer-key coverage", () => {
  const root = defaultCurriculumRoot();

  it("generates, byte-gates, and includes a nonempty answer key in every registered book", () => {
    const registry = JSON.parse(
      readFileSync(join(root, "core", "languages.json"), "utf8"),
    ) as { languages: Array<{ id: string }> };
    const outputs = generatedBookOutputs(root);
    expect(registry.languages.length).toBeGreaterThan(0);
    for (const { id } of registry.languages) {
      const relative = `${id}/book/chapters/appendix-answer-key.tex`;
      expect(outputs.get(relative), relative).toMatch(/^% GENERATED FILE\./);
      expect(outputs.get(relative), `${id} canonical activities`).toMatch(
        /% canonical-activities: [1-9]\d*/,
      );
      expect(existsSync(join(root, relative)), `${id} answer key`).toBe(true);
      expect(
        readFileSync(join(root, id, "book", "book.tex"), "utf8"),
        `${id} book input`,
      ).toContain("\\input{chapters/appendix-answer-key}");
    }
  });
});

describe("subject-index coverage", () => {
  const root = defaultCurriculumRoot();

  it("generates, byte-gates, and includes a nonempty index in every registered book", () => {
    const registry = JSON.parse(
      readFileSync(join(root, "core", "languages.json"), "utf8"),
    ) as { languages: Array<{ id: string }> };
    const outputs = generatedBookOutputs(root);
    expect(registry.languages.length).toBeGreaterThan(0);
    for (const { id } of registry.languages) {
      const relative = `${id}/book/chapters/appendix-index.tex`;
      expect(outputs.get(relative), relative).toMatch(/^% GENERATED FILE\./);
      expect(outputs.get(relative), `${id} canonical index entries`).toMatch(
        /% canonical-index-entries: [1-9]\d*/,
      );
      expect(existsSync(join(root, relative)), `${id} index`).toBe(true);
      expect(
        readFileSync(join(root, id, "book", "book.tex"), "utf8"),
        `${id} book input`,
      ).toContain("\\input{chapters/appendix-index}");
    }
  });
});

// ---------------------------------------------------------------------------
// Does the manifest COVER the corpus?
//
// The book is manifest-driven; narration and modality are corpus-driven. That
// asymmetry hid a real defect. `core/book-generation.json`'s Spanish entries drifted
// one out of step — the target whose `output` was `ch39-bring-get-play-meet.tex`
// declared `"chapter": 38`, and `ch40-wait-answer-buy.tex` declared 39, with nothing
// declaring 40. The book printed chapter 38 TWICE (ch39's file came out
// content-identical to ch38's, same canonical-source-hash) and dropped chapter 40.
//
// Every check passed. `check:books` verifies each DECLARED target round-trips, so a
// manifest naming the wrong chapter round-trips perfectly. `titleDrift` stayed 0
// because each file took its title from its own (correct) target. Nothing asked
// whether the declarations line up with the corpus at all.
//
// These run over targets AND handwritten together. Keeping them apart is what let the
// handwritten half — 105 of the 452 declarations — go unchecked for the same class.
// ---------------------------------------------------------------------------
describe("the manifest covers the corpus", () => {
  const root = defaultCurriculumRoot();
  const config = loadBookGenerationConfig(root) as {
    targets: { language: string; chapter: number; output: string }[];
    handwritten: { language: string; chapter: number; output: string }[];
  };
  const declared = [...config.targets, ...(config.handwritten ?? [])];

  /** `ch38-narrating.tex` -> 38. Appendices parse to null and are skipped. */
  function chapterInFilename(output: string): number | null {
    const match = /^ch0*(\d+)/.exec(output.split("/").pop() ?? "");
    return match ? Number(match[1]) : null;
  }

  it("never declares one chapter number twice in a track", () => {
    // The drift's direct signature: two declarations claiming chapter 38.
    const seen = new Map<string, string>();
    const offenders: string[] = [];
    for (const entry of declared) {
      const key = `${entry.language}#${entry.chapter}`;
      const previous = seen.get(key);
      if (previous) offenders.push(`${key}: ${previous} and ${entry.output}`);
      seen.set(key, entry.output);
    }
    expect(offenders).toEqual([]);
  });

  it("gives every declaration a filename that agrees with its chapter number", () => {
    // The cheapest tripwire, and it would have fired the instant the drift was
    // written: `ch39-*.tex` must not be declared as chapter 38. The filename is what
    // a maintainer reads; the number is what the generator obeys.
    const offenders = declared
      .filter((entry) => {
        const inName = chapterInFilename(entry.output);
        return inName !== null && inName !== entry.chapter;
      })
      .map(
        (entry) =>
          `${entry.language}: ${entry.output} declares chapter ${entry.chapter}`,
      );
    expect(offenders).toEqual([]);
  });

  it("keeps every declaration inside its own track's directory", () => {
    // A target writing into another track's folder passes every other check while
    // silently adding a chapter to a book nobody edited.
    const offenders = declared
      .filter(
        (entry) => !entry.output.startsWith(`${entry.language}/book/chapters/`),
      )
      .map(
        (entry) => `${entry.language} ch${entry.chapter} -> ${entry.output}`,
      );
    expect(offenders).toEqual([]);
  });

  it("writes every declaration to a distinct file", () => {
    // Two declarations on one path means whichever runs last silently wins.
    const seen = new Map<string, string>();
    const offenders: string[] = [];
    for (const entry of declared) {
      const previous = seen.get(entry.output);
      if (previous)
        offenders.push(
          `${entry.output}: ${previous} and ${entry.language} ch${entry.chapter}`,
        );
      seen.set(entry.output, `${entry.language} ch${entry.chapter}`);
    }
    expect(offenders).toEqual([]);
  });

  it("puts every ledgered chapter into its book, not merely into a file", () => {
    // "Reaches a file" is the weaker claim, and the weaker claim is what let the
    // original bug through in spirit: a chapter can have a declaration, and a file on
    // disk, and still be invisible because `book.tex` never \input's it.
    const byLanguage = new Map<string, Set<number>>();
    for (const entry of declared) {
      const set = byLanguage.get(entry.language) ?? new Set<number>();
      set.add(entry.chapter);
      byLanguage.set(entry.language, set);
    }
    const inputs = new Map<string, string>();
    const offenders: string[] = [];
    for (const track of loadTrackChapters()) {
      const language = track.language;
      if (!inputs.has(language)) {
        inputs.set(
          language,
          readFileSync(join(root, language, "book", "book.tex"), "utf8"),
        );
      }
      const bookTex = inputs.get(language)!;
      for (const entry of track.chapters) {
        const match = declared.find(
          (d) => d.language === language && d.chapter === entry.chapter,
        );
        if (!match) {
          offenders.push(`${language} ch${entry.chapter}: no declaration`);
          continue;
        }
        const stem = (match.output.split("/").pop() ?? "").replace(
          /\.tex$/,
          "",
        );
        if (!bookTex.includes(`\\input{chapters/${stem}}`)) {
          offenders.push(
            `${language} ch${entry.chapter}: ${stem} is never \\input into book.tex`,
          );
        }
      }
    }
    expect(offenders).toEqual([]);
  });
});

describe("chapter order against the back matter", () => {
  const root = defaultCurriculumRoot();

  // Six books printed chapters AFTER \backmatter and after all four appendices:
  // hindi 18, sanskrit 24, telugu 24, kannada 17, malayalam 17, tamil 16 -- 116
  // chapters in all. Every one of them began at a "more-verbs" chapter, so a single
  // tranche appended past the back matter and every later tranche inherited the
  // mistake by appending after it in turn.
  //
  // The reader saw the pronunciation guide, glossary, answer key and index wedged
  // between two chapters, and `book`'s \backmatter also strips chapter numbering,
  // so those 116 chapters printed unnumbered. Nothing caught it: book.tex is
  // hand-maintained, and every gate here reads the CHAPTER FILES rather than the
  // order the entrypoint inputs them in.
  it("inputs every chapter before the back matter, in ascending order", () => {
    const registry = JSON.parse(
      readFileSync(join(root, "core", "languages.json"), "utf8"),
    ) as { languages: Array<{ id: string }> };
    expect(registry.languages.length).toBeGreaterThan(0);
    for (const { id } of registry.languages) {
      const lines = readFileSync(
        join(root, id, "book", "book.tex"),
        "utf8",
      ).split("\n");
      const backmatter = lines.findIndex(
        (line) => line.trim() === "\\backmatter",
      );
      if (backmatter < 0) continue;
      const isChapter = (line: string) =>
        line.startsWith("\\input{chapters/ch");
      const stranded = lines.slice(backmatter).filter(isChapter);
      expect(stranded, `${id}: chapters input after \\backmatter`).toEqual([]);

      // Ascending, and parsed as a full integer -- reading only two digits makes
      // ch100 sort as ch10 and reports a false failure on the long tracks.
      const numbers = lines
        .slice(0, backmatter)
        .filter(isChapter)
        .map((line) => Number(/\/ch(\d+)/.exec(line)?.[1]));
      expect(numbers, `${id}: chapter inputs out of order`).toEqual(
        [...numbers].sort((left, right) => left - right),
      );
    }
  });
});

describe("chapter label validation", () => {
  // `label` is interpolated RAW into \label{...} by the generator -- the only
  // author-controlled field in chapters.json that had no guard. A security review
  // demonstrated that a label closing its own brace emits a live control sequence
  // into a generated .tex. These pin the guard AND its own falsifiability: the
  // second case proves the check can still fail, so a clean first case means
  // something.
  const hostile = "ch:x}\\immediate\\write18{id}{";

  function withLabel(label: string): string {
    const root = fixture();
    const path = join(root, "test", "chapters.json");
    const parsed = JSON.parse(readFileSync(path, "utf8")) as {
      chapters: { label: string }[];
    };
    parsed.chapters[0]!.label = label;
    writeFileSync(path, `${JSON.stringify(parsed)}\n`);
    return root;
  }

  it("refuses a label that can break out of \\label{...}", () => {
    expect(() => loadTrackChapters(withLabel(hostile))).toThrow(
      /label must match/,
    );
  });

  it("refuses a label carrying a backslash, a brace, or whitespace", () => {
    for (const label of [
      "ch:a\\input{x}",
      "ch:a}",
      "ch:a{",
      "ch a",
      "ch:a\n",
      "",
    ]) {
      expect(() => loadTrackChapters(withLabel(label))).toThrow(
        /label must match/,
      );
    }
  });

  it("still accepts every label convention the corpus actually uses", () => {
    for (const label of [
      "ch:hello",
      "ch:fa-alefbe",
      "ch:persian-greetings",
      "ch:zh-components",
      "ch:a_b",
      "ch:1",
    ]) {
      expect(() => loadTrackChapters(withLabel(label))).not.toThrow();
    }
  });

  it("accepts the committed corpus unchanged", () => {
    expect(() => loadTrackChapters(defaultCurriculumRoot())).not.toThrow();
  });
});
