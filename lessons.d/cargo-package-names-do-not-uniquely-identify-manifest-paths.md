---
category: Testing & coverage
---

# Cargo package names do not uniquely identify manifest paths

The final CV04a carrier audit reused a helper that indexed workspace manifests
by package name, then overlaid program manifests. Both package and program
directories contain `grammar-tools`; the overlay selected the program instead
of the package actually resolved by Cargo. A strict expected-count assertion
failed at 2804 objects rather than accepting the incomplete native inventory.

Keep Cargo's resolved manifest path for workspace dependencies. Index other
manifests separately and reject ambiguous fallbacks. Recompute recursive
mode/type/blob inventories for every dependency root, including documentation.
The corrected standalone Closure inventory contains 3705 objects over 36 roots,
not the earlier 3699. All corrected inputs are identical to the actually tested
source316e; this is corrected attribution, not a new test execution. Preserve
the failed audit and old receipts, and independently review the correction.
