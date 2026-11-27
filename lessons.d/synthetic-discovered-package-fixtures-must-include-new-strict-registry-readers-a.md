---
category: Testing & coverage
---

# Synthetic discovered package fixtures must include new strict registry readers after a merge

A merge added OCaml and Swift historical-Unix readers to the production
registry, but a Ruby identity regression constructed a temporary discovered
package universe containing only the previous readers. Strict selection then
correctly rejected an absent OCaml consumer. The real repository package was
not missing; the synthetic fixture was incomplete.

Extend that fixture's literal package list with both new readers and retain
the exact selection, discovered Ruby underscore identity, obsolete-alias
rejection and cache-input assertions. Review fixtures introduced on only one
side of a merge as well as textual conflict blocks. Run the focused regression
and the complete root package suite; do not weaken strict missing-consumer
validation to make a stale fixture pass.
