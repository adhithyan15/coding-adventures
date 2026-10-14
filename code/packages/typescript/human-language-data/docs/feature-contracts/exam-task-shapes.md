# Exam task shapes (HL18)

`<track>/task-shapes/<level>.json` records the sourced performance target behind
an exam-ready claim: what the learner reads or hears, what they must write or
say, timing, interaction, replay, scoring and aids for all four skills. Unknown
source measurements remain explicit rather than being guessed.

```ts
import {
  buildTaskShapeBacklog,
  listTaskShapeInventories,
  loadLanguageRegistry,
  loadTaskShapeInventory,
} from "@coding-adventures/human-language-data";

const frenchA1 = loadTaskShapeInventory("french", "A1");
const germanA1 = loadTaskShapeInventory("german", "A1");
const latinA1 = loadTaskShapeInventory("latin", "A1");
const present = listTaskShapeInventories();
const missing = buildTaskShapeBacklog(
  loadLanguageRegistry().languages.map((track) => track.id),
  present,
);
```

The first five inventories are official DELE Spanish A1, official DELF French
A1, official Goethe German A1, a clearly labelled project-defined Latin A1
equivalent, and Avant STAMP 4S Arabic with a project mapping to Level 3 /
Novice-High in every skill. They are targets for later five-minute lesson
decomposition and mocks, not claims that any current book is pass-ready. DELE's
paired-skill rule, Latin's independent floors, Arabic's adaptive counts,
alternate DELF forms, open-ended lengths, and duration ranges remain distinct
rather than being collapsed into invented single values.
