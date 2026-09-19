import { describe, expect, it } from "vitest";
import { loadTrackLessons, loadExamInventory } from "../../src/loader.js";
import {
  measureExamCoverage,
  trackIntroducedAtoms,
} from "../../src/exam-inventory.js";

describe("the committed French A1 inventory", () => {
  const inventory = loadExamInventory("french", "A1");

  it("keeps every point's probe key, because a MISSING probe scores as covered", () => {
    // `covered` is `point.probe !== null && …`. A point whose `probe` key is
    // absent reads as `undefined`, which is not `null`, so it scores COVERED
    // while demonstrating nothing. Authoring this file with a helper that
    // omitted null-valued keys reported 65/74 (88%) for a track with nine
    // grammar atoms, and only the implausibility of the number caught it.
    for (const point of inventory.points) {
      expect(point, `${point.id} has no probe key`).toHaveProperty("probe");
    }
  });

  it("refuses an empty probe, which would score as covered", () => {
    for (const point of inventory.points) {
      expect(Array.isArray(point.probe) ? point.probe.length : 1).toBeGreaterThan(0);
    }
  });

  it("probes only atoms that EXIST, so a guessed id cannot under-report", () => {
    // The other direction of the same failure: `FR-LEX-CAFE-01` exists but
    // `FR-LEX-VERT-01` does not, because the suffix varies per lesson. A guessed
    // id resolves to "not introduced", which is fail-safe, silent, and wrong —
    // it reports taught material as a content gap.
    const lessons = loadTrackLessons("french");
    const taught = trackIntroducedAtoms(lessons, "french");
    const unknown: string[] = [];
    for (const point of inventory.points) {
      for (const atom of point.probe ?? []) if (!taught.has(atom)) unknown.push(`${point.id}:${atom}`);
    }
    expect(unknown).toEqual([]);
  }, 60_000);

  it("reports the gap as GRAMMAR-shaped, which is the finding", () => {
    const lessons = loadTrackLessons("french");
    const coverage = measureExamCoverage(inventory, lessons);
    expect(coverage.enumerated).toBe(74);
    // Pinned so a future tranche has to say which points it moved. It may rise;
    // a fall means coverage was lost and wants explaining.
    //
    // 20 -> 25: HL-C229 authored chapter 32 and took L'interrogation from 0/5 to
    // 5/5 -- the first time this loop closed end to end, with the plan naming the
    // gap, the inventory naming the five points, nine lessons teaching them, and
    // the probes then resolving against real atoms. The number moved because the
    // CORPUS changed, not because the target was edited.
    //
    // 25 -> 26: retiring hand-written chapter 6 closed A1-PRON-03, obligatory
    // liaison. The hand-written chapter mentioned the six/dix -s and the neuf
    // f-to-v in passing inside two `sounds` blocks; the generated chapter teaches
    // liaison as a named rule with its own atom, which is what a probe can
    // resolve against. Same rule here: the corpus changed, not the target.
    //
    // 26 -> 27: retiring hand-written chapter 8 closed A1-LEX-07, telling the
    // time. The hand-written chapter stopped at whole hours and named et quart,
    // et demie and moins le quart in one sentence while deferring them, so the
    // corpus could not have satisfied the point however the probe was written.
    // The chapter now teaches all three, and the probe lists all seven atoms
    // rather than a sample: a candidate asked for half past does not get partial
    // credit for o'clock.
    //
    // 27 -> 30: retiring hand-written chapter 1, the first chapter in the book,
    // closed three at once -- and all three were unmapped for the same reason,
    // which is the finding. A1-LEX-01 is "greetings and farewells" in a track
    // whose opening chapter is called Greetings: the farewells had atoms because
    // chapter 4 was generated, the greetings did not because chapter 1 was not,
    // so half the point existed and the whole point read as absent. A1-D-01 (the
    // definite article) and A1-A-01 (adjective agreement) are the two grammar
    // rules those greetings run on -- bon versus bonne is agreement, and le/la is
    // where the gender it agrees with becomes visible. Both were taught on page
    // one from the beginning and neither could be probed, because a hand-written
    // chapter's grammarlens owns no atom.
    //
    // 30 -> 31: retiring hand-written chapter 2 closed A1-V-12, reflexive verbs
    // with `se` in the present. Chapter 27 has conjugated s'asseoir and se lever
    // through the whole present since it was written, and chapter 2 has been
    // teaching je m'appelle -- but the CONSTRUCTION was owned by nobody: ch27's
    // atoms type the two verbs and their stems, and ch2 was hand-written, so a
    // corpus that fully teaches the point had no atom that named it. The probe
    // lists the rule and the two conjugated verbs, because the rule alone is not
    // the present tense and the verbs alone were typed as lexis.
    //
    // A1-P-04 was already covered and its probe is corrected in the same pass:
    // it read FR-GRAMMAR-PLEASE-REGISTER-04, chapter 19's s'il vous plait, which
    // DEMONSTRATES the tu/vous register without naming it. Chapter 2 owns the
    // point directly and both its atoms are added.
    //
    // 31 -> 32: A1-V-11, vouloir / pouvoir / devoir in the singular. Nothing was
    // authored for it. It was found while writing the A2 inventory -- chapter 33
    // has given each of the three its own lesson with je / tu / il printed since
    // it was written, and types the chain rule besides. The point was reading as
    // a content gap and would have sent an author to write what already exists,
    // which is the failure mode an inventory is supposed to PREVENT.
        // 27 -> 28: the chapter-9 split closed A1-LEX-06, days/months/seasons. This
    // one was deliberately held back through two earlier tranches: the days were
    // taught, but the track owned two headwords -- `les mois` and `les saisons`
    // -- for twelve months and four seasons, so any probe naming a month would
    // have been a claim the corpus could not support. Splitting chapter 9 into
    // three chapters taught all sixteen, and the probe resolves honestly.
    // Both closures were authored on separate branches from the same base of 27
    // and met in this merge, so the figure below is RE-MEASURED against the merged
    // tree rather than obtained by adding three and one to twenty-seven.
    // The two branches above met in this merge, each written from its own base,
    // so the figure below is RE-MEASURED against the merged tree by running the
    // suite, never obtained by adding the two branches' deltas.
    //
    // 31 -> 33: retiring chapters 17 and 19 gave `avoir` and `etre` a typed atom
    // per person instead of one lesson holding each whole paradigm, which is
    // what the two remaining verb points were waiting for.
    //
    // 33 -> 42, AND NOT ONE LESSON CHANGED. Nine points were taught in full and
    // carried `probe: null`, which this module documents as "no atom in the
    // corpus corresponds to this point" -- a finding, scored uncovered. Here it
    // was not a finding: it was 41 points nobody had written a probe for, and the
    // chapters that closed nine of them were generated after the inventory was
    // authored. Each was confirmed by reading the lesson, not by an atom name
    // looking right:
    //
    //   A1-V-01  A1-V-02   chapter 19 gives etre ONE LESSON PER PERSON and
    //     chapter 17 does the same for avoir, so there are twelve atoms where an
    //     author looking for `FR-VERB-ETRE-PRESENT` finds none. That shape is not
    //     an accident: `maxNewGrammarCellsPerLesson` is 1, so a paradigm CANNOT be
    //     one atom, and an inventory that expects one will always read it as absent.
    //   A1-V-14  A1-V-15   the avoir-perfect (chapter 18) and the etre-perfect with
    //     its agreement (chapter 20), including FR-C16-accord-unifie, which shows
    //     the two agreement rules are one rule.
    //   A1-P-01   six subject pronouns across six lessons, plus the rule that makes
    //     them obligatory -- parle, parles and parlent are one sound.
    //   A1-N-02   FR-C01-le-la: "a grammatical gender baked into the noun ... the
    //     gender can't be guessed."
    //   A1-A-03   FR-C13-vin-rouge states the position rule AND its exception.
    //   A1-PRON-05  FR-W02-cedille.
    //   A1-LEX-08   le temps, il fait chaud, il pleut -- a whole weather lesson.
    //
    // The other 32 now each carry a note saying what the track holds and what is
    // missing, so the next author does not repeat the 232-lesson read.
    //
    // 42 -> 67. Seven chapters, thirty-five items, thirty-nine atoms, forty-two
    // lessons. Twenty-five points close and EIGHT COLUMNS go to full:
    //
    //   Le verbe            12/17 -> 17/17
    //   Lexique de base     10/10 -> 10/10  (already full)
    //   Les pronoms          2/5  ->  5/5
    //   La phrase            0/4  ->  4/4
    //   Le nom               1/4  ->  4/4
    //   Prononciation        4/5  ->  5/5
    //   La negation          2/3  ->  3/3
    //   L'interrogation      5/5  ->  5/5  (already full)
    //
    // The ratio is high because the track was FULL OF UNOWNED FUNCTION WORDS. et
    // was glue in two practice dialogues; et toi ? was a phrase in two more; de
    // was inside du and never met alone; que was inside est-ce que and parce que
    // and never met at all. Eleven words this tranche teaches were already in the
    // book's own prose with no lesson owning any of them, which is why 35 items
    // buy 25 points.
    expect(coverage.covered).toBe(67);
    for (const full of [
      "Le nom", "Les pronoms", "Le verbe", "La negation",
      "L'interrogation", "La phrase", "Prononciation et orthographe",
      "Lexique de base",
    ]) {
      const column = coverage.byCategory[full];
      expect(column?.covered, full).toBe(column?.enumerated);
    }
    // Les prepositions moves 0 -> 1 and no further, on purpose: a, de and their
    // four contractions close A1-PREP-01, and the other three points are place,
    // country and time preposition SETS -- six, four and four words each, one
    // point apiece. They are the worst ratio left in the inventory and the next
    // tranche's job.
    expect(coverage.byCategory["Les prepositions"]).toEqual({ enumerated: 4, covered: 1 });
  }, 60_000);

  it("closes A1-PH-02 on a word the book was already using and had never taught", () => {
    // Named rather than left to the aggregate. `et` appears in FR-C03-practice as
    // "the glue word", glossed inside a dialogue and owned by nothing; `ou` and
    // `mais` were not in the corpus at all. The point needs all three, so it
    // could not close on the one the book was already saying.
    //
    // Both halves falsified before this was kept: a fabricated id fails the
    // "probes only atoms that EXIST" test above, and nulling this probe drops the
    // total to 66.
    const point = inventory.points.find((p) => p.id === "A1-PH-02");
    expect(point?.probe).toEqual(["FR-LEX-ET-01", "FR-LEX-OU-CONJ-02", "FR-LEX-MAIS-03"]);
  });

  it("never lets an unmapped point read as 'nobody has looked yet'", () => {
    // The rule Marathi has had since HL-C290. Thirty-two of this inventory's 41
    // unmapped points carried no note, so "the corpus does not teach it" and
    // "nobody has checked" were the same JSON -- and nine of the 41 turned out to
    // be taught in full. A note is what stops that read being redone.
    for (const point of inventory.points) {
      if (point.probe !== null) continue;
      expect(point.note?.trim(), `${point.id} is unmapped and must say why`).toBeTruthy();
    }
  });
});

