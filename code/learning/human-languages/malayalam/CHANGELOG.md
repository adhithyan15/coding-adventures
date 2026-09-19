# Changelog

## Weekday comparison repair — HL-C406

Chapter 10 no longer prints Malayalam **ആഴ്ച** in Tamil letters or glosses it
as "day." Its comparison now says exactly what the two weekday builders mean:
Tamil **கிழமை** is "day of the week," while Malayalam **ആഴ്ച** is "week."
The lesson keeps the cautious note that the two words are not confirmed
cognates and does not promote the comparison word into a standalone headword.

## Romanization consistency — HL-C405

The lesson corpus now uses one letter-by-letter convention for recurring
Malayalam forms. The pass began with chapter 102's visible contradiction —
chapter 6 wrote **ജോലിക്ക്** as *jōlikku* while the new clock-time chapter wrote
the same ending in **മണിക്ക്** as *maṇikkŭ* — and then censused headwords and
native/romanized body pairs without presupposing which Latin spelling was wrong.

That census also found **പേര്** as *pēru/pēr*, **മണി** as *mani/maṇi*,
**ചായ** as *chaaya/chāya/cāya*, and **തിങ്കളാഴ്ച** as
*thiṅkaḷāzhcha/tiṅkaḷāḻca*. They now follow the convention already used by the
later chapters: final chandrakkala is **ŭ**, anusvara is **ṁ**, retroflex and
palatal letters keep their diacritics, and **ച / ഴ / ത** are **c / ḻ / t**
unless the Malayalam letter itself is aspirated.

The pass changes romanization, not Malayalam, meaning, sequencing or knowledge
ownership. Comparisons with Tamil, Telugu and Kannada keep those languages'
forms; only Malayalam's side of each comparison was normalised. A focused
corpus test now pins fourteen representative owners and the three body sites
that originally disagreed.

## Chapter 92 — eating out

`ML-A1-LEX-55` closes. Coverage **198/243 → 199/243 (82%)**.

Its blocker was named on the point: *"with the missing money vocabulary"* it was
out of reach, and chapter 91 supplied that. The rest of the note was right too —
the track taught **the meal at home** and no word for a place that serves one.

### A category the book never had

You owned **അരി** (raw rice), **ചോറ്** (cooked rice) and **ഊണ്** (a meal), and
**no name for the category all three belong to.**

| | |
|---|---|
| **അരി** | raw rice |
| **ചോറ്** | cooked rice |
| **ഊണ്** | a meal |
| **ഭക്ഷണം** | **food** — all of it |

That is what a vocabulary grown from the particular outward tends to leave out,
and it is worth naming when it happens: **a course can leave a learner able to
order lunch and unable to say the word *food*.**

### The two borrowings are the chapter's real lesson

| | came from English | means here |
|---|---|---|
| **ഹോട്ടൽ** | *hotel* | **a place to eat** |
| **ബിൽ** | *bill* | the bill |

**One changed and one did not**, and that is exactly why they sit in the same
chapter. In Kerala a **ഹോട്ടൽ** is ordinarily somewhere you go to eat — **a bed
is not what the word promises.**

**A word that crosses into another language stops belonging to the one it came
from.** It keeps whichever part of its meaning the borrowers needed and lets the
rest go. So the habit taught is **check each borrowing, do not trust the
family**: recognising the English inside a word tells you how to say it and
**nothing reliable about what it means.**

The book has met well-behaved borrowings before — **കാപ്പി** is coffee,
**മേശ** came from Portuguese. **ഹോട്ടൽ** is the first that would mislead a
reader who trusted the source.

### The scene ends with a question already owned

> **ഹോട്ടലിൽ** — at the eating place.
> **ഭക്ഷണം** — food.
> **ബിൽ. എത്ര രൂപ?** — the bill. How many rupees?

**Three new words**, and the question closing it is chapter 91's, unchanged.

### Verified before writing

**ഭക്ഷണം**, **ഹോട്ടൽ** and **ബിൽ** each returned **zero** files. Every Malayalam
character in them was checked against `data/scripts/malayalam.json` **before
writing**, after `HL-C398` — and the check itself was validated first against a
glyph known to be missing, so a clean result meant something.

The full suite passed with no gate firing.

### Metrics

| metric | before → after |
|---|---|
| exam-point coverage | 198/243 → **199/243 (82%)** |
| the lexicon column | 35/55 → **36/55** |
| `forwardReferences` | unchanged |

## Chapter 91 — what it costs

`ML-A1-LEX-30` and `ML-A1-LEX-31` both close. Coverage **196/243 → 198/243
(81%)**.

The two are **one situation**, and the notes said so: *"Three Spanish points, one
gap, and it blocks the whole shopping scenario"*; *"with ML-A1-LEX-30 this makes
shopping unreachable."*

### Five words

| | |
|---|---|
| **കട** *kaṭa* | a shop |
| **വില** *vila* | price |
| **പണം** *paṇaṁ* | money |
| **രൂപ** *rūpa* | a rupee |
| **വാങ്ങുക** *vāṅṅuka* | to buy |

### What made it cheap was chapter 88

The price question is **എത്ര രൂപ?** — and **എത്ര** was lifted out of the age
question back in chapter 81. **The question is two words and one of them was
already owned.**

**The answer is the part that had been impossible.** Before the tens were taught,
the count stopped at twenty and **every price above it was unsayable** — exactly
what `ML-A1-NUM-04`'s note had warned: *"Prices, ages over twenty and years are
all out of reach."*

| | |
|---|---|
| **അമ്പത് രൂപ** | fifty rupees |
| **നൂറ് രൂപ** | a hundred rupees |

So this chapter only had to **name the unit being counted.**

### The rules underneath all held

- **കടയിൽ** takes a **-യ-** to join, because the word ends in a vowel — the same
  choice the owner-ending makes, decided by the same thing.
- **പണം** ends in **ം**, which tells the learner in advance how it will behave
  when something attaches.
- **ഞാൻ പുസ്തകം വാങ്ങുന്നു** — the object stays **bare**. **പുസ്തകം** takes no
  object ending because it is a thing and not a person. **The verb changed and
  chapter 87's rule did not.**

### The accounting the recall makes

Of the four lines of the finished scene, **three words were new.** The in-ending,
the question word, the number and the bare-object rule had all arrived earlier
and were waiting.

**A vocabulary chapter is cheap when the grammar under it has already been
built**, and expensive when it has not. This one was cheap, and the counting
chapter is why.

### Verified before writing

**പണം**, **വില**, **കട**, **രൂപ** and **വാങ്ങുക** each returned **zero** files.

The full suite passed on the first run — no gate fired.

### Metrics

| metric | before → after |
|---|---|
| exam-point coverage | 196/243 → **198/243 (81%)** |
| the lexicon column | 33/55 → **35/55** |
| `forwardReferences` | unchanged |

## Chapter 90 — a clause in front

`ML-A1-PRON-08` and `ML-A1-JOIN-10` both close. Coverage **194/243 → 196/243
(81%)**.

The file called this *"structurally the biggest gap"* in it. **The gap was real.
The size was the surprise.**

### Measured first

A sweep for attributive **-ഉന്ന** forms across the whole corpus returned
**zero**, while **sixteen distinct -ഉന്നു present forms** were already in use. So
the construction was genuinely absent — and the material for it was already in
the learner's mouth.

### One vowel sign

| | |
|---|---|
| **പോകുന്നു** *pōkunnu* | he goes — a whole sentence |
| **പോകുന്ന** *pōkunna* | **that goes** — describes something |

**The ു comes off the last letter**, leaving it with its own built-in *a*. That
is the entire change, and it is visible on the page.

The rule is about the **letter**, not the tense. The past does it too:

| | |
|---|---|
| **വന്നു** *vannu* | he came |
| **വന്ന** *vanna* | **that came** |

### The position was already in use

**പോകുന്ന കുട്ടി** — the child who is going. Read that against **നല്ല കുട്ടി**,
*a good child*: **they are built the same way.** A quality stands in front of
what it describes, and so does a verb once it has dropped that vowel sign.

**Malayalam needed no new arrangement to let a verb describe a noun.** It reuses
the slot the adjectives have held since chapter 42.

### Two things differ from English, and the second is the one to watch

> **ഞാൻ വായിക്കുന്ന പുസ്തകം** — the book I read.

| | |
|---|---|
| English | the book **that** I read |
| Malayalam | **ഞാൻ വായിക്കുന്ന** പുസ്തകം |

**The clause moved to the front.** And **the joining word disappeared.** English
needs *that*, *which* or *who*; **Malayalam has no such word and needs none**,
because the missing vowel sign already says the verb is describing, and the noun
after it is what is described.

So the lesson names the habit to drop: **there is nothing to translate *that*
into.** Build the clause, take the vowel sign off, put the noun after it.

### What it cost

**No new vocabulary at all.** Every verb and noun in the chapter is taught, and
the describing form is a taught verb form with one sign removed.

The largest thing the track was missing came to **one vowel sign** — because the
position it needed was already occupied by the adjectives, and the verb forms
were already being produced sixteen ways over.

### Metrics

| metric | before → after |
|---|---|
| exam-point coverage | 194/243 → **196/243 (81%)** |
| the joining column | 8/11 → **9/11** |
| new words taught | **none** |
| `forwardReferences` | unchanged |

`info-dump` fired at 33 against 32 on a *"the rule is about…"* opener. Rewritten,
not pinned.

## Chapter 89 — what you think of it

`ML-A1-F-28` closes. Coverage **193/243 → 194/243 (80%)**.

### The point's note was substantially wrong, and that is the story

`ML-A1-F-28` was flagged in the inventory as *"the single highest-value fix in
the file after the coordinator"*, on the strength of this: *"'I think that ...'
needs ennu plus cintikkuka."*

**Both are taught.** **എന്ന്** is chapter 72's quoting marker and
**ചിന്തിക്കുക** is chapter 33's verb — and `ML-C72-ennu-more` **already pairs
them explicitly.** Its worked example is literally:

> *"enikku malayāḷaṁ aṟiyāṁ" ennŭ ñān cintikkunnu* — 'I think I know Malayalam.'

So the assembly the point asked for was **on the page before this chapter was
written.** What was genuinely missing was the other half of the note, and there
it was right: **നല്ലത്** and **മോശം** both returned **zero files**.

**The chapter therefore teaches two words and no grammar.**

### A quality with nothing to lean on

**നല്ല** has always needed a noun in front of. You could say *a good book* and
not *the good one*.

| | |
|---|---|
| **നല്ല** | good — needs a noun after it |
| **നല്ലത്** | **a good one** — stands alone |

And the ending is not arbitrary: **അത്** is the word for *that*, owned since the
pointing chapter, so **നല്ലത്** is *the that-which-is-good*. **The sense fits the
shape**, and it works on **every quality the book has given** — *valiyathŭ*,
*ceṟiyathŭ*, *puthiyathŭ*, *paḻayathŭ* — with no exceptions among them.

### Five qualities and not one of them negative

**മോശം** fills a hole that had been there since chapter 42: a thing could be
good, big, small, new or old, and there was **no way to say it was bad** — half
of every opinion anybody offers.

**മോശമാണ്**, *it is bad*. The **ം** becomes **മ** before the vowel of the *is*
word, which is the same habit of changing shape before something attaches that
the learner met on **കേരളം** and on the tens.

### The opinion needed nothing new

> **അത് നല്ലതാണ് എന്ന് ഞാൻ ചിന്തിക്കുന്നു.** — I think it is good.

| piece | new? |
|---|---|
| **അത് നല്ലതാണ്** | the two words of this chapter |
| **എന്ന്** | **no** |
| **ഞാൻ ചിന്തിക്കുന്നു** | **no** |

**The quoting chapter did the hard part.** The lesson says so rather than hiding
it, because it is the interesting part: **machinery is often in place before
there is anything worth carrying with it**, and the chapter that finally uses it
can be very small.

### Metrics

| metric | before → after |
|---|---|
| exam-point coverage | 193/243 → **194/243 (80%)** |
| new words taught | **two** |
| new grammar | **none** |
| `forwardReferences` | unchanged |

The full suite passed on the first run — no gate fired on this chapter.

## Chapter 88 — counting past twenty

`ML-A1-NUM-04` closes. Coverage **192/243 → 193/243 (79%)**.

The note was plain: *"The count stops at twenty. Prices, ages over twenty and
years are all out of reach."*

### The pattern was already on the page

**ഇരുപത്** had been explained since the numbers chapter as *iru* + *pathŭ*,
**two-tens**. Every ten from thirty up is that same word with a different digit
on the front:

| | | built on |
|---|---|---|
| **മുപ്പത്** | 30 | three |
| **നാൽപത്** | 40 | four |
| **അമ്പത്** | 50 | five |
| **അറുപത്** | 60 | six |
| **എഴുപത്** | 70 | seven |

**-പത്-** is in all of them.

But the digits appear in **older short forms that survive only in compounds** —
*āṟŭ* shows up as **അറു-**, *ēḻŭ* as **എഴു-**, and **അമ്പത് has worn furthest of
all**, its link to *five* real and no longer visible. So the lesson says what
kind of pattern this is: **one to read with, not to build with.**

### Ninety breaks the run, so a hundred comes first

**തൊണ്ണൂറ്** has no **-പത്-** in it. What it has is **നൂറ്** — a hundred —
sitting at its end.

| | built from |
|---|---|
| *eṇpathŭ* 80 | eight **tens** |
| **തൊണ്ണൂറ്** 90 | a **hundred**, reduced |
| **നൂറ്** 100 | itself |

**Ninety is named from above**, where the other tens are named from below. That
is why the lesson teaches a hundred **first**: learn *toṇṇūṟŭ* after *nūṟŭ* and
it is one irregular word with a visible reason; learn it before, and it is noise.

### The compound rule is a stem change the learner has met

**ഇരുപത്തിയൊന്ന്** — twenty-one. **The ten does not keep its standing-alone
shape**; it takes **-ത്തി-** and the digit follows, in the plain order English
also uses.

That is the third thing in this book to do it: **കേരളം** became **കേരളത്തി**ൽ
before its ending. **A word wears one shape alone and another before something
else** — worth expecting from here rather than learning case by case.

### Eighty is taught by ear, and why

**എൺപത്** needs the chillu **ൺ**. `ML-S131-chillu-nn` teaches that letter at
sequence 144 — but **`data/scripts/malayalam.json` does not list it**, so any
lesson that writes it trips the `uncovered-glyphs` gate. Eighty is simply the
first word the curriculum has wanted that contains it.

Adding the letter properly means **a sourced stroke-order citation** — the other
four chillus are each pinned in `malayalam.evidence.ts` to a named animation with
frame timings — and **a citation cannot be guessed**. It is also script-owner
territory by design.

So eighty is given **in romanization only**, with the lesson saying its written
shape comes when its letter does. **That is this track's own precedent**:
`ML-C07-numbers-6-10` reads *"hear and say six to ten before meeting their
written forms."*

Recorded as backlog item **`HL-C398`**, with the fix and a note to check whether
any other taught letter is missing from its script's inventory — **the failure is
silent until some lesson happens to need the glyph.**

### `ML-A1-NUM-08` updated

Its numeric blocker is gone: a learner can now count to a hundred. What remains
is genuinely the **units** — no measure of length, no measure of weight — which
is a vocabulary chapter on its own rather than something waiting on the numerals.

### Metrics

| metric | before → after |
|---|---|
| exam-point coverage | 192/243 → **193/243 (79%)** |
| the numerals column | 7/9 → **8/9** |
| `forwardReferences` | unchanged |

## Chapter 87 — who it happens to

`ML-A1-CASE-04` closes, and **the case column is complete at 7/7**. Coverage
**191/243 → 192/243 (79%)**.

**No new vocabulary at all.**

### An ending that places nothing

Every case ending so far put a noun somewhere — **in** a place, **from** a place,
**owning** something, **with** somebody. This one does something else. **It says
what role the noun has.**

| | |
|---|---|
| **അധ്യാപകൻ** | a teacher — the one doing |
| **അധ്യാപകനെ** | the teacher — the one it is done **to** |

**ഞാൻ അധ്യാപകനെ കാണുന്നു** — I see the teacher. **The doer wears nothing.**

English separates *I see the teacher* from *the teacher sees me* **only by which
word came first.** Malayalam marks it on the word, so a marked object is doing
work English has to spend its word order on — and the marking survives wherever
the word stands.

### The animacy rule is the part worth the chapter

| | |
|---|---|
| **ഞാൻ കുട്ടിയെ കാണുന്നു** | I see the child — **ending** |
| **ഞാൻ പുസ്തകം വായിക്കുന്നു** | I read the book — **no ending** |

Both are objects, both sit before the verb. **Being the object is not what
decides it. What decides it is the kind of thing the object is** — a living
thing acted on takes **-എ**, a lifeless one ordinarily does not.

That is a question **no European language the reader is likely to know asks in
quite that way**, and it is the reason the chapter exists rather than being a
footnote to the case endings.

How it attaches is nothing new: **a vowel-final word takes a -യ- to join, and a
word ending in the standalone ൻ opens it back into a plain ന.** The same last
sound that decided the owner-ending decides this one.

### The pronoun costs nothing, and that is a pattern now

English keeps a second form — *he* becomes **him**. Spanish goes further and
shrinks the object into a small unstressed piece that leans on the verb and moves
around the sentence. **Malayalam does neither**:

