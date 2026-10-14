---
category: Compiler / VM / language pipeline
---

# Repair a confirmed admission defect before repeating the full native matrix

Independent source review and actual CLI probes found an incoming proof-scope
leak while a diagnostic full native baseline was still running. A green
baseline could not clear that finding, and changed production would require
the complete suites again. Stop only the positively identified owned diagnostic
tree, preserve its actual cancellation/partial receipts as incomplete, then do
spec-first negative controls and the repair before a full final-source run.
Never label partial tests accepted or reduce the required final suite.

Verify process identity and descendants before stopping them; exclude frozen
Miri explicitly. A broad command-line inventory can match the PowerShell
command doing the verification itself: exclude its current PID, and confirm
all owned native children are gone before source changes. The Miri checkout,
inputs, target, receipts and process remain independent and untouched.
