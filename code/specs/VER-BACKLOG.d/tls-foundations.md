# TLS foundations backlog

This shard tracks a repository-owned, auditable TLS 1.3 implementation path
across the repository's 18 language lanes. Its 18 `TLS-Fxx` rows below are
protocol milestones, not language lanes. This is a planning and gap record,
not an implementation or an assurance claim.
The existing [`tls-platform`](../tls-platform.md) contract deliberately delegates
cryptography and certificate verification to operating-system providers; this
backlog describes a separate, future Rust implementation that could eventually
sit behind a transport seam. It does not change that platform contract or
authorize production use.

## Protocol and evidence policy

Use [RFC 9846](https://www.rfc-editor.org/rfc/rfc9846.html) as the current TLS
1.3 specification baseline; it obsoletes
[RFC 8446](https://www.rfc-editor.org/rfc/rfc8446.html). Record the exact
edition and relevant sections in each future spec and fixture manifest. RFC
8446 may be cited for historical context or when an RFC
9846 section explicitly references it, but must not silently remain the
normative baseline. Every future profile/spec must include an RFC 9846 §9
compliance table. In the absence of a published application profile that says
otherwise, §9.1 requires `TLS_AES_128_GCM_SHA256`, signature support for
`rsa_pkcs1_sha256` (certificates), `rsa_pss_rsae_sha256` (CertificateVerify
and certificates), and `ecdsa_secp256r1_sha256`, and `secp256r1` key exchange;
it recommends `TLS_AES_256_GCM_SHA384`, `TLS_CHACHA20_POLY1305_SHA256`, and
X25519. Section 9.2 requires support for `supported_versions`, `cookie`,
`signature_algorithms`, `signature_algorithms_cert`, `supported_groups`,
`key_share`, and `server_name`, with conditional send/use requirements for
the applicable features. Deviations from §9 SHOULD recommendations require a
written profile rationale and review. The initial no-PSK pilot below is
deliberately a bounded profile and does not claim a generally TLS-compliant
endpoint. Queue a
server and full declared-profile milestone before any endpoint/compliance
claim; document any application-profile exception rather than silently
omitting an RFC §9 requirement.

Keep [RFC 8448](https://www.rfc-editor.org/rfc/rfc8448.html) example traces as
**illustrative examples with provenance**:
record the RFC section, example/flight, source bytes or exact transcription,
any transformations, and the expected comparison. They are useful for checking
that a stated handshake example still parses and for teaching/debugging. They
are not normative requirements, a complete conformance corpus, or a substitute
for independently specified expected results. Label every 8448-derived fixture
as such; distinguish it from vectors generated from normative algorithms and
from independent implementation/interoperability evidence.

No lane may claim constant-time behavior from source inspection alone. Each
cryptographic implementation needs an explicit leakage/threat boundary, review
of compiler and target assumptions, and a concrete validation plan. A passing
fixture proves only the fixture's bounded case. Keep unsupported algorithms,
extensions, groups, certificate forms, and platform behaviors explicit and
fail closed. Do not imply complete TLS 1.3 support until the applicable
conformance and interoperability scope is enumerated and met.

## Milestone contract and status

Each `TLS-Fxx` milestone is a bounded body of work, not a promise that the
entire protocol is implementable in one change. A row that spans distinct
specification, corpus, primitive, and composition changes must be split into
separate row-sized PRs, with the milestone row retained as the parent/receipt
index. `Proposed` means this shard makes no assurance claim. Reconcile actual
packages, evidence, fetched `origin/main`, and open PRs before choosing work.
Every implementation PR's definition of done (DoD) means:

1. a normative, version-pinned spec states input/output bounds, validation,
   errors, security boundary, and non-goals;
2. a machine-readable fixture corpus has provenance per case, valid and invalid
   controls, and an independent oracle or derivation; fixture runner behavior
   is deterministic and fails closed;
3. local tests cover boundary values, malformed encodings, and regression
   cases, and CI runs the canonical corpus; reported test evidence names the
   exact revision and commands;
4. security review addresses secret lifetime, zeroization where applicable,
   timing/leakage assumptions, parser/resource bounds, and unsafe/FFI boundary;
5. remaining unsupported behavior and evidence limits are recorded. No
   conformance claim exceeds the checked corpus and target matrix.

## Language-lane evidence rollout

VER04i defines the lanes as `rust`, `python`, `typescript`, `go`, `ruby`, `c`,
`cpp`, `java`, `kotlin`, `csharp`, `fsharp`, `elixir`, `lua`, `perl`,
`haskell`, `swift`, `dart`, and `ocaml`. These are 18 independent evidence
consumers. The initial implementation/protocol pilot remains Rust-only; every
other lane is a rollout item, not an inferred port. Use VER30's single canonical
corpus/generator and lane-specific runners/manifests rather than maintaining 18
independent fixture generators.

| Rollout stage | Per-lane acceptance and honest label |
|---|---|
| A. Portable, required before lane promotion | The real exported TLS primitive/component is called through its lane binding against the canonical corpus; include canonical finite cases, malformed/negative controls, and sabotage proving the runner detects a wrong result. Re-derive expected outputs with an independent oracle outside the implementation under test. Write manifests from the actual run and verify them with the canonical report/check path. This proves only the enumerated behavior. |
| B. Native, after A | Run that lane's PBT/fuzz engines against the same model and canonical `pbt-v1`/`fuzz-v1` controls where those engines are available. Record seeds, bounds and manifests. This is sampled evidence, not exhaustiveness. |
| C. Proof, only when toolchain supports it | Use the real lane source-to-WIR lowering and WIR proof pipeline. Never hand-author or copy labels. Until a claim has the corresponding generated certificate, retain the actual non-Rust `unchecked` label. Do not call a sampled or finite corpus run `bounded(N)`; VER31 reserves `bounded(N)` for a checked certificate over every input through N. `exhaustive` requires the full finite domain and the qualifying report fields, and is currently unreachable through the canonical report gate. Use `proved`, `proved-to-isa`, or `proved-to-gate-level` only when the VER31 gate validates the exact certificate/evidence chain and required assumptions. VER04i currently says non-Rust proof labels are unchecked. |

The 18-lane rollout order is Rust pilot, then one portable lane at a time using
the VER04i conformance kit, then that lane's native rung, then proof only when
its WIR frontend exists. F02's Python stdlib oracle replay is distinct from its
Python public encoder implementation port; only the latter can count as a
Python portable-lane API check. Track lane states as `not inventoried`,
`planned`, `portable`, `native`, or `proof` with linked receipts, never as a
TLS-support boolean. Initial inventory: Rust has existing implementation
packages but requires evidence reconciliation; the other 17 lanes are
`not inventoried` for TLS support until their real package/API and evidence are
checked.

## Existing Rust inventory and platform assumptions

Fetched `origin/main` already contains many narrowly scoped `tls13-*` Rust
packages, including `tls13-hkdf-label`, record/inner-plaintext/nonce and record
protection, key schedule, handshake/message components, X25519, AES-GCM,
ChaCha20-Poly1305, and a no-PSK server-flight path. This inventory is a
discovery aid, not an acceptance receipt. The initial F17a client composition
remains queued; the existing server-flight package is not itself an end-to-end
handshake receipt. Before changing any milestone, map
existing package specs, fixture corpora, tests, open PRs, and their limits into
the row; preserve independent package boundaries and do not rewrite existing
APIs to fit this roadmap. In particular, F01 closes specification/corpus/
independent-evidence gaps around the existing encoder; it does not add another
encoder, a decoder, or a public API.

The current `tls-platform.md` describes Schannel on Windows 11/Server 2022,
Network.framework on macOS, and OpenSSL FFI on Linux. These are design-level
provider baselines only: supported macOS/Linux release minima and exact
provider versions, exact compiler/toolchain versions,
OpenSSL version/configuration, CPU/ISA assumptions, timing properties, root
store behavior, clock policy, and CI target matrix remain to be pinned by
evidence before any platform claim. The Rust in-memory pilot does not inherit
these provider claims.

## Composition gates and run-loop receipts

VER04i controls finite evidence and labels per lane; do not turn a corpus pass
into a timing or universal proof claim. VER29 applies only when one component
assumes another component's published claim: pin the lower manifest/claim and
digest, check exact operation coverage and failure paths, and keep unresolved
links visible. VER30 supplies the per-language rollout and inventory discipline;
do not create a parallel TLS evidence registry. VER31 is the release-tier
computation and release gate: a TLS release needs its computed report, declared
assumptions, exact reviewed bytes and all applicable floors. No publishing is
authorized here, and VER00 §12.1 still blocks approval machinery. VER33's ADL
algorithm profile remains a draft pending approval; do not treat its proof
requirements as live before that approval.

For each row-sized PR and after each merge, follow the central [VER delivery
and prioritization run loop](../VER-BACKLOG.md#delivery-and-prioritization):
reconcile fetched `origin/main` and open PRs; update this row with the merged
commit, exact commands/CI run or evidence receipt, what it does not establish,
and the next dependency. Preserve failed runs and unresolved assumptions; do
not replace them with a successful retry without recording both.

| Milestone | Bounded work | Prerequisites | Definition of done / evidence boundary | Status |
|---|---|---|---|---|
| TLS-F01 | Specify and evidence the existing Rust `tls13-hkdf-label` public encoding | Repository already contains `code/packages/rust/tls13-hkdf-label`; inspect the current API/implementation and RFC 9846 before scoping the gap. This first work documents/tests the existing encoder; it adds no decoder, public API, or replacement encoder. | Specify and fixture only the public `HkdfLabel` wire framing/encoding operation. A caller label is 1..=249 opaque bytes (non-ASCII is allowed); context is 0..=255 bytes and may be empty. The public framing value stores any u16 output length, including 0 and 65535; distinguish that two-byte wire field from the separate HKDF-Expand limit (`255 * HashLen`) enforced by expansion. Include exact bytes, boundary/error and no-mutation controls, and independent encoding derivation. Expansion/output-secret cases belong to F08/F12. Do not add decode/round-trip requirements. | Proposed; first gap closure; implementation exists, evidence/spec status to reconcile |
| TLS-F02 | Second-lane API replay and portable fixture provenance | F01 corpus; Rust API and one additional public language API are needed. | Replay the corpus through the existing Rust public encoder and an actual second-lane public implementation (start with a Python public encoder port); compare both against independently derived expected bytes. Python stdlib may separately replay/reference-check expected fixture records, but label that as oracle/reference replay, not TLS lane implementation evidence. Verify corpus digest/version and stable bounded diagnostics. This proves only the two APIs and cases exercised, not general cross-language TLS parity. | Proposed; depends on F01 |
| TLS-F03 | Fixed-width integer and carry/borrow arithmetic | None; foundational math contract before field/curve code. | Specify supported widths, overflow/underflow, constant-time constraints and checked/unchecked boundaries; exhaustive small-width plus independent big-integer oracle tests. No general arbitrary-precision API or claim about compiler-generated machine code. | Proposed |
| TLS-F04 | Prime-field arithmetic for selected TLS groups | F03; selected group and limb layout explicitly scoped. | Independently checked add/subtract/multiply/reduce/invert/square operations with canonical encoding, zero/noncanonical controls, arithmetic invariants, and target-specific timing review. Does not imply curve correctness. | Proposed; depends on F03 |
| TLS-F05 | Required P-256 and recommended X25519 key exchange | F03-F04; selected implementation groups are explicit. P-256 is mandatory by RFC 9846 §9.1 absent an application-profile exception; X25519 is recommended. | Keep encoding rules distinct. For P-256, enforce the selected SEC1 encoding, canonical coordinates, on-curve and required subgroup/point checks with invalid-point controls. For X25519, implement RFC 7748 decoding/acceptance semantics, including permitted non-canonical u-coordinate inputs and all-zero shared-secret handling at the protocol boundary; do not apply P-256 rejection rules. Test against independent vectors and state exactly which group(s) the profile supports. No generic curve framework implied. | Proposed; depends on F03-F04 |
| TLS-F06 | SHA-256 primitive boundary for TLS | Existing repository implementation inventory and API review. | Pin digest behavior and bounded streaming semantics; run applicable published vectors, chunking equivalence, empty/boundary cases, and negative API tests. Review aliasing/state reset. A hash vector suite is not collision-resistance proof. | Proposed |
| TLS-F07 | HMAC-SHA-256 | F06. | Independent published vectors, key-length and message boundaries, verify/truncation rules where used, and constant-time tag comparison contract. Zeroization and key handling reviewed. | Proposed; depends on F06 |
| TLS-F08 | HKDF Extract/Expand | F07; F01 encoding remains a separate tested serialization dependency for TLS use. | RFC algorithm vectors plus empty/maximum legal input cases; enforce output bound, label separation, and PRK/OKM handling. TLS-specific labels are composed only through F01. Does not establish handshake state correctness. | Proposed; depends on F01, F07 |
| TLS-F09 | AES-GCM AEAD boundary | Existing AES primitive inventory; F03 as needed by implementation. | Published vectors, nonce/tag/AAD/ciphertext boundary cases, invalid-tag rejection with no plaintext release, nonce uniqueness ownership contract, and timing/side-channel review for supported targets. No claim that callers satisfy nonce policy automatically. | Proposed |
| TLS-F10 | ChaCha20-Poly1305 AEAD boundary | Existing primitive inventory; F03 as needed. | Published vectors, counter/length boundaries, invalid-tag rejection with no plaintext release, nonce ownership contract, and target review. Does not imply TLS record integration. | Proposed |
| TLS-F11 | Signature arithmetic/verification on typed keys | F06 and selected hash; typed public-key input is supplied by fixtures, not parsed from X.509. | Specify each scheme's verification encoding and bounds, with valid/invalid signatures, algorithm/key mismatch controls, and an independent oracle. First bounded slice may cover only the no-PSK pilot's certificate-verification schemes. RFC 9846 §9.1 mandatory certificate/CertificateVerify schemes remain tracked as a full-profile requirement. No certificate parsing or signing API is implied. | Proposed; depends on F06 |
| TLS-F12 | TLS 1.3 key schedule and traffic-secret derivation | F01, F07-F08. Accepts an independently computed transcript-hash input; does not own transcript parsing or depend on AEAD. Initial pilot covers no-PSK only; PSK modes, resumption and 0-RTT are later separately scoped work. | Derive no-PSK early/handshake/master/application/exporter secrets from bounded input secrets and supplied transcript-hash bytes, with independently checked fixtures, labels and failure behavior. Keep PSK/resumption/early-data unsupported and fail closed in the pilot. RFC 8448 traces are supplemental. Record-traffic key/IV derivation, sequencing and AEAD remain in F16. No handshake parser/state-machine claim. | Proposed; depends on F01, F07-F08 |
| TLS-F13 | Transcript hash, handshake encodings, and core state transitions | F06; F12 consumes the separately supplied digest input. | Bounded handshake message parser/encoder and transcript updates with canonical lengths, fragmentation-independent transcript bytes, illegal ordering and truncation controls, and state-transition table. Only listed messages/states supported. | Proposed; depends on F06 |
| TLS-F14 | Key shares, supported groups, PSK modes, and negotiation policy | F05, F12-F13; only declared groups and modes. | Negotiation rejects duplicate, unsupported, malformed, or policy-forbidden inputs; test downgrade/selection controls and key-share validation. State extension and PSK identity limits. No resumption or 0-RTT claim unless separately specified. | Proposed; depends on F05, F12-F13 |
| TLS-F15 | Certificate parsing, path validation, and server identity composition | Existing X.509/identity specs and implementation inventory; F11 typed-key verification is complete first. | Explicit DER/resource limits, path-building/trust-anchor policy, validity-time source, name constraints and identity rules; malformed certificate and chain fixtures; compose with `tls-server-identity` without upgrading SAN match to trust. No system-root parity claim absent per-platform evidence. This is a separate integration milestone after F11, so there is no F11/F15 dependency cycle. | Proposed; depends on F11 |
| TLS-F16 | Traffic-key derivation, TLS record protection and alert handling | F09/F10, F12-F13. | Derive record traffic key/IV from traffic secrets; specify sequence/nonce derivation, AEAD associated data, limits/key updates and alert behavior; reject tampering, truncation, overflow, and wrong epoch. No application-data stream claim beyond tested record transitions. | Proposed; depends on F09-F10, F12-F13 |
| TLS-F17 | Client/server handshake composition and platform boundary | F14-F16 and `tls-platform` contract reconciliation. | Split into row-sized child PRs: **F17a**, first no-PSK client handshake over deterministic in-memory transport (pilot only); **F17b**, server flight/peer composition; **F17c**, full declared-profile reconciliation and interoperability. Each child has bounded states/messages, time/resource limits, alerts and close/error behavior. F17a is not RFC-wide compliance; F17b/F17c must show §9 matrix including mandatory items and justified SHOULD deviations before an endpoint claim. No production claim follows from unit composition. | Proposed; depends on F14-F16 |
| TLS-F18 | Wider conformance, differential and interoperability evidence | Declared profile in F17; 18 language rollout; independent implementations and platform matrix selected. | For every supported language lane, inherit VER04i Layer A portable corpus + sabotage controls, Layer B native `pbt-v1`/`fuzz-v1` where the lane has those engines, and Layer C only the actual bounded proof labels/evidence. Apply VER29 contract linking when composed packages assume lower claims; use VER30's per-lane rollout/inventory, not a parallel evidence registry. Compute any release tier with VER31; this backlog authorizes no publication, and VER00 §12.1 remains authoritative. VER33 is still draft: its ADL algorithm-profile gates are not in force until approved; then apply only to algorithms opting into that profile. Record OS/provider/compiler/target baselines and unresolved timing, trust-store, clock, and interoperability assumptions. Passing means only declared-profile conformance on named lanes/targets. | Proposed; depends on declared F17 profile and lane evidence |

## Dependency and prioritization notes

F01 is intentionally first: TLS uses a compact, independently testable encoding
for HKDF labels, and the Rust encoder already exists. Its first PR should
specify and fixture the existing public encoding operation with provenance and
independent byte derivations; add no encoder replacement, decoder, or API. F02
then checks those vectors against a real second-lane public API as well as a
separate independent oracle replay. F03-F11 establish mathematical and
cryptographic leaves; F12-F16 compose secrets, handshake messages, identity,
traffic keys, and records; F17a is the first no-PSK client composition, followed
by server/full-profile children; F18 is the wider conformance phase. Independent
leaves can proceed in parallel only after their chosen implementation and
oracle dependencies are explicitly reconciled.

The current `tls-platform.md` describes OS-backed TLS and is not evidence for
any lane above. `tls-server-identity.md` specifies a narrow already-decoded SAN
matching primitive; it is not certificate parsing, path validation, or chain
trust. Repository cryptography specifications such as
[`msg-crypto-foundation.md`](../msg-crypto-foundation.md) are educational design
material until a TLS lane explicitly maps an implementation and evidence to
the relevant normative requirements. Keep those boundaries visible in future
status updates.
