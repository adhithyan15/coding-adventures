### Fixed — a chapter's fingerprint now covers the capability the book prints

- `canonicalChapterHash` covered lessons only, so `chapters.json` was **invisible to
  the fingerprint**. CI still caught a stale chapter — `book-cli --check` compares
  full text — but `core/generated-book-hashes.json` came out **byte-identical** after
  a capability edit, so `language-ladder`'s `bookHashStatus` reported a genuinely
  stale `.tex` as *synced*. The README's claim that the fingerprint "detects drift
  between book and app inputs" was false for that input class; it is now true, and
  the sentence says exactly what is covered.
- **Only the two fields the book prints are hashed** (`canDo`, `payoff.summary`).
  Hashing the whole capability would make `payoff.note` — deliberately non-printed
  tooling prose — regenerate every chapter carrying one, churn with no reader-visible
  cause. A fingerprint covers what the artifact SHOWS, no more.
- The capability argument is **optional**, and the narration export passes none: it
  builds a spoken script from lessons alone, so a capability edit must not churn 789
  narration files that cannot have changed. Verified — this change touches 310 book
  chapters and zero narration files.
- Russian chapter 3, the one generated chapter with no capability, hashes exactly as
  before. Adding the opening did not renumber chapters that have no opening.
- **The browser app was updated in the same change, and had to be.** `language-ladder`
  reproduced only the lesson half via `combineLessonHashes` — correct while that WAS
  the whole fingerprint. Folding the capability in without giving it a seam would have
  turned "always synced" into **"always stale"**: the same broken signal, inverted, on
  every lesson in every chapter. Pre-push review caught it with 188 of its 189 tests
  failing. `combineChapterHash` is now exported over already-computed lesson hashes —
  the app has no `ParsedLesson`, since it globs lesson sources rather than using the
  Node-only loader — and the app globs `chapters.json` the same way it globs lessons.
- The printed check is gated on `canDo`, matching `chapterIntro`'s own condition
  exactly: a capability with no `canDo` prints nothing, so it must hash as though
  absent. Otherwise the fingerprint would claim a difference the reader cannot see.

