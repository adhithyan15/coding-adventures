### Added — HL-C10: the shared spine reaches above A1

- Add an **A2 tranche** of five spine nodes — `SPINE-SAY-WHAT-I-DO`,
  `SPINE-NEGATE-AND-ASK`, `SPINE-SAY-WHAT-I-WANT`, `SPINE-TALK-ABOUT-PAST`,
  `SPINE-TALK-ABOUT-FUTURE` — and the seven canonical concepts they own
  (`VERB-INFINITIVE`, `VERB-PRESENT-HABITUAL`, `VERB-NEGATE`, `QUESTION-POLAR`,
  `VERB-WANT`, `VERB-PAST`, `VERB-FUTURE`).
- **This unblocks the entire Easy-to-Advanced grammar arc, and nothing else could.**
  Schema v2 requires a canonical `spine_node`. Every one of the previous eleven nodes was
  an A1 social function — greeting, taking leave, counting to five — with nothing covering
  verbs or tense, so a lesson teaching a present tense had no node it could legally
  declare. The arc was unauthorable in v2 for all 22 tracks. It was found the hard way,
  by trying to migrate a Hindi verb lesson and discovering its chapter belongs to no node.
- All 22 realization ledgers declare where they stand on each new node. An unrealized node
  is recorded as `segments: []` **with `omits` naming every concept it is not yet
  delivering** — the validator requires this, and rightly: "we have not built this yet" is
  a recorded position, so the debt stays countable instead of being an absent key nobody
  can see. Today that is all 22 tracks on all five nodes; those numbers are the burn-down.
- The taxonomy grows 46 → 53 concepts. Each concept is owned by exactly one node, which
  the validator enforces, so a later tranche cannot quietly re-file a concept it wants.

