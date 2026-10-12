---
category: Testing & coverage
---

# Narrative contract assertions should normalize line wrapping and case

A focused Sanskrit A2 test searched raw Markdown for a sentence whose words
were split by a normal line wrap, and for an imperative whose capitalization
differed between two equivalent masters. Both contracts were present, but the
literal assertions failed. Normalize whitespace and case for prose contracts;
reserve exact raw-text assertions for machine-readable markers, headings, and
values whose spelling is itself the contract.
