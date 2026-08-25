### Added - the grouped projection for `core/book-generation.json`, not yet enabled

- A second projection: several parallel arrays partitioned into **one file per
  language** rather than one file per element. Element-wise, that ledger would
  be 1,153 files nobody opens individually; what an author touches is "Spanish's
  slice of everything".
- Built, tested against the real ledger, and **deliberately absent from
  `SHARD_PLANS`** — `BOOK_GENERATION_PLAN` is exported and is one line from
  being enabled.
- **The spec's recorded blocker resolved itself.** §5.3 measured `targets` at 27
  runs for 23 languages and called for a one-time re-sort. At 1,007 entries it
  is 23 runs for 23 languages — the split runs for hindi, kannada, spanish and
  telugu closed as later tranches inserted into them. No re-sort needed, and a
  test pins the contiguity so a future append to the end of `targets` reopens
  the question loudly.
- **A different blocker replaced it.** `core/book-generation.json` does not
  round-trip at all: twelve `marwadi` entries in `targets` (lines 2911–2984) are
  indented two spaces deeper than canonical. 74 lines, identical line count,
  leading whitespace only, data deep-equal. The sharded rebuild is byte-identical
  to the *canonical* reserialization and differs from the *committed* file by
  exactly those 74 lines. Unlike the other two ledgers this cannot be worked
  around by skipping a track — it is one file shared by all 23 — so the whole
  ledger waits on a deliberate re-indent commit. A test states that blocker as an
  executable fact and **fails the day someone re-indents the file**, which is
  exactly when the plan should be enabled.

