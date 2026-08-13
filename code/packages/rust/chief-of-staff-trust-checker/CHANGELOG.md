# Changelog

## 0.1.0

- Add bounded exact-resource validation and canonical maximum-tier evaluation.
- Add an injected approval-provider contract carrying the exact request and tier policy.
- Preserve Tier 1 timeout auto-approval while failing Tier 2 and Tier 3 timeouts closed.
- Reject approval evidence weaker than biometric or hardware-key requirements.
