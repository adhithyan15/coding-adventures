import { describe, expect, it } from "vitest";
import { HELD_THRESHOLD, type MasteryBook, newAtom } from "../src/atommastery.ts";
import {
  type DrillableLesson,
  domainOf,
  lessonIsHeld,
  piecesUsed,
  synthesisDrill,
} from "../src/synthesisdrill.ts";

const T0 = 1_700_000_000_000;

/** A book in which every named atom is comfortably held. */
function held(atoms: string[]): MasteryBook {
  const book: MasteryBook = new Map();
  for (const atom of atoms) book.set(atom, { ...newAtom(atom, 1, T0), strength: 0.9 });
  return book;
}

function lesson(
  id: string,
  concept: string,
  headword: string,
  atoms: string[],
): DrillableLesson {
  return { id, language: "spanish", headword, gloss: `${headword} gloss`, concept, introducesAtoms: atoms };
}

const FOOD = lesson("l-food", "ES-FOOD-WATER", "el agua", ["A-FOOD"]);
const TIME = lesson("l-time", "ES-TIME-MORNING", "la mañana", ["A-TIME"]);
const VERB = lesson("l-verb", "VERB-EAT", "comer", ["A-VERB"]);
const GRAMMAR = lesson("l-gram", "ES-GRAMMAR-AR-PRESENT", "hablo", ["A-GRAM"]);

describe("domains", () => {
  it("recognises the domains a drill can be built from", () => {
    expect(domainOf("ES-FOOD-WATER")?.label).toBe("a food or drink word");
    expect(domainOf("VERB-EAT")?.label).toBe("a verb");
    expect(domainOf("GREETING")?.label).toBe("a greeting");
  });

  it("refuses grammar-only tags, because you cannot say a rule", () => {
    expect(domainOf("ES-GRAMMAR-AR-PRESENT")).toBeNull();
    expect(domainOf("ES-REVIEW-DIRECT-OBJECT-SINGULAR")).toBeNull();
    expect(domainOf("ES-SOUND-WRITTEN-ACCENT")).toBeNull();
    expect(domainOf("")).toBeNull();
  });

  it("prefers the longest matching prefix", () => {
    // ES-COURTESY must not be read as some shorter key that happens to match.
    expect(domainOf("ES-COURTESY-THANKS")?.key).toBe("ES-COURTESY");
  });
});

describe("what counts as held", () => {
  it("needs every atom the lesson teaches, not just one", () => {
    const two = lesson("l", "ES-FOOD-X", "pan", ["A", "B"]);
    expect(lessonIsHeld(held(["A", "B"]), two, T0)).toBe(true);
    expect(lessonIsHeld(held(["A"]), two, T0)).toBe(false);
  });

  it("refuses a lesson that teaches nothing, and one just below threshold", () => {
    expect(lessonIsHeld(held(["A"]), lesson("l", "ES-FOOD-X", "pan", []), T0)).toBe(false);
    const weak: MasteryBook = new Map([
      ["A", { ...newAtom("A", 1, T0), strength: HELD_THRESHOLD - 0.01 }],
    ]);
    expect(lessonIsHeld(weak, lesson("l", "ES-FOOD-X", "pan", ["A"]), T0)).toBe(false);
  });
});

describe("building a drill", () => {
  const BOOK = held(["A-FOOD", "A-TIME", "A-VERB", "A-GRAM"]);

  it("returns null until two different domains are held", () => {
    expect(synthesisDrill(new Map(), [FOOD, TIME, VERB], T0)).toBeNull();
    expect(synthesisDrill(held(["A-FOOD"]), [FOOD, TIME, VERB], T0)).toBeNull();
    expect(synthesisDrill(held(["A-FOOD", "A-TIME"]), [FOOD, TIME], T0)).not.toBeNull();
  });

  it("never repeats a domain inside one drill", () => {
    const extraFood = lesson("l-food2", "ES-FOOD-BREAD", "el pan", ["A-FOOD2"]);
    const book = held(["A-FOOD", "A-FOOD2", "A-TIME", "A-VERB"]);
    const drill = synthesisDrill(book, [FOOD, extraFood, TIME, VERB], T0, 0, 3)!;
    const keys = drill.pieces.map((p) => p.domainKey);
    expect(new Set(keys).size).toBe(keys.length);
  });

  it("never offers a grammar lesson as a piece", () => {
    const drill = synthesisDrill(BOOK, [FOOD, TIME, VERB, GRAMMAR], T0, 0, 4)!;
    expect(drill.pieces.some((p) => p.lessonId === "l-gram")).toBe(false);
  });

  it("skips placeholder headwords like (review) and (synthesis)", () => {
    const placeholder = lesson("l-rev", "ES-FOOD-REVIEW", "(review)", ["A-REV"]);
    const book = held(["A-FOOD", "A-TIME", "A-REV"]);
    const drill = synthesisDrill(book, [placeholder, FOOD, TIME], T0, 0, 3)!;
    expect(drill.pieces.some((p) => p.headword.startsWith("("))).toBe(false);
  });

  it("is deterministic for a seed, and different seeds give different drills", () => {
    const first = synthesisDrill(BOOK, [FOOD, TIME, VERB], T0, 0, 2)!;
    expect(synthesisDrill(BOOK, [FOOD, TIME, VERB], T0, 0, 2)).toEqual(first);
    const second = synthesisDrill(BOOK, [FOOD, TIME, VERB], T0, 1, 2)!;
    expect(second.pieces.map((p) => p.domainKey)).not.toEqual(
      first.pieces.map((p) => p.domainKey),
    );
  });

  it("names the pieces by kind, and says why the exercise is hard", () => {
    const drill = synthesisDrill(BOOK, [FOOD, TIME, VERB], T0, 0, 3)!;
    expect(drill.prompt).toContain("a food or drink word");
    expect(drill.prompt).toContain("never been shown them together");
    expect(drill.language).toBe("spanish");
  });

  it("asks for no more pieces than there are domains", () => {
    const drill = synthesisDrill(held(["A-FOOD", "A-TIME"]), [FOOD, TIME], T0, 0, 4)!;
    expect(drill.pieces).toHaveLength(2);
  });
});

describe("checking an answer", () => {
  const pieces = synthesisDrill(
    held(["A-FOOD", "A-TIME", "A-VERB"]),
    [FOOD, TIME, VERB],
    T0,
    0,
    3,
  )!.pieces;

  it("finds the pieces the learner actually used", () => {
    const used = piecesUsed("Por la mañana quiero comer el agua", pieces);
    expect(used).toHaveLength(3);
  });

  it("forgives a missing accent, because a keyboard is not a test", () => {
    expect(piecesUsed("por la manana", pieces).map((p) => p.headword)).toContain("la mañana");
  });

  it("ignores case and surrounding punctuation", () => {
    expect(piecesUsed("¡COMER!", pieces).map((p) => p.headword)).toContain("comer");
  });

  it("reports honestly when a piece is absent", () => {
    expect(piecesUsed("comer", pieces)).toHaveLength(1);
    expect(piecesUsed("", pieces)).toEqual([]);
  });
});
