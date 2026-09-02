# German

The third track of the [Human Languages](../README.md) curriculum, built the
same way as [Spanish](../spanish/README.md) and [French](../french/README.md):
one word per lesson, taken apart and traced to its root; every noun's gender
learned with the noun; the pieces taught before the whole; and a book you can
read straight through.

## What's different about the German track

German flips the etymology channel. Spanish and French are Latin's children;
**German is English's sibling** — both Germanic — so German words are usually
*direct cousins* of English words, no Latin middleman (*gut* = *good*, *Tag* =
*day*, *Nacht* = *night*). The differences follow the **High German Consonant
Shift** (*d→t*, *t→s*, *k→ch*), taught once and reused as a decoder — German's
counterpart to Spanish's *-ct-→-ch-* rules. Three genders (*der/die/das*),
not two — German kept the neuter Spanish and French dropped. And *gut* means
both "good" *and* "well," collapsing the Romance *bueno/bien* split.

The showpiece is **Nacht**: German *Nacht*, English *night*, Spanish *noche*,
French *nuit* — one Indo-European word, split four ways.

## Progress

- **Chapter 1 — Greetings** ([`lessons/GE-C01-*`](./lessons/)): hallo, gut,
  der/die/das (gender), Tag, Guten Tag, Morgen, Guten Morgen, Abend, Guten
  Abend, Nacht, Gute Nacht, practice. In the book.
- **Chapter 2 — Introducing Yourself** ([`lessons/GE-C02-*`](./lessons/)): ich,
  heißen, **ich heiße** ("my name is"), **du / Sie**, wie, **wie heißen Sie?**
  ("what's your name?"), freut mich, practice. In the book.
- **Chapter 3 — How Are You**: danke, bitte, gehen, *wie geht es*, *es geht*,
  practice.
- **Chapter 4 — Farewells**: auf Wiedersehen, tschüss, bis bald, bis morgen,
  practice.
- **Chapter 5 — The First Verbs**: wohnen, machen, lernen, *ich lerne Deutsch*,
  practice.
