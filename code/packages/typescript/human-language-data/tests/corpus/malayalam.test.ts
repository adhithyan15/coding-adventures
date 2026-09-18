import { expect, it } from "vitest";
import { measureContinuity } from "../../src/continuity.js";
import {
  defaultCurriculumRoot,
  loadChapterPolicy,
  loadEverything,
  loadExamInventory,
  loadTrackLessons,
} from "../../src/loader.js";
import {
  formatExamCoverage,
  measureExamCoverage,
  trackIntroducedAtoms,
} from "../../src/exam-inventory.js";
import { measureRamp, readingOrder } from "../../src/ramp.js";
import {
  expectLanguageContinuity,
  expectLanguageModality,
  languageWritingStages,
} from "./assert-language-corpus.js";
it("pins Malayalam continuity", () => expectLanguageContinuity("malayalam"));
it("pins Malayalam modality", () => expectLanguageModality("malayalam"));
it("keeps Malayalam's opening free of genuine future farewells and pronouns", () => {
  const references = measureContinuity(
    loadTrackLessons("malayalam", defaultCurriculumRoot()),
  ).forwardReferences;
  expect(references.length).toBeLessThanOrEqual(12);
  expect(references.filter((reference) => /-C0[12]-/.test(reference.lessonId))).toEqual([]);
});
it("keeps the santosham payoff inside the three-glyph lesson budget", () => {
  const root = defaultCurriculumRoot();
  const report = measureRamp(loadTrackLessons("malayalam", root), loadChapterPolicy(root)).script;
  expect(report.lessons.find((lesson) => lesson.lessonId === "ML-C02-santosham")).toBeUndefined();
});

it("keeps Malayalam Chapter 7 meaning-first and below the three-glyph step budget", () => {
  const root = defaultCurriculumRoot();
  const lessons = loadTrackLessons("malayalam", root).sort(readingOrder);
  const chapter = lessons.filter((lesson) => /^ML-[CW]07-/.test(lesson.realization.lessonId));
  expect(chapter.map((lesson) => lesson.realization.lessonId)).toEqual([
    "ML-C07-numbers-1-5",
    "ML-W07-digits-1-3",
    "ML-W07-digits-4-5",
    "ML-W07-number-words-1-5",
    "ML-W07-numbers-1-5-guided-copy",
    "ML-W07-numbers-1-5-delayed-copy",
    "ML-W07-numbers-1-5-dictation",
    "ML-C07-numbers-6-10",
    "ML-W07-digits-6-8",
    "ML-W07-digits-9-10",
    "ML-W07-number-words-6-10",
    "ML-W07-numbers-6-10-guided-copy",
    "ML-W07-numbers-6-10-delayed-copy",
    "ML-W07-numbers-6-10-dictation",
    "ML-C07-numbers-practice",
  ]);

  const spoken = chapter.filter((lesson) => lesson.realization.lessonId.startsWith("ML-C07-numbers-") && lesson.realization.lessonId !== "ML-C07-numbers-practice");
  expect(spoken).toHaveLength(2);
  expect(spoken.every((lesson) => !lesson.body.match(/\p{Script=Malayalam}/u))).toBe(true);
  expect(spoken.every((lesson) => lesson.frontmatter.skills?.join(",") === "listening,speaking")).toBe(true);

  const script = measureRamp(lessons, loadChapterPolicy(root)).script;
  expect(script.lessons.filter((lesson) => lesson.chapter === 7)).toEqual([]);
  expect(new Set(chapter.flatMap((lesson) =>
    [...lesson.body.matchAll(/hl-writing-stage:\s*([a-z-]+)/g)].map((match) => match[1]),
  ))).toEqual(new Set([
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "dictation-transcription",
  ]));
});

it("keeps Malayalam's first greeting meaning-first and its script runway learner-visible", () => {
  const opening = loadTrackLessons("malayalam").sort(readingOrder).slice(0, 9);
  expect(opening.map((lesson) => lesson.realization.lessonId)).toEqual([
    "ML-C01-namaskaram",
    "ML-W01-na-ma-trace",
    "ML-W01-na-ma-guided-copy",
    "ML-W01-na-ma-delayed-copy",
    "ML-W01-na-ma-dictation",
    "ML-W01-sa-chandrakkala-ka",
    "ML-W01-aa-ra-anusvaram",
    "ML-W01-namaskaram-read",
    "ML-W01-namaskaram-dictation",
  ]);
  expect(opening.every((lesson) => lesson.frontmatter.chapter === "1")).toBe(true);
  expect(opening[0]?.frontmatter.skills).toEqual(["listening", "speaking"]);
  expect(opening[0]?.body).not.toMatch(/\p{Script=Malayalam}/u);
});

