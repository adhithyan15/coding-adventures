import { describe, it, expect } from "vitest";
import type { Lesson } from "../src/lessons";
import { loadLessons } from "../src/lessons";
import { buildSession, connectionPairs } from "../src/session";

// Minimal Lesson factory — only the fields the orchestrator reads (language,
// concept, chapter, roots) matter.
function L(language: string, concept: string, roots: string[], chapter = 1): Lesson {
  return {
    id: `${language}-${concept}-${roots.join("+")}-${chapter}`,
    language,
    headword: "x",
    gloss: "x",
    type: "word",
    chapter,
    concept,
    prerequisites: [],
    reviewsOf: [],
    roots,
    romanization: "x",
    script: language,
    etymologyHook: "",
  };
}

const langs = (steps: { language: string }[]) => steps.map((s) => s.language);

describe("buildSession", () => {
  it("annotates the sweep with connections back to earlier languages sharing a root", () => {
    // spanish and french share the Latin root `cattus`; german shares nothing.
    const lessons = [
      L("spanish", "CAT", ["cattus"]),
      L("french", "CAT", ["cattus"]),
      L("german", "CAT", ["katta"]), // a different root — no link
    ];
    const steps = buildSession("CAT", lessons, 10);
    expect(langs(steps)).toEqual(["spanish", "french", "german"]); // chain order (from the sweep)

    // spanish is first — no earlier stop to connect to.
    expect(steps[0].connections).toEqual([]);
    // french links back to spanish via cattus.
    expect(steps[1].connections).toEqual([{ to: "spanish", sharedRoots: ["cattus"] }]);
    // german shares no root with either — no connections.
    expect(steps[2].connections).toEqual([]);
  });

  it("links a later language to EVERY earlier one it shares a root with", () => {
    const lessons = [
      L("spanish", "C", ["r1"]),
      L("french", "C", ["r1"]),
      L("italian" /* not on chain — dropped by the sweep */, "C", ["r1"]),
      L("hindi", "C", ["r1", "r2"]),
    ];
    const steps = buildSession("C", lessons, 10);
    // italian is not a chain language, so it never appears.
    expect(langs(steps)).toEqual(["spanish", "french", "hindi"]);
    // hindi shares r1 with both spanish and french (in chain order).
    expect(steps[2].connections).toEqual([
      { to: "spanish", sharedRoots: ["r1"] },
      { to: "french", sharedRoots: ["r1"] },
    ]);
  });

  it("reports multiple shared roots between a pair, sorted", () => {
    const steps = buildSession("C", [L("kannada", "C", ["dhanya", "vada"]), L("telugu", "C", ["vada", "dhanya"])], 10);
    expect(steps[1].connections).toEqual([{ to: "kannada", sharedRoots: ["dhanya", "vada"] }]);
  });

  it("surfaces NO connection when languages share no root — the grounding rule", () => {
    const steps = buildSession("C", [L("spanish", "C", ["a"]), L("french", "C", ["b"])], 10);
    expect(connectionPairs(steps)).toEqual([]); // CONTROL: nothing shared → nothing asserted
  });

  it("respects the active prefix — an inactive language contributes no connection", () => {
    const lessons = [L("spanish", "C", ["r"]), L("french", "C", ["r"])];
    const steps = buildSession("C", lessons, 1); // only spanish active
    expect(langs(steps)).toEqual(["spanish"]);
    expect(connectionPairs(steps)).toEqual([]);
  });
});

describe("buildSession against the real curriculum", () => {
  const lessons = loadLessons();

  it("links Kannada and Telugu 'thank you' back to Hindi via the Sanskrit root dhanya", () => {
    // COURTESY-THANKS: hindi/kannada/telugu all cite the root `dhanya`.
    // (hindi's second root is `vaada`, kannada/telugu's is `vada` — so only
    // `dhanya` is shared with hindi, but kannada/telugu share both with each other.)
    const steps = buildSession("COURTESY-THANKS", lessons, 10);
    const pairs = connectionPairs(steps);

    const teluguToHindi = pairs.find((p) => p.from === "telugu" && p.to === "hindi");
    expect(teluguToHindi, "telugu should link back to hindi").toBeDefined();
    expect(teluguToHindi!.roots).toContain("dhanya");

    const teluguToKannada = pairs.find((p) => p.from === "telugu" && p.to === "kannada");
    expect(teluguToKannada, "telugu should link back to kannada").toBeDefined();
    expect(teluguToKannada!.roots).toEqual(expect.arrayContaining(["dhanya", "vada"]));
  });

  it("CONTROL: a connection is never asserted from thin air (from precedes to in chain order)", () => {
    const steps = buildSession("COURTESY-THANKS", lessons, 10);
    const chainPos = (l: string) =>
      ["spanish", "latin", "french", "german", "arabic", "hindi", "tamil", "kannada", "telugu", "malayalam"].indexOf(l);
    // Every connection points BACKWARD (to an earlier chain language) and every
    // asserted shared root is really present in BOTH lessons' roots.
    const byLang = new Map(loadLessons().filter((l) => l.concept === "COURTESY-THANKS").reduce((m, l) => {
      const arr = m.get(l.language) ?? [];
      arr.push(...l.roots);
      m.set(l.language, arr);
      return m;
    }, new Map<string, string[]>()));
    for (const p of connectionPairs(steps)) {
      expect(chainPos(p.to)).toBeLessThan(chainPos(p.from)); // backward only
      for (const r of p.roots) {
        expect(byLang.get(p.from) ?? []).toContain(r); // root really in `from`
        expect(byLang.get(p.to) ?? []).toContain(r); // root really in `to`
      }
    }
  });
});
