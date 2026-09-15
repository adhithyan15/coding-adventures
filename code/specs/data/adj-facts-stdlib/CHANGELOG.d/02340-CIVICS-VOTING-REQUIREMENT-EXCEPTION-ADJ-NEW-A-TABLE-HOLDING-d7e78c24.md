- `civics/voting-requirement-exception.adj` (new) — a `table` holding the carve-out USA.gov
  states for each U.S. voting requirement: `voting_requirement_exception(requirement, exception)`,
  `us_citizenship` → `non_citizens_may_vote_in_some_local_elections_only`, `state_residency` →
  `experiencing_homelessness_still_meets_it`, `age_eighteen_by_election_day` →
  `some_states_allow_seventeen_year_olds_in_primaries`, `voter_registration_by_state_deadline` →
  `north_dakota_does_not_require_registration`. The EIGHTH library in the `civics/` domain and the
  FIRST from the voting pages, sourced from USA.gov's "Who can and cannot vote" page — curl-fetched
  and read byte-for-byte, with the URL reachability-checked from this machine before any scoping
  work. `trust authoritative`.

  WHY THE EXCEPTIONS AND NOT THE REQUIREMENTS. The four requirements are a flat bulleted list with
  no second column the source states, so a `requirement → description` table would have had to
  paraphrase each bullet into a description the page never separately gives — the same trap the
  how-laws-are-made idea-origin list sets, and a violation of the rule
  `bill-stage-successor.adj` established one slice earlier (match the shape of the SOURCE, not a
  familiar table shape). The EXCEPTIONS are different: the page attaches exactly one stated
  carve-out to each of the four bullets, so `requirement → exception` is a relation the source
  genuinely supplies on both sides, uniformly, with nothing invented.

  It is also the more useful half. An LLM asked "do you have to register to vote?" will confidently
  say yes; that North Dakota requires no voter registration at all is precisely the sort of detail
  that disappears into a confident summary. Recalling the carve-out WITH its citation is the
  behaviour this stdlib exists to make possible, so a dedicated e2e test asserts that specific row
  and its verbatim sentence rather than only checking the table loads.

  Honest abstention on the page's separate "Who cannot vote?" section — non-citizens including
  permanent legal residents, some people convicted of a felony, some people with a mental
  disability, and U.S. citizens residing in U.S. territories (who cannot vote for president in the
  general election). Those state DISQUALIFICATIONS, not exceptions to a requirement: "you do not
  qualify because of X" is the opposite claim from "you still qualify despite X", and folding them
  into one table would look like broader coverage while reversing the meaning of half the rows. They
  belong in their own table. Also abstains on the registration bullet's "In almost every state, you
  can register to vote before you turn 18…", which is about WHEN you may register rather than an
  exception to whether registration is required. New `voting-requirement-exception.query.adj` and
  `facts_votingrequirementexception_e2e.rs` (5 tests: the North Dakota carve-out with its verbatim
  sentence and citation, all four requirements carrying their stated exception, all four grounding
  sentences carried as one `source` plus three `cites`, backward recall from exception to
  requirement, and honest abstention on two disqualifications). New manifest objective
  `adj.civics.3to5.voting_requirement_exception`, with NO prerequisite — voting is a separate strand
  from the branches/Congress chain, and inventing an edge to it would misrepresent the dependency
  graph.

