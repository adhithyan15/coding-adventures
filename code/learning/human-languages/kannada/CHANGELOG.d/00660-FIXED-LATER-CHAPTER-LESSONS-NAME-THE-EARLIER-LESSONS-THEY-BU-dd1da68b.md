## Fixed — later-chapter lessons name the earlier lessons they build on

A lesson past chapter 1 with `prerequisites: []` is a root: the app unlocks it, and every lesson chained after it, on a fresh profile, and the validator credits it with knowing nothing. Each such lesson was read, and the ones that build on an earlier lesson now name it (human-language-data: `laterChapterLessonsWithoutPrerequisites` 50 → 5). Only the frontmatter changed: the book and the narration say what they said before.

- **KA-C02-hesaru** → KA-C01-namaskara, the lesson its `reviews_of` says it resurfaces.
- **KA-S111-vowel-sign-e** stays a root. Its anchor words, ಹೆಸರು and ನಿಮ್ಮ ಹೆಸರು ಏನು?, come later in curriculum.json, so naming them would fail `curriculum-prerequisite-order`.
