# A number that means one thing, read as meaning another — and the fix reproducing the bug (human-language-data, HL09)

The gap report said Spanish "reached A2". It had not: 178 words against the
~1,000–1,500 A2 asks for, fourteen lessons all realizing one spine node, present
tense only. **Nothing in the code lied.** `TrackLevelCoverage.reach` is documented
as *"the highest level this track has any lesson at"*, and that was accurate. One
lesson pointing at one A2 node moves it. The failure was that a number meaning
**touches** was read, for the life of the project, as meaning **attains**.

Then the gate written to fix that committed the identical error inside itself. Its
first version measured **whole-track** vocabulary (Spanish 138) against a
**per-level cumulative** target, and applied the atom-budget and reinforcement
criteria track-wide — so one over-budget Hindi lesson sitting *above* pre-A1 blocked
pre-A1, making that criterion unfalsifiable at the bottom of the ladder for every
track. Correctly scoped, Spanish's pre-A1 vocabulary is **44, not 138**.

**The general rule: when a criterion says "at or below level X", the measurement
must be filtered to level X.** A whole-corpus figure compared against a per-slice
threshold is not a stricter version of the right check — it is a different check
that happens to return a number. Three sibling bugs in the same module, all found by
the pre-push security review:

- criterion read "never revisited" where the spec said "fewer than twice", hiding 51
  of 141 failures;
- vocabulary counted every lesson type, so drill titles became words — `(practice)`,
  `qu-`, `fact or wish?`, 25 of 138;
- a level with **no authored nodes passed** its node criterion, because "no node is
  unrealized" was implemented as "every node is realized" — the same touches-vs-means
  error one level up, and live for B1–C2, which have zero nodes.

Corollaries worth keeping:

1. **Name the population in the field name or the doc.** `vocabulary` alone invites
   the misreading; `vocabulary` documented as *"at ANY level — context, not the
   criterion"* does not.
2. **Report the shortfall, not a boolean.** `false` moves the argument; *"teaches 44
   distinct headwords at or below pre-A1, against 300"* settles it.
3. **Absent ≠ zero.** "Not measured" and "attained nothing" are opposite facts; the
   section is `undefined` when its inputs were not supplied, and a test pins that.
4. A related discovery in the same PR: `report-cli` had never passed `curricula`/`spine`,
   so the whole `levels` section had been **silently absent from every CLI run** since
   it shipped — implemented, tested, and invisible to anyone reading the output. A
   feature with tests but no rendered output is not shipped.
