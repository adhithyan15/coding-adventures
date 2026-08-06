import {
  existsSync,
  mkdtempSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  generatedBookOutputs,
  handwrittenBookChapters,
  runBookGeneration,
} from "../src/book-cli.js";
import { defaultCurriculumRoot } from "../src/loader.js";

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
      targets: [{ language: "test", chapter: 1, title: "Hello", label: "ch:hello", output }],
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

Read the [curriculum guide](../guide.md), then say hello again.
`,
  );
  return root;
}

afterEach(() => {
  vi.restoreAllMocks();
  for (const root of roots.splice(0)) rmSync(root, { recursive: true, force: true });
});

describe("canonical book generator filesystem shell", () => {
  it("writes and checks the generated chapter plus source-hash manifest", () => {
    const root = fixture();
    vi.spyOn(process.stdout, "write").mockImplementation(() => true);
    vi.spyOn(process.stderr, "write").mockImplementation(() => true);

    expect(runBookGeneration(["--write"], root)).toBe(0);
    const chapter = join(root, "test", "book", "chapters", "ch01-first.tex");
    const manifest = join(root, "core", "generated-book-hashes.json");
    expect(existsSync(chapter)).toBe(true);
    expect(readFileSync(manifest, "utf8")).toContain('"algorithm": "fnv1a64"');
    expect(readFileSync(chapter, "utf8")).toContain(
      "\\href{https://example.test/curriculum/test/guide.md}{curriculum guide}",
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
    expect(() => generatedBookOutputs(root)).toThrow(/unsafe generated book output/);
  });

  it("resolves reusable script sets for cross-script comparison chapters", () => {
    const root = fixture();
    const lesson = join(root, "test", "lessons", "hello.md");
    writeFileSync(lesson, readFileSync(lesson, "utf8").replaceAll("hello", "తెలుగు தமிழ்"));
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
        targets: [{
          language: "test",
          chapter: 1,
          title: "Hello",
          label: "ch:hello",
          output: "test/book/chapters/ch01-first.tex",
          scriptSet: "comparisons",
        }],
      })}\n`,
    );

    const chapter = generatedBookOutputs(root).get("test/book/chapters/ch01-first.tex");
    expect(chapter).toContain("\\te{తెలుగు} \\ta{தமிழ்}");
  });

  it("fails closed on an unknown reusable script set", () => {
    const root = fixture();
    const config = join(root, "core", "book-generation.json");
    writeFileSync(
      config,
      readFileSync(config, "utf8").replace('"output":"test/book/chapters/ch01-first.tex"', '"output":"test/book/chapters/ch01-first.tex","scriptSet":"missing"'),
    );
    expect(() => generatedBookOutputs(root)).toThrow(/unknown scriptSet 'missing'/);
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
    expect(process.stderr.write).toHaveBeenCalledWith("usage: book-cli (--check | --write)\n");
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
    expect(() => generatedBookOutputs(root)).toThrow(/must declare an HTTP\(S\) sourceBaseUrl/);
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
  const config = JSON.parse(
    readFileSync(join(root, "core", "book-generation.json"), "utf8"),
  ) as { targets: { language: string; chapter: number; output: string }[] };

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
        if (depth === 0) return tex.slice(start.index + start[0].length, i).trim();
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
            title: "Escape",
            label: "ch:escape",
            output: "../../../etc/passwd.tex",
          },
        ],
      })}\n`,
    );
    expect(() => handwrittenBookChapters(sandbox)).toThrow(/unsafe generated book output/);
  });

  it("never lists the same chapter as both generated and hand-written", () => {
    for (const entry of handwritten) {
      const clash = config.targets.find(
        (t) => t.language === entry.language && t.chapter === entry.chapter,
      );
      expect(clash, `${entry.language} ch${entry.chapter} is in both lists`).toBeUndefined();
    }
  });

  it("records the title and label the .tex actually declares", () => {
    // Transcribed, not invented: re-read the file and compare. This is what makes the
    // ledger cross-check in chapters.test.ts mean anything.
    for (const entry of handwritten) {
      const path = join(root, entry.output);
      expect(existsSync(path), `${entry.output} is missing`).toBe(true);
      const tex = readFileSync(path, "utf8");
      expect(chapterTitle(tex), `${entry.output} title`).toBe(entry.title);
      expect(tex, `${entry.output} label`).toContain(`\\label{${entry.label}}`);
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
        expect(known.has(relative), `${relative} is in neither targets[] nor handwritten[]`).toBe(
          true,
        );
      }
    }
  });
});
