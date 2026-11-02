# Changelog

## 0.1.0 — 2026-10-22

- Add exact-object binding across certificate-wide issuing-CA role policy,
  certificate-bound algorithm policy, and caller-owned self-issued
  classification.
- Add explicit-time inclusive validity classification derived from the exact
  retained intermediate certificate.
- Add non-copyable success evidence and payload-free mismatch, role, and
  validity failure categories.
- Add a non-copyable follow-up that retains optional critical NameConstraints
  decoded under a caller-owned budget beside the exact intermediate policy.