**അവൻ** → **അവനെ**, by the same rule as **അധ്യാപകൻ** → **അധ്യാപകനെ**.

So the list of things to memorise here is **empty**. The recall states the
generalisation, which four chapters now support: **where a European language
keeps a special form, Malayalam usually keeps the ordinary word and adds the
ordinary ending.**

### Verified before writing

Every word in the chapter is already taught — **അധ്യാപകൻ**, **കുട്ടി**,
**പുസ്തകം**, **അവൻ**, **കാണുക**, **വായിക്കുക**. **അവനെ** and the other marked
forms each returned **zero** files.

`info-dump` fired at 35 against 32, on a heading reading *"the rule is about…"*,
an *always* in a practice line, and a *"The rule is not…"* opener. All three
rewritten, none pinned.

### Metrics

| metric | before → after |
|---|---|
| exam-point coverage | 191/243 → **192/243 (79%)** |
| the case column | 6/7 → **7/7 — complete** |
| new words taught | **none** |
| `forwardReferences` | unchanged |

## Chapter 86 — two kinds of with

`ML-A1-CASE-05` closes. Coverage **190/243 → 191/243 (79%)**.

The note named both halves: *"Neither -aal nor oppam is taught, so 'with a
friend' and 'by bus' are both out of reach although suhruthu is taught."* Both
close here, and the chapter's point is that **they are not the same
relationship.**

### English asks one question, Malayalam asks two

*I went **with** a friend.* *I cut it **with** a knife.* One English word, and
**two relationships that have nothing in common** — a companion and a tool.

| | | |
|---|---|---|
| **company** | **എന്റെ ഒപ്പം** / **എന്റെ കൂടെ** | after the owner-form, standing apart |
| **instrument** | **കത്തിയാൽ** | glued onto the noun |

So the useful question is never *how do I say with*. It is **which kind of
*with* this is.**

### The company words cost no new arrangement

**ഒപ്പം** and **കൂടെ** stand after the owner-form — exactly where **മുകളിൽ**
stood after *the chair's*:

| | |
|---|---|
| **കസേരയുടെ മുകളിൽ** | on top of the chair |
| **എന്റെ ഒപ്പം** | together with me |

**The slot does not care whether what follows it is a position or a company.**
That is the return the place-words chapter earned by spending its first lesson
on the pattern.

The pair splits by register the way the degree words did — **ഒപ്പം** to the
page, **കൂടെ** to the room.

### The instrument ending is one the learner already owns

**ആൽ** was taught in the conditional chapter, on a **verb**, meaning *if*. Put
it on a **noun** and it means *by means of*.

| what it lands on | what it does |
|---|---|
| a **verb** — *vannāl* | **if** he comes |
| a **noun** — *kattiyāl* | **by means of** a knife |

**The ending is identical and the meanings are unrelated.** So the lesson teaches
the habit rather than the form: **look at the host before you read the ending.**
One ending doing two unrelated jobs is ordinary here rather than exceptional.

### A glyph the book could not print

The full suite failed on `glyph-coverage`, not on anything a reader would call a
mistake: the romanization of **സുഹൃത്ത്** had been written with **r + U+0325**,
a combining ring below, which **the book font cannot render**.

`ML-C35-suhruthu` already romanizes it **suhṛttŭ**, with **U+1E5B**, a single
precomposed character. Three occurrences corrected to match.

Worth keeping: **the corpus already had the right answer**, and the failure was a
new file spelling a sound its own track had spelled correctly for fifty chapters.
Copy the romanization from the lesson that owns the word rather than typing it
again.

### Verified before writing

**ഒപ്പം** and **കൂടെ** each returned **zero** files. The instrument example uses
**കത്തി**, which is taught — **no vehicle word was invented for "by bus"**, even
though the point's own note mentions it.

### Metrics

| metric | before → after |
|---|---|
| exam-point coverage | 190/243 → **191/243 (79%)** |
| the case column | 5/7 → **6/7** |
| `forwardReferences` | unchanged |

## Chapter 85 — held sounds

`ML-A1-PHON-04` and `ML-A1-PHON-05` close. Coverage **188/243 → 190/243 (78%)**.

**`ML-A1-PHON-03` is deliberately left open**, and the reason is the reason this
chapter exists in the shape it does.

### The tranche was measured before it was written, and the estimate was wrong

All three phonology points had been surveyed as *"three points, zero new words"*
— the best remaining value in the file. So they were measured: all 292 taught
tokens, normalised once with every long vowel mapped to its short counterpart
and once with every doubled consonant collapsed.

| | |
|---|---|
| vowel-length minimal pairs among taught words | **zero** |
| gemination minimal pairs among taught words | **zero** |
| taught tokens containing a doubled consonant | **102 of 292** |

**Abundance is not contrast.** A hundred words carry a held consonant and not one
pair of them differs *only* in that.

### The verb comes first because its stem is the contrast partner

**കുട്ടി** was already taught; **കുടി** returned zero files. So the chapter opens
by teaching **കുടിക്കുക** — *to drink* — which is not a device: the book had
given the learner **water, tea, coffee and milk and no verb to do anything with
them.**

| | | |
|---|---|---|
| **കുടി** | *kuṭi* | drinking — the stem of the new verb |
| **കുട്ടി** | *kuṭṭi* | a child |

Same letters at either end, same vowels, **one consonant held**. That is the
entire difference and it is the whole meaning.

A doubled letter is **not a spelling convention.** It is an instruction: stop on
that sound and hold it before you let go. The lesson also explains the spelling —
the letter, the vowel-killing mark, the letter again — because **the held front
half genuinely has no vowel of its own.**

The reader has been obeying that instruction by imitation across *amma*, *illa*,
*uppŭ* and ninety-nine others. Naming it means they can now produce a word they
have only ever seen written.

### Stress pairs with it, and the point is how little stress does

**The weight falls on the first syllable.** No list, no marks. But that rule is
not what the lesson is for.

**No two Malayalam words differ only in where the weight falls.** Misplace it and
you are **accented, not misunderstood** — which is a relief and also a warning:

| language | what carries a difference in meaning |
|---|---|
| English | where the weight falls |
| Malayalam | **how long a sound is held** |

That is why the two lessons sit together. **കുടി and കുട്ടി are not a stress
pair** — the weight is on the first syllable in both — and saying so is what
makes the redirection land: **an English ear arrives trained on the wrong
feature.**

### Why PHON-03 was not taken with them

There is no vowel-length minimal pair among the taught words either, and no
partner in hand that could be verified rather than constructed. Writing it would
mean inventing a pair that reads well, which is the failure this campaign has
already cut three times. The point's note now records the measurement and says
plainly not to write the lesson on a guessed pair.

### Verified before writing

**കുടിക്കുക** and **കുടി** each returned **zero** files.

Recorded for a future pass: **കുടി is shown as a form, not taught as a
headword**, so nothing owns that token. A later lesson claiming it would inherit
this chapter's uses — the `ML-C82-chechi-address` trap.

`banned-words` fired at 1057 against 1056 and `info-dump` at 34 against 32, on
*"you just learned"*, *"the rule is not the useful part"* and a *never* clause.
All three rewritten, none pinned.

### Metrics

| metric | before → after |
|---|---|
| exam-point coverage | 188/243 → **190/243 (78%)** |
| the pronunciation column | 2/5 → **4/5** |
| new words taught | **one** — കുടിക്കുക |
| `forwardReferences` | unchanged |

## Chapter 84 — where a thing is

`ML-A1-CASE-07` closes. Coverage **187/243 → 188/243 (77%)**.

The point's note was exact: *"the track teaches five house objects and no way to
say where any of them is."*

### The first lesson spends itself on the shape, not the words

| | |
|---|---|
| **മുകളിൽ** *mukaḷil* | on top of |
| **താഴെ** *tāḻe* | below |
| **മുന്നിൽ** *munnil* | in front of |
| **പിന്നിൽ** *pinnil* | behind |

Four words, but the chapter opens on the **pattern**, because the pattern is
what makes each word after the first cost a word and nothing else:

**കസേരയുടെ മുകളിൽ** — *kasērayuṭe mukaḷil* — on top of the chair.

**The noun in front takes the owner-ending**, and that is **not new grammar** —
it is the genitive from the possession chapter, unchanged, picking its form by
the last sound of the noun exactly as it did there.

### Why the owner-ending is there at all

Read it literally: **the chair's top-part.** Malayalam is not saying *on* the
chair. It names a **part that belongs to** the chair and puts you in it — which
is why the two words would not be connected without the owner-ending.

Swap the last word and nothing else moves:

| | |
|---|---|
| **കസേരയുടെ മുകളിൽ** | on top of the chair |
| **കസേരയുടെ താഴെ** | under the chair |
| **കസേരയുടെ മുന്നിൽ** | in front of the chair |
| **കസേരയുടെ പിന്നിൽ** | behind the chair |

### A pattern that covers three of the four, said plainly

**മുകളിൽ**, **മുന്നിൽ** and **പിന്നിൽ** all end in **-ിൽ** — the in-ending the
learner has had since the first verbs. These are not really postpositions in
origin: they are **part-words already sitting in the in-form**, which is exactly
why *the chair's top-part* reads the way it does.

**താഴെ does not.** It ends in **-െ** and means the same kind of thing.

The recall states that rather than tidying it away. **A pattern that explains
most of a set is still useful, and the reader who notices the exception has read
correctly rather than made a mistake.** Learn the shape from the three, and
learn താഴെ as it stands.

### A word deliberately not taught

The point's domain invites **മേശ** (*table*), and the chapter does not teach it.
**മേശ already appears untaught in `ML-C52-chair`'s etymology prose**, so claiming
it as a headword would convert that use into a forward reference — the exact trap
`ML-C82-chechi-address` hit. The examples use **കസേര**, which is taught.

That check took one grep and is the reason this chapter needed no pin raised.

### Verified before writing

**മുകളിൽ**, **താഴെ**, **മുന്നിൽ** and **പിന്നിൽ** each returned **zero** files.

`info-dump` fired once, at 33 against 32, on *"it is used in exactly the same
shape"* — the gate reads *is used for* as a rule statement. Rewritten, not
pinned.

### Metrics

| metric | before → after |
|---|---|
| exam-point coverage | 187/243 → **188/243 (77%)** |
| the case column | 4/7 → **5/7** |
| `forwardReferences` | unchanged |

## Chapter 83 — where you came from

`ML-A1-CASE-06` closes. Coverage **186/243 → 187/243 (77%)**.

The point's blocker was named on the point itself: *"combined with the missing
place names at ML-A1-N-02, 'where are you from' cannot be answered at all."*
Chapter 80 supplied the place names, so this chapter could be written.

**It adds no vocabulary at all.**

### From is built on in

| | |
|---|---|
| **ഊരിൽ** *ūril* | in the town |
| **ഊരിൽ നിന്ന്** *ūril ninnŭ* | **from** the town |

**നിന്ന് follows a word that already carries the in-ending**, so *from* is a
second step **on top of** *in* rather than an ending competing with it, and the
sense follows the shape.

One thing separates it from every ending taught so far. The endings **glue on**;
**നിന്ന് is a word of its own, standing apart.** The lesson explains why rather
than asking the reader to remember it: it began as a form of a verb meaning *to
stand* — *having stood in the town* — and hardened into the ordinary way to say
*from*.

### The question costs nothing

**എവിടെ** already asks about a place you are **at**. Put the new word after it
and the question builds itself:

**എവിടെ നിന്ന്?** — *eviṭe ninnŭ?* — where from?

Two words the learner owns, in the order the previous lesson gave them. **A
well-built system spends its second question for free**, and the lesson says so
instead of presenting the phrase as something to memorise.

### The third lesson reaches well past this point

| | in it |
|---|---|
| **ഇന്ത്യ** | **ഇന്ത്യയിൽ** — a vowel and a **-യ-** join |
| **കേരളം** | **കേരളത്തിൽ** — the **-ം is gone**, **-ത്ത-** in its place |

That is **a class, not an exception**. Every noun ending in **-ം** does it, and
the book has taught a pile of them — **പുസ്തകം** becomes **പുസ്തകത്തിൽ** by the
same rule. Learning it once puts **every -ം noun the learner owns** into the
in-form, which is a far better return than memorising two place names.

The framing the lesson gives it: **-ം is a citation dress.** It is what the word
wears standing alone and being named, and it is dropped the moment anything is
added. Look for that habit rather than for a list.

And the chapter lands:

**കേരളത്തിൽ നിന്ന്** — *kēraḷattil ninnŭ* — from Kerala.

### Verified before writing

**നിന്ന്**, **കേരളത്തിൽ**, **ഊരിൽ** and **ഇന്ത്യയിൽ** each returned **zero**
files. A sweep for any existing oblique **-ത്തി-** form found only unrelated
words — *aniyatti*, *katti*, *vākkatti* — so no lesson was silently using the
shape this chapter teaches.

`banned-words` fired once, at 1057 against 1056, on a *"just taught"* in the
second lesson. Rewritten, not pinned.

### Metrics

| metric | before → after |
|---|---|
| exam-point coverage | 186/243 → **187/243 (77%)** |
| new words taught | **none** |
| `forwardReferences` | unchanged |

## Chapter 82 — opening a conversation

`ML-A1-S-07`, `ML-A1-F-35` and `ML-A1-REG-05` all close. Coverage **183/243 →
186/243 (77%)**.

The learner could count, describe, own, negate and grade — and could not
**start**. This chapter is the opening line, which is the thing a learner needs
first and usually gets last.

### The words were already there

The note said no term of address was taught. True, but the fix was cheaper than
it sounds, because **ചേട്ടൻ** and **ചേച്ചി** have been taught since the family
chapter.

**In Kerala the sibling words are used well outside the family** — a shopkeeper,
a driver, somebody a little older than you in a queue. **You address a stranger
as kin**, and that is ordinary politeness rather than a familiarity you have to
earn. English has nothing that works this way; *excuse me* names no relationship
at all.

### What was new is the calling form

| | about them | **to** them |
|---|---|---|
| older brother | **ചേട്ടൻ** | **ചേട്ടാ** |
| older sister | **ചേച്ചി** | **ചേച്ചി** |

**A word ending in -ൻ swaps that ending for -ാ when you call somebody with it.**
*cēcci* has no **-ൻ** to swap, so it stands as it is — stated as **a rule with
nothing to act on** rather than as an exception, because that is what it is.

### The register lesson teaches no word at all

`ML-A1-REG-05`'s note contained its own method: *"the evidence is in the
lessons' own register fields; the guidance is not in their prose."* So the
lesson states the rule those fields already encode.

| greeting | its own `register` field | reach |
|---|---|---|
| **നമസ്കാരം** | `respectful-neutral` | anyone, any hour |
| **സുപ്രഭാതം** | `formal` | the morning — **formal or not** |
| **ശുഭ മധ്യാഹ്നം** | `formal` | the afternoon |
| **ശുഭ സായാഹ്നം** | `formal` | the evening |
| **ശുഭ രാത്രി** | `formal` | the night |

**നമസ്കാരം is never wrong.** The **ശുഭ** family is written and announced more
than it is spoken. **സുപ്രഭാതം is the exception inside its own family** — its own
lesson already records it as used in *both* formal and informal contexts and as
the most general morning greeting — so anybody can say it before noon.

**Knowing a word and knowing when it is used are two different pieces of
knowledge**, and the corpus had given the first and not the second.

### A forward reference the full suite caught, and the real fix

A first version made `ML-C82-chechi-address` a **`word`** lesson with the
headword **ചേച്ചി**. The suite failed at `forwardReferences` 13 against a ceiling
of 12, naming a script lesson **255 lessons earlier**.

The cause is worth recording. `ML-C12-kudumbam` teaches six family words under
**one six-word headword**, and `continuity.ts` keeps a multi-word headword
**whole** rather than splitting it. So no lesson owned the bare token ചേച്ചി, and
the script lesson's use of it was **invisible debt**. Giving that token an owner
converted the debt into a visible forward reference.

**The fix was not to raise the pin.** This lesson does not teach the word — the
family chapter did. It teaches a **use**, and the new knowledge is a rule, so its
type is `grammar`. That is the accurate classification and it happens to be the
one that does not claim ownership of a token taught long before.

### Verified before writing

**ചേട്ടാ** returned **zero** files. **ചേട്ടൻ** and **ചേച്ചി** returned three each,
all citation uses — which is what made the chapter cheap, and also what created
the trap above.

### Metrics

| metric | before → after |
|---|---|
| exam-point coverage | 183/243 → **186/243 (77%)** |
| new words taught | **one** — ചേട്ടാ |
| `forwardReferences` | unchanged at 12 (13 at one point — see above) |

## Chapter 81 — how far away

`ML-A1-ADV-09` closes. Coverage **182/243 → 183/243 (75%)**.

The point's note stated the problem exactly: *"ivide and avide give a two-way
here/there and no way to say that something is FAR, so distance can be pointed at
and not measured."*

### Pointing and measuring are different jobs

| word | job |
|---|---|
| **ഇവിടെ / അവിടെ** | **point** — my side or not my side |
| **അടുത്ത് / അകലെ** | **measure** — a little space or a lot |
| **എത്ര ദൂരം?** | **ask** — and expect a number back |

Splitting the world in two is not the same as measuring across it. Something
*aviṭe* can be a step away or a country away, and the word is the same either
way — which is why a learner could own *here* and *there* for forty chapters and
still not be able to say that the shop is close.

### One of the four words cost nothing

