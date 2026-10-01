### Added — chapter titles and goals never name a bare article

- `tests/chapter-title-stubs.test.ts` fails if any chapter's title has a
  comma-separated part that is only "A", "An" or "The", or if its "I can say …"
  goal lists such an item. The vocabulary generators name each word by cutting
  its gloss at the first comma or bracket, so a gloss like "a (girl's) friend"
  left only "a". Hindi chapter 254 shipped that way, and is fixed in the same
  change.
