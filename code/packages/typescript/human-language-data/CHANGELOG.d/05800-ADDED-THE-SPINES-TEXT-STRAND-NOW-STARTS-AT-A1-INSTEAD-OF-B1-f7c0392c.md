### Added — the spine's TEXT strand now starts at A1 instead of B1

- `SPINE-READ-SIGNS-AND-NOTICES` (TEXT, A1) is the first text node below B1.
  Before it, the spine said working with text begins after A2 — which is why 22
  tracks had no reading lesson and why an A1 reading lesson had nowhere to hang.
  The node was the missing hook, not the missing effort.
- `reading` joins `REALIZING_TYPES`. It was absent, so a reading lesson could
  never realize a spine concept and a node whose concept is CONNECTED-READING
  was unsatisfiable by construction — the same shape as the `comprehension`
  block type that no heading produced. The exempt types name a recap
  (`practice`, `review`) or an orthographic nuance (`writing`); connected
  reading is neither.
- `CONNECTED-READING` is a canonical concept, deliberately unlike
  `SPINE-DESCRIBE-QUALITIES`, which was minted with an empty `concepts` list so
  it would ask nothing of the other 22 tracks. Reading is an ability every track
  owes its learners, so the claim on all 23 is the honest one — and the 22
  tracks that now carry `omits: [CONNECTED-READING]` state a real gap in their
  own ledger rather than staying silent about it.
- Spanish's three passages moved onto the new node from
  `SPINE-DEFINITE-REFERENCE`. Not cosmetic: a new A1 node is a requirement for
  every track's A1 attainment, and Spanish — the only track that had reached A1 —
  dropped to pre-A1 until its reading hung where reading belongs.
