# UTI organism identification (G4) — the first specialty expansion

MYCIN-2026's first disease beyond meningitis/bacteremia: **urinary-tract infection**. It
proves the expansion thesis — *a new specialty is new GROUNDED FACTS pointed through the
existing machinery, not new code.* The vocabulary, write gate, and rulebook mirror the
meningitis organism-id (`diagnosis/organisms/`) exactly; the gate even **reuses** the
meningitis gate's proportion parser, citation escaper, and verdict logic.

## Files

| File | Role |
|---|---|
| `uti-vocab.adj` | Closed vocabulary: the 7 uropathogens (E. coli, S. saprophyticus, Klebsiella, Proteus, Enterococcus, Pseudomonas, GBS) + urinalysis findings (nitrite, leukocyte esterase, urease/alkaline, complexity). |
| `uti_id_ground.py` | The adversarial write gate — consumes `grounding/uti-id-grounding.json` (spider output) and regenerates `uti-id.adj` so every prior + finding clause is byte-cited + gated. |
| `uti-id.adj` / `uti-id-manifest.json` | **Generated** — do not hand-edit; correct a fact by editing the grounding and re-running the gate. |
| `test_uti_id_ground.py` | Gate counts + `--check`, and an **engine differential** run (compose a case importing the rulebook, observe findings, rank the uropathogens). |

## Grounded knowledge (spider → byte-provenance → adversarial gate)

The UTI spider grounded **9 of 10** claims against primary sources (IDSA/ESCMID
uncomplicated-cystitis guideline, StatPearls, peer-reviewed UTI series):

- **Priors:** E. coli **0.75** (IDSA "75%–95%"), S. saprophyticus 0.10, Enterococcus 0.048,
  Klebsiella 0.035, Proteus 0.03, GBS 0.01, Pseudomonas 0.101 (the catheter-associated
  figure — see limitation below).
- **Findings:** urine nitrite → nitrate-reducing Enterobacterales (E. coli/Klebsiella/Proteus,
  `direction_only` → FLAG); urease / alkaline urine / struvite → **Proteus** (grounded). Urine
  leukocyte esterase grounds *pyuria* (infection present) — it doesn't discriminate the
  organism, so it lives in the "is this a UTI?" arm, not this "which uropathogen?" differential.

Manifest: **8 ACCEPT, 1 FLAG.**

## Honest limitations (follow-ups)

- The priors mix **uncomplicated** (community cystitis) and **complicated/catheter** contexts
  — Pseudomonas's 0.101 is the CAUTI figure. The `uti_complexity` finding is defined in the
  vocab but **not yet wired** as a prior-shifting host factor (a G4b host-factor batch, like
  the meningitis G2).
- The UTI grounding's cited **sources are not yet decomposed** into the CAS, so the system
  ledger shows the UTI citations as *pending* verification (a UTI `decompose-source` run is the
  recursion follow-up, exactly as G0 was for meningitis).
- No UTI **formulary** yet (nitrofurantoin / TMP-SMX / fosfomycin / fluoroquinolone coverage);
  the identified set is ready to feed one when it's grounded.
