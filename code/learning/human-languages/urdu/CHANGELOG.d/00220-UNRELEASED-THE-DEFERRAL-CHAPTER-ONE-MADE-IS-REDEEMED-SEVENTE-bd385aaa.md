## Unreleased — the deferral chapter one made is redeemed, seventeen chapters late

Nineteen lessons in four chapters (19-22), fifteen of them items, close
**fifteen A1 exam points**. The joining column goes from 2/11 to 8/11, negation
from 1/4 to 3/4, and the track acquires a repair kit it did not have at all.

The functional hole was recorded verbatim by the inventory. Chapter one's نہیں
lesson says, in as many words: *"Inside a sentence it commonly means not. That
second job will matter when the track introduces verbs."* Fourteen verbs and
seventeen chapters later, no lesson came back to it — and every piece of
میں نہیں سمجھتا was already taught: میں in chapter 4, نہیں in chapter 1, سمجھنا
in chapter 7, the present-tense agreement in chapter 6.

### Every number re-measured against the merged tree, not derived from a delta

    urdu A1 exam coverage                    101/234 (43%) -> 116/234 (50%)
    urdu lessons                                       89 -> 108
    urdu atoms taught                                 186 -> 201
    urdu atoms never revisited                          4 ->   3
    urdu forward references                             2 ->   2   (held)
    urdu forward prerequisites / reviews              0/0 -> 0/0   (held)
    urdu script closure violations                      4 ->   4   (held)
    urdu glyphs taught / shown / never taught     15/35/20 -> 15/35/20 (held)
    urdu exposure-only lessons                         49 ->  52
    urdu headwords without romanization                 0 ->   0   (held)
    urdu reinforcement window misses                  183 -> 210
      R1 (1-3)                                          38 ->  39
      R2 (5-15)                                         48 ->  51
      R3 (20-60)                                        75 ->  80
      R4 (80-250)                                       22 ->  40
    lessons at or over the computed 300s ceiling         0 ->   0
    corpus banned-word total                         1077 -> 1059

### The design constraint was the SCRIPT, and it was measured first

The two words this track's inventory names first — معاف for *sorry* and
مہربانی for *please* — are **not** in this tranche, and the reason is not
scheduling. Urdu's script ladder has taught **fifteen letters in eighty-nine
lessons**, and معاف کیجیے needs عین, فے and جیم while مہربانی needs بے. None of
the four has a lesson.

So every candidate headword was tested against the taught letter set before it
was designed, and the ones that passed turned out to be the ones that closed the
most points anyway:

    inside the taught set   نہیں  اور  یا  لیکن  کیوں  کون  کیونکہ  کہ
                            کے لیے  کہنا  کیا کہا  آرام سے
    outside it              معاف  مہربانی  مگر  جب  جو  بھی  مت

Each word in the second row is now named in the inventory beside the letter that
blocks it. A vocabulary gap that names its own script lesson is worth more than
a covered point built on a letter the reader cannot read.

### What is taught, one item per lesson

    ch19  نہیں before the verb, کیا at    the deferral redeemed, and the
          the front, نہ … نہ               question جی ہاں answered for
                                           eighteen chapters
    ch20  اور, یا, لیکن, ٹھیک ہے           adding, choosing, turning, agreeing
    ch21  کیوں, کون, کیونکہ, کہ, کے لیے    the k- family completed, and two
                                           different answers to "why"
    ch22  کہنا, کیا کہا؟, آرام سے          the repair kit

### Three spine ledgers move

UR-C19-nahin-verb realizes VERB-NEGATE and UR-C19-kya-polar realizes
QUESTION-POLAR, so SPINE-NEGATE-AND-ASK omits nothing. UR-C21-kyunki realizes
CONNECTIVE-BECAUSE, so SPINE-SAY-WHY omits nothing. UR-C20-thik-hai realizes
RESPONSE-OKAY, and UR-C22-kahna realizes VERB-SAY on a relocated node.

### The exposure exemption, used three times and counted

`exposureOnly` rose 49 -> 52. Three lessons own a headword carrying a letter the
ladder has not reached — میں نہیں سمجھتا, کیا آپ ٹھیک ہیں؟ and ٹھیک ہے — and
their bodies then use those letters. Every other new lesson body was written
inside the taught set, which is why violations held at 4 and never-taught glyphs
at 20.

### The reinforcement number, decomposed

    created by this tranche      0    no new atom misses R1, R2, R3 or R4
    exposed by its added length 58    all pre-existing atoms
    cleared by two distant bands 31   R2 -1, R3 -13, R4 -17

The two review lessons were rewritten to reach back on purpose rather than to
report a number. UR-R20 runs اور, یا and نہ … نہ over the thirty-odd nouns of
chapters 8-18 — which is exactly what teaching a coordinator buys — and clears
thirteen R3 misses. UR-R22 replays the opening conversation of chapters 3-5 and
drops the repair kit into it, which is where a beginner actually loses the
thread, and clears seventeen R4 misses.

### The banned-word ceiling was NOT edited, and the reason is a conflict

Urdu's eighteen `just` / `simply` occurrences are paid down to zero here, and
the nineteen new lessons carry none. The corpus total falls 1077 -> 1059 and
the ceiling assertion in `banned-words.test.ts` is deliberately left at 1076:
the sibling Bengali tranche lowers that same literal, and two branches editing
one line is a merge conflict for no gain. Whichever lands second still passes.

### Left uncovered, with the reason written into the inventory

    UR-A1-NEG-03    مت needs an imperative the track lacks AND a letter
    UR-A1-JOIN-04   ایک … دوسرا needs the numerals, still 0/5
    UR-A1-JOIN-06   جو needs جیم
    UR-A1-JOIN-08   جب needs جیم and بے
    UR-A1-FUN-11    معاف کیجیے needs عین, فے, جیم
    UR-A1-FUN-13    مہربانی needs بے — and see the note: UR-C06-bolna introduces
                    an atom called UR-SCRIPT-BE-LETTER, so the TRACK teaches the
                    letter, but that lesson is `type: word` with no
                    `delivery: script`, so measureScriptClosure cannot see it
    UR-A1-ADV-04    بھی needs بے and دو چشمی ہے

### Verified

`npx vitest run` (134 files, 1921 passed), every `check:*` gate, and the Urdu
book compiled with XeLaTeX: exit 0, zero missing characters, four new chapters
read on the page as images rather than through `pdftotext`, which cannot be
trusted for Nastaliq.

