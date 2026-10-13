---
category: Repo policy / workflow reminders
---

# Parenthesize independent jq projections before piping arrays

A multi-result `jq` audit separated projections with commas and then appended an
array pipeline without parentheses. Operator precedence fed earlier scalar
results into an object projection and failed with `Cannot index string with
string`. For audits that emit several independent values, wrap every piped
projection in parentheses or construct one explicit object whose fields each
contain their own complete pipeline.
