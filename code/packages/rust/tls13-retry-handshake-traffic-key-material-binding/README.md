# tls13-retry-handshake-traffic-key-material-binding

`tls13-retry-handshake-traffic-key-material-binding` accepts only the existing
exact-message `BoundRetryHandshakeTrafficSecrets` evidence, one explicit
client-or-server sender role, and caller-owned traffic-key-material storage. It
imports the retained sender's exact handshake traffic secret into a fixed
wipe-on-drop PRK and derives the selected suite's traffic key and static IV.

Caller-owned output storage must retain the exact selected suite before any
mutation. Success returns a non-cloneable wrapper retaining the complete CH1,
HelloRetryRequest, CH2, final ServerHello, base-schedule, and handshake-secret
proof beside the sender role and a borrowed key-material view into caller-owned
fixed wipe-on-drop storage. Callers cannot supply a detached secret, suite,
hash, output length, label, or context.

The complete traffic-secret wrapper can instead be consumed into a single-use
record-state issuer. That issuer permits one successful client issuance and
one successful server issuance. Repeating a sender is rejected before output
storage is touched. Each success derives the retained sender's key material,
moves it into fresh sequence-zero nonce state, marks that sender issued, and
returns a non-cloneable wrapper that borrows the issuer. Callers can borrow or
mutably borrow the state through that wrapper, but cannot detach it from the
complete retry proof or create a second sequence-zero state for the same
sender through this boundary. Finished-key and transcript-continuation proofs
for the same secrets must be derived through the issuer's immutable proof
accessor, preserving that single owner and its issuance state.

The exact state wrapper can also seal or open one retry-handshake record through
the repository-owned record dispatcher. Sealing fixes the authenticated inner
content type to `handshake` and returns a proof that retains the exact state,
plaintext request, and sealed output prefix. Opening authenticates before it
returns content, rejects an authenticated non-handshake content type, and
returns a proof retaining the exact state, ciphertext input, and wipe-on-drop
opened owner. Both proof types keep the borrowed state and record buffers from
being reused or mutated while live. Record sequence and failure behavior remain
the dispatcher's existing policy.

The exact state can additionally be consumed into a bounded authenticated
handshake reassembler. Each record is authenticated under that one monotonic
sender state before any bytes enter reassembly storage. The owner retains a
final fragment across records and synchronously lends every complete,
policy-authorized message to a caller callback, including multiple messages
coalesced in one record. Caller-owned opened-record and reassembly buffers are
wiped by their repository-owned owners, while the result retains the exact
ciphertext input and retry evidence until it is consumed.
Any record authentication, content-type, reassembly, or handler-unwind failure
clears the fragment and permanently closes the owner before a later record can
be processed, preventing cross-gap message splicing.

The authenticated reassembler can then be consumed into a server-only ordered
owner. It rejects client sender state, creates the no-PSK server-flight policy
internally at `ExpectEncryptedExtensions`, rejects any reassembler that already
consumed or failed a record, and advances that policy before it lends each
authenticated message to the callback. Callback wrappers and
per-record results retain the post-accept state and client-authentication
requirement. An ordering failure clears and permanently closes the reassembler
before a later record can be processed. If one authenticated record contains
an earlier valid message followed by an invalid one, the earlier callback may
already have run; caller-owned effects remain caller-audited and are not
transactional.

A fresh ordered owner can also be consumed together with the exact retry
server transcript-continuation proof. The join requires pointer identity of
the retained handshake-traffic-secret object, zero consumed records, no
fragment or failure, the initial `ExpectEncryptedExtensions` position, exact
suite equality across ordered state, transcript, and server Finished key, and
the retained ServerHello snapshot. Success moves the ordered owner and
reconstructed transcript into one non-cloneable wrapper while retaining the
complete server Finished-key proof. Only immutable access is exposed.

