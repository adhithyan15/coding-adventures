## Fixed — later-chapter lessons name the earlier lessons they build on

A lesson past chapter 1 with `prerequisites: []` is a root: the app unlocks it, and every lesson chained after it, on a fresh profile, and the validator credits it with knowing nothing. Each such lesson was read, and the ones that build on an earlier lesson now name it (human-language-data: `laterChapterLessonsWithoutPrerequisites` 50 → 5). Only the frontmatter changed: the book and the narration say what they said before.

- **IT-C02-stare** → IT-C02-come: "*Come stai?* is, at the root, 'how do you stand?'", and its `reviews_of`.
- **IT-C03-io** → IT-C02-practice: "*Io* sto bene — e tu?" is the exchange that practice teaches.
- **IT-C04-arrivederci** → IT-C02-practice: "You can open a conversation in Italian and ask how someone is", and its `reviews_of`.
- **IT-C06-numeri-1-5** → IT-C05-practice, the lesson its `reviews_of` says it resurfaces.

