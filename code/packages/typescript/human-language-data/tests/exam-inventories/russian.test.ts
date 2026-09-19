import { describe, expect, it } from "vitest";
import { loadTrackLessons, loadExamInventory } from "../../src/loader.js";
import {
  EXAM_CONTENT_DIMENSIONS,
  measureExamCoverage,
  formatExamCoverage,
  isExamInventoryComplete,
  trackIntroducedAtoms,
} from "../../src/exam-inventory.js";

describe("the committed Russian A1 inventory", () => {
  const inventory = loadExamInventory("russian", "A1");
  const spanish = loadExamInventory("spanish", "A1");

  it("keeps every point's probe key, and never an empty probe", () => {
    for (const point of inventory.points) {
      expect(point, `${point.id} has no probe key`).toHaveProperty("probe");
      expect(Array.isArray(point.probe) ? point.probe.length : 1, point.id).toBeGreaterThan(0);
    }
  });

  it("probes only atoms that EXIST, so a guessed id cannot under-report", () => {
    // The rule HL-C290 calls out by name, and the one two earlier agents broke.
    // A probe pointing at an id somebody expects a future lesson to introduce
    // resolves to "not introduced" forever and sits in the report
    // indistinguishable from the honest gaps around it.
    const lessons = loadTrackLessons("russian");
    const taught = trackIntroducedAtoms(lessons, "russian");
    const unknown: string[] = [];
    for (const point of inventory.points) {
      for (const atom of point.probe ?? []) if (!taught.has(atom)) unknown.push(`${point.id}:${atom}`);
    }
    expect(unknown).toEqual([]);
  }, 60_000);

  it("keeps the derivation total in both directions", () => {
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

  it("transfers the WHOLE proxy, which is the finding for an alphabetic Indo-European track", () => {
    // Tamil and Kannada each dropped two points and Marathi three. Russian
    // drops NONE, including A1-O1-06 (superscript letters in abbreviations),
    // which reads as untransferable Spanish typography and is really the demand
    // that a numeral carry a written grammatical ending — Russian's hyphenated
    // `1-y`. Restating a point around the target language's machinery IS
    // deriving it, so an empty notTransferred here is a claim, not an omission.
    const proxy = (inventory as unknown as {
      proxy: { notTransferred: unknown[]; note: string };
    }).proxy;
    expect(proxy.notTransferred).toEqual([]);
    expect(proxy.note).toMatch(/EMPTY ON PURPOSE/);
    expect(proxy.note).toMatch(/A1-O1-06/);
  });

  it("marks its own points as its own, in both directions", () => {
    for (const point of inventory.points) {
      const cast = point as unknown as { derivedFrom: string[]; russianSpecific?: boolean };
      expect(cast.russianSpecific === true, point.id).toBe(cast.derivedFrom.length === 0);
    }
    const specific = inventory.points.filter(
      (point) => (point as unknown as { derivedFrom: string[] }).derivedFrom.length === 0,
    );
    // Case, aspect, the two soundless letters, the cursive hand, vowel
    // reduction, the hard/soft contrast, the yery vowel, the impersonal plural,
    // the ty/vy choice as a social act, and the joining column measured as a
    // column. None of these has a Spanish point to derive from.
    expect(specific.length).toBeGreaterThanOrEqual(20);
  });

  it("refuses to borrow an authority it does not have", () => {
    // Russian is the first track in this series with a REAL exam and a
    // PUBLISHED CEFR mapping, which makes the borrowing risk higher here than
    // anywhere before it: a reader could take this file for a TORFL syllabus.
    expect(inventory.about).toMatch(/NOT A TRANSCRIPTION OF THAT EXAM'S SYLLABUS/);
    expect(inventory.about).toMatch(/MAY BE ATTRIBUTED TO THE TORFL SYSTEM/);
    expect(inventory.about).toMatch(/MAY BE ATTRIBUTED TO DELE/);
    expect(inventory.about).toMatch(/NO SEARCH WAS RUN, BY INSTRUCTION/);
    // The caveat that is not there, and what was done instead.
    expect(inventory.about).toMatch(/NO CAVEAT/);
    expect(inventory.about).toMatch(/THE CASE COLUMN WAS OPENED BY MEASUREMENT INSTEAD/);
    expect(inventory.source).toMatch(/^PROJECT-DEFINED\./);
    // Russian, unlike Tamil and Sanskrit, HAS an assessment contract — and its
    // A1 half dangles. Saying "partial and dangling" is what stops the contract
    // from reading as evidence it is not.
    expect(inventory.source).toMatch(/EXAM ENVELOPE: PARTIAL AND DANGLING/);
    expect(isExamInventoryComplete(inventory)).toBe(false);
    for (const dimension of EXAM_CONTENT_DIMENSIONS) {
      expect(inventory.scope[dimension].status, dimension).toBe("partial");
    }
  });

  it("names an anchor for every point, and says what kind of anchor it is", () => {
    const anchors = (inventory as unknown as {
      anchors: { id: string; kind: string; title: string; note: string }[];
    }).anchors;
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

  it("reports a CASE-shaped and JOINING-shaped gap, with the letters nearly closed", () => {
    // Pinned so a future tranche has to say which points it moved. It may rise;
    // a fall means coverage was lost and wants explaining.
    const lessons = loadTrackLessons("russian");
    const coverage = measureExamCoverage(inventory, lessons);
    expect(coverage.enumerated).toBe(228);
    // 104 -> 107: HL-C350's numeral tranche (chapters 23-26) closes RU-A1-Q-01,
    // RU-A1-Q-02 and RU-A1-L-09.
    expect(coverage.covered).toBe(107);
    expect(coverage.unmapped).toBe(121);
    // Zero partials is a property of the "existing atoms only" rule, not a
    // coincidence: with no guessed ids, a point is either fully probed or null.
    expect(coverage.partial).toBe(0);
    // THE HEADLINE. Six cases gate every Russian noun, adjective, pronoun and
    // numeral, and the track teaches two contrasts: the object pronouns
    // (menya/vas) and the genitive after `do`. Everything else — accusative of
    // nouns, dative, prepositional, instrumental, and the whole plural — is
    // absent, and the plural is absent outright rather than late.
    expect(coverage.byCategory["Padezh - case"]!).toEqual({ enumerated: 10, covered: 3 });
    // The sixth empty joining column in the series, and the first outside
    // South Asia — CLOSED. Both halves are now full: five coordinators
    // (i, ili, ni ... ni, no/a, odin ... drugoy) and eight subordination
    // points (the bare infinitive, chto, kotoryy, potomu chto, chtoby, kogda,
    // li, and the column measured as a column).
    expect(coverage.byCategory["Sochinenie - joining two clauses"]!).toEqual({
      enumerated: 5,
      covered: 5,
    });
    expect(coverage.byCategory["Podchinenie - subordination"]!).toEqual({
      enumerated: 8,
      covered: 8,
    });
    // The repair column, which had no word for sorry at all. What is still
    // open is asking for somebody by name at a door, and asking somebody to be
    // quiet.
    expect(coverage.byCategory["Vesti razgovor - managing the conversation, and repairing it"]!)
      .toEqual({ enumerated: 7, covered: 5 });
    // Punctuation went 0/7 to 4/7: the full stop, the closing-only question
    // mark, the dash that stands in for the absent copula, and the comma rule
    // that could not be stated until the track owned a subordinator.
    expect(coverage.byCategory["Punktuatsiya - punctuation"]!).toEqual({ enumerated: 7, covered: 4 });
    // And the other end: 29 of the 33 Cyrillic letters have their own writing
    // lesson, which is the closest thing this track has to a finished column.
    // Unmoved by the joining tranche, and deliberately: every one of the
    // thirteen joining devices, both apologies, the whole question family and
    // all four punctuation marks were checked against the union of taught
    // glyphs before a lesson was designed, and not one of them needed a new
    // sign. The three untaught lower-case letters — shcha, the hard sign and
    // e-oborotnoe — do not occur in any of them.
    // Reading that number alone would say the script is done; the five
    // uncovered points here — the letter names, the four untaught letters, the
    // cursive hand, the lower-case rule, the spelling rule — are the
    // distinction.
    // 5 -> 6: the written ordinal, 1-y, is the one letters point that was a
    // numerals point seen from the other side. Its derivation was written to
    // defend a transfer from Spanish superscripts and now pays off: the demand
    // is a numeral carrying a written grammatical ending, and Russian answers it
    // with a hyphen instead of a superscript.
    expect(coverage.byCategory["Kirillitsa - the letters"]!).toEqual({ enumerated: 10, covered: 6 });
    // THE COLUMN THAT HELD EXACTLY ONE NUMBER. odin was taught as half of the
    // odin ... drugoy joining pattern rather than as a numeral, so the track
    // could ask skolko and understand no answer to it. HL-C350 closes the
    // cardinals and the ordinals at NO new Cyrillic letter, and leaves the two
    // that are not downstream of a numeral.
    expect(coverage.byCategory["Chislitelnye i kolichestvo - numerals and quantity"]!)
      .toEqual({ enumerated: 5, covered: 2 });
    // The case government the numerals impose is NOT claimed. dva/tri/chetyre
    // take the genitive singular and pyat upward the genitive plural, and this
    // track has taught the genitive in one place, after `do`. RU-C25-practice
    // tells the reader so in as many words, and the atom below is the record of
    // their having been told -- which is a different thing from teaching it.
    const cardinals = inventory.points.find((point) => point.id === "RU-A1-Q-01")!;
    expect(cardinals.probe).toContain("RU-GRAMMAR-TSAT-TENS-01");
    expect(cardinals.probe).not.toContain("RU-GRAMMAR-NUMERAL-CASE-01");
    expect(cardinals.note).toMatch(/Counting THINGS is a genitive chapter/);
    expect(inventory.points.find((point) => point.id === "RU-A1-C-05")!.probe).toBeNull();
    // Nothing in the corpus can be described, because no adjective is taught.
    expect(coverage.byCategory["Prilagatelnoe - the adjective"]!.covered).toBe(0);
    expect(formatExamCoverage(coverage)).toContain(
      "russian A1 (partial inventory): 107/228 points covered (47%)",
    );
  }, 60_000);
});

// ---------------------------------------------------------------------------
// The first inventory for a track with THREE writing systems, and the first
// whose exam anchor cannot score half the construct it is anchoring.
//
// Two things forced this file's shape, and both are asserted here.
//
//   1. THE SCRIPT COLUMN HAD TO BE THREE COLUMNS. This corpus stands in three
//      completely different places: 31 of 46 hiragana signs taught, 2 of 46
//      katakana, and 3 kanji. A single "script" column averages those into a
//      number that describes nothing and hides the fact that a reader who can
//      decode a hiragana sentence still cannot read a menu, a name or a sign.
//      The mixed-script fact — which is what a single column would most
//      obviously lose — is its own point at JA-A1-HYO-01.
//
//   2. THE CAVEAT SAYS THE EXAM SCORES NO PRODUCTION AND NO INTERACTION.
//      `exam-levels.json` records that the published CEFR indication covers
//      "only the language knowledge, reading, and listening competence JLPT
//      tests". An inventory that quietly reported reading and listening
//      coverage would flatter this track exactly the way HL20 §1 warns about.
//      So the four production tasks named in `japanese/assessment-spec.md#a1`
//      are enumerated as points one for one, plus interaction. One of five is
//      covered — and it is interaction, which is the half JLPT cannot see.
//
// The result worth reporting is not the percentage. Japanese is one of only two
// tracks in the whole corpus with ZERO findings in every gentle-ramp queue, and
// its REPAIR column reads 7 of 8: it is the only track measured so far that
// holds the complete CEFR A1 repair kit — apologise, report the failure, ask
// for a repeat, ask for slower speech, point, and confirm the repair worked.
// Russian has half of it. Chinese has one move. Gujarati had none.
//
// And its joining column is still 0 of 8. THAT is the finding: the
// best-constructed track in the corpus has the same empty column as the worst,
// which is the clearest evidence available that the hole is structural rather
// than a symptom of neglect.
// ---------------------------------------------------------------------------