- **Chapter 6 — Numbers One to Ten**: Zahlen 1–5, 6–10 (Grimm's law).
- **Chapter 7 — The Days of the Week**: Wochentage (the gods, and Mittwoch).
- **Chapter 8 — Telling the Time**: Uhr, Mittag/Mitternacht.
- **Chapter 9 — Months and Seasons**: Monate, Jahreszeiten (*Herbst*/harvest).
- **Chapter 10 — Family**: Eltern, Geschwister.
- **Chapter 11 — Bread, Water, Wine**: Brot, Wasser/Wein.
- **Chapter 12 — Numbers Eleven to Twenty**: elf/zwölf (the *-lif* "left over"
  story), 13–20.
- **Chapter 13 — Colours**: schwarz/weiß, rot/blau.
- **Chapter 14 — To Have, and How Old You Are**: haben (the *habēre* false
  cognate), Alter.
- **Chapter 15 — The Two Past Tenses**: Perfekt, Präteritum.
- **Chapter 16 — I Am, You Are**: *sein*, met one present form per lesson —
  *bin*, *bist*, *ist*, *sind*, *seid* — plus *müde*, and the grid only at the
  end, as a recap.
- **Chapter 17 — Three Verbs Under One Roof**: *war/warst/waren*, the three
  Proto-Indo-European roots inside one infinitive, suppletion, and why the
  commonest words are the last to be regularised.
- **Chapter 18 — The Past That Takes To Be**: the *sein*-perfect —
  *kommen*, *fahren*, *werden*, *bleiben*, the learned list, and the
  participle that agrees with nothing where French has four forms.
- **Chapter 19 — Head and Hand**: *der Kopf*, *Haupt*, *die Hand*.
- **Chapter 20 — Yes and No**: *ja*, *nein*, and the negative-answer *doch*.
- **Chapter 21 — Please**: *Wasser, bitte* from previously learned words.
- **Chapter 22 — Sorry**: *Entschuldigung*, *es tut mir leid*.
- **Chapter 23 — Weather**: *das Wetter*, *es ist heiß/kalt*, *es regnet*.
- **Chapter 24 — Dog and Cat**: *Hund*, *Katze*.
- **Chapter 25 — Green and Yellow**: *grün*, *gelb*.
- **Chapter 26 — Verbs of the Mind**: *denken*, *verstehen*, *lesen*,
  *schreiben* — and the strong-verb vowel break (*du liest*).
- **Chapter 27 — Taking, Asking, Helping, Liking**: *nehmen*, *fragen*,
  *helfen*, *mögen/lieben* — and *gern*, German'''s third way of liking.
- **Chapter 28 — Sitting, Standing, Sleeping, Hearing**: *sitzen*, *stehen*,
  *schlafen*, *hören* — the second sound shift's *t*-branch, and a second way
  for a strong verb to break (*du schläfst*).
- **Chapter 29 — Going, Running, Opening, Closing**: *gehen*, *laufen*,
  *rennen*, *öffnen*, *schließen* — where German's walk/run line actually
  falls, and the first separable verbs (*Ich mache die Hand auf*).
- **Chapter 30 — Coffee, Tea, and Milk**: *der Kaffee*, *der Tee*, *die
  Milch* — extends the *Wasser, bitte* request pattern to two loanwords
  (Arabic/Turkish/Italian; Hokkien Chinese by way of Dutch) and one native
  word.
- **Chapter 31 — Friend and Family**: *der Freund*, *die Freundin* (the
  native *-in* feminine suffix), *die Familie* (the one Latin loan in the
  chapter).
- **Chapter 32 — Eyes, Ears, Mouth, Nose**: *das Auge*, *das Ohr*, *der
  Mund*, *die Nase* — extends Chapter 19's body-part material to the rest of
  the face.
- **Chapter 33 — Arm, Finger, Foot, Heart**: *der Arm*, *der Finger*, *der
  Fuß*, *das Herz* — completes the five-word Hand/Arm/Finger/Fuß/Herz set
  Chapter 19 named but only a fifth of which it taught.

**All thirty-three chapters are authored and in the book (279 pages).**

---

## For contributors

Everything below this line is about how the track is built and checked. It is
here for people working on the curriculum; nothing in it is needed to learn the
language.

## Chapter capabilities (HL05)

[`chapters.json`](./chapters.json) states what a reader can *do* when they
finish a chapter, and names the lesson that proves it. It is authored intent —
no validator may rewrite it.

**Twenty-two of thirty-three chapters are atom-scored: 1–4 and 16–33.** Those
are exactly the chapters whose lessons have been migrated to schema version 2
and so declare real knowledge atoms. Chapters 5–15 are still schema v1 and carry
no `practises.knowledge`, so a payoff written for them could only assess
invented atoms — they carry an authored payoff anchored to real lesson content
and an empty `assesses` until the migration reaches them. They are left out on purpose: an absent entry is debt the gap report can
measure, a stub is a chapter falsely claiming a capability it never delivered.

Representativeness — the share of a chapter's introduced atoms its payoff
actually assesses, floored at 0.5 by `core/chapter-policy.json`:

| Chapter | Payoff lesson | Assessed / introduced |
|---|---|---|
| 19 Head and Hand | `GE-C17-hand` | 4 / 12 = 0.33 — **below the floor** |
| 20 Yes and No | `GE-C18-nein` | 5 / 8 = 0.63 |
| 21 Please | `GE-C19-bitte-requests` | 3 / 3 = 1.00 |
| 22 Sorry | `GE-C20-entschuldigung` | 3 / 3 = 1.00 |
| 23 Weather | `GE-C21-das-wetter` | 5 / 5 = 1.00 |
| 24 Dog and Cat | `GE-C22-hund-katze` | 5 / 5 = 1.00 |
| 25 Green and Yellow | `GE-C23-gruen-gelb` | 5 / 5 = 1.00 |
| 26 Verbs of the Mind | `GE-C24-schreiben` | 10 / 10 = 1.00 |
| 27 Taking, Asking, Helping, Liking | `GE-C25-moegen-lieben` | 10 / 10 = 1.00 |
| 28 Sitting, Standing, Sleeping, Hearing | `GE-C26-hoeren` | 10 / 10 = 1.00 |
| 29 Going, Running, Opening, Closing | `GE-C27-schliessen` | 10 / 10 = 1.00 |
| 30 Coffee, Tea, and Milk | `GE-C28-milch` | 9 / 9 = 1.00 |
| 31 Friend and Family | `GE-C29-familie` | 9 / 9 = 1.00 |
| 32 Eyes, Ears, Mouth, Nose | `GE-C30-nase` | 12 / 12 = 1.00 |
| 33 Arm, Finger, Foot, Heart | `GE-C31-herz` | 12 / 12 = 1.00 |

Chapter 19 is the one authored chapter that fails. It runs three word lessons
deep — *Kopf*, *Kopf/Haupt*, *Hand* — with no terminal consolidation lesson, so
its payoff can only be the last lesson by `sequence` and reaches a third of the
chapter. The `assesses` list is **not** padded to hide that: the honest fix is a
real Kopf/Haupt/Hand practice lesson. Chapter 20 has the same missing-practice
shape but clears the floor because *nein* reassesses *ja*.

Chapters 26 and 27 close over **all** of their own atoms, and both payoffs also
reach back past their own chapter — chapter 26's to `GE-LEX-HAND-02`,
`GE-ETYMON-HAND-MANUS-05` and `GE-SOUND-GRIMMS-LAW-04` from chapter 19;
chapter 27's to all four of chapter 26's verbs plus `GE-LEX-HUND-02`,
`GE-LEX-KATZE-04` (ch. 24) and `GE-LEX-WETTER-02` (ch. 23). That is HL09 §7:
a payoff scoped only to its own chapter adds to the orphan pile rather than
draining it.

Chapters 28 and 29 do the same and were written to drain it deliberately.
Chapter 28's payoff reaches to `GE-SOUND-GRIMMS-LAW-04` (ch. 19),
`GE-LEX-HUND-02`/`GE-ETYMON-HUND-03`/`GE-LEX-KATZE-04` (ch. 24) and all three
of chapter 27's closing atoms; chapter 29's reaches to `GE-LEX-NEHMEN-02`,
`GE-ETYMON-NEHMEN-03` and `GE-ETYMON-HELFEN-08` (ch. 27), `GE-LEX-HAND-02` and
`GE-SOUND-HAND-03` (ch. 19), and back into chapter 28. Six atoms that no lesson
had ever revisited are revisited here: `GE-SOUND-HAND-03`, `GE-ETYMON-HUND-03`,
`GE-LEX-REGNET-05`, `GE-LEX-MOEGEN-LIEBEN-09`, `GE-ETYMON-MOEGEN-LIEBEN-10` and
`GE-GRAMMAR-GERN-11`. The track's never-revisited share falls from **31 of 61
atoms (51%) to 27 of 81 (33%)**.

Chapters 30–33 are the pre-A1 vocabulary tranche (fourteen nouns; see
CHANGELOG). All four payoffs close over their own chapter's atoms at 1.00
representativeness. Chapter 30's payoff also rescues chapter 29's two
never-revisited atoms, `GE-LEX-SCHLIESSEN-10` and `GE-ETYMON-SCHLIESSEN-11`;
chapter 32's rescues chapter 28's disputed `GE-ETYMON-HOEREN-10` "sharp-eared"
link, never revisited since it was flagged; chapter 33's reaches to chapters
10, 13, 19, 24 and 28 at once, completing the Hand/Arm/Finger/Fuß/Herz set
chapter 19 printed but only a fifth of which it taught.

## Reinforcement chaining (HL09 §7)

A chapter-end payoff cannot close the R1 window (n+1…n+3), so the reach-back
runs at two cadences. Every lesson in chapters 26–27 also names atoms from the
**one to three lessons immediately before it**, across the chapter seam:

| Lesson | Reaches back to |
|---|---|
| `GE-C24-denken` | ch. 23 weather (`GE-LEX-WETTER-02`, `GE-GRAMMAR-WEATHER-SEIN-04`) — *Ich denke, es ist kalt* |
| `GE-C24-verstehen` | `GE-C24-denken`; ch. 25 `GE-ETYMON-GRUEN-03` — the built-twice-not-shared parallel |
| `GE-C24-lesen` | `GE-C24-verstehen`, `GE-C24-denken`, ch. 25 `GE-ETYMON-GRUEN-03` |
| `GE-C24-schreiben` | all three earlier chapter-24 lessons + ch. 19 |
| `GE-C25-nehmen` | `GE-C24-schreiben`, `GE-C24-lesen`, ch. 19 `GE-LEX-HAND-02` |
| `GE-C25-fragen` | `GE-C25-nehmen`, `GE-C24-lesen`, ch. 20 `ja`/`nein`/`doch` |
| `GE-C25-helfen` | `GE-C25-fragen`, `GE-C25-nehmen`, ch. 19 and ch. 21 |
| `GE-C25-moegen-lieben` | all of chapters 26 and 27, plus chapters 23 and 24 |
| `GE-C26-sitzen` | `GE-C25-moegen-lieben`, `GE-C25-helfen`, `GE-C24-lesen` — *Ich sitze gern*, and the *p*-branch beside the new *t*-branch |
| `GE-C26-stehen` | `GE-C26-sitzen`; ch. 26 `GE-C24-verstehen`, which had *stehen* inside it |
| `GE-C26-schlafen` | `GE-C26-stehen`, `GE-C26-sitzen`, ch. 26's vowel break, ch. 23 `GE-LEX-REGNET-05` |
| `GE-C26-hoeren` | all of chapter 28, plus chapters 19, 24 and 27 |
| `GE-C27-gehen` | `GE-C26-hoeren`, `GE-C26-stehen`, `GE-C26-schlafen`, `GE-C26-sitzen` |
| `GE-C27-laufen` | `GE-C27-gehen`; ch. 28's umlaut break, ch. 27 `GE-ETYMON-HELFEN-08`, ch. 24 `GE-LEX-HUND-02` |
| `GE-C27-oeffnen` | `GE-C27-laufen`, `GE-C27-gehen`, ch. 27 `GE-ETYMON-HELFEN-08`, ch. 19 *Hand* |
| `GE-C27-schliessen` | all of chapter 29, plus chapters 19, 27 and 28 |
| `GE-C28-kaffee` | ch. 21 `bitte` pattern; ch. 29's orphaned `GE-LEX-SCHLIESSEN-10`/`GE-ETYMON-SCHLIESSEN-11` |
| `GE-C28-tee` | `GE-C28-kaffee` |
| `GE-C28-milch` | all of chapter 30, plus ch. 21's `bitte` pattern again |
| `GE-C29-freund` | `GE-C28-milch` |
| `GE-C29-freundin` | `GE-C29-freund` |
| `GE-C29-familie` | all of chapter 31 |
| `GE-C30-auge` | `GE-C29-familie`; ch. 19 `GE-LEX-KOPF-02` |
| `GE-C30-ohr` | `GE-C30-auge`; ch. 28 `GE-LEX-HOEREN-09`/`GE-ETYMON-HOEREN-10` |
| `GE-C30-mund` | `GE-C30-ohr` |
| `GE-C30-nase` | all of chapter 32, plus ch. 19 `GE-LEX-KOPF-02`/`GE-LEX-HAND-02` |
| `GE-C31-arm` | `GE-C30-nase`; ch. 19 `GE-LEX-HAND-02` |
| `GE-C31-finger` | `GE-C31-arm` |
| `GE-C31-fuss` | `GE-C31-finger`; ch. 19 `GE-SOUND-GRIMMS-LAW-04` (via the prerequisite chain) |
| `GE-C31-herz` | all of chapter 33, plus ch. 19 `GE-LEX-HAND-02`, ch. 24 `GE-ETYMON-HUND-03`, ch. 28 `GE-ETYMON-HOEREN-10` |

The field that carries this is `practises.knowledge`. `reviews_of` names lesson
ids, not atoms, so it cannot close a reinforcement window and never has.

## Files

- [`chapters.json`](./chapters.json) — the HL05 chapter capability ledger.
- [`lessons/`](./lessons/) · [`pronunciation-reference.md`](./pronunciation-reference.md)
  · [`roadmap.md`](./roadmap.md) · [`session-map.md`](./session-map.md)
  · [`book/`](./book/) (`code/scripts/check-book-compile.sh --strict german` from the repository root)

Lessons are slug-named (e.g. `GE-C01-tag`); order lives in the book (LaTeX
auto-numbers) and `session-map.md`.
