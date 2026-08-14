# Changelog

## Unreleased

- Export the canonical Tier 1, Tier 2, and Tier 3 approval deadlines so
  configuration adapters cannot drift from the enforced policy.
- Raise the bounded resource count to represent the complete maximum-size D18 channel membership.
- Add validated request context for authoritative resource-resolution adapters.

## 0.1.0

- Add bounded exact-resource validation and canonical maximum-tier evaluation.
- Add an injected approval-provider contract carrying the exact request and tier policy.
- Preserve Tier 1 timeout auto-approval while failing Tier 2 and Tier 3 timeouts closed.
- Reject approval evidence weaker than biometric or hardware-key requirements.
