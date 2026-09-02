import { loadEverything } from "./dist/loader.js";
import { measureScriptClosure } from "./dist/script-closure.js";
const e = loadEverything();
const r = measureScriptClosure(e.lessons);
for (const t of r.tracks) if (["tamil","kannada","telugu","malayalam","sanskrit"].includes(t.language))
  console.log(t.language, "scriptLessons="+t.scriptLessons, "taught="+t.taughtGlyphs, "shown="+t.shownGlyphs, "neverTaught="+t.neverTaughtGlyphs, "violations="+t.violations);