**എത്ര** *ethra* — *how much* — has been sitting inside the age question since
the learner met it: **നിങ്ങൾക്ക് എത്ര വയസ്സുണ്ട്?**, *to you, **how much** age
exists?* It was a piece of a phrase. The chapter takes it out and uses it:

**എത്ര ദൂരം?** — *ethra dūraṁ?* — how far?

The lesson names the habit rather than only using it: **a long sentence you
learned whole usually holds two or three words that work on their own**, and
going looking for them is the cheapest vocabulary there is.

### A lesson spent on refusing a pattern

**അകലെ** opens with **അ**, and the deictic pattern the learner knows says **അ-**
is the *away* one. It is not that here — the **അ** is part of the word.

The lesson gives the test rather than the verdict: **the question is never how
the word looks, it is whether the other members of the set exist.** There is no
*ikale* meaning near and no *ekale* meaning how far, so there is no set, so
there is no pattern.

That is worth its own page because **a pattern you have learned will start
finding matches everywhere, including where there are none**, and this is the
first place in the track where the learner's own knowledge would mislead them.

### `ML-A1-NUM-08` updated but deliberately NOT closed

Distance is now nameable and askable, which is half of what that point wants.
It stays open because it asks for **units** — no measure of length, no measure
of weight, and no number past twenty to put in front of one. `ML-A1-NUM-04`
blocks it. Recorded on the point so the next pass does not mistake it for
nearly-done.

### Verified before writing

**അടുത്ത്**, **അകലെ** and **ദൂരം** each returned **zero files**, counted by
token. **എത്ര** returned exactly one — the age question — which is what made the
fourth atom free.

### Metrics

| metric | before → after |
|---|---|
| exam-point coverage | 182/243 → **183/243 (75%)** |
| the adverb column | 8/9 → **9/9 — complete** |
| `SPINE-ASK-LOCATION` | **0 segments → 1** — the node had been empty |
| `forwardReferences` | unchanged |

## Chapter 80 — where you are from

`ML-A1-N-02` and `ML-A1-ADJ-04` both close. Coverage **180/243 → 182/243
(75%)**.

The adjective point's note named the relationship itself: *"the same hole as
ML-A1-N-02 seen from the adjective's side."*

### A category was not an answer

**നാട്** was handed over with a warning printed inside it — **its scale floats.**
A *nāṭŭ* is a country, a district or a home region, and the situation decides.
That is true and it is also why *which nāṭŭ are you from* was, until now, a
question the learner could **hear and not answer**: they owned the word for the
category and no word for a place.

| | |
|---|---|
| **ഊര്** *ūrŭ* | a town — one settled place |
| **കേരളം** *kēraḷaṁ* | Kerala |
| **ഇന്ത്യ** *indya* | India |

*ūrŭ* pins the small end down; the two names are fixed points on the scale
*nāṭŭ* floats over. All three answer the question, and the lesson says the choice
among them is **the same choice English makes**: you pick the one your listener
will recognise.

### The gentilic needed no new grammar, and that is the chapter's finding

English keeps **two** shapes for this job — *India* and **Indian**, *Kerala* and
**Keralite**. Malayalam keeps **one**:

**മലയാളി അധ്യാപകൻ** — *malayāḷi adhyāpakan* — a Malayali teacher.

**മലയാളി is a person-word, not a describing word**, and placing somebody with it
means doing what this language always does — **stand it in front, unchanged.**
That is the same position the quality word occupies before the thing, and the
degree word before the quality. **Three different jobs, one position, nothing
altered.**

So the point closes on a rule the learner has held since the adjective chapter,
and the chapter's payoff is the observation rather than the word. It is also why
the gentilic waited rather than arriving with the place names: it is worth more
as evidence that a position is being reused than as a fourth item in a list.

### The word was derivable on sight

| | |
|---|---|
| **മലയാള**ം *malayāḷaṁ* | the language |
| **മലയാള**ി *malayāḷi* | the person |

Everything up to the last piece is shared, and the learner has owned the language
word since the fifth chapter's sentence — **twelve lessons use it**. So the
lesson shows the join rather than asserting it.

The repo's own root ledger already carries `mala-mountain-dravidian` and
`alam-place-dravidian` on that chapter's lesson, so the chain **hill → land →
language → person** is stated consistently with what the corpus already claims
rather than as a fresh etymology.

### Verified before writing

**ഊര്**, **കേരളം**, **ഇന്ത്യ** and **മലയാളി** each returned **zero files** across
the whole corpus, counted by token rather than by substring. None created a
forward reference.

### Metrics

| metric | before → after |
|---|---|
| exam-point coverage | 180/243 → **182/243 (75%)** |
| the noun column | 4/6 → **5/6** |
| the adjective column | 5/6 → **6/6 — complete** |
| `forwardReferences` | unchanged |

## Chapter 79 — how much of it

`ML-A1-ADJ-05` and `ML-A1-ADV-08` both close. Coverage **178/243 → 180/243
(74%)**.

The two points are **one gap seen from two sides**, and both notes said so.
`ML-A1-ADJ-05`: *"the track teaches five adjectives and no way to grade any of
them, so 'good' cannot become 'very good'."* `ML-A1-ADV-08`: *"same gap as
ML-A1-ADJ-05, from the adverb's side."*

### One word graded five

| | |
|---|---|
| **നല്ല** *nalla* | good |
| **വളരെ നല്ല** *vaḷare nalla* | very good |

**വളരെ** costs the learner nothing structurally, because it behaves the way the
quality words already behave: **it stands in front and nothing changes shape.**
*nalla* is *nalla* in both rows of that table. So a single lesson graded all
five qualities at once rather than one of them.

### The pair is a register split, not a meaning split

| word | where you meet it |
|---|---|
| **വളരെ** *vaḷare* | a notice, a newspaper, a teacher writing |
| **ഒരുപാട്** *orupāṭŭ* | a person talking to you across a table |

They do the same job in the same position. A learner who owns only one of them
is understood; a learner who owns both sounds like they have **listened** to
somebody and not only read. **ഒരുപാട്** also measures stuff rather than
qualities — *orupāṭŭ veḷḷaṁ*, a lot of water, with **no joining word between
them**, where English needs *a lot **of** water*.

### The third grade was already paid for

`ML-A1-ADV-08` asks for *very, quite, a bit*. The small end needed **no new
word**: **കുറച്ച്** was handed over long ago as *a little* for amounts of rice
and water, and it stands in the same slot pointing **down** instead of up. The
chapter only had to point that out.

| slot | quality | |
|---|---|---|
| — | **വലിയ** | big |
| **കുറച്ച്** | **വലിയ** | a bit big |
| **വളരെ** | **വലിയ** | very big |
| **ഒരുപാട്** | **വലിയ** | really big |

Read the *valiya* column downward: **the slot carries all of the grading and the
quality word carries none of it.** That table is the chapter's third lesson, and
it is legal under the grammar-cell policy precisely because **every one of its
three fillers was taught individually first** — two in this chapter, one long
before it.

### The lesson teaches the position, not the words

Naming the slot is worth more than naming two words: a degree word the learner
has never met, dropped into that position, is understood on sight. And the slot
**nests** with the one the adjective chapter established — how much, then what
kind, then the thing, each piece reaching forward over everything after it, and
nothing ever changing shape:

**വളരെ നല്ല പുസ്തകം** — *vaḷare nalla pustakaṁ* — a very good book.

### Verified before writing

**വളരെ** and **ഒരുപാട്** each returned **zero files** across the whole corpus,
so neither created a forward reference. `കുറച്ച്` returned exactly one — the
lesson that teaches it — which is what made the small end free.

Two draft claims were cut for being wrong rather than merely weak. One said the
*valiya* column was *"the same four letters in every row"*; it is three akshara,
and the sentence was counting codepoints while pretending to count letters. The
other built a house out of **വീട്**, which **this track does not teach** — an
untaught word in an example is a forward reference waiting for the lesson that
would create it, so the example uses *pustakaṁ*, which is taught.

### Metrics

| metric | before → after |
|---|---|
| exam-point coverage | 178/243 → **180/243 (74%)** |
| the adjective column | 4/6 → **5/6** |
| the adverb column | 7/9 → **8/9** |
| `forwardReferences` | unchanged |

## Chapter 78 — whose it is

`ML-A1-POS-02` (the genitive suffix) closes. Coverage **177/243 → 178/243**.

The note was exact: *"enre and ninre are both taught whole, and the suffix
inside them is never named. So the learner owns two possessives and cannot make
a third."*

### A closed pair becomes an open rule

| | |
|---|---|
| എ**ന്റെ** | my |
| അധ്യാപക**ന്റെ** | the teacher's |

**എന്റെ** and **നിന്റെ** were two words to memorise. Naming the ending inside
them means **any noun this book has taught can now own something** — which is
the difference between a word and a piece of grammar.

### Both allomorphs, because the choice is not the speaker's

| noun ends in | ending |
|---|---|
| a consonant | **-ന്റെ** — *അധ്യാപകന്റെ* |
| a vowel | **-യുടെ** — *അമ്മയുടെ* |

One per lesson, since each is its own rule statement. The lesson's point is that
**the noun decides by its last sound**, so the question is never *which ending
do I want* but *what does this word end in*.

### The suffix was already on the page as a shape

`ML-C02-enre`'s script section showed **ന്റെ** on its own — as a conjunct to
read. It was a piece of writing then and it is a piece of grammar now, and the
chapter says so: **something introduced as a shape often turns out later to be a
part.**

### Verified before writing

**No third genitive existed anywhere in the corpus** — the only bare `-ന്റെ`
token was that script decomposition — so teaching the suffix here created **no
forward reference**. `forwardReferences` unchanged at 12.

A grep for the old pin also turned up `toBe(177)` in **Tamil's** file and in
Tamil's block of `exam-inventory`. Those were left alone; the Malayalam pins
were anchored on the `covered`/`unmapped` **pair**, since `unmapped: 66` is
Malayalam's alone.

## Chapter 77 — not there, not so

`ML-A1-NEG-02` (**alla**) and `ML-A1-Q-09` (the tag question) both close.
Coverage **175/243 → 177/243 (73%)**.

They close together because **അല്ലേ is built on അല്ല and visibly carries it**,
so the second lesson confirms what the first made possible.

| metric | before → after |
|---|---|
| exam-point coverage | 175/243 → **177/243 (73%)** |
| atoms taught | 396 → 399 |
| measurable lessons | 325 → 328 |
| `forwardReferences` | unchanged (12) |

### Malayalam negates twice where English negates once

| | denies | pairs with |
|---|---|---|
| **ഇല്ല** | that it is **there** | *is there any?* |
| **അല്ല** | that it is **so** | *is it that?* |

**ഇല്ല had 27 occurrences and അല്ല had zero**, so the track could deny that
something was present and not that something was what you called it. *"I am not
a teacher"* was unsayable.

അല്ല is taught as the negative of **ആണ്**, closing a sentence in the same place
ആണ് closes it. The split is framed as **a decision English never makes you
make**: look at the question you are answering, not at the word you would use at
home.

### The tag costs almost nothing

> **നിങ്ങൾ അധ്യാപകൻ ആണ്, അല്ലേ?**

`ML-A1-Q-09`'s note framed it exactly right — *"the -o particle asks a genuine
question; a tag question invites agreement"* — so the lesson contrasts what the
two endings **want back** rather than what they mean:

| | |
|---|---|
| **-ഓ** | *I do not know; tell me* |
| **അല്ലേ** | *I think I know; agree with me* |

**ചായ വേണോ?** from the previous chapter genuinely does not know. **അല്ലേ** is
fairly sure and is checking.

### Checked before writing, not after

Both **അല്ല** and **അല്ലേ** returned **zero files** before this chapter, so
neither created a forward reference. That check came first this time — the
previous chapter needed **വേണം** rehomed into chapter 58 for exactly that
reason, and the same grep would have caught it earlier.

**ഒരു** was considered for this tranche and set aside for the same reason: it
already appears in eight lessons from chapter 18 onward, so `ML-A1-ART-02` is a
relocation rather than the one-word lesson its note describes.

## Chapter 76 — asking for it (and a lesson moved into chapter 58)

`ML-A1-V-23` (wanting) closes. Coverage **174/243 → 175/243**; the verb column
**19/23 → 20/23**.

The point's own note called this *"the single cheapest fix in this file"* and
was right — but it was cheap in a way that took a restructure to collect.

### The learner could refuse an offer they had no way of making

**വേണ്ട** (*no need*) was taught. **വേണം** (*wanted*) appeared **exactly once**
in the whole corpus — inside `ML-C58-no-need`, which names it as *"the negative
partner of വേണം"* and then goes on without handing it over.

### So the lesson went where the word was already needed

A first version taught വേണം in chapter 76. That made `ML-C58-no-need`'s mention
a **forward reference eighty lessons early**, and Malayalam's pin caught it:
13 against a ceiling of 12.

The fix was not to raise the pin. **വേണം now lives in chapter 58 at sequence
1815, immediately before its own negative**, and `ML-C58-no-need` takes it as a
prerequisite — which is what it had been assuming all along. Forward references
are back to **12**, with none for വേണം.

Chapter 58 was already the short-answers chapter (*more*, *less*, *a little*,
*no need*, *that will do*). It is now **six** short answers, and the new one is
the only one that makes an offer rather than answering one.

### What stayed in chapter 76

| | |
|---|---|
| **ചായ വേണോ?** | do you want tea? |
| **വേണം** | yes |
| **വേണ്ട** | no |

**വേണോ** is വേണം plus the **-ഓ** question particle `ML-C70-o` already taught, so
the reader builds it in the warm-up and the lesson confirms rather than teaches.
One genuinely new atom.

### Nobody wants anything in Malayalam

> **എനിക്ക് വെള്ളം വേണം** — *to me, water is wanted*

**The water is the subject; you are not.** That is the dative-subject shape
`ML-A1-V-15` already covers, arriving again — knowing and age took it too.

## Chapter 75 — when, and if

`ML-A1-Q-06` (*when?*), `ML-A1-JOIN-07` (*when* it happens) and `ML-A1-JOIN-08`
(*if* it happens) all close. Coverage **171/243 → 174/243 (72%)**; the joining
column **6/11 → 8/11**, the question column **8/10 → 9/10**.

| metric | before → after |
|---|---|
| exam-point coverage | 171/243 → **174/243 (72%)** |
| atoms taught | 391 → 394 |
| measurable lessons | 318 → 322 |
| `atomsNeverRevisited` | 25 → **24** |
| `forwardReferences` | unchanged (12) |
| `atomChapterSpikes` | unchanged |
| `durationViolations` | unchanged (0) |
| `payoffSurprises` | unchanged (0) |
| reinforcement-window misses | 820 → 827 |

### Three points, one piece

The three close together because they are one thing: **പോൾ**.

| | |
|---|---|
| **ഇപ്പോൾ** *ippōḷ* | now |
| **അപ്പോൾ** *appōḷ* | then |
| **എപ്പോൾ** *eppōḷ* | when? |

`ML-C50-now` handed over the top two rows and stopped. `ML-C41-deixis-system`
had already promised, in so many words, that meeting a new word in this family
means being *"taught one and work out the others."* The question form is the
missing third, and the chapter is that promise being kept.

Then the same piece turns up on a verb — **വരുമ്പോൾ**, *when he comes* — and the
conditional **വന്നാൽ** is taught beside it, because one ending is the entire
difference between the two sentences.

Verified before writing: **എപ്പോൾ**, **മ്പോൾ**, *eppol* and *mbol* each returned
**zero files** across 333 lessons, and every existing `-ാൽ` in the corpus was
lexical (പാൽ *milk*, കാൽ *leg*) or one of the two frozen words എന്നാൽ and
എന്തുകൊണ്ടെന്നാൽ.

### A ramp defect found on the way in, and repaired first

**`ML-C32-pokuka` named all three tenses and printed none of them.** Its table
gave *pōkunnu / pōyi / pōkuṁ* in romanization from end to end; extracting every
Malayalam-script token from the lesson body returned പോകുക, പോ, പോക്, ഉക and
പോയി. Across the whole corpus **പോകും and വരും each appeared zero times**, and
every script word ending in `-ും` was something else — the coordinator, or a
frozen adverb.

So a reader had been told the future ending, could say it aloud, and had never
seen a Malayalam verb in the future tense written down.

`ML-A1-V-05` was **not** mis-marked: the atom is introduced and the point is
probed. Coverage cannot see this, and `glyph-coverage` cannot either, since
every glyph in പോകും is taught elsewhere. It is not a glyph debt — it is a word
the reader can say and cannot read.

It is repaired **here rather than later**, because `-ുമ്പോൾ` is built on വരും,
and meeting വരും for the first time inside a longer subordinate form is the ramp
inverted. Filed as `HL-C393`.

### The past form is not about the past

The conditional builds on the past — വന്നു → **വന്നാൽ**, പോയി → **പോയാൽ** — and
the sentence it builds is not about the past at all. *Avan vannāl ñān pōkuṁ*:
**he has not come**, and he may never come.

The lesson slows down there, because the obvious reading is the wrong one. The
time of the sentence is carried by the verb at the end, which is the same habit
the purpose chapter pointed at from the other direction.

### What the chapter refuses to do

**It does not settle എന്നാൽ.** Chapter 73 declined to take the tail of
എന്തുകൊണ്ടെന്നാൽ apart, because grammars read it different ways. Knowing `-ആൽ`
does not change that, and `ML-C75-aal` says so on the page instead of quietly
claiming the win.

