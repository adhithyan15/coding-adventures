// Lost script: "?" standing where a word belongs.
//
// Twenty-six Japanese lessons shipped with their kana and kanji replaced by
// ASCII question marks ("Say ??? and tap all three morae"), through every gate
// and into the published book, because the damage is valid text and nothing
// asked whether the text still said anything. See `src/lost-script.ts` for the
// three shapes and why each allowance exists; this file pins all of them, the
// linear-time guarantee, and the corpus itself.
import { readdirSync, readFileSync } from "node:fs";
import { join, relative } from "node:path";
import { describe, expect, it } from "vitest";
import { measureLostScript, renderLostScript } from "../src/lost-script.js";
import { parseLesson } from "../src/parse.js";
import { defaultCurriculumRoot, loadLessons } from "../src/loader.js";

/** A minimal lesson in `language` whose body is `body`. */
const lesson = (body: string, language = "japanese") =>
  parseLesson(
    [
      "---",
      "schema_version: 2",
      "id: JA-W01-ko",
      "chapter: 1",
      "type: word",
      "headword: x",
      "gloss: x",
      "---",
      "",
      body,
    ].join("\n"),
    language,
  );

/** The rules that fire on one body, in order. */
const rules = (body: string, language = "japanese") =>
  measureLostScript([lesson(body, language)]).findings.map((f) => f.rule);

describe("lost script: the damage that shipped", () => {
  // Every line here is verbatim from the corpus before the fix.
  it.each([
    ["[PAUSE 15s] Say ??? and tap all three morae before writing the new sign.", ["run"]],
    ["[PAUSE 15s] Cover the model and write ?? from its two known signs.", ["run"]],
    ["[PAUSE 15s] Write ? once from memory, then add the two dakuten strokes.", ["lone"]],
    ["[PAUSE 15s] Write ? from memory before tracing ?.", ["lone", "lone"]],
    ["[PAUSE 15s] Before the new sign, write ? once and then ? once from memory.", ["lone", "lone"]],
    [
      "[PAUSE 15s] Write ?, then write the different sign that closes ????? and explain its particle reading.",
      ["lone", "run"],
    ],
    [
      "[PAUSE 15s] Say ?????, add the dakuten to ?, and recall how ?hard to exist? became thanks.",
      ["run", "lone", "glued"],
    ],
    ["[PAUSE 15s] Build ? once from ?, ?, and ? before joining the full word.", ["lone", "lone", "lone", "lone"]],
    ["From memory, write ??, ???, ?????, ?????, ???, and ????.", ["run", "run", "run", "run", "run", "run"]],
  ])("flags %s", (body, expected) => {
    expect(rules(body)).toEqual(expected);
  });

  it("flags a lone mark whose sentence carries on across a wrapped line", () => {
    // Lessons wrap at 80 columns, so `write ?` can end a source line.
    expect(rules("Cover the model and write ?\nonce from its two known signs.")).toEqual(["lone"]);
  });

  it("flags a lost accented letter inside a Latin-script word", () => {
    // The same encoder turns `español` into `espa?ol`. No such line exists in
    // the corpus today; this pins that the glued rule would see one.
    expect(rules("¿Hablas espa?ol?", "spanish")).toEqual(["glued"]);
  });

  it("flags a bolded run, where the markers sit right against the marks", () => {
    expect(rules("Read **?????** aloud.")).toEqual(["run"]);
  });

  it("flags a hit inside an hl-activity prompt, which the learner reads", () => {
    expect(rules('<!-- hl-activity: {"prompt":"Write ?? from memory."} -->')).toEqual(["run"]);
  });

  it("reports the line the damage is on", () => {
    const report = measureLostScript([lesson("clean line\n\nstill clean\nSay ??? now")]);
    // Body lines: 1 blank (after the frontmatter), 2 clean, 3 blank, 4 clean, 5 the hit.
    expect(report.findings.map((f) => f.line)).toEqual([5]);
    expect(report.findings[0]!.where).toBe("JA-W01-ko");
    expect(report.findings[0]!.excerpt).toContain("Say ??? now");
  });
});

describe("lost script: punctuation that is not damage", () => {
  it.each([
    // The review lessons' doubled mark: a gloss ending in `?` meets the prompt's
    // own `?`. The word is present, so nothing was lost.
    ["Why?? (**quārē**.) Why?? (**quamobrem**.)"],
    ["Surely ... not?? (**مگر**, *magar*.)"],
    // An ordinary question, in English and in a target script.
    ["What is *ikkaḍa*? (here — where I am.)"],
    ["How many beats is じゅ, and how many is いっ?"],
    ["- [YOU SAY: is it ...?, no, that is not right]"],
    // A quiz blank and a table blank: the mark is followed by an arrow or a pipe.
    ["Walk the erosion: *quo modo* → ? → *cómo*."],
    ["| retroflex | **ટ** *ṭa* | ? |"],
    // Punctuation lessons naming the mark itself.
    ["# ? and ! — the marks that go only at the end"],
    ["# ? — the mark you have been reading all along"],
    ["| . , ? | : ( ) \" \" — |"],
    ["(**Nothing but the voice** — and the ¿ ? marks.)"],
    // Quoting rather than emitting: code and URLs.
    ["Use `a ?? b` in TypeScript."],
    ["```\nconst x = y ?? z;\nwrite ? once\n```"],
    ["Source: [Marugoto](https://words.marugotoweb.jp/search_detail.php?cd=1&id=435&lang=en)."],
  ])("does not flag %s", (body) => {
    expect(rules(body)).toEqual([]);
  });

  it("allows French's spaced question mark, and only the lone rule", () => {
    // French typography: `Combien ? has had no answer` is the lone shape exactly.
    expect(rules("Combien ? has had no possible answer since.", "french")).toEqual([]);
    expect(rules("Comment ça va ? je ne sais pas.", "french")).toEqual([]);
    // A spaced `?` is never doubled or glued, so those rules stay on in French.
    expect(rules("Say ??? now.", "french")).toEqual(["run"]);
    expect(rules("fran?ais", "french")).toEqual(["glued"]);
    // And the allowance is French's, not everyone's.
    expect(rules("Combien ? has had no possible answer since.", "spanish")).toEqual(["lone"]);
  });
});

