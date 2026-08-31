## HL-C179 — glossed-not-taught is now a reproducible review queue

With both Spanish A1 mocks green, the broader audit returned to the oldest
mechanizable correctness risk in the backlog. `npm run report --
--glossed-not-taught <track>` now extracts every native-script token in a track,
subtracts every token taught as a lesson headword, and reports the remainder
with occurrence counts and lesson IDs. JSON output is available through the
existing `--format json` switch.

This remains deliberately a **report, not a gate**. A candidate may be a real
inline gloss, an etymological ancestor, or merely a word mentioned without a
translation. The command makes the review queue reproducible; it does not claim
that every row needs a new lesson.

The same audit also rechecked HL-C215. Its two Hindi mixed-script strings are no
longer present in the lessons, generated book, or narration, so that historical
"left unfixed" note is stale rather than current work. The old entry remains as
the discovery record; this entry records the current disposition.

The next tranche should read and classify the highest-frequency Hindi report
rows, promoting only genuinely pre-glossed words into teaching or explicit
discard evidence.
