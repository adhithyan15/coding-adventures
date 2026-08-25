### Added — the B1–C2 spine, so the ladder has rungs above A2 (HL09 §3.1)

`spine.json` stopped at A2. The level gate handled that correctly — it refused B1–C2
on the grounds that "no node is unrealized" is not "every node is realized" — but the
effect was that **no amount of content could ever certify above A2**, because there
was nothing to certify against.

**17 nodes**: B1 ×5, B2 ×4, C1 ×4, C2 ×4, each a CEFR can-do statement with
prerequisites that resolve and never point up a level (both asserted).

- **B1** narrate in order, give reasons, cope while travelling, describe experience,
  express a real condition.
- **B2** argue a view, report what others said, read extended prose, discuss the
  abstract.
- **C1** infer what is implied, structure a long text, shift register — and
  **follow regional variation**, which had no home in the spine at all.
- **C2** synthesise several sources, express fine shades, read literary and older
  text, and **read the cultural weight of a phrase**, not only its meaning.

**68 concepts registered canonically**, each owned by exactly one node. Five were
first written as `VERB-EXPLAIN`, `VERB-HOPE` and so on, which silently pushed
`coreVerbCount` from 40 to 45 — joining a baseline that HL-C46/47/49 owns. They are
named by discourse function instead (`EXPLANATION-GIVE`, `AMBITION-EXPRESS`).

**All 22 tracks declare all 17 as gaps** — `segments: []` with every concept in
`omits`. A ledger that stays silent about a node reads the same as one that has
nothing to say; this one says exactly what is missing.

Nothing is realized, and the gate still reports every track at pre-A1 with its real
blockers. Authoring a rung does not climb it — the test that used to assert "B1 is not
authored" now asserts the stronger thing: B1 exists, and no track attains it.

