// ---------------------------------------------------------------------------
// The cross-track cousin join (HL-C88, HL10 §6.7).
//
// The panel this feeds claims "reflexes of the same etymon", so the tests that
// matter are the ones about what it REFUSES to pair, not what it pairs.
// ---------------------------------------------------------------------------
import { describe, expect, it } from "vitest";
import { ROMANCE_COUSINS, buildCousinIndex, cousinsFor } from "../src/cousins.js";
import { loadEverything } from "../src/loader.js";
import { parseLesson } from "../src/parse.js";

function lesson(options: {
  id: string;
  headword: string;
  roots: string;
  sequence?: number;
  language: string;
}) {
  const source = `---
schema_version: 2
id: ${options.id}
spine_node: HELLO
sequence: ${options.sequence ?? 10}
chapter: 1
type: word
headword: ${options.headword}
gloss: a test
concept_tag: TEST-TAG
prerequisites: []
roots: ${options.roots}
duration:
  max_seconds: 120
requires:
  knowledge: []
introduces:
  knowledge: []
practises:
  knowledge: []
skills: [reading]
modes: [interpretive]
strands: [meaning-input]
register: neutral
variety: general
---

# ${options.headword}

## Warm-up

[PAUSE 2s] Recall it.
`;
  return parseLesson(source, options.language);
}

describe("cross-track cousins", () => {
  const corpus = [
    lesson({ id: "ES-1", headword: "la hora", roots: "[hora-latin]", language: "spanish" }),
    lesson({ id: "FR-1", headword: "heure", roots: "[hora-latin]", language: "french" }),
    lesson({ id: "IT-1", headword: "ora", roots: "[hora-latin]", language: "italian" }),
    lesson({ id: "PT-1", headword: "hora", roots: "[hora-latin]", language: "portuguese" }),
    lesson({ id: "LA-1", headword: "hōra", roots: "[hora-latin]", language: "latin" }),
    lesson({ id: "TA-1", headword: "நேரம்", roots: "[hora-latin]", language: "tamil" }),
  ];
  const index = buildCousinIndex(corpus);

  it("pairs the reflexes of one etymon across the Romance tracks", () => {
    expect(cousinsFor(index, corpus[0]!).map((cousin) => `${cousin.language}:${cousin.headword}`))
      .toEqual(["french:heure", "italian:ora", "portuguese:hora"]);
  });

  it("never lists the lesson's own language", () => {
    // Spanish lessons frequently share a root with each other, and a Spanish
    // word is not its own cousin.
    const withSibling = [
      ...corpus,
      lesson({ id: "ES-2", headword: "ahora", roots: "[hora-latin]", language: "spanish" }),
    ];
    const cousins = cousinsFor(buildCousinIndex(withSibling), withSibling[0]!);
    // The positive half is the control: without it this assertion passes just as
    // happily against an empty result, which is how an early draft that returned
    // nothing for every lesson still went green here.
    expect(cousins.map((cousin) => cousin.language)).toEqual(["french", "italian", "portuguese"]);
    expect(cousins.some((cousin) => cousin.language === "spanish")).toBe(false);
  });

  it("leaves Latin out, because an ancestor is not a sibling", () => {
    // The etymology block above the panel already names the Latin etymon.
    // Printing it again under "cousins" would misdescribe the relationship.
    expect(ROMANCE_COUSINS).not.toContain("latin");
    const cousins = cousinsFor(index, corpus[0]!);
    expect(cousins).toHaveLength(3);
    expect(cousins.some((cousin) => cousin.language === "latin")).toBe(false);
  });

  it("leaves out tracks that are not in the requested family", () => {
    const cousins = cousinsFor(index, corpus[0]!);
    expect(cousins).toHaveLength(3);
    expect(cousins.some((cousin) => cousin.language === "tamil")).toBe(false);
  });

  it("prints one word per language, the earliest by reading order", () => {
    // A root can be carried by several lessons in a track. The panel wants the
    // language's word, not every place it was mentioned — and picking the
    // earliest keeps the panel stable as later lessons are added.
    const many = [
      corpus[0]!,
      lesson({ id: "FR-late", headword: "horaire", roots: "[hora-latin]", sequence: 90, language: "french" }),
      lesson({ id: "FR-early", headword: "heure", roots: "[hora-latin]", sequence: 20, language: "french" }),
    ];
    const cousins = cousinsFor(buildCousinIndex(many), many[0]!);
    expect(cousins).toHaveLength(1);
    expect(cousins[0]).toMatchObject({ language: "french", headword: "heure", lessonId: "FR-early" });
  });

  it("orders languages fixedly, not by corpus order", () => {
    // Generated output that reorders itself churns every book hash for nothing.
    const shuffled = [corpus[0]!, corpus[3]!, corpus[1]!, corpus[2]!];
    expect(cousinsFor(buildCousinIndex(shuffled), corpus[0]!).map((c) => c.language)).toEqual([
      "french",
      "italian",
      "portuguese",
    ]);
  });

  it("gives the same answer whatever order the corpus arrives in", () => {
    // The module promises stable output, and the first draft did not deliver it:
    // most non-Spanish lessons carry no `sequence:`, so candidates tied, and the
    // tie fell to whichever the corpus yielded first -- that is, to readdirSync
    // order. Reversing the corpus changed the printed cousin for 35 real
    // lessons. A shuffle test is the cheapest thing that would have caught it.
    const unsequenced = [
      lesson({ id: "ES-T", headword: "bueno", roots: "[bonus]", language: "spanish" }),
      lesson({ id: "IT-late", headword: "buongiorno", roots: "[bonus]", language: "italian" }),
      lesson({ id: "IT-early", headword: "buono", roots: "[bonus]", language: "italian" }),
    ];
    const forward = cousinsFor(buildCousinIndex(unsequenced), unsequenced[0]!);
    const reversed = cousinsFor(buildCousinIndex([...unsequenced].reverse()), unsequenced[0]!);
    expect(forward).toEqual(reversed);
    expect(forward).toHaveLength(1);
  });

  it("returns nothing for a lesson with no roots, rather than guessing", () => {
    const rootless = lesson({ id: "ES-9", headword: "hola", roots: "[]", language: "spanish" });
    const withRootless = buildCousinIndex([...corpus, rootless]);
    expect(cousinsFor(withRootless, rootless)).toEqual([]);
    // Control: the same index still finds cousins for a lesson that HAS roots,
    // so the empty result above is about this lesson and not about the index.
    expect(cousinsFor(withRootless, corpus[0]!)).toHaveLength(3);
  });

  it("carries the shared root, so a panel can say why the words are paired", () => {
    const cousins = cousinsFor(index, corpus[0]!);
    expect(cousins).toHaveLength(3);
    expect(cousins.every((cousin) => cousin.root === "hora-latin")).toBe(true);
  });
});

