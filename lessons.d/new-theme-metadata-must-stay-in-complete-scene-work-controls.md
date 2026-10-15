---
category: Testing & coverage
---

# New theme metadata must stay in complete scene work controls

The sign and edge-label theme main import at 629e5a49 added the default
`swimlane.theme.textColor` field. The integer two-support authored-scene control
failed because its complete metadata oracle still expected four fields. The
failed run is retained in the CV04a sign-edge initial receipt; it is not an
acceptance result.

Update the exact expected map with the new default field while preserving all
geometry, route, style, duplicate-occurrence and work-bound assertions. Add an
authored theme control that verifies complete font queries, glyphs and background
rectangle fields and instruction positions. Remove only individually verified
background rectangles and their exact metadata entry before comparing the entire
remaining scene with the same authored font/color baseline.

When an upstream theme evolves, audit the complete scene difference first. Do
not clear metadata or replace exact scene equality with token-presence checks to
make a resource or rendering control pass. Native and lint acceptance still
require an actual successful run on the amended source.
