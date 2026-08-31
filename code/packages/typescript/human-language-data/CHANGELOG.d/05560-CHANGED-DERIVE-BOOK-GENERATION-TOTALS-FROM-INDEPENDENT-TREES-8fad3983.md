### Changed

- The real book-generation ledger tests no longer pin corpus-wide totals. The
  reconstructed byte length, its SHA-256, and the literals `1_118` / `69` /
  `1_187` / `23` were rewritten by every chapter added in any language — one
  Spanish chapter in #13609 moved five lines here — which made this file a
  global write-lock. Two branches adding a chapter in two unrelated languages
  conflicted by construction, and the one that merged second carried a digest
  computed before the other landed, so a green PR broke main on merge.
- The chapter total is now proved against `<track>/chapters.d/`, compared as a
  SET in both directions. That tree is authored per track and the book pipeline
  only checks one direction (`requireChapterCapability` demands a capability for
  every ledger entry, never the reverse), so it is a genuine second opinion.
  This is stricter than the literal it replaces: a count only catches a change
  in size, while the set comparison also catches a swap, a rename, or a chapter
  moved between languages that preserves the total.
- `core/generated-book-hashes/` is deliberately NOT used as that source, though
  it looks like an obvious candidate. `book-cli` builds it by iterating
  `config.targets` straight out of this ledger, so comparing the two asserts
  `f(X) == X` — a bogus target plus the regeneration CI forces would move both
  sides together and keep the test green. The header comment records this so the
  comparison is not reintroduced.
- The handwritten count stays a literal, deliberately. Not every pinned number
  here was part of the write-lock: across the last 300 commits `69` moved
  exactly once, when owner sharding was introduced, while the targets/combined
  literals moved with ordinary chapter work. It is also the only thing that can
  see a chapter flipped between the two halves — `shard-cli` never mentions
  `handwritten`, and deriving the set from the authored `.tex` fails the same
  f(X) == X way, because the `% GENERATED FILE.` stamp is itself a function of
  `targets`, so regenerating destroys the witness. What that flip would cost is
  recorded in `data/scripts/handwritten_parity.py`: prose that lives only in the
  hand-written LaTeX, measured at 88 blocks across the six Indic tracks, deleted
  "with every gate still green". A number that only moves when a person
  deliberately changes the thing it measures is a tripwire, not a maintenance
  tax.
- The per-track `glossaries.d`, `answer-keys.d`, and `indexes.d` counts come
  from `core/languages.json` rather than a retyped `23`. The frozen digest is
  replaced by a per-owner canonical-bytes check, which still fails on formatting
  drift but no longer claims to detect changed chapter content — that belongs to
  the per-chapter hashes `check:books` verifies.
- A registered track missing its `chapters.d` now fails loudly instead of
  quietly contributing zero, and registry ids are validated before being joined
  onto a path, matching the checks every other reader in the package applies.
