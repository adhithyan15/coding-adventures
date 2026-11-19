# Changelog

## Unreleased

- Derive and retain the server Finished key only through the exact-message
  retry handshake traffic-secret proof, with pre-mutation role and suite
  substitution checks.

## 0.1.0

- Bind client and server handshake traffic secrets to the exact retained TLS
  1.3 retry transcript and base-secret schedule.
- Reject output-storage suite substitution before mutation.
