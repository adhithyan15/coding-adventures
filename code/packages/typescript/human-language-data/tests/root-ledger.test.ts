// HL10 section 6.2 -- the Root Ledger (HL-C83).
//
// Every gate gets a firing fixture AND a control. The corpus block pins the
// first measurement, and it is a bleak one: 97% of roots are spent fewer than
// three times. That number is the finding, so it is pinned rather than skipped.

import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { buildRootLedger, renderRootLedger } from "../src/root-ledger.js";
import { loadChapterPolicy, loadEverything } from "../src/loader.js";
import { parseLesson } from "../src/parse.js";

function lesson(
  id: string,
  sequence: number | null,
  opts: { roots?: string[]; introduces?: string[]; practises?: string[] } = {},
) {
  const seq = sequence === null ? "" : `sequence: ${sequence}\n`;
  const roots = opts.roots ? `roots: [${opts.roots.join(", ")}]\n` : "";
  const intro = opts.introduces ? `introduces:\n  knowledge: [${opts.introduces.join(", ")}]\n` : "";
  const prac = opts.practises ? `practises:\n  knowledge: [${opts.practises.join(", ")}]\n` : "";
  return parseLesson(
    `---
schema_version: 2
id: ${id}
${seq}chapter: 1
type: vocabulary
headword: x
gloss: x
${roots}${intro}${prac}---

# x
`,
    "spanish",
  );
}

