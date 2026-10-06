import { expect, it } from "vitest";
import { measureContinuity } from "../../../src/continuity.js";
import { loadTrackLessons } from "../../../src/loader.js";
import { readingOrder } from "../../../src/ramp.js";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
  languageWritingStages,
} from "../assert-language-corpus.js";

it("keeps Marathi's opening script runways below the chapter atom budget", () => {
  const ordered = loadTrackLessons("marathi").sort(readingOrder);
  const opening = ordered.slice(0, 14);
  expect(opening.map((lesson) => lesson.realization.lessonId)).toEqual([
    "MR-C01-namaskar",
    "MR-W01-ha",
    "MR-W01-o-matra",
    "MR-W01-ho-delayed-copy",
    "MR-W01-ho-dictation",
    "MR-W01-na",
    "MR-W01-aa-matra",
    "MR-W01-ii-matra",
    "MR-W01-ma",
    "MR-W01-sa",
    "MR-W01-ka",
    "MR-W01-virama",
    "MR-W01-ra",
    "MR-W01-namaskar-read",
  ]);
  expect(opening.every((lesson) => lesson.frontmatter.chapter === "1")).toBe(true);

  // HL-C443: each runway chapter after the first opens with short words the
  // reader says before writing their letters, so a letter is a piece of a word
  // already known rather than a shape on its own. Chapter 1 has no room: it is
  // at the twelve-atom budget, and its cold letters are written from हो in the
  // lesson itself.
  const firstDoorway = ordered.slice(14, 25);
  expect(firstDoorway.map((lesson) => lesson.realization.lessonId)).toEqual([
    "MR-C01-dhanyavad",
    "MR-C02-dukh",
    "MR-C02-aabhaal",
    "MR-C02-sant",
    "MR-C02-bet",
    "MR-W02-visarga",
    "MR-W02-aa-independent",
    "MR-W02-bha",
    "MR-W02-e-matra",
    "MR-W02-anusvara",
    "MR-W02-ta",
  ]);
  expect(firstDoorway.every((lesson) => lesson.frontmatter.chapter === "2")).toBe(true);

  const secondDoorway = ordered.slice(25, 32);
  expect(secondDoorway.map((lesson) => lesson.realization.lessonId)).toEqual([
    "MR-W03-da",
    "MR-W03-dha",
    "MR-W03-ba",
    "MR-W03-ya",
    "MR-W03-lla",
    "MR-W03-va",
    "MR-W03-dhanyavad-write",
  ]);
  expect(secondDoorway.every((lesson) => lesson.frontmatter.chapter === "3")).toBe(true);

  // The SECOND runway, chapters 5-8. It exists because closure is measured in
  // READING ORDER: twenty-three of the twenty-four signs below were already
  // somewhere in the corpus, but the earliest lesson that could be said to
  // teach them sat at reading position 112, and every lesson from chapter 9
  // onward that used them therefore asked the reader to decode something they
  // had not been shown. Teaching more letters later moved nothing; teaching
  // these letters HERE retired all forty-four violations at once.
  //
  // Pinned as an ordered list, not a count, because the order is the argument:
  // marks first (they block the most lessons), then two consonant rows that
  // teach the voice/breath pattern, then the retroflex row, then the leftovers.
  const secondRunway = ordered.slice(37, 80);
  expect(secondRunway.map((lesson) => lesson.realization.lessonId)).toEqual([
    "MR-C05-amrut",
    "MR-C05-kiran",
    "MR-C05-tup",
    "MR-W05-i-matra",
    "MR-W05-u-matra",
    "MR-W05-uu-matra",
    "MR-W05-ru-matra",
    "MR-W05-candrabindu",
    "MR-W05-a-independent",
    "MR-R05-marks-recall",
    "MR-C06-ghagar",
    "MR-C06-chav",
    "MR-C06-chatri",
    "MR-C06-jahaj",
    "MR-C06-jhoka",
    "MR-W06-kha",
    "MR-W06-ga",
    "MR-W06-gha",
    "MR-W06-ca",
    "MR-W06-cha",
    "MR-W06-ja",
    "MR-W06-jha",
    "MR-R06-two-rows-recall",
    "MR-C07-taali",
    "MR-C07-thaam",
    "MR-C07-daba",
    "MR-W07-tta",
    "MR-W07-ttha",
    "MR-W07-dda",
    "MR-W07-nna",
    "MR-W07-pa",
    "MR-R07-retroflex-recall",
    "MR-C08-ulat",
    "MR-C08-shesh",
    "MR-C08-uub",
    "MR-C08-ekda",
    "MR-W08-la",
    "MR-W08-sha",
    "MR-W08-ssa",
    "MR-W08-u-independent",
    "MR-W08-uu-independent",
    "MR-W08-e-independent",
    "MR-R08-runway-recall",
  ]);
  expect(
    secondRunway.every((lesson) => ["5", "6", "7", "8"].includes(lesson.frontmatter.chapter)),
  ).toBe(true);
  // Every sign lesson teaches exactly one letter, and each chapter closes with a
  // retrieval payoff that adds none.
  expect(secondRunway.filter((lesson) => lesson.realization.type === "writing")).toHaveLength(24);
  expect(secondRunway.filter((lesson) => lesson.realization.type === "review")).toHaveLength(4);
  // Fifteen anchor words, one atom each: chapters 5-8 carry 9, 12, 8 and 10.
  expect(secondRunway.filter((lesson) => lesson.realization.type === "word")).toHaveLength(15);

  const chapterSizes = new Map<string, number>();
  for (const lesson of ordered) {
    const chapter = lesson.frontmatter.chapter;
    chapterSizes.set(chapter, (chapterSizes.get(chapter) ?? 0) + 1);
  }
  expect([...chapterSizes.entries()]).toEqual([
    ["1", 14],
    // Chapter 2 gains four anchor words (7 -> 11), chapters 5-8 fifteen more.
    ["2", 11],
    ["3", 7],
    ["4", 5],
    // Chapters 5-8 are the second script runway. Everything from here down used
    // to sit four numbers lower; inserting four chapters rather than stretching
    // an existing one is what keeps every chapter under the twelve-atom budget
    // while still putting all twenty-four signs BEFORE the lessons that need
    // them. Length is never a cost in this corpus, so splitting was free.
    ["5", 10],
    ["6", 13],
    ["7", 9],
    ["8", 11],
    // Chapter 9 gains one ear-only reach-back and chapter 13 four more: the
    // twenty-four new atoms need R2 and R3 retrieval, and those windows fall
    // inside chapters that already existed.
    ["9", 10],
    ["10", 6],
    // 6 -> 7 and, below, 65: 5 -> 6. HL-C432 adds one `review` lesson to each --
    // the first meeting retrieved as one exchange, and the three script marks
    // from chapter 64-65. `chapterSizes` counts LESSONS, not atoms, and both
    // lessons introduce none, so the chapter atom budget this test is named for
    // is untouched.
    ["11", 7],
    ["12", 5],
    ["13", 10],
    ["14", 6],
    ["15", 4],
    ["16", 4],
    ["17", 4],
    ["18", 6],
    ["19", 4],
    ["20", 1],
    ["21", 12],
    // Chapter 22 is where R4 lands -- eighty lessons or more after each sign
    // was taught, which is the whole point of the window.
    ["22", 9],
    ["23", 10],
    ["24", 18],
    ["25", 10],
    // Chapters 26-29 are the pre-A1 verb tranche. They sit AFTER the A1 writing
    // runways in book order while realizing pre-A1 spine nodes, which is the
    // shape the other Indic tracks already use: a node's level is a property of
    // the node, not of where the chapter falls in the book.
    ["26", 5],
    ["27", 5],
    ["28", 5],
    ["29", 4],
    // Chapters 30-36 are the joining tranche: three flat conjunctions, three
    // paired shapes, sentence negation, the polar particle, why and because,
    // and clause subordination. Seven chapters rather than three, because at
    // three new atoms per lesson the material does not fit in fewer -- and
    // because every one of them closes on a review, which is what held the
    // tranche's own reinforcement misses at zero while the track grew by 25.
    ["30", 4],
    ["31", 4],
    ["32", 3],
    ["33", 3],
    ["34", 4],
    ["35", 3],
    ["36", 4],
    // Chapters 37-40 are the asking-word tranche: koṇ, kitī, kuṭhe, kadhī, the
    // rule that says where an asking word STANDS, and the ithe/tithe pair that
    // answers a where-question without naming a place. Four chapters rather
    // than two, because the two script lessons this needed -- थ and the
    // independent इ -- must sit at least two lessons apart under
    // minLessonsBetweenScriptSegments, and each must land in the chapter
    // immediately before the word that spends it.
    ["37", 3],
    ["38", 4],
    ["39", 3],
    ["40", 3],
    // Chapters 41-44 are the place tranche: a house and a room, the oblique
    // stem those two nouns make visible, then five postpositions run off it and
    // three ordinary place words that are NOT postpositions. lāmb, ujvā and
    // ḍāvā sit in the same tranche on purpose -- the contrast with a
    // postposition is what makes the oblique rule falsifiable rather than
    // decorative.
    ["41", 4],
    ["42", 4],
    ["43", 4],
    ["44", 3],
    // Chapters 45-48 are the accompaniment tranche: the three endings English
    // calls "with", the pronoun's own oblique, the two ablatives and their
    // question word, the animate object marker, and -kade, which carries
    // possession in a language with no verb for having.
    ["45", 5],
    ["46", 4],
    ["47", 3],
    ["48", 3],
    // Chapters 49-52 are the adjective tranche, and they close
    // SPINE-DESCRIBE-QUALITIES -- an A1 core node this track had never
    // realized at all. Two classes are taught as a contrast rather than a
    // list: the -ā adjectives that agree in three genders, and the larger
    // class that never agrees, because a learner who meets only the first
    // over-applies its endings.
    ["49", 4],
    ["50", 4],
    ["51", 4],
    ["52", 4],
    // Chapters 53-54 teach NO adjective at all: they multiply the eight the
    // previous tranche taught. Three degrees (jaraa, khuup, and repetition),
    // a four-step scale of amount, and the exclamative use of a question word
    // the book has had since chapter 37.
    ["53", 4],
    ["54", 5],
    // Chapters 55-59 are the numbers. Fifteen numerals at one per lesson,
    // because Marathi's are the Sanskrit ones worn down and each has to be
    // learned rather than derived -- and two vowel SIGNS, placed where the
    // words that need them are: au in 56 for chaudaa, ai in 59 for paise.
    // Twenty comes before nineteen on purpose: ekoNiis is built as one LESS
    // than twenty, so the round number has to exist first.
    ["55", 5],
    // +1: HL-C443 anchor word मौज, before the ौ letter lesson.
    ["56", 6],
    ["57", 5],
    ["58", 5],
    ["59", 4],
    // Chapters 60-61 are the ordinals, and they are two chapters rather than
    // one because the set breaks in half. 60 carries the four Marathi
    // INHERITED -- pahilaa, dusraa, tisraa, chauthaa, in three different shapes
    // and joined by no rule -- and opens on SECOND, which costs no new word
    // because dusraa has been in the reader's mouth since chapter 31. 61
    // carries the five it BUILDS, every one of them the cardinal plus -vaa.
    // Six lessons each: five words and the chapter's own retrieval payoff.
    ["60", 6],
    ["61", 6],
    // chapter 62 — the reading rung: labels, a message, and an 83-word paragraph
    ["62", 3],
    // Chapter 63 is one lesson and always will be: the timed A1 writing paper.
    // It is the last of the seven writing stages, it introduces no atom, and it
    // adds only the condition the six stages before it deliberately withheld --
    // a clock the candidate does not control.
    ["63", 1],
    // Chapter 64: ddha, pha, the danda, and the cold-retrieval review that
    // keeps the danda from being an atom nothing revisits.
    // +2: HL-C443 anchor words ढोल and फणस, before the ढ and फ letter lessons.
    ["64", 6],
    // Chapter 65: the four standing vowels plus the review that keeps the last
    // of them from being an atom nothing revisits.
    // +3: HL-C443 anchor words ईश्वर, ओला and ऐवज, before the independent
    // ई, ओ and ऐ letter lessons.
    ["65", 9],
    // Chapter 66: the four parts of the day, the whole day, and the review that
    // keeps the last of them from being an atom nothing revisits.
    ["66", 6],
    // Chapter 67: asking and telling the time, placing an event at an hour, and
    // the review that keeps the last atom from being one nothing revisits.
    ["67", 3],
    // Chapter 68: three shubh greetings built on one new word, plus the review.
    ["68", 4],
    // Chapter 69: the near row, the far row, the system lesson that makes them
    // pronouns, and the review.
    ["69", 4],
    // Chapters 70-120: the pre-A1 vocabulary tranche, five word lessons each,
    // cut into three runs of seventeen chapters. The last chapter of each run
    // (86, 103, 120) also carries that run's two review lessons, which is why
    // they hold seven. Three runs rather than two keeps each review under the
    // 300-second lesson ceiling.
    ...Array.from({ length: 51 }, (_, i): [string, number] => [
      String(70 + i),
      [86, 103, 120].includes(70 + i) ? 7 : 5,
    ]),
    // Chapters 121-158: the A1 tranche, five word lessons each, in four runs
    // (121-130, 131-140, 141-149, 150-158). The last chapter of each run also
    // carries that run's two reviews.
    ...Array.from({ length: 38 }, (_, i): [string, number] => [
      String(121 + i),
      [130, 140, 149, 158].includes(121 + i) ? 7 : 5,
    ]),
    // Chapters 159-161: the A2 spine chapters (the past, the future, practical
    // texts), five word lessons each. Chapter 161 also carries their two
    // reviews.
    ["159", 5],
    ["160", 5],
    ["161", 7],
    // Chapters 162-192: A2 vocabulary tranche 1, five word lessons each, in runs
    // of at most nine chapters. The last chapter of each run (170, 179, 188, 192) also
    // carries that run's two reviews.
    ["162", 5],
    ["163", 5],
    ["164", 5],
    ["165", 5],
    ["166", 5],
    ["167", 5],
    ["168", 5],
    ["169", 5],
    ["170", 7],
    ["171", 5],
    ["172", 5],
    ["173", 5],
    ["174", 5],
    ["175", 5],
    ["176", 5],
    ["177", 5],
    ["178", 5],
    ["179", 7],
    ["180", 5],
    ["181", 5],
    ["182", 5],
    ["183", 5],
    ["184", 5],
    ["185", 5],
    ["186", 5],
    ["187", 5],
    ["188", 7],
    ["189", 5],
    ["190", 5],
    ["191", 5],
    ["192", 7],
    // Chapters 193-222: A2 vocabulary tranche 2, five word lessons each, in runs
    // of at most nine chapters. The last chapter of each run (201, 210, 216, 222) also
    // carries that run's two reviews.
    ["193", 5],
    ["194", 5],
    ["195", 5],
    ["196", 5],
    ["197", 5],
    ["198", 5],
    ["199", 5],
    ["200", 5],
    ["201", 7],
    ["202", 5],
    ["203", 5],
    ["204", 5],
    ["205", 5],
    ["206", 5],
    ["207", 5],
    ["208", 5],
    ["209", 5],
    ["210", 7],
    ["211", 5],
    ["212", 5],
    ["213", 5],
    ["214", 5],
    ["215", 5],
    ["216", 7],
    ["217", 5],
    ["218", 5],
    ["219", 5],
    ["220", 5],
    ["221", 5],
    ["222", 7],
    // Chapters 223-250: A2 vocabulary tranche 3, five word lessons each, in runs
    // of at most nine chapters. The last chapter of each run (231, 240, 245, 250) also
    // carries that run's two reviews.
    ["223", 5],
    ["224", 5],
    ["225", 5],
    ["226", 5],
    ["227", 5],
    ["228", 5],
    ["229", 5],
    ["230", 5],
    ["231", 7],
    ["232", 5],
    ["233", 5],
    ["234", 5],
    ["235", 5],
    ["236", 5],
    ["237", 5],
    ["238", 5],
    ["239", 5],
    ["240", 7],
    ["241", 5],
    ["242", 5],
    ["243", 5],
    ["244", 5],
    ["245", 7],
    ["246", 5],
    ["247", 5],
    ["248", 5],
    ["249", 5],
    ["250", 7],
    // Chapters 251-275: A2 vocabulary tranche 4, five word lessons each, in runs
    // of at most nine chapters. The last chapter of each run (259, 268, 275) also
    // carries that run's two reviews.
    ["251", 5],
    ["252", 5],
    ["253", 5],
    ["254", 5],
    ["255", 5],
    ["256", 5],
    ["257", 5],
    ["258", 5],
    ["259", 7],
    ["260", 5],
    ["261", 5],
    ["262", 5],
    ["263", 5],
    ["264", 5],
    ["265", 5],
    ["266", 5],
    ["267", 5],
    ["268", 7],
    ["269", 5],
    ["270", 5],
    ["271", 5],
    ["272", 5],
    ["273", 5],
    ["274", 5],
    ["275", 7],
  ]);
});
