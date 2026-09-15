- `civics/veto-override.adj` (new) — a `table` recording whether Congress can override each
  kind of presidential veto: `veto_override(veto_type, override_status)`, `veto` →
  `congress_can_override_in_most_cases`, `pocket_veto` → `cannot_be_overridden_by_congress`. The
  SIXTH library in the `civics/` domain, from the same USA.gov "How laws are made" page
  `chamber-exclusive-power.adj` already cites, "How a bill becomes a law" section, curl-fetched and
  read byte-for-byte before writing. `trust authoritative`.

  THE HEDGE IS PART OF THE FACT. The source does NOT say Congress can always override an ordinary
  veto — it says "in most cases Congress can vote to override that veto". Collapsing that to a bare
  `yes` would state something the source deliberately does not, so the atom keeps the qualifier:
  `congress_can_override_in_most_cases`. There is an e2e test that asserts `"S":"yes"` NEVER appears
  in the output, so a future edit cannot quietly upgrade a hedge into a certainty. The pocket-veto
  row carries no such qualifier because its own sentence carries none ("it cannot be overridden by
  Congress"), and the resulting asymmetry between the two atoms is therefore a faithful reflection
  of the source rather than sloppy parallelism.

  A genuinely NEW axis, not a restatement. `checks-and-balances.adj` already ships
  `checks_and_balances(president, veto, legislation_created_by_congress)` — that row says the veto
  EXISTS as an act one branch takes against another. This table says something that row cannot:
  whether the act can be UNDONE, and by whom. The two compose rather than overlap — the check and
  its own counter-check — and there is a test importing both libraries to demonstrate it. Note
  `veto` appears as an atom in both files, but in different argument positions of DIFFERENT
  predicates: an ACTION there, a KIND of veto here. Distinct predicates, so there is no collision to
  disambiguate and a recall against one can never be confused with a recall against the other.

  Honest abstention on the CONDITIONS that produce a pocket veto ("if the president does not sign
  off on a bill and it remains unsigned when Congress is no longer in session, the bill will be
  vetoed by default") — stated by the same paragraph, but answering WHEN a pocket veto happens
  rather than whether it can be overridden, so it is a different axis and belongs in its own table;
  on `line_item_veto`, a real term in U.S. civics vocabulary that this source never names; and on
  the president's other two options the same paragraph states (approving and signing a bill into
  law), which are not vetoes at all and have no override status to record. New
  `veto-override.query.adj` and `facts_vetooverride_e2e.rs` (5 tests: the hedge survives and a bare
  `yes` is absent, the pocket-veto row with its sentence carried verbatim, backward recall from
  status to veto kind, composition with the `checks_and_balances` act, and honest abstention on both
  the unnamed kind and the different-axis condition). New manifest objective
  `adj.civics.3to5.veto_override` with `checks_and_balances` as prerequisite.

