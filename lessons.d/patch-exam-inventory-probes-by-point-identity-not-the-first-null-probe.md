---
category: Testing & coverage
---

# Patch exam-inventory probes by point identity, not the first null probe

An exam-inventory edit that replaced a generic `"probe": null` match attached
two new probe arrays to the first null probes in the file: reflexives and an
orthography gap. The intended value-and-price and shopping points kept their
null probes, while the aggregate count happened to rise by the expected amount.

Patch these large inventories with the point's `id`, category, and label in the
context, then inspect the semantic diff before trusting an aggregate total. A
correct total cannot prove that the right points received the new evidence.
