# Changelog

## Unreleased

- Reconstruct a fresh selected transcript only from the retained exact retry
  messages and original policy, require exact suite and snapshot equality, and
  retain it beside the complete server Finished-key provenance.
- Derive and retain the server Finished key only through the exact-message
  retry handshake traffic-secret proof, with pre-mutation role and suite
  substitution checks.
- Expose the retained final ClientHello extension block and original extension
  context policy only through the sealed complete retry base-schedule evidence
  so downstream validation cannot substitute detached policy inputs.

## 0.1.0

- Bind client and server handshake traffic secrets to the exact retained TLS
  1.3 retry transcript and base-secret schedule.
- Reject output-storage suite substitution before mutation.
