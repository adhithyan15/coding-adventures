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
import { measureEtymologyCoverage, type EtymologyGap } from "../src/etymology-coverage.js";
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

/**
 * Etymology coverage per track (HL24 §9): AN EXACT PIN THAT MAY ONLY BE LOWERED.
 *
 * Two counts per track, over the lessons that introduce a word (a `-LEX-`
 * atom, or `type: word` / `phrase`): those with no `etymology_hook`, and
 * those whose body shows the reader no origin statement. See
 * src/etymology-coverage.ts for both rules and their measured misses.
 *
 * Pinned EXACTLY, like FORWARD_REFERENCE_CEILINGS in continuity.test.ts and
 * for the same reason: a ceiling leaves slack under it once debt is paid, and
 * the next lesson could spend that slack unnoticed. So:
 *
 *   - a count that RISES fails and names the newest offending lessons. The
 *     fix is in the lesson -- say where the word comes from, from a cited
 *     source, or say plainly that the origin is unknown or disputed. Never
 *     invent one, and never raise the pin.
 *   - a count that FALLS fails too, asking for the pin to be lowered in the
 *     same change, so the cleared debt cannot be re-spent.
 *
 * The rule is for every NEW word, so a new word lesson must carry both a hook
 * and visible etymology to leave these numbers where they are.
 *
 * Measured when the gate was added: 25,682 word-introducing lessons; 22,744
 * without a hook, 22,378 without visible etymology, 21,697 with neither.
 * Spanish, the pilot, is the only track where most words have both. Chinese
 * has a hook on every word lesson -- its hooks take the character apart --
 * and 62 lessons whose body never does.
 */
