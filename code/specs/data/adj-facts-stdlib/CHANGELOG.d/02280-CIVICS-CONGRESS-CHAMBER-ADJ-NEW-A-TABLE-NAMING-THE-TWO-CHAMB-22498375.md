- `civics/congress-chamber.adj` (new) — a `table` naming the two chambers the U.S.
  Congress is divided into: `congress_chamber(chamber, parent)`, `senate` → `congress`,
  `house_of_representatives` → `congress`. The SECOND library in the `civics/` domain, and it
  ships expressly to CLOSE an abstention its own sibling deliberately opened:
  `government-branch-member.adj` abstains on `senate`/`house_of_representatives` because USA.gov's
  "Branches of the U.S. government" page nests them UNDER Congress in a colon-introduced sub-list
  rather than naming them as direct members of the branch, and that file's header names THIS table
  as their intended home. Sourced from the same page, whose raw markup was curl-fetched and read
  byte-for-byte before writing: the stem is a single `<p>` reading "The legislative branch is made
  up of Congress:" (the linked anchor text inlined) and the two chambers are its
  immediately-following `<ul>`'s only two `<li>` items, with no third — so the two-chamber domain
  is covered with no gaps. `trust authoritative`, the same tier and the same page
  `government-branch-member.adj` already cites.

  Kept as a SEPARATE predicate rather than two more rows on `government_branch_member`, because the
  sub-list states a DIFFERENT relation: that table answers "who makes up this BRANCH", this one
  answers "what are the chambers of this BODY". Flattening the chambers into branch members would
  have silently discarded one level of the nesting the source itself states. Keeping them apart
  means the two COMPOSE instead — `congress_chamber(senate, congress)` and
  `government_branch_member(legislative, congress)` are the two premises that ground "the Senate is
  part of the legislative branch" as an auditable two-hop derivation whose provenance is the
  composition of its premises, rather than a third asserted row nobody can trace back to a
  sentence. (The derived `rule` itself is a natural next slice and is deliberately NOT part of this
  one.) The relation is MULTI-VALUED on `parent`, which is the direction actually asked: "what are
  the two chambers of Congress?" is one query returning two solutions.

  Honest abstention on `supreme_court` and `president` — genuinely named institutions the SAME
  source names, but of the judicial and executive branches, not chambers of Congress; both are
  already correctly tabled in `government-branch-member.adj` against their own branches — and on
  `special agencies and offices that provide support services to Congress`, the open-ended category
  phrase the same paragraph attaches to the legislative branch, which names no specific body and is
  not part of the chamber sub-list at all. New `congress-chamber.query.adj` and
  `facts_congresschamber_e2e.rs` (4 tests: forward recall + citation check, multi-valued
  two-chamber enumeration, a COMPOSITION test importing both civics libraries together and
  asserting they meet at the same `congress` atom, and honest abstention on both other-branch
  institutions). New manifest objective `adj.civics.3to5.congress_chamber`, declaring
  `adj.civics.3to5.government_branch_member` as a prerequisite — the first civics objective with a
  prerequisite edge.

