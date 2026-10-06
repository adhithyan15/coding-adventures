---
category: Repo policy / workflow reminders
---

# A stroke-order animation's restarts show a teaching pace, not how often native writers lift

**Context:** the Devanagari ductus in `script-ductus`
(`src/strokes/devanagari.ts`) and the letter records in
`code/learning/human-languages/data/scripts/devanagari.json`, shared by the
Hindi, Marathi, Sanskrit and Marwadi tracks. Most consonants cite
Opiaterein's or JackPotte's stroke-order GIFs on Wikimedia Commons; the
vowels cite Saurmandal's panel diagrams.

**What happened:** each GIF restart (the pen tip jumping to a new place,
usually after a 200-250 ms hold) and each new panel was recorded as a pen
lift. That is a real lift in the animation, so it passed the rule written
after the Tamil fix ("count a lift only when the source shows one"). But the
animations build a letter one part at a time for a learner: क came out as
four strokes, य and ल as four, औ as seven. HP Labs India's native-writer
data (hpl-dvng-iso-char, counted from the prototypes in the MIT-licensed
LipiTk 4.0 Devanagari recognizer) shows 1% of writers drawing क in four
strokes, 0% for य, and at most 15% for nine more letters. Most write the
body and the right stem in one run and add the headline last. The Devanagari
ledger averaged 2.39 lifts per glyph against a native modal 1.74.

**Why:** a teaching animation slows the letter down and stages it. Its
restarts are evidence of *order* and *direction*, not of how many times a
fluent writer lifts the pen. JackPotte's GIFs already joined some of the
parts Opiaterein's separated (य's curl and bowl, र's loop and tail), which
was a hint that the restarts were a pedagogical choice.

**Fix:** क, य, र, प, ध, ल, द, ठ, घ, ष and औ were re-segmented to the native
modal stroke count. Every animated run is kept as a segment, with its start
and direction; where the next run starts elsewhere, the path climbs or
retraces ink it is about to draw or has drawn (up the stem, then down it).
Each record now cites the native count and share next to the animation.

**Do differently:** before taking a lift count from any stroke-order source
(numbered arrows, GIF restarts, panels, a primer), check it against
native-writer data where it exists: the pen-up counts in HP Labs India's
LipiTk 4.0 Indic character recognizers (lipi-reco-indic-char4.0.0 on
SourceForge) cover Devanagari, Telugu, Tamil and Bangla. Cite counts and
shares only; the underlying datasets may not be redistributed. If fewer than
about 15% of native writers use the source's count, keep the source's order
and directions as segments and lift only as often as native writers do.
