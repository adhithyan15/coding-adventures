# MYCIN-2026 — worked audit trails (proof DAGs)

Rendered from the content-addressed CAS library `c3551b18e5f3b86e` by re-running the engine. Each row is one fired clause; the running P is the posterior after applying it. A reviewer audits the decision line by line — the trail is the decision.

### MEN-1 — leader: `bacterial_meningitis`  (decision: determinate)

| step | evidence | log-LR | running P | cited source | trust |
|---|---|---:|---:|---|---|
| prior |  | -3.259 | 0.0370 | Nigrovic 2007 JAMA (PMID 17200475), n=3295 | authoritative |
| contribution | csf_gram_stain(positive) | +4.443 | 0.7656 | WHO 2025 guideline NBK614844 (pooled Straus 2006 | consensus |
| contribution | csf_neutrophilic_pleocytosis(high) | +2.708 | 0.9800 | Straus 2006 JAMA (PMID 17062865); CSF WBC >=500/ | authoritative |
| contribution | serum_procalcitonin(elevated) | +3.307 | 0.9993 | Vikse 2015 Int J Infect Dis (PMID 26188130), 9 s | authoritative |
| contribution | seizure(present) | +1.765 | 0.9999 | Nigrovic 2002 Pediatrics; seizure 22% bacterial  | empirical |

**Final P(bacterial_meningitis) = 0.9999** — every step traces to a cited source clause; no model was consulted to produce or to audit this.

### MEN-3 — leader: `viral_meningitis`  (decision: determinate)

| step | evidence | log-LR | running P | cited source | trust |
|---|---|---:|---:|---|---|
| prior |  | +3.259 | 0.9630 | Nigrovic 2007 JAMA complement; aseptic 3174/3295 | authoritative |
| contribution | csf_glucose(normal) | +1.589 | 0.9922 | Straus 2006 JAMA, derived: normal glucose LR for | authoritative |
| contribution | csf_lactate(normal) | +2.617 | 0.9994 | Sakushima 2011, derived: normal lactate LR for a | authoritative |
| contribution | csf_lymphocytic_pleocytosis(high) | +1.609 | 0.9999 | [inferred] lymphocyte predominance is characteri | inferred |
| contribution | enteroviral_pcr(positive) | +3.912 | 1.0000 | [inferred] CSF enterovirus RT-PCR is near-diagno | inferred |

**Final P(viral_meningitis) = 1.0000** — every step traces to a cited source clause; no model was consulted to produce or to audit this.

