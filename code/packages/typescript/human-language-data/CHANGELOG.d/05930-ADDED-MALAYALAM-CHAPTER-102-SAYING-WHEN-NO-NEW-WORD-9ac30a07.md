### Added — Malayalam chapter 102, saying when, with no new word in it

- `ML-A1-TIME-10` closes. Malayalam A1 coverage 208/243 -> **209/243 (86%)**, 34
  points unmapped. **The percentage holds** — 209/243 is 86.0 — recomputed, not
  carried.

#### The point's note was half stale, and the stale half was the case

`ML-A1-TIME-10` had stood open behind this note:

> Requires the **locative** on a time word, which is not taught. The parts are
> all present — days, hours, the **-il** suffix — and are never assembled.

**The parts half is right. The case is wrong.** Malayalam fixes a clock time
with the **dative**, not the locative — **രണ്ട് മണിക്ക്**, with no form in
**-ഇൽ** anywhere near it — and the dative is `ML-C06-dative-ikku`, taught at
sequence **320**, in the same chapter that gives the reader **എനിക്ക്**. The point
was never waiting on a suffix. It was waiting on a joint.

That is the third stale note in seven tranches, and it was caught by the
standing rule to verify a note before scoping from it rather than by any gate.
The note is corrected in place, not worked around.

#### One joint, and nothing else

| | | |
|---|---|---|
| **രണ്ട് മണിക്ക്** | *raṇṭŭ maṇikkŭ* | the hour takes **ക്ക്** |
| **തിങ്കളാഴ്ച** | *thiṅkaḷāzhcha* | the day takes **nothing** |

Both answer *when*. Both stand in the slot at the front of
**ഞാൻ പോകും** that `ML-C94-three-days` opened. Only one of them pays a toll to
get in, and the day-name is in good company: **ഇന്ന്** and **നാളെ** have stood
there bare since chapter 94.

**No word in this chapter is new.** The hour is chapter 18, the day-names are
chapter 10, **എത്ര** is chapter 81, the slot is chapter 94, and the ending is
chapter 6 — where it meant *to* and *for* and landed on **ജോലി** exactly as it
now lands on **മണി**. English is what spreads that one ending across three
prepositions; the lesson says so rather than presenting *at* as a new case.

| | | |
|---|---|---|
| **എത്ര ദൂരം** | *ethra dūraṁ* | ch. 81 — noun bare |
| **എത്ര രൂപ** | *ethra rūpa* | ch. 91 — noun bare |
| **എത്ര മണിക്ക്** | *ethra maṇikkŭ* | here — noun carries **ക്ക്** |

#### The witness list was wrong twice, and the second time was the correction

The draft said *"the third question you have asked with this word"*. A sweep
found `ML-C19-vayassu` at sequence 480 and the count became three named
witnesses. **Review then found `ML-C34-codikkuka` at sequence 760**, which
drills ***ethra maṇi?*, "what hour?"** in its own Guided Practice — so the
learner has been able to **ask the hour** for 2,930 sequence points.

That is not a missing row in a list. It is the lesson's thesis. The question
half of this point was **one ending short, not one phrase short**, and the
lesson is rewritten around that: **എത്ര മണി** is presented as already owned, and
the ending is the whole distance between *what time is it* and *at what time*.

**Two passes over the same sentence, and the count was wrong in both.** Naming
witnesses does not help when the sweep that finds them is keyed on script:
`ML-C34` gives the phrase in **romanization only**, so a script-keyed grep for
**എത്ര** returns `ML-C19`, `ML-C81` and `ML-C91` and misses it. That is the
`HL-C402` failure — a census keyed on one of a construct's two spellings —
arriving again in a different file.

The question word never changes. The ending belongs to the hour, and the answer
gives it back: **എത്ര മണിക്ക്? — രണ്ട് മണിക്ക്.**

#### Not one Malayalam token gains a first owner

All four content headwords are **multi-word**, which `continuity.ts` keeps
whole, so no earlier use turns into a forward reference. That is the trap
`lessons.d` records as *"a multi-word headword owns no single token, so a later
one-word lesson inherits every earlier use of it"*, and **തിങ്കളാഴ്ച** is
exactly the case it warns about: chapter 10 prints all seven day-names in its
table under a seven-word headword and owns **none of them singly**. A one-word
lesson for Monday would have converted chapter 10's own table into a forward
reference, the way ചേച്ചി did. It is taught here as a **use**, `type: grammar`,
inside a sentence.

