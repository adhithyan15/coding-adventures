# Engram ↔ Anki Desktop parity and migration fidelity

**Status:** Gap analysis (living document)
**Goal:** Engram has all the features of Anki Desktop, and a user can move
between the two in either direction without losing anything.

Engram is the proof that Mosaic ships real products. A framework demo that
almost works is not that proof, so this document is deliberately honest about
what is missing rather than about how much exists.

---

## 1. How to read this

Every row was checked against the code on `main`, not inferred from issue
history. Where a claim is *presence* rather than *depth* it says so, because a
symbol existing is not a feature working — that distinction has already cost
this project a stale wasm artifact, a Qt app that never compiled, and a release
that only worked at a domain root.

| Mark | Means |
|---|---|
| **Yes** | Implemented and exercised by tests |
| **Partial** | Real code, narrower than Anki's version |
| **No** | Absent |
| **Unverified** | Present in code; depth not established here |

---

## 2. Where Engram is strong

The data model and migration plumbing are the mature part, which is the
opposite of what a demo usually looks like.

| Area | State | Notes |
|---|---|---|
| Scheduling — FSRS | **Yes** | Dedicated `fsrs` crate; scheduler writes stability/difficulty |
| Scheduling — SM-2 | **Yes** | `sm2.rs`, ease factor bounds, interval growth |
| Learning steps / lapses | **Yes** | `learning_step_index`, lapse handling |
| Leeches | **Yes** | `LeechEvent`, `LeechAction`, tag-and-suspend |
| Suspend / bury | **Yes** | `suspended_at`, `buried_until` |
| Flags, tags, marking | **Yes** | `CardFlag`, tag edit in browser |
| Note types, templates | **Yes** | `NoteType`, `CardTemplate`, `FieldDef`, requirement modes |
| Cloze | **Yes** | Including front-side rendering |
| Type-in-the-answer | **Yes** | `TypeAnswerSpec` |
| Card browser + search | **Yes** | `SearchContext`, `CardSearchResult` |
| Deck options + presets | **Yes** | `DeckOptions`, `DeckOptionsPreset` |
| Stats | **Partial** | `DeckStats`, `ReviewHistorySummary`, `RatingCounts` — depth vs Anki's graphs unverified |
| CSV / TSV import | **Yes** | `csv.rs`, Anki-flavoured TSV options |
| `.apkg` legacy V11 | **Yes** | Read and write, zero-dependency |
| `.anki21b` / `.colpkg` | **Yes** | On our own zstd, browser included |
| Media reference tracking | **Yes** | `[sound:]`, `<img src>`, `<video poster>`, data-URI skip |

### 2.1 The round-trip mechanism already exists

`ExternalSourceRecord` stores the original Anki values for fields Engram does
not model — `updateSequenceNumber`, `originalDue`, original deck id, and the
card `data` blob — keyed by target and id. The V11 export writes back
`type`, `queue`, `due`, `ivl`, `factor`, `reps`, `lapses`, `left`, `odue`,
`odid`, `flags`, and `data`, preferring the preserved original where Engram has
no opinion.

That is a deliberate design for exactly the "move between the two" requirement,
and it is the single most important thing already built. **What is missing is
proof that it holds end to end** — see §4.1.

---

## 3. Where Engram is not Anki yet

Ordered by how badly each breaks a real user's migration.

### 3.1 Content rendering — the gaps that break imported decks

A user's existing Anki deck imports, and then individual cards are wrong. This
is worse than a refused import, because it looks like it worked.

| Feature | State | Why it matters |
|---|---|---|
| **LaTeX / MathJax** | **No** | Anki renders `\(...\)`, `[$]...[/$]`, and `[latex]` blocks. Maths and science decks are a large share of shared decks; without this their cards show raw markup |
| **Image occlusion** | **No** | A first-class Anki note type since 23.10. Decks using it import as notes whose cards cannot render |
| **Audio playback / TTS** | **No** | `[sound:]` references are *tracked and packaged* but nothing plays them. Language decks — the single most common Anki use — are silent |

### 3.2 Application features

