### Added — Malayalam chapter 107, medicine, and the check I skipped last time

- `ML-A1-LEX-42` closes. Malayalam A1 coverage 213/243 -> **214/243 (88%)**, 29
  points unmapped. The percentage holds — 214/243 is 88.1.

#### The note held on all five checks, and this time I ran the fifth one

> The symptoms are taught (`ML-A1-LEX-27`) and nothing that treats them is.

That second clause is a claim about **existing material** — the exact shape that
caught me on `ML-A1-TIME-09` one tranche ago, where a note anticipated my domain
check and I read its anticipation as a result. So `LEX-27`'s five probe atoms were
**opened** rather than trusted. All five are named here, because naming four of
five is how a census starts drifting: **വിശപ്പ്** (`FEEL-01`), **ക്ഷീണം** (`-02`),
**വേദന** (`-03`), **ചുമ** (`-04`) and **പനി** (`-05`) are all taught headwords in
chapter 63. The claim is true.

#### The domain check paid off again, and it shapes the chapter

**വൈദ്യൻ**, the doctor, is taught at `ML-C48-doctor` (1310). So the learner has
had the **symptoms** and the **person** and nothing in between — no substance, no
practice. That gap *is* the chapter, and it is built as a line:

| | | |
|---|---|---|
| **വൃത്തി** | *vṛtti* | **before** — keeping well |
| **പനി**, **ചുമ** | *pani*, *cuma* | the thing going wrong |
| **ചികിത്സ**, **മരുന്ന്** | *cikitsa*, *marunnŭ* | **after** — getting well |

**`ML-C48-doctor`'s own caveat is carried forward rather than tidied away.** That
lesson says **വൈദ്യൻ** today usually points at a practitioner of the traditional
medicine, and that the hospital doctor goes by the English word worn down into
Malayalam sounds. `ML-C107-chikitsa` repeats it, and marks its *person / practice
/ substance* table as **one** of the two pictures Kerala keeps side by side.

#### Three words, because they are the label's three parts

**മരുന്ന്**, **ചികിത്സ** and **വൃത്തി** are medicine, treatment and hygiene, one
word each.

**ആശുപത്രി** (hospital) and **ഗുളിക** (tablet) were **dropped**, and the honest
reason is that **neither is in the label**. The first draft gave the script debt
as the reason and that argument does not hold: **U+0D36** and **U+0D33** are
never-taught characters, but they already stand in **17** and **24** headwords
respectively, so one more apiece would not have changed their standing.

#### What is genuinely new here is the cluster ത്സ

Every **codepoint** in the three words is already script-taught and already in use
in headwords. But **ത** + chandrakkala + **സ** is the corpus's **first** occurrence
of that conjunct in any headword, and the first draft's "every glyph is already in
use" quietly let it pass as familiar. `ML-C107-chikitsa` now takes it apart in a
**The word taken apart** block, against `ML-S08-letter-ta`, `ML-S124-letter-sa` and
the chandrakkala join **നമസ്കാരം** taught on day one. `malayalam-conjunct-tsa` is
added to the sound-tag registry beside `malayalam-conjunct-kssa`.

#### വൃത്തി is glossed honestly as the everyday word

It is **cleanliness** — what you say about a room or a person's hands — **not**
hygiene as a subject. The lesson says so outright rather than letting the exam
label's English word stand unexamined, and it **names** Malayalam's formal word
*śucitvaṁ* in romanization only, as a word this book has not taught.

Its Sanskrit sense is **conduct**, a way of living, and the road from conduct to
cleanliness is offered as **the traditional account rather than asserted** —
which is the treatment **പള്ളിക്കൂടം** got in chapter 103, after review caught the
first draft asserting it.

#### The native/borrowed split is the second thread — a fact about three words, not a pattern

**മരുന്ന്** is Dravidian — Tamil has *marundhu*. **ചികിത്സ** and **വൃത്തി** are
both Sanskrit. The substance kept its own name; the practice and the virtue came
in borrowed.

The first draft called this "the same shape the school words showed" and that
claim is **dropped**. It is not the same shape: **പാഠം** and **പരീക്ഷ** are
Sanskrit, **മാർക്ക്** is English and **പള്ളിക്കൂടം** is a local compound — a story
about **institutions**, with no substance in it to keep its own name.

#### Verification

The three ceiling greps run **in draft** caught **two chapter numbers** in the
recall before they reached the gate. The full suite passed on the **first run for
the third chapter running** — 145 files, 2085 passed, 1 skipped — and
`check-book-compile.sh --strict malayalam` compiled and verified.
