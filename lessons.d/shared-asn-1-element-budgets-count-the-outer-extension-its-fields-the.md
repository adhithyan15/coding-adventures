---
category: Cryptography & security review
---

# Shared ASN.1 element budgets count the outer extension, its fields, the embedded root, and every nested field exactly once

An AIA decoder exhaustion test set `max_total_elements` to seven and expected
failure, but the checked path legitimately consumes exactly seven elements:
the outer Extension, its OID and OCTET STRING fields, the embedded AIA root,
one AccessDescription, its method OID, and its URI. Count the complete composed
decode path before choosing a falsification limit, assert success at the exact
count when useful, and set the failure probe one element below it. Do not
change production accounting merely to satisfy a miscounted test.

The same composition rule applies to OID-arc limits. A low limit supplied to
the complete Extension parser can reject the outer extension OID before a test
reaches an embedded access-method OID. When testing a separately authorized
embedded decoder budget, first obtain the generic Extension under its own
valid limits, then invoke the embedded semantic decoder with the deliberately
smaller budget.