const ETYMOLOGY_GAP_PINS: Readonly<Record<string, { withoutHook: number; withoutVisible: number }>> = {
  arabic: { withoutHook: 1187, withoutVisible: 1195 },
  bengali: { withoutHook: 1136, withoutVisible: 1144 },
  chinese: { withoutHook: 0, withoutVisible: 62 },
  // french: 1048 / 1049 -> 0 / 0. All 1,045 generated word lessons (FR-C49-bras through
  // FR-C257-anniversaire) gained a taken-apart block and a matching hook, and seven earlier
  // hand-written lessons the measure missed gained the half they lacked. Sources: EtymDB 2.1
  // (Fourrier & Sagot 2020), CC BY-SA 4.0, cross-checked row by row, with Lewis & Short for
  // textbook Latin forms EtymDB lacks; 18 say the origin is uncertain and give the usual account.
  french: { withoutHook: 0, withoutVisible: 0 },
  // german: 988 / 1071 -> 0 / 0. Every German word lesson now carries an etymology_hook
  // and a learner-visible "taken apart" block, sourced from EtymDB 2.1 (Fourrier &
  // Sagot 2020), CC BY-SA 4.0, where a row supports the line; well-established origins
  // with no supporting row carry no source tag, and 12 genuinely uncertain origins say so.
  german: { withoutHook: 0, withoutVisible: 0 },
  gujarati: { withoutHook: 529, withoutVisible: 543 },
  // hindi: 1126 / 967 -> 0 / 0. Every Hindi word lesson now has an etymology_hook and says
  // where its word comes from: 962 new "taken apart" blocks, plus one origin sentence in 43
  // lessons whose block or section had none. Inherited words and Sanskrit loans cite Turner's
  // CDIAL (read from the moli-mandala files), keeping its "prob.", "?", Dravidian, Munda and
  // Austroasiatic notes; Arabic loans name their root in Lane's Lexicon; Persian, English and
  // Portuguese loans name the donor, Persian ones as "usually traced to". About 48 say the
  // origin is uncertain or only partly known. 15 older blocks that contradicted CDIAL were
  // corrected. 41 max_seconds rose to the computed length, all still under 300 s.
  hindi: { withoutHook: 0, withoutVisible: 0 },
  // italian: 1126 / 1135 -> 745 / 745 -> 0 / 0. The first 401 gap lessons in sequence
  // order (through IT-C110-pepe) gained a sourced origin -- EtymDB 2.1 (Fourrier & Sagot
  // 2020), CC BY-SA 4.0, with Lewis & Short for textbook Latin forms EtymDB lacks --
  // a well-established origin stated without a source tag (4), or an explicit "not recorded in
  // EtymDB 2.1" (12 lessons, all genuinely uncertain). The remaining 745 (IT-C111-infermiera
  // through IT-C259-storia) followed the same way: from the same two sources, with sister-word
  // tracing where EtymDB has no row; 14 say the origin is uncertain and why.
  italian: { withoutHook: 0, withoutVisible: 0 },
  japanese: { withoutHook: 601, withoutVisible: 688 },
  // kannada: 1330 / 1281 -> 0 / 0. All 1,345 gap lessons (KA-C01 through KA-C307) now say
  // where their word comes from in a taken-apart block or an origin sentence, with a matching
  // hook. Native words cite DEDR (Burrow & Emeneau 1984), checked against the full article
  // text with every "?", "Cf. Skt." and "< IA" kept; Sanskrit tatsamas and tadbhavas cite
  // CDIAL (Turner); English and Perso-Arabic loans name the donor. About 63 say the origin is
  // uncertain after a search of both sources by form and by gloss, giving the usual account
  // as a hedge where there is one.
  kannada: { withoutHook: 0, withoutVisible: 0 },
  // latin: 1095 / 1105 -> 0 / 0. All 1,090 generated word lessons (LA-C63-bracchium through
  // LA-C280-vallum) gained a taken-apart block and a matching hook; the five Chapter 1
  // greetings gained the hook they lacked, and fifteen hand-written lessons a short taken-apart
  // block. Sources: EtymDB 2.1 (Fourrier & Sagot 2020), CC BY-SA 4.0, checked row by row, and
  // Lewis & Short, whose outdated comparisons are left out. PIE and Proto-Italic forms are
  // starred and hedged; 183 say the origin is uncertain and give the usual account where
  // one exists.
  latin: { withoutHook: 0, withoutVisible: 0 },
  // malayalam: 1367 / 1275 -> 0 / 0. Every Malayalam word lesson now carries an etymology_hook
  // and says where its word comes from, almost always in a "taken apart" block. Native words
  // cite the Dravidian Etymological Dictionary (DEDR, Burrow & Emeneau 1984), naming only
  // cognates the full DEDR article lists and keeping its own "Cf. Skt." and "?" caveats;
  // Sanskrit loans cite Turner's CDIAL where it has the form; English, Portuguese, Arabic and
  // Persian loans name the donor. 47 say the origin is uncertain or only usually traced.
  malayalam: { withoutHook: 0, withoutVisible: 0 },
  marathi: { withoutHook: 1163, withoutVisible: 1116 },
  marwadi: { withoutHook: 552, withoutVisible: 716 },
  persian: { withoutHook: 1300, withoutVisible: 1311 },
  // portuguese: 1142 / 1141 -> 0 / 0. Every word lesson now states its word's origin in a
  // "taken apart" block and a matching etymology_hook: EtymDB 2.1 (Fourrier & Sagot 2020),
  // CC BY-SA 4.0, with Lewis & Short for certain Latin forms EtymDB lacks. 62 say the
  // origin is uncertain or disputed, each a genuinely open question, never a stand-in.
  portuguese: { withoutHook: 0, withoutVisible: 0 },
  punjabi: { withoutHook: 549, withoutVisible: 524 },
  // russian: 1135 / 1148 -> 0 / 0. All 1,135 generated word lessons gained a taken-apart
  // block and a matching hook, and 13 earlier hand-written lessons the measure missed gained
  // the half they lacked (six as a block, seven as one origin sentence in an existing section,
  // paid for by trimming prose; only RU-C02-menya-zovut needed a raise, 240 -> 275 s). Source:
  // Vasmer's Russisches etymologisches Woerterbuch (Russian edition, with Trubachev's notes),
  // paraphrased and cited, never copied; Church Slavonic shapes are named by their sound
  // change, and loans name the donor and the route. 86 say the origin is uncertain or disputed
  // and give Vasmer's usual account with his hedges.
  russian: { withoutHook: 0, withoutVisible: 0 },
  sanskrit: { withoutHook: 1200, withoutVisible: 1099 },
  spanish: { withoutHook: 486, withoutVisible: 209 },
  // tamil: 1185 / 1144 -> 0 / 4. Every Tamil word lesson now has an etymology_hook, and all
  // but four have a "taken apart" block: native words cite the Dravidian Etymological
  // Dictionary (Burrow & Emeneau, DEDR) and Sanskrit loans cite Turner's CDIAL, both read
  // from the moli-mandala/data (Jambu) files; no source text is copied. 0 / 4 -> 0 / 0:
  // the four that estimated at 294-299 s (TA-C06-dative-subject, TA-C37-ivar,
  // TA-C37-ivar-en-nanbar, TA-C39-evvalavu) each gained one origin sentence in an existing
  // section, paid for by trimming redundant prose; only TA-C39-evvalavu needed a raise
  // (294 -> 298 s, still under 300).
  tamil: { withoutHook: 0, withoutVisible: 0 },
  // telugu: 1376 / 1318 -> 0 / 0. Every Telugu word lesson now carries an etymology_hook and
  // says where its word comes from, almost always in a "taken apart" block. Native words cite
  // the Dravidian Etymological Dictionary (DEDR, Burrow & Emeneau 1984), naming only cognates
  // the full article lists and keeping its "Cf. Skt.", "?" and Indo-Aryan-appendix notes;
  // Sanskrit loans cite Turner's CDIAL where it has the form, with its "<- Drav." notes; both
  // read from the moli-mandala/data files, no source text copied. English, Persian, Arabic and
  // Portuguese loans name the donor. 79 say the origin is uncertain, after a search of both
  // sources by form and by gloss.
  telugu: { withoutHook: 0, withoutVisible: 0 },
  urdu: { withoutHook: 1123, withoutVisible: 1137 },
};

