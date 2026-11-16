# Hamurabi Conformance Corpus (hamurabi-v1)

Canonical deterministic test corpus for the Hamurabi mainframe resource allocation simulation (`GAME01`, `HA01`).

## Contents

- `cases.json`: 31 deterministic test cases comprising:
  - **Full 10-Year Games (4 cases)**: End-to-end multi-turn transcripts reaching each terminal rating (`Charlemagne`, `Acre`, `Nero`, `Disaster`).
  - **Single Turn State Dynamics (4 cases)**: Targeted verification of plague mortality, rat grain destruction, land purchases, and land sales.
  - **Starvation & Impeachment Boundaries (3 cases)**: Immediate 100% starvation impeachment, 46% borderline impeachment, and 45% non-impeachment threshold.
  - **All 8 HamurabiError Refusals (8 cases)**: Validating each canonical error condition in sequence:
    1. `GameAlreadyEnded`
    2. `InvalidOrderBounds`
    3. `InsufficientBushelsForLand`
    4. `InsufficientAcresToSell`
    5. `InsufficientBushelsToFeed`
    6. `InsufficientAcresToPlant`
    7. `InsufficientPopulationToPlant`
    8. `InsufficientGrainForSeed`
  - **Boundary & Capacity Limits (6 cases)**: Exact capacity allocations (selling all acres, feeding all bushels, exact labor limit, exact seed bushels, odd acreage ceiling calculation, max affordable land purchase).
  - **Overflow & Edge Refusals (6 cases)**: Bounds checking with `u32::MAX` inputs and exact off-by-one refusal boundaries.
