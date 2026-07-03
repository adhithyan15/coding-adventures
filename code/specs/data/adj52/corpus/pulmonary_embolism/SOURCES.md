# Pulmonary embolism — grounded corpus provenance

Every likelihood ratio below was derived by a forward byte-provenance crawl to
primary data (Phase 1 of the [ADJ55](../../../ADJ55-provenance-first-corpus.md)
proof). **12/12 links grounded.** The full byte-anchored chains (with literal
source quotes) are in [`../../provenance/pe/grounding-results.json`](../../provenance/pe/grounding-results.json).

| finding | LR | formula | primary source |
|---|---|---|---|
| **prior** (prevalence) | 0.192 | Prior = total PE / total worked-up = (226 + 408) / 3306 = 63 | The Christopher Study Investigators (van Belle A, et al.). "Effectiven |
| d_dimer(elevated) | 1.64 | LR+ = 0.97 / (1 - 0.41) = 0.97 / 0.59 = 1.64 | Stals MAM, Klok FA, et al. (CTEPH/PE diagnostic accuracy network). Sys |
| d_dimer(normal) | 0.073 | LR- = (1 - sensitivity) / specificity = (1 - 0.97) / 0.41 =  | Patel P, Patel P, Bhatt M, Braun C, Begum H, Wiercioch W, ... Lim W, L |
| clinical_signs_of_dvt(present) | 2.11 | The Wells 'clinical signs of DVT' criterion is leg swelling  | West J, Goodacre S, Sampson F. 'The value of clinical features in the  |
| pe_is_leading_diagnosis(present) | 4.2 | No sensitivity/specificity is published for this single Well | Klok FA, Kralingen KW, van Dijk APJ, et al. 'Alternative diagnosis oth |
| heart_rate(over_100) | 1.8 | LR+ = P(tachycardia|PE) / P(tachycardia|no PE) = sensitivity | Marchick MR, Courtney DM, Kabrhel C, et al. '12-lead ECG findings of p |
| recent_immobilization_or_surgery(present) | 2.18 | OR = exp(0.78) = 2.18; used as LR+ proxy. Prior = 0.20 (Chri | Le Gal G, Righini M, Roy PM, Sanchez O, Aujesky D, Bounameaux H, Perri |
| previous_vte(present) | 7.796 | No clean PE-only sensitivity/specificity is published for th | Wong et al., 'Cohort study of prediction of venous thromboembolism in  |
| hemoptysis(present) | 2.84 | OR = (a*d)/(b*c) with a=PE+hemoptysis=3, b=PE+no-hemoptysis= | Bannelier H, Gorlicki J, Penaloza A, et al. Evaluation of the "hemopty |
| active_malignancy(present) | 1.74 | Active cancer is a single clinical criterion for which the p | West J, Goodacre S, Sampson F. The value of clinical features in the d |
| ctpa(filling_defect_positive) | 20.75 | LR+ = sensitivity / (1 - specificity) = 0.83 / (1 - 0.96) =  | PIOPED II — Stein PD, Fowler SE, Goodman LR, et al. Multidetector comp |
| ctpa(negative) | 0.18 | LR- = (1 - sensitivity) / specificity = (1 - 0.83) / 0.96 =  | Stein PD, Fowler SE, Goodman LR, et al. Multidetector Computed Tomogra |

## The load-bearing insight

`d_dimer(elevated)` grounds to **LR+ = 1.64** (sens 0.97 / spec 0.41, Blood
Advances 2020 meta-analysis, 34 studies / 22,849 patients) — a *weak* positive,
because D-dimer is a rule-OUT test. An inventing deriver that hands a positive
D-dimer a large LR over-calls PE; the grounded number cannot. This is exactly the
clause that decided the validation case (PMC11999957): grounded, P(PE) stays at
0.28 pretest (mandating the CTPA that found the clot); the ungrounded deriver
invented its way to 0.01 and excluded a real PE.
