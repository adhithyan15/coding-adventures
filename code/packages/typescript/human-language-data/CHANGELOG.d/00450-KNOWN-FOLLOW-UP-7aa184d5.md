### Known follow-up

- `canonicalChapterHash` covers lessons only, not the capability. CI still catches a
  stale chapter — `book-cli --check` compares full text, and the workflow's path
  filter includes `chapters.json` — but `core/generated-book-hashes.json` is
  byte-identical after a capability edit, so `language-ladder`'s `bookHashStatus`
  reports a genuinely stale `.tex` as synced. Folding the capability into the hash is
  the fix, and it regenerates every chapter, so it ships separately.

