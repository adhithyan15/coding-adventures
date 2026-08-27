## 0.7.1 — each script's at-a-glance identification signature

- Every script now carries a **`signature`** — the one visual feature that gives
  it away at a glance (Devanagari's head-line; Gujarati being Devanagari with
  that line erased; Arabic's joined right-to-left ribbon vs Hebrew's blocky
  separate letters; Cyrillic's Я/Ж/Д tells; Chinese's dense square blocks).
- Added to all seven script data files (`data/scripts/*.json`) and to the
  `ScriptData` type; a test asserts every script ships a non-empty signature.
- Each signature was written **against the rendered font**, not from memory —
  the same verify-by-looking discipline the stroke data uses. This is the data
  backbone for a future "spot the script" identification mode.

