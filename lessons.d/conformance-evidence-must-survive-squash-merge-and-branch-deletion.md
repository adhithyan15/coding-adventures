---
category: CI & GitHub Actions
---

# Conformance evidence must survive squash merge and branch deletion

A metadata gate on unrelated PR #16716 could not resolve the Elixir/Lua barcode adoption's pre-merge revision after #16706 squash-merged and its branch was deleted. A full-history checkout does not retain unreachable PR-head objects. Rebind conformance evidence to the durable squash-merge revision after checking that each package tree still matches its pinned tree hash. Update the owning backlog item, preserve the validator, and use existing issue #16719 instead of filing duplicate reports.
