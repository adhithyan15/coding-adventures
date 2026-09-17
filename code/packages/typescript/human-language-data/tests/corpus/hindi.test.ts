import { expect, it } from "vitest";
import { loadEverything, loadExamInventory, loadTrackLessons } from "../../src/loader.js";
import {
  formatExamCoverage,
  measureExamCoverage,
  trackIntroducedAtoms,
} from "../../src/exam-inventory.js";
import { readingOrder } from "../../src/ramp.js";
import { measureScriptClosure } from "../../src/script-closure.js";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
  languageWritingStages,
} from "./assert-language-corpus.js";
it("pins Hindi continuity", () => expectLanguageContinuity("hindi"));
it("pins Hindi modality", () => expectLanguageModality("hindi"));
it("pins Hindi lesson-content budgets", () =>
  expectLanguageLessonBudgets("hindi", {
    // 310 -> 343: this vocabulary tranche adds thirty-three lessons in seven
    // chapters (75-81), one new word each. Thirty-five were authored; kab and
    // kyon were cut, because #14113's joining tranche teaches both and a
    // headword introduced twice is a hard error. Re-measured against the tree.
    //
    // 343 -> 355: the ordinal tranche adds twelve lessons in three chapters
    // (82-84), one new item each, closing HI-A1-NUM-04.
    //
    // 355 -> 358: chapter 85, the reading rung — six signs, six notices, and a
    // 91-word paragraph. It adds NO new word: every token in all three was
    // checked to occur in a lesson with a lower sequence number, which is what
    // lets the lessons claim nothing in them is new. The count moves because
    // reading is its own skill, not because the track learned more Hindi.
    // 358 -> 359: the timed A1 writing paper. One lesson, no new atoms -- it
    // practises the form atoms the track already teaches and adds only the
    // clock, which is the last of the seven writing stages and the one Hindi
    // had never proven. Timing, task count, field count and word range are read
    // out of hindi/task-shapes/a1.json rather than invented for the lesson.
    // 366 -> 380: two adjective chapters. Chapter 90 authors the four shape
    // words (lambaa, naataa, patlaa, motaa) plus two retrieval lessons; chapter
    // 91 authors the six disposition words (buraa, hoshiyaar, mehnati,
    // sharmeela, milansaar, gambhir) plus two. Ten new atoms, four retrieval
    // lessons, budgeted rather than discovered.
    // 380 -> 388: chapter 92, the possessive. Six lessons introduce one atom
    // each and two retrieve, and the chapter closes FOUR inventory points
    // because they are one system rather than four facts -- the possessive
    // grid, the reflexive apna, the paas construction and the oblique a
    // postposition demands are all the same bend seen from four sides.
    // 388 -> 394: chapter 93, the imperative. Four lessons introduce one atom
    // each and two retrieve, closing three points that are the same choice
    // made in three places -- the command, the copula and the possessive all
    // picking the same level of you.
    // 394 -> 401: chapter 94, the pronouns. Five lessons introduce one atom
    // each and two retrieve, closing three points. This is the FOURTH system
    // in which the aap/tum level decision appears -- after the possessive,
    // the copula and the command -- and the review lesson sets all four side
    // by side rather than teaching a fourth list.
    // 401 -> 410: chapter 95, the past, plus one letter lesson. Six lessons
    // introduce one atom each and two retrieve. The ninth is HI-S126-letter-tha:
    // the letter tha had never been taught anywhere in the track, so every
    // past-tense form was a glyph the reader had not been given, which is the
    // mechanical reason the corpus had no past tense at all.
    // 410 -> 421: chapter 96, the future, plus three letter lessons. Six lessons
    // introduce one atom each and two retrieve. The three extra are
    // HI-S127/128/129: the independent vowels u, uu and o had never been taught
    // anywhere in the track, and uu is what the whole first-person future waits
    // on -- aaungaa puts a long u where the stem ends in a vowel, so nothing can
    // carry the matra and it must be written as a full letter.
    // 421 -> 428: chapter 97, the present continuous. Five lessons introduce one
    // atom each and two retrieve. No new glyphs were needed -- the pattern was
    // blocked by nothing but the absence of an atom presenting it, which is the
    // opposite of the last two chapters.
    // 428 -> 436: chapter 98, the daily routine. Six lessons introduce one atom
    // each and two retrieve. No new glyphs again; what it needed was se and
    // uthna, because three of the five things F-59's note called missing had
    // been taught since.
    // 436 -> 445: chapter 99, the noun. Seven lessons introduce one atom each and
    // two retrieve, and the chapter uses ZERO new nouns -- baccha, kitaab,
    // bhaasha and aadmi were all already taught, so it is pure morphology on
    // known vocabulary.
    // 445 -> 453: chapter 100, degree, plus one letter lesson. Five lessons
    // introduce one atom each and two retrieve; the eighth is HI-S130-letter-chha,
    // which chapter 99 had to route around because achchhaa contains chha and
    // could not be used in its agreement examples.
    // 453 -> 462: chapter 101, the compound postpositions. Seven lessons introduce
    // one atom each and two retrieve. No new grammar and no new glyph: every one of
    // the six relation words is a NOUN sitting in an ordinary possessive phrase, so
    // ke is the oblique kaa that chapters 92 and 99 already taught, and the seventh
    // atom -- the oblique infinitive karne -- is the same -aa to -e bend on the
    // fifth and last kind of word that wears it.
    // 462 -> 469: chapter 102, the span and the correction. Five lessons introduce
    // one atom each and two retrieve. ONE new noun (tren): bas se was taught in
    // chapter 98 and the weekday nouns in chapter 10, so both points were blocked
    // only on tak and on permission to put se and tak together.
    // 469 -> 478: chapter 103, plus THREE letter lessons. The corrected glyph
    // check -- taught = subject of a lesson, not merely present in one's body --
    // found that chaahiye needs e and jaannaa needs ja, and that NEITHER had ever
    // been drawn, though both were credited by measureScriptClosure. e goes to
    // chapter 29, ja to chapter 30, jha to chapter 31, and all three had sourced
    // stroke data sitting bundled and unused.
    // 478 -> 480: the au PAIR, script only. The au-maatraa at chapter 33 clears
    // the three closure violations it was causing, and the independent au at
    // chapter 34 pays a debt HI-C68-aur had WRITTEN DOWN IN ITS OWN BODY -- that
    // lesson's section was titled "one is still owed to you" and promised the
    // letter would come. It has, so the section is rewritten. No inventory point
    // closes; this is script debt.
    // 480 -> 482: gha and retroflex dha, THE LAST TWO GLYPHS THAT CAN STILL BE
    // TAUGHT BEFORE THEIR FIRST WORD (HL-C384). Every other undrawn character is
    // first used in chapters 1-5, before the HI-S* letter track begins at
    // chapter 6. Script only; no inventory point closes.
    // 482 -> 484: ga and pha, the first two of the LATE letters -- drawn twenty-two
    // chapters after the reader met them. neverTaughtGlyphs and
    // scriptClosureViolations DO NOT MOVE, because measureScriptClosure already
    // credited both (HL-C383). By the honest count undrawn goes 7 -> 5.
    lessons: 484, // 359 -> 360: HI-C79-dopahar-raat-ka-khana // 360 -> 362: HI-C80-aasaan-mushkil, HI-C80-sundar-badsurat // 362 -> 366: HI-C81 school pair plus its two retrieval lessons
    idioms: 21,
    // +2: HI-C70-song declares gana's singing sense and HI-C73-drink declares
    // khana's eating sense, which is what covers HI-A1-V-26.
    // +1: HI-C90-bas declares bas's VEHICLE sense. The corpus taught bas as a
    // reply meaning "enough, that's all" (Persian) and both mocks use the
    // English bus, shortened from omnibus -- two different words spelled alike,
    // told apart by whether a postposition follows.
    senses: 25,
    cultureClaims: 27,
    unitPrefix: "HI",
  }));