describe("buildRootLedger", () => {
  it("scores an introduction as zero payoffs, not one", () => {
    // A root named in exactly one lesson has been taught and never spent. If the
    // introduction counted, every root would start at 1 and the ledger would
    // flatter the corpus by exactly the number of roots it has.
    const l = buildRootLedger([lesson("L1", 10, { roots: ["stare-latin"] })], 3);
    expect(l.entries).toHaveLength(1);
    expect(l.entries[0]?.payoffCount).toBe(0);
    expect(l.entries[0]?.introducedBy).toBe("L1");
    expect(l.summary.neverSpent).toBe(1);
  });

  it("credits each later lesson that names the root again", () => {
    const l = buildRootLedger(
      [
        lesson("L1", 10, { roots: ["stare-latin"] }),
        lesson("L2", 20, { roots: ["stare-latin"] }),
        lesson("L3", 30, { roots: ["stare-latin"] }),
      ],
      3,
    );
    expect(l.entries[0]?.payoffCount).toBe(2);
    expect(l.entries[0]?.payoffs).toEqual(["L2", "L3"]);
    expect(l.entries[0]?.underspent).toBe(true); // 2 < 3
  });

  it("control: a root spent three times is not underspent", () => {
    const l = buildRootLedger(
      [10, 20, 30, 40].map((s, i) => lesson(`L${i}`, s, { roots: ["stare-latin"] })),
      3,
    );
    expect(l.entries[0]?.payoffCount).toBe(3);
    expect(l.entries[0]?.underspent).toBe(false);
    expect(l.summary.underspent).toBe(0);
  });

  it("reads etymon atoms from the DOTTED frontmatter keys", () => {
    // The keys are flat: `introduces.knowledge`, never a nested `introduces`
    // object. Reading them as nested returns undefined for every lesson in the
    // corpus, and the first draft of this module did exactly that -- silently
    // contributing ZERO etymon atoms, which reads as "the corpus has none"
    // rather than "the reader is broken".
    const l = buildRootLedger(
      [
        lesson("L1", 10, { introduces: ["ES-ETYMON-GRATIA"] }),
        lesson("L2", 20, { practises: ["ES-ETYMON-GRATIA"] }),
      ],
      3,
    );
    expect(l.entries).toHaveLength(1);
    expect(l.entries[0]?.namespace).toBe("etymon-atom");
    expect(l.entries[0]?.payoffCount).toBe(1);
  });

  it("keeps the two namespaces separate rather than merging them", () => {
    const l = buildRootLedger(
      [lesson("L1", 10, { roots: ["gratia"], introduces: ["ES-ETYMON-GRATIA"] })],
      3,
    );
    expect(l.entries).toHaveLength(2);
    expect(l.entries.map((e) => e.namespace).sort()).toEqual(["etymon-atom", "roots"]);
  });

  it("counts a root named twice by one lesson as a single payoff", () => {
    const l = buildRootLedger(
      [
        lesson("L1", 10, { roots: ["gratia"] }),
        // named through BOTH a root slug and, separately, again in the same file
        lesson("L2", 20, { roots: ["gratia", "gratia"] }),
      ],
      3,
    );
    const slug = l.entries.find((e) => e.namespace === "roots")!;
    expect(slug.payoffs).toEqual(["L2"]);
  });

  it("orders by `sequence`, which the parser emits as a STRING", () => {
    // L2 comes first in the array but second by sequence, so L1 must be the
    // introducer. A `typeof raw === "number"` test would send both to Infinity
    // and let array order decide.
    const l = buildRootLedger(
      [lesson("L2", 20, { roots: ["gratia"] }), lesson("L1", 10, { roots: ["gratia"] })],
      3,
    );
    expect(l.entries[0]?.introducedBy).toBe("L1");
    expect(l.entries[0]?.payoffs).toEqual(["L2"]);
  });

  it("breaks unsequenced ties stably instead of by a NaN comparator", () => {
    // Infinity - Infinity is NaN. Without the id tie-break the introducer of an
    // unsequenced pair would vary between runs and the ledger would not be
    // reproducible.
    const run = () =>
      buildRootLedger(
        [lesson("L2", null, { roots: ["gratia"] }), lesson("L1", null, { roots: ["gratia"] })],
        3,
      ).entries[0]?.introducedBy;
    expect(run()).toBe("L1");
    expect(run()).toBe(run());
  });

  it("does not merge two roots whose composite key could collide", () => {
    // Length-prefixed keying. With a plain space join, ("roots", "a b") and a
    // hypothetical ("roots a", "b") would land in the same bucket and silently
    // sum two roots' payoffs.
    const l = buildRootLedger([lesson("L1", 10, { roots: ["a b", "a", "b"] })], 3);
    expect(l.entries).toHaveLength(3);
  });

  it("sorts worst first, so the root to cut is not found by scrolling", () => {
    const l = buildRootLedger(
      [
        lesson("L1", 10, { roots: ["spent", "unspent"] }),
        lesson("L2", 20, { roots: ["spent"] }),
      ],
      3,
    );
    expect(l.entries[0]?.root).toBe("unspent");
  });
});

describe("renderRootLedger", () => {
  it("names roots taught once and never returned to", () => {
    const lines = renderRootLedger(
      buildRootLedger([lesson("L1", 10, { roots: ["orphan"] })], 3),
    ).join("\n");
    expect(lines).toContain("never spent");
    expect(lines).toContain("orphan");
  });

  it("control: says nothing about unspent roots when every root is spent", () => {
    const lines = renderRootLedger(
      buildRootLedger(
        [10, 20, 30, 40].map((s, i) => lesson(`L${i}`, s, { roots: ["good"] })),
        3,
      ),
    ).join("\n");
    expect(lines).not.toContain("taught once and never returned to");
    expect(lines).toContain("best-spent root");
  });
});

