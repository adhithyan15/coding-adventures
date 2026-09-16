---
category: Repo policy / workflow reminders
---

# A markdown table wider than three columns is silently dropped from the audiobook

Spanish chapters 421-423 were authored with five four-column teaching tables.
Every `check:*` gate passed, the books compiled under XeLaTeX, and nothing in
the authoring loop complained. The full test suite then failed on one number in
`tests/narration.test.ts`: the count of tables the narrator **refuses** to read
aloud went 44 to 49.

The narrator linearises tables up to `maxLinearisableTableColumns: 3`. Anything
wider is replaced in the audio with "Come back and look at it" — so a
four-column table is not a slightly worse listening experience, it is a hole.
The pin exists precisely so that hole cannot be opened without someone deciding
to open it, and its own comment records track after track where a wide table was
reshaped rather than accepted.

The wrong fix is to bump the pin to 49; the diff would look tiny and five tables
would silently leave the audiobook. The right fix took ten minutes: every one of
the five carried a column that was derivable from the others. A gender table
listing word / kind / article / what-decides-it folds `kind` into the last
column. An agreement table listing adjective / ending / masculine / feminine
drops `adjective`, because the masculine column already spells it. A character
list with word / meaning / feminine / changes? drops `changes?` and asks the
reader to compare the feminine against the word, which is the same observation
done by looking instead of by being told. The count went back to 44 and the pin
was never touched.

Two things to do differently. Check table width **while authoring**, not after
the suite runs: `awk '/^\|/{n=gsub(/\|/,"|"); if(n-1>=4) print FILENAME": "$0}'`
over the new lessons takes a second. And treat an unlabelled leading column as
four columns even when it looks like three — a row that starts `| | German |
English | build |` is the exact shape `chapter-policy` calls unspeakable, and
the pin comment names several tracks that shipped it by accident.
