# Etymology coverage — does every new word say where it comes from? (HL24 §9)

HL00 makes etymology the engine of the course, and the promise is made per
word. This counts the lessons that introduce a word (a `<XX>-LEX-` atom, or
`type: word` / `phrase`) and, of those, how many leave the question unanswered.

```ts
import { loadEverything, measureEtymologyCoverage } from "@coding-adventures/human-language-data";

const coverage = measureEtymologyCoverage(loadEverything().lessons);

coverage.summary.wordLessons;                         // 25682
coverage.summary.wordLessonsWithoutEtymologyHook;     // 22744
coverage.summary.wordLessonsWithoutVisibleEtymology;  // 22378 — nothing in the printed body
coverage.gaps[0];   // { lessonId, language, position, missingHook, missingVisible }
```

There are two counts because `etymology_hook` reaches the app and nothing that
prints. The book reader sees only the body, so a lesson with a perfect hook can
still tell them nothing. "Visible" is a conservative pattern match: a "taken
apart" block, an origin heading, or a line such as `día ← dies`, "from Old
French", "Proto-Germanic", or "Latin *october*". Its measured misses are written
in `src/etymology-coverage.ts`. Both counts are pinned per track in
`tests/root-ledger.test.ts`, so they may fall and may not rise.
