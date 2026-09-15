- `civics/elector-allocation-method.adj` (new) — a `table` recording how a jurisdiction assigns
  its presidential electors: `elector_allocation_method(jurisdiction, method)`,
  `forty_eight_states_and_dc` → `winner_take_all`, `maine` → `proportional`, `nebraska` →
  `proportional`. The TENTH library in the `civics/` domain, from the same USA.gov "Electoral
  College" page, curl-fetched and read byte-for-byte. `trust authoritative`.

  THIS FILE CLOSES A DOCUMENTED ABSTENTION. `electoral-college-count.adj` holds
  `winner_take_all_states -> 48` and deliberately declined the Maine/Nebraska fact, its header
  recording that a proportional system is "a METHOD, not a count … Different axis, its own future
  table". This is that table. The two compose rather than overlap — the count table says HOW MANY
  jurisdictions are winner-take-all, this one says WHICH METHOD a named jurisdiction uses — and an
  e2e test imports both to demonstrate it. Both columns are source-stated, across two consecutive
  sentences.

  WHY THE GROUP ROW IS KEPT. It would have been simpler to table only the two exception states, but
  shipping an exception without the rule it is an exception TO would leave a learner able to recall
  that Maine is proportional without being able to recall that almost nowhere else is. The group
  atom is `forty_eight_states_and_dc` because that is exactly how the source delimits the group ("In
  48 states and Washington, D.C."); a vaguer `most_states` would be a paraphrase the page never
  uses, and a bare `other_states` would hide that the source states a specific count. That count
  also travels, with its own citation, in `electoral-college-count.adj`'s `winner_take_all_states`
  row, so if a future apportionment changes the number both rows are re-derived from the same
  re-fetched sentence rather than one silently disagreeing with the other.

  Honest abstention on `california` — and on every other individual state. The source describes the
  other 48 only as a GROUP, so binding a specific state name would require deciding it is one of
  them, an inference the page does not license for ANY particular state. This is the abstention
  worth reading: a model asked "how does California award its electors?" answers confidently, and
  this table declines, which is the entire point of a grounded recall library. Also abstains on the
  proportional MECHANISM — the page names the system without explaining HOW it apportions (by
  congressional district, statewide, or otherwise), so the well-known detail must not be filled in
  from outside the source. And the faithless-elector penalties the same page lists (fined,
  disqualified, replaced by a substitute elector, prosecuted by their state) are consequences for an
  individual elector's vote rather than a jurisdiction's allocation method, and the list states no
  second axis uniformly — only one of the four names who imposes it — so it is not tabled here or
  anywhere. New `elector-allocation-method.query.adj` and `facts_electorallocationmethod_e2e.rs`
  (5 tests: Maine's method with its citation, reverse recall returning BOTH exception states, the
  group row preserving the rule the exception is an exception to, cross-library composition with
  `electoral_college_count`, and honest abstention on an unplaced state and on the unexplained
  mechanism). New manifest objective `adj.civics.3to5.elector_allocation_method` with no
  prerequisite — elections are their own strand.

