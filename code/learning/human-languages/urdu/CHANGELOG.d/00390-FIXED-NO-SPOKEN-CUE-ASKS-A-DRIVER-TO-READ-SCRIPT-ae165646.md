## Fixed — no spoken cue asks a driver to read script

`[YOU RUN: …]` is a spoken cue action, so the narration reads it to a driver
as an ordinary turn. UR-C05-practice, a drivable lesson, ended its guided
practice with "run: both voices once from the romanization, then read the
closing line off the script alone", asking someone at the wheel to read
**خدا حافظ** in Urdu script (issue #12070). The reading check in
human-language-data now reads every spoken cue and counts "script" as an
object on the page, and the cue is split in the authored order:
`[YOU RUN: both voices once from the romanization]`, then `[YOU READ: the
closing line off the script alone]`, which the narration defers ("once you
have stopped driving — read: the closing line off the script alone"). The
cue was wrapped across two source lines; both new cues sit on one line each.
The lesson stays `drivable: true` (only its `core/lesson-modality` source
hash changes).

- **Count:** 1 cue in 1 drivable lesson.
- Judgement call: "from the romanization" stays in the RUN cue as authored.
  The lesson says every line of the exchange is already learned by ear, the
  romanization is the support for that spoken run, and the check this pass
  extends is about printed script; whether a drivable run should lean on a
  printed romanization at all is a separate question for this lesson's
  author.
- Regenerated: book chapter 5 and its hash, the narration and narration hash
  for the same chapter, and the `core/lesson-modality` owner (source hash
  only).
