// ---------------------------------------------------------------------------
// Corpus-wide numeral probe (HL): which cardinals does each track teach, how
// far does the count reach, and which exam points sit behind it.
//
// TWO LAYERS, because neither alone is trustworthy.
//
//  1. THE INVENTORY LAYER is the one that can be cited. Every track has an A1
//     exam inventory whose points were derived, point by point, from the
//     DELE/PCIC-sourced Spanish set. The quantifier points name the numeral
//     demand and their probes name the atoms that would satisfy it, so
//     `measureExamCoverage` answers "does this track teach numbers" without
//     any guessing on my part.
//
//  2. THE CORPUS LAYER exists because an inventory point can be uncovered for
//     two different reasons -- nobody taught it, or nobody wrote the probe --
//     and only a corpus scan tells those apart. It matches on the HEAD of a
//     lesson's gloss ("six — the first number past five") rather than anywhere
//     in it, because "the two rows" and "three genders" appear in every script
//     runway in the corpus and a loose match reports 73 numeral lessons for a
//     track that teaches none.
// ---------------------------------------------------------------------------
// Run from anywhere:
//   node code/learning/human-languages/data/scripts/numeral-probe.mjs [out.json]
// Requires the human-language-data package to have been built (npx tsc).
const P = new URL("../../../../packages/typescript/human-language-data/dist/", import.meta.url).href;
const fs = await import("node:fs");
const { loadEverything, loadExamInventory, listExamInventories, defaultCurriculumRoot } = await import(P+"loader.js");
const { measureExamCoverage } = await import(P+"exam-inventory.js");
const { introducedAtoms, readingOrder } = await import(P+"ramp.js");

const root = defaultCurriculumRoot();
const { lessons } = loadEverything();

const WORDS = ["one","two","three","four","five","six","seven","eight","nine","ten",
  "eleven","twelve","thirteen","fourteen","fifteen","sixteen","seventeen","eighteen",
  "nineteen","twenty","thirty","forty","fifty","sixty","seventy","eighty","ninety",
  "hundred","thousand"];
const VALUE = {}; WORDS.forEach((w,i)=>{ VALUE[w] = i<20 ? i+1 : [30,40,50,60,70,80,90,100,1000][i-20]; });
// "one to five", "six to twenty", "1-10" etc. all appear as gloss heads
// A lesson TEACHES a numeral when its gloss head is made of nothing but number
// words and connectives. Matching a number ANYWHERE in the head reports
// "three pieces of Gurmukhi" and "five source-attested shopping words" as
// numeral lessons, which is how a track that teaches no numeral at all scored
// twenty of them on the first pass.
const CONNECTIVE = new Set(["the","a","numbers","number","numerals","numeral",
  "cardinal","cardinals","ordinal","ordinals","digit","digits","to","and",
  "through","counting","count","up","in","system"]);
const META = /\b(digits?|numerals?|cardinals?|ordinals?)\b/i;

function headIsNumeral(head) {
  const toks = head.toLowerCase().replace(/[\u2010-\u2015\u2212\u002D\u2013\u2014]/g, " ").replace(/[^0-9a-zA-Z\u0080-\uFFFF]+/g, " ").split(/[\s,]+/).filter(Boolean);
  if (toks.length === 0) return null;
  let sawNumber = false, best = 0;
  for (const t of toks) {
    const bare = t.replace(/[^a-z0-9]/g, "");
    if (bare === "") continue;
    if (VALUE[bare] !== undefined) { sawNumber = true; best = Math.max(best, VALUE[bare]); continue; }
    if (/^\d+$/.test(bare)) { sawNumber = true; best = Math.max(best, Number(bare)); continue; }
    if (CONNECTIVE.has(bare)) continue;
    return null;                       // a noun in the head: not a numeral lesson
  }
  if (!sawNumber && !META.test(head)) return null;
  return best;
}

function glossHead(g) {
  return (g ?? "").split(/\s+[—–-]\s+|\s*\(/)[0].trim();
}

const byLang = new Map();
for (const l of lessons) {
  if (!byLang.has(l.language)) byLang.set(l.language, []);
  byLang.get(l.language).push(l);
}

// Exam points that ARE the numeral demand, and points that WAIT on it.
//
// The hard part is that "number" is two different words in these labels. A
// third of every inventory's grammar column says "number: the plural" or
// "agreement in gender and number", which is the SINGULAR/PLURAL sense and has
// nothing to do with counting. So the numeral test matches numeral, cardinal,
// ordinal and digit -- the words that can only mean counting -- plus the two
// spellings ("numbers 0-100", "numbers beyond 100") that two tracks use
// instead, and then subtracts anything phrased as grammatical number.
const IS_NUM = /\b(cardinals?|ordinals?|numerals?|digits?)\b|\bnumbers?\s*[0-9]|\bnumbers beyond\b|\bcount aloud\b/i;
const GRAMMATICAL_NUMBER = /^number:|gender[, ]+(and )?number|number,? (and|or) case|agreement in .*\bnumber\b|marks no plural|\bplural\b/i;
const WAITS_ON = /\b(age|price|telephone|phone number|money|paying|personal data|the date|clock|time of day|measures?|house number)\b/i;

const out = [];
for (const [lang, ls] of [...byLang].sort()) {
  const ordered = [...ls].sort(readingOrder);
  const taught = [];
  let maxN = 0, metaOnly = 0;
  for (const l of ordered) {
    const r = l.realization;
    const head = glossHead(r.gloss);
    const v = headIsNumeral(head);
    if (v === null) continue;
    if (v > maxN) maxN = v;
    if (v === 0) metaOnly += 1;
    taught.push({ id: r.lessonId, head, value: v, atoms: [...introducedAtoms(l)] });
  }
  // inventory layer
  const inv = { points: [], covered: 0, enumerated: 0, waiting: [] };
  for (const { language, level } of listExamInventories(root)) {
    if (language !== lang) continue;
    const inventory = loadExamInventory(language, level, root);
    const cov = measureExamCoverage(inventory, ordered);
    inv.enumerated = cov.enumerated; inv.covered = cov.covered;
    for (const p of cov.points) {
      if (IS_NUM.test(p.label) && !GRAMMATICAL_NUMBER.test(p.label)) inv.points.push({ level, id: p.id, label: p.label, covered: p.covered, note: (p.note ?? "").slice(0,140) });
      else if (WAITS_ON.test(p.label) && !p.covered) inv.waiting.push({ level, id: p.id, label: p.label, note: (p.note ?? "").slice(0,120) });
    }
  }
  out.push({ lang, lessons: ordered.length, numeralLessons: taught.length, metaOnly, highest: maxN, taught, inv });
}
fs.writeFileSync(process.argv[2] ?? "/tmp/probe.json", JSON.stringify(out, null, 1));
console.log("track        lessons  numeral-lessons  highest  numeral-points(covered/all)  waiting");
for (const r of out) {
  const np = r.inv.points.length, nc = r.inv.points.filter(p=>p.covered).length;
  console.log(`${r.lang.padEnd(12)} ${String(r.lessons).padStart(5)} ${String(r.numeralLessons).padStart(14)} ${String(r.highest).padStart(9)} ${(nc+"/"+np).padStart(20)} ${String(r.inv.waiting.length).padStart(10)}`);
}
