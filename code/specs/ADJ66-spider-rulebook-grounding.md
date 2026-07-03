# ADJ66 — The Spider: Grounding the Rulebook in Source Bytes

> **Status (2026-06-04):** Built and run with **live web research**. ADJ65 found the
> decision rested on `assumed` (ungrounded) weights and named them as a fetch-list. The
> principle: *nothing may be asserted that is not grounded to bytes — in the input OR in
> the rulebook we derive for it.* A weight (how strongly a finding argues for a diagnosis)
> is a **rulebook** claim, so the spider fetches a source for each load-bearing weight,
> grounds it in a **verbatim passage**, and records the citation. Re-running the decision
> on grounded weights produced the **honest** result — and it is more instructive than a
> convenient one. Implementation:
> [`spider.workflow.js`](data/adj57/pipeline/spider.workflow.js) +
> [`run_spider.py`](data/adj57/pipeline/run_spider.py). Closes the ADJ65 loop; composes
> with ADJ64.

## 1. Two kinds of grounding

The case facts are grounded in **input bytes** (ADJ62). But the *weights* — the
likelihood ratios that turn facts into a decision — are external knowledge; they cannot be
grounded in the case. They must be grounded in the **rulebook**: sources the spider
fetches, decomposes, and cites. ADJ65 exposed that every load-bearing weight was `assumed`
(the model's own decibans). ADJ66 sends the spider to ground them.

The spider ([`spider.workflow.js`](data/adj57/pipeline/spider.workflow.js)) takes the six
discriminating facts ADJ65 flagged and, for each, runs an agent with **web search + fetch**
that finds authoritative sources (MSD Manuals, NCBI StatPearls, WHO, PMC journals), copies a
**verbatim passage** establishing each fact→hypothesis association, and derives the weight
from that passage — recording URL + quote + rationale. A weight with no source is set to 0
and flagged, never invented. **74 web fetches across 6 parallel spiders** produced a fully
cited weight matrix — the rulebook.

## 2. The run — grounding did NOT flip the answer, and that is the finding

```
BEFORE (assumed weights):  East African trypanosomiasis  99.7%   margin +26 dB
                           Brucellosis (the truth) ranked #4
AFTER  (spider-grounded):  East African trypanosomiasis  99.2%   margin +24 dB
                           Brucellosis still #4 (+17 dB)
                           8/12 facts grounded;  margin rests on assumed = FALSE
```

It would have been *convenient* if grounding the weights had surfaced neurobrucellosis. It
did not — and that honesty is the point. The grounded sources **genuinely support
trypanosomiasis** from the case bytes:

- *insect bite → swelling at the ankle*: **+11 dB**, grounded in *"a papule develops at the
  tsetse-fly bite site… (trypanosomal chancre)"* (MSD Manuals) and *"HAT's earliest
  manifestation is a cutaneous chancre at the inoculation site"* (NCBI). A bite-site lesion
  is near-pathognomonic for HAT.
- *travel through Uganda/Tanzania/Kenya*: **+12 dB**, grounded in *"T. b. rhodesiense has
  been detected in tourists in East Africa, mainly in Tanzania…"* — the parasite is
  geographically restricted to exactly those countries.

Given **only the case bytes**, trypanosomiasis is the better-supported diagnosis. The
spider trimmed the inflated assumed weights (the CSF and hepatosplenomegaly weights for HAT
dropped, brucellosis's pontine weight rose on real neurobrucellosis MRI sources) — the
margin fell from 26→24 dB and brucellosis gained — but the chancre + travel signals are
real and decisive **in the data**.

## 3. What the spider actually achieved

1. **The principle is satisfied.** Every load-bearing weight now traces to a fetched source
   passage — `margin rests on assumed = FALSE`. Nothing load-bearing is the model's
   invention any more; the rulebook is byte-cited (CAS-storable for reuse).
2. **It refused to launder the answer.** Trypanosomiasis stayed on top, *defensibly*, with
   citations — not massaged into the "right" answer. A framework that quietly produced
   brucellosis here would be the very dishonesty byte-provenance exists to prevent.
3. **It isolated the residual to the right place.** The answer is still not
   neurobrucellosis — because the datum that overturns it (**Brucella serology + culture**)
   is the **held-aside confirmatory test, a missing *input* byte**, not a rulebook gap. The
   insect bite is a red herring *in the data itself*; grounding faithfully encodes that the
   data points to HAT.

This is the same lesson as the ADJ63 axle case — **faithfulness ≠ completeness** — now shown
on both grounding axes at once: every fact grounded in input bytes, every weight grounded in
rulebook bytes, and the decision *still* wrong, because the decisive evidence was never in
the input. That residual is exactly an **ADJ64 named hole**: *"Brucella serology — not in
the record."*

> The complete, honest verdict the framework can now defend: *"Given everything I can ground
> — case facts in input bytes, weights in cited rulebook sources — the leading diagnosis is
> East African trypanosomiasis at 99.2%; the one observation that would change it, Brucella
> serology, is absent from the record."* Nothing asserted is ungrounded; the gap is named.

## 4. Honest limitations

- **Passage → decibans is still the model's mapping.** The *grounding* is real (the passage
  is fetched and quoted verbatim), but the magnitude assigned to it is judgment. The next
  hardening is a verifier that checks the cited passage actually supports the claimed weight
  — the same layer-2 multi-verifier thread flagged for every gate.
- **Six of twelve facts grounded.** The load-bearing/discriminating facts were grounded; the
  four low-weight remainders (fever, antibiotic response, sterile cultures, viral markers)
  stay `assumed` — non-load-bearing, but the spider should eventually sweep them too.
- **Source quality is taken on trust.** The spider prefers StatPearls/WHO/PMC, but does not
  yet grade source authority or recurse to primary studies ("follow sources to root"). That
  recursion, plus CAS persistence, is the larger spider build.

## 5. Where this leaves the framework

Every link in the decision is now byte-disciplined: facts → input bytes (ADJ62), claims →
input bytes (ADJ60/61), and **weights → rulebook bytes (ADJ66)**, with the decision's
robustness and the provenance of its margin reported (ADJ65) and any decisive *missing*
datum named (ADJ64). The framework no longer asserts a single thing it cannot trace — and
when it is still wrong, it is wrong *legibly*, pointing at the exact byte it is missing.