describe("the committed French A2 inventory", () => {
  const inventory = loadExamInventory("french", "A2");

  it("keeps every point's probe key, because a MISSING probe scores as covered", () => {
    for (const point of inventory.points) {
      expect(point, `${point.id} has no probe key`).toHaveProperty("probe");
    }
  });

  it("refuses an empty probe, which would score as covered", () => {
    for (const point of inventory.points) {
      expect(Array.isArray(point.probe) ? point.probe.length : 1).toBeGreaterThan(0);
    }
  });

  it("probes only atoms that EXIST, so a guessed id cannot under-report", () => {
    // This caught a real one on the first run. A2-F-11 probed
    // FR-IDIOM-CA-MARCHE-AGREEMENT-01, which is a real, committed, correctly
    // spelled unit -- declared in `introduces_idioms`. `measureExamCoverage`
    // resolves against `introducedAtoms`, which reads `introduces.knowledge` and
    // the block directives and NOTHING else, so an idiom or a culture claim in a
    // probe is indistinguishable from a typo: the point silently reports
    // uncovered. The three namespaces are separate and only one of them is
    // probeable.
    const lessons = loadTrackLessons("french");
    const taught = trackIntroducedAtoms(lessons, "french");
    const unknown: string[] = [];
    for (const point of inventory.points) {
      for (const atom of point.probe ?? []) if (!taught.has(atom)) unknown.push(`${point.id}:${atom}`);
    }
    expect(unknown).toEqual([]);
  }, 60_000);

  it("mirrors the A1 file's categories, so the two read as one ladder", () => {
    // The A2 file is comparable to the A1 file BY CONSTRUCTION, not by accident:
    // same category names in the same order, plus `Actes de parole` at the front
    // (A2 is where the exam starts testing what you can DO with a paragraph) and
    // `Lexique` renamed from `Lexique de base` because it is no longer basic.
    // A future reader must be able to see a point move from one column to the
    // other; that only works if the columns are the same shape.
    const a1 = new Set(loadExamInventory("french", "A1").points.map((p) => p.category));
    const a2 = new Set(inventory.points.map((p) => p.category));
    const shared = [...a1].filter((c) => a2.has(c));
    expect(shared.sort()).toEqual([
      "L'adjectif", "L'adverbe", "L'interrogation", "La negation", "La phrase",
      "Le nom", "Le verbe", "Les determinants", "Les prepositions", "Les pronoms",
      "Prononciation et orthographe",
    ]);
    expect([...a2].filter((c) => !a1.has(c)).sort()).toEqual(["Actes de parole", "Lexique"]);
  });

  it("reports the gap as FUNCTION- and PAST-TENSE-shaped, which is the finding", () => {
    const lessons = loadTrackLessons("french");
    const coverage = measureExamCoverage(inventory, lessons);
    expect(coverage.enumerated).toBe(104);
    // Pinned so a future tranche has to say which points it moved. It may rise;
    // a fall means coverage was lost and wants explaining.
    expect(coverage.covered).toBe(16);
    // The shape, and it is a sharper finding than the number. FIFTEEN of the
    // sixteen `Actes de parole` are uncovered, because A2 is the level at which
    // the exam stops asking for words and starts asking for a paragraph that
    // does something -- and this corpus is a vocabulary corpus with a grammar
    // spine. The one that is covered, grading an opinion, is covered by half:
    // the corpus can say `aimer` against `aimer bien` and cannot yet disagree.
    // The same story runs through the past: A2's construct is dominated by the
    // passe compose and the imparfait, and the two French chapters that carry
    // them are still HAND-WRITTEN, so neither owns an atom.
    expect(coverage.byCategory["Actes de parole"]).toEqual({ enumerated: 16, covered: 1 });
    for (const empty of ["Le nom", "Les determinants", "L'adjectif", "Les prepositions",
                         "L'adverbe", "La phrase", "La negation"]) {
      expect(coverage.byCategory[empty]?.covered, empty).toBe(0);
    }
    // Where it IS strong is exactly where the retirement work has already been:
    // everyday verbs, the question system, and register.
    expect(coverage.byCategory["Lexique"]!.covered).toBe(7);
  }, 60_000);
});
