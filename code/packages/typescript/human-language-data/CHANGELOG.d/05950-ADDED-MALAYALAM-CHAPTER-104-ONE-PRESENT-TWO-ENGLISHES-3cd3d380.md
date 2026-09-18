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
tense"*.

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

**ഇപ്പോൾ** is owned since 1390, **എപ്പോൾ** since 2590, **നീ** since 110,
**ആണ്** since 90. **വരുന്നു** is never headworded alone, which is the `lessons.d`
rule that a word the corpus can already build must not be introduced later as a
word.

**The draft justified that with a false claim, and it is the script-keyed census
failing a third time.** It said *varunnu* "appears nowhere in the corpus" — true
only of the **Malayalam-script** spelling. `ML-C32-varuka:78` gives it in the
tense table as *varunnu*, `:97` drills it (*"the three times: varunnu … vannu …
varuṁ"*), and `ML-C32-ariyuka:89` and `ML-C33-ezhutuka:86` recite it in a
six-verb set. It was **given and drilled**, in romanization, at sequence 670.
`ML-C104-no-am` no longer says *"Nobody had to give you varunnu as a word"*.
`HL-C402`, `ML-C34`'s *ethra maṇi*, and now this: **three times in one run**, a
census keyed on script has missed what the corpus teaches in sound.

#### The warning is the part a learner most needs

English says *I **am** going*. **ആണ്** has been taught since sequence **90** as
*is/am*, so a learner **will** reach for it:

| | |
|---|---|
| **ഇപ്പോൾ ഞാൻ വരുന്നു** | right — one verb, nothing added |
| *ഇപ്പോൾ ഞാൻ ആണ് വരുന്നു* | **not a Malayalam sentence** |

**ആണ്** joins a thing to what it *is*; it is not a helper propping up another
verb; propping one up is not its job. The ungrammatical form is shown and
marked as such, because the mistake is predictable and naming it is cheaper than
leaving the learner to make it.

#### Two of my own claims were false and caught before commit

- **"you have seen the same shape before in *this* and *which*"** — **ഏത്**
  (*which*) is taught **nowhere**, so that pair was wrong. **The conclusion I drew
  from it was worse than the error.** I wrote that the claim was *"dropped, not
  repaired, because there is no taught precedent to point at"* — and
  `ML-C41-deixis-system` (sequence 1000) teaches exactly this alternation, under
  the headword **"i- / a- / e-"**. `ML-C50-now` names it when introducing
  **ഇപ്പോൾ**, and `ML-C75-eppol` lays out **ഇപ്പോൾ**/**അപ്പോൾ**/**എപ്പോൾ** in a
  three-row table cross-referenced to **ഇവിടെ**/**അവിടെ**/**എവിടെ**.
  **I checked one instance of a pattern, found it absent, and concluded the
  pattern was untaught.** The repair is the opposite of what I did: the lesson now
  **names** the system, keeps the three-way row rather than reducing it to two,
  and requires and reviews `ML-GRAMMAR-C41-DEIXIS-SYSTEM`.
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

#### A seventh false universal, caught before the reviewer reported

`ML-C104-no-am` asserted *"Malayalam has no such helpers"* — that the language
has no auxiliary verbs at all. **Three taught lessons contradict it**, and the
sharpest is `ML-C32-undu`'s **own gloss** at sequence 650, which calls **ഉണ്ട്**
and **ഇരിക്കുക** *"the two-piece machine **every** Malayalam verb is built on"*.
**കഴിയും** (2570) and **വേണം** (1815) are taught too.

The narrower claim is true and does all the work the lesson needs: **ആണ്** joins
a thing to what it *is*, propping up another verb is not its job, and the *am
going* already lives inside **വരുന്നു** so there is nothing for a second word to
carry. The lesson now says outright that Malayalam **does** put verbs together
elsewhere — naming **ഉണ്ട്** and **കഴിയും**, both of which the learner owns — and
that this is not one of those places. Both atoms are declared, because citing
another lesson's content without declaring it is the defect `ML-C92` produced two
tranches ago.

**Seven of this run's defects have now been the same sentence shape**: a claim
that something is unique, universal, or already-seen. None was caught by a gate.

#### What the chapter refuses to claim

Malayalam's dedicated progressive in **-കൊണ്ടിരിക്കുന്നു** is **not** taught and
is **not** claimed. At A1 the simple present *is* how ongoing action is said, and
that is what this point asks for. Saying otherwise would claim a range the
corpus does not teach.
