- **#13934 follow-up: a cell quoted with a space the page does not have, and 4o's unsupported
  proportion.** Not an installment — the six actionable candidates closed at 4o. Two repairs that
  turn on no held question, plus the pins that make the first one load-bearing.

  ### The NIST day cell

  `metrology/time-units.adj` and `metrology/time-unit-composition.adj` both quote NIST SP 811's
  Table 6. Measured against the fetched page (108,352 bytes, sha256 `75fa1875…`), with a positive
  control (`1 h = 60 min = 3600 s` → **1** occurrence) and a one-character negative
  (`1 h = 60 min = 3600 x` → **0**) run before any count below was believed:

  | quoted as | occurrences on the page |
  | --- | --- |
  | `1 d = 24 h = 86 400 s` — as shipped | **0** |
  | `1 d = 24 h = 86 400s` — as the page writes it | **1** |

  The page writes the day cell with **no space before the `s`**. **4** occurrences corrected — 1 in
  `time-units.adj`, 3 in `time-unit-composition.adj`. All three cells these two files quote
  (`1 min = 60 s`, `1 h = 60 min = 3600 s`, `1 d = 24 h = 86 400s`) now occur exactly **once** each
  on the page.

  **What is NOT repaired, and why.** Both `source` fields still occur **zero** times, because each
  welds two or three cells with a ` | ` that occurs on the page **only as page chrome**: four
  times in the raw HTML — a `citation_title` meta, the `<title>`, and two footer link separators —
  which is three in the rendered text. **Never between two cells of Table 6.** That separator
  is #13934's held question — *is a table row a verbatim span?* — which is the owner's to answer,
  not something to settle as a side effect of a spacing repair. Both files now say so in their own
  headers rather than leaving it to be discovered. `time-units.adj` also called its `source` a
  "span"; it is a field joining three cells, and the word is corrected. Its sibling said the
  stronger thing — that it "reproduces, **byte-for-byte**, the SAME NIST … **span**" — and the
  review caught that the first pass corrected one file and left the other; it now says *cells*,
  each byte-exact, and points at the paragraph that gives the zero count.

  An older entry in this file — the one that introduced `time-unit-composition.adj` — quotes the
  same cell with the same extra space and calls it verbatim. It is left standing as the record of
  what shipped then; this entry is the correction.

  ### The pins that make the repair load-bearing

  Both e2e tests pinned the citation as a **hostname plus a trust tier**, which cannot see the
  `source` field at all: neither test would have reddened for the defect above, and neither would
  redden if the quoted cells were altered or replaced wholesale. Both are now pinned to the
  **whole serialised citation object**, closing on the object's own `}` after `"corroborations":[]`
  — one assertion bounding source text, locator, trust tier and corroboration set, bounded by the
  next delimiter and never by a character count. Each test also gains a negative naming the
  specific defect (`!out.contains("86 400 s")`), placed **after** the object pin so that a
  restoration reddens the object pin rather than leaving it a survivor.

  That pin alone catches five things the old one could not see or could not fail on: the repaired
  defect restored; the minute cell dropped from the quoted source; a path segment appended to the
  locator (`contains("nist.gov")` is blind to it); a corroboration added under an unrelated
  locator; and the trust tier downgraded, the one case the old pin did catch.

  ### Presence is not exclusivity — found by the security review, demonstrated not described

  The object pin above proves the NIST citation **is** in the output. It does not prove it is the
  **only** one. The review appended a second `table` plus a `rule` whose head aliases the shipped
  relation, and the library emitted a fabricated `hour → 1 s` answer citing `https://evil.example/x`
  at `trust authoritative` — as a **sibling answer**, with its own `citations` array — while
  **every assertion in the test stayed green**. Two adjacent language-level defences do not cover
  it: a duplicate `table` name is a `Lower(DuplicateTable)` error, and an extra `cites` fills
  `corroborations` and reddens the pin, but a new relation plus an aliasing `rule` is accepted.

  Answered with three **equalities** per test: every `"source":"`, every `"locator":"` and every
  `"trust":"` the CLI emits must be the shipped value. Equalities rather than thresholds because
  that is what the real output measures — in both tests each key occurs only with its expected
  value (6/6/6 and 4/4/4).

  **Round 2 broke that first draft, and the way it broke is the point.** The right-hand needles
  counted the bare *value* — the URL, the span — not the key-and-value pair. So a fabricated
  clause that embedded the NIST URL and the shipped span *inside its own `source` string* moved
  both sides together, and everything stayed green while the output carried a fabricated
  `hour → 1 s` citing `evil.example` at `trust authoritative`. A free substring search is not an
  exclusivity check. Every needle is now key-anchored (`"locator":"<url>"`), the way the `trust`
  one already was.

  **And three equalities do not close what "closed" would imply.** An injected clause re-using the
  genuine source, locator and trust *verbatim* keeps all three balanced by construction, and the
  fabrication then wears the real NIST citation. What the equalities close is an answer citing a
  **second** source, locator or trust tier. What closes a fabricated **binding** under the genuine
  citation is naming the answer set, so each test also pins **how many facts answer and which** —
  3 for `time-units`, 2 for `time-unit-composition`.

  **Round 3 found that the answer-set pin only defends the rows the queries reach.** Two of the
  five rows across the two files were reached by no query, and both sat *inside* the table
  carrying the NIST citation: `row (minute, 60)` → `row (minute, 7)` and `row (day, hour, 24)` →
  `row (day, hour, 99)` both shipped **green**, serving a 7-second minute and a 99-hour day under
  `trust authoritative`.

  The first answer to that was to bind each row with its own query. **Round 4 showed why that is
  the wrong shape of answer, by deleting one space.** Alongside the per-row queries, round 3 had
  added a file guard that counted declarations as *line prefixes* — `l.trim_start().starts_with("row ")`.
  The ADJ lexer skips whitespace entirely, so `row(week, 604800)` is a live row that no prefix
  count can see: it shipped **green**, serving the very `week` this file's own header records NIST
  Table 6 as not stating. So did a whole second table under `https://evil.example/x`, laid out so
  no trimmed line began with a guarded keyword. **A lexical guard over a language whose lexer
  ignores layout is not a guard.**

  The semantic answer is the one that holds. Each test now asks a fully **unbound** query —
  `time_unit_seconds($U, $S)` — so the engine enumerates every row the library can serve, and the
  answer set pinned above is *the whole table*. A row can hide from a lexical count; it cannot hide
  from an enumeration, because it either answers or it does not exist.

  **The lexical guard was rebuilt to tokenise properly, and round 5 broke it four more ways.** A
  `rule` hidden behind a number (`let x = 86400rule { … }` lexes as `NUMBER`, `IDENT`, since ADJ's
  NUMBER and IDENT are different tokens and may abut) — and that rule's conclusion comes back with
  the **table's own citation** in its `citations` array, so a fabricated
  `safe_daily_dose_mg(aspirin, 5000)` answered carrying NIST at `trust authoritative`. A `relate`
  statement, which needs none of the guarded keywords at all. A `columns` clause permuted with the
  original hidden in a `%` comment. A forged `quote` pin — the strongest provenance claim ADJ has,
  text at a byte offset in a page of a given SHA-256 — naming a span that occurs **zero** times on
  the cited page, accepted with exit 0.

  So **the keyword guard is retired rather than patched a third time.** It was broken in both
  rounds it existed; it is a re-implementation of the lexer inside an assertion; and `statement`
  has **27** alternatives against a hand-written blocklist of five. Worse than the gap was the
  claim: while the guard stood, this entry said the file was bounded — a false premise offered in
  support of a true conclusion, which is the failure this library exists to prevent.

  What stays is the one lexical check that is bounded and checkable without a lexer: the
  **`columns` clause**, compared as an exact list after `%` comments are stripped. It earns its
  place — rows are positional, so a permutation changes no recall output at all, while
  `lookup … give <col>` resolves the names by position and starts answering `hour` where it
  answered `day`, still carrying the genuine citation.

  **What is NOT closed, stated rather than papered over.** An injected `relate`, a `quote` pin, or
  a second table whose `columns` sits on its brace line are all invisible to these tests. That is a
  property of what an `.adj` library is *permitted* to contain — a job for a CI lint that lexes
  with the real grammar, not for an e2e test of two metrology files — and it is filed as **#14822**
  with all four bypasses reproduced. Nothing shipped is wrong today: neither metrology file
  contains a `rule`, a `relate` or a `quote` pin.

  **Ten mutants defended, three documented gaps, two baselines, all through real `cargo test`.**
  Every defended mutant reddens; each gap is asserted to stay green *as documented*, because a
  mutant expected to survive is a gap with a ticket, not a pass. Each test is attacked in its own
  right rather than inheriting the other's verdict — a control belongs to a rule, not to a checker.
  And every injected clause is checked to **parse, and to actually emit the fabricated answer**,
  before its verdict counts: one draft wrote `rule evil {` (ADJ `rule` takes no name), every run
  reddened on a parse error, and without that liveness guard it would have been recorded as
  defended attacks that never happened.

  ### The premise that was false while the conclusion was true

  Round 1's other finding was in this entry and in both `.adj` headers: they said the ` | `
  separator *"appears NOWHERE in the page's content"*. Re-measured directly — **four** occurrences
  in the raw HTML (a `citation_title` meta, the `<title>`, and two inside the one footer line) and
  **three** in the rendered text, every one of them page chrome, none between two cells of Table 6.
  The conclusion they were offered to support is still true and independently measured — both
  welded `source` fields occur zero times — but the premise as written was false and checkable,
  which is exactly the failure this library exists to prevent. All three places now give both
  counts and name the chrome.

  ### 4o's debt

  4o's entry below, and `physics/energy-conversion-example.adj`, both read: *why the rest are not
  verbatim is not claimed — inline tags account for most, but at least one is a doubled space and
  two are HTML entities.* The clause after the colon contradicts the clause before it. It asserts a
  **proportion** over a set whose absolute counts the very next sentence declines to give, because
  two implementations of one written census disagreed by one — and counting was not the way out
  either, since the disputed population *is* the set the proportion ranges over. Both places now
  name only the cases actually inspected, say the remainder were not examined one by one, and state
  that **no proportion is asserted**.

