- `civics/government-branch-member.adj` (new) — a `table` naming which officer or institution
  makes up each branch of the U.S. federal government:
  `government_branch_member(branch, member)`, `legislative` → `congress`, `judicial` →
  `supreme_court`, `executive` → `president` / `vice_president` / `president_cabinet`. This
  opens a BRAND-NEW `civics/` domain — the first entry against the "Social knowledge → Civics"
  Major Gap that ADJ-STDLIB-COVERAGE.md §5.1 has carried unaddressed since the coverage baseline
  was measured, and the first non-`money`/`transportation` entry in the social-knowledge area at
  all. A full-tree grep for `legislative`, `judicial`, `executive_branch`, `branch_of_government`,
  `congress`, and `supreme_court` across every shipped `.adj` returned nothing beforehand, so
  there is no existing table for this to duplicate. Sourced from USA.gov's "Branches of the U.S.
  government" page (curl-fetched and read byte-for-byte before writing, not an AI-summarized
  WebFetch); `trust authoritative` — USA.gov is the General Services Administration's official
  guide to government information, a first-party .gov publisher of exactly the fact recalled, the
  same tier this stdlib reserves for NASA/NOAA/USGS/NPS/NIST. The relation is deliberately
  MULTI-VALUED on `branch` (one cited sentence names all three executive members at once, so the
  executive query yields three solutions), and runs BACKWARD as a genuine reverse recall —
  "which branch is the Supreme Court part of?" being the direction an elementary civics learner
  is actually quizzed in. ROW-INCLUSION RULE, applied uniformly to all three branches: a row
  ships only for a NAMED officer or NAMED institution the page states a branch is made up of or
  includes. Honest abstention on `senate` and `house_of_representatives` — genuinely named
  institutions the SAME page names, but nested UNDER Congress in a colon-introduced sub-list,
  making them CHAMBERS OF CONGRESS rather than direct members of the branch; flattening them into
  rows here would silently discard the nesting the source states, so they are left to a future
  `congress_chamber` table — and on every open-ended category phrase (`other federal courts`,
  `special agencies and offices that provide support services to Congress`, `Executive
  departments`, `Independent agencies`, `Other boards, commissions, and committees`), which name
  no specific body to bind a stable atom to. New `government-branch-member.query.adj` and
  `facts_governmentbranchmember_e2e.rs` (4 tests: forward recall + citation check, reverse
  recall, multi-valued executive enumeration, honest abstention on both a chamber and a category
  phrase). New manifest coverage root `c3.socialstudies` (the C3 Framework for Social Studies
  State Standards — no social-studies root existed, so civics objectives had nothing to map to;
  declared at the same `status: "declared"`, `cas_hash: null` tier as `ngss`, with the official
  NCSS-hosted PDF as locator, whose identity was verified by fetching it and reading its title
  page, since socialstudies.org's HTML pages are Cloudflare-blocked from this box) and new
  objective `adj.civics.3to5.government_branch_member`, both added via a surgical text edit
  (JSON validated before write, parsed back out to confirm, `git diff --stat` showing +24 lines
  and no reformatting).