**മണിക്ക് is the same question from the other side.** The reader can already
*build* it — **മണി** plus the chapter 6 ending — and `lessons.d` says a word the
corpus can already build must not be introduced later as a word. So the lesson
is about the ending's third sense, not about a word.

#### What the chapter refuses to claim

**That a day-name cannot take an ending.** It can, for other jobs. The lesson
says it does not *need* one for saying *when*, which is what the corpus teaches
and what is true. The temptation to write the tidier, stronger sentence is the
one that has cost this track four of its last six review rounds.

#### Romanizations are copied, not re-derived

From the **headword** romanization of the lesson that owns each word: **raṇṭŭ**
from `ML-C93-counted`, **ethra** from `ML-C81-dooram` *and* `ML-C91-ethra-rupa`
(both spell it *ethra*, not *etra*), **ñān pōkuṁ** from `ML-C94-three-days`, and
**thiṅkaḷāzhcha** from `ML-C10-azhcha`'s own *thiṅkaḷ* plus *āzhcha* — which
keeps chapter 10's older **zh** and **th** rather than the corpus's more recent
**ḻ** and **t**, because the learner meets the word in that table first and
should meet the same spelling twice.

**"From the owning lesson" is doing less work than it sounds like, and review
established the limit.** `ML-C18-mani`'s *headword* reads *maṇi* and its *body*
reads *mani*, in the very phrase this chapter builds on: `ML-C18-mani.md:73`
prints `**രണ്ട് മണി.** — "Two o'clock." (*raṇṭŭ mani*)`. And `ML-W07` prints
രണ്ട് as *randu* where `ML-C93-counted` gives *raṇṭŭ*. So the same word is
spelled two ways **inside one lesson** and two ways **across two lessons**, in
**ṇ/n** and **ṭ/d** — not only in the final vowel. `HL-C405` is widened
accordingly: a census keyed on *u* against *ŭ* would not have seen either of
these, which would have reproduced one level up exactly the failure that shard
was filed to avoid. (`ML-C34-codikkuka` writes *ethra maṇi*, with the **ṇ**,
which is why this chapter's spelling is not an invention.)

**The corpus romanizes word-final chandrakkala two ways** — *jōlikku* in chapter
6 against *raṇṭŭ* in chapter 93 — and this chapter does not resolve it. **ജോലിക്ക്**
is quoted in **script only** so the two spellings never stand side by side on one
page, and the inconsistency is filed rather than papered over.

#### One metric moves, and it is the recommended shape's price

`atomMeasurementBlindLessons` goes **9 → 10**. `ML-C102-day-and-hour` is a
synthesis lesson with an **empty `introduces` list**, and `ramp.ts:573` counts a
lesson that introduces no atom as unmeasurable. That shape is the one `lessons.d`
prescribes — a chapter's last lessons must not introduce atoms, or the atom can
never be revisited — so the blind count is what the prescription costs. Recorded
rather than engineered away.

#### One limit on the close

The point's label is **copular**: *"it is on Monday, it is at three"*. This
chapter produces no copular sentence. It locates an event with a **fronted
adjunct** — തിങ്കളാഴ്ച രണ്ട് മണിക്ക് ഞാൻ പോകും — which is the ordinary Malayalam
way of doing it; *"it is at two"* would need **-ആണ്** on the time phrase and is
not taught here. Both halves of the **function** are delivered, on a named day
and at a named hour. The copular framing is not, and the inventory note now says
so rather than claiming the label whole.

#### Verification

Twelve gates green. `npm run validate` 21/21 — after one fix: the *at what time*
lesson came in at **342 effective seconds against a 300 ceiling** and was cut
back rather than given a higher ceiling. The whole-token owner sweep over all
five new files returns three tokens with no headword owner, all of them
legitimate residue: **ക്ക്** (a metalinguistic ending fragment), **ജോലിക്ക്** (a
joined form built from two owned pieces, and printed verbatim in
`ML-C06-dative-ikku`'s own practice), and **ി** (owned by `ML-S03-vowel-sign-i`).