The joined owner can authenticate a ciphertext record, enforce each completed
message's no-PSK ordering transition, and lend the exact message with its
pre-message transcript snapshot to a synchronous non-retaining handler. A
`true` result authorizes hashing that message's exact header-plus-body bytes;
`false` rejects it without hashing it and permanently closes record
processing. Success retains the exact ciphertext, authenticated record result,
server Finished-key proof, post-record flight state, and post-record transcript
snapshot in one non-cloneable result. Record, reassembly, ordering, rejection,
transcript, and handler-unwind failures are terminal. Earlier callback effects
are not transactional, and a transcript-capacity failure can occur after the
handler authorizes the affected message.

When that pending ordered message is `EncryptedExtensions`, the handler can
decode its exact authenticated body under explicit structural limits and bind
every extension to the retained retry policy and final ClientHello offer. The
result is a non-cloneable proof retaining the pending pre-message transcript
snapshot beside the exact decoded body. The underlying base-schedule evidence
remains private to the pending value, so this body check cannot be redirected
to a detached offer or policy. This helper validates only extension-block
structure, message placement, allowed context, uniqueness, and offer matching;
it does not interpret extension payload semantics.

When the pending message is a main-handshake `CertificateRequest`, the handler
can decode its exact authenticated body under explicit context, extension, and
signature-list limits. The helper requires the exact post-request ordered
state and client-authentication flag, an empty request context, and the
retained policy's `CertificateRequest` placement rules. It then requires and
structurally decodes `signature_algorithms`, and similarly decodes
`signature_algorithms_cert` when present. The non-cloneable proof retains the
pending pre-message snapshot, exact request, required handshake-signature list,
and optional certificate-signature list. It does not select an algorithm,
access credentials, sign, or decide whether client authentication is supported.

When the pending message is the main-handshake server `Certificate`, the
handler can decode its exact authenticated body under bounded message and
per-entry extension limits. The helper requires the exact post-certificate
ordered state, an empty request context, and at least one certificate entry.
It decodes every entry's retained length-prefixed extension vector and applies
the exact retry policy and final ClientHello offer in the server-certificate
context. The non-cloneable proof retains the pending pre-message snapshot and
the exact nonempty certificate message only after every entry passes. It does
not parse X.509 bytes or semantically interpret entry extension payloads.

When the pending message is server `CertificateVerify`, the handler can decode
its exact authenticated body at the post-accept `ExpectFinished` position. The
helper decodes the retained final ClientHello's exact `signature_algorithms`
offer under a caller-selected count ceiling, requires the peer's scheme to be
present in that offer, and authorizes that exact wire identifier through a
caller-supplied data-driven repository policy. The non-cloneable result retains
the pending pre-message transcript snapshot, decoded message, exact offer, and
authorized RSA-PSS scheme. An empty signature remains structurally valid at
this boundary and must fail later cryptographic verification.

This package derives one direction's handshake key and IV, creates matching
record state, protects exact retry-handshake records in caller-owned buffers,
enforces the public no-PSK server-flight type order, joins fresh ordered state
to exact retry transcript and Finished-key provenance, and advances that
transcript only after explicit caller authorization of authenticated ordered
messages. It provides the bounded `EncryptedExtensions` structure and retained
offer/context check and the bounded main-handshake `CertificateRequest` checks
and server `Certificate` framing and entry-policy checks above, plus exact
server `CertificateVerify` framing, offer membership, and algorithm policy; the
authorization bit is otherwise not body validation. This package does not
parse X.509, access credentials, sign, cryptographically authenticate
CertificateVerify or Finished, bind a reviewed leaf key, validate certificate paths,
establish trust, verify hostnames, enforce revocation, perform I/O, authorize
HTTP or OAuth, or provide TLS, HTTPS, or live-provider readiness.

## Verification

```bash
bash BUILD
cargo clippy -p tls13-retry-handshake-traffic-key-material-binding --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-retry-handshake-traffic-key-material-binding --no-deps
```
