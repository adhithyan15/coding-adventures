# Changelog

## Unreleased

- Bind authenticated local sessions to the stable non-secret `operator:local`
  requester identity and authorize the new pipeline wire/unwire API operations.
- Accept validated Trust Checker request context while preserving unconditional production denial.

## 0.1.0

- Add an OS-random 256-bit local daemon bearer credential encoded as lowercase hex.
- Authenticate credentials with repository-owned constant-time comparison.
- Zeroize retained and generated credential material on drop.
- Authorize every current daemon operation only after connection-local authentication.
- Deny channel topology mutations until a real Trust Checker approval adapter exists.
