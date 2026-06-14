# ADJ99 — HLE-100 run (in progress, autonomous, 5 questions/batch)

Goal: test whether **Haiku + framework is as DEFENSIBLE as plain Opus**, and **Opus + framework is slightly more defensible than plain Opus**. Accuracy is measured but NOT the target. The deliverable is an **audit trail a human can inspect to find where reasoning went wrong — or a CAS fact that was extracted incorrectly that can be overridden and fixed**.

## Arms (per question)
- `plain-haiku`, `plain-opus` — one-shot baselines
- `fw-haiku` — Haiku spider → CAS → provenance-gated cited reasoning chain
- `fw-opus` — same, all-Opus

Primary metric: **defensibility** (blind Opus judge, 0–5, grounded/auditable/traceable, independent of correctness). Byte provenance enforced at every layer (spider facts cite a source; every chain step cites a CAS fact or the givens; a grounded adversarial read drops unsupported steps).

## Audit-trail verification (on every fw-haiku trail)
A fresh **same-model adversarial Haiku** AND a fresh **cross-model adversarial Opus** each localize the flaw and flag whether it traces to a **CAS fact extracted incorrectly** (`flaw_is_cas_extraction`, `flagged_cas_facts`) — the correctable-CAS deliverable.

## Raw data
`items_100.json` (the frozen item set) and `batches/batch_NN.json` (full per-arm answers, trails, audits, judge scores). Each batch is committed + pushed to this PR the moment it completes. `FINDINGS.md` + `aggregate.json` are written at the end.