it("gives Malayalam a complete pre-A1 writing runway", () => {
  const malayalam = languageWritingStages("malayalam");
  expect(malayalam.defects).toEqual([]);
  expect(malayalam.levels[0]).toMatchObject({
    level: "pre-A1",
    complete: true,
    missingStages: [],
  });
  expect(new Set(malayalam.validEvidence.map((entry) => entry.stage))).toEqual(new Set([
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "dictation-transcription",
  ]));
});

// ---------------------------------------------------------------------------
// THE MALAYALAM A1 INVENTORY HAD NO ASSERTION IN THIS FILE -- the hole HL-C354
// found in Telugu and Hindi and told the next reader to look for in the other
// eighteen. Without these two tests the ordinal tranche could land its atoms,
// wire ML-A1-NUM-05's probe, and leave a coverage number that nothing in the
// track's own test file reads. Both halves were falsified before being kept: a
// fabricated atom id in the probe fails the first, and nulling ML-A1-NUM-05's
// probe fails the second.
// ---------------------------------------------------------------------------
it("probes only Malayalam atoms that EXIST, so a guessed id cannot under-report", () => {
  const { lessons } = loadEverything();
  const taught = trackIntroducedAtoms(lessons, "malayalam");
  const unknown: string[] = [];
  for (const point of loadExamInventory("malayalam", "A1").points) {
    for (const atom of point.probe ?? []) if (!taught.has(atom)) unknown.push(`${point.id}:${atom}`);
  }
  expect(unknown).toEqual([]);
}, 60_000);

