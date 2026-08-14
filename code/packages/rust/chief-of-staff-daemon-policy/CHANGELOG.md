# Changelog

## Unreleased

- Bind authenticated local sessions to the stable non-secret `operator:local`
  requester identity and authorize the new pipeline wire/unwire API operations.
- Accept validated Trust Checker request context while preserving unconditional production denial.
- Add an exact config-backed resource-tier resolver and production Trust Checker
  composition: fully declared Tier 0 requests can proceed, while missing mappings
  and every interactive tier fail closed through an unavailable provider.
- Select an optional shell-free Tier 1 notification command from validated
  configuration while preserving fail-closed Tier 2/3 behavior.
- Route an independently optional shell-free Tier 2 biometric helper through a
  strict exact-request process boundary while Tier 3 remains unavailable.
- Route an independently optional shell-free Tier 3 hardware-key helper through
  a strict exact-request process boundary without opening lower tiers.
- Prove validated privilege deadlines equal the exact canonical timeout carried
  to each production approval provider.

## 0.1.0

- Add an OS-random 256-bit local daemon bearer credential encoded as lowercase hex.
- Authenticate credentials with repository-owned constant-time comparison.
- Zeroize retained and generated credential material on drop.
- Authorize every current daemon operation only after connection-local authentication.
- Deny channel topology mutations until a real Trust Checker approval adapter exists.