/**
 * How many offending lessons a rise names. A track's whole gap list runs to
 * a thousand ids, which buries the one that moved; new lessons are usually
 * appended, so the latest in reading order come first.
 */
const NEWEST_GAPS_SHOWN = 40;

describe("the committed corpus", () => {
  const { lessons } = loadEverything();
  const minReuse = loadChapterPolicy().rootLedgerMinReuse ?? 3;
  // Measured once, here, where the corpus is already loaded: one linear pass,
  // well under a second, rather than a second whole-corpus load in its own file.
  const etymology = measureEtymologyCoverage(lessons);

  it("pins etymology coverage for every track, and no track that is gone", () => {
    expect(Object.keys(ETYMOLOGY_GAP_PINS).sort()).toEqual(
      etymology.tracks.map((track) => track.language).sort(),
    );
  });

  for (const [language, pin] of Object.entries(ETYMOLOGY_GAP_PINS)) {
    const kinds = [
      { key: "withoutHook", label: "word lessons without an etymology_hook", missing: (gap: EtymologyGap) => gap.missingHook },
      { key: "withoutVisible", label: "word lessons without visible etymology", missing: (gap: EtymologyGap) => gap.missingVisible },
    ] as const;
    for (const kind of kinds) {
      const pinned = pin[kind.key];
      it(`holds ${language}'s ${kind.label} at exactly ${pinned} (a pin that may only be lowered)`, () => {
        const found = etymology.gaps.filter((gap) => gap.language === language && kind.missing(gap));
        // Two directions, two fixes, as in continuity.test.ts. A rise is a
        // lesson that arrived without its etymology: name it. A fall is
        // progress the table has not recorded: lower the pin.
        const newest = [...found]
          .sort((a, b) => b.position - a.position)
          .slice(0, NEWEST_GAPS_SHOWN)
          .map((gap) => gap.lessonId)
          .join(", ");
        const message =
          found.length > pinned
            ? `${language} has ${found.length} ${kind.label}, above its pin of ${pinned}. ` +
              `Every new word says where it comes from (HL24 §9) -- from a cited source, or ` +
              `"unknown"/"disputed" stated plainly; never invented. Fix the lessons, not the pin. ` +
              `Newest ${Math.min(found.length, NEWEST_GAPS_SHOWN)}, latest first: ${newest}`
            : `${language} has ${found.length} ${kind.label}, below its pin of ${pinned}. ` +
              `Lower ETYMOLOGY_GAP_PINS.${language}.${kind.key} to ${found.length} in this change.`;
        expect(found.length, message).toBe(pinned);
      });
    }
  }

  it("uses the policy's configured minimum rather than a constant", () => {
    expect(minReuse).toBe(3);
  });

  it("keeps the corpus summary internally consistent", () => {
    const l = buildRootLedger(lessons, minReuse);
    expect(l.summary.roots).toBe(l.entries.length);
    expect(l.summary.underspent).toBe(l.entries.filter((entry) => entry.underspent).length);
    expect(l.summary.neverSpent).toBe(
      l.entries.filter((entry) => entry.payoffCount === 0).length,
    );
    /* Superseded Arabic-only totals:
    expect(l.summary.roots).toBe(3083); // Arabic's greeting ramp records two previously unledgered roots.
    expect(l.summary.underspent).toBe(2968); // Both new roots begin below the reuse floor.
    expect(l.summary.neverSpent).toBe(2033); // Arabic's interleaving reuses four roots that previously appeared only at declaration.
    */
    /* Historical cumulative context:
    expect(l.summary.roots).toBe(3082); // Italian's ciao and Portuguese's ola openers each record an etymon atom.
    expect(l.summary.underspent).toBe(2967); // Both new opener etymons begin below the reuse floor.
    expect(l.summary.neverSpent).toBe(2037); // HL-C250: +1 -- est is declared by its own lesson and spent nowhere else in russian yet // HL-C247: +3 -- the three latin roots are declared by their own french lessons and spent nowhere else in french yet // HL-C128 step 10: +4 // -6: HL-C94 payoff lessons re-spend roots latin-vos, introduced and not yet re-spent // +1: HL-C98 // +51: vocabulary wave 5 // +6: HL-C88 slices 5-6 // +2: HL-C88 slice 8 // +72: vocabulary wave 6 // +2: HL-C113 (B1 si-condition rung) // HL-C113 preterite close // HL-C113: HL-C113 imperfect subjunctive // HL-C128 step 7: +1 -- cum-latin, minted by conmigo and not re-spent // HL-C128 step 8: +2 // HL-C157: ayer + hablare close A2 // HL-C158: +4 -- the B1 travel rung (chapter 268) // HL-C159: +4 -- the B1 describe-experience rung (chapter 269) // HL-C160: +1 -- depende closes SPINE-EXPRESS-CONDITION, and B1 // HL-C172: +4 -- the B2 argue rung (chapter 270) // HL-C173: +2 -- B2 closes (chapter 271) // HL-C175: +5 -- chapter 272, reading between the lines // HL-C177: +5 -- chapter 273, C1 closes // HL-C178: +5 -- chapter 274, C2 opens // HL-C179: +5 -- chapter 275, fine shades // HL-C181: +5 -- chapter 277, the spine closes at 33/33 // HL-C194: +16 Spanish pre-A1 words // spanish pre-A1 tranche: +35 lessons, +7 chapters (chapters 282-288)
    */
    expect(l.summary.underspentPercent).toBe(
      Math.round((l.summary.underspent / l.summary.roots) * 100),
    );

    // Both namespaces contribute. If the etymon-atom count ever returns to
    // zero, the dotted-key reader has broken again.
    const byNamespace = l.entries.reduce<Record<string, number>>((acc, e) => {
      acc[e.namespace] = (acc[e.namespace] ?? 0) + 1;
      return acc;
    }, {});
    // +1 roots: latin-vos, from ES-C03-vos.
    // +53 roots, +25 etymon-atom: vocabulary wave 5.
    expect(byNamespace.roots).toBeGreaterThan(0);
    expect(byNamespace["etymon-atom"]).toBeGreaterThan(0);
    expect(Object.values(byNamespace).reduce((sum, count) => sum + count, 0)).toBe(
      l.summary.roots,
    );
    /* Superseded merge-side totals:
    expect(byNamespace).toEqual({ roots: 2196, "etymon-atom": 887 }); // Arabic adds two root-namespace entries.
    expect(byNamespace).toEqual({ roots: 2194, "etymon-atom": 888 }); // +2: Italian and Portuguese migrated opener etymons.
    */
  });

  it("pins Spanish, the pilot track", () => {
    const l = buildRootLedger(
      lessons.filter((lesson) => lesson.language === "spanish"),
      minReuse,
    );
    expect(l.summary.roots).toBeGreaterThanOrEqual(484); // spanish pre-A1 survival tranche // spanish pre-A1 survival tranche: +15 lessons, +3 chapters (chapters 303-305): +13 -- fourteen etyma minted, but senior is spent twice (senor, senora) // HL-C194: +16 Spanish pre-A1 words // HL-C181: +5 -- chapter 277, the spine closes at 33/33 // HL-C180: +4 -- chapter 276; ARCHAIC-FORM was already taught at chapter 3 // HL-C179: +5 -- chapter 275, fine shades // HL-C178: +5 -- chapter 274, C2 opens // HL-C177: +5 -- chapter 273, C1 closes // HL-C175: +5 -- chapter 272, reading between the lines // HL-C173: +2 -- B2 closes (chapter 271) // HL-C172: +4 -- the B2 argue rung (chapter 270) // HL-C128 step 10: +4 -- primus-latin, secundus-latin and their etymons, plus the otro etymon // HL-C128 step 8: +4 -- hodie-latin and dare-latin, and their two etymons; hora-latin is re-spent by ahora rather than minted // +3: HL-C88 slice 4 adds profiteri-latin, istes-greek and ES-ETYMON-PROFITERI // +6: HL-C88 slices 5-6 // +2: HL-C88 slice 8 adds arius-latin and ES-ETYMON-ARIUS // +2: HL-C88 slice 9 adds exitus-latin and ES-ETYMON-EXITUS // +2: B1 si-condition adds si-latin and ES-ETYMON-SI-LATIN // +1: HL-C113 imperfect subjunctive adds only ES-ETYMON-RA-PLUPERFECT; it re-spends fabulari-latin rather than minting a slug for a grammatical category // HL-C113 step 8: +7 // HL-C128 step 2: +7 // HL-C128 step 3: +6 // HL-C128 step 4: +6 // HL-C128 step 5: +1 // HL-C128 step 7: +2 -- cum-latin and the conmigo etymon // HL-C158: +4 -- the B1 travel rung (chapter 268) // HL-C159: +4 -- the B1 describe-experience rung (chapter 269) // HL-C160: +1 -- depende closes SPINE-EXPRESS-CONDITION, and B1 // FLOOR — content only grows; exact pins serialize parallel tranches
    expect(l.summary.underspent).toBeGreaterThanOrEqual(455); // spanish pre-A1 survival tranche // spanish pre-A1 survival tranche: +15 lessons, +3 chapters (chapters 303-305): +13 -- each new root is spent once so far // HL-C194: +16 Spanish pre-A1 words // HL-C181: +5 -- chapter 277, the spine closes at 33/33 // HL-C180: +4 -- chapter 276; ARCHAIC-FORM was already taught at chapter 3 // HL-C179: +5 -- chapter 275, fine shades // HL-C178: +5 -- chapter 274, C2 opens // HL-C177: +5 -- chapter 273, C1 closes // HL-C175: +5 -- chapter 272, reading between the lines // HL-C173: +2 -- B2 closes (chapter 271) // HL-C172: +4 -- the B2 argue rung (chapter 270) // HL-C128 step 10: +3 // HL-C128 step 8: +4 // +2: HL-C88 slice 4 // +6: HL-C88 slices 5-6 // +2: HL-C88 slice 8 adds arius-latin and ES-ETYMON-ARIUS // +2: HL-C88 slice 9 adds exitus-latin and ES-ETYMON-EXITUS // +2: B1 si-condition adds si-latin and ES-ETYMON-SI-LATIN // HL-C113 step 6: +1 -- unspent until the reported-speech review revisits it // HL-C113 step 8: +7 // HL-C128 step 2: +7 // HL-C128 step 3: +6 // HL-C128 step 4: +6 // HL-C128 step 5: +1 // HL-C128 step 7: +2 // HL-C158: +4 -- the B1 travel rung (chapter 268) // HL-C159: +4 -- the B1 describe-experience rung (chapter 269) // HL-C160: +1 -- depende closes SPINE-EXPRESS-CONDITION, and B1 // FLOOR — content only grows; exact pins serialize parallel tranches
    expect(l.summary.neverSpent).toBeGreaterThanOrEqual(307); // spanish pre-A1 survival tranche // spanish pre-A1 survival tranche: +15 lessons, +3 chapters (chapters 303-305): +11 -- senior, and the two roots the chapter-payoff lessons re-spend, are not among them // HL-C194: +16 Spanish pre-A1 words // HL-C181: +5 -- chapter 277, the spine closes at 33/33 // HL-C179: +5 -- chapter 275, fine shades // HL-C178: +5 -- chapter 274, C2 opens // HL-C177: +5 -- chapter 273, C1 closes // HL-C175: +5 -- chapter 272, reading between the lines // HL-C173: +2 -- B2 closes (chapter 271) // HL-C172: +4 -- the B2 argue rung (chapter 270) // HL-C128 step 10: +4 // HL-C128 step 8: +2 -- the ch256 review spends two of the four // +3: HL-C88 slice 4 introduces three roots not yet re-spent // +6: HL-C88 slices 5-6 // +2: HL-C88 slice 8 adds arius-latin and ES-ETYMON-ARIUS // +2: HL-C88 slice 9 adds exitus-latin and ES-ETYMON-EXITUS // +2: B1 si-condition adds si-latin and ES-ETYMON-SI-LATIN // +1: HL-C113 imperfect subjunctive adds only ES-ETYMON-RA-PLUPERFECT; it re-spends fabulari-latin rather than minting a slug for a grammatical category // HL-C113 step 6 added ES-ETYMON-DIJO-DIXIT and step 7 spent it: the reported-speech review revisits the Latin x, so the net move is zero // HL-C113 step 8: +4 -- tam-latin is spent by tampoco reusing tambien's frame; the rest wait for the review rung // HL-C128 step 2: +4 -- ille-latin is re-spent by aquel pointing back at el, so it is not among them // HL-C128 step 3: +4 // HL-C128 step 4: +6 // HL-C128 step 5: no change -- ES-ETYMON-GERUND-NDUM is minted by ch236 and re-spent by the ch240 review, so it never enters the unspent set // HL-C128 step 7: +1 -- cum-latin, minted by conmigo and not yet re-spent // HL-C158: +4 -- the B1 travel rung (chapter 268) // HL-C159: +4 -- the B1 describe-experience rung (chapter 269) // HL-C160: +1 -- depende closes SPINE-EXPRESS-CONDITION, and B1 // spanish pre-A1 tranche: +35 lessons, +7 chapters (chapters 282-288) // FLOOR — content only grows; exact pins serialize parallel tranches
    expect(l.summary.underspentPercent).toBeGreaterThanOrEqual(89); // HL-C194: +16 Spanish pre-A1 words // HL-C128 step 2: +1 -- four new roots, none re-spent yet; the demonstrative review revisits the atoms but not the slugs // FLOOR — content only grows; exact pins serialize parallel tranches
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