describe("the committed corpus", () => {
  const { lessons } = loadEverything();
  const minReuse = loadChapterPolicy().rootLedgerMinReuse ?? 3;

  it("uses the policy's configured minimum rather than a constant", () => {
    expect(minReuse).toBe(3);
  });

  it("pins the first ledger, and it is the reason the rule exists", () => {
    const l = buildRootLedger(lessons, minReuse);
    expect(l.summary.roots).toBe(3083); // Italian, Persian, and Portuguese typed openers each record an etymon atom.
    expect(l.summary.underspent).toBe(2968); // All three opener etymons begin below the reuse floor.
    expect(l.summary.neverSpent).toBe(2036); // Persian's opener re-spends one previously stranded root.
    expect(l.summary.underspentPercent).toBe(96); // -1: HL-C98

    // Both namespaces contribute. If the etymon-atom count ever returns to
    // zero, the dotted-key reader has broken again.
    const byNamespace = l.entries.reduce<Record<string, number>>((acc, e) => {
      acc[e.namespace] = (acc[e.namespace] ?? 0) + 1;
      return acc;
    }, {});
    // +1 roots: latin-vos, from ES-C03-vos.
    // +53 roots, +25 etymon-atom: vocabulary wave 5.
    expect(byNamespace).toEqual({ roots: 2194, "etymon-atom": 889 }); // +3: Italian, Persian, and Portuguese opener etymons.
  });

  it("pins Spanish, the pilot track", () => {
    const l = buildRootLedger(
      lessons.filter((lesson) => lesson.language === "spanish"),
      minReuse,
    );
    expect(l.summary).toMatchObject({
      roots: 470, // HL-C194: +16 Spanish pre-A1 words // HL-C181: +5 -- chapter 277, the spine closes at 33/33 // HL-C180: +4 -- chapter 276; ARCHAIC-FORM was already taught at chapter 3 // HL-C179: +5 -- chapter 275, fine shades // HL-C178: +5 -- chapter 274, C2 opens // HL-C177: +5 -- chapter 273, C1 closes // HL-C175: +5 -- chapter 272, reading between the lines // HL-C173: +2 -- B2 closes (chapter 271) // HL-C172: +4 -- the B2 argue rung (chapter 270) // HL-C128 step 10: +4 -- primus-latin, secundus-latin and their etymons, plus the otro etymon // HL-C128 step 8: +4 -- hodie-latin and dare-latin, and their two etymons; hora-latin is re-spent by ahora rather than minted // +3: HL-C88 slice 4 adds profiteri-latin, istes-greek and ES-ETYMON-PROFITERI // +6: HL-C88 slices 5-6 // +2: HL-C88 slice 8 adds arius-latin and ES-ETYMON-ARIUS // +2: HL-C88 slice 9 adds exitus-latin and ES-ETYMON-EXITUS // +2: B1 si-condition adds si-latin and ES-ETYMON-SI-LATIN // +1: HL-C113 imperfect subjunctive adds only ES-ETYMON-RA-PLUPERFECT; it re-spends fabulari-latin rather than minting a slug for a grammatical category // HL-C113 step 8: +7 // HL-C128 step 2: +7 // HL-C128 step 3: +6 // HL-C128 step 4: +6 // HL-C128 step 5: +1 // HL-C128 step 7: +2 -- cum-latin and the conmigo etymon // HL-C158: +4 -- the B1 travel rung (chapter 268) // HL-C159: +4 -- the B1 describe-experience rung (chapter 269) // HL-C160: +1 -- depende closes SPINE-EXPRESS-CONDITION, and B1
      underspent: 441, // HL-C194: +16 Spanish pre-A1 words // HL-C181: +5 -- chapter 277, the spine closes at 33/33 // HL-C180: +4 -- chapter 276; ARCHAIC-FORM was already taught at chapter 3 // HL-C179: +5 -- chapter 275, fine shades // HL-C178: +5 -- chapter 274, C2 opens // HL-C177: +5 -- chapter 273, C1 closes // HL-C175: +5 -- chapter 272, reading between the lines // HL-C173: +2 -- B2 closes (chapter 271) // HL-C172: +4 -- the B2 argue rung (chapter 270) // HL-C128 step 10: +3 // HL-C128 step 8: +4 // +2: HL-C88 slice 4 // +6: HL-C88 slices 5-6 // +2: HL-C88 slice 8 adds arius-latin and ES-ETYMON-ARIUS // +2: HL-C88 slice 9 adds exitus-latin and ES-ETYMON-EXITUS // +2: B1 si-condition adds si-latin and ES-ETYMON-SI-LATIN // HL-C113 step 6: +1 -- unspent until the reported-speech review revisits it // HL-C113 step 8: +7 // HL-C128 step 2: +7 // HL-C128 step 3: +6 // HL-C128 step 4: +6 // HL-C128 step 5: +1 // HL-C128 step 7: +2 // HL-C158: +4 -- the B1 travel rung (chapter 268) // HL-C159: +4 -- the B1 describe-experience rung (chapter 269) // HL-C160: +1 -- depende closes SPINE-EXPRESS-CONDITION, and B1
      neverSpent: 294, // HL-C194: +16 Spanish pre-A1 words // HL-C181: +5 -- chapter 277, the spine closes at 33/33 // HL-C179: +5 -- chapter 275, fine shades // HL-C178: +5 -- chapter 274, C2 opens // HL-C177: +5 -- chapter 273, C1 closes // HL-C175: +5 -- chapter 272, reading between the lines // HL-C173: +2 -- B2 closes (chapter 271) // HL-C172: +4 -- the B2 argue rung (chapter 270) // HL-C128 step 10: +4 // HL-C128 step 8: +2 -- the ch256 review spends two of the four // +3: HL-C88 slice 4 introduces three roots not yet re-spent // +6: HL-C88 slices 5-6 // +2: HL-C88 slice 8 adds arius-latin and ES-ETYMON-ARIUS // +2: HL-C88 slice 9 adds exitus-latin and ES-ETYMON-EXITUS // +2: B1 si-condition adds si-latin and ES-ETYMON-SI-LATIN // +1: HL-C113 imperfect subjunctive adds only ES-ETYMON-RA-PLUPERFECT; it re-spends fabulari-latin rather than minting a slug for a grammatical category // HL-C113 step 6 added ES-ETYMON-DIJO-DIXIT and step 7 spent it: the reported-speech review revisits the Latin x, so the net move is zero // HL-C113 step 8: +4 -- tam-latin is spent by tampoco reusing tambien's frame; the rest wait for the review rung // HL-C128 step 2: +4 -- ille-latin is re-spent by aquel pointing back at el, so it is not among them // HL-C128 step 3: +4 // HL-C128 step 4: +6 // HL-C128 step 5: no change -- ES-ETYMON-GERUND-NDUM is minted by ch236 and re-spent by the ch240 review, so it never enters the unspent set // HL-C128 step 7: +1 -- cum-latin, minted by conmigo and not yet re-spent // HL-C158: +4 -- the B1 travel rung (chapter 268) // HL-C159: +4 -- the B1 describe-experience rung (chapter 269) // HL-C160: +1 -- depende closes SPINE-EXPRESS-CONDITION, and B1 // spanish pre-A1 tranche: +35 lessons, +7 chapters (chapters 282-288)
      underspentPercent: 94, // HL-C194: +16 Spanish pre-A1 words // HL-C128 step 2: +1 -- four new roots, none re-spent yet; the demonstrative review revisits the atoms but not the slugs
    });
  });
});

describe("source hygiene", () => {
  it("has no NUL bytes in any source file", () => {
    // Not paranoia. While writing root-ledger.ts, the spaces inside a template
    // literal were written to disk as U+0000: `${language}\0${namespace}`. The
    // file still compiled, grep silently found nothing in it, and an exact-match
    // edit could not touch the line. A NUL in source is always a write accident,
    // never intent, so it is cheaper to assert than to rediscover.
    const here = dirname(fileURLToPath(import.meta.url));
    const src = join(here, "..", "src");
    const offenders: string[] = [];
    for (const name of readdirSync(src)) {
      if (!name.endsWith(".ts")) continue;
      if (readFileSync(join(src, name)).includes(0)) offenders.push(name);
    }
    expect(offenders).toEqual([]);
  });
});
