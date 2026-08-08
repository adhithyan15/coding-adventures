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

Read the [curriculum guide](../guide.md) and the
[Wiktionary entry](https://example.test/wiki/hello), then say hello again.
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
    // sourceBaseUrl no longer feeds the book: a repository-relative link keeps
    // its label and loses its destination, while a real citation stays a link.
    const generated = readFileSync(chapter, "utf8");
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
  const config = JSON.parse(
    readFileSync(join(root, "core", "book-generation.json"), "utf8"),
  ) as {
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
      .map((entry) => `${entry.language}: ${entry.output} declares chapter ${entry.chapter}`);
    expect(offenders).toEqual([]);
  });

  it("keeps every declaration inside its own track's directory", () => {
    // A target writing into another track's folder passes every other check while
    // silently adding a chapter to a book nobody edited.
    const offenders = declared
      .filter((entry) => !entry.output.startsWith(`${entry.language}/book/chapters/`))
      .map((entry) => `${entry.language} ch${entry.chapter} -> ${entry.output}`);
    expect(offenders).toEqual([]);
  });

  it("writes every declaration to a distinct file", () => {
    // Two declarations on one path means whichever runs last silently wins.
    const seen = new Map<string, string>();
    const offenders: string[] = [];
    for (const entry of declared) {
      const previous = seen.get(entry.output);
      if (previous) offenders.push(`${entry.output}: ${previous} and ${entry.language} ch${entry.chapter}`);
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
        inputs.set(language, readFileSync(join(root, language, "book", "book.tex"), "utf8"));
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
        const stem = (match.output.split("/").pop() ?? "").replace(/\.tex$/, "");
        if (!bookTex.includes(`\\input{chapters/${stem}}`)) {
          offenders.push(`${language} ch${entry.chapter}: ${stem} is never \\input into book.tex`);
        }
      }
    }
    expect(offenders).toEqual([]);
  });
});
