## Fixed — later-chapter lessons name the earlier lessons they build on

A lesson past chapter 1 with `prerequisites: []` is a root: the app unlocks it, and every lesson chained after it, on a fresh profile, and the validator credits it with knowing nothing. Each such lesson was read, and the ones that build on an earlier lesson now name it (human-language-data: `laterChapterLessonsWithoutPrerequisites` 50 → 5). Only the frontmatter changed: the book and the narration say what they said before.

- **TA-C02-peyar** → TA-C01-vanakkam, the lesson its `reviews_of` says it resurfaces.
- **TA-C08-sontham** → TA-C07-numbers-practice: the anchor-word warm-up asks for "the last thing you learned".
- **TA-C42-come** → TA-C32-vaa: both teach வா, and chapter 32 taught it first.
- **TA-S01-va** → TA-C01-vanakkam: "the first letter of the first word you learned".
- **TA-S101-letter-pa** stays a root. Its anchor words, பெயர் and the phrases built on it, come later in curriculum.json, so naming them would fail `curriculum-prerequisite-order`.