it("teaches independent ऋ before ऋतु becomes load-bearing", () => {
  const ordered = loadTrackLessons("hindi").sort(readingOrder);
  const scriptLessonIndex = ordered.findIndex(
    (lesson) => lesson.realization.lessonId === "HI-S125-letter-vocalic-r",
  );
  const seasonLessonIndex = ordered.findIndex(
    (lesson) => lesson.realization.lessonId === "HI-C14-ritu",
  );

  expect(scriptLessonIndex).toBeGreaterThanOrEqual(0);
  expect(seasonLessonIndex).toBeGreaterThan(scriptLessonIndex);
  expect(ordered[scriptLessonIndex]?.body).toContain("ऋ");
  expect(ordered[seasonLessonIndex]?.realization.headword).toContain("ऋतु");
  expect(
    measureScriptClosure(ordered).violations.filter(
      (violation) => violation.lessonId === "HI-C14-ritu",
    ),
  ).toEqual([]);
});

it("pins Hindi's first cumulative pre-A1 writing-stage runway", () => {
  const hindi = languageWritingStages("hindi");
  expect(hindi.defects).toEqual([]);
  expect(hindi.levels[0]).toMatchObject({ level: "pre-A1", complete: true, missingStages: [] });
});

it("removes support gently from a visible glyph trace to one heard known word", () => {
  const ordered = loadTrackLessons("hindi").sort(readingOrder);
  const stageLessons = ordered.filter((lesson) =>
    [
      "HI-W01-shirorekha-na-ma",
      "HI-W01-na-ma",
      "HI-W05-namaste-delayed-copy",
      "HI-W05-namaste-dictation",
    ].includes(lesson.realization.lessonId),
  );

  expect(stageLessons.map((lesson) => lesson.realization.lessonId)).toEqual([
    "HI-W01-shirorekha-na-ma",
    "HI-W01-na-ma",
    "HI-W05-namaste-delayed-copy",
    "HI-W05-namaste-dictation",
  ]);
  expect(stageLessons.map((lesson) => Number(lesson.frontmatter["duration.max_seconds"]))).toEqual([
    257, 186, 120, 120,
  ]);
  expect(stageLessons[2]?.frontmatter.prerequisites).toContain("HI-W05-write-namaste");
  expect(stageLessons[3]?.frontmatter.prerequisites).toContain("HI-W05-namaste-delayed-copy");
});

