import { loadTrackLessons, defaultCurriculumRoot } from "./dist/loader.js";
import { estimateLessonDuration } from "./dist/report.js";
const only = process.argv.slice(2);
const lessons = loadTrackLessons("kannada", defaultCurriculumRoot());
const rows = [];
for (const l of lessons) {
  const e = estimateLessonDuration(l);
  if (only.length && !only.includes(l.realization.lessonId)) continue;
  if (!only.length && e.computedSeconds < 250) continue;
  rows.push([e.computedSeconds, e.declaredSeconds, l.realization.lessonId, "words="+e.wordCount, "prompts="+e.promptCount, "pause="+e.explicitPauseSeconds].join(" "));
}
rows.sort();
console.log(rows.join("\n"));
console.log("total kannada lessons:", lessons.length);