It also does not claim **പോൾ** is a word. It is not one you can use alone, and
the lesson says that rather than leaving a reader hunting for it in a sentence.

## Chapter 74 — one ending, two sentences

`ML-A1-JOIN-09` (purpose) and `ML-A1-V-22` (ability) both close. Coverage
**169/243 → 171/243 (70%)**; the joining column **5/11 → 6/11**, the verb column
**18/23 → 19/23**.

| metric | before → after |
|---|---|
| exam-point coverage | 169/243 → **171/243 (70%)** |
| atoms taught | 388 → 391 |
| measurable lessons | 314 → 318 |
| `forwardReferences` | unchanged (12) |
| `atomChapterSpikes` | unchanged |
| `durationViolations` | unchanged (0) |
| `atomsNeverRevisited` | unchanged |
| `payoffSurprises` | unchanged (0) |

### The note predicted the pair

`JOIN-09` said it plainly: *"the -aan purpose form is not taught, which also
blocks the ability frame at ML-A1-V-22 — both are built on it."* That was
correct, and both forms returned **zero files** before anything was written.

### One swap, and what it buys

**-ഉക off, -ആൻ on, stem untouched.** The word that *follows* the -ആൻ form is
what decides which sentence you have said:

| what follows | what it says |
|---|---|
| another verb | **in order to** — *vāyikkāṉ pōkunnu* |
| **കഴിയും** | **can** — *enikku vāyikkāṉ kaḻiyuṁ* |

**The purpose goes in front and the event at the back**: *ñān vāyikkāṉ pōkunnu*
reads *I — to-read — go*. English puts the purpose after the verb and usually
needs a word for it; Malayalam needs none, because the ending is the whole of
it. That is the verb-last habit the quotative chapter already named, turning up
again.

### Ability arrives at you, and the track had already said so

**എനിക്ക് വായിക്കാൻ കഴിയും** is literally *reading is possible to me*, with the
person in the **dative** — which is exactly how this book teaches knowing.
`ML-C32-ariyuka`'s own warm-up says *knowing **arrived** at you, so the "I" left
the subject slot*:

| | |
|---|---|
| **എനിക്ക് മലയാളം അറിയാം** | Malayalam is known to me |
| **എനിക്ക് വായിക്കാൻ കഴിയും** | reading is possible to me |

English makes both an action with *I* in front. Malayalam marks which things are
done by you and which arrive at you, rather than leaving a reader to guess.

### One new word in the whole chapter

Every verb it uses — *vāyikkuka*, *pōkuka*, *kāṇuka*, *varuka*, *paṟayuka* —
comes from the reading and going chapters. **കഴിയും** is the only genuinely new
piece, and two points closed on it.

## Chapter 73 — why, and because

`ML-A1-Q-07` and `ML-A1-JOIN-05` both close. Coverage **167/243 → 169/243
(70%)**; the question column **7/10 → 8/10**, the joining column **4/11 → 5/11**.

| metric | before → after |
|---|---|
| exam-point coverage | 167/243 → **169/243 (70%)** |
| atoms taught | 385 → 388 |
| measurable lessons | 310 → 314 |
| `forwardReferences` | unchanged (12) |
| `atomChapterSpikes` | unchanged |
| `durationViolations` | unchanged (0) |
| `atomsNeverRevisited` | unchanged |
| `payoffSurprises` | unchanged (0) |
| reinforcement-window misses | 799 → 810 |

### Two points, one piece of work

Both notes said the same thing from opposite ends: **cause could be handled in
neither direction.** There was no way to ask what was behind something and no
way to give it. Closing one without the other would leave a learner able to ask
a question nobody could answer.

Verified before writing: **എന്തുകൊണ്ട്**, **കാരണം** and **എന്തുകൊണ്ടെന്നാൽ**
each returned zero files across the whole corpus.

### Most of the chapter was already in the reader's hands

**Malayalam has no separate word for *why*.** എന്തുകൊണ്ട് is chapter two's
**എന്ത്** — *what* — plus **കൊണ്ട്**, the ending meaning *by means of*. The
question asks, literally, **"by what?"**

And the written because carries the whole question word at its front:

| | |
|---|---|
| **എന്തുകൊണ്ട്** | why |
| **എന്തുകൊണ്ടെന്നാൽ** | because |

A long unfamiliar word with a word you know at the front of it stops being
unfamiliar. That is the lesson, and it needs no new machinery.

**കാരണം is the one for speech.** It is a noun first — a reason, a cause — taken
whole from Sanskrit **कारण**, which puts it beside ചിന്ത and സുപ്രഭാതം as nouns
this track borrows unreshaped.

### What the chapter refuses to do

**It does not take the tail of എന്തുകൊണ്ടെന്നാൽ apart.** The `-ennaal` can be
read as the *but*-word chapter 64 teaches, or as a conditional of the saying
verb, and grammars differ. The lesson says so on the page rather than picking
one and teaching a reader something that might be wrong.

That is the same refusal this campaign applies to an unsourced stroke order: the
visible containment of എന്തുകൊണ്ട് is checkable and worth teaching; the
decomposition of the rest is not.

### A draft claim that was wrong

The first version said all four question words share the front letter **എ**.
**ആര്** — *who* — opens on ആ.

What is true is more useful: three of the four carry the asking letter that
`ML-C41-deixis-system` already named when it taught ഇ- for near, അ- for far and
എ- for a question, and *who* is the exception. Question words are a family, not
a rule.

## Chapter 72 — the thing said, and a verb moved to chapter 50

`ML-A1-JOIN-06` closes — the quotative **എന്ന്**, which the point's own note
calls Malayalam's single most productive subordinator. Malayalam A1 coverage
**166/243 → 167/243 (69%)**.

| metric | before → after |
|---|---|
| exam-point coverage | 166/243 → **167/243 (69%)** |
| atoms taught | 382 → 385 |
| measurable lessons | 306 → 310 |
| `forwardReferences` | unchanged (12) |
| `scriptClosureViolations` | unchanged |
| `durationViolations` | unchanged (0) |
| `atomsNeverRevisited` | unchanged |
| `payoffSurprises` | unchanged (0) |
| reinforcement-window misses | 791 → 799 |

### Checked by token, not by substring

The note said the quotative was untaught. The string **എന്ന** *does* appear in
the corpus — five times in `ML-C64-but` and four in `ML-C69` — but every one of
those is inside **എന്നാൽ**, the word for *but* that chapter 64 teaches. The
quotative, with the virama, appeared nowhere.

That is the same trap the Malayalam pronoun chapter hit two passes ago, when
നാം looked taught because it is a substring of three ordinals. **Check the
token.**

### One marker, and the corpus becomes reportable

**എന്ന് leaves the quoted sentence completely alone.**

> **"എനിക്ക് മലയാളം അറിയാം" എന്ന് ഞാൻ പറയുന്നു.**

The quoted half is the same five syllables the reader has said since chapter
nine. English rebuilds a reported clause — *I know Malayalam* becomes *that I
know Malayalam*, with more changes for a different person or tense — and
Malayalam does not.

**It opens four verbs that were already taught**: പറയുക, ചിന്തിക്കുക, അറിയാം and
ചോദിക്കുക. Two atoms, four frames, no new verb shapes. That is why the point's
note called it the most productive subordinator in the language.

### The saying-verb was a second finding, and it belongs to chapter 50

**പറയുക was never a headword anywhere in this book**, and the corpus had been
using it since chapter 50: `ML-C50-farewell` builds *viṭa paṟayuka* and calls it
**"the speaking-verb"** — because there was no name to give it — and
`ML-C50-journey` uses it too.

Teaching it in chapter 72 turned both of those into forward references, **104 and
106 lessons early**, and `forwardReferences` went 12 → 14.

**The lesson moved rather than the prose.** It now sits at sequence 1405,
immediately before the first use, inside the chapter whose farewell is built on
it — which returned the metric to its baseline and paid a debt that predated
this work. Malayalam has no single word for *to say goodbye*: it names the thing
given, a journey or a leave, and hangs this verb off it, so the verb belongs in
that chapter on its own merits.

### Two claims removed before shipping

A draft derived പറ from Tamil **பற**- via *paṟai*, "to proclaim". That is an
etymology this project has no source for, and it came out. What replaced it is
checkable from the corpus: the other three verbs in this strand are Sanskrit —
*cintikkuka* from ചിന്ത, *cōdikkuka* from a Sanskrit word for pushing — and
this one is not.

A draft also said the farewell lesson *built* the phrase after the verb was
taught. With the lesson moved, that is no longer true, and the prose now points
only at the request form **പറയൂ** from the politeness chapter, which genuinely
comes earlier.

## Chapter 71 — the people in the sentence

`ML-A1-PRON-03` and `ML-A1-PRON-04` close. Malayalam A1 coverage
**164/243 → 166/243 (68%)**.

| metric | before → after |
|---|---|
| exam-point coverage | 164/243 → **166/243 (68%)** |
| atoms taught | 379 → 382 |
| measurable lessons | 302 → 306 |
| `forwardReferences` | unchanged |
| `scriptClosureViolations` | unchanged |
| `durationViolations` | unchanged |
| `atomsNeverRevisited` | unchanged |
| `payoffSurprises` | unchanged |
| reinforcement-window misses | 777 → 791 |

### Seventy chapters with nobody in them but the two people in the room

**അവൻ, അവൾ, അവർ and ഞങ്ങൾ appeared in zero lesson files** — not as headwords,
not anywhere in a body. A learner could say *I* and *you* and could not say
*he*, *she*, *they* or *we*.

### നാം looked taught and was not

It appeared as a headword in three lessons — `ML-C67-first`, `ML-C67-third` and
`ML-C68-eleventh` — where it is a **substring** of the ordinals **ഒന്നാം**,
**മൂന്നാം** and **പതിനൊന്നാം**. Every ordinal ending in **-ന്നാം** is a false
positive for the pronoun.

That is the same trap the Hindi campaign recorded when every ordinal matched its
own cardinal. **Check the token, not the substring.**

### The gap sat inside a system already taught

`ML-C41-that` teaches the **ഇ-/അ-** pointing pair and says in as many words that
**അ-** means far — and the corpus only ever used it on **things**. അവൻ, അവൾ and
അവർ carry that same **അ-**, so the column for *people* was predicted by a rule
the reader already had and never filled.

That is the third chapter in a row to find a gap of exactly this shape, after
Tamil's `TA-A1-PRON-03` and Kannada's punctuation. **The pattern gets taught,
the pattern is correct, and half of what it predicts never arrives.**

### Two of the three new words cost nothing new

- **അവർ** reuses `ML-C02`'s own rule that **a plural raises the register** —
  the move that turned നീ into നിങ്ങൾ in chapter two, turning അവൻ into അവർ here.
  Learning it once buys it twice.
- **അവൻ / അവൾ** split on **-ൻ** against **-ൾ**, endings that recur elsewhere.

Only the two we-words are genuinely new, and the question behind them has no
English equivalent: **ഞങ്ങൾ** leaves the listener out, **നാം** takes them in, and
Malayalam makes you choose every time. Tamil draws the same line with nearly the
same sounds, which this track's comparative voice says out loud.

77 exam points remain open. Coverage means the teaching exists, not that a
reader scores.


## Chapter 70 — the "and" that was missing from sixty-nine chapters

`ML-A1-JOIN-02` closes. Malayalam A1 coverage **163/243 → 164/243**.

| metric | before → after |
|---|---|
| exam-point coverage | 163/243 → **164/243 (67%)** |
| joining column | 2/11 → **3/11** |
| atoms taught | 376 → 379 |
| measurable lessons | 298 → 302 |
| `forwardReferences` | unchanged |
| `scriptClosureViolations` | unchanged |
| `durationViolations` | unchanged (0) |
| `atomsNeverRevisited` | unchanged |
| `payoffSurprises` | unchanged |
| reinforcement-window misses | 768 → 777 |

**One exam point for four lessons is the wrong way to read this change.**

### Malayalam had no "and"

The language coordinates with the clitic **-ഉം**, repeated on each item, and
**nothing in 307 lessons taught it**. A learner who owned sixty-nine chapters of
vocabulary — five body parts, five animals, five kitchen words — could not say
*water and rice*.

That was verified rather than taken on trust. **-ഉം** appeared in exactly **one**
lesson file, inside an *etymology note* as a morphological component of the word
for evening. The sequence **ും** appeared in nine files and every one of them had
it **inside a word** — വീണ്ടും, കുടുംബം, ഹൃദയം, തീർച്ചയായും, പോരും, the month
names. Not one taught it as a coordinator, and the track carried no joining atom
at all.

### What the chapter teaches

**English hangs one *and* in the gap between two nouns. Malayalam hangs one
-ഉം off the back of every item and leaves the gap empty.** Count the joins, not
the gaps.

> **വെള്ളവും അരിയും** — water and rice
> **വെള്ളവും അരിയും പാലും** — water, rice and milk

A list of five would carry five. That is a rule a reader can extend without
being taught the extension, unlike English lists, which change shape as they
grow.

Three sandhi landings cover the cases the examples need: **-ം** becomes
**-വും**, a vowel-final noun takes a **യ്** glide, and the chillu **ൽ** unfurls
back to **ല** because it is no longer at the end.

### The choosing ending belongs in the same chapter

> **ചായയോ കാപ്പിയോ?** — tea or coffee?

**-ഓ** is built exactly like **-ഉം** and differs from it only in the vowel: one
on each item, nothing in the gap. Having learned where Malayalam puts a joining
ending, a reader already knows where it puts a choosing one — which is why these
sit together rather than fifty pages apart. **അല്ലെങ്കിൽ** is taught alongside as
the standing word an English reader expects to find in the gap.

### What stays open, and why

**`ML-A1-JOIN-01` is not probed.** Its label is *"joining two nouns, and joining
two clauses"*, and this chapter delivers the noun half. The everyday clause link
in this corpus is the **-i** participle already covered by `ML-A1-JOIN-11`, and
claiming clause coordination on the strength of the noun lessons would claim a
range the corpus does not teach.

`ML-A1-JOIN-04` (the distributive) also stays open, but **its blocker is gone**:
its own note said it depended on the missing coordinator, which now exists.

Nine of the eleven points in this column were open before this chapter. Eight
still are.

**Malayalam has no untransferable points**, unlike Kannada's four, so its ceiling
is the full 243.

79 exam points remain open. Coverage means the teaching exists, not that a
reader scores.


## Assessment: the A1 task inventory, and the distance it measures

`task-shapes/a1.json` is checked in, realising the A1 envelope this track's own
assessment specification already published: **58 written minutes** (reading 20,
listening 18, writing 20) plus 10 speaking with 5 of preparation, 400 points,
and 60% required independently in each of the four skills.

**What declaring it immediately measured.** `reading-reach` can now see the A1
rung, and the first reading is not flattering. A1's shortest reading input is
**70 words**. This track's longest comprehension passage is **53** —
**0 of 3 parts in reach**, and **17 words short** of the nearest one.

That is not a defect introduced here. Before this file existed the rung was not
in the table at all, so the gap was not small — it was **invisible**. The pre-A1
rung still measures in reach, so the zero is a new rung rather than a
regression, and the number now says exactly how far there is to go.

## Assessment: the pre-A1 task inventory, and the script question answered

Malayalam's pre-A1 four-skill task inventory is checked in, together with the
assessment specification it cites. The track is now measurable by
`reading-reach`, which could not see it at all before: with no
`task-shapes/pre-a1.json` there was no row in the table, so a reading passage
could be written or deleted and the report read identically. Chapter 69's
passage landed while this was in review, so the measurement is now a real one:
**53 words against a 1-word ceiling, 3/3 parts in reach**, with a floor
committed so the number cannot quietly fall.

**No `assessment.json` yet, on purpose.** The machine-readable contract names
its artifacts by path and the artifact gate treats a path that leads nowhere as
an error. The contract is written after the inventories, mocks, rubrics and
answer keys, not before them.

**The script question.** Malayalam is written in two orthographies and an
examination has to say which it accepts. A Kerala committee reported in 1969 and
in 1971 the state made a reformed script official -- detaching the -u, -ū and -r̥
vowel signs, linearising most consonant clusters with the chandrakkala instead
of stacking them, and keeping the chillu letters. The deciding fact is what the
reform was FOR: the simplified orthography was intended for **printing**, while
the traditional orthography was retained for **handwriting** -- and every
writing paper in this contract is handwritten. Accepting only the printed
convention would mark candidates down for writing the way the reform expected
them to write. Both are accepted; the book's own model is the reformed
convention, which is what the corpus writes (five chillu letters ൺ ൻ ർ ൽ ൾ, and
ന്റ thirty-nine times with ൻ്റ never).

**Why the Malayalam Mission is named and not borrowed.** The Government of
Kerala's Department of Cultural Affairs teaches Malayalam to the diaspora
through four flower-named stages -- കണിക്കൊന്ന, സൂര്യകാന്തി, അമ്പൽ, നീലകുരിഞ്ഞി --
a ten-year ladder aimed at exactly this book's reader, ending in a certificate
the state treats as tenth-standard equivalent. It certifies a course completed,
not four skills measured, and publishes no per-skill pass rule. Its existence is
recorded as evidence that the audience is real, not as the target.