it("removes support gently from a known phrase to a no-model two-sentence purpose", () => {
  const ordered = loadTrackLessons("hindi").sort(readingOrder);
  const ids = [
    "HI-W06-name-sentence-frame",
    "HI-W06-name-sentence-stop",
    "HI-W06-name-sentence-delayed",
    "HI-W06-name-sentence-dictation",
    "HI-W06-two-sentence-card",
    "HI-W06-two-sentence-no-model",
  ];
  const lessons = ordered.filter((lesson) => ids.includes(lesson.realization.lessonId));

  expect(lessons.map((lesson) => lesson.realization.lessonId)).toEqual(ids);
  expect(lessons.map((lesson) => Number(lesson.frontmatter["duration.max_seconds"]))).toEqual([
    150, 120, 150, 150, 180, 180,
  ]);
  for (let index = 1; index < lessons.length; index += 1) {
    expect(lessons[index]?.frontmatter.prerequisites).toContain(ids[index - 1]);
  }

  const markdown = lessons.map((lesson) =>
    lesson.blocks.map((block) => block.markdown).join("\n"),
  );
  expect(markdown[0]).toContain("four visible word groups");
  expect(markdown[1]).toContain("changes only the sentence boundary");
  expect(markdown[2]).toContain("no visible answer and no romanization");
  expect(markdown[3]).toContain("from sound alone");
  expect(markdown[4]).toContain("new classmate");
  expect(markdown[5]).toContain("There is no Devanagari model and no romanized answer");
  expect(markdown[5]).toContain("two meanings in the requested order");
  expect(markdown[5]).toContain("one **।** after each sentence");
});

