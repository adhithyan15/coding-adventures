### Added — Malayalam chapter 108, the marks on the page

- `ML-A1-SCR-15` closes. Malayalam A1 coverage 214/243 -> **215/243 (88%)**, 28
  points unmapped. *Lipi (script and orthography)* goes 11/16 -> **12/16**.

#### Nine Spanish points behind one Malayalam point, at zero new headwords

`ML-A1-SCR-15` derives from `A1-O3-01` through `A1-O3-09`: the full stop, the
comma, the colon, the question and exclamation marks, parentheses, quotation
marks, the hyphen, the dialogue dash and the slash. The Spanish track closed its
own *Puntuación* category the same way at chapter 424 — *"eight marks the corpus
printed constantly and never named."*

This chapter spends **no new vocabulary** and opens **no script debt**.

#### The note's claim was opened, not trusted

> Modern Malayalam uses the European marks, and not one lesson teaches any of
> them.

That second clause is a claim about **existing material**. Six Malayalam lessons
matched a punctuation grep and **five of the six** were the word *command*
matching *comma*. The sixth, `ML-C70-um-more`, mentions **English's** commas
contrastively and teaches no Malayalam mark. The claim is true.

#### The marks were already on the page, and that is the chapter

`ML-C02-ninre-peru-entaanu` prints a question mark **in its headword** at
sequence 130. `ML-C72-ennu` prints quotation marks around its quoted sentence.
The book has shown these marks since the reader's second chapter and has never
named one, so each lesson opens at the place the reader has already seen its
mark.

#### The load-bearing fact is checkable in Unicode, not asserted

The Malayalam block **U+0D00–U+0D7F** contains **zero** characters of general
category `P`. The script supplied no punctuation of its own, so every mark taught
here is borrowed — and the older answer is the **daṇḍa**, which `SA-C60-danda`
already teaches in the Sanskrit track.

The generator routes `।` through `\ml{}` rather than `\dv{}`, so Noto Sans
Malayalam was checked for **U+0964** and **U+0965** *before* the mark was
printed. Both are in the cmap; `missing_character` is 0.

#### Three marks landed on jobs Malayalam had already filled

| | |
|---|---|
| the comma in a list | **-ഉം**, on every item |
| the question mark | the question words, and the **-ō** ending |
| the quotation marks | **എന്ന്**, after the thing said |

So a Malayalam list of that kind carries **no commas at all** — not because a
writer avoids them, but because nothing is left for them to do. The marks are for
the eye; the Malayalam pieces are the grammar, and they were there first.

#### Two claims were corrected in draft, against the corpus

**ശുഭ രാത്രി** was about to be cited as a solid compound and is written **with a
space**. The hyphen lesson now says the taught compounds run their pieces
together (**പള്ളിക്കൂടം**, **തലസ്ഥാനം**), that a plain space separates them where
they stay apart, and that neither way uses a hyphen.

The **-ō** question ending was nearly asserted unattributed. `ML-C03-sukhamaano`
teaches it outright, so it is cited.

#### The Malayalam names of the marks are deliberately not given

They exist, and this book cannot source them yet. That is the same call
`ML-S147-letter-ttha` makes about stroke order, and the lesson says so in the
same words rather than leaving a silent gap.

#### Verification caught three things before CI did

- **`chapter-references`** rejected the first draft at **59 against a ceiling of
  46**: every lesson opened by naming an earlier chapter *number*. All thirteen
  were rewritten to name the thing — *"since you first asked anybody their
  name"* — which is what that test's own documentation prescribes, and is better
  teaching than a number that rots on the next renumber.
- **The duration ceiling** caught `ML-W108-question-mark` at 359s against 300s.
- **`scan_latex_log_warnings.py`**, new to the routine after it failed CI on
  chapter 107, caught an underfull hbox at badness 2644 that
  `check-book-compile.sh` reported as `ok`.

The `info-dump` rule-statement ceiling held at 32 — the four `RULE_PATTERNS`
were grepped in draft, because `RU-W10-zapyataya`'s own ceiling comment records
that a punctuation lesson is exactly the shape that spends one.