**What the writing paper scores** is the chandrakkala for both of its jobs.
Inside a word it joins two consonants; at the end of a word it is the half-u,
and only that one is at risk in dictation, because a careless ear loses nothing
audible by dropping it -- so it is scored by name rather than folded into a
general orthographic control. Conjuncts are scored for **consistency** rather
than correctness, since both orthographies are accepted.

## Chapter 69 — reading: words, lines, and a first passage

Malayalam's reading rung. No task shapes are declared for this track, so it
moves no number.

Three lessons, 6 -> 10 -> 27 tokens, no new word and no new sign in any.
Malayalam is the first track to reach this rung with its alphabet already
closed -- 67 glyphs taught, 67 shown -- so for once the binding constraint was
the vocabulary rather than the letters, and the vocabulary is deep enough that
every draft passed on the first attempt.

- **Words**: six words that all end in the same hook. The chandrakkala does two
  different jobs depending on where it sits -- inside a word it stacks two
  consonants into one shape, at the end of a word it is a faint half-u,
  breathed rather than said. A reader who treats the final mark as silence will
  sound wrong on five of the six.
- **Lines**: English would translate **ആണ്** and **ഉണ്ട്** with the same word.
  Malayalam keeps them apart: **ആണ്** names, putting an equals sign between two
  things and saying nothing about where either is; **ഉണ്ട്** places. So the LAST
  word of a line is worth reading first -- it says what kind of line this is
  before the middle has been read. **ഇല്ല** cancels **ഉണ്ട്**, never **ആണ്**.
- **Passage**: fifteen lines standing at one doorway and never moving. One
  **എന്നാൽ** turns the page against itself -- no flower here, but a garland is --
  and it is the only word on the page joining two thoughts. Everything else is
  held together by the order of the lines, which is what a language can do
  before it has taught a single conjunction.

Fourteen lines describe; the fifteenth turns and speaks to the reader.

## Ordinals: the only track in the column whose set has no exception at all

`ML-A1-NUM-05` was one of the thirteen ordinal points HL-C354 left open. It is
closed by twelve lessons in two new chapters, 67-68, one word a lesson, and
Malayalam A1 coverage moves **162/243 -> 163/243**, with the numeral column at
**7 of 9**. The old note also said the *ordering notion* was unavailable; that is
closed in the same tranche.

**THE ORDER IS NUMERICAL HERE, AND THAT IS THE FINDING RATHER THAN A DEFAULT.**
Every one of the five tracks repaired before this one had a reason NOT to count:
Latin's *prīmus* and *secundus* carry no cardinal, Portuguese already had five of
its ten as weekday names, Italian has a seam at eleven, Telugu has one irregular
word, Kannada has an irregular stem. Malayalam has **none of that**. **ആം**
attaches to a cardinal with no exception anywhere in the set — *including at one*
— so **ഒന്ന് → ഒന്നാം** is exactly what the rule predicts, and this is the one
track in the column where the honest way to teach the set is to count. The first
lesson opens on *first* in order to say so, and the chapter payoff states the
arithmetic: **five of five were built, none was memorized.**

**THE SISTER MALAYALAM IS NOT.** Tamil *mudal*, Kannada *modalu* and Telugu
*modalu* are one Dravidian word for "a beginning", and all three build their
"first" on it rather than on their word for **one**. Malayalam has the cognate —
**മുതൽ** *mutal* — but in Malayalam it drifted to mean "**from**", the point a
thing starts at. The slot its sisters filled with a special word stayed empty,
and the ordinary ending filled it. That box is the payoff the Tamil-comparison
thread this track runs was built for, and it is placed at *second* rather than at
*first*, because it is a claim about what Malayalam did **not** do and only lands
once the rule has been seen working twice.

**THE ORDERING NOTION, CLOSED WITH THE SAME TRANCHE.** The last lesson teaches
**ആദ്യം** (*ādyaṁ*), "at first", against the **പിന്നെ** the track already had:
*ādyaṁ ūṇŭ, pinne chāya*. It is deliberately not a twelfth ordinal. ഒന്നാം picks
a thing out of a line; ആദ്യം puts an action first in time — and it is a Sanskrit
borrowing sitting next to a native machine so regular it never needed one, which
is worth carrying as a habit of reading: a borrowing is evidence about a **slot**,
not about a language.

**THE CENSUS SAID THE SCRIPT WAS CLEAR AND THE CLOSURE GATE SAID IT WAS NOT, AND
THE GATE WAS RIGHT.** A census of the 67 distinct Malayalam characters the
track's lessons already use — headwords AND worked examples — covers every
character of every word in the tranche. But **ഏ**, the independent long *ē* that
opens *ēḻāṁ*, has **no sourced stroke order** in `data/scripts/malayalam.json`:
it is a recognition-only row, and a headword containing it does not enter glyph
closure. A Donald R. Davis Jr. handwriting clip for it exists on the source the
other vowels cite, but it is a video clip and could not be observed here, so **no
stroke order was invented**. The seventh lesson is therefore headed by its
romanization — the precedent this track's own chapter-7 counting lessons already
set — and the lesson says on the page that ഏ is a letter you may read and should
not yet copy. The corpus-wide glyph-gap queue stays empty.

That is worth recording as a general lesson: **a body-text character census
answers "can the reader see this shape before?" and the closure gate answers "has
the reader been taught to write it?"** Those are different questions, and here
they gave different answers about the same letter.

**REINFORCEMENT, DECOMPOSED, AND VERIFIED ATOM BY ATOM RATHER THAN BY TOTALS.**

    reinforcementWindowMisses        770 -> 765
    reinforcementMissesByWindow-R1   117 -> 117
    reinforcementMissesByWindow-R2    59 ->  59
    reinforcementMissesByWindow-R3   319 -> 319
    reinforcementMissesByWindow-R4   275 -> 270
    atomsTaught                      358 -> 373
    atomsNeverRevisited               24 ->  24

The twelve lessons make **30 (lesson, window) slots newly judgeable** — debt the
added length EXPOSES rather than creates — and **none of the 30 is a miss**. The
tranche's own **15 atoms** create **zero** debt in any window. Each lesson recalls
the previous lesson's ordinal (R1, distance 1), the ordinal five lessons back
(R2, distance 5), the atom exactly twenty positions back (R3) and the atom
exactly eighty positions back (R4), so every slot the new length opens is
answered by the lesson that opens it.

**THE FIVE R4 DEFECTS THE TRANCHE PAYS DOWN ARE ALL THE CARDINALS THEMSELVES:**
`ML-CONCEPT-C07-NUMBERS-1-5-01`, all three of `ML-CONCEPT-C07-NUMBERS-6-10-*`,
and `ML-CONCEPT-C20-PATHINONNU-IRUPATHU-01`. The numbers had been taught and then
never needed again at distance. Teaching the ordinals is what finally gave them
something to do.

**THE MALAYALAM A1 INVENTORY HAD NO ASSERTION IN ITS OWN TEST FILE** — the hole
HL-C354 found in Telugu and Hindi. `tests/corpus/malayalam.test.ts` now pins the
coverage total AND checks that every probe names an atom that exists. Both halves
were falsified before being kept.

**NOT TAUGHT, and the inventory note says which and why:** the longer attributive
**-ആമത്തെ** (no source was found stating when it is required over bare **ആം**, so
no usage rule is claimed); and "the first street on the **left**", which is still
out of reach because no word for left or right is taught anywhere in the track —
the ordinal half of that phrase now exists and the direction half does not.

## The pronunciation reference stops being hand-written LaTeX

`malayalam/book/chapters/appendix-pronunciation.tex` was hand-authored and printed as a
`\chapter*`. It is now rendered from `malayalam/pronunciation-reference.md`. The
chapter title, the contents line and the running head keep their three separate
strings, so the head over the page still reads "Malayalam script" rather than
"Pronunciation".

**Restored from the LaTeX before the flip:** **eight worked syllables**, കി കീ
കു കൂ കെ കേ കൊ കോ. The Markdown kept only കാ and reduced the rest to bare signs.
Both are on the page now, and the note that the *e*/*o* signs are written before
their consonant but read after — which the Markdown had and the LaTeX did not —
stays.

**Corrected:** the conjunct examples wrote **സ + ക → സ്ക**, with the chandrakkala
materialising in the answer. Worse here than in the sister tracks, because the
chandrakkala appears nowhere else in the retired file — a reader was told about a
mark they never saw. It now reads **സ് + ക → സ്ക**, and the mark itself is shown
in the fact that names it.

**Kept from the Markdown:** the Tamil cognate for every everyday word — *nandi* =
*naṉṟi*, *illa* = *illai*, *śari* = *sari*, *pōyi varām* ≈ *pōy varugiṟēṉ* —
which is the evidence for the "closest sister of Tamil" claim the LaTeX made
without it.

## Unreleased — the retrieval is seated per lesson, and Malayalam enters R2

Seventh track through the HL-C313 fix, using the per-lesson seating rule
(HL-C318). Of Malayalam's 278 R2 misses, 254 had a word lesson with duration
headroom inside their window; **219 of those closed.**

### One track-specific invariant this fix had to learn about

`tests/corpus/malayalam.test.ts` asserts that chapter 7's two spoken number
lessons contain **no Malayalam glyph at all** — a deliberate meaning-first step
taken before the script arrives. Glyph closure does not protect them: by chapter
7 the letters HAVE been taught, so a `read **X**` line passes the closure check
and still breaks the lesson. It did, on the first run.

The guard is therefore not "are these glyphs taught" but **"does this lesson
already use the script"**. A lesson showing no target script is showing none on
purpose, so it gets a spoken retrieval. That needs no list of which lessons are
special and preserves the same intent in any track that has made the same
choice.

### Every number re-measured against the merged tree, not derived

    malayalam R2 misses (5-15)                        278 ->  59   (-219)
    malayalam R1 misses (1-3)                         117 -> 117   (held)
    malayalam R3 misses (20-60)                       319 -> 319   (held)
    malayalam R4 misses (80-250)                      275 -> 275   (held)
    malayalam reinforcement window misses             789 -> 570
    malayalam atoms taught                            358 -> 358   (held)
    malayalam atoms never revisited                    54 ->  24   (improved)
    malayalam lessons                                 292 -> 292   (held)
    forward prerequisites                               0 ->   0   (held)
    forward references                                 12 ->  12   (held)
    script closure violations                         271 -> 271   (held)
    corpus R2 misses                                 4212 -> 3725
    lessons at or over the 300s ceiling                 0 ->   0
    computed seconds, median                          140 -> 142

164 lessons gained a line; the book carries 164 recall lines, 72 with a read.

Falsified before shipping: reverting `ML-C46-book` and re-measuring put R2 back
to 60.

Of the 59 that remain, only 6 were introduced by a word lesson; the rest are
phrase (24), writing (24), etymology (3) and grammar (2) atoms, which need a
different recall phrasing before they can be scheduled.

## 2026-09-01 — The opening five chapters are generated from lessons

Malayalam had five hand-written LaTeX chapters. A hand-written chapter is not
built from lessons, so every lesson-level gate reported on it by not reporting
on it at all: "0 lessons over 300 effective seconds" was true and said nothing
about Chapters 1–5, because nothing in those chapters was a lesson the gate
could see. Opening the book and finding an alphabet and several headwords
arriving together is what that silence looked like from the reader's side.

All five are now generated. **Malayalam has no hand-written book chapters left**,
and `handwritten_parity.py --check malayalam` exits 0 because there is nothing
handwritten to check.

- **Twenty-nine lessons were carried from schema v1 to schema v2.** This was
  the real blocker, and it is why the parity script's 33 blocks understated
  the job: a generated chapter requires schema v2 for *every* lesson in it,
  and 29 of the 67 lessons behind these five chapters were v1. They declared
  no knowledge atoms, no duration, no skills, no block boundaries — so they
  were invisible to the ramp, the budget, and the reinforcement windows alike.
- **Lessons the ramp cannot measure at all: 38 → 9** (87% → 97% measurable).
  Malayalam is now a version-2 track with no legacy lessons anywhere.
- **Fifteen prose blocks had no stable block type** and would have been
  rejected outright by the generator. Each was re-homed to a heading the
  renderer emits rather than being cut: two "Across the family" cousin tables
  moved into `The word, taken apart`; four "The phrase, assembled" and "The
  reply" sections became `The phrase, taken apart` and `The exchange`; three
  "The atoms" / "The engine" runs of production prompts became
  `Guided Practice`; three "Roots you now carry" recaps became
  `The roots, taken apart`. `ML-C02-practice` opened on a dialogue table with
  no warm-up and gained the one-line opening its four siblings already had.
- **Fourteen duplicate `Sounds you'll need` blocks were deleted.** Each was a
  verbatim restatement of the `The letters in this word` block directly above
  it, added to satisfy a block *counter*; generated, they would have printed
  the same letter breakdown twice on one page. One of them was also leaking
  raw LaTeX (`n\=\i`) into markdown. Deleting them raises the parity
  script's block count and lowers the reader's page count, and the reader wins.
- **Two cousin tables the `.tex` carried and the lessons did not were carried
  across.** `cognates` has no generated equivalent, so both folded into the
  etymology block, which is what they always were: the five-language "hello"
  table in `ML-C01-namaskaram` (romanisation only — the book's first lesson is
  listening-first and shows no Malayalam script at all), and the Bengali row
  and closing line of the farewell table in `ML-C04-poyi-varaam`.
- **One block was deliberately not carried.** Chapter 1's hand-written closing
  `culture` box previewed the farewell **പോയി വരാം** with its Tamil and
  Kannada cousins. `ML-C01-practice` replaced it with a promise for later
  during the gentle-ramp pass, and Chapter 4 now teaches that farewell one
  piece at a time. Re-adding it would forward-reference four untaught words in
  the chapter that is meant to be gentlest.
- **The chapters roughly doubled in depth without gaining a section.** All
  five keep exactly the section count they had (15, 20, 10, 8, 14); what they
  gained is the rest of each lesson — warm-up, letters, cousin web, grammar
  lens, practice and recall — which the hand-written versions had compressed
  away. The five hand-written sources were 1,101 lines of LaTeX; the
  generated five are 2,916, and they occupy the book's first 78 pages.

**What this cost, stated plainly.** Chapters over the twelve-atom budget went
**0 → 3**: Chapter 1 at 20, Chapter 2 at 22, Chapter 5 at 16. Not one new atom
was invented to get there — these are the atoms the v1 lessons were always
teaching and never declaring, and the budget can only see them now that they
are declared. Chapter 1 genuinely teaches a greeting, its eight-step writing
runway, and four more words; Chapter 2 genuinely teaches ten letters and eight
words. The measurement is right and the chapters are too long. The fix is the
split that `HL-C281` already specifies, now extended to Chapter 2 and costed in
this tranche's backlog entry; compressing them back down to hit a number would
undo the thing this change was for.

Held: script-closure violations **1** (unchanged — `ML-C01-practice`, still
waiting on that same split), characters shown but never taught **0**, headwords
without a romanization **0**, lessons over three new atoms **1** (unchanged,
`ML-C08-dayavayi`), lessons over five minutes **0**. The book compiles under
XeLaTeX at 459 pages with zero overfull boxes and zero missing characters, and
the split o-sign **ൊ** and pre-posed **േ**/**െ** were confirmed by rasterizing
the pages rather than by counting warnings.

## 2026-09-01 — The letters arrive where the words that need them arrive

Closure is measured in reading order, so a letter taught in Chapter 33 cannot
retire a mark the reader was asked to decode in Chapter 2. Nineteen lessons
were in that position. The fix was never more letters — it was moving the
ladder to sit under the words.

- **The whole recognition drizzle was re-sequenced against the corpus.** Every
  glyph the opening chapters put in front of the reader was traced to the
  lesson that first *needs* it, and the letter that teaches it was placed one
  or two pages earlier, anchored on a word already said aloud. Fourteen
  existing letter lessons moved (**പ ണ ു ൺ ൾ ഓ വ ഉ ഞ ച യ ഹ ഭ ഊ**) and fourteen
  new ones were written (**അ േ എ റ ആ ീ െ ഇ ൂ ൻ ൊ ൽ ജ ല**), so the ladder now
  runs 46 characters long and lands its first thirty inside Chapters 1–5,
  where the words are.
- **Lessons asking the reader to decode an untaught mark: 19 → 1.** The one
  that remains is `ML-C01-practice`, the Chapter 1 recap, which shows
  **ഇല്ല** and **ശരി** back to the reader before Chapter 1 has room to teach
  **ഇ**, **ല** and **ശ**. That is a chapter-split problem, not a letter
  problem, and it is written up in the backlog rather than paid for by pushing
  Chapter 1 past its atom budget.
- **Nothing was compressed to fit.** Chapters 1 and 2 sit at exactly twelve
  new atoms, not thirteen; no lesson gained a second new letter; no lesson
  crossed the five-minute ceiling. Two chapters that were already at the
  ceiling — `ML-C06-dative-subject` and `ML-C26-raavile` — were left alone
  rather than trimmed to make room for a review block.
- **Every letter now gets reviewed outside its own lesson.** The five opening
  chapter checkpoints gained a *Script check* block that names each letter
  beside the word it was met in, so twenty-six recognition atoms are assessed
  by the chapter payoff that closes their chapter instead of by nothing at all.
- **Three late letters came forward to where their words live**: **ഉ** from
  Chapter 17 to Chapter 4, **ഭ** from Chapter 36 to Chapter 25, and **ഊ** from
  Chapter 33 to Chapter 31 — the last of these clearing `ML-C32-tinnuka`,
  which had been meeting **ഊ** one lesson before it was taught, without
  splitting Chapter 32 or renumbering the thirty-four chapters after it.
