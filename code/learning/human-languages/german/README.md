# German

The third track of the [Human Languages](../README.md) curriculum, on the
same [`HL00`](../../../specs/HL00-human-language-curriculum-framework.md)
framework as [Spanish](../spanish/README.md) and [French](../french/README.md):
one word per lesson, slug ids, gender-before-nouns, atom-first assembly,
derivations shown, LaTeX book.

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
- **Chapter 16 — To Be, and the Past That Takes It**: sein (three ancient verbs
  in one paradigm), the Perfekt with *sein*.
- **Chapter 17 — Head and Hand**: *der Kopf*, *Haupt*, *die Hand*.
- **Chapter 18 — Yes and No**: *ja*, *nein*, and the negative-answer *doch*.
- **Chapter 19 — Please**: *Wasser, bitte* from previously learned words.
- **Chapter 20 — Sorry**: *Entschuldigung*, *es tut mir leid*.
- **Chapter 21 — Weather**: *das Wetter*, *es ist heiß/kalt*, *es regnet*.
- **Chapter 22 — Dog and Cat**: *Hund*, *Katze*.
- **Chapter 23 — Green and Yellow**: *grün*, *gelb*.

**All twenty-three chapters are authored and in the book (104 pages).** Chapters
17–23 are generated from the same canonical lesson AST and source hashes that
Language Ladder verifies independently.

## Files

- [`lessons/`](./lessons/) · [`pronunciation-reference.md`](./pronunciation-reference.md)
  · [`roadmap.md`](./roadmap.md) · [`session-map.md`](./session-map.md)
  · [`book/`](./book/) (`latexmk -xelatex book.tex`)

Lessons are slug-named (e.g. `GE-C01-tag`); order lives in the book (LaTeX
auto-numbers) and `session-map.md`.
