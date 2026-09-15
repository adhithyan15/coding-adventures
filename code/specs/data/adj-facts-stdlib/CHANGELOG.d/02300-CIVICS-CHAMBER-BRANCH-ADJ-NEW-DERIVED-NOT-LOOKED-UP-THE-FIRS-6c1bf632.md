- `civics/chamber-branch.adj` (new) — **DERIVED, not looked up**: the FIRST `rule` in the
  `civics/` domain, grounding which BRANCH of the U.S. federal government a named CHAMBER of
  Congress belongs to — `chamber_branch(chamber, branch)`, `senate` → `legislative`,
  `house_of_representatives` → `legislative`.

  The point of this library is that **the cited source never states the conclusion**. USA.gov's
  "Branches of the U.S. government" page does not write "the Senate is part of the legislative
  branch". It writes two separate facts one level apart: "The legislative branch is made up of
  Congress:" (the branch's member is Congress) and, nested beneath that stem, "The U.S. Senate"
  (the Senate is a chamber OF Congress). This rule derives the answer by composing the two
  already-shipped, already-grounded tables through the `congress` atom they both independently
  bind — `congress_chamber($Chamber, $Body), government_branch_member($Branch, $Body)` — rather
  than asserting a third row nobody could trace back to a sentence. Every answer therefore carries
  BOTH premises' citations in a three-step proof trail (rule + both facts): the provenance of the
  conclusion is the composition of the provenance of its premises, which is exactly the property
  that makes a derived answer auditable rather than merely plausible. Same shape
  `biology/abo-genotype-antigen.adj` established for heredity, applied to the civics graph.

  WHY THIS IS THE RIGHT WAY TO ANSWER IT — and why two earlier decisions that looked like extra
  work were not. `government-branch-member.adj` deliberately ABSTAINS on
  `senate`/`house_of_representatives`, because the source nests them under Congress rather than
  naming them as direct members of the branch; `congress-chamber.adj` was then split out as its own
  predicate specifically so that nesting would not be flattened away. Both of those decisions were
  made so this rule could exist. Deriving the fact here — instead of quietly adding
  `government_branch_member(legislative, senate)` back as a row — keeps the source's own two-level
  structure intact AND still answers the learner's question. The abstention was never a gap to be
  papered over; it was the seam the derivation joins on.

  Runs BACKWARD too (`? chamber_branch($C, legislative)` enumerates both chambers). ABSTENTION IS
  INHERITED, NOT RESTATED: the rule adds no facts of its own, so it abstains exactly where its
  premises do — `supreme_court` and `president` are named institutions of the other two branches,
  `congress_chamber` abstains on both, so the derivation finds no premise to join and abstains too
  rather than inventing a chamber-of relationship. New `chamber-branch.query.adj` and
  `facts_chamberbranch_e2e.rs` (4 tests: the derivation itself with a `rule` step in the trail,
  BOTH premise citations carried in the proof trail, reverse enumeration of a branch's chambers,
  and inherited abstention). New manifest objective `adj.civics.3to5.chamber_branch` — the first
  civics objective with competency `infer` rather than `recall`, and the first with TWO
  prerequisites (`congress_chamber` and `government_branch_member`), mirroring the two-hop
  derivation it performs.