| Feature | State | Notes |
|---|---|---|
| **General undo** | **Partial** | Only `UndoLastReview`. Anki undoes any operation, with a stack |
| **AnkiWeb sync** | **No** | The "sync" symbols in the tree are `sync_generated_cards_for_note`, unrelated. This is the largest single gap and may be out of scope — decide explicitly rather than by omission |
| **Filtered decks** | **Partial** | `custom_study_limit` / `custom_study_reschedule` exist; Anki's filtered decks with search-based rebuild are broader |
| **Add-ons** | **No** | Almost certainly out of scope; record the decision |
| **Note type editor** | **Unverified** | Screens exist (`Options`); depth vs Anki's field/template/styling editor not established |

### 3.3 Known live bugs blocking the promise

| Bug | Effect |
|---|---|
| #13933 | Deleting a note with no explicit id silently does nothing |
| #13671 | Media serialisation amplifies memory ~24×, capping browser import size |
| #13645 → UI47 | Host-capability effects have no completion path; confirm/disambiguate flows cannot work |

---

## 4. What to do, in order

The ordering principle: **prove what exists before adding to it**, then close
gaps by how badly they break an imported deck.

### 4.1 Prove the round trip (first, and it is verification not features)

The preservation mechanism exists and is untested end to end. Needed: a test
that takes a real `.apkg`, imports it, exports it, and asserts the second
archive is equivalent to the first — same notes, cards, scheduling, media,
tags, and deck configuration — with any divergence named.

This is first because every feature below is worth less if migration silently
loses data, and because it is the cheapest way to discover which of §3's gaps
actually corrupt data versus merely render poorly.

**There is currently no Anki oracle at all.** This is worse than it sounds and
was found while writing this document.

The one committed fixture, `golden-v11-filtered-media.apkg`, is **produced by
our own code**: a test builds a collection by inserting hand-chosen rows with
`rusqlite` and then calls our own `write_legacy_apkg`. Real SQLite writes the
bytes, so the *file format* is genuinely oracled — but the **Anki semantics are
entirely our own understanding of them**. Nothing in the repository was
produced by Anki.

So every `.apkg` import and export test validates our model against our model.
If our reading of what `queue = 2` means, or how `left` encodes learning steps,
or what belongs in the `col` table's `models` JSON is wrong, every test still
passes. That is precisely the circularity that let the zstd decoder ship
without Huffman support: **two halves wrong in the same way agree perfectly.**

The name `golden` makes this worse by implying external provenance the file
does not have.

Closing this needs a small corpus of genuinely Anki-produced `.apkg` files,
committed with their provenance recorded, covering at minimum: a review-scheduled
card, a card in learning, a cloze note, a deck with media, and a filtered deck.
Sourcing them is a decision — Anki's own test suite, a deck exported from a real
install, or a shared deck with a compatible licence — and it should be made
explicitly rather than by whoever gets there first.

### 4.2 Then, by user impact

1. **LaTeX / MathJax rendering** — largest share of decks affected, and it is a
   rendering feature, so it exercises Mosaic's component layer, which is the
   point of the exercise.
2. **Audio playback** — media is already tracked and packaged; this is a host
   capability, so it lands naturally on UI47's effect channel.
3. **Image occlusion** — a note type plus an editor; the largest of the three.
4. **General undo** — a core-model change, wide blast radius, no import impact.
5. **AnkiWeb sync** — decide in or out explicitly.

### 4.3 Design quality is scope, not polish

"Looks great" is part of the deliverable. Engram is the shop window for Mosaic,
so a visual pass belongs in this plan rather than after it. That pass needs its
own document; it is named here so it is not quietly dropped.

---

## 5. Honest summary

Engram is not a demo. Scheduling, note types, search, and the Anki package
formats are genuinely built, and the round-trip preservation design is better
than most projects manage.

The gap is narrower than it looks but sharper: **a user's real deck will import
and then show broken cards** if it contains maths, audio, or image occlusion —
and those three cover a large share of what people actually study. Closing them,
after proving the round trip holds, is what turns Engram from "releases" into
"a user can switch to this".
