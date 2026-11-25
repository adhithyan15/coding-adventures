# Changelog

## 0.1.0

- Join the exact authenticated ordered server Finished verdict to the record
  that successfully appended its encoding, retaining the resulting transcript
  snapshot. Consume the record result and verdict so a single transition can
  produce at most one committed result. Reject a different record owner or
  key proof and a record without a Finished append at its boundary.
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
- Decode an exact authenticated ordered main-handshake `CertificateRequest`,
  require its empty context and `signature_algorithms` extension, apply the
  retained request-context placement policy, and retain structurally decoded
  handshake and optional certificate signature-scheme lists in the proof.
- Decode an exact authenticated ordered main-handshake server `Certificate`,
  require its empty context and nonempty entry list, and validate every exact
  entry extension vector against retained placement and final-offer policy.
- Bind the exact first authenticated server `Certificate` entry to one
  repository-decoded X.509 certificate only when their complete retained DER
  bytes match, while retaining both complete proofs without claiming path,
  hostname, trust, revocation, or signature validation.
- Decode an exact authenticated ordered server `CertificateVerify`, retain its
  pre-message transcript snapshot, require its scheme in the exact bounded
  final ClientHello offer, and authorize that scheme through caller-owned
  data-driven repository policy without claiming signature verification.
- Bind that exact ordered proof to a repository-owned RSA-PSS verification
  verdict only when server role, selected-transcript snapshot, authorized
  scheme, and complete decoded message all match, while leaving the verified
  public key unbound to the preceding Certificate and reviewed path.
- Attach one opaque redacted identity for the exact single-use server record-
  state issuer to every pending ordered transcript message so sealed
  downstream receipts can reject cross-owner substitution across callback
  lifetimes without retaining borrowed message bytes. The identity retains the
  issuer lifetime so an address recycled after issuer destruction cannot match.
- Authenticate the exact ordered retry server `Finished` body against its
  exact pre-message transcript snapshot and retry-derived server Finished-key
  proof, rejecting detached retry evidence before HMAC work and retaining the
  ordered-owner identity, key proof, snapshot, client-authentication flag, and
  exact peer verify-data verdict in non-copyable success evidence.
