# Treatment constraints (CC-3) — the grounded contraindication / interaction rules

CC-1/CC-2 turn chart facts into the optimizer's exclusions + dose-feasibility constraints.
CC-3 grounds the clinical **rules** behind them through the same cold path, so the
constraints rest on byte-provenanced facts, not authored ones.

- `ground-treatment-constraints.workflow.js` (in `grounding/workflows/`) — the spider:
  grounds each rule against a primary source (FDA label / CDC / IDSA / ACOG) with a verbatim
  byte-quote + an independent adversarial re-extraction.
- `ci_ground.py` — the write gate: consumes the spider output and emits the content-addressed
  `treatment-constraints.json` (each rule with verdict ACCEPT/FLAG, trust, byte-quote, and the
  structural EFFECT it justifies). Reuses the organism-id gate's verdict/cite/safe_status.

Result (8 rules): **4 ACCEPT grounded · 4 FLAG (direction_only)**. Grounded: penicillin↔
3rd-gen cephalosporin cross-reactivity <1% (CDC), TMP-SMX contraindicated in pregnancy,
vancomycin nephrotoxicity, vancomycin renal dose adjustment. The adversarial verifier
correctly downgraded the "aztreonam is *safe* in β-lactam allergy" claim to FLAG — the FDA
label hedges "cross-reactivity is rare, administer with caution", not "safe".

**Wired into the optimizer:** a `pregnancy=present` chart fact now excludes the
pregnancy-contraindicated drugs (moxifloxacin, TMP-SMX) by name in `chart_to_cop.py`, each
with its own provenance constraint — a pregnant penicillin-allergic patient correctly
abstains (no safe agent left). The remaining FLAG rules (QT, additive nephrotoxicity) are
recorded for follow-up wiring. Sources are not yet decomposed → the system ledger shows the
treatment-constraints citations as pending (a `decompose-source` follow-up).
