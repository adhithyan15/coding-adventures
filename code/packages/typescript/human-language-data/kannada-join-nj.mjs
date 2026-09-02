import { loadEverything } from "./dist/loader.js";
import { measureContinuity } from "./dist/continuity.js";
const e = loadEverything();
const c = measureContinuity(e.lessons);
const NEW = new Set(process.argv.slice(2).flatMap(a => a.split(/\s+/)).filter(Boolean));
const rows = c.reinforcement.filter(r => r.language === "kannada" && !NEW.has(r.introducedBy) && r.introducedAt > 262 && r.missed.includes("R2"));
console.log("R2 newly judged:", rows.length);
for (const r of rows.slice(0,40)) console.log(" ", r.introducedAt, r.atom, r.introducedBy, r.missed.join(","), "revisits="+r.revisits);
