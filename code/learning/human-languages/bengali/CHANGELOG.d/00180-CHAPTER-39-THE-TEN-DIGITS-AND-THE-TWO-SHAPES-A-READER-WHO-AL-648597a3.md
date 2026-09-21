## Chapter 39 — the ten digits, and the two shapes a reader who already reads digits will get wrong

`BN-A1-Q-03` read "NOT ONE DIGIT IS TAUGHT." It is now covered, and so is
`BN-A1-LIP-11`, which had been filed in the script column for the same piece of
work and said in its own note that closing it in one place would close it in
both. Coverage 139/244 → **141/244**. Twelve lessons in one chapter, eleven
atoms, ten new glyphs.

**SIXTY SCRIPT LESSONS HAD PUT NOTHING BUT LETTERS ON THE PAGE.** That is the
finding underneath the point. The Bengali strand is the largest in the corpus
after Tamil's, and every one of its lessons taught a letter or a sign — so a
price, a date, a clock face and a page number were all unreadable, and chapter
24's `BN-W02-tin-read` had taught the reader to read **তিন**, the *word*, while
**৩** remained a shape they had never seen. The chapter's payoff is that
distinction.

**THE ORDER IS BY WHAT THE READER CAN BRING TO EACH SHAPE, AND THE FOUR BANDS
WERE READ OFF THE RENDERED GLYPHS RATHER THAN REMEMBERED.** Every claim below was
checked by rasterising the digits in the book's own vendored font at 80pt and
looking at them:

- **০ and ২ agree.** ০ is a plain ring, exactly like **0**, and means zero. ২ is
  close enough to **2** to be kept after one telling, and means two. Wikipedia
  calls the Bengali numerals *a variety of the Hindu–Arabic numeral system* —
  one system, not two — which is why the agreement is there at all.
- **৩ collides with a letter rather than with a digit.** Set **৩** beside **ও**,
  which this book taught long ago: same rounded body, and **ও** carries a stroke
  down its right-hand side that **৩** does not. One stroke is the whole
  difference between a number and a letter here, and it is the only such
  collision in the set.
- **১, ৫ and ৬ carry nothing** and are simply learned. The book says so rather
  than inventing a mnemonic.
- **৪ and ৭ LIE, and they lie in the same way.** **৪** is drawn as an **8** and
  means **four**. **৭** is drawn as a **9** and means **seven**. In both cases
  the Bengali digit that really means eight or nine — **৮**, an upright with a
  single bowl, and **৯**, a body with its tail sweeping *up* — refused the
  familiar shape, leaving it empty for a smaller number to move into. So each
  trap is taught **immediately after** the digit whose value its shape suggests:
  ৮ then ৪, ৯ then ৭. Two of ten, and the rule lesson says which eight may be
  trusted so that a reader slows down on two shapes rather than distrusting the
  page.

**NO PEN PATH IS CLAIMED FOR ANY OF THE TEN, AND THAT WAS SEARCHED FOR RATHER
THAN ASSUMED.** HL-C212 established that Commons holds stroke-order animations
for every Bengali independent vowel and for no consonant at all. The same search
run for the digits finds nothing: the Commons numeral categories hold
photographs, the letter-writing app the Gujarati track cites covers Bengali
*letters* only, and Wiktionary gives ৪ as "Bengali Digit Four, U+09EA" with no
writing guidance. So every lesson teaches the **shape** against something the
reader already holds and cites the Unicode Bengali chart — exactly what
`BN-W05-tha` and `BN-W06-nya` do, and for the same reason.

**THE RULE ATOM SHIPS WITH BOTH ITS EDGES.**
`BN-SCRIPT-DIGIT-FALSE-FRIENDS-01` names where the reader's existing reading
helps (০, ২), where it is simply absent (১ ৩ ৫ ৬ ৮ ৯), and where it actively
misleads (৪, ৭) — and the last band is closed at exactly two members.

**REINFORCEMENT, DECOMPOSED ATOM BY ATOM RATHER THAN BY TOTALS:**

    reinforcementWindowMisses        242 -> 242
    reinforcementMissesByWindow-R1    38 ->  38
    reinforcementMissesByWindow-R2    49 ->  49
    reinforcementMissesByWindow-R3    90 ->  90
    reinforcementMissesByWindow-R4    65 ->  65
    atomsTaught                      206 -> 217
    atomsNeverRevisited                3 ->   3
    scriptClosureViolations           21 ->  21
    neverTaughtGlyphs                  8 ->   8
    taughtGlyphs                      37 ->  47

Every window is flat. The tranche's own eleven atoms create **zero** debt: each
digit lesson recalls the previous digit by name (R1), and the rule lesson and the
payoff between them retrieve all ten (R2). Twelve more lessons make **25**
(atom, window) slots newly judgeable on OLDER atoms — five R2, ten R3, ten R4 —
debt the length **exposes** rather than creates, and **all twenty-five are
answered**. The ten R4 slots fall on a clean diagonal: lesson *n* services the
atom introduced at position *n*−80, so the four clothing words of chapter 15 land
on the four hardest digit lessons, where a price is exactly what a reader would
be looking at. Zero regressions; nothing closed outright, which is honest — this
chapter had no pre-existing miss inside its reach.

**THE SCRIPT COST WAS MEASURED PER LESSON, AND ONE LEAK WAS CAUGHT THAT WAY.**
Because a script lesson teaches every target-script glyph in its body, a lesson
that quotes an untaught letter silently marks it taught. A per-lesson walk over
the chapter — taught set derived from the corpus at point of use, not
transcribed — caught three: **১** appearing in the *zero* lesson before its own
lesson, and **শ** entering through শাড়ি and চশমা in two warm-up recalls. All
three were removed; the two clothing recalls now cue their words in romanization,
which retrieves the same lexical atoms without teaching a letter by accident.
Every lesson now teaches exactly its own digit and nothing else, and
`scriptClosureViolations` and `neverTaughtGlyphs` are both unchanged.

**WHAT IS NOT CLAIMED, and the pages say it rather than leaving it to be
inferred.** This chapter teaches no *word*. The reader can now READ every number
and SAY only the first five, because the count in this book still stops at পাঁচ.
The sixth lesson states that boundary in the middle of the chapter, the payoff
restates it, and the test asserts that no lexical atom exists for শূন্য, ছয়,
সাত, আট or নয়. Two of those words additionally need য়, a letter the track does
not teach — which is a second reason the spoken half is a separate piece of work
and is filed as `BN-A1-Q-01`.

