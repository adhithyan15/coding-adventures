---
category: CI & GitHub Actions
---

# Classify truncated pinned downloads before generated conformance failures

PR17288 head0d316837 failed the Unicode17 Elixir job116286299314 in
run38692240034 while `_download_sources` called `_download_exact`. Python raised
`IncompleteRead(383931 bytes read)` inside the bounded HTTP response read.
Toolchain setup succeeded; generated-artifact comparison and Elixir conformance
did not execute. Preserve the actual failed job log rather than treating the
job title as evidence of a runtime defect or a conformance pass.

Retain the pinned HTTPS origin, redirect rejection, exact size and SHA256
checks. Once the workflow is terminal and the head is still applicable, one
targeted retry can distinguish a transient source transfer from a repeatable
defect. A queued retry is not a pass. Do not change package behavior or relax
source verification to hide acquisition failure.
