- `civics/bill-stage-successor.adj` (new) — a `table` recording which stage a bill moves to
  next on its way through Congress: `bill_stage_successor(stage, next_stage)`, seven rows chaining
  `introduced` → `committee_review` → `first_chamber_vote` → `second_chamber_process` →
  `reconcile_differences` → `vote_on_same_version` → `presented_to_president` →
  `president_considers`. The SEVENTH library in the `civics/` domain, from the same USA.gov "How
  laws are made" page, curl-fetched and read byte-for-byte. `trust authoritative`.

  THE FIRST ORDERED SEQUENCE IN THIS STDLIB EXPRESSED AS A SUCCESSOR RELATION RATHER THAN AN
  ORDINAL POSITION — and the reason is the point of the slice. The obvious move was a fifth ordinal
  table, `bill_stage_step(stage, step_number)` 1 through 7, matching `astronomy/moon-phases.adj`'s
  `moon_phase_order`, `astronomy/planets.adj`'s `planet_order`,
  `biology/mitosis-phase-order.adj`, and `earth-science/sedimentary-rock-formation-step.adj`. That
  would have been wrong, and the failure would have been invisible.

  Those four all decode sources that state POSITIONS: a numbered or explicitly-counted sequence.
  This source does not. It states TRANSITIONS, in continuous narrative prose, each marked by its own
  connective — "Once a bill is introduced, it is assigned to a committee…", "The bill is THEN put
  before that chamber…", "IF the bill passes one body of Congress, IT GOES TO the other body…".
  Every sentence says what comes AFTER what. None says what number anything is. Assigning absolute
  ordinals would have required inventing an answer to a question the page never addresses: where
  does the count start — is `introduced` step 1, or is the idea-origin list before it? Pick
  differently and EVERY number changes, yet no test would fail and no citation would look wrong; the
  table would simply assert positions its own source never states. A successor relation records
  exactly the adjacency the prose does state, nothing more, and is incidentally stable: a future
  added stage changes two rows here, where an ordinal table would renumber wholesale. GENERAL RULE
  worth carrying forward: match the shape of the SOURCE, not the shape of the sibling tables. An
  established shape is a convenience, never a reason to assert something the source does not say.

  THE CHAIN STOPS WHERE THE SOURCE STOPS BEING LINEAR. `president_considers` has no successor row,
  deliberately: from there the prose BRANCHES — the president "can approve the bill and sign it into
  law", or can "refuse to approve a bill" (a veto), or may let it go unsigned into a pocket veto. A
  successor relation cannot honestly represent a branch; it would have to pick one outcome
  arbitrarily or return three "next stages" as if all happened. There is an e2e test asserting no
  branch outcome is ever named as THE successor. What happens on each branch is already held by
  `checks-and-balances.adj` (the veto as an act) and `veto-override.adj` (whether that act can be
  undone).

  Honest abstention also on the idea-origin list the same page states ("The idea for a bill can come
  from a: Sitting member of the U.S. Senate or House of Representatives / Proposal during a
  congressional candidate's election campaign / Petition by people or citizen groups…"), which
  answers WHERE a bill starts rather than what stage follows what — a different axis and its own
  future table. All seven transition sentences are carried as one `source` plus six `cites`, so each
  row stays auditable back to the sentence whose connective states that particular hop. New
  `bill-stage-successor.query.adj` and `facts_billstagesuccessor_e2e.rs` (4 tests: walking the whole
  linear chain hop by hop with no index arithmetic, all seven transition sentences carried, backward
  recall to the prerequisite stage, and the branch-point abstention including negative assertions
  that no outcome is named as the successor). New manifest objective
  `adj.civics.3to5.bill_stage_successor` with `congress_chamber` as prerequisite.

