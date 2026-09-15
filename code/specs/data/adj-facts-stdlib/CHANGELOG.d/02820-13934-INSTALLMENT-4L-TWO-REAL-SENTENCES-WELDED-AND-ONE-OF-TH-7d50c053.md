- **#13934 installment 4l: two real sentences, welded — and one of them is a heading.** **1** value
  line rewritten into a `source` plus **1** `cites`, alongside **48** comment lines added against
  **10** removed, in **1** `.adj` file (50 added / 11 removed lines in total) — `geography/oceans` —
  plus one e2e test file. **Six mutations redden.**

  `oceans:42` presented two sentences as one continuous quotation. Both are real and each occurs
  exactly once; the weld does not, because they sit **805 characters apart**, separated by an image
  caption about the Pacific's "Ring of Fire" and other prose. The `source` now keeps the first and
  the second is a `cites` under the same locator. Together they still fix all five ranks.

  ### The two are different kinds of span, which the header did not say

  The old header called them "prose" and "that one span". They are neither one span nor both prose:

  | | where it lives | byte-exact in raw HTML? |
  |---|---|---|
  | 1st | the page's `<h2>` **subheader** — the one-line answer to its own title question, with `<span>` around "Pacific Ocean" | only inside two `<meta>` description attributes; the body copy is broken by that span |
  | 2nd | a `<p>` in the body | **no** — the quoted sentence carries two DOUBLE-SPACE runs, `followed by␣␣the` and `Arctic Ocean␣␣basin`. (␣ marks a literal space: markdown code spans render `white-space: normal`, so real double spaces would show here as single and the exhibit would prove nothing — which is what this row did until review caught it.) |

  So the second is verbatim under whitespace collapsing, not byte-exact. The shipped value was
  already collapsing those spaces silently; the header now says so. A `WHITESPACE` mutant that
  restores the page's own double spaces reddens, which is what makes the distinction load-bearing
  rather than decorative.

  ### Both extractors agree here, and why that sharpens #14752

  4k found that the sweep's every-tag-is-a-break extractor manufactures phantom spaces. It does not
  bite here, and the reason is specific to this page: the `<span>` boundary in the `<h2>` falls
  where a space already sat (`The <span>Pacific Ocean</span> is`), so the injected break collapses
  into that space and changes nothing.

  **No general rule about when that extractor fails is asserted.** Two were attempted and both were
  wrong. The first — "only when the boundary abuts **punctuation**" — is falsified on the
  cloud-types page by `generally&nbsp;t</span>hick`, which splits a word. The second — "whenever
  the boundary is **not already adjacent to whitespace**" — is falsified on *this* page by
  `Service.<br>Stewardship.`, where `<br>` is a real break and the injected space is correct.

  The question belongs to #14752, which now carries a corrected statement. Nothing is inferred here
  about how much of the backlog is affected — that is a measurement, and it has not been made.

  ### Why it survived: the pin was a host and a trust tier

  Third file in a row with that shape, after 4j's hostname and 4k's locator-and-trust. The suite
  asserted `contains("oceanservice.noaa.gov") && contains(trust)` and never the `source` text, so
  the welded string satisfied it exactly as happily as the real spans do.

  The needle is now the whole citation object as the serialiser emits it, closing on the
  corroborations `]`. Its string appears **four** times in this test's output — two sections per
  answer, two binding queries — and that was checked before trusting it: mutating the `.adj` moves
  all four together, so they are same-file echoes rather than the independently-driftable copies of
  #14745.

  ### Mutants

  Six, one per thing the repair asserts: RESTORE (the welded string put back exactly as it
  shipped), WHITESPACE (the page's own double spaces restored), DROP-CITES, EXTRA-CITES (a
  fabricated corroboration under an attacker-controlled locator), LOCATOR-TAIL and WORD. All six
  redden. **RESTORE earns the pin**; WHITESPACE earns the header's care about which representation
  the second sentence is verbatim in.

