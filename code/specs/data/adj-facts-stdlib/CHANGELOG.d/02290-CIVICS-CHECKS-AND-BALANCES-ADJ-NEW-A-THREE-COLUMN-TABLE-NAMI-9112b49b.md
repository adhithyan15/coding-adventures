- `civics/checks-and-balances.adj` (new) — a THREE-column `table` naming which act each part
  of the U.S. federal government can take against the others:
  `checks_and_balances(actor, action, object)`, with five rows —
  `president`/`veto`/`legislation_created_by_congress`,
  `president`/`nominate`/`federal_agency_heads_and_high_court_appointees`,
  `congress`/`confirm_or_reject`/`presidential_nominees`,
  `congress`/`remove_from_office`/`president`, and
  `supreme_court`/`overturn`/`unconstitutional_laws`. The THIRD library in the `civics/` domain
  and the first here that is genuinely RELATIONAL rather than a lookup: rows are not "X is a kind
  of Y" but "actor A can do action B to object C", so EVERY column can be the bound one and a
  single table answers three different question shapes — "what can the president do to the other
  branches?" (bind the actor), "who can veto?" (bind the action), and "who acts on the president?"
  (bind the object, a question neither sibling civics library can answer). Three columns is the
  same arity `physics/energy-conversion-example.adj` already ships.

  Each row is a subject-verb-object decomposition of exactly one `<li>` in the page's "How each
  branch of government provides checks and balances" list — nothing merged across bullets, nothing
  the source states as one act split apart — and because the five rows come from five different
  sentences, each row's own grounding sentence is carried verbatim in the `source`/`cites` tail so
  every row stays auditable back to one sentence. Raw markup curl-fetched and read byte-for-byte
  before writing: an `<h2>`, a lead-in `<p>`, a second `<p>` reading "Each branch of government can
  change acts of the other branches:", and a `<ul>` of exactly three `<li>` items stating the five
  acts. `trust authoritative`, the same tier and the SAME page the two sibling civics libraries
  already cite.

  DESIGN NOTE — the third column is the OBJECT ACTED UPON, not the BRANCH acted upon. The obvious
  design for a checks-and-balances table is branch → action → branch, and it is wrong for this
  source. Three of the five sentences do name another part of government as their object, but the
  Supreme Court's does not: it names LAWS. A branch-typed target would force either abstaining on
  the Supreme Court row — leaving the JUDICIAL branch unrepresented in a table whose whole subject
  is the balance BETWEEN the three branches — or inferring "unconstitutional laws" → "Congress",
  which the sentence never states (it does not say whose laws, and courts also review state law and
  executive action). Object-typing keeps all three branches represented with every row
  verbatim-grounded and nothing inferred; the branch-to-branch graph remains available by composing
  with `government-branch-member.adj`, the same composition-over-assertion principle
  `congress-chamber.adj` was split out for.

  Honest abstention on "These justices are nominated by the president and confirmed by the Senate"
  (a real sentence in the same `<li>`, but a PASSIVE restatement of two checks this table already
  holds in the active voice — tabling it again would double-count them under swapped subjects), on
  the section's lead-in "The ability of each branch to respond to the actions of the other branches
  is the system of checks and balances" (a definition of the system, not an act by a branch, with
  no actor/action/object to decompose), and on every power in the page's separate per-branch "key
  roles" lists — drafting proposed laws, surveying the budget, declaring war — which describe what
  a branch does in its OWN right rather than what it does to ANOTHER branch. New
  `checks-and-balances.query.adj` and `facts_checksandbalances_e2e.rs` (5 tests: both presidential
  checks with citation, all five grounding sentences carried as source + corroborations, backward
  recall on BOTH the action and the object columns, all-three-branches-represented — which is
  precisely what the object-typed third column buys — and honest abstention on both the passive
  restatement and an own-key-role power). New manifest objective
  `adj.civics.3to5.checks_and_balances` with `adj.civics.3to5.government_branch_member` as a
  prerequisite.

