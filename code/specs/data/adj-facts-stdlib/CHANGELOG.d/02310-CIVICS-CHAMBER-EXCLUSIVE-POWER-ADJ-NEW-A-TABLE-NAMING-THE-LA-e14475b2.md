- `civics/chamber-exclusive-power.adj` (new) — a `table` naming the lawmaking power each
  chamber of Congress holds EXCLUSIVELY: `chamber_exclusive_power(chamber, power)`,
  `house_of_representatives` → `initiate_tax_and_revenue_legislation`, `senate` →
  `draft_legislation_on_presidential_nominations_and_treaties`. The FIFTH library in the `civics/`
  domain and the FIRST from a source page other than "Branches of the U.S. government" — that page
  is now fully decoded by `government-branch-member.adj`, `congress-chamber.adj`,
  `checks-and-balances.adj`, and the derived `chamber-branch.adj`, so this slice opens a new page
  and a new axis rather than restating a mined one.

  Sourced from USA.gov's "How laws are made" page, "How the House and Senate's lawmaking procedures
  are different" section, curl-fetched and read byte-for-byte before writing. Both rows are verbatim
  "Only the …" sentences — that phrasing is exactly what makes them EXCLUSIVE powers rather than
  powers a chamber merely happens to have — carried as one `source` plus one `cites`.
  `trust authoritative`, the same tier the four sibling civics libraries cite. (The candidate URL was
  fetch-verified from this machine BEFORE scoping, along with four other usa.gov civics pages, all
  200; whitehouse.gov, bensguide.gpo.gov, senate.gov and loc.gov all 403/404 here, so source
  reachability is checked first rather than assumed.)

  ATOM REUSE IS LOAD-BEARING, NOT COSMETIC. This table deliberately binds the same
  `house_of_representatives`/`senate` atoms `congress-chamber.adj` already established, so it
  composes with the shipped civics graph instead of forking a parallel vocabulary. Chained through
  the derived `chamber_branch` rule, a learner can now get from an exclusive power all the way to a
  branch — `chamber_exclusive_power(house_of_representatives, initiate_tax_and_revenue_legislation)`
  then `chamber_branch(house_of_representatives, legislative)` — and there is an e2e test that walks
  that chain end to end. A forked chamber vocabulary would have broken this silently. Note the source
  writes "the House" in both sentences while naming it "The U.S. House of Representatives" elsewhere;
  the longer atom is used because it is the one the sibling established.

  Honest abstention on `supreme_court` (not a chamber of Congress at all) and on the
  majority-vote/deliberation contrast — a real House/Senate difference the SAME section states, but
  it describes a PROCEDURE each chamber uses, not a kind of legislation only that chamber may
  originate, so it is a different axis and belongs in its own table rather than flattened in beside
  two "Only the …" powers. Also distinguished from `checks-and-balances.adj`: that library holds what
  one part of government can do TO another, so
  `checks_and_balances(congress, confirm_or_reject, presidential_nominees)` is a distinct fact from
  this table's senate row — confirming a nominee is a check on the executive, whereas drafting
  legislation RELATED TO nominations and treaties is a lawmaking power the Senate holds alone.

  NAMING NOTE recorded in the header to save a future reader a grep: `money/us-bills.adj` and
  `money/bill-back-vignette.adj` use the atom `bill` for CURRENCY. That is a homonym, not legislative
  coverage — a naive grep for "bill" will mislead. New `chamber-exclusive-power.query.adj` and
  `facts_chamberexclusivepower_e2e.rs` (4 tests: both powers with citation and both grounding
  sentences carried, backward recall from power to chamber, the cross-library composition chain
  through to a branch, and honest abstention on the non-chamber and the procedural contrast). New
  manifest objective `adj.civics.3to5.chamber_exclusive_power` with `congress_chamber` as
  prerequisite.

