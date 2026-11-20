category: Testing & coverage
---

# Parse LCOV line and branch records by their distinct field shapes

LCOV `DA` records contain a line number and hit count, while `BRDA` records
contain four fields. A coverage-report inspection script that unpacked both
shapes as if they were identical stopped on the first line record. Parse and
validate each record type independently before deriving uncovered lines or
branches.