// ---------------------------------------------------------------------------
// THE HINDI A1 INVENTORY HAD NO COVERAGE ASSERTION, which is the failure mode
// HL-C350's repairs were told to avoid: land the atoms, wire the probes, and
// let a number nothing reads stay whatever it was. A stale pin that agrees
// merges silently. Both tests below were falsified before being kept -- a
// fabricated atom id fails the first, and nulling HI-A1-NUM-04's probe fails
// the second.
// ---------------------------------------------------------------------------
it("probes only Hindi atoms that EXIST, so a guessed id cannot under-report", () => {
  const { lessons } = loadEverything();
  const taught = trackIntroducedAtoms(lessons, "hindi");
  const unknown: string[] = [];
  for (const point of loadExamInventory("hindi", "A1").points) {
    for (const atom of point.probe ?? []) if (!taught.has(atom)) unknown.push(`${point.id}:${atom}`);
  }
  expect(unknown).toEqual([]);
}, 60_000);

it("pins Hindi A1 coverage, and the numeral column the ordinal tranche moved", () => {
  const { lessons } = loadEverything();
  const coverage = measureExamCoverage(loadExamInventory("hindi", "A1"), lessons);
  expect(coverage.enumerated).toBe(282);
  expect(coverage.covered).toBe(232); // 230 -> 232 (ch103): HI-A1-V-21 (chaahiye) and HI-A1-V-24 (jaannaa). BOTH NOTES WERE RIGHT, and V-21's was right about the thing a mechanical check denied -- it said chaahiye needs the independent vowel e which no writing lesson has drawn, and the glyph check said ok because measureScriptClosure credits a glyph to any script lesson whose BODY contains it, and HI-S117-letter-ca lists ek do tiin chaar paanch. V-24's defect is SHARPER than its note: jaantaa is in HI-C07-nahin's HEADWORD, glossed as the masculine present of to know, while all three of that lesson's atoms are about nahiin itself -- the headword promised a word the atoms never delivered, so a learner could deny knowing something without claiming it. chaahnaa vs chaahiye is taught as a question about what FOLLOWS (a verb vs a noun) rather than what it means, since English says want for both; chaahiye never bends and its wanter is dative, making it the liking sentence with one word swapped. // 228 -> 230 (ch102): HI-A1-POST-09 (se ... tak for a span) and HI-A1-NEG-03 (X se nahiin, Y se). BOTH NOTES WERE EXACTLY RIGHT and needed no correction -- the first two accurate notes in six chapters. Both were blocked only on se, taught in chapter 98, so the chapter costs ONE new noun. se now does FIVE jobs -- from a place, by a means, measured against, the start of a span, one option in a correction -- and NEVER BENDS in any of them, unlike kaa which had to become ke the moment another word followed, because se is a plain postposition with nothing to agree with. The load-bearing contrast is that English keeps ONE by across both halves of a correction and Hindi gives each option its own se, so counting the se tells a reader whether a swap is being offered. // 225 -> 228 (ch101): HI-A1-POST-07 (ke saath, ke binaa), HI-A1-POST-08 (ke paas, ke saamne, ke piichhe) and HI-A1-POST-10 (ke liye and the oblique infinitive). Three points, six new words, NO new machinery -- Hindi has no prepositions, only possessive phrases whose head noun names a relation, so dost ke saath is the friend's COMPANY and ghar ke paas is the house's NEARNESS, which explains the middle ke for free. POST-10's note was HALF STALE: it said neither the postposition nor the oblique infinitive is taught, and HI-C84-mere-liye had taught mere liye in chapter 92 -- what was actually missing was ke liye after a NOUN, which needs the separate ke a pronoun carries inside itself. paas arrives with an old debt paid: HI-C84-mere-paas taught it in chapter 92 as the phrase for HAVING without ever saying what the word means, and the physical sense is the one that sense was borrowed from. // 221 -> 225 (ch100): HI-A1-ADJ-08 (thoda), HI-A1-ADV-06 (well and badly), HI-A1-ADJ-09 (comparison) and HI-A1-PRON-10 (the exclamative). TWO of the four notes had gone stale because earlier chapters unblocked them without anything re-reading them: ADJ-09 said Hindi compares with se WHICH IS UNTAUGHT, and se was taught in chapter 98; ADV-06 said no word for badly is introduced, and buraa was introduced by HI-C83-buraa in chapter 91. All five new words end in -aa and bend, so the chapter adds five words and no new machinery. // 217 -> 221 (ch99): HI-A1-N-07 (plural morphology), HI-A1-N-08 (the oblique noun), HI-A1-POST-06 (ko) and HI-A1-DET-03 (the adnominal demonstrative). Four points on ZERO new nouns. N-08 is not a new system: the -aa to -e bend in front of a postposition has been taught four times already on other kinds of word, and a noun is the fourth and last kind to wear it -- which makes -e mean two different things, plural and singular-oblique, with the postposition behind the noun as the only signal. DET-03's note was PARTLY STALE: the corpus DOES print yah kitaab (HI-C69-verb-last) and vah kitaab (HI-C78-pehla-paath), but neither is an atom, so the precise complaint is that the shape was SHOWN and never TAUGHT. // 214 -> 217 (ch98): HI-A1-F-59 (describe a daily routine), HI-A1-POST-04 (se for origin and means) and HI-A1-T-09 (pahle and ke baad). F-59's note was PARTLY STALE in the way that is now the commonest finding in this campaign: it named jana, uthna, baje, se and par as missing, and three of those five had since been taught -- jaana by HI-C69-go, par by chapter 95, baje by chapter 96 -- so the gap was uthna and se plus the sequencing words the note did not mention. POST-04's and T-09's notes were exactly right and needed no correction. // 213 -> 214 (ch97): HI-A1-V-07, the present continuous as a PRODUCTIVE pattern. Its note was exactly right and needed no correction -- the pattern existed only as two fixed frames, ek baj rahaa hai and baarish ho rahii hai, with no atom presenting it as something applying to any stem. Both frames are COLLECTED rather than left as lumps: the review lesson takes them apart and shows the only difference between them is the gender of the subject. // 211 -> 213 (ch96): HI-A1-V-11 (the future as a productive paradigm with agreement) and HI-A1-T-06 (the adverbial baje). Together they make four scored mock items readable, including mock 1 reading item 8, main panch baje aunga, which the synthesis lesson reads word for word. HALF of V-11's note was too strong and is corrected: milenge is NOT taught as one memorised farewell -- HI-C04-milenge takes it apart and names -enge as the plural future -- but one cell is not a paradigm, so the point was genuinely uncovered. T-06's note is exactly right: chapter 18 teaches bajnaa as a predicate and never the adverbial form. Blocked on SCRIPT rather than grammar until now. // 209 -> 211 (ch95): HI-A1-V-12 (the past copula tha/thi/the and the past habitual) and HI-A1-POST-05 (par for location at a point). V-12's note said NOTHING in the corpus was past-tense; the cause was a single untaught letter, tha, which HI-S126-letter-tha now teaches. The past habitual costs one substitution rather than a new conjugation -- it is the present habitual the track has taught since chapter 5 with its final word swapped. POST-05 is named as turning mock 1 reading item 14, mai ghar par hu, which the synthesis lesson reads back word for word; par had existed only inside the chapter-94 phrase ham station par milenge, carried as part of a lump. // 206 -> 209 (ch94): HI-A1-PRON-04 (the subject paradigm, ham and ve), HI-A1-PRON-06 (the oblique pronouns) and HI-A1-DET-01 (Hindi has no article). PRON-04 names the item it was blocking: mock 2 reading item 10 is ham station par milenge, unreadable from its first word. PRON-06 turns out to be a finding rather than a gap -- mujhe existed only inside the pasand frame, and the frame was never a lump but a pronoun in a case. DET-01 is called the single largest structural difference from the proxy language, and the corpus had simply never stated it. // 203 -> 206 (ch93): HI-A1-V-16 (the bol/bolo/boliye series), HI-A1-V-17 (the negative imperative with mat) and HI-A1-V-03 (the tum copula ho). Three points, one grid: the chapter builds on the aap/tum/tu levels ch92 established, so choosing a person now picks a whole row -- possessive, copula and command together. V-16 also pays off an old debt named in its own note: the -iye form had only ever appeared inside maaf kijiye as a lump, and this chapter takes it apart to show it was a respectful command all along. V-17 is tested verbatim by Mock 2 reading item 3, yahan mat baithiye, which the synthesis lesson reads. // 199 -> 203 (ch92): HI-A1-DET-06 (uska, hamara, tumhara), HI-A1-DET-07 (the reflexive apna), HI-A1-V-09 (mere paas, because Hindi has no verb to have) and HI-A1-PRON-07 (a pronoun before a postposition). Four points for one chapter because they are one system: the -aa possessive bends to -e in front of any postposition, and that single habit is what all four need. The chapter turns on a correction rather than a new word -- chapter 2 taught mere as the PLURAL of mera, and it is ALSO the oblique, identical in spelling with only what follows telling you which. DET-07 was the urgent one: its note records that both mock papers use apna in their personal accounts while nothing in the corpus introduced it. // 197 -> 199 (ch90-91): HI-A1-LEX-09 (physical characteristics of a person) and HI-A1-LEX-11 (character and personality adjectives). LEX-09 cost four words rather than three: its note said bara and chhota were taught for THINGS, and chhota could not be pressed into service for "short" because about a person it means YOUNGER -- so naataa is a real gap and not a synonym. LEX-11 had achha and nothing else, and derives from the Spanish A1-NE02-01 closed in the same campaign. Both chapters teach the SAME rule from opposite sides: chapter 90 is four adjectives that all end in -aa and all agree, against the ease-and-difficulty chapter whose four all ended in a consonant and none did. // 196 -> 197 (ch89): HI-A1-LEX-23, educational institutions. school appeared in four separate mock items across the two papers while the track could name the teacher, the student and the act of studying but not the building. Both registers are taught because the reading paper is built of signs and the listening paper of speech. // 194 -> 196 (ch88): HI-A1-LEX-06 (ease and difficulty) and HI-A1-LEX-04 (attractiveness). Four adjectives, one observation: all four are consonant-final so none agrees, which the chapter teaches off the kaalaa/safed contrast from ch11 rather than as a new rule. // 193 -> 194 (ch87): HI-A1-LEX-18, the meals of the day. nashta was already taught; HI-C79-dopahar-raat-ka-khana names lunch and dinner as the transparent compounds they are, so the point closes without the track acquiring a single new word.
  expect(coverage.unmapped).toBe(50); // 52 -> 50 (ch103) // 54 -> 52 (ch102) // 57 -> 54 (ch101) // 61 -> 57 (ch100) // 65 -> 61 (ch99) // 68 -> 65 (ch98) // 69 -> 68 (ch97) // 71 -> 69 (ch96) // 73 -> 71 (ch95) // 76 -> 73 (ch94) // 79 -> 76 (ch93) // 83 -> 79 (ch92) // 85 -> 83 (ch90-91) // 86 -> 85 (ch89) // 88 -> 86 (ch88) // 89 -> 88 (ch87) // 193 -> 194 (ch87): HI-A1-LEX-18, the meals of the day. nashta was already taught; HI-C79-dopahar-raat-ka-khana names lunch and dinner as the transparent compounds they are, so the point closes without the track acquiring a single new word.
  expect(coverage.partial).toBe(0);
  // HL-C350 measured ordinals as the weakest single column in the corpus --
  // twenty tracks enumerate an ordinal point and eighteen left it uncovered.
  // HI-A1-NUM-04 is the one that moved here, and it is a COMPOUND point: the
  // ordinals to tenth AND the distributive ek … dusra. Both halves are taught,
  // and the second needed no new word, because dusra was taught with both its
  // senses. The three that remain in this category are a different problem and
  // stay named: NUM-03 (numerals above twenty), NUM-05 (the Devanagari digit
  // shapes) and NUM-06 (measures).
  expect(coverage.byCategory["Sankhya-vachak (quantifiers and numerals)"]!).toEqual({
    enumerated: 7,
    covered: 4,
  });
  expect(formatExamCoverage(coverage)).toContain(
    "hindi A1 (partial inventory): 232/282 points covered (82%)",
  );
}, 60_000);
