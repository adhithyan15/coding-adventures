import { loadEverything } from "./dist/loader.js";
import { measureContinuity } from "./dist/continuity.js";
import { writeFileSync } from "node:fs";
const e = loadEverything();
const c = measureContinuity(e.lessons);
const NEW = new Set(process.argv.slice(2).flatMap(a => a.split(/\s+/)).filter(Boolean));
const byId = new Map();
for (const l of e.lessons) if (l.language === "kannada") byId.set(l.realization.lessonId, l);
const rows = [];
for (const r of c.reinforcement.filter(r => r.language === "kannada" && !NEW.has(r.introducedBy))) {
  if (!r.missed.includes("R4")) continue;
  const l = byId.get(r.introducedBy);
  rows.push({ atom: r.atom, by: r.introducedBy, at: r.introducedAt,
              headword: l?.realization.headword ?? "", rom: (l?.frontmatter?.romanization ?? "").toString(),
              type: l?.realization.type ?? "" });
}
rows.sort((a,b) => a.at - b.at);
writeFileSync(process.env.OUT, JSON.stringify(rows, null, 1));
console.log("R4-missing old atoms:", rows.length, "positions", rows[0]?.at, "..", rows.at(-1)?.at);
