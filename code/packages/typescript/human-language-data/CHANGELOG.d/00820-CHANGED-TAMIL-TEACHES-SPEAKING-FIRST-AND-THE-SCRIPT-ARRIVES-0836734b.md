### Changed — Tamil teaches speaking first, and the script arrives gently

Tamil chapter 1 held **eleven writing lessons against nine speaking lessons**, and the
curriculum path put all eleven at positions 3-13: a learner met வணக்கம், then did eleven
consecutive lessons on letter shapes before reaching the word for *yes*. The chapter's
own declared capability said so out loud — *"I can write வணக்கம் and நன்றி by hand, put
the puḷḷi and the ி sign in the right places"* — a chapter about greeting people whose
stated payoff was handwriting.

You can speak a language without reading a letter of it, so the course now does that:

- **Chapters 1-3 carry no writing lesson.** 23 lessons — greetings, yes/no, thanks,
  names, how-are-you. Chapter 1's capability is now *greet someone, say yes, no and
  thank you, agree with சரி, and hold a short exchange out loud — entirely by ear*,
  and chapter 1 becomes the first Tamil chapter that is fully drivable by ear.
    `TA-C01-practice` had a section headed *"Read them back"* that taught the puḷḷi,
  vowel signs and word-initial vowels inside the chapter-1 recap; it now says the five
  words aloud and states plainly that reading is not expected yet. The **book** said the
  same thing more loudly and had to be fixed too: all five `sounds` boxes in
  `ch01-greetings.tex` taught left-to-right reading, the inherent vowel, the puḷḷi and
  the ி and ை signs, and its recap table was headed *"Read"*. Chapters 1-5 are
  hand-authored, so `book-cli --check` never compares them against the lessons and
  nothing flagged the divergence. Those boxes now teach **pronunciation** — retroflex ṇ,
  the held *kk*, Tamil's three *n* sounds, vowel length, the *ai* diphthong — which is
  what a box called "sounds" should have held all along.
- **The script starts in chapter 4, one lesson at a time.** The eleven script lessons  are spread across chapters 4-19, admitted after every third speaking lesson.
  Measured, they sit at 0-indexed reading positions 27, 31, 36, 40, 44, 48, 52, 56, 60,
  64, 68. Chapter 4's lesson teaches no letter at all — it is the palm-leaf lesson on
  why the script is round; the first actual letters, வ and க, arrive in chapter 5.
- **Every script lesson spells a word already known by ear.** வணக்கம் is learned in
  chapter 1 and written in chapter 8, once all its letters exist; நன்றி, ஆம், இல்லை and
  சரி likewise. The writing strand carries no new vocabulary at all.
- **The script is shown from page one but never taught early.** Tamil words still appear
  in the speaking lessons so the shapes grow familiar; nothing asks the learner to read
  or write them until chapter 4. `tamil/book/chapters/ch01-greetings.tex` said the
  opposite — *"Each lesson introduces the letters its word needs"* — and now says what
  the chapter actually does.

