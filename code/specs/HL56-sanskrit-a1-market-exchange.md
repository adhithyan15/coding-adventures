# HL56 — Sanskrit A1 market exchange from owned words

## Status

Proposed for issue #17536 under #13415 and #13295.

## Problem

The project-defined Sanskrit A1 points `SA-A1-EV-02` and `SA-A1-M-09` say
that value, price, buying, selling, a shop, and a shopkeeper are absent. Those
notes are stale. Chapter 74 owns **क्रीणाति** (*krīṇāti*, “buys”), **धनम्**
(*dhanam*, “money”), **मूल्यम्** (*mūlyam*, “a price”), **आपणः**
(*āpaṇaḥ*, “a shop”), and **विपणिः** (*vipaṇiḥ*, “a market”). Later chapters
own **विक्रीणाति** (*vikrīṇāti*, “sells”), **मूल्यवान्** (*mūlyavān*,
“valuable”), **आपणिकः** (*āpaṇikaḥ*, “a shopkeeper”), **पण्यम्**
(*paṇyam*, “goods for sale”), and **रूप्यकम्** (*rūpyakam*, “a rupee”).

The two A1 mocks already require a complete transaction: choose an item, ask
or confirm its price, request a quantity, and close politely. The lessons own
the pieces but do not compose that exchange, and they teach only third-person
**क्रीणाति**, not the productive first-person **क्रीणामि** used by both mocks.
Mapping isolated market nouns would therefore overclaim functional coverage.

## Contract

Append one Sanskrit-owned chapter after chapter 265 with six lessons:

1. retrieve the already-owned buy, sell, money, price, shop, market,
   shopkeeper, goods-for-sale, and rupee atoms;
2. bridge from known **क्रीणाति** to the complete form **क्रीणामि** and use it
   with a familiar item;
3. compose a price question and a short price answer in rupees, using the
   wording already present in the checked-in A1 mocks;
4. join choosing, buying, asking the price, requesting the item, and thanking;
   and
5. finish with two successive model-free mixed-transaction rounds in which
   the learner chooses a familiar item and completes the exchange
   independently.

The ramp teaches whole usable phrases before it names the first-person ending.
The checked-in A1 mock scripts anchor **अहं ... क्रीणामि**, **मूल्यं कियत्?**,
and price answers in **रूप्यकाणि**. The lesson must not turn one present form
into a complete class-nine conjugation algorithm or paradigm.

## Knowledge ownership

The six lessons own exactly two new atoms:

- `SA-VERB-C266-KRINAMI-01`
- `SA-PATTERN-C266-MARKET-EXCHANGE-02`

The first is introduced by the second lesson and the second by the third. Both
are retrieved by each remaining lesson. The last two lessons introduce no
atoms, require every point atom, and show no copyable Sanskrit answer before
the learner responds.

The exact `SA-A1-EV-02` probe is:

- `SA-LEX-C74-MARKET-01`
- `SA-LEX-C74-MARKET-02`
- `SA-LEX-C74-MARKET-03`
- `SA-LEX-C138-ACT138-01`
- `SA-LEX-C215-QUAL215-05`
- `SA-LEX-C248-THG248-03`
- `SA-VERB-C266-KRINAMI-01`
- `SA-PATTERN-C266-MARKET-EXCHANGE-02`

The exact `SA-A1-M-09` probe is:

- `SA-LEX-C74-MARKET-01`
- `SA-LEX-C74-MARKET-03`
- `SA-LEX-C74-MARKET-04`
- `SA-LEX-C74-MARKET-05`
- `SA-LEX-C222-THG222-03`
- `SA-LEX-C222-THG222-04`
- `SA-LEX-C248-THG248-03`
- `SA-LEX-DHANYAVADA`
- `SA-GRAMMAR-C265-DADATU-01`
- `SA-PATTERN-C265-POLITE-ORDER-02`
- `SA-VERB-C266-KRINAMI-01`
- `SA-PATTERN-C266-MARKET-EXCHANGE-02`

## Reinforcement boundary

Appending six lessons makes older atoms newly judgeable in R2, R3, and R4.
In lesson order, the warm-ups must retrieve these groups at the first valid
position:

1. `SA-LEX-C248-THG248-04`, `SA-GRAMMAR-C261-DUAL-OBJECTS-03`, and
   `SA-GRAMMAR-C265-DADATU-01`;
2. `SA-LEX-C248-THG248-05`, `SA-PRON-C262-ASMAN-01`, and
   `SA-PATTERN-C265-POLITE-ORDER-02`;
3. `SA-LEX-C249-THG249-01` and `SA-PRON-C262-YUSMAN-02`;
4. `SA-LEX-C249-THG249-02` and
   `SA-GRAMMAR-C262-OBJECT-PERSON-NUMBER-03`;
5. `SA-LEX-C249-THG249-03`; and
6. `SA-LEX-C249-THG249-04` and `SA-VERB-C263-GAMISYAMI-01`.

This boundary work is short recall, not new teaching. The two new atoms still
need ordinary R1 revisits inside chapter 266.

## Ownership and sharding

Lesson files, chapter ledger, curriculum extension/path/membership owners,
book target, generated modality/book/narration owners, corpus evidence, the
operational lesson, and the Sanskrit changelog entry must be Sanskrit-owned or
single-record additive files. The two inventory points and existing Sanskrit
aggregate pins are the only same-track files whose measured contract changes.
No cross-language mutable aggregate may be introduced.

## Evidence boundary

This slice closes two project-defined editorial A1 points. It proves one small
market exchange with familiar items and prices. It does not prove complete
commerce, currency, bargaining, consumer-rights, or present-paradigm coverage;
nor does it prove B1 human validation, exam readiness, learner success, or
equivalence to a traditional Sanskrit examination. The A1 inventory remains
partial.

## Acceptance

- A Sanskrit-owned red-before-green test pins all six lessons, their order,
  exact ownership, prerequisites, duration ceilings, chapter payoff, and both
  exact inventory probes.
- No owned market, price, item, rupee, request, or thanks vocabulary is
  retaught.
- The first new form starts from **क्रीणाति**; no learner infers an unseen stem.
- Every lesson stays within five minutes, and the final two introduce nothing.
- Learner copy states that **क्रीणामि** is the taught first-person form here,
  not a complete present paradigm.
- Sanskrit A1 coverage moves from 147/164 to 149/164, with 15 unmapped and
  zero partial points.
- The exact completion plan reports 15 outstanding
  `exam-point/sanskrit/A1` points; its tranche count is measured after authoring.
- Corpus, curriculum, shard, document-shard, modality, generated-book,
  narration, lesson, figure, compile, affected-package, and full book checks
  pass.
- Independent security and data-integrity review passes on the exact pushed
  head before every push.
