# Changelog

## 0.1.0

- Add a certificate-wide, fail-closed gate for caller-recognized critical
  extensions.
- Add bounded algorithm-independent issuing-CA and end-entity role checks that
  compose Basic Constraints, Key Usage, and Extended Key Usage under one
  explicit semantic decoder budget.
- Add registry-free selection of one exact caller-supplied certificate
  algorithm profile with parameter and inclusive encoded BIT STRING length
  policy.
