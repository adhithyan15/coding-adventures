# Japanese

This track teaches contemporary standard (Tokyo) Japanese through the shared
human-language spine. Every lesson is written for a reader who knows no Japanese,
takes under five minutes, and introduces only the writing the expression in front
of them actually needs.

Chapter 1 opens all three of Japan's writing systems, because a single ordinary
sentence uses all three: **はい** and **いいえ** establish hiragana and the mora;
**こんにちは** adds four signs and the topic particle **は**, read *wa*;
**日本語** opens kanji; **コーヒー** opens katakana; **ありがとう** adds the
voicing mark and its own etymology; **ありがとうございます** names the fact that
Japanese politeness is grammar, not word choice. A six-line doorway exchange
closes the chapter using nothing that was not taught.

Chapter 2 turns the reading round. Chapter 1 shows a reader whole words in three
scripts; **Chapter 2 teaches eight hiragana signs, one per lesson**, until three of
those words can be read from their parts rather than recalled as shapes. Two of its
ten lessons introduce nothing new at all — they exist to mark the moment the pieces
become a word.

The sign for *wa* is taught **last, and on purpose**. The daytime greeting sounds
like it ends in *wa* and is written with the sign read *ha*, because that sign is
also the topic marker. Teaching the *wa* sign before that fact is established
would manufacture the commonest beginner spelling error in Japanese.

Eight of forty-six basic hiragana are done. The remaining 35 glyphs this track
shows — 16 hiragana, 10 katakana, 9 kanji — arrive the same way, and the katakana
and kanji sets are separate writing systems with ramps of their own.

## What this track had to do differently

Japanese is the first track in this corpus with no shared ancestry with English,
and the first that needs more than one writing system at once. Three departures
are deliberate and are recorded here rather than buried:

- **The etymology method is redirected, not dropped.** HL00's cousin web assumes
  a taproot the reader already owns through English. Japanese has none, and this
  track does not invent one. What it uses instead is real: the **Sino-Japanese**
  layer, where 日本語 has checkable relatives in Mandarin *Rìběnyǔ* and Korean
  *ilbon-eo*; **internal** etymology, where ありがとう decomposes into 有り難し
  "hard to exist"; and genuine **shared borrowings**, where コーヒー and English
  *coffee* both descend from Arabic *qahwa* by different routes. Where no honest
  connection exists — **はい** is a case — the lesson says so.
- **"The letters in this word" spans systems.** HL00's rule assumes one script
  per track. こんにちは is hiragana, 日本語 is kanji, コーヒー is katakana, and a
  normal sentence mixes them. The rule still holds if "letters" is read as "the
  writing this word needs," which is how the lessons apply it, but the data layer
  still gives a track exactly one `script` id — so hiragana, katakana, and kanji
  share one inventory file, [`../data/scripts/japanese.json`](../data/scripts/japanese.json),
  with a per-sign `role` distinguishing them.
- **A kanji is taught as a set of readings.** 日 is *nichi*, *jitsu*, *hi*, *bi*,
  or *ka*. No lesson claims a character has a sound; lessons teach the reading the
  **word** selects, and name the on/kun split as the reason there are several.

The `register` frontmatter field is a plain string, and it is doing more work here
than anywhere else in the corpus: `plain-casual` and `teineigo-polite` are not two
styles of one form but two grammatical levels, and the full keigo system adds
speaker-humbling and listener-elevating axes on top. This track uses explicit
values (`plain-casual`, `teineigo-polite`, `neutral-across-levels`) as a stopgap.

## Read and practise

- [`roadmap.md`](./roadmap.md) orders the authored and planned chapters toward B1.
- [`session-map.md`](./session-map.md) composes the eight lessons into sessions
  with an exact review ledger through S23, and records which of them need eyes.
- [`pronunciation-reference.md`](./pronunciation-reference.md) collects the sound
  and script facts for lookup; it is never a prerequisite chapter.
- [`chapters.json`](./chapters.json) is the HL05 capability ledger: Chapter 1's
  first-person "I can …" promise and the payoff lesson that proves it.
- [`curriculum.json`](./curriculum.json) is the ordered shared-spine realization
  path plus the seven Japanese-specific extension nodes.
- [`lessons/`](./lessons/) holds the eight canonical short practice lessons.
- [`book/book.tex`](./book/book.tex) builds the free starter edition with
  XeLaTeX; Chapter 1 is generated from the canonical lessons, so the book and the
  app never drift.

Japanese is set in Noto Sans JP, vendored as a subset under
[`../_fonts/`](../_fonts/) under the SIL Open Font License.
