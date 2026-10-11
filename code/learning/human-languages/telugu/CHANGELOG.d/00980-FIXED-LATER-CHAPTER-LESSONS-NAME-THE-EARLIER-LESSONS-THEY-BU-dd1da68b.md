## Fixed — later-chapter lessons name the earlier lessons they build on

A lesson past chapter 1 with `prerequisites: []` is a root: the app unlocks it, and every lesson chained after it, on a fresh profile, and the validator credits it with knowing nothing. Each such lesson was read, and the ones that build on an earlier lesson now name it (human-language-data: `laterChapterLessonsWithoutPrerequisites` 50 → 5). Only the frontmatter changed: the book and the narration say what they said before.

- **TE-C02-peru** → TE-C01-namaskaram, the lesson its `reviews_of` says it resurfaces.
- **TE-C41-this** and **TE-C42-big** → TE-C40-second-pass-meal. Each says its word "with a word you already know after it", and that review holds the meal nouns closest behind it.
- **TE-C43-come** → TE-C32-raa and **TE-C44-look** → TE-C32-cuudu: each teaches a headword chapter 32 taught first.
- **TE-S157-letter-ba** → TE-S07-vowel-sign-aa: "the long-*ā* sign you have been reading since the greeting". Its anchor, బాగా, comes later in curriculum.json, so it cannot be named.
