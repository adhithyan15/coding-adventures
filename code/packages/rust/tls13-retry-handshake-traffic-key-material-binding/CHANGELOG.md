# Changelog

## 0.1.0

- Bind one sender's handshake traffic key and static IV to the exact retained
  TLS 1.3 retry handshake traffic-secret proof.
- Reject output-storage suite substitution before mutation.
- Consume the complete retry traffic-secret proof into a single-use issuer
  that permits one fresh monotonic record state per sender and rejects
  duplicate sender issuance before output mutation. Finished-key and
  transcript-continuation proofs for those secrets derive through the issuer's
  immutable proof accessor so a second sequence-zero issuer remains impossible.
- Seal and open exact retry-handshake records through the repository-owned
  record dispatcher while retaining the state, exact input or request, and
  output owner in non-cloneable proof wrappers.
- Reject authenticated non-handshake inner content types and wipe their opened
  plaintext before returning the stable error.
- Consume exact retry record state into bounded handshake reassembly so only
  bytes authenticated under that monotonic sender state can become messages.
- Retain fragments across authenticated records and synchronously deliver every
  complete policy-authorized message, including coalesced messages, without
  permitting borrowed message bytes to escape the callback.
- Clear buffered fragments and permanently close the owner on record,
  content-type, reassembly, or handler-unwind failure so rejected sequence
  positions cannot splice non-contiguous records into a message.
- Consume authenticated server-sender reassembly into an internally initialized
  no-PSK server-flight owner and advance its public message-type order before
  lending each accepted message to the caller.
- Reject client sender state and reassemblers that already consumed or failed a
  record, retain post-accept ordering and client-auth state in callback and
  record evidence, and permanently close the owner on ordering failure before
  any later record can be processed.
- Consume one fresh ordered server-flight owner with the exact retry server
  transcript continuation only when both retain the same traffic-secret
  object, suite, and ServerHello snapshot, then retain the ordered owner,
  transcript, and server Finished-key provenance in one non-cloneable wrapper.
- Authenticate records through that joined owner, lend each ordered message
  with its pre-message transcript snapshot to a synchronous authorization
  handler, and hash its exact encoding only after authorization.
- Retain the post-record transcript snapshot with the exact ciphertext,
  authenticated record result, ordered state, and Finished-key proof; reject
  unauthorized messages without hashing them and permanently close record
  processing on rejection, transcript failure, or handler unwind.
- Decode an exact authenticated ordered `EncryptedExtensions` body under
  caller-selected limits and bind its extension block to the retained final
  ClientHello offer and exact retry extension-context policy before the caller
  authorizes transcript advancement.
