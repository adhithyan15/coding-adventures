## Unreleased — COBOL INSPECT BEFORE/AFTER region proofs (VM-047c)

Add five seven-backend cells promoting the already-implemented single-region
`INSPECT ... BEFORE`/`AFTER` window to the unified matrix: TALLYING FOR ALL
narrowed BEFORE a delimiter, TALLYING narrowed AFTER a delimiter, REPLACING
ALL narrowed BEFORE a delimiter, REPLACING narrowed AFTER a delimiter, and
BEFORE/AFTER used together across the independently-regioned TALLYING and
REPLACING halves of one combined `INSPECT` statement. Each BEFORE/AFTER pair
pins the ISO not-found asymmetry: an absent BEFORE delimiter covers the WHOLE
source while an absent AFTER delimiter covers an EMPTY region.

