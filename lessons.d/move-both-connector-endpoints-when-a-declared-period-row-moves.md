---
category: Testing & coverage
---

# Move both connector endpoints when a declared period row moves

Independent review withheld the first event-height native oracle migration:
the following Q period and its event moved, but the connector's period-center
endpoint remained at the old row. The generic patch seam also omitted y1.
Retain that reviewed candidate and finding. Add an exact y1 preimage patch,
then re-audit every item linked to a moved period rather than only the visible
event stack. Valid finite arrow geometry does not prove semantic attachment
to the correct period center. No production or execution acceptance preceded
the review finding.
