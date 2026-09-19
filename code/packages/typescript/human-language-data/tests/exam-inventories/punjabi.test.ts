import { describe, expect, it } from "vitest";
import { loadTrackLessons, loadExamInventory } from "../../src/loader.js";
import {
  EXAM_CONTENT_DIMENSIONS,
  measureExamCoverage,
  formatExamCoverage,
  isExamInventoryComplete,
  trackIntroducedAtoms,
} from "../../src/exam-inventory.js";

describe("the committed Punjabi A1 inventory", () => {
  const inventory = loadExamInventory("punjabi", "A1");
  const spanish = loadExamInventory("spanish", "A1");

  it("keeps every point's probe key, and never an empty probe", () => {
    for (const point of inventory.points) {
      expect(point, `${point.id} has no probe key`).toHaveProperty("probe");
      expect(Array.isArray(point.probe) ? point.probe.length : 1, point.id).toBeGreaterThan(0);
    }
  });

  it("probes only atoms that EXIST, so a guessed id cannot under-report", () => {
    const lessons = loadTrackLessons("punjabi");
    const taught = trackIntroducedAtoms(lessons, "punjabi");
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

  it("marks its own points as its own, in both directions", () => {
    for (const point of inventory.points) {
      const cast = point as unknown as { derivedFrom: string[]; punjabiSpecific?: boolean };
      expect(cast.punjabiSpecific === true, point.id).toBe(cast.derivedFrom.length === 0);
    }
    const specific = inventory.points.filter(
      (point) => (point as unknown as { derivedFrom: string[] }).derivedFrom.length === 0,
    );
    expect(specific.map((point) => point.id)).toEqual([]);
  });

  it("names the A1 paper it measures against, and says the mocks are missing", () => {
    // Punjabi is the FIRST proxy-derived inventory that can point at a checked-in
    // task shape, so its `about` must not copy the "EXAM ENVELOPE: NONE EXISTS"
    // sentence the Malayalam and Kannada files carry. It says the opposite, and
    // the anchor that lets a point cite a paper part has to exist.
    expect(inventory.about).toMatch(/PROJECT-DEFINED EDITORIAL EQUIVALENT, NOT AN EXTERNAL SYLLABUS/);
    expect(inventory.about).toMatch(/EXAM ENVELOPE: AN A1 TASK SHAPE EXISTS, AND THE MOCKS DO NOT/);
    expect(inventory.about).not.toMatch(/EXAM ENVELOPE: NONE EXISTS/);
    expect(inventory.about).toMatch(/punjabi\/mocks\/ DOES NOT EXIST/);
    expect(inventory.about).toMatch(/NOT SEARCHED, BY INSTRUCTION/);
    expect(inventory.source).toMatch(/^PROJECT-DEFINED\./);
    const anchors = (inventory as unknown as { anchors: { id: string }[] }).anchors;
    expect(anchors.map((anchor) => anchor.id)).toContain("PA-TASK-SHAPES");
    expect(isExamInventoryComplete(inventory)).toBe(false);
    for (const dimension of EXAM_CONTENT_DIMENSIONS) {
      expect(inventory.scope[dimension].status, dimension).toBe("partial");
    }
  });

  it("claims a tone column and a BINARY honorific, both measured here", () => {
    // Tone is Punjabi's own: no Spanish point comes near it, and the corpus
    // teaches four tone atoms plus the letter that writes tone without being
    // said. The honorific is deliberately two-way — `aap` appears once in 226
    // lessons and only as the HINDI word, so no three-way system is claimed.
    expect(inventory.about).toMatch(/TWO COLUMNS ARE PUNJABI'S OWN, AND BOTH WERE MEASURED HERE/);
    expect(inventory.about).toMatch(/BINARY: tu against tusi/);
    const tone = inventory.points.filter((point) => point.category.startsWith("Sur ("));
    expect(tone).toHaveLength(7);
    const register = inventory.points.filter((point) => point.category.startsWith("Bolchaal"));
    expect(register).toHaveLength(5);
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

  it("reports a joining column that is no longer empty, and a script closed over the corpus only", () => {
    const lessons = loadTrackLessons("punjabi");
    const coverage = measureExamCoverage(inventory, lessons);
    expect(coverage.enumerated).toBe(227);
    // 112 -> 136. Chapters 37-43 answer this file's own uncovered list.
    // 136 -> 137: the ordinal tranche closes PA-A1-NUM-05 (chapter 44). One
    // point for six lessons, and no more than one: ordinals unlock nothing else
    // here, because the count still stops at panj -- PA-A1-NUM-02 and -03 are
    // both open -- so nothing above fifth can be said at all.
    expect(coverage.covered).toBe(137);
    expect(coverage.unmapped).toBe(90);
    expect(coverage.partial).toBe(0);
    // THE HEADLINE, and it is the starkest of the three tracks measured in this
    // series. ZERO of eleven. Not one of `te`/`ate`, `jaan`, `par`/`lekin`,
    // `kyunki`, `je`, the complementiser `ki`, `jadon` or `jo` occurs anywhere in
    // 226 lessons, in Gurmukhi or in romanisation — every apparent hit is a
    // script-drill syllable or a substring. The longest structure the track
    // teaches is a four-slot single clause, so `a1-writing-reader-purpose-message`
    // in the checked-in A1 task shape asks for a message this corpus cannot
    // produce. Malayalam's column came back 2/11 on the same walk; Punjabi's is
    // empty, and the difference was measured rather than assumed.
    // THE HEADLINE HAS CHANGED, and this assertion records it. It read
    // `covered: 0` -- not one of te/ate, jaan, par/lekin, kyunki, je, the
    // complementiser ki, jadon or jo occurred anywhere in 226 lessons, so the
    // longest structure the track taught was a four-slot single clause and the
    // A1 writing paper asked for a message the corpus could not produce.
    //
    // The finding under the finding is why it was cheap: ELEVEN of the eleven
    // devices needed NO NEW SIGN. Every one is spelled in Gurmukhi the track
    // taught long ago. This was never a script debt -- nobody had written the
    // words down. The one new letter in seven chapters (tha) was bought for a
    // question word, not for a joining word.
    const joining = coverage.byCategory["Jorr (joining and subordination)"]!;
    expect(joining).toEqual({ enumerated: 11, covered: 10 });
    // Two columns went FULL, and neither was the target: the clause pattern
    // closed negation, and the par/lekin doublet closed the register rule the
    // file said one lesson would close.
    expect(coverage.byCategory["Nanh (negation)"]!).toEqual({ enumerated: 5, covered: 5 });
    expect(coverage.byCategory["Bolchaal (register: familiar and respectful, Sanskritic and Perso-Arabic)"]!)
      .toEqual({ enumerated: 5, covered: 5 });
    // Two demonstratives, neither taught — which is why nothing in the track can
    // be pointed at.
    expect(coverage.byCategory["Sanketak (demonstratives and deixis)"]!).toEqual({
      enumerated: 2,
      covered: 0,
    });
    // DO NOT READ THIS AS "the script is done". Closure over the CORPUS is
    // perfect — 50 of 50 characters used in headwords are taught — and closure
    // over the ALPHABET is not: seven akhar and six of the ten digits are never
    // taught. The one uncovered point in this column is exactly that distinction.
    expect(coverage.byCategory["Gurmukhi (script and orthography)"]!.covered).toBe(10);
    // The two columns that carry this track, and they are not the ones the
    // Dravidian tracks lead on.
    expect(coverage.byCategory["Faram (filling in a form)"]!).toEqual({ enumerated: 10, covered: 9 });
    expect(coverage.byCategory["Sur (tone and pronunciation)"]!.covered).toBe(6);
    // The ordinal point, named rather than left to the aggregate, so a nulled
    // probe or a fabricated id is caught here and not only by the total. Both
    // halves were falsified before this was kept.
    const ordinals = inventory.points.find((point) => point.id === "PA-A1-NUM-05");
    expect(ordinals?.probe).toEqual([
      // Second first, because duujaa still shows its doo; the set named beside it.
      "PA-LEX-DUJA-01",
      "PA-GRAMMAR-ORDINAL-SET-01",
      "PA-LEX-TIJA-01",
      "PA-LEX-CHAUTHA-01",
      // First arrives fourth: pahilaa keeps no letter of ikk.
      "PA-LEX-PAHILA-01",
      // And the seam, where an inherited word came to look like a sum.
      "PA-LEX-PANJVAN-01",
      "PA-GRAMMAR-ORDINAL-VAAN-01",
    ]);
    expect(
      coverage.points.find((point) => point.id === "PA-A1-NUM-05")?.missingAtoms,
    ).toEqual([]);
    // And the point it does NOT close, which is why the ratio is one for six:
    // the cardinals stop at five, so sixth upward cannot be said.
    expect(coverage.points.find((point) => point.id === "PA-A1-NUM-02")?.covered).toBe(false);
    expect(formatExamCoverage(coverage)).toContain(
      "punjabi A1 (partial inventory): 137/227 points covered (60%)",
    );
  }, 60_000);
});