- Held at zero: characters shown but never taught (**0**), headwords without a
  romanization (**0**), chapters over the twelve-atom budget (**0**), lessons
  over three new glyphs (**0**), lessons over five minutes (**0**).

## 2026-08-31 — Every shape the book shows is now a shape the book taught

Two halves of the same rule, which is that a learner may meet a word by ear
long before they meet it on the page, but must never be asked to decode a mark
nobody showed them.

- **Thirty-one headwords stopped being load-bearing.** Every Malayalam-script
  headword in the track now declares a `romanization`, so the word is offered
  the way the reader first meets it — sayable by ear, with the script beside it
  as something to recognise rather than something to decrypt. The lessons
  already printed these romanizations in their own prose and titles; the
  metadata had never carried them, so the measurement counted the reader as
  being asked to read what they had in fact been told how to say. Load-bearing
  headwords: **31 → 0**.
- **The drizzled script ladder gained its last eight letters.** `ML-S125`
  through `ML-S132` teach **ഊ · ഓ · ഹ · ൃ · ഭ · ഥ · ൺ · ഗ**, one character per
  lesson, one lesson per chapter, spread across Chapters 33–39 so that ordinary
  vocabulary and dialogue sit between every pair of them. Each anchors on words
  the reader has already said — *suhṛttŭ* for **ൃ**, *sahāyaṁ* for **ഹ**,
  *śubha* and *kuṁbhaṁ* for **ഭ**, *kālāvastha* for **ഥ**, and the name *Aruṇ*
  from the very first name-exchange for the chillu **ൺ**.
- **The track now teaches every letter it shows.** Characters shown but never
  taught: **8 → 0**, with taught glyphs rising 59 → 67 to meet the 67 the track
  displays. Lessons asking the reader to decode an untaught mark: **37 → 19**.
- Nothing was compressed to fit: the eight letters arrived as eight separate
  sub-three-minute lessons rather than a page of new shapes, and the **ഊ**
  lesson moved from Chapter 32 to Chapter 33 rather than push Chapter 32 one
  atom past its budget.

## 2026-08-23 — Numbers arrive by ear, then in two- or three-shape steps

- Replaced both Chapter 7 whole-row reveals with spoken meaning lessons that
  contain no Malayalam script, so sound and quantity always precede decoding.
- Split the ten numeral shapes into four observe-and-trace lessons of at most
  three shapes, then introduced the only three still-unfamiliar letters in two
  directly relevant number-word lessons.
- Carried the two halves through visible copy, delayed copy, and dictation, and
  closed with a four-skill cumulative payoff that introduces nothing new.
- Kept all fifteen Chapter 7 lessons below five minutes and removed both measured
  six- and seven-glyph cliffs without changing any curriculum budget.

## 2026-08-23 — Namaskāram starts with the ear, then eight tiny script steps

- Replaced the opening eight-glyph reveal with an ear-first meaning lesson and
  script lessons that introduce only two or three shapes at a time.
- Carried the meaningful unit **നമ** through observe-and-trace, guided copy,
  delayed copy, and dictation before asking for the complete greeting.
- Delayed the whole written **നമസ്കാരം** until every shape was familiar, then
  closed with whole-word reading, guided copy, dictation, and a chapter payoff.
- Kept every new lesson under five minutes and made the runway, chapter ledger,
  session map, book, and regression evidence describe the same learning path.

## 2026-08-20 — A spaced സന്തോഷം writing ramp

- Reduced the Chapter 2 script step to the two genuinely new shapes in the
  usable one-word expression, deferring an untaught fuller phrase.
- Added a two-minute model-visible copy and a two-minute delayed copy eight
  lessons later, while keeping the chapter checkpoint within five minutes.
- Made the sequence explicit cumulative writing evidence: observe and trace,
  guided copy, then delayed copy, with no invalid stage claims.
- Wired both writing lessons into the local script extension, introduced each
  evidence atom before assessing it, and kept the delayed-copy close as a typed
  recall block so the complete schema gate can validate the ramp.

## Unreleased -- Chapters 60-66: a third thirty-five

Malayalam stood at 119 headwords against the 300 the pre-A1 vocabulary floor asks
for -- tied with Tamil for the lowest of the twenty-two tracks, a second time.
These seven chapters answer with thirty-five more on the same terms as the two
tranches before them: **one new word per lesson**, and reuse of everything already
taught, unlimited and deliberate.

  60 Animals Around a House   പശു ആട് കോഴി കാക്ക ആന
  61 On the Kitchen Shelf     തേങ്ങ എണ്ണ തൈര് ശർക്കര മുളക്
  62 Made by Hand             കയർ നൂല് സൂചി ചൂല് മുറം
  63 How the Body Reports     വിശപ്പ് ക്ഷീണം വേദന ചുമ പനി
  64 Five Words That Join     പിന്നെ ഉടനെ ചിലപ്പോൾ മാത്രം എന്നാൽ
  65 Asking Well              അപേക്ഷ അനുവാദം സമ്മതം മര്യാദ വിശ്വാസം
  66 Out in the Paddy         നെല്ല് പുല്ല് കതിര് കലപ്പ കൊയ്ത്ത്

The vocabulary criterion moves 119 to 154 of 300 -- a shortfall of 181 falling to
146, by exactly the thirty-five lessons written and not one more. Each of the seven
pre-A1 spine nodes carries one chapter, all seven used a third time; a node is a
thing you can do rather than a slot that fills up, so a third pass over it is the
design working.

Chaining is unbroken. Chapter 60's first lesson picks up from കനൽ, each lesson
chains to the one before it, each chapter's second lesson still practises the
previous chapter's final word, and each fifth lesson is the payoff that says all
five. That chaining is why the ramp got *gentler* again: Malayalam's own R1
reinforcement ratio falls 0.3444 to 0.3007, its numerator unmoved at 83 while the
denominator grew 241 to 276. Whole-corpus R1 falls 0.2823 to 0.2800 with its
numerator likewise unmoved at 1181. Not one of the thirty-five new atoms misses its
R1 window, and `atomsNeverRevisited` holds at 51.

**Nine candidate words were written and then discarded before these thirty-five
survived.** Three fell to the never-re-teach check reading inside existing
headwords rather than only at whole words: കുട "umbrella" sits whole inside both
കുട്ടി and കുടുംബം, വിള "crop" sits whole inside വിളക്ക്, and വല "net" sits whole
inside വലിയ. One fell to a gloss that already exists: ഉറങ്ങൂ is glossed "sleep" in
the imperative chapter, so ഉറക്കം would have re-taught the word rather than added
one. Four fell to the taught-glyph filter -- ദാഹം "thirst" needs ഹ, ആരോഗ്യം
"health" and വേഗം "quickly" need ഗ, and കൃഷി "cultivation" needs ൃ, none of which
Malayalam's writing lessons teach.

The ninth is the interesting one, and it is the class no string search reaches: a
**gloss of a morpheme inside somebody else's etymology note**. മീൻ "a fish" scans
clean against every headword, every romanization and every lesson body -- but
`ML-C53-star` explains that the inherited Dravidian way of naming a star was to
call it a fish in the sky, and glosses the Tamil compound *viṇmīn* on the way past.
The morpheme *mīn* has already been handed to the reader with its meaning attached.
ആന "an elephant" took that slot instead.

**Ten more candidates were examined under the same rule and KEPT**, because a rule
that only ever rejects is a filter rather than a rule. In each of these the
colliding gloss is of an etymon, an English analogy or a grammatical label, never of
a Malayalam headword:

  - കലപ്പ "a plough" -- `ML-C48-farmer` glosses the SANSKRIT root *karṣ-* as "the
    plough pulled through the ground". An etymon, not a Malayalam word. Kept, and
    the new lesson names the connection outright.
  - ആന "an elephant" -- `ML-C54-branch` lists "the tusk of an elephant" among the
    senses of കൊമ്പ്. That is a sense-list of an already-taught headword. Kept, and
    made the chapter's payoff so the tusk-word finally gets its animal.
  - ആട് "a goat" -- `ML-C36-kutti` glosses the ENGLISH word "kid" as covering a
    child and a young goat. An English analogy for കുട്ടി. Kept.
  - തേങ്ങ "a coconut" -- `ML-C15` glosses ഇളനീർ "tender coconut water", a compound
    of ഇള and നീർ; neither morpheme is തേങ്ങ. Kept.
  - മുളക് "a chilli" -- `ML-C56-basket` mentions "a load of pepper to market" in
    English, with no Malayalam word attached. Kept.
  - അനുവാദം "permission" -- `ML-C32-kaanuka` uses "permission" as a label for one
    of the moods a Malayalam ending can carry. Kept.
  - അപേക്ഷ "a request" -- `ML-C08-dayavayi` uses "respectful request form" to name
    an ending, not a noun. Kept.
  - കൊയ്ത്ത് "the harvest" -- `ML-C52-flower` names the harvest festival in English
    while glossing പൂക്കളം. Kept, and the new lesson points back at it.
  - നെല്ല് "unhusked rice" -- `ML-C55-field` glosses വയൽ "a paddy field". Same
    English word, different referent: the field against the grain standing in it.
    Kept, and the lesson sets നെല്ല്, അരി and ചോറ് side by side as the three stages
    of one plant.
  - വിശപ്പ് "hunger" -- `ML-C06-dative-subject` lists "being hungry" among the
    things that arrive at a person. English prose, no Malayalam word. Kept.

Two romanization near-collisions were kept and turned into teaching rather than
avoided: *āṭŭ* sits inside *nāṭŭ* and *nellŭ* contains *ellŭ*, so the goat lesson
and the paddy lesson each say so and send the ear to the front of the word.

Every one of the thirty-five headwords, and every Malayalam citation in their
bodies, is spelled from the forty-nine characters Malayalam's writing lessons
teach. `scriptClosureViolations` holds at 43 and `exposureExemptedGlyphs` at 122:
the tranche adds zero to both. `forwardReferences` holds at 20, the rule-statement
count at its ceiling, and cross-chapter prose references at 46 -- the tranche names
no chapter by number at all. Cousin words are cited in romanization rather than in
Tamil or Kannada script, keeping the tranche's script surface to one writing system.

HL-C201: `ML-C59-ember` needed no rewording. Its "the run is closed" names that
chapter's five words rather than the book, and stays true with seven chapters
appended after it.

A whole-tree sweep (HL-C202/C203/C208) over all 460 files in the track -- reading
file bytes, classifying by Unicode BLOCK RANGE rather than by `unicodedata.name`
(which raises on unassigned codepoints) and rather than by `\w` (which drops
combining marks), with every fixture built from `chr()` in a source file asserted
pure ASCII -- reports zero mixed-script words, zero NUL, zero ZWJ/ZWNJ, zero bidi
controls, zero BOM, zero soft hyphens and zero replacement characters. The scanner
was self-tested against fifteen known-dirty and eleven known-clean controls, and
then run against the PRE-FIX bytes of the three files this changelog already
records as defective; it rediscovered all three before the clean result was
believed. The chillu letters remain atomic U+0D7B-U+0D7E throughout -- twenty-one
of them across the new lessons, not one spelled with a joiner.

All twenty-two books rebuild at exit 0 with zero missing characters, zero overfull
and zero underfull boxes. Malayalam's own book is now 401 pages.

## Unreleased -- Chapters 53-59: a second thirty-five, and three mixed-script repairs

Malayalam stood at 84 headwords against the 300 the pre-A1 vocabulary floor asks
for -- tied with Tamil for the lowest of the twenty-two tracks. These seven
chapters answer with thirty-five more, on the same terms as the first tranche:
**one new word per lesson**, and reuse of everything already taught, unlimited
and deliberate.

  53 Overhead                  ആകാശം സൂര്യൻ ചന്ദ്രൻ നക്ഷത്രം വെയിൽ
  54 A Tree, Part by Part      മരം കൊമ്പ് തടി വേര് വിത്ത്
  55 Ground Underfoot          പുഴ പാറ നാട് വഴി വയൽ
  56 Things About the House    പായ കൊട്ട കത്തി പാത്രം പെട്ടി
  57 Five More of the Body     കഴുത്ത് മുതുക് ചുണ്ട് നഖം എല്ല്
  58 Five More Short Answers   കൂടുതൽ കുറവ് കുറച്ച് വേണ്ട പോരും
  59 What the Air Carries      കാറ്റ് മണൽ ചെളി പുക കനൽ

The vocabulary criterion moves 84 to 119 of 300 -- a shortfall of 216 falling to
181, by exactly the thirty-five lessons written and not one more. Each of the
seven pre-A1 spine nodes carries one chapter, all seven used a second time;
reusing them is the point, since a node is a thing you can do rather than a slot
that fills up.

Chaining is unbroken. Chapter 53's first lesson picks up from മാല, each lesson
chains to the one before it, each chapter's second lesson still practises the
previous chapter's final word, and each fifth lesson is the payoff that says all
five. That chaining is why the ramp got *gentler* again rather than steeper: the
R1 reinforcement ratio falls 0.2780 to 0.2757, its numerator unmoved at 1123
while the corpus grew. Not one of the thirty-five new atoms misses its R1 window.

**Nineteen candidate words were written and then discarded before these
thirty-five survived.** Thirteen fell to the never-re-teach check, which reads
both directions and looks inside existing headwords rather than only at whole
words: മല "hill" sits whole inside മലയാളം, തീ "fire" inside തീർച്ചയായും,
ഇല "leaf" inside ഇല്ല, കുട്ട "basket" inside കുട്ടി, മഞ്ഞ് "mist" over the
colour-word മഞ്ഞ, and പുറം "back" was simply already glossed in the
setting-out lesson. Teaching any of them here would have made an earlier page
point forward, which is the failure the forward-reference count exists to catch;
that count holds at 508 and the rule-statement count holds at 30.

The other six fell to a **taught-glyph filter**. Malayalam's writing lessons
teach forty-nine characters, and a headword spelled with anything outside that
set either breaks script closure or gets laundered through the romanization
exemption. മേഘം "cloud" needs ഘ, ഗ്രാമം "village" and അഗ്നി "fire" need ഗ,
ഓല needs ഓ -- so the sky chapter ends on വെയിൽ instead, the village chapter on
നാട്, and the fire chapter on കനൽ. Every one of the thirty-five headwords and
every Malayalam word in their bodies is spelled from characters the reader has
already been taught, so script-closure violations hold at 756 and
exposure-exempted glyphs at 2220: the tranche adds zero to both.

Cousin words are cited in romanization throughout rather than in Tamil or
Kannada script, which keeps the tranche's script surface to one writing system.

### Three mixed-script repairs (HL-C202)

Three words in the committed track were each spelled across two scripts, so the
renderer emitted them over two font commands mid-word and they printed as
plausible wrong text while the build exited 0 with no missing characters:

  - `ML-C11-nirangal.md` -- Tamil நீல was carrying U+0D32 MALAYALAM LETTER LA
    in place of U+0BB2 TAMIL LETTER LA.
  - `ML-C37-mookku.md` -- Kannada ಮೂಗು was carrying U+0D41 MALAYALAM VOWEL
    SIGN U in place of U+0CC1 KANNADA VOWEL SIGN U.
  - `roadmap.md` -- Tamil எழுது was carrying U+0D41 MALAYALAM VOWEL SIGN U in
    place of U+0BC1 TAMIL VOWEL SIGN U.

One codepoint changed in each; the corrected spelling was taken from an
occurrence already committed elsewhere in the corpus rather than composed. The
four narration files that quote these lessons cleared on regeneration. A
whole-tree sweep -- reading file bytes, walking Unicode categories L/Mn/Mc/Me
rather than `\w`, and self-tested against twelve known-dirty and eleven
known-clean fixtures built from `chr()` codepoints -- now reports zero mixed-script
words, zero NUL, zero ZWJ/ZWNJ, zero bidi overrides or isolates, zero BOM and
zero soft hyphens across the whole Malayalam track. The chillu letters remain
atomic U+0D7B-U+0D7E throughout.

## Unreleased — Chapters 46-52: Thirty-five everyday words, one per lesson

Malayalam stood at 49 headwords against the 300 the pre-A1 vocabulary floor asks
for. These seven chapters answer with thirty-five, and with nothing else — **one
new word per lesson**, and reuse of everything already taught, unlimited and on
purpose.

  46 Things You Ask For        പഴം തുണി വിളക്ക് ഉപ്പ് പുസ്തകം
  47 The Leg and the Tooth     കാൽ പല്ല് മുടി വിരൽ വയറ്
  48 Who Someone Is            അധ്യാപകൻ വിദ്യാർത്ഥി വൈദ്യൻ കർഷകൻ അതിഥി
  49 Short Replies             സത്യം മതി തീർച്ചയായും ഒരുപക്ഷേ അങ്ങനെ
  50 Taking Leave              ഇപ്പോൾ മറ്റന്നാൾ യാത്ര പുറപ്പെടുക വിട
  51 Courtesy                  കൃതജ്ഞത ഉപകാരം ബഹുമാനം അനുഗ്രഹം വന്ദനം
  52 Welcoming a Guest         വാതിൽ കസേര കോലം പൂവ് മാല

Each chapter sits on one of the seven pre-A1 spine nodes, five lessons sharing
it, and each lesson chains to the one before — so a word introduced by a
chapter's payoff lesson is still being practised two lessons into the next
chapter. That is why the ramp got *gentler* rather than steeper: the R1
reinforcement ratio falls 0.3034 to 0.3005 even though the corpus grew. Not one
of the thirty-five new atoms misses its R1 window.

