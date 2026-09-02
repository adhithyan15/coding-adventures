import { loadEverything, loadTrackChapters, loadChapterPolicy } from "./dist/loader.js";
import { runChapterGates } from "./dist/chapters.js";
const e = loadEverything();
const r = runChapterGates({ registry: e.registry, lessons: e.lessons, books: e.books,
  trackChapters: loadTrackChapters(), policy: loadChapterPolicy() });
for (const f of r.findings.filter(f=>f.language==="kannada")) console.log(f.code, "| ch", f.chapter, "|", f.message);
