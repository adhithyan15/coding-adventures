---
category: BUILD files & dependency management
---

# Adding a BUILD front must retire its pending orphan exemption

NN44 supplied real Ruby matrix BUILD fronts, but left its PENDING entry in
code/BUILD-EXEMPTIONS. Both CI and CodeQL planning failed before package execution
with ORPHAN_EXEMPTION_STALE / COVERED. Passing package-local tests did not cover
the repository planner's exemption inventory.

When an orphan package becomes covered, remove only its exact stale exemption
in the same change and pin that absence in a regression test. Preserve unrelated
pending owners. Inspect actual planner logs before changing package code; do not
disable orphan validation or claim that a bootstrap failure is a numerical defect.
