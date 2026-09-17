## Unreleased — chapter 103: what I know and what I want, and three letters nobody had drawn

Hindi A1 exam coverage **230/282 (82%) → 232/282 (82%)**, closing two points —
and paying the script debt those points turned out to be sitting on.

| point | what was missing |
|---|---|
| `HI-A1-V-21` | *chāhiye* for wanting and needing |
| `HI-A1-V-24` | *jānnā*, to know |

### The first chapter authored with an honest glyph check

The old check said both words were fine. The corrected one — **taught means the
subject of a lesson, not merely present inside one** — said both were blocked:

| word | blocked on | credited to |
|---|---|---|
| चाहिए | **ए** | `HI-S117-letter-ca`, whose example list contains **एक** |
| जानना | **ज** | the same class of accident |
| मुझे | **झ** | genuinely never taught, and correctly reported |

`HI-A1-V-21`'s note had said this all along: *"It also needs the independent
vowel `e`, which no Hindi writing lesson has drawn — so this is a script lesson
as much as a vocabulary one."* **It was right, and the mechanical check denied
it.** When a note and a metric disagree, the note is the one that looked.

So the chapter carries **three letter lessons**, and all three had **sourced
stroke data sitting bundled and unused** in `data/scripts/devanagari.json` —
unused because the metric said the letters were done.

| lesson | glyph | chapter | why there |
|---|---|---|---|
| `HI-S131-letter-e` | ए | 29 | the standing form of the **◌े** taught at chapter 8 |
| `HI-S132-letter-ja` | ज | 30 | on the page since chapter 2, never drawn |
| `HI-S133-letter-jha` | झ | 31 | immediately before **साँझ** at sequence 830 |

**ज and झ are the third breath pair**, after त/थ and च/छ. Each pair has cost
less than the last, because the idea was learned once and only the shapes
change.

**झ is the first letter in the book to arrive before any word that needs it.**
Every other character was taught out of words the learner was already saying;
this one has nothing to be found inside yet, and the lesson says so rather than
reaching forward to a word from the next chapter.

### The headword that promised a word the atoms never delivered

`HI-A1-V-24`'s note says the negation lesson's example sentence is *main nahīṁ
jāntā*, so the learner meets the word while no atom introduces it. The real
shape is sharper. **जानता is in `HI-C07-nahin`'s headword** — `"नहीं / जानता"`,
glossed *"no / not / masculine present form of to know"* — while all three of
that lesson's atoms are about **नहीं** itself: its Sanskrit root, its
Indo-European cousins, its two jobs.

The lesson's own contract advertised the verb and never taught it. **A learner
could deny knowing something and could not claim it.**

The affirmative needs no new grammar. Take **नहीं** out and **हूँ** comes back,
because a Hindi present needs its copula and the negative drops it — which is
why the affirmative is not the negative with a word removed.

### Two forms of one root, told apart by what follows

| form | what follows | example |
|---|---|---|
| चाहता / चाहती | a **verb** | मैं जाना चाहता हूँ |
| चाहिए | a **thing** | मुझे चाय चाहिए |

**Ask what comes next, not what it means.** Both are *want* in English, so the
meaning cannot separate them; the shape of what follows can, every time.

**चाहिए never bends.** No gender, no number, the same five letters whoever is
speaking and whatever is wanted — unusual enough to be a landmark: if you hear
**चाहिए**, stop looking for agreement.

And its wanter is **dative**, which makes the sentence the liking sentence with
one word swapped:

| Hindi | word for word |
|---|---|
| मुझे चाय पसंद है | to me, tea is pleasing |
| मुझे चाय चाहिए | to me, tea is wanted |

**Wanting a thing and liking a thing happen to you; wanting to act is something
you do**, and Hindi draws that line with the case of the first word. The proxy
reaches the same shape in *me gusta* and Italian in *mi piace* — what is worth
a lesson is that Hindi extends it to **wanting**, where those languages switch
back to an ordinary verb.

### forward-language 21 → 22, and three of the four were mine

| entry | verdict |
|---|---|
| `HI-S132-letter-ja` shows **आज** | **mine** — *āj* is introduced 214 lessons later |
| `HI-S132-letter-ja` shows **जाना** | **mine** — 208 lessons later |
| `HI-S133-letter-jha` shows **समझना** | **mine** — 5 lessons later |
| `HI-C70-chahna` shows **चाहिए** | **real** — 171 lessons before anything introduced it |

A letter lesson illustrates itself with familiar words, and I reached for the
most natural ones without checking when they arrive. **ज** now uses **जी** and
**जनवरी**, both taught well before chapter 30; **झ** names no word at all.

The one that survives is genuine debt this chapter exposed: the wanting lesson
at chapter 70 mentions **चाहिए** in passing, 171 lessons before anything
introduces it. Reported, not hidden.

### Numbers

| metric | before → after |
|---|---|
| atoms taught | 509 → 516 |
| measurable lessons | 453 → 461 |
| writing-practice lessons | 104 → 107 |
| **never-taught glyphs** | **6 → 5** |
| **script-closure violations** | **30 → 29** |
| `forward-language` | 21 → 22 |
| `measurement-blind` | 16 → 17 |
| `payoff-surprise` | **unchanged** |
| `durationViolations` | **0** |

`neverTaughtGlyphs` falls by one because **झ** was among the six the metric
already admitted. **ए** and **ज** do not move it — the metric still thinks they
were taught, which is exactly the defect `HL-C383` records. By the honest count
this chapter takes the real figure **16 → 13**.

`measurement-blind` rises by one for the `type: synthesis` payoff, reported
rather than relabelled. Reinforcement misses 1184 → 1211, in step with nine new
lessons.

### Lessons

| id | headword |
|---|---|
| `HI-S131-letter-e` | ए |
| `HI-S132-letter-ja` | ज |
| `HI-S133-letter-jha` | झ |
| `HI-C95-jaanta-hun` | मैं जानता हूँ |
| `HI-C95-jaante-hain` | क्या आप जानते हैं? |
| `HI-C95-chahiye` | चाहिए |
| `HI-C95-mujhe-chahiye` | मुझे चाय चाहिए |
| `HI-C95-repaso-chaah` | चाह की तालिका |
| `HI-C95-sintesis-chaay` | आपको क्या चाहिए? |
