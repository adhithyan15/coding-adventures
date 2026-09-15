# A repertoire check is only as good as the corpus you scope it to

The Spanish book compiles with **Latin Modern**; the Indic and CJK books load vendored
**Noto** faces. A missing glyph does not fail locally — nothing opens a font until
XeLaTeX runs in CI — so the only pre-push defence is to diff the characters your new
`.tex` uses against the characters a book has already rendered.

The first version of that check scanned **`code/learning/human-languages/**/*.tex`**, the
whole corpus. That is wrong in the direction that produces silence: the union of 1,311
`.tex` files includes every Devanagari, Arabic and CJK character in the repo, so it would
have certified as safe any glyph Latin Modern cannot draw. A corpus-wide repertoire is not
a conservative approximation of a per-track one — it is the opposite of one.

**Scope the repertoire to the track whose font will render it.** 368 Spanish `.tex` files,
152 distinct characters. That is the set that has actually been proved.

**And self-test the instrument in BOTH directions.** The bug above was not caught by
reasoning; it was caught by an assertion:

    if "ñ" not in repertoire:  fail   # loaded empty -> everything reads novel
    if "好" in repertoire:      fail   # loaded the wrong thing -> nothing reads novel

The second assertion is the one that fired. A one-directional self-test ("did I load
anything?") passes cheerfully on a repertoire that loaded 1,311 files when it should have
loaded 368. **Every check that compares against a baseline needs a negative control**, or
it cannot tell "nothing is wrong" from "I am not looking at the right thing."
