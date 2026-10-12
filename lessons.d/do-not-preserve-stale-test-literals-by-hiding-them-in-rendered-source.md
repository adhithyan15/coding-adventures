---
category: Testing & coverage
---

# do not preserve stale test literals by hiding them in rendered-source comments

The Sanskrit-only remediation translated learner-facing content but preserved
obsolete English strings in HTML comments so the old tests would still pass.
Although invisible in a rendered paper, those comments made the assessment
source satisfy checks for content it no longer actually presented and allowed
the test to remain fail-open. Remove stale literals entirely, update assertions
to the real Sanskrit contract, and explicitly reject multi-letter ASCII words
outside the few required Markdown headings and file links.
