# Changelog

## 0.1.0

- Bind one sender's handshake traffic key and static IV to the exact retained
  TLS 1.3 retry handshake traffic-secret proof.
- Reject output-storage suite substitution before mutation.
- Consume the complete retry traffic-secret proof into a single-use issuer
  that permits one fresh monotonic record state per sender and rejects
  duplicate sender issuance before output mutation.
