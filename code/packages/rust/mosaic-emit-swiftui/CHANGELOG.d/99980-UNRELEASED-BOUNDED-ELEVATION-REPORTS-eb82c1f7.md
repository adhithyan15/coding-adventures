## Unreleased bounded elevation reports

- Compute the existing supported-elevation predicate once per expanded part
  instead of rescanning every property for every dropped non-inset shadow.
  Preserve returned fields/order, inset/standalone/unsupported semantics and
  all existing gap/align/border filters and production limits.
- Add authored duplicate-property controls with exact predicate visit counts
  at 32/128 shadows, multi-part locality and ordered report comparisons. The
  private observer uses a no-op in the unchanged public API; no new allocation,
  dependency or timing proxy.
