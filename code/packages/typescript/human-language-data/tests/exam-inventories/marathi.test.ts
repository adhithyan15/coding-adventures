import { createHash } from "node:crypto";
import { existsSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { defaultCurriculumRoot, loadTrackLessons, loadExamInventory } from "../../src/loader.js";
import {
  EXAM_CONTENT_DIMENSIONS,
  measureExamCoverage,
  formatExamCoverage,
  isExamInventoryComplete,
  trackIntroducedAtoms,
} from "../../src/exam-inventory.js";

describe("the committed Marathi A1 inventory", () => {
  const inventory = loadExamInventory("marathi", "A1");
  const spanish = loadExamInventory("spanish", "A1");

  it("reconstructs the exact pre-migration inventory from 301 direct point owners", () => {
    const core = join(defaultCurriculumRoot(), "core");
    const aggregate = join(core, "exam-inventory-marathi-a1.json");
    const owners = join(core, "exam-inventory-marathi-a1.d");
    const rendered = `${JSON.stringify(inventory, null, 2)}\n`;

    expect(existsSync(aggregate)).toBe(false);
    expect(readdirSync(owners)).toHaveLength(302);
    expect(inventory.points).toHaveLength(301);
    // 151,077 -> 151,041: point MR-A1-SN-04 no longer probes
    // MR-SCRIPT-PAACH-NONNASAL. Chapter 13's numbers lesson introduced four
    // atoms, one over the budget; its two visible spelling tells (don's final n
    // and paach's missing nasal) are now one atom, MR-SCRIPT-DON-FINAL-N, which
    // the point still probes.
    expect(Buffer.byteLength(rendered)).toBe(151_041);
    expect(createHash("sha256").update(rendered).digest("hex")).toBe(
      "35cd589328757ff3ba9406379d95331c361dd65a74d8cbdd5e61c422cda6f495",
    );
  });

  it("keeps every point's probe key", () => {
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
    // The same rule the French and German blocks pin, and it matters MORE here.
    // Marathi covers 29% of its own target, so most points are uncovered; if a
    // guessed id were allowed to sit in a probe it would be indistinguishable
    // from the 213 honest gaps around it, and it would never flip to covered even
    // after the lesson was written, because the suffix would not match. `null`
    // plus a note is the only honest way to say "nothing here yet".
    const lessons = loadTrackLessons("marathi");
    const taught = trackIntroducedAtoms(lessons, "marathi");
    const unknown: string[] = [];
    for (const point of inventory.points) {
      for (const atom of point.probe ?? []) if (!taught.has(atom)) unknown.push(`${point.id}:${atom}`);
    }
    expect(unknown).toEqual([]);
  }, 60_000);

  it("never lets an unmapped point read as 'nobody has looked yet'", () => {
    for (const point of inventory.points) {
      if (point.probe !== null) continue;
      expect(point.note?.trim(), `${point.id} is unmapped and must say why`).toBeTruthy();
    }
  }, 60_000);

  // The ordinal point, named rather than left to the aggregate. The 162/301
  // total above would move if this probe were nulled, but it would move for a
  // hundred other reasons too; this pins WHICH eleven atoms close the point and
  // that every one of them is really taught. Both halves were falsified before
  // this was kept: adding a fabricated id fails the "probes only atoms that
  // EXIST" test above, and nulling the probe fails the coverage total.
  it("closes MR-A1-QU-04 on eleven atoms, ten words and one rule", () => {
    const lessons = loadTrackLessons("marathi");
    const taught = trackIntroducedAtoms(lessons, "marathi");
    const ordinals = inventory.points.find((point) => point.id === "MR-A1-QU-04");
    expect(ordinals?.probe).toEqual([
      // The set itself, taught on dusraa without a new word.
      "MR-GRAMMAR-ORDINAL-SET",
      // The four Marathi INHERITED. dusraa is absent on purpose: chapter 31
      // already taught MR-LEX-DUSRA, and the tranche re-opens it rather than
      // teaching it again, so claiming a new lexical atom for it would be a
      // fabrication.
      "MR-LEX-PAHILA",
      "MR-LEX-TISRA",
      "MR-LEX-CHAUTHA",
      // The seam, and the rule that starts at it.
      "MR-LEX-PACHVA",
      "MR-GRAMMAR-ORDINAL-VA",
      // The five it BUILDS.
      "MR-LEX-SAHAVA",
      "MR-LEX-SATVA",
      "MR-LEX-AATHVA",
      "MR-LEX-NAVVA",
      "MR-LEX-DAHAVA",
    ]);
    for (const atom of ordinals?.probe ?? []) expect(taught.has(atom), atom).toBe(true);
    // And the word the tranche does NOT re-teach is taught all the same, in the
    // chapter the tranche points back to.
    expect(taught.has("MR-LEX-DUSRA")).toBe(true);
  }, 60_000);

  it("derives from the Spanish set TOTALLY, so nothing is dropped by accident", () => {
    // The property that makes a proxy auditable rather than a gesture. Every one
    // of Spanish's 273 points must be either (a) named by some Marathi point's
    // `derivedFrom`, or (b) listed in `proxy.notTransferred` with a reason. A
    // point that is silently absent is indistinguishable from one nobody thought
    // of — which is the failure mode the whole file exists to prevent — and
    // writing this assertion is what caught `A1-O1-06` going missing.
    const proxy = (inventory as unknown as {
      proxy: { notTransferred: { spanishPoints: string[]; why: string }[] };
    }).proxy;
    const dropped = new Set(proxy.notTransferred.flatMap((entry) => entry.spanishPoints));
    for (const entry of proxy.notTransferred) expect(entry.why.trim().length).toBeGreaterThan(0);
    const derived = new Set<string>();
    for (const point of inventory.points) {
      for (const id of (point as unknown as { derivedFrom: string[] }).derivedFrom) derived.add(id);
    }
    const known = new Set(spanish.points.map((point) => point.id));
    // No dangling references in the other direction either: a `derivedFrom`
    // naming a Spanish point that does not exist is a typo that would quietly
    // shrink the audit.
    for (const id of derived) expect(known.has(id), `derivedFrom cites unknown Spanish point ${id}`).toBe(true);
    const unaccounted = [...known].filter((id) => !derived.has(id) && !dropped.has(id));
    expect(unaccounted).toEqual([]);
    // Dropping and deriving the same point would let a reader believe both.
    expect([...derived].filter((id) => dropped.has(id))).toEqual([]);
  }, 60_000);

  it("marks a point with no Spanish source as Marathi-specific, and means it", () => {
    // Devanagari orthography, the postpositions, the ergative and the
    // gender-marked present have no Spanish counterpart. Those points are honest
    // additions; what they must never be is padding that hides behind an empty
    // field. `marathiSpecific` has to agree with `derivedFrom` in both
    // directions, and such a point must still name a non-editorial anchor or an
    // explicit note.
    for (const point of inventory.points) {
      const cast = point as unknown as { derivedFrom: string[]; marathiSpecific?: boolean };
      expect(cast.marathiSpecific === true, point.id).toBe(cast.derivedFrom.length === 0);
    }
    const specific = inventory.points.filter(
      (point) => (point as unknown as { derivedFrom: string[] }).derivedFrom.length === 0,
    );
    // A proxy is a scaffold, not a template: some points must be Marathi's own.
    expect(specific.length).toBeGreaterThanOrEqual(20);
  });

  it("refuses to borrow an authority it does not have", () => {
    // Two bodies could lend this file weight it has not earned: a real Marathi
    // examiner, and the awarding body behind the proxy. The file must disclaim
    // both, because "derived from the DELE A1 inventory" is one careless edit
    // away from reading as "DELE says this about Marathi".
    expect(inventory.about).toMatch(/PROJECT-DEFINED EDITORIAL EQUIVALENT, NOT AN EXTERNAL SYLLABUS/);
    expect(inventory.about).toMatch(
      /NOTHING IN THIS FILE MAY BE ATTRIBUTED TO THE MAHARASHTRA DIRECTORATE OF LANGUAGES/,
    );
    expect(inventory.about).toMatch(/MAY BE ATTRIBUTED TO DELE/);
    expect(inventory.about).toMatch(/THE SEARCH IS SETTLED, DO NOT REPEAT IT/);
    expect(inventory.source).toMatch(/^PROJECT-DEFINED\./);
    expect(inventory.source).toMatch(/no external Marathi A1 syllabus/i);
    // Marathi has no timed mocks, so the file must say what it used instead of
    // the artifact Hindi mined. An unstated substitution is an unauditable one.
    expect(inventory.source).toMatch(/NOTE ON METHOD/);
    expect(inventory.source).toMatch(/Marathi has no mocks/);
    // Partial in every dimension. Neither an editorial basis nor a sourced proxy
    // is a shortcut to `complete`; a proxy does not close a dimension.
    expect(isExamInventoryComplete(inventory)).toBe(false);
    for (const dimension of EXAM_CONTENT_DIMENSIONS) {
      expect(inventory.scope[dimension].status, dimension).toBe("partial");
    }
  });

  it("names an anchor for every point, and says what kind of anchor it is", () => {
    // `anchorIds` answers "which of these did you read, and which did you
    // decide?". Without the KIND, a project-owned file, a sourced proxy and the
    // Council of Europe read the same on the page — and the weakest points, the
    // purely editorial ones, are the ones a reader most needs flagged.
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

  it("separates a content gap from a gap in the MEASUREMENT", () => {
    // The finding that nearly got written up wrong, twice, in two shapes. A probe
    // reads DECLARED atoms:
    //
    //   SCHEMA-V1 — 26 of Marathi's 205 lessons, the whole of chapters 9 to 12,
    //   declare no atoms while teaching mii, tuu/tumhii, maazhaM, kaay, kasaa,
    //   "what's your name?", "how are you?", kaam karne and raahne.
    //
    //   EMPTY-INTRODUCES — worse, because it hides inside schema-v2 where
    //   everything LOOKS declared. Four v2 lessons carry `introduces: []` while
    //   teaching new material: MR-R22-request-verbs and MR-R23-wellbeing-verbs
    //   drill five polite imperatives and a future while declaring only the
    //   infinitive atoms they review, and MR-A1M17/18 teach the guided and
    //   independent 30-to-40-word message — the A1 writing paper's second task —
    //   while declaring nothing at all.
    //
    // The first draft recorded several of both kinds as "untaught", which would
    // have sent an author to rewrite chapter 9 and chapter 24. No corpus-internal
    // metric can see either class; only a target list asks the question that
    // exposes them. This pins the marker so a future edit cannot quietly collapse
    // the distinction back into an undifferentiated "not covered".
    // WHAT CHANGED, AND WHY THE ASSERTION IS NOW A DIFFERENT SHAPE.
    //
    // The count used to be pinned at 13-or-more, and a floor on a count of
    // KNOWN DEFECTS is a bad pin: it only ever rises, it rewards nobody for
    // clearing one, and it went stale in the flattering direction for the
    // AUTHOR rather than for the corpus. All 26 schema-v1 lessons were
    // migrated to v2 in the chapters 9-12 work, and for a whole tranche
    // afterwards this file still said the material was unmeasurable -- so
    // eight points read as content debt while the teaching sat in the book,
    // already done. That is the failure mode worth pinning against.
    //
    // The SCHEMA-V1 class is therefore asserted CLOSED, permanently: no point
    // note may carry that marker again, because there is no schema-v1 lesson
    // left in the track to carry it. EMPTY-INTRODUCES is the class that
    // survives, it hides inside v2, and at least one point must still name it
    // -- a zero there would mean somebody deleted the distinction rather than
    // fixed it.
    const marked = inventory.points.filter((point) => (point.note ?? "").includes("MEASUREMENT GAP"));
    expect(marked.length).toBeGreaterThanOrEqual(1);
    for (const point of marked) expect(point.probe, point.id).toBeNull();
    const staleClass = inventory.points.filter((point) =>
      (point.note ?? "").includes("SCHEMA-V1 MEASUREMENT GAP"),
    );
    expect(staleClass.map((point) => point.id)).toEqual([]);
    expect(marked.some((point) => (point.note ?? "").includes("EMPTY-INTRODUCES"))).toBe(true);
    expect(inventory.probeSemantics).toMatch(/SCHEMA-V1 MEASUREMENT GAP/);
    expect(inventory.probeSemantics).toMatch(/EMPTY-INTRODUCES MEASUREMENT GAP/);
  });

  it("reports the gap as domain-shaped, which is the finding", () => {
    // Pinned so a future tranche has to say which points it moved. It may rise;
    // a fall means coverage was lost and wants explaining.
    //
    // The number moved 55/131 -> 88/301 when the point set was rebuilt from the
    // Spanish proxy rather than from CEFR descriptors, and BOTH halves of that
    // are the result. The numerator rose because the Spanish walk found taught
    // material an editorial list had never asked about — evaluative notions,
    // mental notions, the colon in a form label. The denominator nearly trebled
    // because it found twenty thematic domains nobody had enumerated: education,
    // work, leisure, media, housing, services, shopping, health, travel, money,
    // government, the arts, religion, the natural world. The corpus covers almost
    // none of them, and that is what an external boundary is FOR.
    //
    // The shape also differs from French and German, which are grammar-shaped
    // with vocabulary as their strongest column. Marathi's strongest columns are
    // its SCRIPT (15/24, after the previous tranche took closure 44 -> 0) and its
    // mental- and evaluative-notion verbs; the columns that carry an exam paper
    // are empty.
    //
    // 88 -> 111: the joining tranche (chapters 30-36). Coordination went 0/5 to
    // 5/5 and Subordination 1/7 to 5/7, and because a conjunction is a tool
    // rather than a topic, twelve further points fell in five other columns --
    // the negated sentence closed four function points, the polar particle three
    // more, and the two punctuation marks their own. Seventeen new words and
    // endings; twenty-three points. Coordination therefore leaves the
    // empty-category list below, which is the movement, and Demonstratives,
    // Temporal notions, Housing and Shopping stay in it, which is the remaining
    // work.
    // 111 -> 124: the asking-word tranche (chapters 37-40), and two different
    // kinds of movement inside one number, which is why they are reported
    // apart. SEVEN points were EARNED by the nine new items: the interrogative
    // pronouns and adverbs, the word-order rule behind them, asking about a
    // person or a place, the ithe/tithe pair, and the dental row, which closed
    // because one missing letter (थ) was the only thing keeping तिथे
    // unwritable. SIX more were already taught and only LOOKED uncovered,
    // because their notes still described chapters 9 to 12 as schema-v1 after
    // those chapters had been migrated -- the politeness contrast, asking a
    // name, asking how somebody is, asking for an evaluation, working
    // activity, and the imperative. Nothing was authored for those six; a
    // stale note was corrected. Work therefore leaves the empty-category list
    // -- on a note fix, not on a lesson, which is worth saying plainly.
    //
    // 157 -> 161: the numbers tranche (chapters 55-59). Nineteen items for
    // four points is the WORST ratio in this file, and it was taken anyway,
    // because MR-A1-QU-03 -- cardinals to a hundred -- is the most blocking
    // single point left: age, value and price, personal data on a form and the
    // interview's opening question all wait on a number above five, and none
    // of them can move until the count does. Six to twenty is the half of it
    // that fits in one tranche; the tens are the other half. Quantifiers 2/6
    // to 4/6, Shopping and "Money and the economy" off the empty-category
    // list, Devanagari letters and signs 15/24 to 17/24.
    //
    // 151 -> 157: the degree-and-amount tranche (chapters 53-54), which
    // teaches no adjective at all and multiplies the eight the previous one
    // taught. Seven items, six points, and the ratio is the point: khuup and
    // jaraa alone close three (ADJ-06, AP-01, ADV-03), because every one of
    // those points was blocked behind SPINE-DESCRIBE-QUALITIES rather than
    // behind its own vocabulary. The adjective 5/7 to 6/7, "The adjective
    // phrase" off the empty-category list at 1/1, Adverbs 6/8 to 7/8.
    //
    // 142 -> 151, which is exactly half. The adjective tranche (chapters
    // 49-52) closes SPINE-DESCRIBE-QUALITIES, an A1 CORE node this track had
    // never realized: forty-eight chapters could name a house, put a room in
    // it and say who it belonged to, and could not say one thing about what
    // it was like. Twelve items, nine points. The adjective went 2/7 to 5/7,
    // Evaluative notions 4/8 to 7/8, "The person: the body" to 3/3 and "The
    // person: character" off the empty-category list.
    //
    // 133 -> 142: the accompaniment tranche (chapters 45-48). Eleven items --
    // three "with" endings that English collapses into one, the pronoun's own
    // oblique, two ablatives and their question word, the animate object
    // marker in its two remaining jobs, and -kade, which is how a language
    // with no verb for HAVE says somebody has something. Case and
    // postpositions went 3/6 to 5/6, Spatial notions 4/7 to 6/7, and The verb
    // phrase 3/6 to 5/6. Only paryant keeps the postposition column open, and
    // it is blocked on the same reph that MR-A1-OR-21 is: one script lesson
    // pair closes both.
    //
    // 124 -> 133: the place tranche (chapters 41-44). Eleven items -- two nouns,
    // the oblique stem, five postpositions and three ordinary place words --
    // closed nine points, and the ratio comes from the STEM rather than from the
    // vocabulary: MR-A1-N-09 is a single rule that four other points were
    // sitting behind. Spatial notions went 1/7 to 4/7, Case and postpositions
    // 1/6 to 3/6, Existential notions 0/5 to 2/5, and Housing leaves the
    // empty-category list on its first lesson. Demonstratives, Temporal notions
    // and Shopping stay in it, which is the remaining work.
    //
    // 161 -> 162: the ordinal tranche (chapters 60-61) closes MR-A1-QU-04, the
    // last vocabulary point in the Quantifiers column and the one HL-C350
    // named as the weakest column in the corpus. ONE point for twelve lessons
    // is a poor ratio and it is the honest one: ordinals unlock nothing else in
    // this inventory, because MR-A1-NG3-06 wants ordering EXPONENTS -- aadhii,
    // nantar, mag -- rather than more ordinals, and none of the three is taught
    // anywhere in the track. Its note now says so rather than saying
    // "Untaught".
    //
    // 162 -> 165: chapter 64 closes MR-A1-OR-05 (ddha), MR-A1-OR-07 (pha) and
    // MR-A1-PU-01 (the danda). THREE POINTS FOR FOUR LESSONS, and unlike the
    // ordinal tranche above the ratio is good because script points are single
    // shapes: the notes named exactly which letters were missing and every one
    // of them was genuinely missing when checked under the glyph-inventory rule.
    // With pha the script has five plain-and-aspirated pairs and no stop left
    // without a partner. The danda is the first mark in the track taught for
    // RECOGNITION rather than production -- the corpus punctuates with a full
    // stop, which is what Marathi does now.
    const lessons = loadTrackLessons("marathi");
    const coverage = measureExamCoverage(inventory, lessons);
    expect(coverage.enumerated).toBe(301);
    //
    // 166 -> 167: chapters 66 and 67 close MR-A1-NT-01, the clock and the parts
    // of the day. TEMPORAL NOTIONS IS NO LONGER ONE OF THE EMPTY COLUMNS BELOW,
    // and that is the point of the change rather than the single number: this
    // track's SPINE-TIME-OF-DAY ledger read segments: [] with all nine concepts
    // omitted, on a node the shared spine marks "core": true whose only
    // prerequisite is greeting somebody. Filed as HL-C394, where the measurement
    // is that TEN of twenty-three tracks omit all nine.
    // Chapter 66 gives the parts of the day and the -ii that places an event in
    // one; chapter 67 gives the clock and adds NO new numbers, because all twelve
    // cardinals and the how-many question word were already taught.
    // 165 -> 166: chapter 65 closes MR-A1-OR-12, the independent ii, o, ai and au.
    // ONE point for five lessons, and the ratio is honest: four letters that each
    // need their own shape practised, plus the review. All four had a sign the
    // reader already drew, so the gap was in the POSITION rather than the sound.
    // 167 -> 168: chapter 68 closes MR-A1-F5-03, choosing a greeting that fits
    // the time of day. That completes the SPINE-TIME-OF-DAY repair begun in 66:
    // the node now omits only GREETING-DAY, matching every track that realizes
    // it. Three greetings for ONE new word, taught for reading, with namaskaar
    // named as what is actually spoken at any hour.
    // 168 -> 171: chapter 69 closes ALL THREE MR-A1-DEM points at once, because
    // they describe one grid: the six forms, the two-way near/far, and the
    // prenominal position. DEMONSTRATIVES LEAVES THE EMPTY-COLUMN LIST BELOW,
    // and that is the movement. It was worked before the verb column because
    // to / tee / te are the third-person PRONOUNS too (HL-C395), so V-01's
    // "all persons" had no third person to conjugate for.
    expect(coverage.covered).toBe(171);
    expect(coverage.unmapped).toBe(130);
    // Zero partials is a property of the "existing atoms only" rule above, not a
    // coincidence: with no guessed ids, a point is either fully probed or null.
    expect(coverage.partial).toBe(0);
    for (const empty of ["Personal identity"]) {
      expect(coverage.byCategory[empty]?.covered, empty).toBe(0);
    }
    // "Temporal notions" was the third empty column and is not any more. It is
    // still the emptiest: five of its six points are open -- today/yesterday/
    // tomorrow, the days, the months and a date, the seasons, and ordering two
    // events.
    expect(coverage.byCategory["Temporal notions"]!).toEqual({ enumerated: 6, covered: 1 });
    expect(coverage.byCategory["Coordination"]!.covered).toBe(5);
    expect(coverage.byCategory["Devanagari letters and signs"]!.covered).toBeGreaterThan(0);
    expect(coverage.byCategory["Sound system"]!.covered).toBeGreaterThan(0);
    expect(formatExamCoverage(coverage)).toContain(
      "marathi A1 (partial inventory): 171/301 points covered (57%)",
    );
  }, 60_000);
});

// ---------------------------------------------------------------------------
// Tamil (HL-C290 again, and the first Dravidian track to get one).
//
// The method is Marathi's and these tests are deliberately its tests, because
// the value of a method is that the second use is cheaper than the first. Two
// things differ and both are properties of the LANGUAGE rather than of the
// derivation:
//
//   1. Tamil is DIGLOSSIC, and `core/exam-levels.json` says so in the caveat it
//      carries for this track and for no other Dravidian one. The written and
//      spoken registers diverge sharply and this curriculum teaches spoken
//      Tamil first, which is a fact about what an exam could even ask. So the
//      inventory has a register column that no proxy-derived file has had, and
//      its first point — that the corpus never tells the learner any of this —
//      is uncovered.
//   2. Tamil's SCRIPT column is nearly closed rather than nearly empty. All 18
//      core consonants and 10 of 12 independent vowels are taught, and walking
//      every Tamil character the track prints against the set its script
//      lessons teach returns zero shown-but-untaught. That is the opposite of
//      the Marathi result and it is why the shape assertions below name
//      different empty columns.
// ---------------------------------------------------------------------------
