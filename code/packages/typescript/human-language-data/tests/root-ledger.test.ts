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
    expect(l.summary.roots).toBe(2722); // +5: vos-latin slug and ES-ETYMON-VOS-03
    expect(l.summary.underspent).toBe(2625); // +3: a payoff lesson re-spends a root // +1: the vos-latin slug is unspent; ES-ETYMON-VOS-03 is spent three times, so it is NOT latin-vos, spent once so far // -1: HL-C98 spends fabulari-latin a third time
    expect(l.summary.neverSpent).toBe(1803); // -6: HL-C94 payoff lessons re-spend roots latin-vos, introduced and not yet re-spent // +1: HL-C98
    expect(l.summary.underspentPercent).toBe(96); // -1: HL-C98

    // Both namespaces contribute. If the etymon-atom count ever returns to
    // zero, the dotted-key reader has broken again.
    const byNamespace = l.entries.reduce<Record<string, number>>((acc, e) => {
      acc[e.namespace] = (acc[e.namespace] ?? 0) + 1;
      return acc;
    }, {});
    // +1 roots: latin-vos, from ES-C03-vos.
    expect(byNamespace).toEqual({ roots: 1968, "etymon-atom": 754 });
  });

  it("pins Spanish, the pilot track", () => {
    const l = buildRootLedger(
      lessons.filter((lesson) => lesson.language === "spanish"),
      minReuse,
    );
    expect(l.summary).toMatchObject({
      roots: 308, // +2: vos-latin and ES-ETYMON-VOS-03
      underspent: 291, // -1: HL-C98 spends fabulari-latin a third time
      neverSpent: 186, // +1: HL-C98
      underspentPercent: 94,
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
