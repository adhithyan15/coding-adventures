import { describe, expect, it } from "vitest";
import { loadTrackLessons, loadExamInventory } from "../../src/loader.js";
import {
  EXAM_CONTENT_DIMENSIONS,
  measureExamCoverage,
  formatExamCoverage,
  isExamInventoryComplete,
  trackIntroducedAtoms,
} from "../../src/exam-inventory.js";

describe("the committed Malayalam A1 inventory", () => {
  const inventory = loadExamInventory("malayalam", "A1");
  const spanish = loadExamInventory("spanish", "A1");

  it("re-derives Malayalam's direct script-owner gaps and both useful counts", () => {
    const lessons = loadTrackLessons("malayalam");
    const glyphs = (text: string) =>
      [...text].filter((character) => {
        const codepoint = character.codePointAt(0)!;
        return codepoint >= 0x0d00 && codepoint <= 0x0d7f;
      });
    const shown = new Set(lessons.flatMap((lesson) => glyphs(lesson.realization.headword ?? "")));
    const directlyOwned = new Set(
      lessons
        .filter(
          (lesson) =>
            lesson.realization.type === "writing" || lesson.frontmatter.delivery === "script",
        )
        .flatMap((lesson) => glyphs(lesson.realization.headword ?? "")),
    );
    const open = [...shown]
      .filter((glyph) => !directlyOwned.has(glyph))
      .map((glyph) => {
        const headwords = lessons
          .map((lesson) => lesson.realization.headword ?? "")
          .filter((headword) => headword.includes(glyph));
        const tokens = new Set(
          headwords.flatMap((headword) =>
            headword.split(/\s+/).filter((token) => token.includes(glyph)),
          ),
        );
        return { glyph, fields: headwords.length, tokens: tokens.size };
      })
      .sort((a, b) => b.fields - a.fields || a.glyph.localeCompare(b.glyph));

    // 459 -> 470. THIS PIN ARRIVED STALE: the tree it was written against had
    // 459 Malayalam lessons, but chapter 107's five had already landed by the
    // time #15566 merged, so main carries 464 against a pin of 459 and this
    // assertion is red on the base branch. Corrected here to the true count,
    // which is 464 plus chapter 108's six.
    // The three counts BELOW it do not move, and that is the useful part: the
    // six new lessons are punctuation, so their headwords contain no character
    // in U+0D00-U+0D7F at all. They enter neither `shown` nor `directlyOwned`,
    // and the nine open glyphs are exactly as they were. A chapter can be six
    // writing lessons long and still owe this census nothing.
    // 470 -> 471: ML-C18-oru, inserted at sequence 465. It teaches a WORD, not
    // a glyph, so the three counts below it stay put again -- oru's characters
    // are all long since owned.
    // 474 -> 479: chapter 110's five lessons. chillu-r goes 11/11 -> 12/12 on
    // inrarneRRu alone; `shown` stays 68 and the owned overlap stays 59, so once
    // again no NEW glyph opens.
    // 471 -> 474: chapter 109's three lessons. THE FIRST DRAFT OF THIS COMMENT
    // CLAIMED THE COUNTS BELOW WOULD HOLD, on the reasoning that a chapter
    // teaching a CONSTRUCTION out of pieces already owned adds no headword and
    // therefore no glyph. Half of that is wrong and the test caught it: the
    // chapter's headwords are whole Malayalam sentences, so they do enter the
    // census. dha goes 7/6 -> 8/7 and the ai sign 5/5 -> 6/6, because
    // adhyaapakanum and vaidyanum are INFLECTED forms of taught words and count
    // as fresh tokens even though no new word is taught.
    // What does hold is the part that matters: `shown` stays 68 and the
    // directly-owned overlap stays 59, so no NEW glyph is opened -- two already
    // open ones merely deepen by one field and one token each.
    // 479 -> 480: ML-S148-letter-pha adds the first headword containing ഫ and
    // gives that same glyph a direct script owner. Both census totals rise by one,
    // while the nine-glyph open set below is unchanged.
    // 484 -> 488: chapter 112's four lessons. THREE COUNTS BELOW MOVE, and the
    // chapter's three headwords are exactly why: ശ 17/13 -> 18/14 from
    // ആശുപത്രി, and ങ 14/12 -> 15/13 plus ബ 4/4 -> 5/5 from ബാങ്ക്. പോലീസ്
    // moves nothing because its glyphs are all script-owned already. `shown`
    // HOLDS AT 69 -- the chapter shows no glyph the track had not already
    // shown, which was verified before drafting and is re-proved here.
    // 480 -> 484: chapter 111's four lessons. Every count below is UNCHANGED,
    // and that is the chapter in one line: it teaches a CONSTRUCTION out of a
    // past form and a word both long owned, so it introduces no headword
    // carrying a glyph the census had not already seen and owned.
    // 488 -> 493: HL-C438's five `review` lessons closing the track's pre-A1
    // reinforcement debt. THREE FIELD COUNTS BELOW MOVE, NO TOKEN COUNT DOES,
    // and one pair reorders. This comment was wrong twice before it was right,
    // in the same way the chapter-109 note above records: the first draft said
    // every count held, the second said two moved. The assertion caught both.
    //
    // `review` is outside `CONTENT_TYPES`, so none of the five introduces a
    // headword and every TOKEN count holds. But a review lesson still HAS a
    // headword field, and three of the five carry glyphs that are
    // shown-but-not-directly-owned:
    //
    //   ള 24 -> 25 fields   from മലയാളി
    //   ശ 18 -> 19 fields   from ശരി
    //   ബ  5 ->  6 fields   from ബിൽ
    //
    // ബ reaching 6 ties it with ൈ, and the sort breaks ties on
    // `a.glyph.localeCompare(b.glyph)`, so ബ now precedes ൈ. The reorder is the
    // tie-break doing its job, not a count moving twice.
    //
    // `shown` HOLDS AT 69 and the directly-owned overlap holds at 60: the five
    // print only glyphs the track already shows, which is what a recall of
    // pieces taught one per chapter should do. No NEW glyph opens.
    // 493 -> 504: HL-C443 chapters 113-114, one letter lesson for each of the
    // nine glyphs below and two reviews. THE OPEN SET EMPTIES, which is the
    // point of the chapters: every one of the nine now heads a writing lesson,
    // so the directly-owned overlap rises 60 -> 69 and meets `shown`. `shown`
    // holds at 69 -- the eleven lessons print no glyph the track had not shown.
    // The nine, with the counts they carried when they closed (fields/tokens):
    // ള 25/19, ശ 19/14, ങ 15/13, ർ 12/12, ധ 8/7, ബ 6/5, ൈ 6/6, ഖ 3/3, ഛ 2/2.
    // 504 -> 643: chapters 115-141, 135 vocabulary lessons and four reviews.
    // Every headword was chosen from glyphs the track already teaches, so
    // `shown` and the directly-owned overlap both hold at 69.
    // 643 -> 652: HL-C443 anchor words, one short word lesson before each of
    // nine letter lessons that no earlier word held.
    expect(lessons).toHaveLength(652);
    expect(shown.size).toBe(69);
    expect([...shown].filter((glyph) => directlyOwned.has(glyph))).toHaveLength(69);
    expect(open).toEqual([]);

    const point = inventory.points.find((candidate) => candidate.id === "ML-A1-SCR-12");
    expect(point?.label).toBe("every headword character has a direct script-lesson owner");
    expect(point?.note).toContain("69 distinct Malayalam characters");
    expect(point?.note).toContain("all 69 occur in the headword of a writing/script lesson");
    expect(point?.note).toContain("U+0D36 ശ is directly owned by ML-W113-sha, and U+0D37 ഷ by ML-S121-letter-ssa");
  }, 60_000);

  it("keeps every point's probe key, and never an empty probe", () => {
    for (const point of inventory.points) {
      expect(point, `${point.id} has no probe key`).toHaveProperty("probe");
      expect(Array.isArray(point.probe) ? point.probe.length : 1, point.id).toBeGreaterThan(0);
    }
  });

  it("probes only atoms that EXIST, so a guessed id cannot under-report", () => {
    // The rule HL-C290 calls out by name, and the reason this file was
    // generated from a table rather than hand-written: a probe pointing at an
    // id somebody expects a future lesson to introduce resolves to "not
    // introduced" forever and sits in the report indistinguishable from the
    // honest gaps around it. Every probe here was checked against the merged
    // tree's atom set before the file was emitted, and the first draft was
    // refused by that check for inventing four `ML-SCRIPT-DIGIT-*` ids where
    // the corpus actually has `ML-SCRIPT-DIGITS-1-3-07` and its three siblings.
    const lessons = loadTrackLessons("malayalam");
    const taught = trackIntroducedAtoms(lessons, "malayalam");
    const unknown: string[] = [];
    for (const point of inventory.points) {
      for (const atom of point.probe ?? []) if (!taught.has(atom)) unknown.push(`${point.id}:${atom}`);
    }
    expect(unknown).toEqual([]);
  }, 60_000);

  it("keeps the derivation total in both directions", () => {
    // Every Spanish point derives into some Malayalam point, and none is
    // dropped. `notTransferred` is EMPTY on purpose: the points whose honest
    // Malayalam answer is "there is no such thing here" -- article
    // contractions, capital letters, written accents -- are enumerated as
    // points recording the absence rather than dropped from the walk, because
    // HL-C290 settled that restating a question around the target language's
    // machinery IS deriving it.
    const proxy = (inventory as unknown as {
      proxy: { notTransferred: { spanishPoints: string[]; why: string }[] };
    }).proxy;
    const dropped = new Set(proxy.notTransferred.flatMap((entry) => entry.spanishPoints));
    for (const entry of proxy.notTransferred) expect(entry.why.trim().length).toBeGreaterThan(0);
    const derived = new Set(
      inventory.points.flatMap((point) => (point as unknown as { derivedFrom: string[] }).derivedFrom),
    );
    const sourceIds = new Set(spanish.points.map((point) => point.id));
    for (const id of derived) expect(sourceIds.has(id), `derivedFrom names unknown ${id}`).toBe(true);
    for (const id of dropped) expect(sourceIds.has(id), `notTransferred names unknown ${id}`).toBe(true);
    expect([...derived].filter((id) => dropped.has(id)), "derived AND dropped").toEqual([]);
    const unaccounted = [...sourceIds].filter((id) => !derived.has(id) && !dropped.has(id));
    expect(unaccounted, "Spanish points that went missing from the walk").toEqual([]);
  });

  it("marks its own points as its own, in both directions", () => {
    for (const point of inventory.points) {
      const cast = point as unknown as { derivedFrom: string[]; malayalamSpecific?: boolean };
      expect(cast.malayalamSpecific === true, point.id).toBe(cast.derivedFrom.length === 0);
    }
    // Deliberately none. Every Malayalam column here answers a demand some
    // Spanish point also makes, even where the machinery is unrecognisable --
    // a dative subject doing what gustar does, a question particle doing what
    // inversion does, a verb that never agrees answering fourteen paradigm
    // points at once. If a later tranche adds a point with no Spanish question
    // behind it, this assertion is where it has to be declared.
    const specific = inventory.points.filter(
      (point) => (point as unknown as { derivedFrom: string[] }).derivedFrom.length === 0,
    );
    expect(specific.map((point) => point.id)).toEqual([]);
  });

  it("refuses to borrow an authority it does not have", () => {
    expect(inventory.about).toMatch(/PROJECT-DEFINED EDITORIAL EQUIVALENT, NOT AN EXTERNAL SYLLABUS/);
    expect(inventory.about).toMatch(/Kerala Sahitya Akademi/);
    expect(inventory.about).toMatch(/Kerala's state school syllabi/);
    expect(inventory.source).toMatch(/^PROJECT-DEFINED\./);
    expect(inventory.about).toMatch(/EXAM ENVELOPE: NONE EXISTS/);
    expect(inventory.about).toMatch(/no malayalam\/task-shapes\/ and no malayalam\/mocks\//);
    expect(inventory.about).toMatch(/NOT SEARCHED, BY INSTRUCTION/);
    expect(isExamInventoryComplete(inventory)).toBe(false);
    for (const dimension of EXAM_CONTENT_DIMENSIONS) {
      expect(inventory.scope[dimension].status, dimension).toBe("partial");
    }
  });

  it("says the register column was MEASURED here and not carried over from Tamil", () => {
    // The instruction this file was written under: where a track's
    // exam-levels.json entry carries no caveat, measure rather than importing
    // another track's shape. Tamil has a diglossia caveat and a register
    // column; Malayalam has no caveat, and the column it does get is a
    // DIFFERENT axis found by counting this corpus's own register fields.
    expect(inventory.about).toMatch(/THE REGISTER COLUMN IS MEASURED, NOT BORROWED/);
    expect(inventory.about).toMatch(/does not claim a literary\/spoken\s*\n?\s*diglossia/);
    const register = inventory.points.filter((point) =>
      point.category.startsWith("Bhaashaabhedam"),
    );
    expect(register).toHaveLength(5);
  });

  it("names an anchor for every point, and says what kind of anchor it is", () => {
    const anchors = (inventory as unknown as {
      anchors: { id: string; kind: string; title: string; note: string }[];
    }).anchors;
    expect(Array.isArray(anchors)).toBe(true);
    expect(new Set(anchors.map((anchor) => anchor.kind))).toEqual(
      new Set(["sourced-proxy", "external-framework", "project-owned", "editorial"]),
    );
    for (const anchor of anchors) expect(anchor.note.trim().length, anchor.id).toBeGreaterThan(0);
    const known = new Set(anchors.map((anchor) => anchor.id));
    for (const point of inventory.points) {
      const ids = (point as unknown as { anchorIds?: string[] }).anchorIds;
      expect(ids?.length, `${point.id} names no anchor`).toBeGreaterThan(0);
      for (const id of ids ?? []) expect(known.has(id), `${point.id} cites unknown anchor ${id}`).toBe(true);
    }
  });

  it("reports a joining column of 9 out of 11, and a script 9 characters short", () => {
    // Pinned so a future tranche has to say which points it moved. It may rise;
    // a fall means coverage was lost and wants explaining.
    const lessons = loadTrackLessons("malayalam");
    const coverage = measureExamCoverage(inventory, lessons);
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
    // 171 -> 174: ML-A1-Q-06, ML-A1-JOIN-07 and ML-A1-JOIN-08, all three
    // carried by one piece -- pol. The now/then pair was already taught and the
    // question form was the missing third of the set ML-C41-deixis-system
    // promised the reader they would work out for themselves.
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
    // "292 lessons" had rotted by well over a hundred files. The real gap was sport,
    // games and shows. FREE TIME IS LEFT OPEN ON PURPOSE -- ozhivusamayam needs
    // samayam, which nothing teaches, and a compound of an unowned part is a
    // headword smuggled in.
    // THE ONE GENERALISING SENTENCE WAS AGAIN WHERE THE ERROR LIVED, three chapters
    // running, AND IT WAS WRONG TWICE. Round one: the draft said the ya "comes
    // anyway", and pashu -- taught since chapter 60 -- gives pashuvaaNu with va.
    // ROUND TWO FOUND THE REPLACEMENT LATENT-FALSE IN ITS TURN. "The last vowel
    // chooses, and -u takes va" is refuted by enthu + aaNu = enthaaNu at sequence
    // 130, and by ithu standing un-glided inside the sinima lesson's OWN example
    // sentence -- because ML-C69-words (2360) teaches word-final chandrakkala as a
    // FAINT HALF-U and the track romanizes it (itu, entu), so by the book's own
    // account those words are vowel-final and take nothing at all. A learner holding
    // the round-one fix predicts *enthuvaaNu. The rule is now stated of FULL vowels,
    // with the half-u given its own paragraph and its own recall question.
    // Both rounds were latent-false in the same way: consistent with every token in
    // the repository and wrong about the language, so no gate could see either.
    // TEN merged lessons carried the broad ya-rule, and the count is not the
    // point -- it went three -> five -> eight -> ten across three review rounds,
    // each time because the sweep's grep was narrower than the defect, so the
    // record NAMES them: ML-C70-um (2390), ML-C70-um-more (2400), ML-C70-o (2410),
    // ML-R70-cherkkuka (2420), ML-C83-keralathil (2900), ML-C87-animacy (3050),
    // ML-R87-object-recall (3070), ML-C91-kada (3200), ML-C97-bhangi (body AND
    // Wrap-up, 3450), ML-R97-beauty-recall (3470).
    // THE SWEEP RULE THIS PRODUCED: whenever ML-Cnn-x is narrowed, ML-Rnn-* is a
    // site until checked. Chapter 98 learned that on ML-C97-bhangi, whose body was
    // narrowed while its Wrap-up still carried the claim, and then failed to
    // generalise it -- so three more recall partners went another round untouched.
    // THREE FURTHER SITES are filed as HL-C402 rather than patched: ML-C78-ude
    // (2700), ML-R78-genitive-recall (2710) and ML-C84-mukalil (2920) state it of
    // the GENITIVE, where it selects an ENDING rather than a glide, and pashu lands
    // in the wrong column (pashuvinte, not *pashuyute) as well as taking the wrong
    // glide -- so naming the vowel is not sufficient there. ML-R78 repeats the
    // table AND supplies the false answer as a Wrap-up drill.
    // All thirteen are latent-false the same way -- pashu is the only u-final noun
    // taught and none of its oblique forms appears in the corpus, so no token
    // contradicts any of them. ML-C70-o's fix was itself wrong on the first try:
    // it called chaaya AA-final, and ML-C39-chaaya's own romanization (chaaya,
    // short final a) makes it A-final, the same vowel as sinima.
    // 205 -> 206: ML-A1-LEX-47, the arts. THE PERCENTAGE MOVES, 84 -> 85, and it was
    // RECOMPUTED rather than carried over -- a stale rounded percent shipped once
    // before (ch94, 82 -> 83) and no gate reads this string.
    // THE POINT'S NOTE WAS ALREADY STALE BEFORE THE CHAPTER BEGAN, the second in a
    // row after LEX-37's: it said "nothing taught" while ML-C98-sinima (3500) had
    // closed the CINEMA limb one chapter earlier, so the probe wires that atom
    // alongside the four new ones.
    // NO NEW GRAMMAR, BY DESIGN. The three nouns each exercise a DIFFERENT one of
    // the three aaNu outcomes already taught: paattu ends in the half-u and gives a
    // sound up, nrttham ends in the anusvara and re-spells its m, katha ends in A
    // and takes the ya glide as sinima does. Given up, re-spelled, added.
    // THE REMEMBERED GLYPH CONTROL WAS SIMPLY WRONG: it held that U+0D7A chillu-NN
    // must report NOT covered, and it reports covered because ML-S131-chillu-nn
    // owns it -- at sequence 144, in the earliest script track, so it had not
    // rotted recently, it had never been right. Re-run with a control DERIVED from
    // the data instead of recalled.
    // Two inventions were caught before the first commit, both by checking the
    // corpus rather than the draft: sundaramaaya, an attributive form attested
    // NOWHERE, and the vocalic r romanized with a ring below where the track uses a
    // dot below in four witnesses (suhRttu, hRdayam, vRScikam, kRtajnjata).
    // The recall's Grammar Lens was drafted FOUR columns wide and narrowed to three
    // before any test ran -- ch98 shipped that and only the narration refusal count
    // caught it, which leaves an audio learner a placeholder where evidence belongs.
    // REVIEW CAUGHT THE ROMANIZATION IN THE LETTER BESIDE THE ONE I CHECKED.
    // nrttham was written nRtthaM. The conjunct in that word is TA+VIRAMA+TA, the
    // geminate tta, NOT TA+VIRAMA+THA: ML-S08 gives ta and ML-S130 gives tha for
    // "the breathy partner of the ta", ~14 corpus words spell tta as tt against 2
    // that do not, and vidyaartthi is genuinely TA+VIRAMA+THA. So nRtthaM wrote an
    // unaspirated geminate as the aspirate, collapsing the exact contrast ML-S130
    // exists to teach. It is nRttaM. The bullet above had DERIVED the vowel from
    // suhRttu -- whose own tta is spelled tt, so the same witness settled the
    // consonant too. Checking one letter and assuming the rest is what "derived,
    // not invented" was supposed to prevent.
    // Three overstatements went with it: "the chapter before last" (the half-u rule
    // is chapter 98, the chapter before); "the book has had it longest" of the
    // anusvara join (enthaaNu 130 < sukhamaaNo 170, and ML-C98-sinima says enthu
    // has welded since the very first question); and "the book has not shown you a
    // third" noun-beside-verb pair (ML-C33 names vaayana, ML-C34 names cOdyam).
    // The recall said FOUR words met aaNu across the two chapters; it is five, and
    // kali is now a fifth row -- ee and a are different vowels both choosing ya,
    // which is the narrow rule ch98 took four rounds to reach.
    // AND THE SWEEP RULE CAUGHT ITS OWN CHAPTER: an ELEVENTH ya-glide site,
    // ML-C98-kali, chapter 98's own first lesson, still credited the ya to "the
    // word ends in a vowel" -- untouched by the round-four fix, absent from the
    // ten-name list and from HL-C402, contradicted by ML-C98-sinima two lessons
    // later. Narrowed here to name ee.
    // 206 -> 207: ML-A1-LEX-29, clothing, footwear and accessories. The percentage
    // STAYS at 85 (85.19 rounds to 85); recomputed, not assumed.
    // THE POINT'S NOTE WAS ACCURATE THIS TIME, and that was VERIFIED rather than
    // assumed -- the two points before it (LEX-37, LEX-47) both had notes that
    // overstated their gaps, so this one was token-swept first. thuni is indeed the
    // only textile word taught, and eleven garment/accessory candidates all return
    // zero. The null result is recorded because two stale notes in a row made it
    // worth checking.
    // NO NEW GRAMMAR AT ALL, deliberately, after ch99 spent itself on three
    // contrasting joins: only two endings appear across the four words -- the
    // half-u (mundu, cheruppu) and -i (saari, thoppi) -- and the learner owns both.
    // The frame is ML-C42's attributive adjective: puthiya and pazhaya in front,
    // unchanged, which is that chapter's whole pattern on nouns it never had.
    // THE GLYPH PRE-CHECK FAILED TWICE AND IS NOW ONLY A SIGNAL. Its control had to
    // be DERIVED rather than recalled (ch99), and it then produced FALSE NEGATIVES:
    // it refused shirt over the chillu RR and "to wear" over DHA, both of which sit
    // in taught headwords already (ten and seven respectively). A check stricter
    // than the rule it stands in for is not a gate. shirt was dropped anyway, but
    // on SCOPE -- the four chosen words already cover all three limbs of the label.
    // EVERY ROMANIZATION WAS DERIVED LETTER BY LETTER, not word by word, which is
    // the correction ch99 earned by getting nrttham's vowel right and the consonant
    // beside it wrong. ca -> c, witnessed by cintikkuka, cevi, cheRiya, chuNTu,
    // cUlu, cEcci, cErkkuka and ML-S111's own ca, against the legacy ch spellings
    // chaaya, nenchu, uchakazhinju, acchan and muttacchan; zha -> zh-with-underdot,
    // witnessed by pazhaya, pazham, vazhi and taazhe, against vyaazham,
    // mazhakkaalam, ezhu and ML-S118's llla; NTu witnessed exactly by chuNTu; saari
    // witnessed inside samsaarikkuka's own romanization; puthiya and pazhaya read
    // off ML-C42's own fields rather than reconstructed.
    // THE FIRST DRAFT GAVE COUNTS INSTEAD OF NAMES -- "sixteen headwords", "the
    // lone chaaya", "six against two" -- and review found every one wrong (30 and
    // 5; 12 and 6). They were offered as EVIDENCE OF the letter-by-letter
    // discipline, in the same commit whose changelog says to name lessons and never
    // count them. The rule existed and the sentence claiming rigour broke it.
    // REVIEW ALSO CAUGHT THE JOIN PROVENANCE WRONG: the draft said the half-u
    // arrived with paattu and the ya with kali and bhangi. enthaaNu takes the
    // half-u at sequence 130 and kuttiyaaNu takes the ya at 810; those three are
    // the most recent words to USE the joins, not the words they began with. Same
    // defect ch98 recorded fixing ("kali said 'two words now do this'"), returned
    // one chapter later in the same shape.
    // AND THE ACCESSORY LIMB IS THIN: thoppi is headwear, which this chapter's own
    // candidate list treats as separate from kaNNata/vaacchu/baagu. Two limbs are
    // properly covered; the point closes on the basis LEX-37 closed with its
    // free-time limb open, with the gap written down rather than papered over.
    // 207 -> 208: ML-A1-LEX-06, a person's build and height. THE PERCENTAGE MOVES,
    // 85 -> 86 (85.6 rounds up); recomputed, not carried.
    // THE POINT'S NOTE WAS ACCURATE, verified before drafting for the second point
    // running: valiya and cheriya really are the only size adjectives taught, they
    // really do describe THINGS, and seven candidates return zero token-bounded.
    // TALL IS REACHED BY A DIFFERENT ROUTE, AND THAT IS THE CHAPTER'S POINT.
    // Malayalam's ordinary word for tall is uyaramulla, which needs the relative
    // participle uLLa -- taught NOWHERE, zero occurrences. Rather than smuggle it
    // in, the chapter teaches uyaram as a NOUN and reaches tall through the
    // dative-existential already owned: ML-C32-undu (650) states undu as "there is
    // / someone has" outright, and ML-C06-dative-subject (330) owns enikku, so
    // enikku uyaramuNTu is built entirely from taught pieces. Same refusal as
    // ozhivusamayam (ch98) and dharikkuka (ch100).
    // THE REAL TEACHING IS THAT ADJACENT WORDS ARE NOT SYNONYMS: cheriya and kuriya
    // differ only in their FIRST SYLLABLE (che against ku -- TWO characters, not
    // one) and do not mean the same thing, small against short, and tall is the
    // mirror of neither.
    // REVIEW CORRECTED FOUR CLAIMS, THREE OF THEM IN THE GENERALISING PROSE.
    // (1) uyaram said the anusvara does this "always, when something follows it".
    // ML-C96-eluppam says the opposite outright ("depends on what attaches to it")
    // and ML-C97-alla-sundaram warns against carrying it across -- and it is
    // refuted by THIS CHAPTER'S OWN recall romanization, uyaravuM: uyaram + um is
    // uyaravuM, with a va. Now scoped to aaNu and undu.
    // (2) "cheriya is for objects, kuriya for people" is false of both words AND
    // contradicts ML-C42-small (1020), which tells the learner to put cheriya "in
    // front of a word you already know" when they already own kutti (810) -- so the
    // book had invited cheriya kutti, which ch101 was about to forbid.
    // (3) "The others end -iya" is false for nalla (-lla) and pazhaya (plain -ya),
    // neither of which has an i-sign. The draft fix that ADDED nalla to the list is
    // what made the next sentence false -- a correction pass is as dangerous as a
    // draft.
    // (4) "a shape you have not met" / "a third shape": -nja and -ccha have been in
    // the book since paccha and manja at chapter 22, sequence 520. Now cited rather
    // than claimed as new.
    // Also: "I am tall" overstated a bare dative-existential (nearer "I have
    // height"; pokkam is the commoner noun for stature), and uyaram declared
    // ML-LEX-AANU-01 while containing zero occurrences of aaNu.
    // TWO INVENTIONS WERE CAUGHT BEFORE COMMIT BY A TOKEN SWEEP RATHER THAN BY
    // READING. aaL (person) was written into an example sentence and is taught
    // NOWHERE; it became kutti, owned since 810. And sahOdaran was about to be
    // quoted in Malayalam SCRIPT from ML-C32-undu, which gives that example only in
    // ROMANIZATION -- quoting it would have introduced an unowned word, so the
    // borrowed example was dropped instead. A WHOLE-TOKEN OWNER SWEEP now runs over
    // every new file before commit: it lists every Malayalam token with no headword
    // owner, leaving only joined forms built from owned pieces and metalinguistic
    // ending fragments. Also caught in draft: the adjective list said "every
    // adjective so far" while omitting nalla.
    // 208 -> 209: ML-A1-TIME-10, locating an event in time. THE PERCENTAGE HOLDS at
    // 86 (209/243 is 86.0); recomputed, not carried. THE POINT'S NOTE WAS HALF
    // STALE, and the stale half was the CASE. It said the point "requires the
    // locative on a time word, which is not taught", and Malayalam fixes a clock
    // time with the DATIVE -- raNTu maNikku, no form in -il anywhere -- which
    // ML-C06-dative-ikku has taught since sequence 320. The note's other half was
    // right and is why this closes cheaply: "the parts are all present and are
    // never assembled". CHAPTER 102 INTRODUCES NO NEW WORD AT ALL. mani is
    // chapter 18, the day-names are chapter 10, ethra is chapter 81, the
    // when-slot at the front of nyaan pOkuM is chapter 94, and the ending is
    // chapter 6. What was missing was one joint: -kku on the hour, NOTHING on the
    // named day, which stands bare exactly as innu and naaLe have since chapter
    // 94. NOT ONE MALAYALAM TOKEN GAINS A FIRST OWNER. All four content headwords
    // are multi-word, which continuity.ts keeps whole, so no earlier use becomes a
    // forward reference -- the trap lessons.d records as "a multi-word headword
    // owns no single token". thiNkaLaazhcha is the case that needed it: chapter
    // 10 prints all seven day-names under a seven-word headword and owns none of
    // them singly, so a one-word lesson for it would have turned chapter 10's own
    // table into a forward reference. It is taught as a USE instead. maNikku is
    // the same question from the other side -- the reader can already BUILD it,
    // and lessons.d says a buildable word must not be introduced later as a word.
    // THE CHAPTER REFUSES ONE CLAIM IT COULD HAVE MADE: that a day-name CANNOT
    // take an ending. It can, for other jobs; it does not NEED one for saying
    // when, and that is what the lesson says.
    // 209 -> 210: ML-A1-LEX-33, the education system. THE PERCENTAGE HOLDS at 86
    // (210/243 is 86.4); recomputed, not carried. THE POINT'S NOTE WAS PARTLY
    // STALE FOR THE SECOND TRANCHE RUNNING, and in the same shape: accurate about
    // the words it named, wrong about the scope it implied. It said "none of
    // paLLikkoodam, paadam or pareeksha is taught", which is true of those three
    // and leaves out that ML-C48-teacher (1290) and ML-C48-student (1300) own
    // adhyaapakan and vidyaarthi. The learner has had the PEOPLE of a school
    // since 1290 with nothing around them, and that gap is what the chapter is
    // built on: the place, the thing taught in it, the test of it, the number at
    // the end. A GLYPH CLAIM WAS CHECKED AGAINST DATA AND THE FONT RATHER THAN
    // AGAINST A NOTE, and it mattered twice. pareeksha needs the ksha conjunct,
    // and SCR-12's note lists "sha" among nine never-taught characters -- but
    // ML-S121-letter-ssa (471) TEACHES U+0D37, and SCR-12 means U+0D36, the other
    // sibilant. Reading the note instead of the data would have refused a word
    // that costs no script debt at all. Filed as HL-C407, with the finding that
    // SCR-12's "14" is one of three numbers: 12 distinct tokens in headword
    // position carry U+0D36, across 15 headword fields. THE FIRST DRAFT
    // EXPLAINED THAT GAP BACKWARDS -- it blamed tokens sitting inside
    // multi-word headwords, and such a token contributes one token AND one
    // field, so it cannot cause a discrepancy. The whole gap is shubha, in
    // FOUR headword fields; every other token sits in exactly one. Separately paaThaM needs
    // U+0D20, which NO headword in this corpus has ever used, so the
    // NotoSansMalayalam cmap was read directly before committing to the word.
    // THE TWO SANSKRIT WORDS DELIBERATELY DISAGREE: paaThaM ends in the anusvara
    // and gives way to -tth- under a case ending, exactly as HL-C400 narrowed that
    // rule; pareeksha ends in a and does not. Same chapter, same subject, same
    // source language -- and the chapter says the ENDING decides, not the meaning.
    // 210 -> 211: ML-A1-V-20, the progressive. THE PERCENTAGE MOVES, 86 -> 87
    // (211/243 is 86.8, which rounds up); recomputed, not carried.
    // THE NOTE WAS STALE IN THE SAME FIRST FAILURE MODE AS TIME-10, and half of
    // it was simply false. It said the -unnu form is "used inside ML-C05's
    // sentence but NEVER NAMED AS A FORM". It is named three times over, from
    // sequence 660: ML-C32-pokuka states "Malayalam's present tense is one word
    // long", ML-C33-cintikkuka repeats it, and ML-C50-parayuka says "paRayunnu is
    // the whole present tense". What is genuinely missing is the MAPPING -- that
    // this ONE form answers BOTH English presents, "I go" and "I am going" -- and
    // ML-C74-purpose (2560) already translates nyaan pOkunnu as "I am going"
    // without ever making the point. Parts present, never assembled, exactly as
    // ML-A1-TIME-10 was. NO NEW WORD: ippOL is owned since 1390, eppOL since
    // 2590, nii since 110, aaNu since 90, and varunnu is BUILDABLE from varuka
    // plus a rule the book states, so it is never headworded alone.
    // THE CHAPTER REFUSES ONE CLAIM: Malayalam's dedicated progressive in
    // -koNTirikkunnu is not taught and is not claimed. At A1 the simple present
    // IS how ongoing action is said, which is what this point asks for.
    // 211 -> 212: ML-A1-LEX-50, geography. THE PERCENTAGE HOLDS at 87 (212/243 is
    // 87.2); recomputed, not carried. THE NOTE WAS STALE IN THE DOMAIN MODE AGAIN,
    // AND A GATE CAUGHT WHAT MY NOTE-CHECK MISSED. It said "naadu ('home country')
    // only". naaTu is ML-C55-homeland, and ML-C80-uuru (2760) ALSO teaches uuru --
    // "a town, a village, the settled place somebody is from" -- and already builds
    // a scale out of the two: uuru is one settlement, naaTu floats from a district
    // to a country. I verified the five words the note implies are missing (all
    // absent, correctly) and did not check what else the DOMAIN held. The
    // duplicate-concept gate did: my draft's paTTaNaM collided with ML-NOUN-TOWN,
    // already ML-C80-uuru's tag.
    // THE COLLISION IMPROVED THE CHAPTER. paTTaNaM is dropped rather than renamed,
    // because "a town" is a concept this corpus already teaches and a second word
    // for it is not a gap. What was genuinely missing is the TOP of the scale (a
    // word that does not float, where naaTu does), the LARGE settlement, and the
    // functional word. So three new words -- raajyaM, nagaraM, talasthaanaM -- and
    // the point closes on those plus ML-LEX-C80-ORIGIN-01, which is uuru. The
    // label's four halves are all delivered; one of them was delivered in
    // chapter 80.
    // EVERY GLYPH WAS ALREADY SCRIPT-TAUGHT AND ALREADY IN USE IN HEADWORDS -- no
    // repeat of chapter 103's U+0D20. Romanizations copied from named witnesses:
    // sth from kaalaavastha (ML-C20), th from ML-S130-letter-tha and atithi and
    // katha. talasthaanaM opens with thala, the head from ML-C13's body-part
    // bundle, and the second half sthaanaM is given in ROMANIZATION ONLY because
    // no lesson teaches it -- the ch103 treatment of palli and kooTaM.
    // ONE INVENTED EXAMPLE WAS CAUGHT BEFORE THE FIRST VALIDATE: the draft wrote
    // "indya oru raajyaM", and oru is taught NOWHERE -- it is the open point
    // ML-A1-ART-02, which this corpus has been using untaught in six lessons.
    // 212 -> 213: ML-A1-TIME-09, before and after. THE PERCENTAGE MOVES, 87 -> 88
    // (213/243 is 87.7); recomputed, not carried.
    // THE NOTE'S FIRST HALF IS TRUE AND ITS SECOND HALF IS NOT, AND THE DRAFT OF
    // THIS COMMENT CELEBRATED IT AS THE FIRST TRUSTWORTHY NOTE OF THE RUN. It read
    // "munpu and shesham are not taught" -- verifiably true, neither appears
    // anywhere -- and then "pinne (then) is the nearest and it SEQUENCES TURNS
    // RATHER THAN EVENTS". ML-C68-at-first (2350) teaches the opposite in as many
    // words: "aadyaM puts an ACTION first in TIME, and it pairs with pinne", its
    // summary table is headed "an action in time", and its example is aadyaM uuNu,
    // pinne chaaya -- lunch first, then tea, which orders the world.
    // I TOOK THE NOTE'S OWN ANTICIPATION OF THE DOMAIN CHECK AS EVIDENCE THAT THE
    // CHECK HAD BEEN DONE. That is the domain mode again, one level up: not
    // trusting a note's list of words, but trusting a note's self-assessment. So
    // this is the FIFTH stale note in five tranches, not the first sound one.
    // THE CHAPTER'S ORGANISING CONTRAST WAS REBUILT ON WHAT THE CORPUS ACTUALLY
    // TEACHES. Both pairs order real events. What aadyaM and pinne cannot do is
    // NAME what they measure against: they lay a run out, this then that. munpu
    // and shEshaM fix an event against a NAMED point. That is what "anteriority
    // and posteriority" asks for, so the point still closes -- on a true framing.
    // NO NEW GRAMMAR: both are postpositions on the dative, so raNTu maNikku munpu
    // reuses ML-C102-manikku's form and uuNinu shEshaM shows the dative's other
    // shape on uuNu, owned since 930. The draft called that ending derivable and
    // it was not -- ML-C06 says the choice is "by the sound the noun ends in" and
    // never says WHICH sounds, so the lesson now supplies the one rule it needs.
    // shEshaM LANDS ON KNOWN SCRIPT DEBT AND NOTHING CAN BE DONE HERE: U+0D36 has
    // no script lesson (SCR-12's "sha", 12 tokens across 15 headword fields), it
    // does not trip neverTaughtGlyphs because it sits inside other lessons'
    // examples, and closure is read in ORDER -- shari needs it in CHAPTER 1, so a
    // lesson at 3870 retires nothing. Filed against HL-C407.
    // 213 -> 214: ML-A1-LEX-42, medicine and treatment. THE PERCENTAGE HOLDS at 88
    // (214/243 is 88.1); recomputed, not carried.
    // THE NOTE HELD ON ALL FIVE CHECKS, AND THIS TIME I RAN THE FIFTH ONE. "The
    // symptoms are taught (ML-A1-LEX-27) and nothing that treats them is" is a
    // claim about existing material, which is the shape that caught me on
    // ML-A1-TIME-09. So LEX-27's five probe atoms were opened: cuma, pani, vEdana
    // and kshiiNaM are all taught headwords in chapter 63. True.
    // THE DOMAIN CHECK PAID OFF AGAIN, AND IT SHAPES THE CHAPTER: vaidyan, the
    // doctor, is taught at ML-C48-doctor (1310). So the learner has had the
    // SYMPTOMS and the PERSON and nothing in between -- no substance, no practice.
    // That gap IS the chapter, the same way teacher-and-student-without-a-school
    // was chapter 103's.
    // THREE WORDS, CHOSEN FOR ZERO SCRIPT DEBT. marunnu, cikitsa and vRtti cover
    // the label's three halves and every glyph in them is already script-taught.
    // aashupatri (hospital) and guLika (tablet) were DROPPED, not because they are
    // wrong but because each adds a headword to an already-open never-taught
    // character -- U+0D36 and U+0D33 -- and neither is in the label.
    // vRtti IS GLOSSED HONESTLY AS THE EVERYDAY WORD. It is cleanliness, not
    // hygiene-as-a-subject; the lesson says so rather than letting the exam
    // label's word stand unexamined. Its Sanskrit sense is CONDUCT, and the road
    // from conduct to cleanliness is offered as the traditional account rather
    // than asserted -- the ch103 treatment of paLLikkooTaM.
    // 214 -> 215: ML-A1-SCR-15, punctuation. THE PERCENTAGE HOLDS at 88
    // (215/243 is 88.5); recomputed, not carried.
    // NINE SPANISH POINTS BEHIND ONE MALAYALAM POINT, AND ZERO NEW HEADWORDS.
    // A1-O3-01..09 are the full stop, comma, colon, question and exclamation
    // marks, parentheses, quotation marks, hyphen, dialogue dash and slash. The
    // Spanish track closed its own Puntuacion category the same way in ch424 --
    // "eight marks the corpus printed constantly and never named."
    // THE NOTE'S CLAIM WAS OPENED, NOT TRUSTED. "Not one lesson teaches any of
    // them" is a claim about existing material. Six Malayalam lessons matched a
    // punctuation grep and FIVE were the word "command" matching "comma"; the
    // sixth, ML-C70-um-more:46, mentions ENGLISH's commas contrastively and
    // teaches no Malayalam mark. The claim is true.
    // THE MARKS WERE ALREADY ON THE PAGE, WHICH IS THE CHAPTER. ML-C02-ninre-
    // peru-entaanu (sequence 130) prints a question mark in its HEADWORD, and
    // ML-C72-ennu prints quotation marks around its quoted sentence. The book
    // has shown these marks since chapter 2 and never named one.
    // THE LOAD-BEARING FACT IS CHECKABLE IN UNICODE: the Malayalam block
    // U+0D00-U+0D7F contains ZERO characters of category P. The script supplied
    // no punctuation of its own, so every mark here is borrowed -- and the older
    // answer, the danda, is the one SA-C60-danda teaches in the Sanskrit track.
    // THREE MARKS LAND ON JOBS MALAYALAM HAD ALREADY FILLED, and that is what
    // the recall lesson drills: -um on every list item does the comma's work,
    // the question words and the -oo ending of ML-C03-sukhamaano do the question
    // mark's, and ennu does the quotation marks'.
    // 215 -> 216: ML-A1-ART-02, the indefinite oru. THE PERCENTAGE TICKS, 88
    // -> 89: 216/243 is 88.9. The first draft of this comment said "still 88"
    // and the format assertion caught it. Recomputed, not carried.
    // THIS ONE WAS A RELOCATION, NOT AN ADDITION, and the note said so: oru is
    // never a headword and has no atom, yet it stands as a STANDALONE TOKEN in
    // six lessons already, the earliest being ML-C18-mani at sequence 470. So a
    // lesson appended at the end would have left six forward references intact.
    // ML-C18-oru lands at 465, immediately before the first use.
    // THE CENSUS WAS RE-DERIVED RATHER THAN TRUSTED, and it matches the note
    // exactly: six files by a word-bounded count against the Malayalam block,
    // ten by a bare substring grep (which also catches orupaadu's family in
    // chapter 79 and a compound in ML-C51-blessing). Count the token.
    // THE NOTE'S "NEVER TAUGHT" NEEDED ONE QUALIFICATION. ML-C49-perhaps:43
    // already GLOSSED oru in prose while taking orupakshee apart -- and glossed
    // it loosely, as "the same numeral this book counted with". The numeral this
    // book counted with is onnu; oru is the shape it takes before a noun. That
    // line is corrected in this tranche, which is the kind of debt a relocation
    // is supposed to find.
    // PLACEMENT WAS DECIDED BY WHAT IS TEACHABLE, NOT BY WHAT IS EARLIEST. The
    // first draft put the lesson in chapter 7 beside the numerals, where almost
    // no countable noun exists yet -- and its examples (oru maNi, oru caaya)
    // were themselves drawn from lessons that come LATER, reintroducing the very
    // forward references being removed. Chapter 18 is the right home: it is the
    // first user, and everything through chapter 17 is available. Even there the
    // choice is narrow, because the colour, family, body-part and food words all
    // sit inside MULTI-WORD headwords and so own no single token; peru is the
    // worked example because it is one of five single-token nouns available.
    // 216 -> 217: ML-A1-NEG-03, negative coordination. 217/243 is 89.3, still 89.
    // ITS NOTE NAMED A BLOCKER THAT NO LONGER EXISTS -- failure-mode 3, a note
    // simply false about what is taught. "Untaught, and it depends on the
    // missing coordinator at ML-A1-JOIN-01" was written before chapter 70, which
    // teaches -um across four lessons (2390-2420). The coordinator arrived; the
    // note was never re-read.
    // THE CONSTRUCTION WAS CENSUSED BY SHAPE, NOT BY THE ENGLISH WORD. A grep
    // for "neither" finds only ordinary English prose (a sound description in
    // ML-C33-ezhutuka, a remark in ML-C106-ordering). The real check is a regex
    // for two -um-marked words followed by a negative on the same line: ZERO
    // matches across the corpus. Genuinely untaught.
    // IT COSTS NO NEW VOCABULARY, which is the whole shape of the point:
    // Malayalam builds it from -um on every item plus a negative at the end, and
    // both halves are long taught -- illa since sequence 40, alla since chapter
    // 77. Two atoms, three lessons, no new headword.
    // THE CHAPTER EXISTS BECAUSE OF THE SECOND NEGATIVE. ML-C77-alla teaches
    // that Malayalam makes you pick where English spends one word on both, and
    // English's "neither ... nor" hides that choice completely. So the tranche
    // is two lessons, not one: caayayum kaappiyum ILLA against adhyaapakanum
    // vaidyanum ALLA, identical on the left and different only in the last word.
    // 217 -> 218: ML-A1-LEX-38, AND THE TRACK CROSSES 90 PER CENT: 218/243 is
    // 89.7, which rounds up. The telephone, the post, the internet and an
    // e-mail address. FIVE Spanish points behind one Malayalam point
    // (A1-NE09-01..03, NE09-06, NE11-01) -- the largest single payoff left in
    // this file.
    // THE NOTE'S "Nothing" HELD, and the check that confirmed it is the one that
    // has caught me before: kattu (a letter) appears to be taught, because
    // grepping for it matches INSIDE akattu ("inside", ML-C95). Word-bounded
    // against the Malayalam block it returns ZERO. Count the token.
    // FOUR OF THE FIVE WORDS COST NO SCRIPT DEBT AT ALL. kattu, thapaal and
    // imeyil use only glyphs already shown and directly owned; inrarneRRu
    // deepens the already-open chillu-r and opens nothing new.
    // THE TELEPHONE IS TAUGHT BY EAR, AND THAT IS NOT A SHORTCUT. phooN needs
    // the chillu NN, which ML-S131 teaches at sequence 144 but
    // data/scripts/malayalam.json omits, so writing it trips uncovered-glyphs
    // and integration.test.ts turns that queue into a hard failure. HL-C398
    // records why a curriculum chapter must not simply add it: each listed
    // chillu is pinned to a SOURCED stroke-order animation, "a citation cannot
    // be guessed", and the inventory is a different owner's surface by design
    // (HL24). So the word is given in romanization with the lesson saying its
    // written shape comes when its letter does -- chapter 88's precedent for
    // eighty, and before that ML-C07-numbers-6-10's. The phone is the SECOND
    // word the curriculum has wanted that contains the glyph.
    // 218 -> 219: ML-A1-V-19, the present perfect. 219/243 is 90.1, still 90.
    // THE NOTE WAS HALF STALE AND HALF RIGHT, which is why this one was worth
    // opening carefully. "Neither the participle nor the construction appears"
    // -- the PARTICIPLE half is FALSE: ML-C90-past-participle teaches vanna at
    // sequence 3180, and it even establishes the regularity the Spanish source
    // points ask about (A1-V-10 irregular participles, A1-V-14 the regular one),
    // saying outright that the rule is "about the letter and not about the
    // tense". The CONSTRUCTION half is true: -ittu returned ZERO files.
    // SO THE CHAPTER COSTS NO VOCABULARY. Both halves of the perfect were
    // already owned -- the past vannu at ML-C32-varuka, and undu at ML-C32-undu
    // (sequence 650), whose gloss has read "[someone] has" since it was written.
    // What was missing was the join, and the recall asks the learner outright
    // how many new words the chapter cost.
    // FOUR SHAPES OFF ONE PAST FORM is the chapter's spine: vannu, then vanna
    // (a vowel sign taken OFF, per ML-C90), then vannittu (-ittu put ON), then
    // vannittundu. Each row is the row above with one change.
    expect(coverage.covered).toBe(220);
    expect(coverage.unmapped).toBe(23);
    expect(coverage.partial).toBe(0);
    // 162 -> 163: the HL-C354 ordinal tranche closed ML-A1-NUM-05 (chapters
    // 67-68). It is the ONLY point that moved, and the numeral column below
    // says so on its own line rather than leaving the total to speak for it.
    // The two still open in that column are ML-A1-NUM-04 (counting past
    // twenty) and ML-A1-NUM-08 (measures); neither is ordinal work.
    expect(coverage.byCategory["Sankhya (numerals and quantity)"]!).toEqual({
      enumerated: 9,
      covered: 8,
    });
    // THE HEADLINE. Malayalam joins clauses with a clitic -um for "and", a
    // quotative ennu for "that", and participles for everything else, and the
    // corpus teaches none of them. Chapter 64 is called "Five Words That Join"
    // and four of its five words are adverbs -- pinne, udane, chilappol,
    // maathram -- so only ennaal ("but") is a real connective. The second
    // covered point is the -i participle inside the goodbye poyi varaam, which
    // the corpus teaches without ever naming it as a way of joining clauses.
    // Same shape as Kannada's chapter 64 finding, measured independently.
    // 3 -> 4: ML-A1-JOIN-06, the quotative ennu -- the point's own note calls it
    // Malayalam's single most productive subordinator, and chapter 72 spends two
    // atoms to open four verb frames whose verbs were all already taught.
    // THE TITLE MOVED WITH THE NUMBER, for the third time. A test name carrying
    // a stale count reads as a finding and is not one.
    const joining = coverage.byCategory["Samuchayam (joining and subordination)"]!;
    // 6 -> 8: ML-A1-JOIN-07 (when) and ML-A1-JOIN-08 (if), taught together in
    // chapter 75 as a minimal pair, since one ending is the whole difference
    // between them. Three of the five still open are the distributive, the
    // relative participle and JOIN-01's clause half.
    // 8 -> 9: ML-A1-JOIN-10, the relative participle, which this file called the
    // biggest structural gap in it. Chapter 90 closes it together with
    // ML-A1-PRON-08, since both name the same construction. Two of the three
    // still open are the distributive and JOIN-01's clause half.
    expect(joining).toEqual({ enumerated: 11, covered: 9 });
    // Direct-owner closure is measured independently above: 59 of 68 headword
    // characters have a writing/script-lesson headword owner. Correcting that
    // census did not move this category, because correcting a census does not
    // teach a glyph -- it stayed at 11/16 across that change.
    // 11 -> 12: ML-A1-SCR-15, punctuation, closed by chapter 108. That one DOES
    // move it, and not by teaching a glyph either: the marks are not Malayalam
    // characters at all. The Malayalam block U+0D00-U+0D7F contains no
    // punctuation, which is why chapter 108 costs this track zero script debt
    // and why its lessons contribute nothing to the direct-owner census above.
    // The four still open in this column are SCR-12 (the never-taught
    // characters, whose own count is under review at HL-C407), SCR-13 (capital
    // letters), SCR-14 (written accentuation) and SCR-16 (abbreviations).
    // SCR-13 and SCR-14 are enumerated-not-applicable: Malayalam has neither
    // case nor accent marks, so nothing can close them.
    expect(coverage.byCategory["Lipi (script and orthography)"]!).toEqual({
      enumerated: 16,
      covered: 12,
    });
    // The two columns that carry this track.
    expect(coverage.byCategory["Kriya (the verb)"]!.covered).toBeGreaterThan(15);
    expect(coverage.byCategory["Vyavahaaram (communicative functions)"]!.covered).toBeGreaterThan(30);
    expect(formatExamCoverage(coverage)).toContain(
      "malayalam A1 (partial inventory): 220/243 points covered (91%)",
    );
  }, 60_000);
});
