---
category: Testing & coverage
---

# An instrument that enforces a convention nobody wrote down manufactures defects in correct prose

A checker I wrote to keep `.adj` header prose tidy has **twice** reported a
defect in prose that was correct. Both times the "defect" was a convention I had
invented and then enforced. Both times the corpus settled it in minutes, while
the "fix" would have rewritten good text.

Throughout, **slack** means `80 - (len(line) + 1 + len(next word))`: the columns
that would still be free if the next line's first word were pulled up. The
definition is stated because a neighbouring instrument defines it as
`80 - len(line)`, and at the same threshold over the same corpus the two differ
by nearly an order of magnitude. A number does not travel without its formula.

### First instance: exact greedy wrapping

The check flagged any line where slack was non-negative — i.e. any line the next
word would have fitted on. Before acting on it I counted, over files I did not
write (`survey_wrap.py`, re-run 2026-09-16 against `origin/main` `48b54219e1`):

```
OTHERS  702 files, 22558 prose lines, 6800 under-filled = 30.1%
        median slack 2; 88% of them within 4 columns.
MINE      6 files,   391 prose lines,  166 under-filled = 42.5%
        median slack 2; 85% within 4 columns.
```

A third of the corpus fails the rule, so greedy wrap is not this repo's
convention — I invented it. **What matches is the shape, not the rate:** median
slack 2 in both, 88% against 85% within four columns. The rates differ (30.1% vs
42.5%), and an earlier draft of this shard called my prose "indistinguishable"
from the corpus, which is false of rates and only true of shape.

Those MINE figures are also a warning about pasting a census into prose: the
first version of this paragraph said 347 lines and 36.3%, true when measured and
already stale when written down, because six merged PRs had edited those files
in between. **Date a count and name the commit it ran against.**

I relaxed the rule to flag only lines with twenty or more columns of unused
room — the far tail of the census's own slack distribution, rather than a number
I preferred.

### Second instance: the `% (See ../README.md; ...)` block

The same check flagged `blood-cell-types.adj:80` — 32 columns, next word `(See`
— and **I made the edit before checking.** Then I counted:

```
predicate: previous line is a comment, not a bare "%" and not a "====" ruler.
scope:     the 723 .adj files under code/specs/data/adj-facts-stdlib.

245 files carry a "% (See " line (one each -- no file carries two).
 47 follow a separator line; 198 follow prose by that predicate.
 of the 198, "(See" WOULD have fitted on the previous line in 179 = 90.4%.
```

The predicate is stated because it changes the answer. Under the stricter one
the width checker itself uses — which also excludes indented display
continuations — the same census reads **50 / 195 / 176 = 90.3%**, differing in
exactly three cases. The convention holds either way; the point is that **198 is
a fact about a predicate, not about the corpus.**

90.4% is not raggedness, it is a convention: that parenthetical starts a fresh
line. A second, independent signal agreed — `blood-cell-types.adj:82` is **85
columns** of unbreakable identifiers and has always shipped that way, so the
block is not held to the width band either. I reverted my edit to the file and
exempted the construct in the instrument instead.

### What the two have in common

Both times the instrument measured something real (line width) and inferred
something it had no evidence for: that the corpus wraps greedily; that every
following line is prose to be wrapped into.

The failure is not the threshold. It is that **a checker encodes a convention
whether or not you meant it to**, and an unexamined one arrives as a confident
FAIL on correct work.

**The rule:** before an instrument's complaint about house style changes a file,
count the convention it asserts across the corpus, and write the count into the
instrument beside the rule it justifies. A threshold with a census beside it can
be argued with. A threshold without one is preference wearing a pass/fail mask.

**The corollary that cost me the edit:** the first time the instrument fires on
someone else's file is the moment to run the census — not the moment to fix the
file.

### The fix did not reach the copies

[[a-census-keyed-on-one-of-a-construct-s-two-spellings-undercounts]] already
says: when one instrument is corrected, grep for every copy of its test. I
corrected the checker and then, only because that shard says so, went looking.

