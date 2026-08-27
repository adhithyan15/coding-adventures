# Japanese

This track teaches contemporary standard (Tokyo) Japanese through the shared
human-language spine. Every lesson is written for a learner who knows no
Japanese, takes no more than five minutes, and asks the learner to decode only
writing that an earlier step has made safe.

The current book is the first **pre-A1 slice**, not a claim of exam readiness.
Its 100 lessons form a deliberately slow twelve-chapter runway: 47 writing
lessons teach 47 distinct load-bearing glyphs; the current closure report finds
47 glyphs shown, 47 taught, zero never taught, and zero script-closure
violations. The vocabulary layer contains 28 word lessons and seven phrase
lessons. Those counts come from the canonical lesson ledger and the
`measureScriptClosure` report, not from a hand-maintained inventory.

## The opening runway

The first seven chapters separate what used to be one crowded doorway lesson
into one small job at a time:

1. write and read **はい / いいえ**;
2. add the signs needed for **こんにちは** and learn why final **は** says *wa*;
3. build plain **ありがとう** sign by sign;
4. add **ございます** and make the first register choice;
5. build **日本語** from practised kanji and components;
6. build **コーヒー** from two katakana signs and the long-vowel mark; and
7. retrieve the whole doorway exchange with listening, speaking, reading, and
   writing scored separately.

Chapter 8 keeps the same order for **さようなら**: sound and social meaning
first, then three new signs, then decoding and a four-skill payoff. Chapters 9
and 10 give the learner survival repair language — “I do not understand,” “one
more time, please,” and “a little more slowly.” Chapters 11 and 12 add fourteen
body words only after the required kana are available, with twelve interleaved
retrieval lessons before the second body-map payoff.

Writing is not postponed until the learner has “finished kana.” A new form moves
through visible-model observation and tracing, guided copy, delayed copy, and
dictation/transcription in small steps. Later levels add controlled composition,
connected composition, and timed production under the same five-minute lesson
ceiling. The complete pre-A1-to-C2 destination is defined in
[`assessment.json`](./assessment.json): official JLPT receptive anchors where
they apply, paired with project-defined JF Standard/CEFR-aligned speaking,
writing, and interaction papers. Finishing the eventual book must prepare a
learner to pass each skill independently; recognition cannot compensate for a
weak productive skill.

## What this track has to do differently

Japanese needs three writing systems at once, but it does not introduce them all
at once. Hiragana carries grammar, kanji carries content words, and katakana
carries many borrowings. Each system first appears only when a useful word needs
it, and each unfamiliar shape gets its own short runway before it becomes
load-bearing.

- **The etymology method is redirected, not dropped.** Japanese has no useful
  shared taproot with English. The track uses the real Sino-Japanese layer,
  internal histories such as **ありがとう** from 有り難し (“hard to exist”), and
  genuine shared borrowings such as **コーヒー** and English *coffee* ultimately
  travelling from Arabic *qahwa*. Where no honest connection exists, the lesson
  says so.
- **“The letters in this word” spans systems.** HL01 gives a track one `script`
  id, so hiragana, katakana, and kanji share
  [`../data/scripts/japanese.d/`](../data/scripts/japanese.d/). A per-sign
  `role` distinguishes them.
- **A kanji is taught as a set of readings.** 日 may be *nichi*, *jitsu*, *hi*,
  *bi*, or *ka*. Lessons teach the reading selected by the word in front of the
  learner; they do not pretend a character has one sound.
- **Politeness is grammar.** `plain-casual`, `teineigo-polite`, and
  `neutral-across-levels` are explicit register values because a Japanese
  predicate cannot be taught honestly as socially neutral.

## Read and practise

- [`session-map.md`](./session-map.md) maps one canonical lesson to each of the
  current 100 five-minute sessions and explains the review rule.
- [`roadmap.md`](./roadmap.md) records the twelve authored chapters and the
  dependency order toward the complete pre-A1-to-C2 destination.
- [`chapters.json`](./chapters.json) is the authoritative chapter and payoff
  ledger; [`curriculum.json`](./curriculum.json) supplies the canonical lesson
  path.
- [`assessment-spec.md`](./assessment-spec.md) and
  [`assessment.json`](./assessment.json) define the seven four-skill assessment
  rungs and their writing requirements.
- [`pronunciation-reference.md`](./pronunciation-reference.md) is a lookup aid,
  never a prerequisite chapter.
- [`lessons/`](./lessons/) contains the canonical short lessons; the generated
  book and app both consume that data.

Japanese is set in Noto Sans JP, vendored as a subset under
[`../_fonts/`](../_fonts/) under the SIL Open Font License.
