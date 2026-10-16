## HL-C456-8e6b0fca — Sanskrit A1 census derives exact totals from direct gap owners

**Status: CLOSED (2026-10-16). Tracks #17619.** The post-#17627 exact-main
audit found the Sanskrit A1 census pin in 15 of the latest 50, 100, and 200
first-parent human-language commits. Although #17617 gave every inventory point
a direct source owner, ordinary point closures still edited shared coverage
totals, category counters, percentages, and cumulative commentary. A
migration-only whole-inventory digest also meant a direct probe change still
required a second shared test edit.

The census now treats every point as covered unless that point owns one
canonical file under `a1-coverage-gaps.d/`. Those files preserve exact unmapped
or partial state and missing atoms. The gate compares every measured point to
that independent expectation, rejects missing or unknown owners, and derives
exact summary, category, percentage, report, and ordered-gap results. Closing a
gap removes only its own owner. Point-specific ordinal and relative-position
evidence also moved out of the census and into independent hashed cases, while
the direct-inventory test retains aggregate absence and owner/point parity
without a moving whole-file digest.
