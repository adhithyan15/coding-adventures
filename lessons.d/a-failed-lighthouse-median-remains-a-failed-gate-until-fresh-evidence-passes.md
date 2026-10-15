---
category: Testing & coverage
---

# A failed Lighthouse median remains a failed gate until fresh evidence passes

Closure PR17288's macOS Forme job built its products but failed the landing
page's three-sample Lighthouse median:0.93 against0.95. Unchanged web source
does not establish a flaky-run cause or waive the failure. Preserve the exact
head, failed step/log and threshold. GitHub rejected a bounded job retry with
HTTP403 while the containing workflow was active; do not keep sending the
same rejected request. Wait for terminal workflow state, then obtain fresh
exact-head evidence without relaxing budgets. A repeated failure needs measured
diagnosis and a real repair; prior/local passes are not hosted acceptance.