**My first route to the count was wrong in a way that happened not to change the
answer.** I ran a needle over the working directory, took the files it matched,
and called that "8 converters" — a list of what one needle found, not an
enumeration of what exists. Enumerating properly: of the **66** `*.py` scripts
in that directory whose filename contains `convert`, **8** carry the rule.

Two predicates were doing silent work there, and both needed saying:

- **Which rule.** 19 of the 66 carry *some* `len(...) < 30` short-line check;
  only 8 carry the `para_final`-guarded form, which is the one the fix changed.
  A review asked what "carry the rule" meant and the answer was not in the text.
- **Which files.** A *filename* net cannot see a script that is not named for
  what it does. The survey script that established the first census sits outside
  it — and carries the rule. So the ninth copy below had to be found by hand,
  and "0 of 66" and "a ninth copy is live" are consistent only because the 66 is
  a filename net.

**A ninth copy is live, and it is the survey that settled the first census.** It
still ends its report with "MY FILES: prose lines with slack >= 20 (candidate
'genuinely ragged')", and that list contains exactly one entry:

```
biology/blood-cell-types.adj:80  slack 43  % state would have been dropped.
```

That is the phantom itself. After the `(See` exemption my six files contain
**zero** ragged lines, and the instrument I used to prove the corpus does not
wrap greedily still cannot see its own exception. **The tool that corrects you
is not thereby corrected.**

### And the copies do not carry the defect I assumed they did

I nearly wrote that the eight copies would reproduce the `(See` phantom. They
would not, and I only know that because I ran the old predicate against the
control file instead of reasoning about it. Measured on `blood-cell-types.adj`,
old predicate versus new, whole file:

```
OLD flags 2:  '%      organs."'  and  '%      system."'
NEW flags 0
the disputed line '% state would have been dropped.':
    in para_final under OLD? no.   under NEW? yes.
```

The disputed line appears in **neither** flag list, because at 32 characters it
never reaches the `< 30` threshold. The `(See` phantom came from the **ragged**
rule, which the copies do not carry at all. What they carry is the **short
mid-paragraph** rule, whose uncorrected form misfires on *indented display
continuations* instead.

Two different blind spots in one instrument, with two different fixes — and I
was one sentence from attributing the wrong one to the copies. Which is this
shard's own lesson arriving from the side: I had measured that the copies exist,
and was about to explain what they would *do*, an assertion the measurement did
not cover.

### A closing note on this shard's own numbers

Three review rounds found sixteen claim defects here, and every one was a
denominator, a predicate or a scope left unstated — in a shard arguing that a
census must state its denominator. Two were numbers that had gone stale in
prose; one was a figure computed with one instrument's arithmetic over another
instrument's population; one was "every" said of 15 members of a 361-member
class.

What finally worked was not repairing them. It was **deleting every census the
claim does not need** — including, in the last pass, the two that had produced
wrong numbers in consecutive reviews: a comparison of two slack formulas, and a
set-identity claim about which files a survey skipped.

A draft of this very paragraph put a tally here — "six censuses before, three
now" — which I had not counted. There are four: the wrapping survey, the `(See`
census, the copy census, and the old-versus-new predicate control below. Writing
an unmeasured count into the closing paragraph of a shard about unmeasured
counts is the failure arriving one last time, on the way out. Keeping a figure
because it was expensive to obtain is not a fix; neither is reaching for a tally
because the sentence has a slot for one.

The instruments named here — the width checker, the wrapping survey, the block
census — are scratch scripts from one session's working directory, not repo
tooling; they are cited for what they measured, not as something to run later.
That is why the slack formula is spelled out at the top instead of delegated to
them.

Related: [[a-zero-occurrence-count-is-a-question-with-several-answers-not-a]] (a
zero may be the instrument);
[[count-what-you-inspected-or-the-loop-that-inspects-nothing-reports-success]]
(an instrument that inspects nothing reports success); and
[[writing-a-lesson-down-does-not-stop-you-rebuilding-the-bug-assert]] — assert
the lesson *in* the instrument, which is why each census about the width rules
sits in the checker as a comment beside the rule it justifies.