describe("the real corpus", () => {
  it("finds cousins for a useful number of Spanish lessons", () => {
    // Measured before the generator was written, because a panel with nothing
    // to show is not worth a rendering change. This is the honest ceiling the
    // feature is designed against, and it moves only when a track gains a
    // `roots:` slug another Romance track already carries.
    const { lessons } = loadEverything();
    const index = buildCousinIndex(lessons);
    const spanish = lessons.filter((entry) => entry.language === "spanish");
    const withCousins = spanish.filter((entry) => cousinsFor(index, entry).length > 0);
    expect(withCousins.length).toBe(80); // HL-C178: +5 -- chapter 274, C2 opens

    // ...but 76 is the join's reach, NOT the number of panels worth printing.
    // A lesson's headword is often a phrase that merely CONTAINS the relative:
    //
    //     bien  ->  italian buongiorno · portuguese bom dia
    //
    // which reads as a claim that `bien` and `buongiorno` are the same word.
    // Restricting both sides to single-token headwords cuts the reach to 25 and
    // makes those 25 genuinely good -- dia/giorno/dia, estar/stare,
    // trabajar/travailler/trabalhar -- but it does not fix `bien`, because
    // `buongiorno` is one token and the shared root slug is simply coarse.
    //
    // So the display rule is an open decision, deliberately NOT baked in here:
    // this module answers "which lessons share an etymon", and what is worth
    // putting on a page is a separate question with a real quality/coverage
    // trade-off. Both numbers are pinned so the trade-off stays visible.
    const singleToken = lessons.filter((entry) => !/[\s·/,]/.test(entry.realization.headword.trim()));
    const strictIndex = buildCousinIndex(singleToken);
    const strict = singleToken
      .filter((entry) => entry.language === "spanish")
      .filter((entry) => cousinsFor(strictIndex, entry).length > 0);
    expect(strict.length).toBe(27);

    // And the join really is etymological: every pairing names a root slug that
    // BOTH lessons declare.
    //
    // The obvious version of this check -- asserting the root against the
    // QUERYING lesson's roots -- is tautological, because `cousin.root` is
    // assigned from exactly that list inside `cousinsFor`. It can never fail.
    // The claim worth testing is that the far side declares it too.
    const byId = new Map(lessons.map((entry) => [entry.realization.lessonId, entry]));
    for (const entry of withCousins) {
      for (const cousin of cousinsFor(index, entry)) {
        const far = byId.get(cousin.lessonId);
        expect(far).toBeDefined();
        const farRoots = far!.frontmatter.roots;
        expect(Array.isArray(farRoots) ? farRoots.map(String) : []).toContain(cousin.root);
      }
    }
  });
});
