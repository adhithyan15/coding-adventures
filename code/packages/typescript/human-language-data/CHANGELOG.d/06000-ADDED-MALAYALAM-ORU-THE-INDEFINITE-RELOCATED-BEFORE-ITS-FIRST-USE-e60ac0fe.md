### Added — Malayalam `ഒരു`, relocated in front of its first use

- `ML-A1-ART-02` closes. Malayalam A1 coverage 215/243 -> **216/243**, and the
  percentage **ticks 88 -> 89** (216/243 is 88.9). 27 points unmapped.

#### A relocation, not an addition

**ഒരു** is never a headword and has no atom, yet it stands as a **standalone
token** in six lessons already:

| | |
|---|---|
| `ML-C18-mani` | **ഒരു മണി** — *one o'clock* |
| `ML-C23-naal` | **ഒരു നാൾ** — *one day* |
| `ML-C39-chaaya`, `ML-C39-kaapi`, `ML-C39-paal` | **ഒരു ചായ** — *a tea* |
| `ML-C49-perhaps` | glossed in prose while taking **ഒരുപക്ഷേ** apart |

So a lesson appended at the end of the track would have left all six as forward
references. `ML-C18-oru` lands at sequence **465**, immediately before
`ML-C18-mani` (470), the earliest of them.

#### The census was re-derived rather than trusted, and it matches

**Six** files by a word-bounded count against the Malayalam block; **ten** by a
bare substring grep, which also catches the **ഒരുപാട്** family in chapter 79 and
a compound in `ML-C51-blessing`. Count the token, not the substring.

#### "Never taught" needed one qualification — which is what a relocation is for

`ML-C49-perhaps:43` already glossed **ഒരു** in prose, and glossed it **loosely**:

> ഒരു (*oru*) is 'one', the same numeral this book counted with.

The numeral this book counted with is **ഒന്ന്** (`ML-W07-number-words-1-5`).
**ഒരു** is the shape it takes before a noun — a distinction the new lesson is
built on, so leaving the old line would have had the corpus contradicting itself
one chapter apart. Corrected here.

#### Placement was decided by what is teachable, not by what is earliest

The first draft put the lesson in **chapter 7**, beside the numerals, which looks
right and is not. Two things killed it:

- At sequence 356 almost **no countable noun** has been taught.
- Its examples — **ഒരു മണി**, **ഒരു ചായ** — are drawn from lessons that come
  *later*, which would have **reintroduced the very forward references being
  removed**.

Chapter 18 is the right home: it is the first user, and everything through
chapter 17 is available. Even there the choice is narrow, because the colour,
family, body-part and food words all sit inside **multi-word headwords** and so
own no single token. **പേര്** is the worked example, as one of only **five**
single-token nouns available before 465.

#### What the lesson teaches

**ഒന്ന്** and **ഒരു** as one number in two shapes, split by whether a noun
follows; then that the same word does the work of *a*, since Malayalam has no
article — **ഒരു പേര്** is both *one name* and *a name*, with nothing in the word
to say which.

#### Verification

`npm run validate` 21/21, all twelve gates, the full suite (145 files, **2089
passed**, 1 skipped), `check-book-compile.sh --strict malayalam`, and all six
LaTeX warning counters at zero. The format assertion caught my own comment
claiming the percentage was "still 88" when it rounds to 89.