it("pins Malayalam A1 coverage, and the ordinal point the tranche closed", () => {
  const { lessons } = loadEverything();
  const coverage = measureExamCoverage(loadExamInventory("malayalam", "A1"), lessons);
  expect(coverage.enumerated).toBe(243);
  // 163 -> 164: ML-A1-JOIN-02 (or). ONE POINT FOR FOUR LESSONS, AND THE RATIO
  // IS THE WRONG WAY TO READ THIS CHANGE. What chapter 70 actually does is give
  // Malayalam its 'and': the language coordinates with the clitic -um repeated
  // on EACH item, and NOTHING IN 307 LESSONS TAUGHT IT. Verified rather than
  // taken from the note -- -um appeared in exactly one lesson file, inside an
  // etymology note as a component of the word for evening, and the nine files
  // carrying the sequence u+m all had it INSIDE a word (veendum, kudumbam,
  // hrudayam, the month names). A learner owning 69 chapters of vocabulary could
  // not say 'water and rice'.
  // ML-A1-JOIN-01 STAYS OPEN ON PURPOSE. Its label is 'joining two nouns, AND
  // joining two clauses', and this chapter delivers the noun half only. The
  // everyday clause link in this corpus is the -i participle already covered by
  // ML-A1-JOIN-11, and claiming clause coordination off the noun lessons would
  // claim a range the corpus does not teach. Same call as TE-A1-L-10.
  // ML-A1-JOIN-04 (distributive) also stays open, but its blocker is gone: its
  // note said it depended on the missing coordinator, which now exists.
  // -o IS TAUGHT BESIDE -um BECAUSE THEY ARE ONE HABIT: both go on every item
  // with nothing in the gap, and only the vowel differs -- veLLavum ariyum for
  // both, chaayayoo kaappiyoo for one of them.
  // MALAYALAM HAS NO UNTRANSFERABLE POINTS, unlike Kannada's four, so its
  // ceiling is the full 243.
  // 164 -> 166: ML-A1-PRON-03 (third person) and ML-A1-PRON-04 (first and second
  // person plural). VERIFIED WORSE THAN THE NOTES SAID: avan, aval, avar and
  // njangal appeared in ZERO lesson files -- not as headwords, not anywhere in a
  // body. A learner with seventy chapters of vocabulary could say I and you and
  // could not say he, she, they or we.
  // naam LOOKED taught and was not: it appeared as a headword in ML-C67-first,
  // ML-C67-third and ML-C68-eleventh, where it is a SUBSTRING of the ordinals
  // onnaam, moonnaam and pathinonnaam. Every ordinal ending -nnaam is a false
  // positive for naam -- the same trap the Hindi campaign recorded when every
  // ordinal matched its own cardinal. Check the token, not the substring.
  // THE GAP SAT INSIDE A SYSTEM ALREADY TAUGHT. ML-C41-that teaches the i-/a-
  // pointing pair and says in as many words that a- means far, and the corpus
  // only ever used it on THINGS. avan, aval and avar carry the same a-, so the
  // column for people was predicted and never filled -- the same shape as Tamil's
  // TA-A1-PRON-03, closed in the chapter before this one. avar additionally
  // reuses ML-C02's own rule that a plural raises the register (nii -> ningal,
  // avan -> avar), so two of the three new words run on machinery already held.
  // 166 -> 167: ML-A1-JOIN-06, the quotative ennu. THE NOTE WAS VERIFIED BY
  // TOKEN RATHER THAN SUBSTRING, and that mattered: the string `enna` appears
  // nine times in the corpus, and every one is inside ennaal, the word for
  // "but" that ML-C64 teaches. The quotative -- with the virama -- was nowhere.
  // ONE MARKER BUYS BACK THE WHOLE CORPUS: ennu leaves the quoted sentence
  // untouched, so every sentence the reader can build becomes something they
  // can report, think, claim to know or ask. Two atoms, four verb frames, all
  // four verbs already taught.
  // THE SAYING-VERB WAS A SECOND FINDING AND IS FIXED IN CHAPTER 50, NOT 72:
  // parayuka was never a headword anywhere, while ML-C50-farewell built
  // `vita parayuka` and called it "the speaking-verb" for want of a name.
  // Teaching it at 72 made those uses forward references 104 and 106 lessons
  // early; the lesson moved to sequence 1405, immediately before the first use,
  // which returned forwardReferences to its baseline of 12 and paid a debt that
  // predated this work.
  // 167 -> 169: ML-A1-JOIN-05 (because) and ML-A1-Q-07 (why), which are ONE
  // PIECE OF WORK and whose notes said so -- cause could be handled in NEITHER
  // direction, so closing one without the other leaves a learner able to ask a
  // question nobody can answer. Both notes were verified: entukondu, kaaranam
  // and entukondennaal each returned ZERO files across the whole corpus.
  // MOST OF THE CHAPTER IS BUILT FROM WHAT THE READER HAD. Malayalam has no
  // separate word for why: entukondu is chapter two's entu plus kondu, "by
  // means of", so the question asks "by what". The written because then carries
  // that whole question word visibly at its front.
  // THE TAIL OF entukondennaal IS DELIBERATELY NOT TAKEN APART: its -ennaal can
  // be read as the but-word ML-C64 teaches or as a conditional of the saying
  // verb, grammars differ, and the lesson says so rather than picking one.
  // A DRAFT CLAIMED ALL FOUR QUESTION WORDS SHARE THE FRONT LETTER e AND WAS
  // WRONG: aaru (who) opens on aa. Three of the four carry the asking letter
  // ML-C41-deixis-system named, and who is the exception.
  // 169 -> 171: ML-A1-JOIN-09 (purpose) and ML-A1-V-22 (ability). JOIN-09's note
  // PREDICTED THE PAIR -- "the -aan purpose form is not taught, which also
  // blocks the ability frame; both are built on it" -- and it was right. Both
  // forms returned ZERO files before the chapter was written.
  // ONE SWAP AND WHAT IT BUYS: -uka off, -aan on, stem untouched. The word that
  // FOLLOWS the -aan form decides the sentence -- another verb gives purpose,
  // kazhiyum gives ability.
  // ABILITY ARRIVES AT YOU RATHER THAN BEING DONE BY YOU, and the track had
  // already taught that shape: ML-C32-ariyuka's warm-up says knowing ARRIVED at
  // you, so the I left the subject slot. enikku vaayikkaan kazhiyum is literally
  // "reading is possible to me", with the person in the dative exactly as the
  // knowing sentence has it. ONE genuinely new word in the whole chapter.
  // 171 -> 174: ML-A1-Q-06 (eppol), ML-A1-JOIN-07 (-umbol) and ML-A1-JOIN-08
  // (-aal). ONE CHAPTER FOR THREE POINTS BECAUSE THE THREE ARE ONE PIECE: pol.
  // eppol, varumpol and the now/then pair the track already taught all carry it.
  // Verified rather than assumed -- eppol, mbol and every spelling of them
  // returned ZERO files across 333 lessons, and every existing -aal in the
  // corpus was lexical (paal, kaal) or one of the two frozen words ennaal and
  // enthukondennaal.
  // THE CHAPTER PAYS OFF A PROMISE THE TRACK ALREADY MADE. ML-C41-deixis-system
  // says in so many words that meeting a new word in this family means being
  // taught one and working out the others; ML-C50-now handed over ippol and
  // appol and stopped. The question form is the missing third.
  // THE CONDITIONAL IS BUILT ON THE PAST FORM AND IS NOT ABOUT THE PAST, which
  // is the one genuinely counter-intuitive thing in the chapter and is taught
  // as such. -umbol and -aal are taught as a minimal pair because one ending is
  // the entire difference between "when he comes" and "if he comes".
  // CHAPTER 73'S REFUSAL IS LEFT STANDING. Knowing -aal does not settle ennaal
  // or enthukondennaal, grammars still differ, and ML-C75-aal says so on the
  // page rather than quietly claiming the win.
  // 174 -> 175: ML-A1-V-23 (wanting). The point's own note called it "the
  // single cheapest fix in this file" and was right -- venda was already
  // taught and venam appeared EXACTLY ONCE, inside ML-C58-no-need, which names
  // it as "the negative partner of venam" and then goes on without handing it
  // over. The learner could refuse an offer they had no way of making.
  // ONE GENUINELY NEW WORD: the question form veno is venam plus the -o
  // particle ML-C70-o already taught, so the reader builds it in the warm-up.
  // The result is a three-way exchange on one root: veno? / venam / venda.
  // 175 -> 177: ML-A1-NEG-02 (alla) and ML-A1-Q-09 (the tag question), which
  // close together because alle is built on alla and visibly carries it.
  // MALAYALAM NEGATES TWICE WHERE ENGLISH NEGATES ONCE: illa denies that a
  // thing is THERE, alla denies that it is SO. illa had 27 occurrences and alla
  // had ZERO, so "I am not a teacher" was unsayable. Both alla and alle returned
  // zero files before this chapter, so neither created a forward reference --
  // checked BEFORE writing, after venam needed rehoming for exactly that.
  // 177 -> 178: ML-A1-POS-02, the genitive suffix itself. enre and ninre were
  // taught WHOLE and the suffix inside them was never named, so the learner
  // owned two possessors and could not make a third. Naming it turns a closed
  // pair into an OPEN RULE: any taught noun can now own something.
  // BOTH ALLOMORPHS, one per lesson, because the choice is not the speaker's:
  // -nre after a consonant, -yude after a vowel. Verified before writing that NO
  // third genitive existed in the corpus, so this created no forward reference.
  // 178 -> 180: ML-A1-ADJ-05 and ML-A1-ADV-08, which are ONE gap seen from two
  // sides -- the track taught five qualities and no way to grade any of them,
  // so "good" could not become "very good". valare closes the written register
  // and orupadu the spoken one; the pair is a REGISTER split, not a meaning
  // split. The third grade the adverb point asks for cost nothing: kuraccu was
  // already taught as a quantity word and stands in the same slot pointing
  // down. The grammar lesson names the SLOT rather than the words, so the next
  // degree word the learner meets is understood on sight. Verified before
  // writing: valare and orupadu both returned ZERO files.
  // 180 -> 182: ML-A1-N-02 and ML-A1-ADJ-04, one hole seen from two sides again.
  // naadu named a KIND of place, so "where are you from" could only be answered
  // with a category -- no settlement name and no name of Kerala or India existed
  // anywhere. uuru pins the small end of the scale naadu floats over; Keralam and
  // India are the two fixed points on it. The GENTILIC needed NO NEW GRAMMAR,
  // which is the chapter's finding rather than a shortcut: English keeps two
  // shapes (India / Indian) and Malayalam keeps one, placing somebody by standing
  // the person-word in front UNCHANGED -- the same position the quality word and
  // the degree word already occupy. Verified before writing: uuru, Keralam, India
  // and Malayali all returned ZERO files.
  // 182 -> 183: ML-A1-ADV-09, near and far. ivide and avide POINT -- my side or
  // not my side -- and something avide may be a step away or a country away, so
  // the track could point at a distance and not measure one. aduthu and akale
  // measure; ethra dooram? asks for a number. One of the four atoms cost nothing:
  // ethra was already inside the age question, doing exactly this job, and the
  // chapter lifts it out. The chapter also asks the learner to REFUSE a pattern --
  // akale opens with a and the distal deictic prefix is a-, but here it is part of
  // the word, and the test is whether the other members exist (no ikale, no ekale,
  // so no set). Verified before writing: all three new words returned ZERO files.
  // 183 -> 186: ML-A1-S-07 (the vocative), ML-A1-F-35 (addressing somebody) and
  // ML-A1-REG-05 (which greeting, when). The first two were cheaper than their
  // notes suggested because the WORDS were already there: chettan and chechi have
  // been taught since the family chapter, and in Kerala the sibling words are used
  // outward -- you address a stranger as KIN. What was new is the CALLING form: a
  // word ending in -an swaps it for -aa, so chettan becomes chettaa; chechi has no
  // -an to swap and stands as it is. REG-05 teaches NO WORD AT ALL: all five
  // greetings were already taught and the lesson states the rule the corpus's own
  // register fields already encode -- namaskaram is respectful-neutral and never
  // wrong, the four shubha greetings are formal and mostly read, suprabhaatham is
  // the everyday exception inside its own family.
  // 186 -> 187: ML-A1-CASE-06, the ablative, whose blocker chapter 80 removed --
  // its note said "combined with the missing place names at ML-A1-N-02, where are
  // you from cannot be answered at all". The chapter adds NO VOCABULARY. ninnu
  // follows a word already carrying the locative -il, so "from" is a second step
  // ON TOP of "in"; and unlike every ending so far it is a WORD standing apart,
  // not a suffix, which the lesson explains from its origin as a form of a verb
  // meaning "to stand". The question costs nothing: evide ninnu is two owned words
  // in the order just taught. The third lesson has reach well past this point -- a
  // noun ending in -am REPLACES it with -att- before any ending (Keralam ->
  // Keralattil, pustakam -> pustakattil), which is a CLASS, not an exception, and
  // puts every -am noun in the book into the locative from one rule.
  // 187 -> 188: ML-A1-CASE-07, the spatial postpositions. Its note was exact --
  // "the track teaches five house objects and no way to say where any of them is."
  // The chapter spends its FIRST lesson on the shape rather than the vocabulary,
  // because the shape is what makes each later place-word cost a word and nothing
  // else: THE NOUN IN FRONT TAKES THE GENITIVE, which is chapter 78's ending
  // unchanged. The reason is worth having -- kaseerayude mukalil is literally "the
  // chair's top-part", so Malayalam names a part BELONGING to the chair rather
  // than saying "on" it, which is why the owner-ending is what connects the two
  // words at all. The recall states a pattern covering THREE of the four: mukalil,
  // munnil and pinnil visibly end in the locative -il because they are part-words
  // already in the in-form; thaazhe does not, and that is said plainly rather than
  // tidied away. meesha (table) was deliberately NOT taught: it already appears
  // untaught in ML-C52-chair's prose, so claiming it as a headword would convert
  // that use into a forward reference -- the ML-C82-chechi-address trap.
  // 188 -> 190: ML-A1-PHON-04 (gemination) and ML-A1-PHON-05 (stress), paired
  // deliberately. A measurement first said this was NOT the free tranche it looked
  // like: 102 of 292 taught tokens carry a doubled consonant, but gemination
  // minimal pairs AMONG TAUGHT WORDS numbered ZERO, so the contrast could not be
  // shown from taught vocabulary alone. kutti (child) was already taught and kudi
  // returned zero files, so the chapter teaches kudikkuka (to drink) FIRST -- which
  // fills a real hole, the book having given water, tea, coffee and milk and no
  // verb for them -- and then sets its stem kudi against kutti. PHON-05 pairs with
  // it because the point is not where the stress falls but how LITTLE it does: no
  // two Malayalam words differ only by it, so English's habit of spending stress on
  // meaning is the wrong ear for a language that spends LENGTH. PHON-03 stays open
  // on purpose: no vowel-length minimal pair exists among taught words either, and
  // it must not be written on a constructed pair.
  // 190 -> 191: ML-A1-CASE-05, the instrumental and the sociative, whose note
  // named both halves -- neither -aal nor oppam was taught, so "with a friend" and
  // "by bus" were both out of reach although suhruthu was taught. The chapter's
  // point is that THEY ARE NOT THE SAME RELATIONSHIP: English spends one word on a
  // companion and a tool, and Malayalam marks them differently, so the question is
  // never "how do I say with" but WHICH KIND OF WITH THIS IS. Company (oppam,
  // koode) stands after the owner-form in the slot chapter 84 established, and
  // splits by register the way the degree words did. Instrument (-aal) glues onto
  // the noun -- and it is an ending the learner ALREADY OWNS, since -aal on a VERB
  // is chapter 75's conditional. Identical ending, unrelated meanings, so the
  // lesson teaches the habit rather than the form: look at the HOST first.
  // 191 -> 192: ML-A1-CASE-04, the accusative, which closes the CASE column at
  // 7/7. Unlike every case ending before it, -e does not PLACE the noun anywhere;
  // it says what ROLE the noun has, and the doer wears nothing. English does that
  // job with word order alone, so a marked object is doing work English spends its
  // word order on. The part worth the chapter is the ANIMACY rule: it is not about
  // being the object but about WHAT KIND OF THING the object is -- kuttiye takes
  // the ending, pustakam does not, in the same slot before the same kind of verb.
  // And there is no clitic: English keeps a second form (he/him) and Spanish
  // shrinks the object onto the verb, while Malayalam keeps the ordinary pronoun
  // and adds the ordinary ending, so avane costs nothing. No new vocabulary at
  // all, and the joining rule is the genitive's.
  // 192 -> 193: ML-A1-NUM-04, counting past twenty. The chapter extends the
  // pattern chapter 20 made visible -- irupathu is iru plus pathu, and every ten
  // from thirty up is that word with a different digit in front. The digits appear
  // in OLDER SHORT FORMS that survive only in compounds (aaru as aru-, eezhu as
  // ezhu-), so the lesson says this is a pattern to READ WITH, NOT BUILD WITH.
  // Ninety breaks the run, and the chapter teaches a hundred FIRST so it is not
  // noise: thonnooru carries nooru, not pathu, and is named from ABOVE. EIGHTY IS
  // TAUGHT BY EAR ONLY: enpathu needs the chillu NN, which ML-S131 teaches but
  // data/scripts/malayalam.json omits, so writing it trips uncovered-glyphs. Giving
  // a number by ear before its written form is this track's own precedent -- see
  // ML-C07-numbers-6-10. Recorded as backlog item HL-C398.
  // 193 -> 194: ML-A1-F-28, giving an opinion, AND THE POINT'S NOTE WAS
  // SUBSTANTIALLY WRONG, which is why it had looked like the most expensive item
  // in the file. It said "I think that ... needs ennu plus cintikkuka" as though
  // neither existed. BOTH ARE TAUGHT -- ennu in chapter 72, cintikkuka in 33 --
  // and ML-C72-ennu-more already pairs them, with the worked example "I think I
  // know Malayalam". So the assembly was on the page before this chapter existed.
  // Only the EVALUATION VOCABULARY was missing, and there the note was right:
  // nallathu and mosham both returned ZERO files. The chapter teaches TWO WORDS
  // AND NO GRAMMAR -- -athu lets a quality stand without a noun to lean on and is
  // visibly the word athu, "that"; mosham fills the hole left by five qualities of
  // which none was negative; the third lesson only assembles.
  // 194 -> 196: ML-A1-PRON-08 and ML-A1-JOIN-10, the relative participle, which
  // the file called STRUCTURALLY THE BIGGEST GAP IN IT. The gap was real -- a sweep
  // for attributive -unna forms returned ZERO while 16 distinct -unnu present forms
  // were already in use -- but the SIZE was the surprise. The describing form is
  // the taught present or past with ONE VOWEL SIGN REMOVED: pokunnu -> pokunna,
  // vannu -> vanna, leaving the final letter with its own inherent a. The rule is
  // about the LETTER, not the tense. The form then stands IN FRONT of the noun, in
  // the slot a quality word has held since the adjective chapter, so no new
  // arrangement was needed. Two things differ from English and the second matters
  // most: the clause moves to the front, and THE JOINING WORD DISAPPEARS -- there
  // is nothing to translate "that" into. No new vocabulary at all.
  // 196 -> 198: ML-A1-LEX-30 (money, price, paying) and ML-A1-LEX-31 (shops),
  // which are one situation and close together. WHAT MADE THIS CHEAP WAS CHAPTER
  // 88. The price question is ethra roopa, and ethra was lifted out of the age
  // question in chapter 81, so the question is two words of which one was owned.
  // The ANSWER is what had been impossible: before the tens, the count stopped at
  // twenty and every price above it was unsayable, exactly as ML-A1-NUM-04's note
  // warned. So the chapter only had to name the unit being counted. vaanguka's
  // object stays BARE -- pustakam takes no object ending because it is a thing,
  // which is chapter 87's rule holding while the verb changed. A vocabulary
  // chapter is cheap when the grammar under it is already built.
  // 198 -> 199: ML-A1-LEX-55, eating out, whose blocker chapter 91 removed. Three
  // words close it and the question ending the scene is chapter 91's, unchanged.
  // bhakshanam is the general word for FOOD, which the book had never had -- it
  // owned ari, choru and oon and no name for the category, which is what a
  // vocabulary grown from the particular outward tends to leave out. THE TWO
  // BORROWINGS ARE THE REAL LESSON and sit together on purpose: hottal came from
  // English "hotel" and in Kerala ordinarily means A PLACE TO EAT, while bil kept
  // its meaning intact. One changed and one did not, so the habit taught is to
  // check each borrowing rather than trust the family.
  // 199 -> 200: ML-A1-N-06, the plural, and THE NOTE'S EVIDENCE WAS WRONG -- it was
  // checked before it was spent. The note called this "the cheapest grammar lesson
  // available", on the claim that -kal already sat inside four taught headwords:
  // nirangal, maasangal, shareera-bhaagangal, kaalangal. Those are FILE SLUGS. A
  // token sweep of every Malayalam lesson file found the chillu-LL sequence -kal in exactly
  // two words, thinkal (Monday) and makal (daughter), and NEITHER IS A PLURAL. The
  // learner had met the suffix zero times, not four.
  // What the same sweep DID find is better: NINGAL, the respectful you from chapter
  // 2, ends in -ngngal, and ML-CONCEPT-C02-RESPECT-BY-PLURAL-01 had already told the
  // learner that word IS a plural -- without ever isolating the ending that made it
  // one. So the chapter reveals rather than introduces.
  // The label's second half cost nothing at all: a noun with a numeral in front
  // needs no ending, and the corpus was ALREADY doing it right in three attested
  // lines -- ampathu rupa, nuru rupa, rantu mani. The lesson points at the learner's
  // own sentences instead of asserting a rule. NO NEW VOCABULARY in the chapter.
  // NJANGAL was cut from the reveal mid-draft: chapter 71 sits on
  // SPINE-READ-SIGNS-AND-NOTICES, which the walk reaches AFTER
  // SPINE-DEFINITE-REFERENCE, so its atom is not available here. Chapter number is
  // not walk order.
  // 200 -> 201: ML-A1-TIME-05, today/yesterday/tomorrow. innu and innale have ZERO
  // occurrences across every Malayalam lesson file, so both are clean introductions.
  // NAALE IS NOT, AND THE DIFFERENCE DECIDED THE CHAPTER'S SHAPE. It appears in
  // FIVE lessons -- ML-C04-naale-kaanaam's phrase headword, ML-C23-naal (which
  // even glosses it as "tomorrow" in passing), ML-C32-kaanuka, ML-S01-letter-ka
  // and ML-S04-vowel-sign-aa -- and is owned by NONE of them, because ML-C04's
  // headword is the multi-word "naale kaanaam" and continuity.ts keeps a
  // multi-word headword WHOLE. Claiming naale as a headword would have converted
  // that invisible debt into five forward references reaching back to sequence
  // 240. So ML-C94-three-days is a GRAMMAR lesson about the word leaving the
  // farewell, not a vocabulary lesson about the word -- which is also the honest
  // description: the learner has had it since the first week and could not use it
  // anywhere except to end a conversation. Only TWO words are new.
  // The chapter then sets the three day words against the three verb endings
  // already taught, and says plainly that the VERB carries the tense while the day
  // word only says WHICH DAY. The tenses were built chapters ago and had nowhere
  // to land.
  // 201 -> 202: ML-A1-LEX-51, open/closed and way in/way out. vaathil was named in
  // chapter 52 and nothing could be done to it or about it until now. All six
  // candidates were grepped as tokens first and every one had ZERO occurrences, so
  // the chapter carries no forward-reference debt.
  // TWO WORDS BUY FOUR FORMS: the asking ending is the long uu already owned from
  // keLkkoo among the first commands, and the three tense endings come from a verb
  // learned long before, so thurakkunnu/thurannu/thurakkum arrive already known.
  // A DRAFT CLAIM WAS CUT HERE and it is the kind nothing in this repo reads. The
  // akatthu/purathu lesson said those two end like the place words for above and
  // below, "a family". THEY DO NOT: mukalil and munnil end in -il, thaazhe in -e,
  // akatthu and purathu in -tthu. Three endings, not one. Checked against the
  // owning lessons rather than from memory. The replacement is true and teaches
  // better -- akatthu/purathu share a TAIL and part at the front, while
  // innu/innale one chapter earlier shared an OPENING and parted at the tail.
  // validate reads frontmatter and atom comments; the gates read structure; this
  // suite reads banned words, glyphs and pins. None of them reads an assertion
  // about how words are built. Same family as HL-C399.
  // 202 -> 203: ML-A1-LEX-54, ease and difficulty. NOTE THE PERCENTAGE ROLLS OVER
  // AGAIN, 83% -> 84%; it was computed, not assumed, after a stale 82% shipped past
  // every check but this one in chapter 94.
  // TWO WORDS AND NO NEW GRAMMAR: eLuppam + aaNu -> eLuppamaaNu is the same welding
  // as sukhamaaNu (chapter 3), ishtamaaNu (34), kudumbamaaNu (35), moshamaaNu (89).
  // TWO THINGS WERE WRONG IN THE FIRST DRAFT, BOTH CAUGHT BY THE REVIEWER AND NOT BY
  // ANY GATE HERE.
  // (1) The chapter claimed the join had never been pointed out. FALSE: ML-C89-mosham
  // states it outright AND tests it in its own Wrap-up Recall, seven chapters earlier,
  // in the lesson that gives the learner moshaM -- one of the three examples cited as
  // silent precedent. Reframed: named once, at moshaM; here it stops being a fact
  // about a given word and becomes something the learner does to words of their own.
  // (2) "the anusvara turns into ma" described a SPELLING change as a SOUND change.
  // The sign already IS the sound m; aaNu puts a vowel behind it and the same m needs
  // the full letter to carry it. Nothing is heard to change -- which is the chapter's
  // own best evidence, and the false version undercut it.
  // ML-C89-mosham's formulation was narrowed in the same pass: it said the anusvara
  // becomes ma "before a word starting with a vowel", which veLLam + um -> veLLavum
  // contradicts (ML-C70-um); the plural takes it to -ngngaL instead (ML-C93-angal).
  // 203 -> 204: ML-A1-LEX-52, beautiful and not. WRITTEN FROM AN ENUMERATION RATHER
  // THAN A GUESS -- the correction taken from chapter 96, whose explanatory paragraph
  // was wrong four times across five review rounds because the tidy version got
  // written first and checked afterwards.
  // The sweep ran BEFORE drafting. Every attested quality-plus-aaNu form in the
  // corpus gives three welds: -am -> maaNu (sukhamaaNu, moshamaaNu, eLuppamaaNu),
  // -i -> yaaNu (kuttiyaaNu), -athu -> thaaNu (nallathaaNu). The chapter teaches the
  // first two and claims ONLY what the sweep supports -- a letter ARRIVES to carry
  // aaNu's vowel, and which one depends on how the word ended. It says nothing about
  // the third case, which no lesson here shows.
  // NO DEDICATED WORD FOR "UGLY", deliberately: vrtthiketta would need an attributive
  // form and a register judgement that could not be defended under review, the same
  // call made about buddhimuttu in chapter 96. The far end is built with alla, which
  // ML-C77 teaches STANDING APART and never welded -- and the lesson makes the
  // not-welding its point, since sundaram keeps its anusvara because nothing attached.
  // 204 -> 205: ML-A1-LEX-37, free time, sport, games and shows. THE POINT'S OWN
  // NOTE WAS OVERSTATED and the chapter says so: it claimed not one leisure word
  // is taught, while vaayikkuka, kaaNuka and pusthakam are all owned, and its
  // "292 lessons" had rotted to 416 files. The real gap was sport, games and shows.
  // FREE TIME IS LEFT OPEN ON PURPOSE -- ozhivusamayam needs samayam, which nothing
  // teaches, and a compound of an unowned part is a headword smuggled in.
  // THE ONE GENERALISING SENTENCE WAS AGAIN WHERE THE ERROR LIVED, three chapters
  // running. sinima is the first vowel-final word in the book to weld to aaNu
  // without ending in -i, and the draft said the ya "comes anyway". pashu, taught
  // since chapter 60, gives pashuvaaNu with va. Nothing in the corpus attests a va
  // glide, so no gate and no earlier lesson could contradict it -- latent-false, and
  // caught only by asking what a taught word would do. The lesson now teaches that
  // the last vowel CHOOSES the letter and exhibits the cow as the row that differs.
  // ML-C97-bhangi, ML-R97-beauty-recall and ML-C70-um-more all stated the broad
  // version and were narrowed to name -i as the environment in the same commit.
  expect(coverage.covered).toBe(205);
  expect(coverage.unmapped).toBe(38);
  expect(coverage.partial).toBe(0);
  // ML-A1-NUM-05 was one of the thirteen ordinal points HL-C354 left open.
  // Malayalam's -aam has no exceptions at all, so all eleven ordinals follow
  // from one ending on cardinals chapter 7 already taught, and the tranche also
  // closes the "ordering notion" the old note named as missing by teaching
  // aadyam against the pinne the track already had.
  expect(coverage.byCategory["Sankhya (numerals and quantity)"]!).toEqual({
    enumerated: 9,
    covered: 8,
  });
  expect(formatExamCoverage(coverage)).toContain(
    "malayalam A1 (partial inventory): 205/243 points covered (84%)",
  );
}, 60_000);
