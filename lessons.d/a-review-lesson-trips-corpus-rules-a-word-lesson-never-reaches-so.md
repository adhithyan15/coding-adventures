---
category: Repo policy / workflow reminders
---

# A review lesson trips corpus rules a word lesson never reaches, so run the full suite on the first one

A `review` lesson looks back across a whole book. That is its job, and it is
also why it meets rules ordinary content never approaches. Three were found in
one tranche, all of them past `validate` and caught only by `npm test`:

1. **`standalone-book` — never write "the course".** A book volume ships on its
   own, so its prose may not tell a reader they already learned something that
   lives in another volume. A review lesson reaches for exactly that phrasing
   ("the course has warned you", "the opening exchange of the whole course").
   Write **this book**, **these chapters**, or name the thing.
2. **`glyph-coverage` is per book.** A middle dot `·` used to list items
   compactly (`女 · 子`) renders in the Telugu, Persian and Bengali books and
   **not** in the Chinese one. A review lesson lists more items side by side
   than anything else, so it hits a font's limit first. Ordinary ASCII
   punctuation or a comma is safe everywhere.
3. **A headword still needs a `romanization:`.** Easy to skip on a review lesson,
   whose headword is usually a word already taught and so feels re-glossed
   already. `script-closure` holds several tracks at a ceiling of **zero**
   unromanised headwords, so one lesson breaks it.

**And check the segment against EVERY prerequisite, not the last one by
sequence.** A Bengali review lesson was filed on the segment holding
`BN-C04-practice`, the latest of its six prerequisites by sequence number. Two
others sat on segments ranked 0250 and 0310 against that segment's 0110:

```
bengali: BN-C02-practice must precede BN-R08-second-pass-four-exchanges
```

The rule was already written down after the previous tranche and was followed
for one prerequisite instead of all six. **Take the latest segment among all of
them**, by path-file rank, not by lesson sequence.

Before authoring for a track you have not touched, read
`tests/corpus/<track>/`. It is where a track's private conventions live, and two
kinds matter most: an activity ledger that demands exactly one `hl-activity`
per lesson (marwadi, french), and pinned reinforcement-window **positions**
(gujarati, marathi). The second is the dangerous one — `measureContinuity`
judges a window in lesson INDEXES, so inserting a review lesson between an atom
and its revisit can push that revisit out of its window. Place late in the
track there, and never re-pin a position to accommodate the insertion.
