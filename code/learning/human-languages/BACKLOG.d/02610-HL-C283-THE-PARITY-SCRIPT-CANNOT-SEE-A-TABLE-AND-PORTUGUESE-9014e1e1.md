## HL-C283 — the parity script cannot see a table, and Portuguese is the case where that did not matter

`handwritten_parity.py` classifies `tabular`, `tabularx` and `center` as LAYOUT
rather than prose, on the reasonable ground that generated chapters contain them
too, so their presence says nothing about lost writing. The consequence is
easier to miss: a table in a hand-written chapter is invisible to the gap count.
A chapter can report a gap of **zero** and still lose a table on flip.

Italian and Portuguese both had a Latin-to-Romance sound-correspondence table in
their `notte`/`noite` sections, and both reported zero. They resolved opposite
ways, which is what makes the pair worth writing down:

  * **Portuguese** kept it. `PT-C01-noite` already carried the same table as
    markdown, so it renders straight through into the generated chapter.
  * **Italian** did not. `IT-C01-notte` had already, deliberately, replaced the
    table with *"Watch just one change today ... there is no list to memorize
    now"*. The table stayed dropped, because carrying it back would have put
    three extra correspondences and two extra languages into a pre-A1 lesson.

Same measurement, same number, two correct-but-different answers. The number
could not have told anyone which. **Read the chapters.**

**What the check should probably become.** Not "count tables too" — that would
have flagged Italian, where the drop was the right call. The honest shape is a
report that NAMES the layout blocks a hand-written chapter contains and whether
the owning lesson has one, so a reviewer is handed the question rather than a
zero. That is a change to `handwritten_parity.py` and wants its own PR.

**Also owed, and shared with the Italian entry.** Portuguese Chapter 1 now
measures 19 atoms against the 12-atom chapter budget. It was over budget the
whole time; a schema-v1 lesson declares no atoms and so contributed zero, not
"a little", to every atom budget in the corpus. Splitting the chapter is the
ramp policy's own answer, and it renumbers everything downstream, so it wants a
PR of its own.
