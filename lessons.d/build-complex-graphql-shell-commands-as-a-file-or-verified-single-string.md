---
category: Repo policy / workflow reminders
---

# Build complex GraphQL shell commands as a file or verified single string before embedding them in JavaScript orchestration.

A JavaScript orchestration cell tried to interpolate a multiline GraphQL query, shell quoting, and a jq program into one template literal. The nesting was malformed, so JavaScript failed to parse before the GitHub query ran. Put the query in a reviewed file or pass a compact, separately verified string to `gh api`; keep JavaScript interpolation and shell quoting to one layer at a time.
