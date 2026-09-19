import { describe, expect, it } from "vitest";
import { loadTrackLessons, loadExamInventory } from "../../src/loader.js";
import {
  EXAM_CONTENT_DIMENSIONS,
  measureExamCoverage,
  formatExamCoverage,
  isExamInventoryComplete,
  trackIntroducedAtoms,
} from "../../src/exam-inventory.js";

describe("the committed Chinese A1 inventory", () => {
  const inventory = loadExamInventory("chinese", "A1");
  const spanish = loadExamInventory("spanish", "A1");

  it("keeps every point's probe key, and never an empty probe", () => {
    for (const point of inventory.points) {
      expect(point, `${point.id} has no probe key`).toHaveProperty("probe");
      expect(Array.isArray(point.probe) ? point.probe.length : 1, point.id).toBeGreaterThan(0);
    }
  });

  it("probes only atoms that EXIST, so a guessed id cannot under-report", () => {
    const lessons = loadTrackLessons("chinese");
    const taught = trackIntroducedAtoms(lessons, "chinese");
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

  it("drops exactly one point, and says why that one and not the neighbours", () => {
    // The alphabet, the case distinction and the written accent all LOOK
    // untransferable to a logographic script and all three restate — onto the
    // stroke, onto pinyin, and onto the tone marks. Only A1-O1-06 returns
    // nothing on either side of the script question, and the reason has to name
    // both sides or it is a guess.
    const proxy = (inventory as unknown as {
      proxy: { notTransferred: { spanishPoints: string[]; why: string }[]; note: string };
    }).proxy;
    expect(proxy.notTransferred).toHaveLength(1);
    expect(proxy.notTransferred[0]!.spanishPoints).toEqual(["A1-O1-06"]);
    expect(proxy.notTransferred[0]!.why).toMatch(/unicameral/);
    expect(proxy.notTransferred[0]!.why).toMatch(/Beida/);
    // And it names the track that DERIVED the same point, which is what keeps
    // "nothing to superscript" from reading as "superscripts looked foreign".
    expect(proxy.notTransferred[0]!.why).toMatch(/RU-A1-L-09/);
    expect(proxy.note).toMatch(/A1-O1-01, the alphabet, becomes the STROKE/);
  });

  it("writes down the pinyin decision IN THE FILE, with its consequence", () => {
    // The instruction this file was written under: decide explicitly whether
    // pinyin coverage is a script claim or a pronunciation claim, and write
    // down which you chose. A decision recorded only in a commit message is a
    // decision the next author re-makes differently.
    const decision = inventory.points.find((point) => point.id === "ZH-A1-PY-06");
    expect(decision, "the pinyin decision point must exist").toBeDefined();
    expect(decision!.probe, "the decision records a choice, not a lesson").toBeNull();
    expect(decision!.note).toMatch(/THE DECISION: pronunciation/);
    expect(decision!.note).toMatch(/script-closure\.ts/);
    expect(decision!.note).toMatch(/ZH-ORTHO/);
    expect(decision!.note).toMatch(/THE CONSEQUENCE, STATED RATHER THAN HIDDEN/);
    expect(inventory.about).toMatch(/THIS FILE TREATS PINYIN AS PRONUNCIATION/);
    // The mechanical half of the decision: no pinyin point may be probed with a
    // SCRIPT atom, because that would file a pronunciation claim as script.
    const pinyin = inventory.points.filter((point) => point.category.startsWith("Pinyin"));
    expect(pinyin.length).toBeGreaterThanOrEqual(6);
    for (const point of pinyin) {
      for (const atom of point.probe ?? []) {
        expect(atom.startsWith("ZH-SCRIPT-"), `${point.id} probes a script atom`).toBe(false);
      }
    }
  });

  it("marks its own points as its own, in both directions", () => {
    for (const point of inventory.points) {
      const cast = point as unknown as { derivedFrom: string[]; chineseSpecific?: boolean };
      expect(cast.chineseSpecific === true, point.id).toBe(cast.derivedFrom.length === 0);
    }
    const specific = inventory.points.filter(
      (point) => (point as unknown as { derivedFrom: string[] }).derivedFrom.length === 0,
    );
    // Tone (5), the particles (2 of 4), measure words (2), aspect (2), the
    // phonetic component, the character census, simplified-against-traditional,
    // handwriting, dictionary lookup, topic-comment order, the compound, the
    // pinyin decision, three repair points and the two mediation points.
    expect(specific.length).toBeGreaterThanOrEqual(20);
  });

  it("refuses to borrow an authority it does not have", () => {
    expect(inventory.about).toMatch(/NOT A TRANSCRIPTION OF ANY HSK SYLLABUS/);
    expect(inventory.about).toMatch(/MAY BE ATTRIBUTED TO HANBAN/);
    expect(inventory.about).toMatch(/MAY BE ATTRIBUTED TO DELE/);
    expect(inventory.about).toMatch(/NO SEARCH WAS RUN, BY INSTRUCTION/);
    // The caveat that DID steer this file, and the column it produced.
    expect(inventory.about).toMatch(/TRANSLATION AND MEDIATION/);
    expect(inventory.about).toMatch(/THAT CAVEAT WAS READ BEFORE ANY POINT WAS WRITTEN/);
    // The number this file must never invent. An HSK character or word count
    // would be the easiest thing in the world to assert and the hardest to
    // defend, given that no search was run.
    expect(inventory.source).toMatch(/NO HSK WORD LIST OR CHARACTER LIST IS CITED ANYWHERE IN THIS FILE/);
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

  it("reports the tranche that broke the particle gap and the joining zero", () => {
    // Pinned so a future tranche has to say which points it moved. It may rise;
    // a fall means coverage was lost and wants explaining.
    const lessons = loadTrackLessons("chinese");
    const coverage = measureExamCoverage(inventory, lessons);
    expect(coverage.enumerated).toBe(191);
    // 70 -> 73: HL-C350's numeral tranche (chapters 20-21) closed ZH-A1-NUM-01,
    // ZH-A1-NUM-02 and ZH-A1-AB-02.
    // 73 -> 98: the grammar tranche of chapters 22-28. TEN CHARACTERS, and five
    // of the twenty-five points cost no character at all.
    expect(coverage.covered).toBe(98);
    expect(coverage.unmapped).toBe(93);
    expect(coverage.partial).toBe(0);

    // THE HEADLINE WHEN THIS FILE WAS WRITTEN, and it was not a vocabulary gap.
    // Mandarin carries almost all of its grammar in a handful of toneless
    // particles, and NOT ONE was taught: de, le, ma, ne, ba, guo and zhe each
    // returned zero occurrences across the 175 lesson files then in the track.
    //
    // 0/4 -> 1/4 -> 3/4. 吗 arrived with the asking chapter; 的 and 了 and 呢
    // arrive here. Four of the seven particles are now taught, and the two that
    // carry the most grammar between them — 的 and 了 — cost eight strokes and
    // two. PART-04 stays open because a CLASS point is not answered by four of
    // seven, and its note now names guo and zhe as what is left.
    expect(coverage.byCategory["Zhuci - the particles"]!).toEqual({ enumerated: 4, covered: 3 });

    // THE JOINING COLUMN COMES OFF ZERO. It had been flat in seven tracks
    // running, and J-08's note had already done the work of saying why and
    // which word was cheapest: every one of the nine joining words needs a
    // character the track does not teach, a character here costs a writing
    // lesson and a reading lesson and a source-verified stroke record and a
    // regeneration of the subset font, and `he` is the cheapest of the nine.
    // This tranche spent that budget. 和 is eight strokes, three of which are
    // 口, and it is a phono-semantic compound whose sounding half IS hé.
    expect(coverage.byCategory["Lianjie - joining two clauses"]!).toEqual({ enumerated: 8, covered: 1 });

    // FOUR COLUMNS CLOSE OUTRIGHT, and two of them were columns of one that no
    // lesson had ever said out loud: Mandarin has no article, and possession is
    // one particle between owner and owned.
    expect(coverage.byCategory["Guanci - the article"]!).toEqual({ enumerated: 1, covered: 1 });
    expect(coverage.byCategory["Lingshu - possession"]!).toEqual({ enumerated: 1, covered: 1 });
    expect(coverage.byCategory["Dongci - the verb, which does not inflect"]!).toEqual({
      enumerated: 7,
      covered: 7,
    });
    expect(coverage.byCategory["Ti - aspect, which is what Mandarin has instead of tense"]!).toEqual({
      enumerated: 2,
      covered: 2,
    });
    expect(coverage.byCategory["Danju - the simple sentence"]!).toEqual({ enumerated: 4, covered: 4 });

    // The column the proxy has NO point for anywhere, and the corpus's best
    // work: tone is lexical, the five contours, third-tone sandhi taught on the
    // first word in the book, and bu sandhi on the commonest bu there is. What
    // is missing is tone across a phrase, and this tranche did not touch it.
    expect(coverage.byCategory["Shengdiao - tone, for which the proxy has no column at all"]!).toEqual({
      enumerated: 5,
      covered: 4,
    });
    // The fifth skill this track's own alignment names, measured and still
    // empty: no lesson in the track declares `mediation` in `modes`.
    expect(coverage.byCategory["Fanyi - translation and mediation"]!).toEqual({ enumerated: 2, covered: 0 });
    // UNTOUCHED, AND DELIBERATELY. Food and drink is a Spanish point and not one
    // Mandarin word, in a language whose learners eat on day one; punctuation is
    // seven points and not one mark taught in 244 lessons — which is why this
    // tranche writes no 。 and no ，either, and prints a dash at a seam where
    // Chinese writes a comma rather than smuggling one in.
    expect(coverage.byCategory["Yinshi - food and drink"]!).toEqual({ enumerated: 1, covered: 0 });
    expect(coverage.byCategory["Biaodian - punctuation"]!).toEqual({ enumerated: 7, covered: 0 });
    // The numeral work of the previous tranche, unchanged by this one — and the
    // assertions that say so are kept rather than replaced by the sentence,
    // because "unchanged" is a claim a test should hold rather than a comment.
    expect(coverage.byCategory["Shuci - numerals and quantity"]!)
      .toEqual({ enumerated: 5, covered: 2 });
    expect(coverage.byCategory["Suoxie - abbreviations and symbols"]!)
      .toEqual({ enumerated: 2, covered: 1 });
    // Education is still the strongest specific-notion field measured in any
    // track in this series — three institutions and four kinds of student built
    // productively out of four characters.
    expect(coverage.byCategory["Jiaoyu - education"]!.covered).toBe(3);
    // The decimal structure stays probed as a RULE and two worked examples, not
    // as eighty-nine lexical atoms, because that is what Mandarin asks of a
    // learner: shi yi is ten-one and er shi is two-ten, and the order is the
    // whole grammar.
    const zhDecimal = inventory.points.find((point) => point.id === "ZH-A1-NUM-02")!;
    expect(zhDecimal.probe).toContain("ZH-GRAMMAR-DECIMAL-TEENS");
    expect(zhDecimal.probe).toContain("ZH-GRAMMAR-DECIMAL-TENS");
    expect(zhDecimal.probe).not.toContain("ZH-LEX-SHISAN");
    // The cardinals stay probed at BOTH doors, because this track splits a
    // character's sound from its hand and a probe naming only the LEX atom would
    // report a number the reader cannot write.
    const zhCardinals = inventory.points.find((point) => point.id === "ZH-A1-NUM-01")!;
    for (const digit of ["LIU", "QI", "BA", "JIU", "SHI"]) {
      expect(zhCardinals.probe, digit).toContain(`ZH-LEX-NUM-${digit}`);
      expect(zhCardinals.probe, digit).toContain(`ZH-SCRIPT-NUM-${digit}`);
    }
    expect(formatExamCoverage(coverage)).toContain(
      "chinese A1 (partial inventory): 98/191 points covered (51%)",
    );
  }, 60_000);

  it("closes five points with NO NEW CHARACTER, which is what a character costs here", () => {
    // A character in this track is the expensive unit — a writing lesson, a
    // reading lesson, a source-verified stroke record in data/scripts/chinese.json
    // and a regeneration of the vendored subset font — so the points that need
    // none are worth naming as a set rather than leaving inside a total.
    //
    // Three of the five are things the reader must STOP doing, which is why no
    // lesson had ever said them: an absence leaves no word to teach. The fourth
    // is a join between two things the track already had and never let meet, and
    // the fifth is a word ORDER.
    const free: Record<string, string> = {
      "ZH-A1-V-02": "ZH-GRAMMAR-VERB-INVARIANT-01",
      "ZH-A1-NP-03": "ZH-GRAMMAR-NO-AGREEMENT-01",
      "ZH-A1-VP-03": "ZH-GRAMMAR-NO-COMPLEMENT-AGREEMENT-01",
      "ZH-A1-ART-01": "ZH-GRAMMAR-NO-ARTICLE-01",
      "ZH-A1-ADJ-02": "ZH-LEX-ZHONGGUOREN-01",
      "ZH-A1-S-04": "ZH-GRAMMAR-TOPIC-COMMENT-01",
    };
    const lessons = loadTrackLessons("chinese");
    const taught = trackIntroducedAtoms(lessons, "chinese");
    for (const [pointId, atom] of Object.entries(free)) {
      const point = inventory.points.find((candidate) => candidate.id === pointId)!;
      expect(point.probe, pointId).toContain(atom);
      expect(taught.has(atom), atom).toBe(true);
      // None of them is probed with a SCRIPT atom, because none of them needed a
      // character. That is the mechanical form of the claim.
      for (const probed of point.probe ?? []) {
        expect(probed.startsWith("ZH-SCRIPT-"), `${pointId} probes a script atom`).toBe(false);
      }
    }
  }, 60_000);

  it("records the numeral blocker that had already lifted before this tranche looked", () => {
    // ZH-A1-NG5-04's note read "blocked on the numerals", and the numeral
    // tranche of chapters 20-21 had already lifted that block. Only 岁 was
    // missing. Pinned rather than merely fixed, because a note that decays into
    // agreement is indistinguishable from real debt and gets a lesson written
    // for a gap that is no longer there.
    const age = inventory.points.find((point) => point.id === "ZH-A1-NG5-04")!;
    expect(age.probe).toContain("ZH-LEX-SUI-01");
    expect(age.note).toMatch(/THE OLD NOTE'S BLOCKER IS GONE/);
    // And the point it unlocked in turn: name, nationality and age, where the
    // nationality half cost no character either.
    const personal = inventory.points.find((point) => point.id === "ZH-A1-F1-03")!;
    expect(personal.probe).toContain("ZH-LEX-ZHONGGUOREN-01");
    expect(personal.probe).toContain("ZH-GRAMMAR-AGE-NO-VERB-01");
  });

});
