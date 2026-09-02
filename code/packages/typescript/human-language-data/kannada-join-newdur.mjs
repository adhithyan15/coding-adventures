import { loadTrackLessons, defaultCurriculumRoot } from "./dist/loader.js";
import { estimateLessonDuration } from "./dist/report.js";
const NEW = new Set(process.argv.slice(2).flatMap(a=>a.split(/\s+/)).filter(Boolean));
const rows=[];
for (const l of loadTrackLessons("kannada", defaultCurriculumRoot())) {
  if (!NEW.has(l.realization.lessonId)) continue;
  const e = estimateLessonDuration(l);
  rows.push([e.computedSeconds, l.realization.lessonId, "words="+e.wordCount, "prompts="+e.promptCount, "repeat="+e.repeatCueCount]);
}
rows.sort((a,b)=>b[0]-a[0]);
for (const r of rows) console.log(r.join(" "));
