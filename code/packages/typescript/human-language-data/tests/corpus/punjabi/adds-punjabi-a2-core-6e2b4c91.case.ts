import { readFileSync } from "node:fs";
import { expect, it } from "vitest";
import {
  loadTrackChapters,
  loadTrackLanguageCurriculum,
  loadTrackLessons,
} from "../../../src/loader.js";
import { lessonModalities, orderChapterLessons, unstartableCause } from "../../../src/modality.js";

// The Punjabi A2 core: chapters 211-224, hand-authored. It realizes the three
// A2 spine nodes chapters 176-210 left unrealized (NEGATE-AND-ASK,
// TALK-ABOUT-PAST, TALK-ABOUT-FUTURE) with one canonical tag each, adds the
// copy-only letters ਔ and ਢ, and keeps every chapter startable by ear.
const CORE = Array.from({ length: 14 }, (_, index) => 211 + index);
const CANONICAL: Record<string, { chapter: number; node: string; headword: string }> = {
  "VERB-NEGATE": { chapter: 212, node: "SPINE-NEGATE-AND-ASK", headword: "ਮੈਂ ਚਾਹ ਨਹੀਂ ਪੀਂਦਾ ਹਾਂ।" },
  "QUESTION-POLAR": { chapter: 214, node: "SPINE-NEGATE-AND-ASK", headword: "ਕੀ ਤੁਸੀਂ ਯਾਤਰੀ ਹੋ?" },
  "VERB-PAST": { chapter: 216, node: "SPINE-TALK-ABOUT-PAST", headword: "ਗਿਆ · ਗਈ" },
  "VERB-FUTURE": { chapter: 221, node: "SPINE-TALK-ABOUT-FUTURE", headword: "ਜਾਵਾਂਗਾ · ਜਾਵਾਂਗੀ" },
};
const WORDS: Record<string, number> = { "ਅਜੇ": 213, "ਕੱਲ੍ਹ": 216, "ਔਰਤ": 217, "ਕੱਢਣਾ": 219, "ਸ਼ਾਇਦ": 223 };
const LETTERS: Record<string, { id: string; anchor: string }> = {
  "ਔ": { id: "PA-C217-au", anchor: "PA-C217-aurat" },
  "ਢ": { id: "PA-C219-ddha", anchor: "PA-C219-kaddhna" },
};

it("opens every Punjabi A2 core chapter by ear", () => {
  const lessons = loadTrackLessons("punjabi");
  for (const chapter of CORE) {
    const ordered = lessons
      .filter((lesson) => Number(lesson.frontmatter.chapter) === chapter)
      .sort((a, b) => Number(a.frontmatter.sequence) - Number(b.frontmatter.sequence));
    expect(ordered.length, `chapter ${chapter}`).toBeGreaterThanOrEqual(5);
    const [entry] = orderChapterLessons(lessonModalities([ordered[0]!]));
    expect(entry?.coreModality, `${ordered[0]!.realization.lessonId}`).toBe("voice");
    expect(unstartableCause([entry!])).toBeNull();
    expect(ordered[0]!.frontmatter.type).toBe("review");
    expect(ordered[0]!.frontmatter["introduces.knowledge"]).toEqual([]);
    expect(ordered.at(-1)!.frontmatter.type).toBe("review");
    expect(ordered.at(-1)!.frontmatter["introduces.knowledge"]).toEqual([]);
  }
});

it("puts each canonical A2 grammar tag on exactly one core lesson of its own node", () => {
  const lessons = loadTrackLessons("punjabi");
  for (const [tag, claim] of Object.entries(CANONICAL)) {
    const owners = lessons.filter((lesson) => lesson.frontmatter.concept_tag === tag);
    expect(owners, tag).toHaveLength(1);
    expect(Number(owners[0]!.frontmatter.chapter)).toBe(claim.chapter);
    expect(owners[0]!.frontmatter.spine_node).toBe(claim.node);
    expect(owners[0]!.frontmatter.type).toBe("grammar");
    expect(owners[0]!.realization.headword).toBe(claim.headword);
  }
  for (const spine of ["0210-SPINE-NEGATE-AND-ASK", "0220-SPINE-TALK-ABOUT-PAST", "0230-SPINE-TALK-ABOUT-FUTURE"]) {
    const ledger = JSON.parse(readFileSync(new URL(
      `../../../../../../learning/human-languages/punjabi/curriculum.d/spine/${spine}.json`,
      import.meta.url,
    ), "utf8")) as { omits: string[] };
    expect(ledger.omits, spine).toEqual([]);
  }
});

