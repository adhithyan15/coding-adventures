## Fixed — spaced recalls that asked a driver to write are now writing cues

The drivable-writing detector in human-language-data now reads inside
`[YOU RECALL: …]` cues. RECALL is a spoken cue action, so the narration read
a spaced recall such as `[YOU RECALL: write **ば** — **R1**, one lesson back]`
to a driver as "your turn — recall: write ば — R1, one lesson back", with no
deferral (issue #12070). The drive-debt fix below could not see these: the
detector skipped every cue. 104 such cues sat in 67 drivable lessons, the
most of any track, because the hiragana chapters (14-18, 131-142) open each
lesson with two or three spaced recalls of the signs taught one, five,
twenty and eighty lessons back, and every one of those asks for the sign
written.

Each is now a `[YOU WRITE: … from memory]` cue, the form the Marwadi fix
used: WRITE is a manual action, so the narration says "once you have stopped
driving — write: …" and the book prints "*Write it:* …". Every lesson stays
`drivable: true`.

- **Chapters:** 14-18 (JA-C14-ni, -san, -yon; JA-C15-juu, -ku, -roku;
  JA-C16-futatsu, -hitotsu, -itsutsu; JA-C17-kokonotsu, -muttsu, -nanatsu,
  -yattsu; JA-C18-count-the-face, -hitori-futari, -hon, -nin; JA-R15-six-to-ten)
  and 131-142 (every hiragana, dakuten and handakuten lesson of those
  chapters that had the shape, and the review lessons JA-R134 to JA-R137 and
  JA-R142).
- The spacing tag (**R1**–**R4**) and the distance note stay in the cue,
  after "from memory": `[YOU WRITE: **ば** from memory — **R1**, one lesson
  back]`. A qualifier of the sign stays before it ("small **ゃ** from memory",
  "the small **っ** from memory", "full-size **つ** from memory"), and a
  stroke or size hint stays after it ("**けさ** from memory, three strokes for
  the first sign and three for the second").
- Two-word recalls put "from memory" after both words
  (`**いりぐち** and **でぐち** from memory`), so the cue still names the pair.
- Eight recall cues of the form "point to the sign in **おちゃ** you can
  already write, and the one you cannot" mention writing and ask for none, so
  they are unchanged and the detector leaves them alone. (They, and four more
  "point to the sign …" recalls, ask the learner to look at the page, which
  is a separate question for the driving edition.)
- The 19 non-drivable Japanese lessons with the same cue shape are untouched:
  their narration already opens with the hands-and-eyes notice.
- Regenerated: book chapters 14-18 and 131-142, their narration (`.json` and
  `.txt`), the generated book and narration hashes, and the 67
  `core/lesson-modality` owners (source hash only).

