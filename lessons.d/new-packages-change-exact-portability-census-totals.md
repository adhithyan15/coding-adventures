---
category: Workspace & package metadata
---

# New packages change exact portability census totals

The TypeScript portability test pins exact repository-wide counts for projects,
shared configuration roots, and compiler locks. Adding a standalone package can
change all three, even when its source and configuration are otherwise valid.

After adding or removing a package, run the official portability audit and
update each affected exact assertion with a comment naming the package and why
that inventory changes. Keep the counts as strong census checks; do not replace
them with ranges or loosen the assertion. Also inspect the complete test module
for other package-count expectations so the first failing assertion does not
hide stale totals later in the test.
