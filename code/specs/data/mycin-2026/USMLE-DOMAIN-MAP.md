# USMLE-DOMAIN-MAP — every domain needed to pass the medical licensing exams

This is the campaign roadmap for the board-exam north star: get MYCIN-2026 to pass the
United States Medical Licensing Examination (USMLE Step 1, Step 2 CK, Step 3) and the
subspecialty boards, built **organ-by-organ, discipline-by-discipline**, with every
answer a citable, correctable proof — never a hallucination. It enumerates the full
domain space (so nothing is missed), maps each cell to the engine tactic that answers
it, records what is already built, and sets a prioritized build order.

Sources (the official content classification this map is grounded in):
- [USMLE Step 1 Content Outline & Specifications](https://www.usmle.org/exam-resources/step-1-materials/step-1-content-outline-and-specifications)
- [USMLE Step 2 CK Content Outline & Specifications](https://www.usmle.org/exam-resources/step-2-ck-materials/step-2-ck-content-outline-specifications)
- [USMLE Step 3 Content Outline & Specifications](https://www.usmle.org/exam-resources/step-3-materials/step-3-content-outline-and-specifications)
- [Public USMLE Content Outline (PDF)](https://www.usmle.org/sites/default/files/2022-01/USMLE_Content_Outline_0.pdf)
- [Public USMLE Physician Tasks/Competencies (PDF)](https://www.usmle.org/sites/default/files/2022-01/USMLE_Physician_Tasks_Competencies_2.pdf)

## The three classification axes (USMLE's own)

Every USMLE item is classified on three axes. The domain space is their product.

**Axis A — Organ systems** (the content outline; ~18 fine categories under 11 named
groups). Each system splits into *normal processes*, *abnormal processes*, and
*principles of therapy*:

1. Immune system
2. Blood & lymphoreticular system
3. Behavioral health
4. Nervous system & special senses
5. Skin & subcutaneous tissue
6. Musculoskeletal system
7. Cardiovascular system
8. Respiratory system
9. Gastrointestinal system
10. Renal & urinary system
11. Pregnancy, childbirth & the puerperium
12. Female reproductive system & breast
13. Male reproductive system
14. Endocrine system
15. Multisystem processes & disorders (incl. infectious disease, neoplasia, genetics)
16. Biostatistics, epidemiology & population health
17. Social sciences (communication, ethics, systems-based practice, patient safety)
18. Human development (life-cycle, normal growth)

**Axis B — Disciplines** (the foundational + clinical sciences):
Pathology · Physiology · Pharmacology · Microbiology · Immunology · Biochemistry ·
Nutrition · Genetics · Gross anatomy & embryology · Histology & cell biology ·
Behavioral sciences. Step 2/3 add the clinical disciplines: Internal medicine ·
Surgery · Pediatrics · Obstetrics & gynecology · Psychiatry.

**Axis C — Physician tasks / competencies:**
Medical knowledge (foundational-science recall) · Patient care: **diagnosis** ·
Patient care: **management** · History & physical · Communication & interpersonal
skills · Practice-based learning (biostatistics/EBM) · Systems-based practice & patient
safety · Professionalism.

## Mapping the axes to the ONE engine (the unifying claim)

Every question type collapses to a query over the same grounded knowledge graph + the
native adj-lang engine. There is no per-question-type code — only a tactic:

| Physician task (Axis C) | MYCIN tactic | Engine | Example item |
|---|---|---|---|
| Medical knowledge / recall | **recall** — relational binding query | SLD over grounded edges | "Which enzyme is deficient in Tay-Sachs?" |
| Diagnosis (vignette → dx) | **differential** — LR engine ranks hypotheses | likelihood-ratio proof DAG | "Most likely diagnosis?" |
| Management (next best step) | **management** — chart-as-constraints solve | adj-constraint-solver | "Best empiric regimen?" / INFEASIBLE |
| Biostatistics / EBM | **biostat** — compute over the LR substrate | compute evaluator | sensitivity, PPV, NNT, pre→post-test odds |
| H&P / communication / ethics | (future) constraint + precedence over guideline rules | defeasible-precedence engine | "Next best step in disclosure?" |

Recall is the zero-uncertainty single-hop special case of the differential
([[project_board_exam_goal]]); biostat is the LR engine read near-free. So the whole
exam reduces to: **ground the edges, then query them.** Grounding is always via the
pipeline — spider → byte-provenance → adversarial gate → CAS, nothing human-authored
([[feedback_nothing_human_authored]]).

## Recall relation schema per discipline (the binding-query vocabulary)

Recall is the densest, most automatable region (foundational science). Each discipline
contributes a family of typed relations `relation(subject, $Var)` the engine binds:

- **Biochemistry / IEM** — `deficient_in(disease, $Enzyme)`, `accumulates(disease, $Substrate)`, `inherited_as(disease, $Pattern)`, `cofactor_of(enzyme, $Vitamin)`, `rate_limiting_enzyme(pathway, $Enzyme)`
- **Nutrition** — `deficiency_causes(vitamin, $Disease)`, `classic_finding(vitamin, $Finding)`
- **Hematology** — `has_mcv(anemia, $Class)`, `factor_deficiency(disorder, $Factor)`, `coag_inheritance(disorder, $Pattern)`, `prolonged_test(disorder, $Test)`, `classic_finding(disorder, $Finding)`
- **Endocrine** — `secreted_by(hormone, $Gland)`, `deficiency_syndrome(hormone, $Syndrome)`, `excess_syndrome(hormone, $Syndrome)`, `stimulated_by` / `inhibited_by`
- **Microbiology** — `gram_stain(organism, $Result)`, `morphology(organism, $Shape)`, `causes(organism, $Disease)`, `virulence_factor(organism, $Factor)`, `treated_with(organism, $Drug)`, `transmission(organism, $Route)`
- **Pharmacology** — `mechanism(drug, $MOA)`, `drug_class(drug, $Class)`, `treats(drug, $Indication)`, `adverse_effect(drug, $AE)`, `metabolized_by(drug, $Enzyme)`, `antidote_for(toxin, $Antidote)`
- **Immunology** — `mediated_by(reaction, $Cell)`, `hypersensitivity_type(disease, $Type)`, `deficiency_of(immunodeficiency, $Component)`, `associated_hla(disease, $HLA)`
- **Pathology** — `tumor_marker(neoplasm, $Marker)`, `oncogene(cancer, $Gene)`, `histology(disease, $Finding)`, `associated_with(disease, $Association)`
- **Genetics** — `inheritance(disorder, $Pattern)`, `gene_defect(disorder, $Gene)`, `trinucleotide_repeat(disorder, $Repeat)`, `imprinting(disorder, $Parent)`
- **Anatomy / neuro** — `innervated_by(muscle, $Nerve)`, `lesion_causes(tract, $Deficit)`, `blood_supply(structure, $Artery)`, `dermatome(level, $Region)`

A new discipline = drop in its `<domain>-edges.adj` (gate-generated, spider-grounded) +
its board items + the filename in `board_eval.EDGE_FILES`; the harness scores it
unchanged. Adding a relation just widens the schema — the engine already binds any
`relate(rel, args)`.

## Built vs. to-build matrix

### Built (grounded, scored, merged)
| domain | discipline | relations | edges | board items |
|---|---|---|---|---|
| IEM (inborn errors) | Biochemistry/Genetics | deficient_in, accumulates, inherited_as | 36 (35 grounded) | ✓ |
| Vitamins | Nutrition | deficiency_causes, classic_finding | 16 grounded | ✓ |
| Anemia | Hematology/Path | has_mcv, classic_finding | grounded | ✓ |
| Endocrine | Endocrine/Path | secreted_by, deficiency_syndrome | grounded | ✓ |
| Coagulation | Hematology | factor_deficiency, coag_inheritance, prolonged_test | 15 grounded | ✓ |
| Meningitis | Infectious dz | **differential** (LR) + **management** (constraints) | grounded LRs + formulary | ✓ |
| Bacteremia / UTI | Infectious dz | organism-id + treatment | grounded | ✓ |
| Microbiology | Microbiology | gram_stain, morphology, causes | 13 grounded (ADJ-only) | ✓ |
| Pharmacology | Pharmacology | drug_class, mechanism, adverse_effect, antidote_for | 10 grounded (ADJ-only) | ✓ |
| Immunology | Immunology | mediated_by, associated_hla, gene_defect, deficiency_of | 7 grounded (ADJ-only) | ✓ |
| Genetics | Genetics | inheritance, gene_defect, trinucleotide_repeat, imprinting | 10 grounded (ADJ-only) | ✓ |
| Rheumatology (Tier-2) | Path/Immuno | associated_autoantibody | 11 grounded (ADJ-only) | ✓ |
| Oncology (Tier-2) | Pathology/neoplasia | tumor_marker | 7 grounded (ADJ-only) | ✓ |
| Histology buzzwords (Tier-2) | Pathology | seen_in (finding→condition) | 8 grounded (ADJ-only) | ✓ |
| Cardiology murmurs (Tier-2) | Cardiology | murmur_indicates (murmur→lesion) | 5 grounded (ADJ-only) | ✓ |
| Neurology localization (Tier-2) | Neurology | lesion_causes (site→deficit) | 5 grounded (ADJ-only) | ✓ |
| GI biopsy (Tier-2) | Gastroenterology | biopsy_finding_in (finding→dx) | 6 grounded (ADJ-only) | ✓ |
| Dermatology (Tier-2) | Dermatology | skin_finding_in (lesion→dx) | 6 grounded (ADJ-only) | ✓ |
| Respiratory occupational (Tier-2) | Pulmonology | inhalation_causes (exposure→dz) | 5 grounded (ADJ-only) | ✓ |

Offline pipeline: prose → local-model decompose → ADJ → native engine, two-sided
faithfulness gate, zero online calls (OFFLINE-BOARD-EXAM.md).

### To build (prioritized — highest yield / densest / most automatable first)

**Tier 1 — high-yield foundational recall (Step 1 backbone):**
1. **Microbiology** — bacteria/virus/fungi/parasite → gram-stain, morphology, disease,
   virulence factor, treatment. The single densest recall region; clean relational facts.
   *Started (MICRO):* `gram_stain` / `morphology` / `causes` for 6 board-classic bacteria
   (13 edges, all grounded to byte-stable NCBI StatPearls spans) shipped as the first
   **ADJ-only** domain (`recall/micro-edges.adj` — facts + inline byte-provenance, no
   Python gate / JSON). Remaining: `virulence_factor`, `treated_with`, `transmission`;
   viruses / fungi / parasites; M. tuberculosis (acid-fast) and the declined causes-edges.
2. **Pharmacology** — drug → mechanism, class, indication, adverse effect, antidote.
   Dense, relational, subject-named in stems.
   *Started (PHARM):* `drug_class` / `mechanism` / `adverse_effect` / `antidote_for` for
   6 board-classic drugs (metformin, lisinopril, naloxone, flumazenil, warfarin,
   acetaminophen — 9 edges, all grounded to byte-stable NCBI StatPearls spans) shipped
   as the second **ADJ-only** domain (`recall/pharm-edges.adj`). Remaining: `treats`,
   `metabolized_by`; more antidotes (protamine, vitamin K, atropine, fomepizole);
   declined for now — propranolol class (no drug-named span) and the one-hop
   adverse_effect(lisinopril, dry_cough) (attributed to the class, not the drug).
3. **Immunology** — hypersensitivity types, immunodeficiencies, HLA associations, cytokines.
   *Started (IMMUNO):* `mediated_by` (type I→IgE, type IV→T cells), `associated_hla`
   (ankylosing spondylitis→HLA-B27), `gene_defect` (XLA→BTK, DiGeorge→22q11.2 deletion),
   `deficiency_of` (CGD→NADPH oxidase, DiGeorge→T cells) — 7 edges, all grounded to byte-
   stable NCBI StatPearls spans, shipped as the third **ADJ-only** domain
   (`recall/immuno-edges.adj`). Remaining: type II/III mediators, more HLA links
   (celiac→DQ2, narcolepsy→DR2 — deferred, no clean self-contained span yet), cytokines.
4. **Genetics** — inheritance patterns, trinucleotide repeats, imprinting, gene defects
   (extends the IEM substrate).
   *Started (GENETICS):* `inheritance` (Huntington/Marfan→AD), `trinucleotide_repeat`
   (Huntington→CAG, fragile X→CGG), `imprinting` (Prader-Willi→paternal), `gene_defect`
   (Huntington→HTT, fragile X→FMR1) — 7 edges, all grounded to byte-stable NCBI StatPearls
   spans, shipped as the fourth **ADJ-only** domain (`recall/genetics-edges.adj`; reuses
   `gene_defect` from IMMUNO over disjoint disorders). Remaining: X-linked / AR patterns
   (Duchenne, CF), more repeats (myotonic→CTG, Friedreich→GAA), Angelman→maternal,
   Marfan→FBN1 (deferred — its page's FBN1 mentions are citation titles / negative clauses).

**Tier 2 — organ-system pathology (Step 1/2 diagnosis):**
5. Cardiovascular (murmurs→lesion, ECG→arrhythmia, drug→effect)
    *Started (CARDIO — fourth Tier-2 domain):* `murmur_indicates` for mitral regurgitation
    (holosystolic apical), aortic stenosis (crescendo-decrescendo systolic), mitral valve
    prolapse (late systolic with mid-systolic click), tricuspid regurgitation (holosystolic
    at the lower-left sternal border), aortic regurgitation (decrescendo diastolic) — 5
    edges, all grounded to byte-stable NCBI StatPearls spans naming both murmur and lesion,
    shipped as the eighth **ADJ-only** domain (`recall/cardio-edges.adj`). Two lesions share
    the "holosystolic" timing; the finding name carries the disambiguating auscultation site
    (apex vs lower-left-sternal-border). Remaining: mitral stenosis (deferred — opening snap
    and diastolic rumble described in separate sentences, no both-endpoints-named span yet);
    ECG→arrhythmia and drug→effect subdomains.
6. Respiratory (PFT pattern→disease, ABG→disorder)
    *Started (RESP — eighth Tier-2 domain):* `inhalation_causes` for silica→silicosis
    (NBK537341), asbestos→asbestosis (NBK555985), beryllium→berylliosis (NBK470364),
    cotton dust→byssinosis (NBK519549), and **coal dust→coal workers' pneumoconiosis
    grounded from the CDC/NIOSH primary federal authority** (cdc.gov/niosh/cwhsp) rather
    than declined — 5 edges, all grounded to byte-stable spans naming both the occupational
    exposure and the pneumoconiosis, shipped as the twelfth **ADJ-only** domain
    (`recall/resp-edges.adj`). This is the first edge sourced outside StatPearls: under the
    **primary-source-first, zero-deferral policy**, an association is grounded to whatever
    primary authority carries the clean both-endpoints span, never omitted for phrasing.
    Remaining: PFT-pattern→disease (obstructive/restrictive) and ABG→acid-base-disorder.
7. Renal/urinary (acid-base, electrolyte, glomerular disease→finding)
8. Gastrointestinal (LFT pattern→disease, biopsy→diagnosis)
    *Started (GI — sixth Tier-2 domain):* `biopsy_finding_in` for villous atrophy→celiac
    (NBK441900), transmural inflammation→Crohn (NBK436021), goblet cells / intestinal
    metaplasia→Barrett esophagus (NBK430979), absence of ganglion cells→Hirschsprung
    (NBK562142), ≥15 eosinophils/HPF→eosinophilic esophagitis (NBK459297) — 5 edges, all
    grounded to byte-stable NCBI StatPearls spans naming both finding and diagnosis, shipped
    as the tenth **ADJ-only** domain (`recall/gi-edges.adj`). Deferred — ulcerative colitis
    (crypt distortion sentence omits the disease name) and Whipple (PAS-foamy-macrophage
    criterion omits the disease name in the same span); LFT-pattern→disease subdomain.
9. Neurology & special senses (lesion→deficit, tract→sign)
    *Started (NEURO — fifth Tier-2 domain):* `lesion_causes` for Wernicke area→fluent
    aphasia, Broca area→nonfluent aphasia, arcuate fasciculus→conduction aphasia (NBK559315),
    substantia nigra→Parkinson disease (NBK470193), subthalamic nucleus→hemiballismus
    (NBK559002) — 5 edges, all grounded to byte-stable NCBI StatPearls spans naming both the
    lesion site and the deficit, shipped as the ninth **ADJ-only** domain
    (`recall/neuro-edges.adj`). Remaining: spinal-cord tracts (Brown-Séquard, dorsal column,
    corticospinal), cranial-nerve lesions, cerebellar→ataxia (deferred — its page names the
    "cerebellar syndrome" and limb incoordination but not "ataxia" alongside the site in one
    declarative); tract→sign and visual-pathway-defect subdomains.
10. Musculoskeletal / rheumatology (autoantibody→disease)
    *Started (RHEUM — first Tier-2 domain):* `associated_autoantibody` for SLE (anti-dsDNA,
    anti-Sm, anti-ribosomal-P), GPA (PR3-ANCA), systemic sclerosis (anti-Scl-70,
    anti-centromere) — 6 edges, all grounded to byte-stable NCBI StatPearls spans, shipped
    as the fifth **ADJ-only** domain (`recall/rheum-edges.adj`). **Backfilled** (primary-
    source-first, zero-deferral): RA→anti-CCP (NBK441999, ACPA defined = anti-CCP on page)
    and Sjögren→anti-Ro/SSA + anti-La/SSB (NBK431049, one span names disease + both markers)
    — now 9 grounded edges. Still pursued for the next backfill (need a both-endpoints span
    from a primary source): PBC→anti-mitochondrial, polymyositis/antisynthetase→anti-Jo1.
11. Skin/derm (lesion→diagnosis)
    *Started (DERM — seventh Tier-2 domain):* `skin_finding_in` for target lesions→erythema
    multiforme (NBK470259), silvery scales→psoriasis (NBK448194), umbilicated papules→
    molluscum contagiosum (NBK441898), herald patch→pityriasis rosea (NBK448091) — 4 edges,
    all grounded to byte-stable NCBI StatPearls spans naming both finding and diagnosis,
    shipped as the eleventh **ADJ-only** domain (`recall/derm-edges.adj`). Deferred — impetigo
    (its honey-colored-crust sentence does not name impetigo; only a negative "Bullous
    impetigo does not form a honey-colored crust") and herpes zoster (dermatomal-distribution
    sentences omit the disease name). Spans favor the disease-named declarative; the molluscum
    span uses the page's defined abbreviation "MC".
12. Reproductive (ob/gyn + male), Breast

**Tier 3 — clinical reasoning (Step 2/3):**
13. Biostatistics & EBM as a first-class **biostat** tactic (sensitivity/PPV/NNT/LR,
    pre→post-test odds) over the compute evaluator — near-free given the LR engine.
14. Behavioral health / psychiatry (criteria→diagnosis; constraint over DSM-style rules).
15. Management verticals beyond meningitis (chart-as-constraints per organ system).
16. Ethics / communication / patient-safety (defeasible-precedence over guideline rules).

## Build protocol (per domain, one PR each)

Each domain ships an **ADJ-only** library — the `.adj` file is the sole source of
truth, carrying its facts AND byte-provenance inline (no Python gate, no JSON, no
manifest). MICRO onward, the order is grounding-FIRST (never seed authored-debt):
1. **Spider-ground first** — act as the spider: fetch an authoritative source per edge,
   extract the VERBATIM self-contained span (byte-stable: re-fetch + normalize + verify
   the span is present character-for-character), adversarially refute. Decline any edge
   no authoritative self-contained span defends — honest absence beats a fabricated
   citation. (No `trust consensus` authored-debt seed; an edge is grounded or omitted.)
2. **Write the `.adj` directly** — one `relate rel(subject, object)` clause per grounded
   edge, with `source "<verbatim span>"` + `locator "<url>"` + `trust authoritative`
   inline. A query is just another `.adj` that `import`s the library and asks
   `? rel(subject, $Var)` — the shape an LLM produces to recall a fact. (Earlier domains
   IEM/vitamin/anemia/endocrine/coag still use the legacy `_edge_ground.py` gate + JSON;
   they regenerate the same ADJ shape and can migrate to ADJ-only incrementally.)
3. **Board items** — add free-text + structured board questions; wire into
   `board/items.json` + `board/free_text_board.json` + `EDGE_FILES`.
4. **Score** — `board_eval` (native engine) + `board_offline` (offline, 0 online calls);
   defensibility must stay 100% (never fabricate), grounded-coverage climbs.
5. **Gate / test / security-review / PR / babysit → merge → loop.**

## Non-negotiables (carried from the framework)
- Nothing human-authored: facts enter the CAS only via spider → provenance → gate.
- One engine for every tactic; no per-question-type Python.
- Every answer is a citable, correctable proof; the system is decision *support*, never
  a replacement; honest abstention is a feature, not a failure.
- Decompose may use a LOCAL in-memory model only; the engine answers; **zero online
  calls at answer time** (the licensing-exam constraint).

## Coverage accounting (the watchable number)

The north-star metric: **% of the USMLE content outline covered by grounded, scored
domains.** Today: 12 recall domains + ID differential/management (a slice of Multisystem,
Endocrine, Hematology, Biochemistry, Nutrition, Microbiology, Pharmacology, Immunology,
Genetics, Rheumatology, Oncology, Histology — 97/98 board recall answers cite an
authoritative grounded edge). All four Tier-1 foundational-recall domains (microbiology,
pharmacology, immunology, genetics) are grounded & scored; RHEUM (autoantibodies), ONCO
(tumor markers), and HISTO (smear/histology buzzwords) are the first three Tier-2
organ-system pathology domains. Each merged domain PR moves this number;
this map is the denominator. The campaign is done when every Tier-1/2/3 cell above has a
grounded, scored domain and the board bank samples each.
