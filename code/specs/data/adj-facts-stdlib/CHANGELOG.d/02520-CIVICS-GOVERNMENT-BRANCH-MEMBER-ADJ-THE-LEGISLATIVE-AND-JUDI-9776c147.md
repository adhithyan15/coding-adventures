- **`civics/government-branch-member.adj`: the legislative and judicial rows were cited by a sentence
  about the EXECUTIVE branch.** Fixes issue #13928. The table has five rows spanning three branches but
  carried a single table-level `source` envelope — the executive sentence — and no `cites`. So every
  answer was evidenced by that one sentence regardless of which row matched:

  ```
  ? government_branch_member(legislative, $M)
    -> congress, cited by "The president, the vice president, and the president's cabinet
       are the members of the executive branch."
  ```

  An answer about the LEGISLATIVE branch, evidenced by a sentence about the EXECUTIVE one. That is a
  step worse than the known table-level attribution limit (#13893), where sibling rows at least share a
  citation that covers them; here the citation was about a different subject than the answer. It
  propagated: `civics/chamber-branch.adj` is a rule deriving from this table, so its answer
  "senate -> legislative" carried the executive sentence as supporting evidence.

  *** THE RIGHT EVIDENCE WAS NEVER MISSING — IT WAS IN A COMMENT. *** The library's own literate header
  documents a per-row evidence table naming THREE distinct sentences, one per branch. Only the
  executive one was ever written into the `table` block. The other two existed solely as prose. This is
  not carelessness at research time; it is that A COMMENT IS NOT MACHINE-CHECKABLE, so nothing could
  report that two thirds of the identified evidence never reached the data. Same root cause as #13918.

  Both missing sentences are now `cites` clauses, RE-EXTRACTED from the locator rather than copied from
  the header — an inherited header quote is exactly the practice #13918 is about. Both are verbatim,
  split in the page only by inline `<a>` markup. The header's own rendering of the legislative sentence
  drops its terminal COLON; the page has "made up of Congress:" because the two chambers are listed
  after it, and the `cites` clause carries the colon.

  WHAT THIS BUYS AND WHAT IT DOES NOT. Every row's supporting sentence is now PRESENT on every answer.
  It is still not ATTRIBUTED per-answer — provenance here is table-level (#13893), so the executive
  answers also carry the legislative and judicial sentences as corroborations. The gain is the
  difference between evidence that is INCOMPLETE and evidence that is WRONG.

  *** THE TEST THAT LET THIS SHIP, AND WHY IT COULD NOT FAIL. *** The e2e test asserted
  `out.contains("\"M\":\"congress\"")` and, SEPARATELY, that the output contained
  `usa.gov/branches-of-government` and `"trust":"authoritative"`. Two independent substring scans over
  one JSON blob cannot tell WHICH answer a citation belongs to, so both passed — in CI, continuously —
  while a legislative answer carried executive-branch evidence. Replaced with a JOINT BINDING that pins
  the answer and its complete evidence as one contiguous span, so the two cannot drift apart.

  This is the fragment-pin failure one level out. A fragment pin could not see the ENDS of a citation;
  this could not see WHICH ANSWER a citation attached to. Both look like coverage, and neither can fail
  when the thing it describes is wrong.

  Falsified by mutation, four ways: drop the legislative `cites`, drop the judicial `cites`, truncate
  the judicial sentence, truncate the primary `source`. All four fail the suite. 533 test binaries /
  1592 tests green, clippy -D warnings clean.
