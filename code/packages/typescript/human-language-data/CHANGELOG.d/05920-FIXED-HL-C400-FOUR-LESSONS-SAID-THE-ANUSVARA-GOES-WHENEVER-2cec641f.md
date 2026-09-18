### Fixed — `HL-C400`: four lessons said the anusvara goes whenever anything attaches

- No coverage change. Malayalam A1 stays at 208/243 (86%).

`HL-C400` was filed while writing chapter 96 and has been open since. Four
lessons told the learner that a word-final **ം** is dropped *"the moment
anything is added"* — and the corpus breaks that four ways.

#### What each site now says

| lesson | now reads |
|---|---|
| `ML-C83-keralathil` | *"the moment **a case ending** is added"*, plus *"Other kinds of ending do other things to it, and you will meet them in their own chapters."* Its recall asks *"when a case ending is added"*. |
| `ML-R83-from-recall` | *"dropped the moment **a case ending** is added"* |
| `ML-C91-vila-panam` | body and recall both say *"when **a case ending** attaches"* |
| `ML-C92-bhakshanam` | see below |

**`ML-C92-bhakshanam` was the worst of the four and the easiest to miss.** It
demonstrated *nothing* — it only asserted *"you already know how it will behave
when anything attaches to it"*, pointing at knowledge the learner did not have.
It now names what it does know: *"you already know what a **case ending** will
do to it — the **ം** gives way to **-ത്ത-**, as it did on **കേരളം** and
**പുസ്തകം**."*

#### The narrowing introduced a false claim of its own

`ML-C91-vila-panam` said the **ം** gives way *"exactly as it did on the state
name **and on the tens**"*. That was loose but harmless while the sentence read
*"when something attaches"*. **Narrowing it to "a case ending" turned it into an
assertion that a case ending removed a ം from the tens** — which have **no ം**
at all (they end in **ത്**: മുപ്പത്, എഴുപത്) and take **no case ending** (their
trigger is a following digit). Now *"on the state name and on the book"*.

**Third time in this run that a correction pass has introduced the error beside
the one it was fixing.** The rule earned from chapters 97 and 101 — identify
which *half* of a claim is wrong before editing — applies to analogies too: the
half that was true was "the state name", and I narrowed the whole sentence.

#### Two method claims in the first draft of this entry were false

- ***"No fifth site"*, after a sweep *"script and romanization"*.** Review found
  the identical sentence still asserted **in romanization** in
  `core/exam-inventory-malayalam-a1.json` (twice), plus a now-stale *"still tell
  the learner"* record in `malayalam/chapters.d/0096.json`. **All three are
  corrected in this commit**, and the claim now says what it supports: four
  canonical lessons. That is the third PR running where the *method* claim, not
  the conclusion, is what failed.
- ***"The learner has met one of the four at that point."*** They have met
  **two**. `ML-C70-um` (2390) teaches the **-ം → -വും** rule outright, **510
  points before** `ML-C83`. So `ML-C83` now **names** it — *"You have met one
  already: the and ending puts **വും** there instead"* — instead of promising it
  as something still to come, which was a forward promise pointing backwards.

#### What did hold

- **The core narrowing is linguistically correct.** No case ending in the corpus
  or in Malayalam fails to trigger **-ത്ത-**, and nothing that is not a case
  ending triggers it.
- **The recall partners were checked**: `ML-R91-shopping-recall` and
  `ML-R92-eating-out-recall` are clean. **Null result stated**, because
  `HL-C402` was the PR where a recall partner's table survived its own drill's
  correction.
- **"Case ending" is not new metalanguage.** `ML-C06-dative-ikku` uses it at
  sequence **320**, and `ML-C96-eluppam` (3410) already models the exact
  phrasing.

#### What was deliberately not done

Three merged lessons (`ML-C88-compound-numbers`, `ML-R88-numbers-recall`,
`ML-C89-mosham`) generalise *stem alternation* more broadly — *"a word wears one
shape alone and another before something else"*. **They are filed as `HL-C404`
with a dissent, not patched**: that claim is about stem alternation, not the
anusvara, and it is **true** — the ten's alternation is real and is not
triggered by a case ending, so narrowing those lessons the same way would make
them false. Having just introduced one error by over-reaching in this very pass,
widening it into three lessons whose claim I had not enumerated would have been
the same mistake twice in one commit.
