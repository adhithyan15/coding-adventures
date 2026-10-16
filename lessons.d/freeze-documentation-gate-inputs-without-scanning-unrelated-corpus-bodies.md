---
category: Testing & coverage
---

# Freeze documentation gate inputs without scanning unrelated corpus bodies

A docs-only validation helper tried hashing all53,305 tracked Markdown bodies
before even recording or running a check. It spent minutes in unrelated corpus
bookkeeping. No validation command or receipt was produced. After verifying its
exact unique helper command, parent and PID, stop only that new owned helper and
consume its session; preserve the abort record. Never query or terminate a
separate frozen Miri run to fix this problem.

For the successor, bind the exact clean Git HEAD/index/tree, all changed document
bodies, current checker source/emission/compiler/runner/helper and the lesson
validator. Keep the FULL unchanged doc-shard check over the actual checkout,
lessons validation and diff check; reducing unrelated preflight hashing does not
mean reducing the command's validation scope. Persist each actual command result
before secondary checks and retain original refused/aborted evidence separately.
