---
category: QR / format-marker / file-format specifics
---

# QR format-info bit ordering is MSB-first across row 8 cols 0–5, LSB-first down col 8 rows 0–5

(copy 1); copy 2 mirrors it. Always verify with `zbarimg` or another standard decoder immediately after implementation — BCH check is ground truth.
