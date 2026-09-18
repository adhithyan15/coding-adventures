### Added — Malayalam chapter 104, one present tense answering two English ones

- `ML-A1-V-20` closes. Malayalam A1 coverage 210/243 -> **211/243 (87%)**, 32
  points unmapped. **The percentage moves, 86 → 87** (211/243 is 86.8, which
  rounds up) — recomputed, not carried.

#### The note was stale, and half of it was simply false

> Not taught. The **-kkunnu** form is used inside `ML-C05`'s sentence but
> **never named as a form**, and there is no gerund lesson.

**It is named three times over, from sequence 660.** `ML-C32-pokuka` states
*"Malayalam's present tense is one word long"*; `ML-C33-cintikkuka` repeats it;
`ML-C50-parayuka` (1405) says outright *"**പറയുന്നു** is the whole present
tense"*. The note was wrong by 2,390 sequence points.

**What is genuinely missing is the mapping.** No lesson tells the learner that
this **one** form answers **both** English presents — *I go* and *I am going*.
And `ML-C74-purpose` (2560) already translates **ഞാൻ പോകുന്നു** as *"I am
going in order to read"*, using the English progressive without ever making the
point. Parts present, never assembled — exactly the shape `ML-A1-TIME-10` had.

#### No new word, and one that is built rather than given

| | | |
|---|---|---|
| **ഞാൻ പോകുന്നു** | *ñān pōkunnu* | I go **and** I am going |
| **ഇപ്പോൾ ഞാൻ പോകുന്നു** | *ippōḷ ñān pōkunnu* | right now I am going |

**ഇപ്പോൾ** is owned since 1390, **എപ്പോൾ** since 2590, **നീ** since 60,
**ആണ്** since 90. **വരുന്നു** appears **nowhere** in the corpus — and it is
*buildable*, from **വരുക** plus a rule the book states, so it is used in prose
and inside a multi-word headword and **never headworded alone**. That is the
`lessons.d` rule that a word the corpus can already build must not be introduced
later as a word.

#### The warning is the part a learner most needs

English says *I **am** going*. **ആണ്** has been taught since sequence **90** as
*is/am*, so a learner **will** reach for it:

| | |
|---|---|
| **ഇപ്പോൾ ഞാൻ വരുന്നു** | right — one verb, nothing added |
| *ഇപ്പോൾ ഞാൻ ആണ് വരുന്നു* | **not a Malayalam sentence** |

**ആണ്** joins a thing to what it *is*; it is not a helper propping up another
verb, because Malayalam has no helpers. The ungrammatical form is shown and
marked as such, because the mistake is predictable and naming it is cheaper than
leaving the learner to make it.

#### Two of my own claims were false and caught before commit

- **"you have seen the same shape before in *this* and *which*"** — **ഏത്**
  (*which*) is taught **nowhere**. The taught pair is **ഇത്**/**അത്**, *this*
  against *that*, which is a different alternation entirely: proximal against
  distal, not proximal against interrogative. The claim is **dropped**, not
  repaired, because there is no taught precedent to point at. The lesson now says
  only what is true of these two words: **ഇ-** points, **എ-** asks.
- **The exchange asked a question nobody asks.** It used one subject in both
  lines to support *"only the first word moves"*, which made the question *"when
  am I coming?"*. It now runs between two speakers — **നീ** asking, **ഞാൻ**
  answering — and claims only what survives that: **the verb is identical in both
  lines**.

#### A third gate the full suite caught, and the fix was to stop asserting

`info-dump.test.ts` holds `ruleStatements` at **32** and says so in its own
comment: *"CEILING — this is debt; it may fall, never grow"*. The draft took it
to **33**, on one sentence in `ML-C104-no-am`: *"**The rule is general**, so take
a second verb through it."*

The precedent in that test's own history is to **rewrite the incidental ones
rather than absorb them** — Persian, Russian, Sanskrit and Spanish tranches each
pushed this ceiling and each kept only the statement whose *entire lesson is a
rule*. This one is not that: the lesson's content is a **demonstration**, a second
verb going through unchanged. The sentence now reads *"Take a second verb through
it and watch nothing bend"*, which shows the same thing without asserting it, and
the count is back to 32.

That is three separate gates in two chapters — `chapter-references`,
`script-closure`, `info-dump` — that `npm run validate` and all twelve
`check:` scripts pass and only the full suite catches.

#### What the chapter refuses to claim

Malayalam's dedicated progressive in **-കൊണ്ടിരിക്കുന്നു** is **not** taught and
is **not** claimed. At A1 the simple present *is* how ongoing action is said, and
that is what this point asks for. Saying otherwise would claim a range the
corpus does not teach.