Every headword was checked against all 142 existing lessons before it was
written, in both directions, so none of the thirty-five re-teaches anything and
none of them makes an earlier page point forward — the corpus forward-reference
figure holds at its 500 ceiling, and so does the thirty-count on rule statements.

Two threads run the length of the seven chapters rather than sitting in one
lesson. The first is the chillu, the consonant written without a vowel and given
a letter of its own: ൽ closes കാൽ, വിരൽ, വാതിൽ and ജനൽ; ൻ marks the three
masculine role-words; ൾ closes ഇപ്പോൾ and മറ്റന്നാൾ; ർ sits inside വിദ്യാർത്ഥി,
കർഷകൻ and തീർച്ചയായും. Each is written as its own atomic character rather than as
a consonant plus virama plus a zero-width joiner, because U+200D belongs to no
Unicode script block at all and so falls through every font selection the book
makes — a trap the Kannada tranche fell into and this one is written to avoid.

The second is the standing division of labour between an inherited word and a
borrowed one. തുണി against വസ്ത്രം, മുടി against രോമം, നന്ദി against കൃതജ്ഞത,
വിളക്ക് against the Sanskrit lamp Kannada took instead: the native word does the
daily work and the borrowed one does the ceremony, again and again, until it
stops being a fact about five words and becomes a fact about the language.

Chapter 49 spends its last lesson collecting on a debt from the chapter on
pointing: അങ്ങനെ is എങ്ങനെ with the far-pointer swapped in for the question one,
so the reader who saw i- / a- / e- once gets the fifth reply for nothing. Chapter
52 does the same with വായ്, the mouth-word from the chapter on the face, which
Tamil's door-word வாயில் shows sitting inside വാതിൽ.

Chapters 43, 44 and 45 also move from a bare `unicodeScript` to the track's
`malayalam-comparisons` script set, matching chapters 6-42 (backlog HL-C200).
Their rendered output is byte-identical and their book hashes did not move,
because they happen to cite no cousin script today; what changes is that a
comparison table added to them later can no longer drop its glyphs silently into
the Latin font at exit 0.

## Unreleased — Chapter 41: Pointing, and Asking

Six words and the pattern behind them: ഇത് അത് ഇവിടെ അവിടെ ആര് എവിടെ

Until now the reader could NAME things and not point at them. With these they
can: *this one*, *that one*, *here*, *there*, and the two questions that matter
first — *who?* and *where?* Everything already in the book becomes a sentence
they can use.

The seventh lesson is the reason these six are one chapter. They are not six
words, they are **i- / a- / e-** — three beginnings on the same ending, and changing
the front walks the meaning from near to far to a question. A reader who sees
that once does not have to be taught the third member of the next family they
meet; they will work it out.

The whole chapter is **voice**: nothing in it needs eyes, so it is learnable end
to end at the wheel.

## Unreleased — the first 8 characters this book actually teaches

8 recognition segments, one character each, in chapters 6-13: ക ◌് ◌ി ◌ാ ന ൾ ◌ു ത

Until now this track taught **no letters at all**. Every word was printed in its
own script and the reader had no way in — HL12's measurement put the track at A2
by aspiration and pre-A1 by attainment, with the script strand simply missing.

Each segment names one character, says what it carries, and shows it inside four
words the reader **already says** — so nothing new has to be learned in order to
do the recognising. That is HL12 §2.1's rule made concrete: a lesson may sit at
the frontier of decoding or of meaning, never both, because a reader who fails
one that is new in both cannot tell which one they failed.

They teach recognition and not writing, and that is a sourcing fact rather than a
pedagogical preference. This script has **no cited stroke order** in the corpus —
its own script file says *"Recognition only"* — and HL11 §5 forbids a pen path
without one, because a learner cannot tell an invented stroke order from an
attested one and will drill it for years. So the reader is asked to trace the
printed shape, which needs no source, and the book says plainly that where to
start the character and which way to travel are not written down yet.

Each segment sits **last** in its chapter, after every word in that chapter that
contains its character — so it consolidates rather than pre-teaches, and it costs
the driving edition nothing: `drivablePrefixTotal` is unchanged corpus-wide.

## Unreleased — 24 words a reader can now say

Added `romanization` to 24 lessons that had none, so their headwords become
HL11 *exposure* — something the reader is shown and can use — rather than script
they are stuck on. Each is recovered from the pronunciation the lesson already
gives in its own prose, then checked against the headword's script so a wrong
grab cannot pass. Nothing is transliterated: a mechanical romanization of this
script disagrees with its own authors often enough to teach mispronunciations.

## Chapters 35-40 — Vocabulary wave 5: family, face, hearts, and drinks (2026-08-08)

- Added fifteen schema-v2 word lessons in six new chapters, closing the HL09
  §3.1 pre-A1 gate's **reinforcement** blocker outright and narrowing (not
  closing — this is one tranche of an ongoing program) the **vocabulary**
  blocker. Malayalam's **spine-nodes** criterion was already satisfied before
  this tranche — all seven pre-A1 spine nodes were already realized through
  existing content — so this wave went straight to vocabulary depth and
  reinforcement, per HL09's measured, not assumed, gate report.
