### Changed — the filmstrip ledger includes derived letter-lesson candidates (HL-C443)

`tests/filmstrip-ledger.test.ts` now builds the ledger from the declared
`script-filmstrip` targets plus human-language-data's `filmstripCandidates`.
Candidates are kept only when `ductusFor` finds a cited ductus, so a derived
letter never outruns the stroke data. A declared target without one still
throws. The lesson load is cached per test file, because building the ledger
twice to check determinism reloaded the whole curriculum each time. The ledger
grows from 3 entries to 21 (19 Tamil letters, Hindi आ, Persian چ).
