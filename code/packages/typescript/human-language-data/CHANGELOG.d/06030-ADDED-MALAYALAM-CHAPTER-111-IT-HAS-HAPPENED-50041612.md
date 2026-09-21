### Added — Malayalam chapter 111, it has happened

- `ML-A1-V-19` closes. Malayalam A1 coverage 218/243 -> **219/243 (90%)**, 24
  points unmapped.

#### The note was half stale and half right

> Not taught. Malayalam builds it with the past participle plus undu, and
> neither the participle nor the construction appears.

- **The participle half is false.** `ML-C90-past-participle` teaches **വന്ന** at
  sequence 3180, and it establishes the *regularity* that two of the three
  Spanish source points ask about (`A1-V-10` irregular participles, `A1-V-14`
  the regular one) — saying outright that the rule is *"about the letter and not
  about the tense"*.
- **The construction half is true.** A census for **-ഇട്ട്** returned **zero**
  files.

Opening both halves separately is the difference between closing this point
honestly and either over-claiming it on `ML-C90` alone or leaving it shut.

#### So the chapter costs no vocabulary

Both halves of the perfect were already owned: the past **വന്നു**
(`ML-C32-varuka`) and **ഉണ്ട്** (`ML-C32-undu`, sequence 650), whose gloss has
read *"[someone] has"* since it was written. Only the join was missing — and the
recall asks the learner outright how many new words the chapter cost.

#### Four shapes off one past form

| | |
|---|---|
| **വന്നു** | he came |
| **വന്ന** | that came — a vowel sign taken **off** |
| **വന്നിട്ട്** | having come — **-ഇട്ട്** put **on** |
| **വന്നിട്ടുണ്ട്** | he has come — that, plus **ഉണ്ട്** |

Each row is the row above it with one change. The contrast between *taking away*
and *adding* is what makes the new form memorable rather than arbitrary, and it
continues `ML-C90`'s own framing.

#### The example verb was chosen, not defaulted to

**വരുക** is the verb `ML-C90` already uses, so the chapter chains directly off
the lesson it extends. **പോകുക** was rejected: its past **പോയി** appears only
*inside* the multi-word headword **പോയി വരാം**, so it owns no single token and is
not independently taught — the same multi-word trap that narrowed chapter 18's
examples.

**അവൻ** *is* taught (`ML-C71-avan-aval`, sequence 2430) and is wired as a
dependency. Worth recording separately: `ML-A1-F-33`'s note still claims *"there
is no third-person pronoun"*, which this chapter's checking incidentally
disproves.

#### Catching up with two refactors

`main` moved 135 commits since the last Malayalam tranche, and two of them
changed how a chapter is wired:

- **Single-own spine membership** — path lists are gone from the spine file, so
  no spine edit is needed at all.
- **Single-own lesson membership** — `path` and `extension` shards must **not**
  store a derived `lessons` array. Membership now lives in one file per lesson
  under `malayalam/curriculum-membership.d/`, with `pathOrder` and extension
  `order` indexed **from zero**.

`generate:gentle-snapshots` no longer exists as a script; only
`check:gentle-snapshots` remains.

#### The migration guard moved, and why that is expected

`tests/curriculum-membership-shards.test.ts` hashes the **live** curriculum
graph, so its digest and its 7196 lesson count move whenever *any* track gains a
lesson — not only when the membership migration changes shape. Updated to 7200
and the new digest, with a comment saying so; the guard still fails loudly on any
other change to the public graph.

#### Verification

`npm run validate` 21/21, all twelve gates, the full suite (172 files, **2201
passed**, 1 skipped), `check-book-compile.sh --strict malayalam`, all six LaTeX
warning counters at zero, and the append-only shard check empty.