it("keeps the negated present's helper and teaches requests only on consonant stems", () => {
  const lessons = loadTrackLessons("punjabi").filter((lesson) => Number(lesson.frontmatter.chapter) >= 211);
  const copy = lessons.flatMap((lesson) => lesson.blocks.map((block) => block.markdown)).join("\n");
  expect(copy).toContain("ਮੈਂ ਚਾਹ ਨਹੀਂ ਪੀਂਦਾ ਹਾਂ।");
  expect(copy).not.toMatch(/ਨਹੀਂ (?:ਪੀਂਦ|ਖਾਂਦ|ਜਾਂਦ)[ਾੀੇ][।?]/u);
  expect(copy).not.toContain("ਓ");
  expect(copy).toContain("ਨਾ ਉੱਠੋ।");
  // The ergative frame: no ਮੈਂ ਨੇ, no ਉਹ ਨੇ; a third-person doer is a name or ਉਸ.
  expect(copy).not.toMatch(/ਮੈਂ ਨੇ|ਉਹ ਨੇ|ਉਨ੍ਹਾਂ ਨੇ/u);
  expect(copy).toContain("ਅਮਨ ਨੇ ਕਿਤਾਬ ਖ਼ਰੀਦੀ।");
  expect(copy).toContain("ਉਸ ਨੇ ਦੁੱਧ ਪੀਤਾ।");
  expect(copy).not.toMatch(/Platts|Steingass|Lane|Wiktionary|learnpunjabi/u);
});

it("teaches five A2 words with a visible source and two copy-only letters after their anchors", () => {
  const lessons = loadTrackLessons("punjabi");
  for (const [headword, chapter] of Object.entries(WORDS)) {
    const owners = lessons.filter((lesson) =>
      lesson.frontmatter.type === "word" && lesson.realization.headword === headword,
    );
    expect(owners, headword).toHaveLength(1);
    expect(Number(owners[0]!.frontmatter.chapter)).toBe(chapter);
    expect(String(owners[0]!.frontmatter.etymology_hook ?? "")).not.toBe("");
    const apart = owners[0]!.blocks.find((block) => block.title.includes("taken apart"));
    expect(apart?.markdown, headword).toMatch(/CDIAL/u);
  }
  for (const [glyph, { id, anchor }] of Object.entries(LETTERS)) {
    const letter = lessons.find((lesson) => lesson.realization.lessonId === id)!;
    const word = lessons.find((lesson) => lesson.realization.lessonId === anchor)!;
    expect(letter.frontmatter).toMatchObject({ type: "writing", delivery: "script", headword: glyph });
    expect(Number(letter.frontmatter.sequence)).toBe(Number(word.frontmatter.sequence) + 10);
    const copy = letter.blocks.map((block) => block.markdown).join("\n");
    expect(copy).toMatch(/no stroke order.*verified source/isu);
    expect(copy).toMatch(/model visible.*not writing from memory/isu);
    const earlier = lessons.filter((lesson) =>
      Number(lesson.frontmatter.sequence) < Number(word.frontmatter.sequence),
    );
    expect(
      earlier.some((lesson) => lesson.blocks.some((block) => block.markdown.includes(glyph))),
      `${glyph} shows before its anchor word`,
    ).toBe(false);
  }
});

it("revisits the two thin script atoms inside the core and stays inside the lesson budgets", () => {
  const lessons = loadTrackLessons("punjabi");
  const inChapter = (chapter: number, atom: string) => lessons.filter((lesson) =>
    Number(lesson.frontmatter.chapter) === chapter &&
    lesson.frontmatter["practises.knowledge"].includes(atom),
  );
  expect(inChapter(214, "PA-SCRIPT-RECOG-YA-01").length).toBeGreaterThanOrEqual(2);
  expect(inChapter(216, "PA-SCRIPT-RECOG-E-206").length).toBeGreaterThanOrEqual(2);
  const core = lessons.filter((lesson) => Number(lesson.frontmatter.chapter) >= 211);
  expect(core.length).toBeGreaterThanOrEqual(80);
  for (const lesson of core) {
    expect(Number(lesson.frontmatter["duration.max_seconds"])).toBeLessThan(300);
    expect(lesson.frontmatter.introduces_idioms).toEqual([]);
    expect(lesson.frontmatter.introduces_senses).toEqual([]);
    expect(lesson.frontmatter.introduces_culture_claims).toEqual([]);
  }
  const ledger = loadTrackChapters().find((track) => track.language === "punjabi");
  const curriculum = loadTrackLanguageCurriculum("punjabi");
  let previous = "PA-EXT-210-A2-NE";
  for (const chapter of CORE) {
    const entry = ledger?.chapters.find((candidate) => candidate.chapter === chapter);
    expect(entry?.payoff.assesses.length, `chapter ${chapter}`).toBeGreaterThan(0);
    const extension = curriculum?.extensions.find((candidate) =>
      candidate.id.startsWith(`PA-EXT-${chapter}-`),
    );
    expect(extension).toMatchObject({ stage: "A2", kind: "required" });
    expect(extension?.prerequisites).toContain(previous);
    previous = extension!.id;
  }
}, 120_000);
