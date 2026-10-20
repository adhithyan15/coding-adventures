## Fixed — later-chapter lessons name the earlier lessons they build on

A lesson past chapter 1 with `prerequisites: []` is a root: the app unlocks it, and every lesson chained after it, on a fresh profile, and the validator credits it with knowing nothing. Each such lesson was read, and the ones that build on an earlier lesson now name it (human-language-data: `laterChapterLessonsWithoutPrerequisites` 50 → 5). Only the frontmatter changed: the book and the narration say what they said before.

- **FR-C02-je** → FR-C01-bonjour, the lesson its `reviews_of` says it resurfaces.
- **FR-C03-aller** → FR-C02-comment: its recall asks what *Comment ça va?* literally asks. Its `reviews_of`, FR-C03-merci, comes after it in curriculum.json, so it cannot be named.
- **FR-C03-merci** → FR-C02-practice: "You can introduce yourself in French", and its `reviews_of`.
- **FR-C04-au-revoir** → FR-C03-practice and FR-C01-soir: "You can open a French conversation", its `reviews_of`, and the *-voir* sound note that cites *soir*.

