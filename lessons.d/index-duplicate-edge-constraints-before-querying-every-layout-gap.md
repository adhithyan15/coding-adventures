---
category: Rust
---

# Index duplicate edge constraints before querying every layout gap

The Swimlane minimum-link feature scanned the entire authored edge list for
every adjacent lane pair during both lane sizing and placement. Valid duplicate
edges multiplied that work to 2(N-1)E even though each parsed length was only
1–3. The previous passing parser/layout/render suites did not detect this;
independent security review rejected current-main integration in #17334.

Commit the work contract first. Build one canonical unordered pair-to-maximum
index, borrowing endpoint IDs and preserving every authored output edge. Count
actual constraint-edge visits and gap queries in deterministic tests of the
real layout path, including reversed/duplicate constraints and all directions;
do not rely on timing thresholds. Keep other existing node-search complexity
and output lowering distinct from the repaired constraint-discovery bound.
Accumulate geometric units without integer overflow for caller-owned IR.

Retain the original successful native receipt separately from the rejected
security review, then run the affected cone and exact-head review again. Miri
DEFERRED/PENDING is not a pass and is not a reason to accept a security finding.
For compact one-line JSON audit caches, decode and select current-head entries
instead of printing a whole matching line with a text search.
