## HL-C286 — the parity gate's false gap: an allowlist of HEADINGS is not an inventory of PROSE

Arabic chapter 2 reported a parity gap of **5**, all `sounds`, right up until the
moment it was retired. Every one of those five blocks was already in the lessons.
The gap was an artefact of the measuring instrument, and it is the kind that
looks exactly like real debt.

`handwritten_parity.py` maps one environment to one heading:

    \begin{sounds}  <-  "## Sounds you'll need ..."

The five sections in question are headed **"The letters in this word"**. That
heading is not unknown to the pipeline — `classifyBlock` in `parse.ts` maps it to
`script`, deliberately, with a comment saying so; it is the heading 240 lessons
across 12 tracks use for inline letter teaching. But the parity script models
only the four environments it named, `script` not among them, so the markdown
side scored zero and the `.tex` side scored five.

**Why this is worth an entry.** HL-C134 already recorded the mirror-image bug —
counting only what the TARGET format can emit undercounts what the SOURCE would
lose, which took the corpus figure from 140 to 217. This is the same mistake
pointed the other way: counting only the headings you thought of undercounts what
the source ALREADY HAS, which invents debt that is not there. Both failures come
from an allowlist standing in for an inventory.

The generated chapter settles it empirically. `cousinweb` and `grammarlens` came
across at identical counts (5 and 3), and the phrase opening all five disputed
sections appears five times in the generated `.tex` and **zero** times in the
hand-written one — the prose had been rewritten on its way into LaTeX, so a text
diff would have been just as blind. The chapter grew 222 -> 512 lines.

**What to do about it.** Before treating a nonzero gap as work, check whether the
missing environment has a DIFFERENT heading that `classifyBlock` already
recognises. Two conservative fixes to the script would each have caught this:
teach `BLOCKS` every alias `classifyBlock` knows (they are one screen apart in
the same repo and are already meant to mirror each other), or report the
markdown-side blocks it could NOT classify, so a heading the script does not
model shows up as an unknown rather than as a silent zero.

Neither is done here, because doing it while the last chapters are mid-flight
would move every track's number at once. It is the right next change to the
script once the retirement is finished.
