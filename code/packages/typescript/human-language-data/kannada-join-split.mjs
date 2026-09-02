import { loadEverything } from "./dist/loader.js";
import { measureContinuity, REINFORCEMENT_WINDOWS } from "./dist/continuity.js";
const e = loadEverything();
const c = measureContinuity(e.lessons);
const OLD_LAST = 267;
const NEW = new Set(process.argv.slice(2).flatMap(a => a.split(/\s+/)).filter(Boolean));
const W = Object.fromEntries(REINFORCEMENT_WINDOWS.map(w => [w.name, w.from]));
let newlyJudged = 0, preexisting = 0, onNew = 0;
const njW = {}, peW = {};
for (const r of c.reinforcement.filter(r => r.language === "kannada")) {
  for (const w of r.missed) {
    if (NEW.has(r.introducedBy)) { onNew++; continue; }
    if (r.introducedAt + W[w] > OLD_LAST) { newlyJudged++; njW[w]=(njW[w]||0)+1; }
    else { preexisting++; peW[w]=(peW[w]||0)+1; }
  }
}
console.log("misses on atoms this tranche introduced:", onNew);
console.log("misses on OLD atoms, window judgeable BEFORE (real pre-existing debt):", preexisting, JSON.stringify(peW));
console.log("misses on OLD atoms, window NEWLY judgeable (debt this length exposed):", newlyJudged, JSON.stringify(njW));
console.log("total:", onNew + preexisting + newlyJudged);
