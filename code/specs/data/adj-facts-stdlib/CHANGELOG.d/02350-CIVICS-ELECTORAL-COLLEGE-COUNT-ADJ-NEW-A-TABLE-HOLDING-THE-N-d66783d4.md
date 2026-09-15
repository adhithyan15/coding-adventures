- `civics/electoral-college-count.adj` (new) — a `table` holding the numbers USA.gov states
  about the U.S. Electoral College: `electoral_college_count(quantity, count)`, `total_electors` →
  538, `district_of_columbia_electors` → 3, `electors_needed_to_win` → 270,
  `winner_take_all_states` → 48. The NINTH library in the `civics/` domain and the first about
  elections themselves, sourced from USA.gov's "Electoral College" page — curl-fetched and read
  byte-for-byte, with the URL reachability-checked before any scoping work. `trust authoritative`.

  "How many electoral votes do you need to win?" has an exact, checkable answer, and exact numbers
  are precisely the sort of fact that should come from a citation rather than a model's
  recollection — so every answer carries the sentence stating it, and a dedicated test checks the
  threshold sentence rides along rather than only that the binding is 270.

  THE FIRST COLUMN NAMES WHAT IS COUNTED, because the units differ: three rows count ELECTORS and
  one counts STATES, so a bare `count` column would be ambiguous on its own. The quantity atom
  carries the unit (`total_electors`, `winner_take_all_states`) rather than leaving a reader to
  infer it from context.

  WHERE THE HEDGE LIVES. The source says "there are CURRENTLY 538 electors in all", and that
  qualifier is real — the total tracks congressional apportionment, so it is stable but not
  constitutionally fixed. Unlike `veto-override.adj`, where the source's "in most cases" attaches to
  a distinguishable VALUE and therefore lives inside the recalled atom, here the hedge qualifies the
  whole count, so the faithful placement is the citation: the verbatim sentence carrying "currently"
  travels with every answer, and a test asserts it, so a reader does not inherit a bare number
  presented as timeless.

  ENCODING NOTE, recorded because it cost a detour. The "270 electors" sentence contains U+2014 EM
  DASHes, and a verification that piped CLI stdout through a Python reader appeared to show them
  mangled. That was the READER decoding UTF-8 with the Windows default codepage, not a CLI defect —
  the CLI emits correct UTF-8 (`\xe2\x80\x94`) and the sentence is byte-identical to the source
  when decoded properly. On Windows, verify verbatim citations by reading the bytes and decoding
  UTF-8 explicitly, or assert from Rust where `String::from_utf8` handles it. A test now pins the
  em-dash round trip, and asserts the dash has not degraded to a hyphen, so nobody re-investigates
  the phantom.

  Honest abstention on Maine and Nebraska's proportional METHOD of assigning electors (not a count,
  and the reason those two states are excluded from the 48); on the three components the page lists
  for the process — selection of electors, meeting of electors, counting of the votes by Congress —
  which are a list of what the process INCLUDES and are not numbered, so tabling them as an ordered
  sequence would assign positions the source never states (see `bill-stage-successor.adj`'s header
  for the case that established this rule); on "this has happened twice" and the 2016/2000
  popular-vote outcomes, which count HISTORICAL OCCURRENCES rather than the size of the College; and
  on the penalties a faithless elector may face (fined, disqualified, replaced, prosecuted), which
  are consequences rather than counts. New `electoral-college-count.query.adj` and
  `facts_electoralcollegecount_e2e.rs` (5 tests: the winning threshold with its sentence, the
  em-dash round trip, every stated number covered, the "currently" hedge carried, and honest
  abstention on a method and a historical count). New manifest objective
  `adj.civics.3to5.electoral_college_count` with no prerequisite — elections are their own strand,
  and inventing an edge to the branches/Congress chain would misrepresent the dependency graph.

