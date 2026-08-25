### Changed — corpus pins moved by the Japanese track (HL-C40)

No source change: the Japanese track is content, and the package loaded it without
a code edit because `japanese/track.json` declares the script (the built-in
`LANGUAGE_SCRIPT` map was deliberately left alone, proving that path works). The
pinned corpus measurements moved, and each pin now records why:

- `registeredTracks`, `authoredBooks`, `schemas.tracks`, `books.tracks`: 21 → **22**,
  Japanese following Mandarin Chinese (HL-C39) as the 22nd track.
- `modality-manifest.test.ts`: `totalLessons` 1125 → **1133**, `voice` 724 → **725**,
  `sight` 348 → **355**, `chapterCount` 376 → **377**, `unstartableChapters`
  121 → **122**; `pen` stays 53 and the drivable share stays **64%**.
- `drivablePrefixTotal` does **not** move (558). Japanese ch1 opens on one of its
  seven `script` lessons, so the chapter's drivable prefix is zero — which is also
  why `unstartableChapters` gains one.
- The compiled-activity id list gains the eight `JA-C01-*` activities.

Seven of the eight Japanese lessons carry a `script` block and therefore derive as
`sight`. That is the honest classification — a kana or kanji shape cannot be read
aloud — and it was chosen over routing the same content through `input` blocks,
which would have held the drivable percentage flat by mislabelling it.

Added one integration test, `keeps the Japanese Chapter 1 mixed-script chain closed
and under five minutes`, which asserts the property rather than only the counts:
the same chapter carries a hiragana, a katakana, and a kanji headword; every lesson
is schema-v2 with exactly one compiled activity; nothing exceeds the duration
budget; and the plain and polite thanks keep distinct `register` values.

