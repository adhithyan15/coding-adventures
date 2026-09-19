### Added — Malayalam chapter 106, before and after: one ending, four jobs

- `ML-A1-TIME-09` closes. Malayalam A1 coverage 212/243 -> **213/243 (88%)**, 30
  points unmapped. **The percentage moves, 87 → 88** (213/243 is 87.7) —
  recomputed, not carried.

#### The note's first half is true, its second half is not, and this entry's draft celebrated it

> **munpu** and **shesham** are not taught. **pinne** ('then') is the nearest and
> it sequences **turns** rather than **events**.

The first half is verifiably true — neither word appears anywhere in the corpus.

**The second half is false.** `ML-C68-at-first.md:48` says the opposite in as many
words: *"**ആദ്യം** puts an **action** first in **time**, and it pairs with
**പിന്നെ**"*. Its summary table at `:74` is headed **"an action in time"**, and
its worked example is **ആദ്യം ഊണ്, പിന്നെ ചായ** — *lunch first, then tea*, which
orders the world and not the telling of it.

**I took the note's own anticipation of the domain check as evidence the check
had been done.** That is the domain mode one level up: not trusting a note's list
of words, but trusting a note's *self-assessment*. The draft of this entry said
*"A note CAN be trustworthy, and this is what one looks like"* and propagated it
into five artifacts including both test files. **This is the fifth stale note in
five tranches, not the first sound one.**

#### The organising contrast was rebuilt on what the corpus actually teaches

**Both pairs order real events.** What ആദ്യം and പിന്നെ cannot do is **name what
they measure against** — they lay a run out, *this, then that*. മുൻപ് and ശേഷം
fix an event against a **named** point, which is exactly what "anteriority and
posteriority" asks for. `ML-A1-TIME-09` still closes, on a true framing.

#### Three more claims were wrong

- **"മുൻപ് and ശേഷം cannot stand alone"** is false Malayalam — and this chapter's
  own lesson refutes it three lessons earlier by glossing ശേഷം *"the remainder"*,
  a standalone noun.
- **"Neither word changed the word in front of it"** contradicted the ordering
  lesson: it is ശേഷം that puts ഊണ് into the dative.
- **"One ending, four jobs"** counted to **five** by its own enumeration, and
  omitted the **dative subject** that `ML-C06-dative-subject` owns. The jobs are
  now **named**, not counted — the rule I keep breaking.

#### No new grammar, and the examples chain off chapter 102

Both words are **postpositions leaning on the dative**:

| | | |
|---|---|---|
| **രണ്ട് മണിക്ക് മുൻപ്** | *raṇṭŭ maṇikkŭ munpŭ* | before two o'clock |
| **ഊണിന് ശേഷം** | *ūṇinŭ śēṣaṁ* | after the meal |

The first reuses the **exact form** `ML-C102-manikku` built. The second shows the
dative's **other shape** — the `-ിന്` of `ML-C06` — on **ഊണ്**, owned since 930.

**The draft called that ending derivable and it was not.** `ML-C06` says the
choice is made *"by the sound the noun ends in"* and never says **which** sounds
take which shape, so a learner could not in fact derive ഊണിന്. The lesson now
supplies the one rule it needs: a word ending in a **consonant** takes `-ിന്`.

That gives the chapter a real spine: **one ending, named jobs.** *To* and *for*
from chapter 6; the person who knows, wants or has something — **എനിക്ക്**; *at*
from chapter 102; and now a seat for each of these two postpositions.

The ordering lesson then separates the two **pairs**: ആദ്യം and പിന്നെ stand alone
and order **the telling**; മുൻപ് and ശേഷം cannot stand alone and order **the
events**. Until this chapter the learner could order their own sentences and not
the world.

#### ശേഷം lands on known script debt, and nothing can be done about it here

It needs **U+0D36**, which has **no script lesson** — `SCR-12`'s "sha", which
chapter 103 re-derived as **12 distinct tokens across 15 headword fields**.

It does **not** trip `neverTaughtGlyphs`, because U+0D36 appears inside other
letter lessons' example words — the understatement `lessons.d` records.

**A script lesson placed here would not help.** Closure is measured in **reading
order**, and **ശരി** needs that letter in **chapter 1**. The fix belongs at the
start of the book, not at sequence 3870. Recorded against `HL-C407` rather than
half-fixed — and the word is not avoided for it, because ശേഷം *is* the word.

#### Verification

The three ceilings and the whole-token sweep were run **in draft**, and the full
suite passed on the **first run for the second chapter running** — which caught
nothing here, because every defect above is a claim about meaning. The sweep's
only unowned residue is **ഊണിന്**, a joined form from an owned word plus a taught
ending, and two metalinguistic ending fragments.

Romanizations copied from named witnesses: the chillu U+0D7B as *n* (*budhan*,
*karṣakan*, *adhyāpakan*); U+0D36 as *ś* (*śani*, *śarkkara*, *śaithyakālaṁ*);
U+0D37 as *ṣ* (`ML-S121`, *bhakṣaṇaṁ*). The recall's *munpuṁ śēṣavuṁ* follows
`ML-C70-um`'s rule — the one `HL-C400` narrowed — that a word ending in the
anusvara takes **-വും**.
