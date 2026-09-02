import { loadEverything } from "./dist/loader.js";
import { measureContinuity } from "./dist/continuity.js";
const e = loadEverything();
const c = measureContinuity(e.lessons);
const NEW = new Set(process.argv.slice(2).flatMap(a => a.split(/\s+/)).filter(Boolean));
const ka = c.reinforcement.filter(r => r.language === "kannada");
let mine = 0, old = 0;
const byWindowMine = {}, byWindowOld = {};
for (const r of ka) {
  const isNew = NEW.has(r.introducedBy);
  for (const w of r.missed) {
    if (isNew) { mine++; byWindowMine[w] = (byWindowMine[w]||0)+1; }
    else { old++; byWindowOld[w] = (byWindowOld[w]||0)+1; }
  }
}
console.log("kannada reinforcement misses: on NEW atoms", mine, JSON.stringify(byWindowMine));
console.log("                              on OLD atoms", old, JSON.stringify(byWindowOld));
console.log("--- worst new atoms");
for (const r of ka.filter(r=>NEW.has(r.introducedBy)).slice(0,60))
  console.log(" ", r.atom, r.introducedBy, "missed="+r.missed.join(","), "revisits="+r.revisits);
console.log("--- forward references (kannada)");
for (const f of c.forwardReferences.filter(f=>f.language==="kannada")) console.log(" ", JSON.stringify(f));
