### Added — HL12: the four silent tracks start teaching their script
- **30 recognition segments** — Telugu, Kannada and Malayalam 8 each, Sanskrit 6 —
  one character per lesson. Before this, four of the six Indic tracks taught **no
  letter at all** while printing every word in a script the reader had no way in
  to. That is the gap HL12 §1 measured and this closes the first part of it.
- Each segment names one character, says what it carries, and shows it inside
  four words the reader **already says**. So it sits at the frontier of decoding
  and never at the frontier of meaning — HL12 §2.1's rule, satisfied by
  construction rather than by inspection.
- **Recognition, not writing, and that is a sourcing fact.** These three scripts
  have zero cited stroke orders (their own script files say *"Recognition
  only"*), and HL11 §5 forbids a pen path without a citation: a learner cannot
  tell an invented stroke order from an attested one and will drill it for years.
  The reader traces the printed shape — which needs no source — and the book says
  plainly that where to start and which way to travel are not written down yet.
- **Placement was measured, not chosen by taste.** Second-in-chapter cost 11
  lessons from the drivable prefixes; last-in-chapter cost **none** —
  `drivablePrefixTotal` is unchanged at 1136 and `unstartableChapters` holds at
  173. Last is also better teaching: the character arrives after every word in
  the chapter that contains it.
- The generator refuses a chapter that cannot afford one more atom. Sanskrit's
  chapters 6 and 7 sit at 15 and 12 against HL08's budget of 12, so they get no
  segment and Sanskrit takes six rather than eight. A chapter that cannot afford
  a letter does not get one.