- **Ch. 35 (Family and Friend)**: കുടുംബം *kuṭumbaṁ* (family, the Sanskrit
  collective word Chapter 12's six people-words never supplied), സുഹൃത്ത്
  *suhṛttŭ* (friend, Sanskrit *su* + *hṛd* "good-heart" — foreshadowing
  Chapter 38's own heart-word, with the everyday spoken alternative
  കൂട്ടുകാരൻ/കൂട്ടുകാരി named too).
- **Ch. 36 (Child, Son, Daughter)**: കുട്ടി *kuṭṭi* (child/kid, matching
  Tamil's *kuṭṭi*), മകൻ *makan* (son, Proto-Dravidian, matching Tamil almost
  exactly where Kannada lenited *-k-* to *-g-* and dropped the final *-n*),
  മകൾ *makaḷ* (daughter, **identical** to Tamil's own word, sound for sound
  — the closest match in the whole tranche).
- **Ch. 37 (Face Words)**: കണ്ണ് *kaṇṇŭ* (eye, near-identical across all four
  literary Dravidian languages), ചെവി *cevi* (ear, Proto-Dravidian *\*kewi*
  — Malayalam, Tamil and Telugu all palatalized the initial *k-*; Kannada
  alone kept it, ಕಿವಿ *kivi*), മൂക്ക് *mūkkŭ* (nose, matching Tamil and
  Telugu, with Kannada's usual *-k-*/*-g-* softening), വായ് *vāy* (mouth,
  DEDR 5352 — the word Chapter 33 already named once, only to rule out that
  വായിക്കുക, "to read," is built on it).
- **Ch. 38 (The Two Hearts)**: ഹൃദയം *hṛdayaṁ* (heart, Sanskrit तत्सम, and a
  genuine — if very distant — Indo-European cousin of English *heart*, via
  the same root as Greek *kardía* and Latin *cor*), നെഞ്ച് *nenchŭ* (chest,
  native Dravidian, cognate with Tamil's நெஞ்சு/நெஞ்சம் — the word idiom
  actually reaches for, e.g. നെഞ്ചിടിപ്പ് "chest-beat").
- **Ch. 39 (Tea, Coffee, Milk)**: ചായ *chāya* (tea, Chinese *chá* carried
  overland through Persian and Hindi-Urdu — the land route), കാപ്പി *kāppi*
  (coffee, from English, itself from Arabic *qahwah* via Turkish and Italian
  — the sea route; tea and coffee arrive at the same tea-shop counter from
  opposite directions), പാൽ *pāl* (milk, native, matching Tamil **exactly**
  and Kannada by the same *p*-to-*h* law documented at *pōkuka*/*hōgu* — the
  mirror image of Chapter 15's വെള്ളം, which broke from Tamil's water-word
  entirely).
- **Ch. 40 (The Meal)** — the tranche's payoff: ഊണ് *ūṇ* (a meal, native
  Dravidian *\*uHṇ-*, the noun behind ഉണ്ണുക *uṇṇuka*, a **second eat-verb**
  this track had never taught, standing beside Chapter 32's everyday
  തിന്നുക; Tamil's cognate உணவு *uṇavu* generalised to "food," Kannada's ഊട
  *ūṭa* narrowed to "a meal" like Malayalam's own ഊണ്). Closes the
  SPINE-POLITE-REQUEST-REPAIR arc: ദയവായി (Ch. 8) asks, ക്ഷമിക്കണം (Ch. 9)
  repairs, and both are folded back in alongside Chapter 6's dative and
  Chapter 34's *iṣṭamāṇŭ* frame.
- **Every etymological claim was checked, and several assumptions going in
  were corrected against the corpus and general Dravidological knowledge
  rather than taken on trust.**
  - ചെവി/ಕಿವಿ are **cognate**, both from Proto-Dravidian *\*kewi* — not two
    unrelated roots. Malayalam, Tamil and Telugu palatalized the inherited
    *k-* before this front vowel; Kannada alone kept it. The lesson names
    this as a regular correspondence on this one word, not a systemic sound
    law across the whole lexicon (the family's usual subgrouping does not
    cleanly predict it).
  - കുടുംബം turned out to be a **genuine gap**: Chapter 12's own id is
    `kudumbam`, but its actual headword is the six people-words
    (അച്ഛൻ/അമ്മ/ചേട്ടൻ/അനിയൻ/ചേച്ചി/അനിയത്തി) — the word "family" itself,
    കുടുംബം, had never been taught. This mirrors Kannada's own
    KA-C35-kutumba finding exactly, and the parallel is real: both
    languages nativize the same Sanskrit loan, but Malayalam adds its own
    **‑അം** neuter-noun ending (as it already does on ഇഷ്ടം, സുഖം) where
    Kannada borrows the bare Sanskrit stem.
  - The bare Chinese character 茶 and the bare Greek **καρδία** were
    **removed** from lesson prose after they produced `Missing character`
    warnings against Latin Modern Roman during the book build — Kannada's
    own chai and heart lessons already establish the precedent of citing
    such forms by romanization only (*chá*, *kardía*), never in their native
    script, and this tranche now follows it.
  - A drafting mistake, not a factual one: several Sanskrit vocalic-*r*
    spellings (*hṛd*, *suhṛttŭ*, *kṛpā*-family words) were first typed as
    the **decomposed** sequence `r` + U+0325 (combining ring below) instead
    of the **precomposed** `ṛ` (U+1E5B) the rest of the corpus uses
    throughout. The decomposed form is invisible in an editor but has no
    glyph in Latin Modern Roman, and produced eight `Missing character`
    warnings in the first build. Fixed corpus-wide across all fifteen
    lessons; the book now compiles with zero missing characters.
- **Reinforcement discipline.** Every lesson's `practises.knowledge` reaches
  back through the two immediately preceding lessons (closing the R1/R2
  windows for every new atom this tranche introduces — measured, not just
  intended: the pre-A1 thin-atom count is confirmed at **zero** after this
  tranche), and specific lessons reach further back to rescue the sixteen
  pre-A1 atoms the HL09 gate report had flagged as under-reinforced before
  this wave: ML-C35-kudumbam and ML-C36-kutti rescue Chapter 12's
  കുടുംബം-atoms; ML-C36-makal rescues Chapter 19's വയസ്സ്; ML-C37-kannu and
  ML-C38-nenchu rescue Chapter 13's ശരീരഭാഗങ്ങൾ; ML-C39-chaaya rescues
  Chapter 6's dative atoms and Chapter 9's ക്ഷമിക്കണം atoms (repeated again
  in ML-C40-oon); ML-C39-paal rescues Chapter 15's വെള്ളം/അരി atoms.
  Malayalam's pre-A1 reinforcement blocker — 16 atoms revisited fewer than
  twice — is fully closed by this tranche.
- **Atom budget.** Each of the six new chapters introduces at most 8 new
  atoms (Chapter 37's four lessons; every other new chapter introduces 3-6),
  against the 12-atom-per-chapter ceiling, and no lesson introduces more
  than 3. Zero atom-budget violations, corpus-wide, after this tranche.
- **Vocabulary.** Malayalam's distinct-headword count rises from **69 to
  84** overall, and from **34 to 49** at or below pre-A1 — fifteen new
  one-headword lessons moving the count by exactly fifteen, per
  `vocabularyOf()`'s 1:1 lesson-to-headword accounting. The pre-A1 shortfall
  against the 300-word target narrows from 266 to 251; closing it fully
  will take further tranches, consistent with every prior wave in this
  program.
- Wiring: `ML-PATH-029` through `ML-PATH-034` and six new
  `ML-EXT-029`.. `ML-EXT-034` extensions in
  [`curriculum.json`](./curriculum.json) — three attached to
  `SPINE-EXCHANGE-NAMES`, two to `SPINE-CHECK-WELLBEING`, two to
  `SPINE-POLITE-REQUEST-REPAIR` — six new chapter-capability entries in
  [`chapters.json`](./chapters.json), six new `core/book-generation.json`
  targets, the generated `book/chapters/ch35..ch40-*.tex`, six new `\input`
  lines in `book.tex`, and generated narration for all fifteen lessons.
- The 166-page XeLaTeX build has **zero missing characters** and zero
  errors (two minor underfull-hbox warnings on the Chapter 38 heading,
  consistent with the small number of pre-existing underfull boxes already
  tracked elsewhere in the track).
- `npm run check:modality`, `check:books` and `check:narration` are all
  clean. The six corpus-wide snapshot tests
  (`tests/{chapters,continuity,levels,modality-manifest,narration,ramp}.test.ts`)
  pin exact whole-corpus totals from before this wave and are **expected**
  to fail until the orchestrating merge re-measures across all of wave 5;
  every other test, including `tests/integration.test.ts` and
  `tests/cli.test.ts`, passes.

## Chapters 33-34 — The eight verbs (2026-08-07)

- Added eight schema-v2 lessons carrying the eight canonical concepts fifteen
  other tracks already teach — `VERB-THINK`, `VERB-UNDERSTAND`, `VERB-READ`,
  `VERB-WRITE`, `VERB-TAKE`, `VERB-ASK`, `VERB-HELP`, `VERB-LIKE-LOVE` — making
  Malayalam the sixteenth track to hold all of them, and the fourth Dravidian
  contributor after Tamil, Kannada and Telugu.
- **Two chapters of four, never one of eight.** Chapter 33 introduces 9 atoms
  and Chapter 34 introduces 9, both under the `maxNewAtomsPerChapter` budget of
  12 that one chapter of eight would have blown. Each carries its own `canDo`
  and its own payoff, and both payoffs assess **every** atom their chapter
  introduces — 9/9 and 9/9, representativeness **1.00** against the 0.5 floor.
- **Ch. 33 (The Mind and the Page)**: ചിന്തിക്കുക, മനസ്സിലാക്കുക, വായിക്കുക,
  എഴുതുക. Its spine is a diagnostic the learner can hear: the **ഇ** of
  **‑ഇക്കുക**, and the **‑ഇച്ചു** past that goes with it, mark a Sanskrit
  borrowing. മനസ്സിലാക്കുക comes apart into *manassŭ* + locative *‑il* +
  *ākkuka* — "to put in the mind" — and its intransitive twin **എനിക്ക്
  മനസ്സിലായി** puts the understander in the dative, joining Ch. 6's *enikku
  malayāḷaṁ aṟiyāṁ*. എഴുതുക is Tamil **எழுது** unchanged, carrying the **ഴ**
  that Kannada and Telugu lost — and with it the verb, which is why they write
  with the line-drawing root Malayalam kept for വരയ്ക്കുക, "to draw."
- **Ch. 34 (Taking, Asking, Helping, Liking)**: എടുക്കുക, ചോദിക്കുക,
  സഹായിക്കുക, എനിക്ക് മലയാളം ഇഷ്ടമാണ്. എടുക്കുക corrects Ch. 32's rule rather
  than breaking it — the stamp is the **ഇ**, not the doubled **ക്ക** a native
  stem carries by itself. ചോദിക്കുക is Sanskrit *cud*, "to urge," and the
  reason a loan was needed at all is structural: Proto-Dravidian *\*kēḷ‑*
  covered hearing **and** asking, Tamil's கேள் still does both, and Malayalam
  narrowed കേൾക്കുക to hearing alone. The same shape recurs four times across
  the two chapters — ഓതുക, ഓർക്കുക, കേൾക്കുക, ഉതവി each gave up an everyday
  slot to a Sanskrit word — which is the Maṇipravāḷam era's lexical footprint.
- **Every etymology was checked against sources rather than taken on trust, and
  four briefed or inherited claims were corrected in the process.**
  - വായിക്കുക is **not** from വായ് "mouth". Gundert marks വായന a *tadbhava* of
    Sanskrit *vac* and gives the Tamil verb as *vācikka*; DEDR 5352 (*vāy*
    "mouth") carries no reading sense in any Dravidian language. The mouth story
    is folk etymology, and the lesson says so.
  - The *c* → *y* is **not** a Malayalam sound law. It happened in the middle
    Indo-Aryan stage the *tadbhava* passed through; Malayalam holds the same
    Sanskrit word borrowed straight as വാചകം, so the doublet is visible in the
    language itself.
  - എഴുതുക is **not** filed under എഴു "to rise". Burrow–Emeneau keep them as
    separate entries, so the lesson names the link and marks it unproven — while
    എടുക്കുക genuinely *is* filed inside the rise-family, which is where that
    fact belongs.
  - English *mind* descends from *\*ménti‑*, not from the *\*ménos* that gave
    Sanskrit मनस्; the lesson claims root-level cognacy only, and names मति and
    Greek *ménos* as the exact matches.
  Two further claims are hedged rather than asserted: Monier-Williams marks the
  *saha* + *aya* reading of सहाय "probable", and whether Kannada/Telugu *ettu*
  is the cognate of എടു is a point Gundert and DEDR disagree on.
- **Reinforced at two cadences.** Every lesson's `practises.knowledge` names
  atoms from the immediately preceding one to three lessons, across the chapter
  seam; each payoff reaches several chapters back. Malayalam's never-revisited
  atoms fall from **72 of 78 (92%)** to **46 of 96 (48%)** — 29 previously
  orphaned atoms rescued, spanning Chapters 6, 7, 8, 9, 10, 13, 15, 16, 18, 19,
  20, 24, 26 and 32. The three that remain from this tranche belong to the
  final lesson of the track, which no later lesson exists to retrieve.
- All eight use the canonical `## The letters in this word` heading. That block
  classifies as `script`, which is **detachable**, so every lesson derives
  `modality: sight` with `coreModality: voice` — the driving edition is intact.
  (`core/lesson-modality.json` reports `drivable` from the whole-lesson modality
  rather than the core, so the published manifest understates this; that is a
  known bug in `modality-manifest.ts`, not a property of these lessons.)
- All eight sit under the 300-second effective ceiling (285-299s computed).
- Wiring: `ML-PATH-027`/`ML-PATH-028` and `ML-EXT-027-MIND-VERBS`/
  `ML-EXT-028-DOING-VERBS` in [`curriculum.json`](./curriculum.json), all eight
  concepts dropped from the `SPINE-SAY-WHAT-I-DO` omission ledger (36 omits down
  to 28), Chapter 33 and 34 ledger entries in [`chapters.json`](./chapters.json),
  two `core/book-generation.json` targets, the generated
  `book/chapters/ch33-mind-and-page.tex` and
  `book/chapters/ch34-taking-asking-helping-liking.tex`, `\input` in `book.tex`,
  and generated narration for both chapters.
- The 136-page XeLaTeX build has **zero missing characters** and zero errors.
  Three underfull boxes remain, two of them pre-existing in Chapters 6 and 30;
  the third is the Chapter 34 payoff's section heading, where the Malayalam
  script at 14.4pt forces an awkward break. The track is `null` in
  `core/latex-warning-baseline.json`, so nothing is re-pinned.

## Chapter 32 — The Core Verbs (2026-08-06)

- Added six schema-v2 core-verb lessons, the track's first A2 material and its
  first realization of `SPINE-SAY-WHAT-I-DO`: `ML-C32-undu` (VERB-BE),
  `ML-C32-pokuka` (VERB-GO), `ML-C32-varuka` (VERB-COME), `ML-C32-tinnuka`
  (VERB-EAT), `ML-C32-kaanuka` (VERB-SEE), `ML-C32-ariyuka` (VERB-KNOW). All
  six take canonical spine concept tags, so the track goes from four namespaced
  verb concepts and none canonical to six canonical ones.
- The chapter is built around the fact that makes Malayalam unlike its three
  Dravidian sisters: **its verb carries no person marking at all**. Chapter 5
  observed this for one verb; Chapter 32 turns it into the chapter's spine.
  `ML-C32-undu` sets up the two-slot machine (stem + tense) against Tamil's
  three (stem + tense + person); `ML-C32-pokuka` shows that each tense form is
  therefore the *whole* conjugation; `ML-C32-varuka` locates the entire
  irregularity budget in the past (*varu-* → *vann-*); `ML-C32-kaanuka` shows
  the freed slot spent on mood (*kāṇāṁ*, *kāṇaṇaṁ*, *kāṇarutŭ*); and
  `ML-C32-ariyuka` closes by taking the person out of the subject slot too,
  giving Chapter 6's **എനിക്ക് മലയാളം അറിയാം** the verb it was always built on.
- Two genuinely conservative facts are recorded rather than glossed: Malayalam
  kept **both** of the family's be-verbs (ഉണ്ട് = Telugu ఉండు for existing and
  having, ഇരിക്കുക = Tamil இரு / Kannada ಇರು for being somewhere), and it kept
  the inherited *tiṉ-* and *kāṇ-* as its everyday eat- and see-words where
  Tamil, Kannada and Telugu each moved on. `ML-C32-tinnuka` also names the
  **-ഉക / -ഇക്കുക** split, which marks a native verb off from a Sanskrit
  borrowing turned into one.
- Every non-Malayalam form is supplied in full — no lesson assumes the reader
  knows another target language — and every cognate claim stays inside
  Dravidian, with Sanskrit material flagged as borrowing.
- Wired the chapter through the pipeline: `ML-PATH-026` in
  [`curriculum.json`](./curriculum.json) (dropping the six concepts from
  `SPINE-SAY-WHAT-I-DO`'s `omits`), a Chapter 32 ledger entry in
  [`chapters.json`](./chapters.json), a `core/book-generation.json` target, the
  generated `book/chapters/ch32-core-verbs.tex`, and `\input` in `book.tex`.
- All six lessons are **voice** modality, so Chapter 32 is drivable end to end,
  and all six sit under the 300-second effective ceiling (272–294s computed).
  The 114-page XeLaTeX build has zero missing characters and adds no over- or
  underfull boxes.

## Chapter capability ledger for Chapters 6–31 (2026-08-06)

- Added [`chapters.json`](./chapters.json), the track's HL05 chapter capability
  ledger: one `canDo` promise and one validated payoff for each of Chapters
  6–31. Titles and labels are copied from `core/book-generation.json` so the two
  agree until HL-C04 inverts that dependency; `spineNodes` are derived from
  `curriculum.json`'s path segments; every `payoff.assesses` atom is taken from
  the payoff lesson's own `practises.knowledge`, never invented.
- Derived from the lessons and `curriculum.json` rather than from
  [`roadmap.md`](./roadmap.md) or [`session-map.md`](./session-map.md), which
  still lag the canonical Chapters 6–31 (known debt, HL-M04).
- **Chapters 1–5 are deliberately absent.** Their terminal practice lessons
  (`ML-C01-practice` … `ML-C05-practice`) are still schema v1 and declare no
  `practises.knowledge`, so no payoff can name an atom without fabricating one.
  Those five chapters also have no `book-generation.json` target to copy a title
  from. The gap is recorded in the file's own `note` and stays visible to the
  HL05 gap report rather than being filled with a placeholder.
- No chapter from 6 on ends in a `practice` lesson, so every payoff is that
  chapter's last lesson by `sequence`. Where that terminal lesson is an
  `etymology` lesson (Chapters 23, 24, 31) the payoff is typed `task` and its
  summary describes the sorting the reader actually does.

## Warning-free complete book (2026-08-03)

- Added explicit static bold and italic faces for Malayalam and every
  comparison script, plus bookmark-safe Unicode commands, eliminating all
  font-shape and Hyperref warnings without dropping multilingual examples.
- Made the five handwritten recap labels unique and shortened only the running
  titles that exceeded the text block. Small sentence-level copy-flow repairs
  in the generated family, number, and colour chapters remove the remaining
  horizontal overflows while preserving the canonical teaching sequence.
- Added natural page bottoms for deliberately short micro-lessons and made
  open-right chapter versos truly empty, without a running header or page
  number.
- The forced 107-page build now has zero missing glyphs, overfull or underfull
  boxes, duplicate destinations, Hyperref warnings, LaTeX warnings, or font
  warnings. All 107 pages were rendered and visually inspected.
- The 33 top-level and 97 total outline entries, title and author metadata,
  generated source hashes, and zero schema or generator leaks remain intact.

## Canonical Chapters 6–31 in the book (2026-08-03)

- Migrated all thirty-three Malayalam lessons after Chapter 5 to the strict
  schema-v2 curriculum contract: canonical spine nodes, unique
  prerequisite-safe sequence, explicit sub-five-minute budgets, typed block
  boundaries, and closed knowledge introductions and assessments.
- Generated twenty-six LaTeX chapters from those canonical lessons instead of
  copying app content into a separate book source. The committed source-hash
  manifest is independently checked against Language Ladder for Chapters 6–31.
- Added a reusable Malayalam comparison-font set for Malayalam, Tamil, Telugu,
  Kannada, Devanagari, and Arabic-script examples. The 107-page PDF has zero
  missing glyphs and preserves the full 33-entry top-level chapter outline.
- Rendered and inspected all 107 pages, including dense case, calendar,
  etymology, daypart, and register sections. No teaching content is clipped,
  colliding, accidentally omitted, or replaced by generator metadata.
- The expanded artifact's cleanup baseline is 17 overfull boxes, four
  underfull horizontal boxes, ten underfull vertical boxes, four duplicate
  practice labels, 108 Hyperref warnings, and seven font warnings. `HL-B27`
  tracks those warnings and the running headers on intentionally empty versos.
- The single all-books publication gate still compiles and catalogs all twenty
  downloadable volumes successfully.

## Sub-five-minute lesson remediation (2026-08-02)

- All thirty-seven Malayalam duration violations are resolved. Thirty-three
  lessons already computed below five minutes and now declare an honest
  four-minute budget without changing their teaching content.
- Four long lessons become gentle prerequisite pairs: **ഉച്ച** noon →
  **പാതിരാ** midnight; Sanskrit *divasam/dinam* → native **നാൾ**; Sanskrit
  **രാത്രി** → native *iravŭ/iruḷ*; formal **ശുഭ മധ്യാഹ്നം** → the three-language
  convergence map. The eight steps compute between 141 and 235 seconds.
- The four support lessons bring the Malayalam track to 64 lessons with zero
  unknown prerequisite ids; downstream lessons now require the moved concept
  before using it.
- A forced book build succeeds at 31 pages with no missing glyphs. Canonical
  lessons continue through Chapter 31 while the book stops at Chapter 5
  (`HL-B26`); existing layout, bookmark, duplicate-label, and font warnings are
  tracked in `HL-B27`; roadmap and session-map drift is tracked in `HL-M04`.

## Chapter 6 — Case endings, and the sentence with no subject

- **Chapter 6 authored** (`ML-C06-dative-ikku`, `-dative-subject`): the track's
  first **case ending** — reviewing Ch.2/3/5 via `reviews_of`.
- **-ിക്ക്/-ിന്** (`ML-C06-dative-ikku`): the dative "to/for," taught as the doorway
  to **agglutination**. Malayalam **adds** a suffix carrying **one** meaning with
  the **seam visible** (*jōli* + *kku*), where a Latin ending like *-īs* **fuses**
  case+number+declension inseparably; the two shapes are **one case** chosen by the
  noun's ending. Includes *ñān* → **എനിക്ക്** *enikku*, flagged as worth memorising
  cold — it opens a great many everyday Malayalam sentences.
- **എനിക്ക് മലയാളം അറിയാം** (`ML-C06-dative-subject`): "I know Malayalam" — literally
  "**to-me Malayalam is-knowable**" — *aṟiyām* being *aṟiy-* "know" plus the
  **abilitative** *-ām*, not a passive — with **no nominative "I"** (contrast Ch.5's
  *ñān malayāḷam saṁsārikkunnu*). Explains the **dative-subject** rule with
  English's "**methinks**" as the bridge.
- **The Dravidian family thread**, new in this chapter: *-ikku / -ukku / -ku / -ge*
  are visibly the **same suffix**, with the extra observation that **Malayalam's
  *enikku* and Tamil's *enakku* are nearly the same word** — the two languages
  separated most recently of the four, and it shows.
- Taxonomy: namespaced `ML-CASE-DATIVE`, `ML-DATIVE-SUBJECT`.

## Chapters 3–5 — How-are-you, Farewells, First Verbs

- Three new chapters carry Malayalam to Chapter 5, matching the leading tracks'
  arc. One word per lesson, atom-first, Malayalam script inline; every root traced
  (`lessons/ML-C0{3,4,5}-*`, `book/chapters/ch0{3,4,5}-*.tex`). Concept tags reuse
  the universal `HL01` taxonomy; verbs namespaced (`ML-VERB-*`). Malayalam's
  double character — Tamil's closest sister, yet the deepest in Sanskrit, and the
  only one with a real copula — runs throughout.
- **Ch. 3 — How Are You**: *eṅṅane* (how; the native *e-* questions) → *sukhamāṇō?*
  ("are you well?" — the Ch.2 copula *āṇŭ* + the question particle *-ō*) → *ñān*
  (I ← Proto-Dravidian; **can't be dropped**, since Malayalam verbs don't mark
  person) → *sukham* (well ← Sanskrit *sukha*, the *su-* that is Greek *eu-*) →
  *sāramilla* ("no matter" = you're welcome; Sanskrit *sāraṁ* + native *illa*) →
  practice.
- **Ch. 4 — Farewells**: *pōkuka*/*varika* → *pōyi varāṁ* ("I'll go and come back,"
  tabled across the family) → *nāḷe kāṇāṁ* (see you tomorrow; *nāḷ* "day" + *kāṇ*
  "see" + the "let's" *-āṁ*) → *vīṇḍuṁ kāṇāṁ* (we'll meet again; native *kāṇ*,
  where Tamil borrowed Sanskrit *sandi*) → practice.
- **Ch. 5 — First Verbs**: *saṁsārikkuka* (Sanskrit-derived; native twin
  *paṟayuka*) → *ñān malayāḷaṁ saṁsārikkunnu* (I speak Malayalam; the *-unnu*
  present — **the verb never changes for person**, Malayalam's great
  simplification) → *tāmasikkuka* (to live; postposition *-il*) → *jōli ceyyuka*
  (to work; *ceyyuka* is the *same root* as Tamil *sey*) → practice. Book compiles
  clean with XeLaTeX (0 missing chars, 0 undefined refs).

## Chapter 2 — Introducing Yourself

- New chapter around the introduction dialogue (*enṟe pēru … āṇŭ / ninṟe pēru
  entāṇŭ?*), atom-first, Malayalam inline (`lessons/ML-C02-*`,
  `book/chapters/ch02-introductions.tex`). Every atom traced:
  - **പേര്** pēru ("name") ← Proto-Dravidian *\*pēr* — twin of Tamil *peyar*,
    **not** the Indo-European *name/nām*.
  - **എന്റെ** enṟe ("my") ← *ñāṉ* ("I").
  - **ആണ്** āṇŭ ("is") — Malayalam's **copula**, from the verb *āka*. The
    standout: Tamil/Kannada/Telugu use the **zero copula**, but Malayalam,
    Tamil's closest sister, grammaticalised a "to be" verb.
  - **എന്റെ പേര് … ആണ്** — **"my name is…"**; verb last (unlike Tamil).
  - **നീ / നിങ്ങൾ** nī/niṅṅaḷ — "you," familiar/respectful; respect by plural.
  - **എന്ത്** entŭ ("what") ← Dravidian question-stem *\*yā-/\*e-*.
  - **നിന്റെ പേര് എന്താണ്?** — **"what's your name?"** (*entŭ* + *āṇŭ* fused).
  - **സന്തോഷം** santōṣam — "pleased to meet you," a **Sanskrit** loan (Malayalam
    borrows selectively: native *nandi* for thanks, Sanskrit here).
  - **practice** — the whole dialogue.
- Example names are invented (Mira / Arun). Book compiles clean with XeLaTeX.

## Chapter 1 — Greetings (Malayalam script taught inline)

- New Malayalam track on the HL00 framework — the last of the four Dravidian
  tracks. One word per lesson, slug ids, atom-first, derivations shown, LaTeX
  book. Uses the **vendored** Noto Sans Malayalam font (relative `Path=`, shaped
  via `Script=Malayalam`, no polyglossia language module needed).
- **No reading course.** Per `HL00`'s inline-letters rule, Malayalam is taught
  *inside* each word lesson.
- Chapter 1 (`lessons/ML-C01-*`), greetings + conversational glue:
  - **നമസ്കാരം** namaskāram ("hello," **Sanskrit** namas + kāra) — inherent
    *a*, vowel signs, the chandrakkala, the സ്ക conjunct, anusvāram ം.
  - **നന്ദി** nandi ("thanks," **native**, root *nal*) — the twin of Tamil
    *naṉṟi*; the ന്ദ conjunct.
  - **അതെ** athe ("yes," native, "that [is so]") — yes/no as demonstratives;
    the *e*-sign written before its consonant.
  - **ഇല്ല** illa ("no / isn't," native, root *il*) — the twin of Tamil
    *illai*; negation by a negative existential verb.
  - **ശരി** śari ("okay," native) — the family word *sari* with Sanskrit ശ.
  - **practice** — recap + the *pōyi varām* farewell (nearly the same words as
    Tamil's *pōy varugiṟēṉ*).
- The recurring thread: **Malayalam is Tamil's closest sister** — four of the
  five everyday words are shared with Tamil (nandi, athe, illa, śari) — **with a
  heavy Sanskrit overlay** (namaskāram; the largest alphabet in the family).
  Each lesson carries an "Across the family" cognate box (English / Sanskrit /
  Hindi / Tamil / Kannada / Telugu), every form supplied so nothing is assumed.
  Book compiles clean with XeLaTeX. Completes the four Dravidian first chapters.
