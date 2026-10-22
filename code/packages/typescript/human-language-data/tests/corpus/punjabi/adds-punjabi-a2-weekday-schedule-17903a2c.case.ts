import { readFileSync } from "node:fs";
import { expect, it } from "vitest";
import {
  loadTrackChapters,
  loadTrackLanguageCurriculum,
  loadTrackLessons,
} from "../../../src/loader.js";

it("adds a slow Punjabi A2 weekday-schedule prerequisite without mock credit", () => {
  const lessons = loadTrackLessons("punjabi");
  const ids = [
    "PA-C182-somvar",
    "PA-C182-mangalvar",
    "PA-C182-budhvar",
    "PA-C182-veervar",
    "PA-C182-shukkarvar",
    "PA-C182-shanivar",
    "PA-C182-aitvar",
    "PA-C182-airhaa-ai",
    "PA-C182-weekday-review",
    "PA-C182-payoff",
  ];
  const chapterLessons = ids.map((id) =>
    lessons.find((candidate) => candidate.realization.lessonId === id),
  );
  expect(chapterLessons.every(Boolean)).toBe(true);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.chapter)))
    .toEqual(ids.map(() => 182));
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.sequence)))
    .toEqual([9550, 9560, 9570, 9580, 9590, 9600, 9610, 9620, 9630, 9640]);

  const atoms = [
    "PA-WEEKDAY-SOMVAR-01",
    "PA-WEEKDAY-MANGALVAR-01",
    "PA-WEEKDAY-BUDHVAR-01",
    "PA-WEEKDAY-VEERVAR-01",
    "PA-WEEKDAY-SHUKKARVAR-01",
    "PA-WEEKDAY-SHANIVAR-01",
    "PA-WEEKDAY-AITVAR-01",
  ];
  const scriptAtom = "PA-SCRIPT-RECOG-AI-01";
  const assessedAtoms = [...atoms, scriptAtom];
  expect(chapterLessons.map((lesson) => lesson?.frontmatter["introduces.knowledge"]))
    .toEqual(atoms.map((atom) => [atom]).concat([[scriptAtom], [], []]));

  const newForms = [
    "ਸੋਮਵਾਰ",
    "ਮੰਗਲਵਾਰ",
    "ਬੁੱਧਵਾਰ",
    "ਵੀਰਵਾਰ",
    "ਸ਼ੁੱਕਰਵਾਰ",
    "ਸ਼ਨੀਵਾਰ",
    "ਐਤਵਾਰ",
  ];
  for (const [ownerIndex, form] of newForms.entries()) {
    const earlierCopy = chapterLessons.slice(0, ownerIndex)
      .flatMap((lesson) => lesson?.blocks.map((block) => block.markdown) ?? [])
      .join("\n");
    expect(earlierCopy.includes(form), `${form} appears before its owner lesson`)
      .toBe(false);
  }

  for (const lesson of chapterLessons) {
    expect(Number(lesson?.frontmatter["duration.max_seconds"]))
      .toBeLessThanOrEqual(300);
    expect(lesson?.frontmatter.spine_node).toBe("SPINE-READ-PRACTICAL-TEXTS");
    expect(lesson?.frontmatter.introduces_idioms).toEqual([]);
    expect(lesson?.frontmatter.introduces_senses).toEqual([]);
    expect(lesson?.frontmatter.introduces_culture_claims).toEqual([]);
  }

  for (const atom of assessedAtoms) {
    const owners = lessons.filter((lesson) =>
      lesson.frontmatter["introduces.knowledge"].includes(atom),
    );
    expect(owners).toHaveLength(1);
    const ownerSequence = Number(owners[0].frontmatter.sequence);
    const laterPractices = chapterLessons.filter((lesson) =>
      Number(lesson?.frontmatter.sequence) > ownerSequence &&
      lesson?.frontmatter["practises.knowledge"].includes(atom),
    );
    expect(laterPractices.length, `${atom} needs two later retrieval lessons`)
      .toBeGreaterThanOrEqual(2);
  }

  const learnerCopy = chapterLessons
    .flatMap((lesson) => lesson?.blocks.map((block) => block.markdown) ?? [])
    .join("\n").replace(/\s+/gu, " ");
  for (const form of newForms) {
    expect(learnerCopy).toContain(form);
  }
  expect(learnerCopy).toContain(
    "ਸੋਮਵਾਰ ਤੋਂ ਸ਼ੁੱਕਰਵਾਰ ਤੱਕ ਖੁੱਲ੍ਹਾ। ਸ਼ਨੀਵਾਰ ਅਤੇ ਐਤਵਾਰ ਬੰਦ।",
  );
  expect(learnerCopy).toMatch(/fifth prerequisite tranche|does not unblock.*mock 1/is);
  expect(learnerCopy).toMatch(/not.*A2 mock|not A2 readiness/is);

  const taughtBefore = lessons
    .filter((lesson) => Number(lesson.frontmatter.sequence) < 9550)
    .flatMap((lesson) => [
      String(lesson.frontmatter.headword ?? ""),
      ...lesson.blocks.map((block) => block.markdown),
    ])
    .join("\n");
  const supportingPunjabi = new Set(learnerCopy.match(/[\u0A00-\u0A7F]+/gu) ?? []);
  for (const newlyTaught of newForms) {
    supportingPunjabi.delete(newlyTaught);
  }
  supportingPunjabi.delete("ਐ");
  for (const token of supportingPunjabi) {
    expect(taughtBefore.includes(token), `${token} must occur before chapter 182`).toBe(true);
  }

  for (const lesson of chapterLessons.slice(0, 6)) {
    const copy = lesson?.blocks.map((block) => block.markdown).join("\n") ?? "";
    expect(copy.indexOf("[YOU HEAR:"), `${lesson?.realization.lessonId} needs an oral bridge`)
      .toBeGreaterThanOrEqual(0);
    expect(copy.indexOf("[YOU HEAR:")).toBeLessThan(copy.indexOf("[YOU READ:"));
    expect(copy).not.toContain("[YOU WRITE:");
  }
  const oralSunday = chapterLessons[6]!;
  const oralSundayCopy = oralSunday.blocks.map((block) => block.markdown).join("\n");
  expect(oralSunday.frontmatter.type).toBe("listening-speaking");
  expect(oralSunday.frontmatter.skills).toEqual(["listening", "speaking"]);
  expect(oralSundayCopy).not.toContain("ਐ");
  expect(oralSundayCopy).not.toContain("[YOU READ:");
  const oralSundaySource = readFileSync(new URL(
    "../../../../../../learning/human-languages/punjabi/lessons/PA-C182-aitvar.md",
    import.meta.url,
  ), "utf8");
  const oralSundayBody = oralSundaySource.replace(/^---[\s\S]*?---\s*/u, "");
  expect(oralSundayBody).not.toContain("ਐ");

  const aiLetter = chapterLessons[7]!;
  const aiLetterCopy = aiLetter.blocks.map((block) => block.markdown).join("\n");
  expect(aiLetter.frontmatter.type).toBe("writing");
  expect(aiLetterCopy).toContain("ਐ");
  expect(aiLetterCopy).not.toContain("ਐਤਵਾਰ");
  expect(aiLetterCopy).toMatch(/No stroke order is taught here/i);
  const aiLetterSource = readFileSync(new URL(
    "../../../../../../learning/human-languages/punjabi/lessons/PA-C182-airhaa-ai.md",
    import.meta.url,
  ), "utf8");
  expect(aiLetterSource).not.toContain("ਐਤਵਾਰ");

  const generatedBook = readFileSync(new URL(
    "../../../../../../learning/human-languages/punjabi/book/chapters/" +
      "ch182-a-week-on-the-noticeboard.tex",
    import.meta.url,
  ), "utf8");
  const aiLessonLabel = generatedBook.indexOf("label{lesson:PA-C182-airhaa-ai}");
  expect(aiLessonLabel).toBeGreaterThan(0);
  const aiSectionStart = generatedBook.lastIndexOf("\\section[", aiLessonLabel);
  expect(aiSectionStart).toBeGreaterThan(0);
  expect(generatedBook.slice(0, aiSectionStart)).not.toContain("ਐ");
  const reviewLabel = generatedBook.indexOf("label{lesson:PA-C182-weekday-review}");
  expect(reviewLabel).toBeGreaterThan(aiSectionStart);
  const reviewSectionStart = generatedBook.lastIndexOf("\\section[", reviewLabel);
  expect(reviewSectionStart).toBeGreaterThan(aiSectionStart);
  expect(generatedBook.slice(0, reviewSectionStart)).not.toContain("ਐਤਵਾਰ");

  const reviewCopy = chapterLessons[8]!.blocks.map((block) => block.markdown).join("\n");
  expect(reviewCopy).toContain("ਐਤਵਾਰ");
  const payoff = chapterLessons.at(-1);
  expect(payoff?.frontmatter.skills)
    .toEqual(["listening", "speaking", "reading", "writing"]);
  expect(payoff?.blocks.some((block) => block.markdown.includes("[YOU WRITE:")))
    .toBe(true);

  const ledger = loadTrackChapters().find((track) => track.language === "punjabi");
  expect(ledger?.chapters.find((chapter) => chapter.chapter === 182)).toMatchObject({
    chapter: 182,
    payoff: { lesson: "PA-C182-payoff", kind: "production", assesses: assessedAtoms },
  });
  const curriculum = loadTrackLanguageCurriculum("punjabi");
  const extension = curriculum?.extensions.find((candidate) =>
    candidate.id === "PA-EXT-182-A2-WEEKDAY-SCHEDULE",
  );
  expect(extension).toMatchObject({ stage: "A2", kind: "required", category: "vocabulary" });
  expect(extension?.prerequisites).toContain("PA-EXT-181-A2-TRANSPORT");
}, 120_000);
