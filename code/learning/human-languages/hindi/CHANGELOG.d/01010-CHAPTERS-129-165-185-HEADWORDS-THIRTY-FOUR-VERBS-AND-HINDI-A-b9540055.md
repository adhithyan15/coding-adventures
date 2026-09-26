## Chapters 129-165: 185 headwords, thirty-four verbs, and Hindi attains A1

Hindi had five A1 gaps:

- It was 180 headwords short.
- It was 24 verbs short.
- NAME-EVERYDAY-THINGS had no segment.
- One lesson went over the new-atom budget.
- Sixty atoms at A1 were revisited fewer than twice.

**The words.** Thirty-seven chapters of five:

- **things** (NAME-EVERYDAY-THINGS): the house and its tools, things you carry and
  wear, food, nature, animals, the body and places in town
- **qualities:** twenty-six describing words, from **गरम** to **भरा**
- **verbs:** thirty-four, from **सोना** and **जागना** to **सिखाना**, **सीखना** and
  **समझाना**

Every candidate was checked against every earlier headword, romanization and
lesson text. The check normalizes nukta letters (**ड़**, **ज़**, **ख़**) the same
way the lessons store them. Every word uses only letters the book has already
taught the reader to write. Forward references stay at 21. Letter anchoring is
unchanged.

**The atom budget.** Chapter 22's **ग्यारह — बीस** introduced four atoms, one over
the budget. Its "honest irregularity" note is about the same eleven-to-eighteen
words as its first atom, so the two are now one. The lesson still teaches both.
The two later lessons that retrieved the old atom now retrieve the merged one.

**The revisits.** Sixty-nine thin atoms from about thirty early lessons each get
a warm-up retrieval in the new chapters. Most of them are the etymology-rich
"You'll want to know" sections of chapters 5-35. Each group is one line, placed
twice where the atom had no revisit at all:

- the week by its sky
- the colours and their stories
- the six seasons and **मौसम**
- the borrowed months
- one **दिन** from **दोपहर** to **आधी रात**
- the three **शुभ** greetings
- the four mind verbs and what each first meant
- **मुझे चाय पसंद है**
- words that never bend
- ordering three things two ways
- counting with its history
- **बोलता था** and **आओगे**

**The contents page numbers.** Hindi is the first book whose lessons run past
page 999, starting at chapter 151. `book.cls` gives contents page numbers a
three-digit box, so each four-digit entry ran 4.93pt too wide: 78 overfull lines
in the typesetting scan. The preamble now widens that box (`\@pnumwidth`
2.4em) and the right margin beside it (`\@tocrmarg` 3.4em).

      hindi headwords at or below A1   420  ->  600+ (A1 target met)
      hindi verbs at or below A1        16  ->  40+
      hindi lessons                    634  ->  827
      level gate                       pre-A1 -> A1
