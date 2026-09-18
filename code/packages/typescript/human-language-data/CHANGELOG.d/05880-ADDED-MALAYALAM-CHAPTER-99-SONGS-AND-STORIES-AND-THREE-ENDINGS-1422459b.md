### Added — Malayalam chapter 99, songs and stories, and three endings that do three different things

- `ML-A1-LEX-47` closes. Malayalam A1 coverage 205/243 -> **206/243 (85%)**, 37
  points unmapped. **The rounded percentage moves, 84 → 85**, and was recomputed
  rather than carried over — a stale one shipped once before (ch94) and no gate
  reads that string.

#### The point's note was already stale before the chapter began

Second consecutive point for which this was true, after `LEX-37`'s. It said
*"Nothing taught — six Spanish points on one gap."* But `ML-C98-sinima`
(sequence 3500) had closed the **cinema** limb one chapter earlier, so the probe
wires that atom alongside the four new ones. Checked **before** drafting, and
the note is corrected on the record rather than quietly replaced.

All four limbs now have an exponent: **പാട്ട്** and **പാടുക** for music,
**നൃത്തം** for dance, **കഥ** for literature, **സിനിമ** for cinema.

#### No new grammar, by design

The three nouns were chosen so that each exercises a **different** one of the
three **ആണ്** outcomes the learner already owns:

| word | joined to **ആണ്** | what happens | taught in |
|---|---|---|---|
| **പാട്ട്** | **പാട്ടാണ്** | a sound is **given up** | `ML-C69-words`, and ch98's half-u rule |
| **നൃത്തം** | **നൃത്തമാണ്** | the **m** is **re-spelled** | ch89, ch96, ch97 |
| **കഥ** | **കഥയാണ്** | a **യ** is **added** | ch98, on **സിനിമ** |

**Given up, re-spelled, added** — three events, and the only thing telling them
apart is how the word ended. **പാട്ട്** matters most of the three: chapter 98
established that a word-final half-u takes nothing, and until now there was no
*new* word for that rule to land on.

#### Two inventions caught before the first commit

Both by checking the corpus rather than trusting the draft — the discipline
chapters 96 through 98 cost four review rounds to install.

- The dance example read **ഇത് സുന്ദരമായ നൃത്തമാണ്**. **`സുന്ദരമായ` is
  attested nowhere** — an attributive form the book has never taught, the same
  trap that kept **വൃത്തികെട്ട** out of chapter 97. Now **ഇത് നല്ല നൃത്തമാണ്**,
  using the attributive **നല്ല** that `ML-C98-sinima` already models.
- The romanization was written `nr̥tthaṁ`, with a ring below. The track
  romanizes the vocalic **ൃ** with a **dot** below in four independent
  witnesses — `suhṛttŭ`, `hṛdayaṁ`, `vṛścikaṁ`, `kṛtajñata`. Derived, not
  invented — **and then review caught the consonant in the same word.**

#### The word I "derived" was still wrong, in the letter beside the one I checked

`നൃത്തം` decomposes as `NA` + `VOCALIC R` + **`TA` + `VIRAMA` + `TA`** +
`ANUSVARA`. The conjunct is **ത്ത**, the geminate *unaspirated* dental — not
**ത്ഥ**. The track's own script lessons settle it: `ML-S08-letter-ta` gives
**ത** = `ta`, and `ML-S130-letter-tha` gives **ഥ** = `tha`, *"the breathy
partner of the ത"*. About fourteen corpus words spell **ത്ത** as `tt` against
two that do not, and `വിദ്യാർത്ഥി` → `vidyārtthi` is genuinely `TA + VIRAMA +
THA`, so the corpus is consistent and I was not.

**`nṛtthaṁ` wrote an unaspirated geminate as the aspirate — collapsing the exact
contrast `ML-S130-letter-tha` exists to teach.** It is **`nṛttaṁ`**.

The part worth keeping: the bullet above derived the *vowel* from `suhṛttŭ` —
and `suhṛttŭ` is `ത്ത` → `tt`, so the same witness settled the consonant too.
Checking one letter of a word and assuming the rest is what "derived, not
invented" was supposed to prevent.

#### The remembered glyph control was simply wrong

The remembered pre-check was *"U+0D7A **ൺ** must report NOT covered"*. It
reports **covered**, because `ML-S131-chillu-nn` owns it — **at sequence 144, in
the earliest script track**, so the control had not rotted recently: it had
never been right. A control that does not discriminate proves nothing, so the
check was re-run with a control **derived from the data** rather than recalled,
and every glyph in every candidate has a named script-lesson owner.

Every candidate was token-swept first — **പാട്ട്**, **പാടുക**, **സംഗീതം**,
**നൃത്തം**, **സാഹിത്യം**, **കഥ**, **കവിത**, **ചിത്രം**, **നടൻ**, **നടി** — all
with **zero** occurrences.

#### Two things chapter 98 learned the hard way, applied from the first draft

- **The recall's Grammar Lens was drafted four columns wide and narrowed to
  three before any test ran.** Chapter 98 shipped that defect, and only the
  corpus-wide narration refusal count caught it — which would have left an audio
  learner a placeholder where the evidence belongs. All five of this chapter's
  tables are ≤3 labelled columns and every one is read aloud.

#### Three overstatements and a miscount, all caught by review

- *"the chapter before last told you what those do"* — the half-u rule is in
  **chapter 98**, the chapter *before*. This PR's own ledger and changelog said
  so while the lesson said otherwise.
- *"the third join, and the book has had it longest"* — **false.**
  **എന്താണ്** is at sequence **130** and **സുഖമാണോ** at **170**, so the half-u
  join is the older one, and `ML-C98-sinima` says **എന്ത്** has welded *"since
  your very first question"*. Now *"the one the book explained to you first"*,
  which is what was meant and is true.
- *"the book has not shown you a third"* noun-beside-verb pair — refuted by
  `ML-C33-vaayikkuka` (*"the noun beside it is **വായന**"*) and
  `ML-C34-codikkuka` (*"noun beside it is **ചോദ്യം**"*). Narrowed to the claim
  actually meant: no other pair changes the consonant's weight.
- The recall said **four** words met **ആണ്** across the two chapters. It is
  **five** — **കളി** was missing. It is now a fifth row, and it earns its place:
  **ഇ** and **അ** are different vowels both choosing **യ**, which is the narrow
  rule chapter 98 spent four rounds arriving at.

#### The sweep rule caught its own chapter

Chapter 98's follow-up produced the rule *"whenever `ML-Cnn-x` is narrowed,
`ML-Rnn-*` is a site until checked"* — and this review found an **eleventh**
`യ`-glide site that the round-four sweep walked straight past:
**`ML-C98-kali`**, chapter 98's own first lesson, still crediting the **യ** to
*"the word ends in a vowel"*. Untouched by the round-four fix, absent from the
ten-name list, absent from `HL-C402`, and contradicted by `ML-C98-sinima` two
lessons later. Narrowed here to name **ഇ**.
- **The `പാട്ട്`/`പാടുക` pair states a relation between two named words and
  refuses to generalise it.** The noun's long **ṭṭ** thins to a single **ṭ** in
  the verb, and the lesson says in as many words not to look for that in every
  noun beside a verb — because **കളി**/**കളിക്കുക**, one chapter earlier, does
  not do it.

#### Literature closes on കഥ, not സാഹിത്യം

**സാഹിത്യം** is abstract and would need **കവിത** or a genre word beside it to
buy anything at A1. **കഥ** is concrete, glyph-clean and takes the **യ** glide
the learner just met.
