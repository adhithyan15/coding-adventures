# Changelog

## 0.1.0

- Bind one sender's handshake traffic key and static IV to the exact retained
  TLS 1.3 retry handshake traffic-secret proof.
- Reject output-storage suite substitution before mutation.
- Consume the complete retry traffic-secret proof into a single-use issuer
  that permits one fresh monotonic record state per sender and rejects
  duplicate sender issuance before output mutation.
- Seal and open exact retry-handshake records through the repository-owned
  record dispatcher while retaining the state, exact input or request, and
  output owner in non-cloneable proof wrappers.
- Reject authenticated non-handshake inner content types and wipe their opened
  plaintext before returning the stable error.
