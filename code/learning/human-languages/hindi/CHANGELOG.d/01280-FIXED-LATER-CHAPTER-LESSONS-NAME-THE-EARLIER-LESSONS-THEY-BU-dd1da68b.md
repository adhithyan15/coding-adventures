## Fixed — later-chapter lessons name the earlier lessons they build on

A lesson past chapter 1 with `prerequisites: []` is a root: the app unlocks it, and every lesson chained after it, on a fresh profile, and the validator credits it with knowing nothing. Each such lesson was read, and the ones that build on an earlier lesson now name it (human-language-data: `laterChapterLessonsWithoutPrerequisites` 50 → 5). Only the frontmatter changed: the book and the narration say what they said before.

- **HI-C02-naam** → HI-C01-namaste, the lesson its `reviews_of` says it resurfaces.
- **HI-C04-phir** → HI-C01-alvida: *phir* "turns a goodbye from 'farewell'", and *alvidā* is that farewell.
- **HI-C42-come** → HI-C05-bolta-hun: "add the endings you already know", the -ता / -ती / -ते that *boltā hū̃* introduces.
- **HI-S01-letter-ma** → HI-W01-na-ma: "न, म — the first two Devanagari letters" is one of the words it says you already say.
- **HI-S110-letter-ra** → HI-W01-shirorekha-na-ma: its first "you already say" word is शिरोरेखा. Its other anchor, मेरा, comes later in curriculum.json, so it cannot be named.
