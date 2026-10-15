### Changed — Sanskrit's etymology gap pins fall to 0 and 0

`tests/etymology-gap-pin-owners/sanskrit.owner.ts` drops from `{ withoutHook: 1200, withoutVisible: 1099 }` to `{ withoutHook: 0, withoutVisible: 0 }`, and `EMPTY_ETYMOLOGY_BLOCK_PINS.sanskrit` in `tests/root-ledger.test.ts` drops from 12 to 0. With this, every track's etymology gap pins are 0.

- **What the lessons gained.** 1,200 hooks, 1,099 new "taken apart" blocks, and one origin sentence in each of the 12 empty blocks; the 91 lessons that already showed an origin had it checked before their hook was added.
- **Sources.** Monier-Williams 1899, CDIAL, DEDR, EtymDB 2.1 and the Dhātupāṭha, read from public data on GitHub; wording is our own, with the source and entry number cited.
- **Slugs.** Reused cross-track `sanskrit-*` slugs where the etymon is the same, gloss-suffixed slugs for homonyms. `check:root-slug-splits` reports no new split.
- **Review.** Six independent reviews checked every lesson: 167 corrected, about 129 of them fill errors (10–16% per range, mostly look-alike cognates taken from MW, descendants the cited CDIAL entry does not list, and dropped hedges) and the rest older hand-written text.
- **No code changed.** Only the pins and the owner's rationale moved.