describe("lost script: the rendered layer", () => {
  it("scans generated LaTeX with the track's own allowance", () => {
    const report = measureLostScript(
      [],
      [
        { path: "japanese/book/chapters/ch02.tex", language: "japanese", text: "Say ??? and tap.\n" },
        { path: "french/book/chapters/ch44.tex", language: "french", text: "Combien ? has had no answer.\n" },
      ],
    );
    expect(report.summary.renderedFindings).toBe(1);
    expect(report.findings[0]!.layer).toBe("rendered");
    expect(report.summary.filesScanned).toBe(2);
  });

  it("does not apply Markdown's code exemption to LaTeX, where a backtick is a quote", () => {
    const report = measureLostScript([], [{ path: "x.tex", language: "x", text: "``Say ??? now''" }]);
    expect(report.summary.renderedFindings).toBe(1);
  });
});

describe("lost script: rendering", () => {
  it("states a clean run rather than printing nothing", () => {
    expect(renderLostScript(measureLostScript([lesson("clean")])).join("\n")).toContain("no \"?\" standing in");
  });

  it("names each finding and cannot be repainted by it", () => {
    const text = renderLostScript(measureLostScript([lesson("Say ??? \u001b[2J now")])).join("\n");
    expect(text).toContain("JA-W01-ko:2");
    expect(text).not.toContain("\u001b");
  });

  it("caps the listing at 25 and says how many more", () => {
    const text = renderLostScript(measureLostScript([lesson("Say ?? now.\n".repeat(30))])).join("\n");
    expect(text).toContain("... and 5 more");
  });
});

describe("lost script: linear time", () => {
  // Each input targets one pattern or blanking pass at 50,000 characters. A
  // quadratic pass takes tens of seconds here; a linear one, milliseconds.
  const N = 50_000;
  it.each([
    ["a single run of marks", "?".repeat(N)],
    ["a run after a word, refused at every position", "a" + "?".repeat(N)],
    ["alternating lone marks", "? ".repeat(N / 2)],
    ["one mark and a long whitespace run", "?" + " ".repeat(N) + "x"],
    ["one mark, whitespace, and a near-miss sibling list", " ?" + " ".repeat(N) + "and"],
    ["unterminated fences", "```".repeat(N / 3)],
    ["unterminated inline code", "`" + "?".repeat(N)],
    ["a URL of marks", "http://" + "?".repeat(N)],
    ["glued marks", "?a".repeat(N / 2)],
  ])("%s", (_name, body) => {
    const started = Date.now();
    measureLostScript([lesson(body)]);
    measureLostScript([], [{ path: "x.tex", language: "x", text: body }]);
    expect(Date.now() - started).toBeLessThan(1_000);
  });
});

/** Every committed `.tex` under `<track>/book/`, as rendered-layer input. */
function committedBookFiles(root: string): { path: string; language: string; text: string }[] {
  const out: { path: string; language: string; text: string }[] = [];
  const walk = (dir: string): void => {
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      const full = join(dir, entry.name);
      if (entry.isDirectory()) walk(full);
      else if (entry.isFile() && entry.name.endsWith(".tex")) {
        const path = relative(root, full);
        out.push({ path, language: path.split(/[\\/]/)[0] ?? "", text: readFileSync(full, "utf8") });
      }
    }
  };
  for (const track of readdirSync(root, { withFileTypes: true })) {
    if (!track.isDirectory()) continue;
    const book = join(root, track.name, "book");
    try {
      walk(book);
    } catch (error) {
      // A track with no book yet is normal; anything else is a real failure.
      if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error;
    }
  }
  return out;
}

describe("lost script: the corpus", () => {
  // The COMMITTED books, not a fresh render: `check:books` already pins every
  // committed .tex byte-for-byte to the generator's output, so the two are the
  // same text, and reading files keeps this gate off the cost of rendering
  // every book of every track a second time.
  const root = defaultCurriculumRoot();

  it("THE GATE: no lesson and no generated book has a '?' standing in for a word", () => {
    const lessons = loadLessons(root);
    const books = committedBookFiles(root);
    const report = measureLostScript(lessons, books);
    // Named rather than counted, so whoever breaks this knows which file to open.
    expect(report.findings.map((f) => `${f.where}:${f.line} [${f.rule}] ${f.excerpt}`)).toEqual([]);
    expect(lessons.length).toBeGreaterThan(1_000);
    expect(books.length).toBeGreaterThan(100);
  });

  it("is not vacuous: the same corpus plus one damaged line fires once", () => {
    const lessons = [...loadLessons(root), lesson("[PAUSE 15s] Say ??? and tap all three morae.")];
    expect(measureLostScript(lessons).summary.sourceFindings).toBe(1);
  });
});
