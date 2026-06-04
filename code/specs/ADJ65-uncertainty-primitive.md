# ADJ65 — Uncertainty as a First-Class Primitive (weight of evidence + decision sensitivity)

> **Status (2026-06-04):** Built and run. The gates so far answer yes/no — grounded?
> determined? ADJ65 makes the *competition between hypotheses* first-class and answers the
> question Adhithya posed: **"if we make some probability shift, how would the decision
> shift?"** Each grounded fact carries a weight of evidence (decibans) toward each
> hypothesis; the deterministic engine computes the decision **and its sensitivity** — the
> margin, the load-bearing evidence, what flips it, and whether the margin rests on
> ungrounded weights. Run on the neurobrucellosis case it produced a *confidently wrong*
> answer **and flagged precisely why it can't be trusted** — the best possible
> demonstration of the primitive. Implementation:
> [`sensitivity.py`](data/adj57/pipeline/sensitivity.py) +
> [`sensitivity.workflow.js`](data/adj57/pipeline/sensitivity.workflow.js).

## 1. The model

Good's weight of evidence — the formulation behind MYCIN's certainty factors. Each fact
E_j contributes, for each hypothesis H_i, a weight in **decibans** (ten times the log10
likelihood ratio): +10 dB ⇒ the fact makes H_i ten times more likely. Scores add in log
space; the **decision is the argmax** — deterministic, no softmax, no temperature (we show
posteriors via softmax only as a *view*; nothing depends on it, so the past softmax mistake
is not repeated). The model's *job* is only to propose the hypotheses and the weight matrix,
each weight tagged **grounded** (cites a real likelihood ratio) or **assumed** (an
estimate). The engine does the rest. 10 unit tests
([`test_sensitivity.py`](data/adj57/pipeline/test_sensitivity.py)).

## 2. Why sensitivity, not a point probability

A single winner is the least useful output. What a decision-maker needs is how *robust* the
call is:

- **margin** M = score(leader) − score(runner-up), in decibans — the decision survives any
  single weight perturbation smaller than M;
- **load-bearing** evidence, ranked by how much each fact pushes leader-over-runner;
- **what flips it** — which single facts, or how many, must fail to change the answer, and
  how far one weight must move (the literal "probability shift → decision shift");
- **provenance of the margin** — of the load-bearing weights, which are grounded and which
  are assumed. *A decision whose margin rests on assumed weights is fragile in the one way
  that matters.*

The weights are the soft underbelly (mostly interpretation, not byte-grounded). The honest
response to soft inputs is not false precision — it is to report exactly how far the
softness can move the answer, and which soft weight to ground first.

## 3. The run — a confidently WRONG answer, correctly exposed

The model weighed 12 grounded facts across 7 hypotheses for the neurobrucellosis case. The
engine's verdict:

```
DECISION: East African trypanosomiasis   99.7%   (+58 dB)
runner-up: visceral leishmaniasis        (+32 dB)
...   Brucellosis (the TRUTH)             4th, +16 dB
MARGIN: +26 dB  (leader ~400x the runner-up's odds)
```

It is **wrong** — the held-aside truth is neurobrucellosis, ranked *fourth*. A naive system
would stop here and report "99.7% trypanosomiasis." ADJ65 keeps going, and that is the whole
point:

```
## What would flip the decision:
   - no SINGLE fact's removal flips it; minimum that must fail: 4
## PROVENANCE OF THE MARGIN:
   ⚠ the margin RESTS ON ASSUMED weights:
       ? pontine_t2_flair_lesions_meningeal_enhancement (+10 dB)
       ? ankle_swelling_after_insect_bite               (+9 dB)
       ? travel_to_east_africa                          (+7 dB)
       ? csf_acellular_normal_glucose_raised_protein    (+6 dB)
```

**Every load-bearing weight is `assumed`.** The only *grounded* weights in the whole model
(the negative malaria smear, the negative Widal) contribute essentially **0 dB** to the
leader-over-runner margin — they rule out malaria and typhoid, which were not in contention.
So the 99.7% confidence is an artifact of four numbers the model **made up**. The single
biggest one — *insect bite → trypanosomiasis, +13 dB* — is the very red herring ADJ60 fell
for (the bite was the brucellosis inoculation site, not a tsetse bite).

> ADJ65 does not make the model right. It makes the model's **confidence auditable**. A
> 99.7% built on assumed weights is correctly exposed as untrustworthy — and converted into
> a *prioritized fetch-list*: ground these four weights before believing the answer.

## 4. The through-line — this is the spider's priority queue

This composes exactly with ADJ64. Trypanosomiasis-vs-brucellosis is *underdetermined* (the
discriminating datum — Brucella serology — was held aside). ADJ64 names the missing data;
ADJ65 says **which missing weight the decision is most sensitive to**, so the spider/CAS
fetches in priority order. Grounding the four assumed load-bearing weights — most of which a
real LR would shrink dramatically — would very likely collapse trypanosomiasis and surface
brucellosis. The framework can't know the answer from incomplete, ungrounded inputs; it can
tell you **exactly which input to ground first to find out.**

## 5. Honest limitations

- **The weights are model-proposed.** ADJ65 quantifies and audits them but does not ground
  them — that is the spider's job, now prioritized. The primitive's value is precisely that
  it refuses to launder assumed weights into trustworthy confidence.
- **`grounded` is taken on the model's word here.** A weight tagged grounded should itself
  carry a CAS-citable likelihood ratio that a deterministic check verifies (as `cite()`
  does for source spans) — the next hardening, mirroring the layer-2 multi-verifier thread.
- **Single decision, single level.** v1 audits one decision; propagating uncertainty across
  a multi-step derivation (each step's margin feeding the next) is the larger build.

## 6. Where this leaves the framework

Four epistemic states, all byte-disciplined: a claim is **grounded**, a conclusion is
**determined** or **underdetermined**, and now a determination carries a **robustness** —
its margin, and whether that margin stands on grounded or assumed weights. The framework no
longer reports confidence it cannot back; it reports confidence *and the provenance of that
confidence*, and turns the gap into the next thing to fetch.
