# Package Parity Roadmap

## Goal

Close package gaps across implementation languages without confusing directory
equality with useful parity. Every deterministic, language-agnostic package
should have a pure implementation in each established language when practical.
Platform, ABI, browser, firmware, and accelerator packages should instead have
honest native implementations or thin tested wrappers.

This roadmap is the durable work queue for the autonomous package-parity PR
loop. It is ordered by leverage: repair the inventory, finish nearly complete
families, then classify and port sparse families in dependency-shaped waves.

## Canonical Inventory

Run:

```sh
python code/scripts/package_parity_report.py --format markdown
python code/scripts/package_parity_report.py --format json
python code/scripts/package_parity_report.py --format csv
```

The reporter reads Git-visible files: tracked files plus untracked files that
are not ignored. It therefore sees newly scaffolded packages without treating
`target`, `node_modules`, `.pytest_cache`, or other ignored build output as
packages.

Package identity is case- and punctuation-insensitive. `directed-graph`,
`directed_graph`, and `Directed.Graph` are one identity. The reporter retains
the original directories and reports collisions when one language contains
multiple directories for the same identity.

### Established implementation lanes

- C#
- Dart
- Elixir
- F#
- Go
- Haskell
- Java
- Kotlin
- Lua
- Perl
- Python
- Ruby
- Rust
- Swift
- TypeScript

### Separately classified lanes

- C, C++, and OCaml are emerging implementation lanes. They need their
  respective package, scaffold, build, security, and CI maturity gates before
  they can join the all-language completion denominator.
- WASM is an execution-target lane, not a requirement for every source package.
- Mosaic and Twig are domain/source-language lanes.
- Starlark is a build/configuration lane.

## July 10, 2026 Baseline

The tracked tree at `8efcb2d5b` contains:

| Metric | Count |
|---|---:|
| Established implementation languages | 15 |
| Tracked package directories in those languages | 4,096 |
| Distinct normalized package identities | 1,102 |
| Package/language slots after identity normalization | 4,094 |
| Missing slots for literal all-language parity | 12,436 |
| Rust identities | 873 |
| Python identities | 494 |
| TypeScript identities | 436 |
| Rust singletons | 465 |
| Python singletons | 88 |
| TypeScript singletons | 81 |
| Packages present in all 15 languages | 34 |

Rust drift is recent as well as cumulative. From June 10 to July 10, Rust added
127 package directories out of 185 total net additions across the 15 lanes.
Rust-only identities grew from 373 to 465.

The previous April baseline had 377 Rust and 377 Python packages. It is no
longer a useful current planning baseline.

## Parity Classes

Every sparse package must receive one of these classifications before a porting
wave treats it as a gap:

| Class | Meaning | Expected action |
|---|---|---|
| `portable` | Pure algorithm, data structure, IR, codec, validator, deterministic transform, simulator, or grammar frontend | Pure implementation in every established language |
| `native-source` | The package directly owns OS, ABI, GPU, firmware, or hardware behavior | Keep the appropriate native implementation |
| `wrapper` | Thin language-facing binding to a native source of truth | Test the wrapper; do not count it as a missing pure port |
| `web-only` | Browser, DOM, Canvas, IndexedDB, Vite, or Web Audio behavior | Keep in web-capable lanes |
| `target-specific` | Compiler backend or artifact writer meaningful only for a particular target | Port only where the target is supported |
| `not-applicable` | The package has no coherent role in the language lane | Document the exception |

Directory presence is not completion. A completed pure port needs matching API
semantics, shared fixtures or reference vectors, package-native tests, README,
CHANGELOG, metadata, BUILD/BUILD_windows where applicable, and CI coverage.

## Work Inventory

The missing matrix is heavily concentrated in singleton packages. The current
working inventory was regenerated from `de58c11e` after the Elixir resolver
branch rebased onto the latest main. The inventory contains 1,264 normalized
implementation identities across 4,419 established-lane package slots and
found zero canonical collisions or unknown language buckets:

| Current breadth | Packages | Missing slots to all 15 |
|---|---:|---:|
| Present in 10-15 languages | 173 | 270 |
| Present in 5-9 languages | 120 | 905 |
| Present in 2-4 languages | 157 | 1,970 |
| Present in one language | 813 | 11,382 |

The loop must not start by attempting 11,368 singleton ports. It should finish
the broadly established portable core, then classify the sparse majority.
Recently classified mixed Rust identities include `smart-home-camera-media`,
`smart-home-onvif-integration`, `smart-home-shelly-integration`,
`smart-home-wled-integration`, `smart-home-govee-lan-integration`,
`smart-home-lifx-lan-integration`, `smart-home-kasa-lan-integration`,
`smart-home-reolink-integration`, `smart-home-roku-ecp-integration`,
`smart-home-wemo-upnp-integration`, `smart-home-sonos-upnp-integration`,
`smart-home-nanoleaf-local-integration`,
`smart-home-tasmota-local-integration`, and
`smart-home-fronius-local-integration`, plus
`smart-home-homewizard-energy-integration`,
`smart-home-heos-cli-integration`, and
`smart-home-airgradient-local-integration`. All are
mixed splits rather than blind parity ports: camera grant policy,
generation-bound lease state, quotas, and redacted audit are portable, while
authenticated host context and media delivery remain native mediation; ONVIF
discovery/SOAP parsing, deterministic UsernameToken construction, origin policy,
and projection are portable, while sockets, TLS, trusted time/randomness, Vault
access, process I/O, and allowlists remain native; Shelly JSON/RPC normalization,
projection, stable identities, and command planning are portable, while mDNS,
DNS/TCP, plaintext LAN HTTP, trusted time, console I/O, endpoint policy, effect
ordering, future credentials, and capability profiles remain native.
WLED DTO validation, master/segment projection, capability-bit interpretation,
state normalization, and command planning are portable, while mDNS, DNS/TCP,
plaintext LAN HTTP, trusted time, console I/O, pairing/origin policy, runtime
effects, and capability profiles remain native. Govee, LIFX, Kasa, Reolink,
Roku, Wemo, Sonos, Nanoleaf, Tasmota, Fronius, HomeWizard, and HEOS contribute
deterministic codecs, bounded parsers and DTO validation, normalization,
projection, stable identities/errors, command planning, and language-neutral
fixtures to the parity backlog. Wemo specifically contributes SSDP header
parsing, bounded
setup/SOAP XML, service/device
normalization, and switch/light command planning. Sonos additionally contributes
credential-free URL/control-path validation, AVTransport, RenderingControl,
DIDL metadata normalization, deterministic inspection planning, and
protocol-neutral media-player projection. Nanoleaf adds credential syntax and
origin-configuration validation, bounded snapshot/state validation, stable
identity and capability projection, RGB/HSV and mirek conversion, command
planning, and verification. Tasmota adds bounded Status 0 JSON validation,
relay/light/sensor normalization, state and capability projection, command
planning, color conversion, and verification fixtures. Fronius adds bounded
Power Flow and API-status validation, site/inverter measurement normalization,
and deterministic sensor projection. HEOS adds bounded command-result and
response-envelope validation, player/now-playing/volume/mute normalization,
HEOS escaping, stable identities/errors, and deterministic read-only media
projection. AirGradient adds bounded `/measures/current` JSON validation,
firmware/model/sensor normalization, environmental measurement identities and
units, stable errors, and deterministic read-only sensor projection. UDP
multicast, DNS/TCP, LAN HTTP and HEOS TCP execution,
timeouts, endpoint approval, CLI I/O, authorization, and runtime mutation remain
native-host responsibilities.

The same refresh added `chief-of-staff-channel-crypto` and
`venture-browser-qt`. The former has a portable, vector-testable record,
framing, epoch, and cryptographic core around explicit entropy, persistence,
and key-custody boundaries. The latter has a reusable deterministic host-bridge
contract around a Qt/Cairo/C++ native shell. Both now have explicit backlog
owners; neither is treated as an unclassified blind all-language port.

The D18F shared fixture lock now fixes that channel-crypto contract across all
six supported implementations. Rust is the production baseline, while
TypeScript, Python, Go, Ruby, and Elixir are portable consumers: all six
reproduce the exact authenticated header, `D18M` v1 bytes, canonical JSON, rich
payload ciphertext, verification order, and stable errors. One central CI gate
now validates the closed corpus and generator provenance, requires all six
consumers, runs every package-native build, and rejects a regenerated manifest
that differs by even one byte.

The `9bb12864` refresh also added `chief-of-staff-channel-store`. It is an
authority-free orchestration layer over injected storage and cryptography:
two-phase CAS reservation, crash recovery, nonce-safe abandoned gaps,
idempotent encrypted records and grants, ordered paging, acknowledgements,
and stable record errors are portable fixture candidates. Backend I/O,
filesystem durability, key custody, entropy, and actor routing remain injected
or native. D18P now specifies the exact portable record, key, atomicity,
recovery, paging, acknowledgement, and error behavior. Its shared fixture/Rust
compatibility lock consumes the production D18C/D18S/D18H/D18A codecs and
transition paths byte-for-byte. TypeScript, Python, Go, Ruby, and Elixir now
reproduce those records, keys, reserve/recover/abandon/paging/ack transitions,
structural endpoint roles, and the closed error roster over injected atomic
backends. The central D18P gate requires exactly those six consumers, runs
every package-native build, verifies generator provenance, and regenerates the
manifest byte-for-byte. Portable sealed-key generation and rotation remain in
#141 rather than the D18P persistence layer.

The `b3a6616a` refresh added `chief-of-staff-channel-endpoints`, another
zero-capability deterministic layer. Bounded identities, durable membership,
role/key authorization, lifecycle, injected message metadata, delivery-session
receipts, and publish/grant/read/acknowledge orchestration are portable fixture
candidates. Storage I/O, durability, clocks, randomness, key custody, and actor
routing stay injected or native. D18P fixes the structural role and lifecycle
contract while leaving sealed-key generation and rotation to #141. All six
endpoint implementations follow the D18P store fixture lock rather than
inventing parallel storage semantics.

D18Q now specifies the portable channel-key grant boundary that #141 owns. It
keeps the shipped Rust `D18G` v1 record and fixes the X25519/HKDF-SHA256
wrapping key, XChaCha20-Poly1305 AAD, Ed25519 signature input, validation order,
receiver epoch installation, pure rotation plan, stable errors, deterministic
fixture seams, and honest secret-erasure capability reporting. The shared
fixture/Rust lock now fixes every cryptographic intermediate, D18G byte,
validation-order failure, receiver-state transition, and A+B to B-only
rotation. The TypeScript (#11753), Python (#11754), Go (#11755), Ruby (#11756),
and Elixir (#11757) consumers
reproduce that full corpus through repository-owned primitives with immutable
values. They report secret erasure honestly as `best_effort`,
`not_enforceable`, `best_effort`, `best_effort`, and `not_enforceable`,
respectively. The central #11758 gate requires exactly those six consumers,
runs every package-native build, verifies generator provenance, regenerates the
manifest byte-for-byte, and feeds both aggregate CI gates. Crash-safe durable
epoch activation (#11734) remains the explicit follow-up work. D18T (#11781)
now fixes its D18S version 2 upgrade, atomic originator-key custody, immutable
activation plan, public-record replay, publish/activate serialization, crash
recovery, and stable errors. The Rust/shared-fixture slice (#11782), five
portable consumers (#11783 through #11787), and aggregate gate (#11788) remain
the ordered implementation backlog.

The `bce05ed6` refresh added `chief-of-staff-service-registry`, a zero-
capability deterministic registry over an injected `StorageBackend`. Its
bounded versioned host-record codec, identity and lifecycle invariants, stable
keys and ordering, idempotent registration, revision-CAS updates/deletes,
corrupt-record rejection, and restart-recovery plans are portable fixture
candidates. Filesystem durability, process liveness and supervision, trusted
clocks, spawning, effectful reconciliation, and secure-channel handshakes stay
injected or native. Its dedicated backlog owner depends on the shared channel-
crypto identifier vectors and cannot bypass that prerequisite.

The `4a66c66d` refresh added `chief-of-staff-secure-host-channel`, a zero-
capability, transport-independent protocol kernel. Its bounded bootstrap and
hello records, exact `D18F` framing, AAD construction, sequencing, malformed-
frame rejection, and terminal authentication-failure state are portable
fixture candidates. The implementation depends on the Rust-only X3DH, Double
Ratchet, and Vault secure-channel stack, so a separate prerequisite owner first
defines safe vectors, injected entropy and key boundaries, and reviewed lane
exceptions. Process spawning, pipes, sockets, key custody, and supervisor
lifecycle remain native; the host-channel item cannot bypass its primitive
stack dependency.

The `9b18406d` refresh added `chief-of-staff-service-reconciler`, a zero-
capability deterministic bridge from the injected service registry to an
injected supervisor. Desired/live/hash/heartbeat observations, restart policy,
one-mutation-per-tick planning, CAS transition claims, conflict handling,
stale-heartbeat classification, and crash convergence are portable fixture
candidates. Process liveness, spawning and stopping, package hashing, trusted
clocks, filesystem access, and supervisor authority remain injected or native.
Its dedicated backlog owner depends on the registry and channel-identifier
contracts, so it cannot bypass those prerequisites.

The `fbd52e79` refresh added `chief-of-staff-host-control-protocol`, a zero-
capability authenticated lifecycle protocol over complete secure-host-channel
frames. `Ready`, `Heartbeat`, and `Terminate` records, role and direction
enforcement, lifecycle ordering, replay and package-hash mismatch rejection,
caller-supplied trusted receipt times, and terminal malformed-input behavior
are portable fixture candidates. The later host-data-plane extension adds
bounded receive/publish/acknowledge and provider-neutral completion records,
monotonic one-in-flight request correlation, canonical UUID validation, and
redacted failures to that same portable kernel. Clocks, descriptors, streams,
processes, filesystems, networks, package verification, channel/LLM effects,
and supervisor authority remain injected or native. Its dedicated backlog owner
depends on the secure-host-channel contract and therefore remains dependency-
blocked.

The `9ad8105f` refresh added `chief-of-staff-process-supervisor`. This crate is
the concrete native authority behind the portable reconciler and host-control
contracts: it reads and re-verifies signed package files, spawns, signals, owns,
and reaps child processes, transports bootstrap records over pipes, and samples
and sleeps against monotonic time. Its deterministic protocol seams already
belong to the service-reconciler, host-control, and secure-channel backlog
owners. A dedicated dependency-blocked review now owns the native-lane
exceptions; the parity loop must not manufacture unsafe process-control ports.

The `481a30c3` refresh adds `chief-of-staff-orchestrator-core` and
`venture-browser-cairo`. The Chief core is a zero-capability, transport-
independent coordinator over injected storage, supervision, authorization,
and monotonic time. Stable host lifecycle, bounded reconciliation, health,
clock regression, safe deregistration, authorize-before-mutate channel wiring,
idempotency, and payload-blind errors now have a dependency-shaped portable
owner. The Cairo crate centralizes a native RGBA renderer and unsafe Qt,
Flutter, and Compose C ABIs around `venture-browser-core`; its reusable event,
navigation, scrolling, hover, link, and projection behavior joins the existing
Venture bridge owner, while Cairo rendering, pointers, buffers, dynamic
libraries, and toolkit launch remain native exceptions.

The `3258981f` refresh adds `websocket-core`. Its spec, empty capability
manifest, and implementation all define a bounded, transport-independent RFC
6455 state machine: handshake validation, accept derivation, frame masking and
canonical lengths, incremental decoding, fragmentation, UTF-8, ping/pong, and
close semantics are portable. The selected first child now provides a closed
Draft 2020-12 schema and 39 shared cases across all seven API families, with
Rust consuming the fixture records directly as the reference implementation.
The package passes its 23 unit tests, 7 shared-fixture tests, package-local
format/lint/docs gates, and 99.81% line coverage. The real repository build
plan selects and builds the complete 13-package prerequisite/downstream closure
through `websocket-runtime`. Current Rust 1.97 exposed three pre-existing
Windows-target lint/format points in `iocp`, `transport-platform`, and
`tcp-runtime`; narrowly documented platform contracts and mechanical
clamp/formatting updates make the full closure clean without changing behavior.
The full validator adds no WebSocket finding to its existing 73-gap debt gate.
A second dependency-blocked child expands the core through every established
lane after shared HTTP framing and the Java/Kotlin/Dart HTTP prerequisites
land. Sockets, DNS, TLS, clocks, random mask-key generation, event loops,
retries, and application dispatch remain native runtime-adapter concerns.

The `c9e4cb2a` refresh adds `websocket-runtime`, a concrete TCP adapter over
`tcp-client`, `tcp-runtime`, `transport-platform`, OS entropy, and the portable
WebSocket core. Its DNS, connect, listen, entropy, stream, loopback, and
platform behavior is host-native and must not enter the all-language portable
denominator. A dedicated review item owns the minimum truthful capability
profile, platform-CI evidence, and the guard that deterministic RFC 6455 logic
stays in `websocket-core`; policy excludes that native-runtime review from
autonomous selection.

The `479ba6d7` refresh adds `chief-of-staff-daemon-api` and
`smart-home-axis-vapix-integration`. The Chief daemon API couples bounded JSON
protocol handling to authenticated host-lifecycle operations and an optional
concrete WebSocket listener, so its minimum listener, stream, authentication,
and host-control authority requires a native-runtime review while reusable
DTO, version, duplicate-field, stable-error, and redaction semantics remain in
portable injected contracts. The Axis adapter performs mDNS discovery,
credential-referenced HTTPS, TLS, endpoint policy, runtime authorization, and
concrete network effects; its bounded VAPIX validation, request planning,
identity normalization, and stable projection remain portable seams, while the
residual adapter is a reviewed native-runtime exception. Both review items are
excluded from autonomous delivery selection.

The `3445da42` refresh adds `chief-of-staff-cli-core` and a Rust
`semantic-ir-to-c` implementation. The CLI core is zero-capability and keeps
credentials, sockets, terminal input, and process policy outside an injected
authenticated client. Its declarative command grammar, conservative defaults,
typed host/path/hash validation, dispatch records, stable errors, and
deterministic JSON rendering are portable fixture candidates. A dedicated
dependency-shaped owner now waits on the service-registry portable contract
and daemon authority split. The additional SIR implementation fills an
existing identity slot and creates no new package identity or exception.

The `8eafdc14` refresh adds `chief-of-staff-daemon-runtime`, a concrete
authenticated listener and reconciliation scheduler with declared
`net:listen`, `time:read`, and `time:sleep` authority. Its continuous serving,
interruptible waiting, and stop-on-failure behavior remain a reviewed native
runtime exception; bounded tick ordering stays available as an injected
portable seam. The dedicated review is excluded from autonomous delivery.
The same refresh merged Haskell strict rockspec decoding and directly unlocked
the binary-safe source-hashing child. PR #9756 completed that cache-boundary
repair and lets the exact `lua/logic_gates` to `lua/arithmetic` closure dry-run
successfully. The full 258-package Lua lane then reached a separate all-node
cycle failure in Haskell's whole-manifest token resolver. PR #9777 repaired the
Lua reader: it distinguishes genuine cycles from false alias matches, parses
only authoritative rockspec dependency fields, preserves BUILD-declared
program edges and identities, and matches the Go oracle exactly. That repair
exposed the same generic-token debt in the real 205-package Haskell lane. PR
#9806 completed the dependency-shaped Cabal slice: it replaced 43 false edges
from non-authoritative manifest text with `build-depends` parsing and now
matches all 486 canonical edges. The next leverage audit selected Python
because its 488 real packages collapsed into an all-node cycle, with 1,492
Haskell edges versus 1,118 canonical edges, 383 false edges, and nine missing
PEP 503-normalized edges. PR #9817 completed that slice by reading only PEP 621
`[project].dependencies` and applying PEP 503 distribution-name normalization;
the real lane now completes without failures and its 1,118-edge graph matches
the Go oracle exactly. The new post-merge audit selects Rust because it is the
largest remaining clean repair: 945 packages, 3,201 Haskell edges versus 2,353
canonical edges, 848 false edges, and zero missing edges. A dependent backlog
owner carries the remaining post-Rust readers. The repair reads only inline
path entries from Cargo's top-level `[dependencies]` table. Full-graph
validation also exposed valid renamed dependencies missing from the Go oracle,
so the shared contract and both engines now honor Cargo `package` overrides.
On the rebased tree, both engines discover 948 packages and match exactly at
2,373 edges; the Haskell front door completes the lane with zero failures.
Guarded squash auto-completion merged exact-head PR #9832 after all 17 checks
were terminal and two consecutive mergeability readings were clean. The
post-merge resolver audit selects Ruby next: its 301-package lane initially
has 543 Haskell edges versus 420 canonical Go edges, with 123 apparent false
edges and zero apparent missing edges. Corpus inspection separates six valid
`add_runtime_dependency` declarations and 28 valid declared-gem-name aliases
from generic-token noise. The shared Ruby contract and both engines now accept
the runtime dependency synonyms, exclude development dependencies, metadata,
comments, and unrelated text, and register manifest-declared aliases. Both
engines now match exactly at 301 packages and 454 edges, with zero one-sided
edges. TypeScript remains larger but is not a clean field-boundary
slice: 469 packages have 147 false and 948 missing edges, and the canonical
audit also encounters separately owned `BUILD_windows` prerequisite debt.
Ruby therefore removes the largest remaining clean false-cycle risk while
repairing its oracle without mixing unrelated manifest grammars or build-file
remediation. Guarded PR #9846 then merged exact head `38c449d322` after all 17
checks were terminal and two consecutive mergeability readings were clean. The
post-Ruby leverage pass selects Perl next: its 256-package lane retains 95 false
and two missing edges, ahead of Swift's 65 false-only edges. TypeScript remains
larger at 147 false and 948 missing edges, but still crosses much broader
manifest semantics and separately owned `BUILD_windows` debt. Perl's 245 root
cpanfiles and 247 root `Makefile.PL` files expose one coherent boundary: runtime
`requires` declarations and authoritative internal distribution/module aliases
must drive graph edges, while test phases and package metadata must not. The
shared repair removes nine internal test-phase declarations, registers exact
module names plus current and legacy distribution aliases, and leaves Haskell
and Go identical across all 256 packages and 217 total edges: 216 authoritative
manifest edges plus one qualified BUILD dependency, with zero one-sided edges.
The rebased `6d206c3c` collision-checked inventory has 1,260
identities and 4,415 implementation slots across 15 established lanes, with
173 high-consensus packages and 270 missing slots, 810 singleton packages and
11,340 missing slots, 614 Rust singletons, and zero collisions or unknown
buckets. The resolver slice adds the newly authoritative AES prerequisite to
aes-modes, restores HMAC's four authoritative hash prerequisites, and teaches
Linux BUILD validation to admit test-only cpanfile references and their runtime
closure without promoting them into the graph. Generic BUILD paths now match
that Linux validation contract. The Windows validator also makes five existing
Perl standalone gaps concrete in
WASM/compiler packages; the new `build-file-standalone-integrity-perl` child
owns that dependency-shaped wave, so resolver semantics do not absorb broader
build-recipe remediation. The two
unrelated Python `BUILD_windows` prerequisite gaps remain owned by the parent
`build-file-standalone-integrity` item. Java/Kotlin/F# ZIP and Java/Kotlin ZStd fill
existing identities; ZStd
is complete across all 15 lanes and ZIP now spans 13. The Chief daemon keyring,
new Chief daemon credential persistence adapter, Chief daemon composition root,
and Synology Surveillance singleton packages own concrete filesystem or
credentialed network authority and have excluded native-authority review
owners. The new zero-capability `chief-of-staff-daemon-service-files` package
is different: its deterministic launchd, systemd-user, and Task Scheduler
renderers have a new portable-conformance backlog owner, while file writes and
native registration commands remain outside that contract. The sole new
identity in the refreshed inventory, `chief-of-staff-daemon-installer`, owns
secure filesystem publication and native launchctl/systemctl/schtasks execution;
it now has an excluded native-authority review and does not expand the portable
denominator. The two later singleton identities are now classified as well.
`neural-learning-capi` is a Rust/C ABI wrapper around the already portable
neural-learning core, with an excluded wrapper/native-ABI review tied to its
existing language-neutral ABI fixture rather than a fabricated all-language
port campaign. `smart-home-enphase-envoy-integration` is mixed: bounded origin
validation, request planning, HTTP/JSON decoding, meter correlation, identity,
health, telemetry projection, and authorization-before-effect ordering have an
eligible portable-core extraction and conformance owner, while live DNS/TCP/TLS,
bearer-token materialization, Vault/runtime effects, and host mutation have a
separate excluded native-authority review. The previously
discovered `chief-of-staff-cli` singleton is the concrete D18 composition root
for environment and path discovery, owner-only credential loading, loopback
WebSocket authentication, terminal output, native service publication, and
supervisor execution. Its deterministic command grammar, typed dispatch,
result rendering, install planning, and service-file rendering already have
portable owners; the residual executable now has an excluded native-authority
review rather than expanding the all-language denominator. The previously
discovered `chief-of-staff-skill-parser` singleton is zero-capability
deterministic parsing, validation, manifest generation, and permission
planning over caller-provided text. It now has a portable-conformance owner for
frontmatter, CommonMark structure, capability taxonomy, stable diagnostics,
canonical manifest JSON, and sorted Deno flags; runtime launch and host approval
remain outside that contract. The previously
discovered singleton,
`smart-home-blue-iris-integration`, is a concrete TCP/TLS and credentialed NVR
adapter; the state backlog records an excluded native-authority review instead
of treating that host integration as an all-language portability gap. The new
`smart-home-frigate-integration` singleton likewise owns concrete TCP/TLS,
authenticated HTTPS, JWT cookies, Vault identity, and runtime authorization;
it has an excluded native-authority review rather than a fabricated portable
lane gap. The new `process-shutdown` and
`smart-home-unifi-network-integration` singletons likewise own native process
signal/FFI authority or concrete authenticated LAN/TLS authority. Both have
excluded native-authority review owners and do not displace the active portable
resolver repair. The same rebase adds three classified singleton identities.
The zero-capability Rust `chief-of-staff-skill-runtime` coordinator and the
authority-free TypeScript `chief-of-staff-sdk` now have dependency-ordered
portable-conformance owners over injected channel, LLM, and JSON-line
interfaces. The Rust `smart-home-zoneminder-integration` package owns concrete
credentialed HTTPS login, short-lived JWT custody, TLS, and runtime effects, so
it has an excluded native-authority review rather than an artificial
all-language port target. None broadens or displaces the completed Perl
field-boundary slice.

The latest rebase adds one C# ZIP implementation to an established portable
identity and two classified Rust singleton identities. The zero-capability
`chief-of-staff-agent-stdio-protocol` codec now has a portable conformance owner
for its bounded JSON-lines framing, canonical fields, base64, and exact response
correlation. The authority-free `http-digest-auth` parser and authorization
builder now has a separate security-vector owner for bounded RFC 7616 parsing,
MD5/SHA-256 families, qop handling, escaping, rejection, and secret-lifetime
behavior. Network I/O, credentials, counters, retries, and client-nonce entropy
remain injected or native. These dependency-shaped owners do not displace the
active field-aware Perl resolver repair.

Guarded squash auto-completion merged the Perl resolver PR #9863 at exact head
`aeff21f74a` as `3d2234a287` after all exact-head checks were terminal and two
consecutive mergeability readings were clean. The post-merge resolver audit
selects Swift next as the largest remaining clean false-only slice: both engines
discover 164 packages, but generic Haskell tokenization initially emits 244
edges against Go's 179, with 65 Haskell-only edges and zero Go-only edges.
Every extra edge comes from comments or unrelated manifest strings. The shared
Swift contract now admits only relative `.package(path: "...")` declarations;
the new language-neutral fixture also exposes and repairs a Go oracle defect
where a block-commented declaration was previously accepted. C# and F# retain
47/178 and 39/178 false/missing drift, TypeScript retains 148/948, Java and
Kotlin retain 4/24 and 9/13, Dart is missing 67, and Go and Elixir have smaller
clean false-only deltas. Swift therefore removes the highest coherent
false-cycle risk without mixing manifest grammars or separately owned BUILD
debt.

The same collision-clean refresh adds two classified Rust singleton identities.
`chief-of-staff-agent-manifest` is an authority-free strict parser, validator,
and deterministic serializer over caller-provided JSON, so it has a new
portable-conformance owner. `chief-of-staff-agent-stdio-host` owns concrete
subprocess launch, piped I/O, signaling, and channel coordination; it has an
excluded native-authority review ordered after the existing stdio protocol,
channel crypto, and endpoint contracts. Neither discovery displaces the
selected Swift resolver slice.

The post-rebase collision-clean refresh adds one further classified Rust
singleton. `chief-of-staff-agent-discovery` mixes an authority-free reducer
with concrete filesystem and signature-verification authority. The backlog
therefore splits it into a portable snapshot/projection conformance owner,
ordered after the manifest and service-registry contracts, and an excluded
native-authority review for directory enumeration, canonicalization, sealed
file reads, keyring verification, and host-path policy. The quick leverage pass
keeps the already implemented Swift resolver repair first; neither dependent
discovery item is eligible to displace it.

Guarded squash auto-completion merged the Swift resolver PR #9878 at exact head
`eb36800f9b` as `4ad79aa054` after all 17 exact-head checks were terminal and
two consecutive mergeability readings were clean. The full post-merge resolver
audit confirms Swift at 164 packages and 179 edges on both engines with zero
drift, then selects Go as the largest remaining clean false-only slice: 302
packages, 962 Haskell edges versus 936 canonical Go edges, 26 Haskell-only
edges, and zero Go-only edges. Elixir retains 8/0; C# and F# retain 47/178 and
39/178; TypeScript retains 148/948; Java and Kotlin retain 4/24 and 9/13; and
Dart retains 0/67. Go therefore removes the highest coherent false-cycle risk
without mixing alias discovery, missing-edge repair, .NET/JVM manifest
semantics, or separately owned BUILD debt. The intervening catalog-reload,
JavaScript shift, and Tamil commits add no package identity, collision, or
priority change.

The later `02626bd6` refresh is also identity-neutral and collision-clean. Its
Chief changes expand the existing agent-manifest and skill-parser portable
owners with schema-v2 channel-version maps, deterministic serialization,
complete positive version validation, and fail-closed writer/reader
compatibility. They require no new backlog identity and do not displace the
already implemented Go resolver repair.

Guarded squash auto-completion merged the Go resolver PR #9884 at exact head
`102936be9e` as `582b15bc38` after all 17 checks were terminal and two clean
mergeability readings. The post-merge schema-3 inventory remains unchanged and
collision-free. Exact comparison confirms the Go lane at 302 packages and 936
edges on both engines with zero drift, then selects Elixir as the smallest
coherent remaining resolver slice: 282 packages, 470 Go edges, 478 Haskell
edges, eight Haskell-only differences, and zero Go-only differences. Two valid
multiline local path declarations are missed by Go, while six comment/prose
references are falsely promoted by Haskell's generic whole-manifest scanner.
A shared field-boundary fixture and paired Elixir readers can repair the whole
slice without mixing TypeScript, .NET, JVM, Dart, standalone BUILD debt,
rockspec UTF-8, or native NIF authority.

The implemented field readers preserve direct project dependency lists and
block or shorthand dependency functions, exclude comments and unrelated
metadata, and converge at 472 edges on both engines across all 282 Elixir
packages. Both real front doors also complete BUILD validation for the full
lane with zero failures.

While the Elixir repair was validating, `de58c11e` added the Rust-only
`chief-of-staff-skill-package` identity. The backlog splits its mixed boundary:
the authority-free loader over caller-provided authenticated bytes has a
portable verification-projection owner after the skill-parser and discovery
contracts, while create-new directory mutation, package signing, secret-key
custody, cleanup, and host path policy have an excluded native-authority
review. This new downstream pair does not displace the already implemented and
fully validated Elixir resolver repair. The adjacent Latin fixes are
package-identity neutral.

Guarded squash auto-completion merged the Elixir resolver PR #9888 at exact
head `a610d247f1` as `9106e9ce78` after all 17 exact-head checks were terminal
and two consecutive mergeability readings were `MERGEABLE/CLEAN`. The later
Spanish-book commit `6800d09448` is package-identity and resolver neutral. The
refreshed schema-3 inventory therefore remains collision-free at 1,264
implementation identities and 4,419 established-lane slots, with 173
high-consensus packages and 270 missing slots, 814 singleton packages and
11,396 missing slots, 618 Rust singletons, and zero unknown buckets.

The complete post-Elixir resolver audit confirms exact Go/Haskell agreement for
Elixir, Go, Haskell, Lua, Perl, Python, Ruby, Rust, and Swift. The remaining
two-sided deltas are C# 178/47, F# 178/39, TypeScript 948/148, Java 24/4, and
Kotlin 13/9 (Go-only/Haskell-only). Dart is one smaller root cause: Go
discovers 82 packages and 67 edges while Haskell discovers none. The loop
selects first-class Haskell Dart discovery plus one field-aware `pubspec.yaml`
reader because that coherent change closes both the 82-package discovery gap
and all 67 resolver edges without mixing .NET, JVM, TypeScript, standalone
BUILD, or strict rockspec UTF-8 debt. A post-Dart audit remains queued to
reprioritize those independent grammars.

The implemented contract admits only direct package keys under root
`dependencies:` and `dev_dependencies:` maps. Its fixture first exposes a Go
false edge from a nested Git source option and zero Haskell discovery, then the
paired bounded readers converge the complete real lane at 82 packages and 67
edges with zero drift. Both real front doors validate all 82 packages. Full Go
test, coverage, vet, build, and module verification; Haskell package suites,
warning-clean compilation, Cabal checks, Haddock, and HPC coverage; the 50-case
153-file shared corpus; schema/runner suites; diff, dependency, and direct
security checks all pass. The parser reads only `pubspec.yaml` text and never
follows source paths or adds execution, network, credential, or host authority.

Guarded squash auto-completion merged the Dart resolver PR #9892 at exact head
`2b8f898d5b` as `5831540244` after all 17 exact-head checks were terminal and
two consecutive mergeability readings were `MERGEABLE/CLEAN`. The refreshed
schema-3 inventory is unchanged and collision-free at 1,264 implementation
identities and 4,419 established-lane slots, with 173 high-consensus packages
and 270 missing slots, 814 singleton packages and 11,396 missing slots, 618
Rust singletons, and zero unknown buckets.

The exact post-Dart resolver audit selects the shared JVM Gradle grammar as the
smallest coherent remaining two-sided repair. Go and Haskell both discover 129
Java packages, but emit 185 and 165 edges respectively, leaving 24 Go-only and
four Haskell-only edges. They both discover 133 Kotlin packages, but emit 165
and 161 edges, leaving 13 Go-only and nine Haskell-only edges. Java and Kotlin
use the same `settings.gradle.kts` composite-build declarations, so one bounded
field-aware contract and adversarial fixture can close both lanes without
mixing ecosystems. The larger independent deltas remain separately owned:
C# has 178 Go-only and 47 Haskell-only edges, F# has 178 and 39, and TypeScript
has 948 and 148. Those .NET and TypeScript grammars remain pending after the
Gradle tranche rather than widening the active PR.

The implemented Gradle contract accepts only actual quoted relative
`includeBuild` calls in root `settings.gradle.kts`, normalizes them lexically
against the declaring package, and matches exact discovered package roots in
the same language scope without following the target. Paired Java and Kotlin
fixtures reject comments, example strings, unrelated calls, absolute and
unknown targets, and build-script coordinates. They first exposed false and
missing edges in both engines. The repaired real graphs now match exactly at
129 Java packages and 186 edges and 133 Kotlin packages and 166 edges, with
zero one-sided drift. Both real front doors validate every BUILD file and dry
run both lanes with zero failures. The newly recovered
`java/conduit -> java/programs/conduit-hello` and
`kotlin/mosaic-flux-compose -> kotlin/programs/visicalc-compose` closures pass
their exact Cargo/Gradle package and downstream commands. Gradle metadata and
Java/Kotlin sources now also participate in hashes, closing a cache-invalidation
gap found during the resolver audit. Full Go and Haskell test, coverage,
warning, build, package, documentation, conformance, parity, state, diff, and
security gates pass. The bounded scanners read only the selected settings text,
do not follow referenced paths or execute content, and add no process, network,
credential, native-runtime, or host-policy authority.

A conflict-free rebase onto `d9f3ed173d` incorporates the package-neutral
human-languages unified-books CI gate. The collision-checked inventory remains
unchanged, so the resolver result and next-item priority do not move.

Guarded squash auto-completion merged the Gradle resolver PR #9895 at exact
head `68ccd82413` as `4b8e56938b` after all 20 exact-head checks were terminal
and two consecutive mergeability readings were `MERGEABLE/CLEAN`. The refreshed
schema-3 inventory is unchanged at 1,264 identities and 4,419 established-lane
slots, with zero collisions and zero unknown buckets; intervening
human-languages work is package-neutral.

The quick dependency/leverage pass therefore selects the already materialized
C#/F# project-reference grammar next. It is one shared XML boundary across 198
C# and 197 F# packages, with 178 Go-only edges in each lane and 47 C# or 39 F#
Haskell-only edges. That coherent two-lane repair remains narrower and better
specified than TypeScript's separate 948-missing/148-false-edge metadata
grammar, so TypeScript stays pending behind it.

The implemented .NET contract accepts only literal quoted `Include`
attributes on unqualified `ProjectReference` start elements in `.csproj` and
`.fsproj` files directly inside the declaring package root. Portable relative
paths normalize lexically and must match an exact discovered root project file
in the shared C#/F#/.NET scope; referenced targets are never opened. Three
shared fixtures cover both language lanes and cross-language references while
rejecting comments, CDATA, processing instructions, escaped examples,
namespaced elements, package references, property expansion, globs, absolute
paths, nested test projects, and unknown targets. The repaired repository
graphs now match edge-for-edge between Go and Haskell at 198 C# packages and
238 edges and 197 F# packages and 239 edges, with zero one-sided drift.

A conflict-free rebase onto `28df5db2b8` incorporates the package-neutral
language-ladder frontier update. The refreshed collision-checked schema-3
inventory is unchanged at 1,264 implementation identities, 4,419 established-
lane slots, 173 high-consensus packages with 270 missing slots, 814 singleton
packages with 11,396 missing slots, 618 Rust singletons, and zero collisions or
unknown buckets, so no new package work is added and the active priority does
not move.

Guarded squash auto-completion merged the .NET resolver PR #9898 at exact head
`c5b0722912` as `e78c9986c2` after all 20 exact-head checks were terminal
acceptable and two consecutive mergeability readings were `MERGEABLE/CLEAN`.
The refreshed collision-checked schema-3 inventory is unchanged at 1,264
implementation identities, 4,419 established-lane slots, 173 high-consensus
packages with 270 missing slots, 814 singleton packages with 11,396 missing
slots, 618 Rust singletons, and zero collisions or unknown buckets.

The post-.NET dependency/leverage pass therefore selects the separately owned
TypeScript package-metadata grammar. A direct resolver-API audit reconfirms
both engines discover 470 packages, while Go emits 1,075 edges and Haskell 275,
leaving 948 Go-only and 148 Haskell-only edges. This is now the largest bounded
remaining established-lane resolver repair. Its package.json name and runtime
dependency tables stay one coherent metadata contract; the 58-package
TypeScript `BUILD_windows` standalone-prerequisite debt reproduced by the real
front door remains independently owned and must not widen this tranche.

The implemented TypeScript contract parses valid root `package.json` objects,
registers only exact top-level `name` aliases, and admits direct keys from root
`dependencies` and `devDependencies` objects. It ignores dependency values,
peer and optional tables, scripts, nested tool objects, nested names, and
malformed JSON rather than inventing a partial graph. The shared adversarial
fixture proves single-line runtime tables, development dependencies, declared
aliases, and every excluded decoy. The repaired real repository graphs now
match edge-for-edge between Go and Haskell at 470 TypeScript packages and
1,076 edges, with zero one-sided drift; the Go field parser also recovers one
real dependency that its previous line scanner missed.

A conflict-free rebase onto package-neutral human-languages main
`658184b112` leaves the refreshed collision-checked schema-3 inventory
unchanged at 1,264 implementation identities, 4,419 established-lane slots,
173 high-consensus packages with 270 missing slots, 814 singleton packages
with 11,396 missing slots, 618 Rust singletons, and zero collisions or unknown
buckets. No new package identity displaces the implemented resolver repair.

The first exact-head CI run exposed one consequence of the corrected graph:
`typescript/rom-bios` became affected, but its Unix and Windows BUILD front
doors did not materialize the five-package standalone closure. The focused
repair adds `transistors -> logic-gates -> arithmetic -> cpu-simulator ->
riscv-simulator` before the package install. Its scoped 7-test coverage suite
and every rerun check pass. Guarded squash auto-completion merged PR #9902 at
repaired head `a7defbbdb5` as `df0692fa76` after all checks were terminal
acceptable and two consecutive mergeability readings were `MERGEABLE/CLEAN`.

The post-merge collision-checked inventory remains unchanged at 1,264
implementation identities, 4,419 established-lane slots, 173 high-consensus
packages with 270 missing slots, 814 singleton packages with 11,396 missing
slots, 618 Rust singletons, and zero collisions or unknown buckets. The quick
dependency/leverage pass selects the shared TypeScript compiler-path boundary
next: it is unblocked and one root configuration affects 132 package/program
configs, so it has broader immediate package-build leverage than a
single-engine follow-up without mixing resolver semantics or the separately
owned 58-package BUILD_windows debt.

PR #9905 made that shared boundary executable with a strict repository audit,
TypeScript 5.5 `${configDir}` paths, and real effective-config validation for
all 129 `rootDir` and 132 `outDir` inheritors. Guarded squash auto-completion
merged exact final head `23302beda2` as `1198eda2eb` after all 19 checks were
terminal acceptable and two independent mergeability readings were
`MERGEABLE/CLEAN`. The refreshed collision-checked inventory remains unchanged
at 1,264 identities and 4,419 slots with zero collisions or unknown buckets.
The next quick dependency/leverage pass selects the separately logged
two-package standalone output-isolation closure: it finishes the TypeScript
compiler-output integrity boundary and stops `window-core` and `window-canvas`
builds from emitting into tracked source trees without widening the 58-package
`BUILD_windows` wave.

PR #9907 closed that two-package gap and merged exact final head `0dfefc77a8`
as `da895587f3` after all 20 checks were terminal acceptable and two independent
mergeability readings were `MERGEABLE/CLEAN`. The refreshed collision-checked
inventory remains unchanged at 1,264 identities, 4,419 slots, 173
high-consensus packages with 270 gaps, 814 singleton packages with 11,396 gaps,
618 Rust singletons, and zero collisions or unknown buckets. A fresh validator
pass decomposes the 58-package TypeScript `BUILD_windows` debt into two exact
children before reprioritization: a 55-package `cli-builder` closure (54 missing
only that prerequisite plus `grammar-tools`, which also lacks `directed-graph`
and `state-machine`) and a separate three-package SIR runtime closure.
The quick dependency/leverage pass selects the 55-package child first because
one common prerequisite layer closes almost the entire TypeScript validator
wave without mixing the SIR dependency family.

Representative validation of that child also exposed a separate compiler-input
gap: algol-lexer inherits the shared src root while TypeScript's default
include admits tests/tokenizer.test.ts, so its optional npm run build fails
with TS6059 even though the authoritative coverage-oriented BUILD_windows
front door passes. The new
typescript-standalone-tsconfig-input-boundaries backlog item owns a repository
audit and dependency-shaped repair; the BUILD prerequisite tranche does not
absorb compiler configuration.

The implemented BUILD closure changes only those 55 Windows overrides. A
minimality audit reconstructs every original file after removing the owned
bootstrap insertion, and the canonical validator falls from the exact
58-package baseline to only the three separately owned SIR diagnostics.
Representative Windows front doors pass 169 grammar-tools tests at 85.85%
statement coverage and 79 algol-lexer tests at 87.5%, with zero production
vulnerabilities. The Go build tool passes test, vet, build, and module
verification; its committed diff plan reports 55 changed and 185 affected
TypeScript packages. The collision-checked parity inventory is unchanged and
still has zero collisions or unknown buckets.

Guarded squash auto-completion merged PR #9909 at exact final head
`8d91b61752` as `4c9d1a6f6b` after all 20 checks were terminal acceptable and
two independent mergeability readings were `MERGEABLE/CLEAN`. The refreshed
collision-checked inventory remains unchanged at 1,264 identities, 4,419
slots, 173 high-consensus packages with 270 gaps, 814 singleton packages with
11,396 gaps, 618 Rust singletons, and zero collisions or unknown buckets. The
quick dependency/leverage pass selects the remaining three-package SIR runtime
prerequisite closure because its exact red baseline is the complete outstanding
TypeScript `BUILD_windows` validator debt. The repository-wide TS6059 input
boundary audit remains explicit and pending until its affected corpus is
classified.

The implemented SIR closure materializes those exact prerequisite orders and
adds a focused repository regression. Clean front-door execution then exposed
same-family strict compiler gaps hidden behind the missing prerequisites: core
and oop now declare the Node typings needed while compiling local core sources,
core's builtin callbacks explicitly accept SIR values, and oop narrows nullable
extremal block keys without changing runtime behavior. The canonical validator
passes all 470 TypeScript packages. Real Windows front doors pass 60 core tests
at 93.2% statement coverage, 120 oop tests at 96.31%, and 23 symbolic tests at
100%; package builds, production audits, and the full Go build-tool gates pass.
The committed diff plan selects exactly the three runtimes and their four
prerequisites, with 463 TypeScript packages skipped. The collision inventory is
unchanged with zero collisions or unknown buckets.

All 20 exact-head checks reached terminal acceptable conclusions for PR #9911;
two independent readings returned `MERGEABLE/CLEAN`; guarded squash
auto-completion merged final head `fdbbe633e0` as `fa9c70094e`. The refreshed
collision-checked inventory remains unchanged at 1,264 identities, 4,419 slots,
173 high-consensus packages with 270 gaps, 814 singleton packages with 11,396
gaps, 618 Rust singletons, and zero collisions or unknown buckets. The input
boundary corpus is now exact: 96 build-script projects have an effective `src`
root, no explicit `include`/`files`/`exclude` boundary, and 202 tracked
TypeScript files outside that root. The quick dependency/leverage pass selects
this one compiler-input contract ahead of the separately pending 10-file Lua
prerequisite wave.

Real validation of the selected input-boundary tranche exposed a distinct
declaration-ownership gap after its TS6059 failure was removed: algol-lexer,
the algol-parser dependency chain, and browser-extension-toolkit reach Node
built-ins or `process`, but do not own a Node type provider. Their coverage
remains green at 79, 33, and 60 tests respectively. The new
`typescript-node-builtin-type-declarations` backlog item owns a corpus audit
and dependency-shaped manifest repair; the 96-config input-boundary tranche
does not absorb package manifests or lockfiles.

The implemented input-boundary contract audits all 458 build-script projects
and reports 422 rooted configs, 420 explicitly bounded rooted configs, zero
unbounded rooted projects, and zero tracked inputs outside their effective
roots. All 96 repairs are semantically minimal `include: ["src"]` additions.
Ten focused Python tests and Ruff pass. Real coverage remains green for the
Algol lexer/parser, arithmetic, browser-extension toolkit, CSV parser, and
canvas measurement packages; arithmetic, CSV, and canvas builds pass from
declared dependencies, and all six production audits report zero
vulnerabilities. The canonical TypeScript validator accepts all 470 packages.
The Go build tool passes test, vet, build, and module verification; its
committed diff plan reports exactly 96 changed and 248 affected TypeScript
packages. The collision-checked inventory remains unchanged at 1,264
identities and 4,419 slots with zero collisions or unknown buckets.

All 20 exact-head checks reached terminal acceptable conclusions for PR #9912;
two independent readings returned `MERGEABLE/CLEAN`; guarded squash
auto-completion merged final head `787e73baab` as `355348901e`. The refreshed
collision-checked inventory remains unchanged at 1,264 identities, 4,419 slots,
173 high-consensus packages with 270 gaps, 814 singleton packages with 11,396
gaps, 618 Rust singletons, and zero collisions or unknown buckets. A quick
direct-ownership scan finds up to 74 build-script TypeScript projects with Node
builtin/global references but no manifest-owned `@types/node` provider. The
dependency/leverage pass selects an executable syntax-aware audit of that
corpus ahead of the separately pending 10-file Lua `BUILD_windows` wave; the
audit must refine the exact set before any manifest or lockfile repair.

The executable syntax-aware audit refines that prioritization ceiling to
exactly 93 Node API build projects: 31 already own direct providers, 62 are
missing them, one of the 31 lacks synchronized lock metadata, and
matrix-rust-napi retains its reviewed native-workspace lock exception. The
optimized scan excludes comments, string and template prose, member properties,
and sources outside effective roots while inspecting nested template
expressions. This exact 62-manifest plus one-stale-lock corpus is the selected
metadata closure.

Real new-lock validation then exposes a separate strict-build owner in
`typescript/starlark-interpreter`: its 119-test Windows front door remains green
at 92.05% statement coverage, while `npm run build` reaches three TS2367
comparisons, one TS2352 cast, one TS2345 return mismatch, and two missing
`VMValue` names. The pending `typescript-starlark-interpreter-strict-build`
item owns those source/type repairs; this metadata-only tranche does not absorb
them.

The dependency-first generic `lattice-docs` BUILD front door also passes all
six tests after its stale lock is materialized. Its optional `tsc -b` reaches
three separate strict diagnostics in compiler-owned dependencies: unused
`LatticeList`, a readonly-child cast in `lattice-ast-to-css`, and unused lexer
`peek`; its Vite production build separately cannot resolve the repository
`lattice.tokens?raw` asset at the current relative path. The pending
`typescript-lattice-docs-strict-build` item owns that exact typecheck and asset
closure without widening the selected lock repair.

The selected tranche's full Go BUILD validator reproduces the exact same 17
diagnostics on clean detached `355348901e`: two Perl, 12 Python, and three Swift,
with no TypeScript diagnostics. The backlog now materializes the remaining
families rather than retaining only umbrella counts: the refreshed two-package
Perl wave; Python CAS/Macsyma, Prolog, SIR runtime, ALGOL WASM, and IRC daemon
closures; and separate Swift Conduit-CAPI and IRC-CAPI closures. This is a
zero-delta baseline discovery and does not widen Node declaration ownership.

The implemented metadata closure adds the direct Node 22 provider to exactly
62 manifests, synchronizes 63 locks including five newly tracked locks, and
preserves every non-provider manifest and lock field. The executable audit now
reports 93/93 providers, zero missing, zero stale, one reviewed exception, and
449 compiler locks. Sixteen focused tests pass with 91% branch-aware audit
coverage; Ruff, Bandit, compileall, JSON, diff, and added-line secret gates are
clean. Real front doors cover lexer/parser, CLI/tooling, server/runtime,
browser-scaffold, new-lock library, LSP, Starlark, and Lattice families; the
seven selected representative builds pass and all nine production audits are
clean. Go test, vet, build, and module verification pass, and the full Go
validator has zero TypeScript diagnostics while matching the clean baseline's
17 unrelated Perl/Python/Swift diagnostics exactly.

After one unrelated curriculum merge, the branch rebases cleanly onto
`00e47db9a4`; the refreshed collision inventory and all contract/Go gates remain
unchanged. The committed TypeScript plan reports exactly 63 changed packages,
236 affected packages, 470 total packages, and 1,076 dependency edges.

Ready-for-review PR #9916 opened at exact validated implementation head
`2cc08914712409f83ed56048829a142072fc541b`; the loop now monitors that one
active parity PR and will not select another item until it merges.

All 20 exact-head checks reached terminal acceptable conclusions for PR #9916;
two independent readings returned `MERGEABLE/CLEAN`; guarded squash
auto-completion merged final head `172bb26f59` as `e634ba0671`. The refreshed
collision-checked inventory remains at 1,264 established identities, 4,419
implementation slots, 173 high-consensus packages with 270 gaps, 814 singleton
packages with 11,396 gaps, 618 Rust singletons, and zero collisions or unknown
buckets. The dependency/leverage pass selects the 10-package Lua
`BUILD_windows` prerequisite wave ahead of four-package Python families and
single-package TypeScript strict-build repairs because it closes the largest
materialized standalone-build family across compiler, serializer, QR, WASM,
and compression front doors.

Before implementing that selected wave, the clean parity branch rebased from
`e634ba0671` onto current `origin/main` at `005c38161e`. The Python build-tool
validator reproduced exactly ten Lua failures: canonical recipes bootstrap
local siblings, but the packages had no `BUILD_windows` standalone recipes.
The implementation adds exactly those ten Windows recipes, preserving every
canonical prerequisite and installing a 67-rock transitive closure in
dependency order with Windows path/redirect syntax, hardened sibling installs,
hardened final self-installs, and the package-specific test commands. Fresh
isolated LuaRocks trees pass all ten real package suites with 351 successes,
zero failures, zero errors, and zero pending tests; representative production
coverage reaches 90.50% for `wasm_module_encoder`. The repository Python
validator is now clean; the full Go validator retains its exact unrelated
17-diagnostic Perl/Python/Swift baseline and reports zero Lua diagnostics.

That audit also materialized a build-tool conformance gap rather than widening
the package repair: Python and Lua validation reject an absent Windows Lua
closure, while Go currently skips the comparison when either recipe is absent.
The pending `build-tool-go-lua-missing-windows-validation-parity` item owns the
absent-override fixture and focused Go validator alignment after this wave.

Fresh-tree validation also showed that several canonical Unix Lua recipes list
only direct sibling rocks even though unpublished transitive local rocks are
required. The pending
`build-file-standalone-integrity-lua-transitive-unix` item owns a complete
canonical-recipe audit and isolated-tree repair after the Windows wave.
Separately, the Python build tool's single global alias table maps 251 of the
258 canonical Lua rock aliases to non-Lua package identities on the current
4,829-package snapshot. The pending
`build-tool-python-ecosystem-scoped-alias-resolution` item owns shared collision
fixtures, ecosystem-scoped ordinary aliases, intentional qualified
cross-language edges, and real-graph comparison with the Go oracle.

After ready-for-review parity PR #10093 merged at `4833046722`, the
collision-checked schema-3 inventory contains 1,264 established implementation
identities and 4,420 package slots across 15 lanes, 173 high-consensus packages
with 269 missing slots, 814 singletons with 11,396 missing slots, 618 Rust
singletons, zero canonical collisions, and zero unknown buckets. The merged
JFET VTOTC work advances the existing shared SPICE parameter-validation owner
without adding an identity. Dart ZIP fills one established high-consensus slot;
new C/C++ Deflate, ZIP, and ZStd implementations remain in emerging lanes and
do not enter the established denominator. This refresh leaves the already
selected Lua Windows closure as the coherent in-progress dependency root; the
new Go-validator, Unix-closure, and Python-alias items remain separately
ordered follow-ups.

After the externally merged SPICE parity sequence through PR #10109, the
collision-checked schema-3 inventory at `1ab517497f` remains unchanged at 1,264
established implementation identities and 4,420 package slots across 15 lanes,
with 173 high-consensus packages and 269 missing slots, 814 singletons and
11,396 missing slots, 618 Rust singletons, zero canonical collisions, and zero
unknown buckets. PRs #10095, #10096, #10097, #10099, #10101, #10103, #10105,
#10106, #10108, and #10109 advance the shared SPICE parameter-validation owner
with TNOM, BEX, BETATCE, XTI, EG, VAF, VAR, NF, NR, and VJE coverage without
adding identities or materializing a new unowned gap.
The dependency/leverage pass therefore retains the already implemented Lua
Windows standalone closure as the sole in-progress root, followed by its
separately owned Go-validator and canonical-Unix closure contracts and the
Python ecosystem-scoped alias repair.

Ready-for-review PR #10110 merged externally as `03f0d5f7a1` on 2026-08-09.
After the subsequent SPICE parser sequence through externally merged PR #10152,
the refreshed collision-checked schema-3 inventory at `7c9bfffbc3` remains
unchanged at 1,264 established implementation identities and 4,420 package
slots across 15 lanes, with 173 high-consensus packages and 269 missing slots,
814 singletons and 11,396 missing slots, 618 Rust singletons, zero canonical
collisions, and zero unknown buckets. PRs #10111, #10113, #10114, #10115,
#10117, #10118, #10119, #10121, #10122, #10123, #10125, #10126, #10127,
#10129, #10130, #10131, #10133, #10134, #10136, #10138, #10140, #10143,
#10144, #10147, #10148, #10149, #10151, and #10152 advance the existing shared
SPICE owner without adding identities or exposing an unowned gap.
Ready-for-review PR #10145 merged externally as `694cfb2822` on 2026-08-09. The
dependency/leverage pass now selects the canonical Lua transitive-Unix closure,
which is the next dependency in the reviewed standalone-build chain, ahead of
the broader Python ecosystem-alias repair. A fresh audit of all 246 Lua
rockspec identities and all 121 canonical recipes that bootstrap sibling rocks
keeps the Brainfuck/IR/WASM chain as the single selected item. It also records
eleven unrelated missing-prerequisite recipes as six pending dependency-shaped
follow-ups: Huffman/heap, parser/lexer, data-store/HyperLogLog, JSON value,
Lattice, and WASM runtime/execution closure.

After the remaining model-card sequence merged externally, the collision-checked
schema-3 inventory at `21326ba062` is still unchanged at 1,264 established
implementation identities and 4,420 package slots across 15 lanes, with 173
high-consensus packages and 269 missing slots, 814 singletons and 11,396 missing
slots, 618 Rust singletons, zero canonical collisions, and zero unknown buckets.
PRs #10155, #10157, #10160, #10162, and #10165 normalize DIODE, NJFET, NJ,
PJFET, and PJ model-card aliases to canonical engine types and complete the
audited diode/JFET alias set without adding an identity or unowned gap. PR #10156
expands the already-owned portable Chief host-control protocol and native process-
supervisor review with an authenticated bounded data plane rather than creating a
new backlog owner. The dependency/leverage pass therefore retains the implemented
Lua transitive-Unix closure as the sole in-progress root ahead of the broader
Python ecosystem-alias repair.

The post-#10190 refresh at `437f19ff06` contains 1,268 established identities
and 4,424 package slots across 15 lanes, with 173 high-consensus packages and
269 missing slots, 818 singletons and 11,452 missing slots, 622 Rust singletons,
zero canonical collisions, and zero unknown buckets. PRs #10166 through #10168,
#10172, #10174, #10176, #10178, #10181, #10182, #10185, #10187, and #10189
extend the existing SPICE owner with model aliases, separator normalization,
finite validation, and family-specific TNOM/T_NOM precedence without adding an
identity. PR #10169 likewise extends the already-owned Chief
host-control, skill-runtime, orchestrator, and native process-supervisor
boundaries with authenticated launch bindings.

PR #10175 introduces `chief-of-staff-pipeline-bindings`, a zero-capability
deterministic reducer over injected storage. Its pending portable-conformance
owner covers bounded versioned records, exact host/package and pipeline/agent
bindings, directional channel membership, immutable cross-pipeline claims,
revision-CAS mutation, restart behavior, and fail-closed revalidation. PR #10180
introduces the sole additional identity, `chief-of-staff-host-data-plane`, an
authority-free dispatcher over injected storage and services. Its new pending
portable owner covers per-request complete-binding checks, direction and AgentId
authorization, exact model settings, response-shape validation, stable redacted
errors, and an explicit empty capability profile. Storage persistence, channel
keys, model providers, process pipes, and daemon composition remain injected or
native.

PR #10173 expands the existing data-governance and AirGradient portable owners
with credential-free MQTT destination validation, exact consent-bound custom
egress, privacy-protective disablement, deterministic HTTP-origin planning,
readback comparison, and redacted projection; live device I/O and mutation remain
native. PR #10184 further expands the data-governance and Enphase portable owners
with exact identifier-retention policy, bounded inverter parsing, injected-key
pseudonymization, and pseudonymous projection while Vault, transport, time, and
runtime effects remain native. PR #10188 adds the sole new identity,
`chief-of-staff-host`, as a concrete child executable over process arguments,
the package working directory, authenticated standard streams, monotonic time,
and bounded sleep. Its dependency-blocked native-authority review records that
exception instead of manufacturing portable executable ports. PRs #10186 and
#10190 add no implementation identity. The dependency/leverage pass retains the
completed Lua transitive-Unix closure as the sole in-progress root because its
dependencies are merged, it
closes four fresh-tree failures, and it unlocks six classified Lua follow-ups.
The portable Chief owners and new native child-host review are dependency-blocked
behind their channel, registry, host-control, supervisor, and skill prerequisites
and do not displace it.

The post-#10210 refresh at `919d683e7a` contains 1,269 established identities
and 4,425 package slots across 15 lanes, with 173 high-consensus packages and
269 missing slots, 819 singletons and 11,466 missing slots, 623 Rust singletons,
zero canonical collisions, and zero unknown buckets. The sole new identity is
`chief-of-staff-daemon-secret-file` from PR #10208. Its explicit filesystem-read
and native-FFI capabilities, no-link handle-relative traversal, owner-only access
checks, exact-length bounded reads, and zeroizing storage make it a reviewed
cross-platform native trust boundary rather than an all-language portable target.
A new excluded native-authority owner records that work and makes the existing
Chief keyring review depend on it.

PRs #10194 and #10203 expand the existing Chief host-data-plane owner with the
real encrypted endpoint service, bounded delivery-to-ack tracking, exact model
selection, and zeroizing pipeline/agent/channel/direction-scoped key release;
secret provisioning, provider construction, cryptography, storage, and transport
remain injected or native. PR #10193 expands the existing Smart Home governance
and UniFi reviews with independently granted bounded presence, official paginated
response handling, host-scoped pseudonyms, five-minute expiry, and redaction while
live TLS, credentials, key custody, and trusted time remain native. PR #10205
adds exact-target operational telemetry with a 64-device bound, one-minute
pre-I/O poll limit, two-minute retention, bounded projection, and deliberate
native-heartbeat omission under those same portable/native owners. PRs #10191,
#10196, #10198, #10200, #10204, #10206, and #10209 extend the existing shared
SPICE owner with canonical MOS alias precedence in both parser facades and engine
normalizers. PR #10210 completes the independent parser/engine model-card mismatch
audit and blocks the sole residual JFET `B` ambiguity on a reviewed meaning-policy
decision. The human-language changes through #10207 add no implementation identity.
None of these changes creates a higher-leverage eligible unowned gap, so the
completed Lua transitive-Unix closure remains the sole in-progress item.

The post-#10212 refresh at `cbe099305e` contains 1,270 established identities
and 4,426 package slots across 15 lanes, with 173 high-consensus packages and
269 missing slots, 820 singletons and 11,480 missing slots, 624 Rust singletons,
zero canonical collisions, and zero unknown buckets. PR #10211 changes only the
existing TypeScript human-language-data package and book artifacts, so it adds
no identity or parity owner. PR #10212 adds the sole new identity,
`chief-of-staff-daemon-authority-provisioning`, as a truthful filesystem-read
boundary that loads exact owner-only 32-byte channel secrets into zeroizing
pipeline/agent/channel/direction authorities and constructs exact bounded Ollama
registries without probing the network. A dependency-blocked native-authority
review owns that package and now gates the existing daemon composition-root
review; the production daemon still deliberately uses its unavailable service
until the provisioned authorities are injected.

The same audit surfaced two previously unowned existing contracts. A new
portable Chief daemon-config owner covers its zero-capability closed TOML parser,
canonical UUID-v7 directional key declarations, bounded unique model selectors,
explicit endpoints and timeouts, caller-supplied-home path resolution, and stable
failures. A separate Ollama native-authority review owns DNS/TCP/local-network
HTTP and socket timeouts while retaining reusable bounded model, endpoint,
timeout, request, and response fixtures. The secret-file and host-data-plane
owners record their exact-length first consumer and deterministic registry
observations. All three additions are either portable backlog work or excluded
native reviews; none displaces the completed Lua tranche, which still closes four
verified fresh-tree failures and unlocks six classified follow-ups.

Ready-for-review PR #10216 merged externally as `a4f5360112` on 2026-08-09.
The post-merge collision-checked schema-3 inventory contains 1,271 established
identities and 4,427 package slots across 15 lanes, with 173 high-consensus
packages and 269 missing slots, 821 singletons and 11,494 missing slots, 625
Rust singletons, zero canonical collisions, and zero unknown buckets. The sole
new identity since `cbe099305e` is Rust
`smart-home-camera-media-http-executor` from PR #10225. It is a concrete pinned
TCP/TLS, credential, CSPRNG, timeout, and bounded-media transport without a
package-local capability manifest, so a dependency-blocked native-authority
review owns it instead of manufacturing all-language executors. Deterministic
camera grant, lease, endpoint, authentication-choice, framing, image-signature,
bound, error, and redaction behavior remains in portable fixture owners.

The same ownership pass classifies every intervening package contract. PR
#10213 receives a portable atomic retained-identity migration owner over
injected storage. PRs #10214 and #10217 expand the existing Chief provisioning,
daemon, host-data-plane, and Level 1 child-host owners with production authority
injection and real receive-complete-publish-acknowledge evidence. PRs #10218 and
#10221 make the Enphase and UniFi owners depend on the retained-identity and
Vault lease contracts; deterministic correspondence and all-or-nothing plans
remain portable while entropy, clock, credentials, transport, and runtime CAS
remain native. PRs #10220, #10224, and #10227 surface separately owned Vault
lease, model-facing tool API, Vault runtime, host-profile and routing, native
host-runtime, SDK binding, and injected skill-store contracts. PRs #10201,
#10215, #10219, #10222, #10223, #10226, and #10228 add no implementation
identity or unowned package gap.

The dependency/leverage pass selects
`build-file-lua-huffman-heap-transitive-closure` next. It is the smallest
unblocked foundational item among the six audited Lua follow-ups: installing
the shared `heap` rock before `huffman-tree` closes clean-tree execution for
`brotli`, `deflate`, and `huffman-compression` in one coherent Unix/Windows
metadata tranche. The parser, data-store, JSON value, Lattice, and WASM runtime
closures remain pending as independent successors.

Before publication, the branch was rebased without conflict through
`94414638fd`. The collision-checked schema-3 inventory there contains 1,272
established identities and 4,428 package slots across 15 lanes, with 173
high-consensus packages and 269 missing slots, 822 singletons and 11,508
missing slots, 626 Rust singletons, zero canonical collisions, and zero unknown
buckets. PRs #10230 and #10234 change only Tamil human-language material. PRs
#10232, #10233, and #10235 expand existing ALGOL and diagram package surfaces
without adding an identity or changing their prior classification. PR #10229
adds the sole new identity, Rust `smart-home-onvif-snapshot-host`: a concrete
SystemTime, OS-CSPRNG, sealed-Vault, transient-credential, camera-media, and
pinned TCP/TLS composition host without a capability manifest. Its
dependency-blocked native-authority review owns that exception while the
camera-media and ONVIF portable owners retain deterministic authorization,
credential-envelope, reference-grammar, request-ordering, cleanup, and
redaction fixtures. This excluded native addition does not displace the
selected Huffman/heap closure.

PR #10238 then merged externally as `ad16d517120` after every required check
completed successfully or was skipped. The collision-checked schema-3 refresh
at `6e0a40b7a6` now contains 1,277 established identities and 4,433 package
slots across 15 lanes, with 173 high-consensus packages and 269 missing slots,
827 singletons and 11,578 missing slots, 631 Rust singletons, zero canonical
collisions, and zero unknown buckets. The exact delta from `94414638fd` is five
new Rust-only identities and no deleted or normalized identities:

- `diagram-layout-sequence` is a deterministic backend-neutral geometry
  package. Its portable-conformance owner covers participant groups and
  lifecycles, messages, notes, activations, nested control frames, ordering,
  geometry, and stable validation failures.
- `mosaic-app-runtime` is an authority-free revisioned protocol state machine.
  Its portable owner covers protocol and startup gates, exact sequence and
  revision progression, transactional failures, snapshot/restore behavior,
  effects, accessibility announcements, and overflow rejection.
- `mosaic-app-capi` is an unsafe panic-contained C ABI and memory-ownership
  bridge over that portable runtime. It has a dependency-blocked native ABI/FFI
  applicability review rather than fabricated all-language C-wrapper ports.
- `smart-home-zoneminder-snapshot-host` and
  `smart-home-synology-snapshot-host` are concrete Human Approval, Vault,
  SystemTime, OS-CSPRNG, credential/session, TCP/TLS, and pinned native-executor
  composition hosts. Separate dependency-blocked reviews own their authority,
  cleanup, redaction, and platform evidence; reusable deterministic request,
  response, authorization-order, and cleanup fixtures stay with portable
  camera-media and vendor integration owners.

The remaining ALGOL, Chief, Mermaid/diagram, Tamil, and Vault changes in this
refresh expand existing package surfaces without adding another identity. The
dependency/leverage pass selects
`build-file-lua-parser-lexer-transitive-closure`: it repairs the lowest shared
`lexer` prerequisite for the independent Dartmouth BASIC and JSON parser front
doors and unlocks the downstream JSON and Lattice closure wave. The existing
WASM runtime/execution follow-up is blocked for re-audit because both current
recipes already install `wasm-execution`; it is not an eligible implementation
gap on this revision.

The final pre-publication fetch advances the collision-checked base to
`1c5ed9c1e9` without changing those counts or the selected dependency shape.
PR #10259 extends the existing Mermaid sequence pipeline and its new layout
owner with participant stereotypes; PR #10256 changes Persian handwriting and
existing human-language surfaces; and PR #10258 expands the existing Rust HTML
parser. The first two stay with existing owners; the HTML change exposes a
pre-existing unowned deterministic frontend, now recorded as
`html-frontend-portable-conformance` for tokenizer handoff, tree construction,
recovery, exact diagnostics, document and fragment projection, and html5lib
fixtures. None adds a package identity or touches the Lua parser recipes, so
the parser/lexer closure remains selected.

A final pre-publication fetch advances the collision-checked base again to
`5e1304f064`: merged PR #10260 adds the sole new identity, authority-free Rust
`vault-pm-format`. The inventory is now 1,278 identities across 4,434 slots,
173 high-consensus packages with 269 missing slots, 828 singleton packages
with 11,592 missing slots, and 632 Rust singletons, with zero collisions or
unknown buckets. `vault-pm-format-portable-conformance` now owns strict V1
repository codecs, canonical field ordering and rejection, domain-separated
IDs and signing preimages, exact version and suite gates, stable failures, and
resource bounds over caller-provided bytes. Storage, clocks, entropy, key
derivation, encryption, signatures, secret custody, and synchronization stay
outside the pure format layer. This independent new owner does not displace the
in-progress shared Lua parser prerequisite.

PR #10264 then merged externally as `c12372bc8784` after all 19 required
checks completed successfully or were skipped. The collision-checked schema-3
refresh at that exact revision contains 1,284 established identities and 4,440
package slots across 15 lanes, with 173 high-consensus packages and 269 missing
slots, 834 singletons and 11,676 missing singleton slots, 638 Rust singletons,
zero canonical collisions, and zero unknown buckets. The exact delta from
`5e1304f064` is six one-slot Rust identities, with no removals or normalization
collisions:

- `mosaic-app-bindings` is generated native host-wrapper material over the
  portable Mosaic runtime and C ABI. Its dependency-blocked applicability
  review owns dynamic library loading, environment discovery, foreign buffer
  lifetimes, generated source, and Compose, SwiftUI, XAML, Flutter, and Qt
  platform evidence rather than demanding fifteen wrapper reimplementations.
- `smart-home-axis-snapshot-host` and
  `smart-home-reolink-snapshot-host` are exact Human Approval, sealed-Vault,
  SystemTime, OS-CSPRNG, transient-credential, camera-media, and pinned TCP/TLS
  composition hosts. Separate dependency-blocked native reviews own their
  authority and platform evidence while portable owners retain entity,
  endpoint, authorization-order, request, bound, cleanup, and redaction
  fixtures.
- `smart-home-pairing-transaction` is a deterministic recoverable state machine
  over injected storage, sealed-store, and runtime-store seams. Its portable
  owner covers secret-free journals, transaction-bound references,
  expected-revision CAS, idempotent recovery and rollback, collision failures,
  and no-mutation guarantees.
- `vault-pm-storage` is the empty-capability VLT-PM02 immutable object-store
  contract with language-neutral fixtures, deterministic in-memory and fault
  models, and a reusable adapter suite. Provider authentication, filesystem,
  and network effects remain outside that portable owner.
- `vault-pm-domain` is the empty-capability VLT-PM03 merge, conflict,
  observed-set, tombstone, compaction, and redacted-view model. Its portable
  owner depends on the pure Vault record and format contracts while clocks,
  storage, transport, cryptography, and device keys remain injected.

The same refresh assigns identity-neutral changes instead of hiding them:
Mermaid sequence half arrows, central connections, autonumber ranges, rect
colors, actor links/properties, and details references extend the sequence
pipeline owner; seven HTML tree-recovery tranches extend the HTML frontend
owner; Mosaic native-emitter and artifact-builder changes extend the generated
wrapper review; and Vault record redaction plus sealed-store custody/coverage
work gains explicit portable-record, portable sealed-core, and native
entropy/time/key-custody owners. A separate state-machine tokenizer/markup
owner records the existing pure family exposed by this no-unowned-gap pass.
The security audit also records the Lua in-memory data-store engine's ambient
`os.time()` use despite its empty capability profile, and makes that injection
or capability-truthfulness repair a prerequisite of the HyperLogLog BUILD
closure.

The dependency/leverage pass selects
`build-file-lua-lattice-transitive-closure` at `c12372bc8784`. One coherent
`lattice_parser -> lattice_ast_to_css -> lattice_transpiler` chain repairs three
successive packages and six Unix/Windows standalone front doors by installing
the exact checked-in directed-graph, state-machine, grammar-tools, lexer,
lattice-lexer, parser, and lattice-parser rocks in leaf-to-root order with
dependency fetching disabled. JSON value remains the smaller one-package
follow-up behind the now-merged parser/lexer prerequisite, and data-store stays
blocked until its clock boundary is truthful.

Clean installed-runtime validation exposed one deeper Lattice boundary that
source-path Busted tests had hidden: deployed lexer and parser rocks walked out
of the LuaRocks tree looking for repository grammar files. The same tranche now
ships generated Lua payloads of the canonical language-neutral token and parser
grammars, locks them to the canonical bytes with drift tests, removes the two
runtime filesystem claims, and runs neutral-directory installed smokes after
all six build front doors. The canonical fixtures remain the specification;
the payloads are deployable checked-in projections, not a second grammar.
The final trust-boundary review also made the transpiler's direct parser import
an explicit rockspec dependency instead of relying on the AST-to-CSS package's
transitive edge. After rebasing, a brand-new isolated tree rebuilt all nine
rocks with fetching disabled and passed the installed transpiler suite and
end-to-end smoke.

Before publication, the collision-checked schema-3 inventory was refreshed
again at `d46de7b34e`. It contains 1,286 established identities and 4,442
package slots across the same 15 lanes, with 173 high-consensus packages and
269 missing slots, 836 singletons and 11,704 missing singleton slots, 640 Rust
singletons, zero canonical collisions, and zero unknown buckets. The exact
delta from `c12372bc8784` is two one-slot Rust identities:

- `vault-pm-storage-storage-core` is an empty-capability pure adapter over an
  injected storage backend. Its portable owner depends on VLT-PM02 storage and
  covers opaque locator binding, immutable create/replay/corruption outcomes,
  scoped cursors, pagination, revision deletion, ambiguous commits, and closed
  errors; filesystem, network, provider, and native durability stay outside.
- `vault-pm-repository` is the empty-capability VLT-PM04 immutable repository.
  Its portable owner depends on the format and storage contracts and covers
  opaque addressing, ordered publication and read-back, injected verification,
  bounded DAG reconstruction, pinned heads, ancestry, withholding and
  equivocation detection, retry, and plan-only garbage collection. Keys,
  clocks, entropy, decryption, signatures, providers, and deletion remain
  injected or native.

The intervening merges add no other identity: #10300 supplies concrete Hue
evidence for the portable pairing transaction and its native host review;
#10302 extends the tokenizer/markup and HTML frontend seam; #10304 and #10308
extend the Mosaic generated Qt wrapper review; #10305 extends the Mermaid
sequence accessibility fixtures; and #10303/#10306 change Persian curriculum
data. None overlaps the Lua files or changes the leverage ranking, so the
single in-progress Lattice tranche remains selected.

The final branch base advanced through merged PRs #10310, #10312, and #10311
to `61cdb3ef14`. No package directory was added or removed, and a fresh
collision-checked report keeps the inventory at 1,286 identities, 4,442 slots,
836 singletons, 11,704 singleton gaps, 640 Rust singletons, zero collisions,
and zero unknown buckets. #10310 extends multiline Mermaid accessibility in
the sequence owner, #10311 extends exact plaintext diagnostics in the HTML
frontend, and #10312 adds lossless retained observed-set persistence to the
Vault domain owner. These identity-neutral changes do not alter the selected
Lua tranche.

PR #10317 was merged externally as `75861d6a838a` on August 10. The required
post-merge refresh at `5415ba120d1a` is collision-clean and contains 1,298
established identities and 4,454 package slots across the same 15 lanes, with
173 high-consensus packages and 269 missing slots, 848 singletons and 11,872
missing singleton slots, 652 Rust singletons, zero canonical collisions, and
zero unknown buckets. Its exact package-identity delta is twelve one-slot Rust
singletons, all assigned before the next selection:

- `mosaic-app-conformance` is an applicable-platform native ABI fixture owned
  by a blocked review over the portable runtime and existing C ABI/generated
  wrapper owners, not a fifteen-language product algorithm.
- Axis, ONVIF, Reolink, Synology, and ZoneMinder pairing services are concrete
  filesystem, entropy, sealed-Vault, actor, vendor-network, and runtime hosts.
  Five blocked native-authority reviews depend on the portable recoverable
  pairing transaction and the matching snapshot-host review.
- `vault-pm-application`, `vault-pm-application-storage-core`, and
  `vault-pm-config` are empty-capability portable contracts with new fixture and
  established-lane owners. The application chain remains dependency-blocked;
  configuration is an unblocked future leaf.
- `vault-pm-cli-host` and `vault-pm-local-host` have blocked terminal, entropy,
  filesystem, locking, environment, and FFI reviews. The normalized
  package/program `vault-pm-cli` identity is split between a portable command
  contract and a blocked native composition review.

The leverage pass therefore selects the already audited
`build-file-lua-json-value-transitive-closure`: both of its prerequisites are
merged, it closes a real one-package Unix/Windows front door, and it is the
smallest coherent successor to the completed Lattice/parser wave. Vault config
is queued behind its language-neutral fixture specification; the new native and
wrapper owners remain excluded from autonomous selection.

The JSON closure must prove more than source-tree tests. Its canonical recipes
install every unpublished sibling rock in leaf-to-root order with dependency
fetching disabled, then exercise `json_value.from_string` from the installed
LuaRocks tree in a neutral directory. Because the current JSON lexer and parser
walk out of their deployed rocks to read repository grammar files, this tranche
must also bundle byte-locked Lua projections of the canonical `json.tokens` and
`json.grammar` fixtures, remove their ambient filesystem claims, and keep drift
tests as the specification guard. The files under `code/grammars/json/` remain
the language-neutral source of truth; deployable payloads are generated
projections, not competing grammars.

The final pre-publication base advanced identity-neutrally through #10512,
#10513, #10514, #10515, #10516, and #10517 to `fc8be1789c6b`. Those merges
extend already owned human-language, ADJ, HTML frontend, Mermaid sequence, and
Language Ladder surfaces, add no package directory, and do not overlap this Lua
tranche or change its leverage ranking. The collision-checked counts and twelve
new-owner classification therefore remain exact at the newer revision.

PR #10521 merged externally as `3add9a1b954d` on August 10 after all required
checks reached terminal green. The mandatory post-merge refresh at
`dda47210d304` is identity-neutral and keeps the schema-3 inventory exact at
1,298 established identities, 4,454 package slots, 848 singletons, 11,872
singleton gaps, 652 Rust singletons, zero canonical collisions, and zero
unknown buckets. The dependency audit adds two previously implicit foundations:
language-neutral canonical-CBOR conformance followed by fourteen-lane parity,
which now block both Vault format and Vault records, and a cross-lane
data-store clock-truthfulness owner because nine additional engines and several
facades repeat the Lua ambient-clock mismatch.

The leverage and security pass selects
`build-tool-python-ecosystem-scoped-alias-resolution` as the sole in-progress
item. Python currently merges every ecosystem's aliases into one table, so a
same-spelled package in another lane can redirect a dependency edge and cause
the wrong local shell `BUILD` to enter a plan; the audited repository already
misbinds 251 of 258 canonical Lua rock aliases. This pure repair defines a
language-neutral adversarial collision fixture, scopes ordinary aliases by
ecosystem, preserves only exact qualified cross-language `BUILD` edges and
within-ecosystem library-over-program precedence, and compares real-lane graphs
with the Go oracle. Haskell discovery and Haskell/Java/Kotlin filter exposure
remain dependent on this repair so default all-language resolution cannot widen
over the known trust-boundary defect. Canonical CBOR remains the highest-leverage
portable-package foundation queued after this build-tool prerequisite.

This loop delivers only deterministic, authority-free package contracts and
implementations. DNS/UDP/TCP/TLS, endpoint review, credentials and Vault,
capability approval, runtime mutation, native executors, and host hardening are
reviewed native-host exceptions and are not selectable parity work. ONVIF is
excluded from this parity tranche. Mixed smart-home packages enter the backlog
only through their portable cores and shared language-neutral fixtures.

The August 10 lane audit at `fc8be1789c6b` is:

| Established lane | Packages present | High-consensus gaps | Rust/Python-core coverage |
|---|---:|---:|---:|
| C# | 197 | 0 | 47.8% |
| Dart | 80 | 101 | 18.9% |
| Elixir | 276 | 0 | 69.3% |
| F# | 196 | 0 | 47.8% |
| Go | 292 | 0 | 72.4% |
| Haskell | 204 | 2 | 49.1% |
| Java | 128 | 57 | 31.5% |
| Kotlin | 127 | 57 | 31.5% |
| Lua | 251 | 0 | 63.3% |
| Perl | 251 | 0 | 63.3% |
| Python | 496 | 1 | 100% |
| Ruby | 294 | 0 | 70.3% |
| Rust | 1062 | 0 | 100% |
| Swift | 160 | 51 | 37.8% |
| TypeScript | 440 | 0 | 82.2% |

These are structural counts, not conformance claims. The full review queue must
cover all 15 rows even when a lane has zero gaps in the current
high-consensus subset.

## Priority 0: Inventory And Identity Integrity

Completed. The reporter now inventories Git-visible files, emits Markdown,
JSON, and CSV, classifies package lanes, detects canonical collisions, and is
covered by CI unit tests. The conflicting `ruby/b_tree` and
`ruby/b_plus_tree` shadow packages were removed in favor of the authoritative
DT11/DT12 `ruby/b-tree` and `ruby/b-plus-tree` implementations. CI now rejects
new canonical identity collisions with `--fail-on-collisions`.

Remaining inventory/build-integrity work discovered in the July 29 audit:

- reconcile stale `BUILD_windows` prerequisite declarations reported by the
  build-tool validator across Python, Perl, TypeScript, Swift, Dart, Kotlin, and
  related packages in dependency-shaped waves. The Python validator now also
  materializes a 10-file Lua wave covering the compiler, serializer, language
  server, QR, and compression dependency chains;
- repair the shared TypeScript compiler-path boundary discovered by the
  field-aware resolver tranche. `npm run build` for `typescript/transistors`
  reproduces TS6059 on unchanged `origin/main` because the shared
  `tsconfig.base.json` makes `rootDir: "src"` relative to the shared config.
  Of 458 TypeScript package/program configs with build scripts, the refined
  audit finds 129 direct shared-base consumers that inherit both faulty paths,
  three that override `rootDir` but still inherit the faulty `outDir`, 155 that
  override both paths, and 36 rootDir-less standalone configs outside the
  shared contract. The repair affects 132 configs in total. This is a
  dependency-shaped portability wave, separate from the 58-package
  `BUILD_windows` debt;
- isolate outputs for the two standalone TypeScript configs that had neither
  `noEmit` nor `outDir`. The executable audit now classifies all 171 standalone
  configs: 28 are type-check-only, and all 143 emit-capable projects declare
  isolated output. `window-core` and `window-canvas` emit only below `dist` and
  exclude compiled test copies while retaining Vitest's defaults. Their fresh
  builds, coverage suites, effective-config checks, and clean-tree assertions
  pass as a separate two-package item rather than part of the shared-base
  repair;
- keep the Python build tool's Lua rockspec decoding deterministic. Merged PR
  #9495 normalized the three CP1252 metadata bytes, added positive and invalid-
  UTF-8 fixtures, and returns `METADATA_INVALID_UTF8`; its refreshed full scan
  succeeds across 4,765 packages and 7,100 edges;
- bring the remaining Perl, Ruby, Elixir, and Haskell
  build-tool resolvers to the shared strict-UTF-8 rockspec contract.
  Their current byte, replacement, silent-drop, and locale-sensitive behavior
  is tracked separately from the Python full-scan blocker. Merged PR #9504
  completed the Go operational-oracle child, and merged PR #9510 completed
  Rust. Merged PR #9537 completed Swift with exact shared success and invalid-
  byte fixtures plus real CLI exit-2 coverage. Merged PR #9572 completed
  TypeScript with strict decoding, exact shared-fixture coverage, and green
  Ubuntu, macOS, Windows, and JavaScript/TypeScript CodeQL checks. Merged PR
  #9603 completed Lua with strict raw-byte validation, both shared fixtures,
  the stable repository-relative diagnostic, real CLI exit code 2, malformed
  Unicode coverage, and green Ubuntu, macOS, and Windows checks. Merged PR
  #9632 completed Perl with strict decoding, exact success edges, typed stable
  diagnostics, real CLI exit 2, malformed-sequence coverage, and green Ubuntu,
  macOS, Windows, fixture, CI-gate, and CodeQL checks. Its security gate
  also discovered that the Perl build tool's `5.026` runtime floor and
  version-free core-module declarations do not produce an auditable clean
  dependency floor; that compatibility-policy review is logged as a separate
  pending child instead of expanding the UTF-8 behavior slice. Merged PR #9648
  completed Ruby with strict byte decoding, typed stable diagnostics, the exact
  shared fixtures, real CLI exit 2, and green Ubuntu, macOS, Windows, fixture,
  CI-gate, and Ruby CodeQL checks. The post-#9717 leverage pass selects Haskell
  next because it is the remaining locale-sensitive established-language
  boundary and will provide a stronger reference for the position-dependent
  Elixir repair; Elixir remains pending. The Haskell real-package validation
  also exposed a separate Windows source-hashing boundary: locale-sensitive
  child-process stdin aborts with GHC `commitBuffer` error `0xc000014b` when
  hashing non-ASCII package text. That binary-safe hashing repair is logged as
  a dependent child rather than widening the rockspec metadata slice;
- close the Ruby build tool's Starlark source-tree execution gaps. The Ruby
  UTF-8 validation found that a clean real plan cannot load the undeclared
  `coding_adventures_starlark_interpreter` runtime until repository library
  paths are injected, after which 44 canonical Elixir, Go, and Rust
  BUILD suffixes still produce parse warnings and raw-command fallback. Runtime
  closure and canonical BUILD compatibility are separate children so the
  metadata-decoding slice stays behaviorally narrow. The post-#9648 leverage
  pass selected runtime closure first because it is the narrowest unblocked
  fix and unlocks the dependent 44-case compatibility repair. The selected
  implementation now imports the interpreter's authoritative repository-local
  Gemfile closure and proves the exact repository feature loads from a clean
  Bundler subprocess with ambient Ruby load paths removed and network proxies
  blocked. Exact-base validation passed 297 Ruby runs/589 assertions with
  87.91% line and 72.05% branch coverage, 38 downstream interpreter runs,
  canonical Go test/vet/build and a 300-package Ruby plan, the 17+40
  conformance suites and 37-case/56-file corpus, and a real 258-package Ruby-
  engine plan with zero LoadErrors while preserving all 44 separately owned
  canonical Starlark parse fallbacks. PR #9660 merged as `bfe5131115` after all
  17 rebased exact-head checks passed, skipped, or completed neutrally. The
  refreshed leverage pass selected the now-unblocked canonical BUILD
  compatibility child next because it removes all 44 silent fallbacks using
  the already-validated Ruby and interpreter toolchains. Merged PR #9677 now
  parses the canonical multiline return shape, preserves loaded-module globals
  and keyword-call structure, injects the normalized v1 context, validates
  structured commands, and fails closed with a root-redacted diagnostic. The
  exact real-plan comparison matches every field for all 9 Elixir, 6 Go, and
  29 Rust Starlark records and the 258-package Lua plan emits zero Starlark
  warnings. Validation passes 302 Ruby build-tool runs/614 assertions at
  88.04% line and 71.11% branch coverage, all six downstream Ruby language
  suites, canonical Go test/vet/build, and the valid 37-case/56-file corpus.
  GitHub's guarded squash auto-complete merged the exact head after 17 terminal
  checks as `d96d518b0f`. The comparison separately found Ruby's package
  identities omit the canonical `programs` segment in dependency edges. The
  dependency/leverage pass selected that now-unblocked registry migration next.
  The Ruby slice now consumes the complete registry, preserves package/program
  identities, excludes specification trees, rejects duplicate identities, and
  honors qualified legacy BUILD dependency comments. Its real 4,776-package
  plan has unique forward-slash identities; Go, Rust, and Lua package/edge sets
  exactly match the Go engine at 301/936, 928/2,250, and 258/383. Validation on
  rebased `29d78677` passes 307 Ruby tests at 89.43% line and 72.55% branch
  coverage, all shared fixture consumers, the valid 38-case/61-file corpus,
  Go test/vet/build, dependency and security checks, and the collision-clean
  1,230-identity inventory without widening the completed evaluator repair.
  Guarded squash auto-completion merged PR #9691 as `9ad8105f38` after all 17
  exact-head checks were terminal: twelve succeeded, four skipped, and one
  completed neutrally;
- adjudicate the residual Ruby/Go Elixir resolver semantics after canonical
  identities remove the structural drift. The real 282-package plans differ
  on eleven Go-only and two Ruby-only edges; the Ruby-only pair corresponds to
  declared `grammar_tools` program dependencies, so the follow-up must derive
  language-neutral fixtures from actual Mix metadata instead of assuming that
  either engine is the oracle. This child depends on the identity migration;
- bring Go discovery to the complete shared language registry. The selected
  implementation recognizes every canonical and retained bucket only at an
  exact `packages` or `programs` boundary, preserves program identities,
  excludes specification and build-artifact trees, and fails closed on a
  duplicate with the shared typed, root-redacted diagnostic and CLI exit 2.
  On rebased `b42d1683`, independent Go and Ruby real plans match all 4,779
  `(name, language, rel_path)` tuples with zero duplicate or backslash paths,
  all fourteen Mosaic identities canonical, and only the intentional
  `unknown/blog` bucket. Exact-head validation passes gofmt, the full Go suite
  at 73.6% aggregate and 93.9% discovery coverage, vet, race, trimpath build,
  module verification, zero-reachable-vulnerability govulncheck, the 17+40
  conformance suites, and the valid 38-case/61-file corpus. The committed Go-
  lane dry-run selects 13 of 301 packages and skips 288. The full validator
  separately reproduces the owned 73-gap BUILD debt gate: 12 Python, 3 Swift,
  and 58 TypeScript. Resolver-semantic work remains a separate child. Guarded
  squash auto-completion merged PR #9703 as `3258981ff1` after all 17 exact-
  head checks were terminal: twelve succeeded, four skipped, and one completed
  neutrally;
- make the TypeScript build-tool git-diff suite portable on Windows. The strict-
  UTF-8 validation run found two hard-coded `/bin/sh` invocations and five
  POSIX-only `/repo` path fixtures. Merged PR #9592 is the completed slice: it
  uses direct Git argument vectors and native temporary roots, and normalizes
  relative package paths at the Git boundary without changing selection
  semantics. Its exact Windows suite is green at 282/282 tests;
- align TypeScript build-tool discovery with the canonical language and identity
  registry. Merged PR #9582 consumes the shared registry and duplicate-
  identity fixtures, preserves package/program identity, excludes spec fixtures,
  and fails closed on collisions. Its real Windows plan now emits 4,768 unique
  identities, zero duplicate groups, and only intentional `unknown/blog`, down
  from 655 unknown records and 217 duplicate groups;
- make TypeScript build-plan package paths portable on Windows. The identity-
  registry validation found that `rel_path` values still use host backslashes;
  merged PR #9589 is the fixture-driven slice because every one of the
  real plan's 4,768 package records is affected on Windows. Normalize serialized
  repository-relative paths without widening discovery behavior;
- make Swift build-tool file options recognize Windows drive-letter absolute
  paths. The UTF-8 validation run discovered that `--emit-plan` reaches
  resolution but then joins an absolute Windows path under the repository;
  cover `--emit-plan`, `--plan-file`, and `--cache-file` in a separate slice;
- align Swift build-tool discovery with the canonical language and identity
  registry. Merged PR #9553 consumes the shared registry and duplicate-identity
  fixtures; its full release plan emits 4,768 unique identities, zero duplicate
  groups, and only the intentional language-neutral `unknown/blog`, without
  widening into file-option handling;
- merged PR #9521 makes the Rust build tool reject resolver self-edges with a
  stable diagnostic and preserves distinct package/program identities for
  `elixir/grammar_tools`;
- bring Rust build-tool discovery to the complete canonical language and
  identity registry. Merged PR #9527 classifies every repository bucket,
  excludes specification fixture trees, and rejects residual duplicate names;
  its real full plan exits zero with 4,765 entries, 4,765 unique identities,
  and only the intentional language-neutral `code/sites/blog` package;
- expose Haskell through the Python build tool's `--language` filter. Haskell
  is already in its resolver and canonical language registry but is missing
  from the native CLI choices;
- make Python build-plan emission replace an existing destination atomically on
  Windows. Repeated output currently leaves the temporary path and fails with
  `WinError 183`, while a fresh destination succeeds;
- remove environment-specific Starlark grammar lookup so build discovery works
  from arbitrary clean worktrees;
- add explicit applicability and maturity data before either C/C++ or OCaml
  enters the all-language completion denominator.

## Priority 1: Complete The 14-Of-15 Set

Priority 1 is complete. Every package that entered this wave at 14-of-15 now
has an implementation in all 15 language lanes.

### Dart: complete

Completed in the Dart lane: `heap`, `bitset`, `pixel-container`,
`image-point-ops`, `logic-gates`, `image-geometric-transforms`, `toml-lexer`.
The grammar-driven `mosaic-lexer`/`mosaic-parser` and
`algol-lexer`/`algol-parser` pairs are also complete.
The dependency-shaped DT11/DT12 `b-tree`/`b-plus-tree` pair is complete.

### Haskell: complete

Completed in the Haskell lane: `activation-functions`, `caesar-cipher`,
`huffman-tree`, `huffman-compression`, `lz77`, `lzss`, `lzw`.

### Swift: complete

Completed in the Swift lane: `wasm-simulator`, `cli-builder`,
`sql-execution-engine`.

### Current 14-of-15 frontier: reopened

The historical Priority 1 cohort is complete, but later Haskell work promoted
13 packages into a new 14-of-15 frontier. Merged PR #9383 closed `trig` and
`wave`, merged PR #9477 closed `matrix`, `loss-functions`, and
`feature-normalization`, and the subsequent document/cipher waves closed
`document-ast`, `atbash-cipher`, `scytale-cipher`, and `vigenere-cipher`. The
exact post-#13083 inventory leaves five current gaps:

- deterministic data structures: `binary-search-tree`, `fenwick-tree`, and
  `trie`, all missing only Dart;
- utility leaf: Dart `uuid`;
- rendering leaf: Swift `paint-vm-ascii`, already owned by open PR #12149.

Merged PR #9375 closed the generator-level prerequisite found by the post-#9363
fixture audit. Dart's native scaffold generator now emits byte-stable,
schema-v1 empty library profiles and truthful generated-program stdout
profiles, while declaring its own reviewed runtime authority. Existing
nonempty Dart profiles remain owned by the legacy migration review. Close the
remaining four unowned Dart packages as small coherent PRs on top of that
scaffold contract; do not duplicate the live Swift owner.

Merged PR #9383 completed the first child item: the zero-dependency PHY00
`trig` leaf and its direct PHY01 `wave` consumer. This closed two Dart-only
14-of-15 gaps while exercising the scaffold and capability contract on a real
dependency chain.

Merged PR #9477 completed the ML child item with the independent `matrix`,
`loss-functions`, and `feature-normalization` leaves. The post-merge leverage
pass selected `document-ast` next: its types-only TE00 model has 68 exact
cross-repository consumers and unlocks substantially more follow-on parity work
than the then-remaining cipher and data-structure leaves. That branch delivered the
sealed, immutable 24-node model with exhaustive discriminator, containment,
value-semantics, and coverage checks. PR #13083 subsequently delivered the
CR01-CR03 cipher trio. Trie, binary-search-tree, fenwick-tree, and UUID remain
explicit child items; the existing Dart LZ78 private-trie migration is tracked
separately.

The post-merge governance audit found no repository-verifiable Layer 5 approval
for #9375's nonempty generator profile: GitHub reports no review decision, and
the merge commit carries the GitHub web-flow signature rather than evidence
bound to `CAPABILITY_SIGNERS`. The externally blocked
`dart-scaffold-capability-layer5-evidence` item owns either recovery of the
actual hardware-key-backed approval or an explicit reviewed policy
reconciliation. The pure `dart-trig-wave` child neither executes nor publishes
the generator and introduces only empty capability profiles.

The child review also exposed three separately owned follow-ups rather than
silently widening that delivery slice: a machine-readable language-neutral
PHY00/PHY01 fixture corpus, a full-range tiny/subnormal PHY00 square-root audit,
and finite-input and overflow-safe PHY01 evaluation reconciliation. Merged PR
#9390 delivered the closed 53-case shared corpus and its first always-on Dart
consumer. The collision-clean post-merge inventory and parallel dependency,
fixture, and security audits then selected
`phy00-small-sqrt-cross-lane-audit` because PHY00 is the
foundational numeric dependency beneath PHY01 and the shared oracle now makes
the known boundary defect testable across all 15 established lanes plus the
emerging C and C++ implementations. PR #9395 merged that repair with all checks
green. The collision-clean `761c60fc3` inventory and dependency audit then
selected `phy01-nonfinite-validation-backfill` as the consumer-side successor;
PR #9400 merged that repair with all 15 checks green. The collision-clean
`20afefa7a` inventory found one newly owned Rust dashboard-core singleton and
selected `phy00-atan-tiny-signed-zero-cross-lane-audit` next because its sole
dependency is merged and every existing trig lane shares the same defect. The
square-root numerical audit had discovered that separate
`phy00-atan-tiny-signed-zero-cross-lane-audit`: current half-angle reduction
underflows at the subnormal floor and loses the sign of negative zero, so that
work remained tracked behind the merged square-root and wave slices rather
than expanding either delivery.
PR #9413 merged that repair at `458405a6e` after all 15 checks passed. The
collision-clean `1e4956369` refresh added the two Rust-only camera identities.
PR #9421 merged the camera-media boundary repair. With no active in-scope parity
PR at `e552707d5`, the leverage pass selected
`dart-current-14-of-15-matrix-family`: the independent `matrix`,
`loss-functions`, and `feature-normalization` leaf packages. PR #9477 merged
that slice at `1233e31db` with successful final-head push CI, PR CI, and
CodeQL. The refreshed leverage pass selected the zero-dependency TE00
`document-ast` model next because its 68 exact consumers make it the strongest
remaining dependency foundation. The branch is
`codex/dart-document-ast-parity`; open ONVIF and other host PRs do not occupy
the scoped parity slot.
The audit also found private matrix/MSE helpers in Dart `single-layer-network`
and `two-layer-network`; their migration to the shared packages is a separate
downstream backlog item rather than hidden scope in this port.
The security pass also found that ML01 does not define a shared NaN/infinity
input policy: Dart and the Python reference can clamp NaN cross-entropy
predictions to a finite boundary. A separate all-lane fixture and conformance
item owns that decision instead of introducing a Dart-only behavioral fork.
The build-tool execution critical path remains blocked on external
immutable-runner and attester provisioning.

Port dependency families together when doing so avoids temporary broken package
graphs. Grammar-generated lexer/parser pairs should be generated from the shared
grammar sources rather than independently handwritten.

## Priority 2: Complete The High-Consensus Core

The exact post-#13083 inventory has 175 packages present in at least ten
implementation languages and 267 missing slots to reach all 15. After Priority
1, select work in this order:

| Language lane | Current high-consensus gaps | Pairing rule |
|---|---:|---|
| C# | 0 | Complete; paired native package wave |
| Dart | 98 | Close the reopened 14-of-15 set, then dependencies before consumers |
| Elixir | 1 | Close `image-codec-png`, then retain as a reference lane |
| F# | 0 | Complete; paired native package wave |
| Go | 0 | Complete; primary build-tool and portable-core reference lane |
| Haskell | 4 | Finish `event-loop`/`brotli`, then `ct-compare`/`image-codec-png` |
| Java | 53 | Move with Kotlin |
| Kotlin | 53 | Move with Java |
| Lua | 1 | Close `ct-compare`, then remediate build-tool drift |
| Perl | 2 | Close `ct-compare` and `image-codec-png`, then build-tool drift |
| Python | 1 | Classify the remaining self-hosted `python-parser` carefully |
| Ruby | 1 | Close `image-codec-png`, then retain as a reference lane |
| Rust | 1 | Close `image-codec-png`; reference broad and singleton families |
| Swift | 52 | Data structures and generated frontends before native app surfaces |
| TypeScript | 0 | Complete; reference lane for web-capable portable contracts |

Zero-gap lanes remain active reference and conformance lanes; they are not
exempt from semantic review or build-tool parity.

The e34f26fad inventory gives Python's lone high-consensus gap a dedicated
`python-parser-self-hosting-applicability` owner. It must record whether a native
Python parser port is portable, self-hosting-applicable, or a reviewed exception
through explicit reporter applicability data instead of leaving the gap
unclassified.

The first seventeen paired Lua/Perl slices are complete: `fenwick-tree`,
`binary-tree`, `binary-search-tree`, `in-memory-data-store-protocol`,
`avl-tree`, `tree-set`, `skip-list`, `hyperloglog`, `trie`, `radix-tree`,
`resp-protocol`, `hash-functions`, `bloom-filter`, `hash-map`, `hash-set`,
`in-memory-data-store-engine`, and `in-memory-data-store` now have pure
implementations, package-native tests, metadata, and capability declarations
in both lanes.
The protocol slice establishes the dependency-free IR needed before the higher
in-memory data store layers move; the AVL slice supplies the ordered backend
for `tree-set`; and the dependency-free skip-list slice adds a span-augmented
ordered map with logarithmic rank and selection. The HyperLogLog slice adds a
fixed-memory approximate distinct counter with deterministic internal hashing.
The trie slice adds Unicode-aware prefix storage with sorted scans, pruning
deletion, and longest-prefix matching without introducing new dependencies.
The radix-tree slice compresses those paths into whole-substring edges while
retaining Unicode-safe splits, post-deletion merges, and mid-edge prefix scans.
The RESP2 slice adds a typed, binary-safe wire codec with distinct null bulk and
null array values plus an incremental decoder tested across arbitrary stream
fragmentation, establishing the wire layer needed by the higher storage stack.
The paired `hash-functions` prerequisite adds binary-safe FNV-1a, DJB2,
polynomial rolling, and MurmurHash3 implementations with deterministic analysis
helpers. It moves the package from 9 to 11 implementation lanes and unblocks
the remaining Bloom-filter and hash-map slices without external dependencies.
The Bloom-filter slice uses that prerequisite for correlation-mixed double
hashing, deterministic composite-value encoding, exact sizing helpers, and live
fill and false-positive statistics, reducing the paired high-consensus gap to
four packages per lane.
The hash-map slice implements both DT18 collision strategies from first
principles, including chaining, linear probing, tombstone deletion, automatic
resizing, deterministic DT17 bucket hashes, bulk access, merge, and clone-based
functional operations. It reduces the paired high-consensus gap to three
packages per lane and supplies the direct dependency for the `hash-set` slice.
The hash-set slice composes that map into a persistent DT19 collection with
copy-on-write add and remove, complete set algebra and relation predicates,
option preservation, identity-safe reference elements, and resize coverage for
both collision strategies. It reduces the paired high-consensus gap to two
packages per lane, leaving the higher in-memory data store layers as the final
paired wave.
The engine slice consumes the existing protocol IR and HyperLogLog packages to
provide binary-safe strings, typed collections, sorted sets, expiry, 16 logical
databases, deterministic response ordering, and the complete 57-command
execution surface. It reduces the paired high-consensus gap to one package per
lane, leaving only the top-level `in-memory-data-store` facade.
The facade slice composes the RESP2 streaming decoder, command protocol IR, and
execution engine into incremental and pipelined byte-stream entry points with
binary-safe response conversion. It moves the package from 11 to 13
implementation lanes and closes the remaining high-consensus gaps in both Lua
and Perl.

The first paired C#/F# slice is complete: `wasm-module-encoder` now has native
implementations in both lanes, built on their existing `wasm-leb128` and
`wasm-types` packages. Parser round-trip tests cover all WebAssembly 1.0 module
sections and import descriptor validation. The package now spans 12 lanes,
reduces each paired high-consensus gap to 16, and unlocks the later
`brainfuck-wasm-compiler` and `nib-wasm-compiler` ports.

The second paired C#/F# slice is complete: `x25519` now has native,
dependency-free implementations in both lanes using the RFC 7748 Montgomery
ladder over `2^255 - 19`. RFC scalar-multiplication, Diffie-Hellman,
high-bit-masking, low-order rejection, and 1,000-round iterated vectors provide
conformance coverage. The package now spans 12 implementation lanes and
reduces each paired high-consensus gap to 15.

The third paired C#/F# slice is complete: `brainfuck-wasm-compiler` now builds
typed WebAssembly modules through the new native encoders in both lanes.
Package-native tests cover source filtering, balanced and depth-limited loops,
8-bit cell and pointer emission, optional WASI I/O imports, file output,
parser/validator round trips, and locally supported runtime execution. The
package now spans 13 implementation lanes and reduces each paired
high-consensus gap to 14.

The fourth paired C#/F# slice is complete: `argon2i` now implements the RFC
9106 data-independent memory-hard password hash in both lanes on top of their
existing BLAKE2b packages. RFC vectors cover secret keys, associated data,
multiple lanes and passes, variable tag lengths, and address-block rollover.
The package now spans 12 target implementation lanes and reduces each paired
high-consensus gap to 13.

The fifth paired C#/F# slice is complete: `argon2d` now implements the RFC
9106 data-dependent memory-hard password hash in both lanes on top of their
existing BLAKE2b packages. RFC vectors cover secret keys, associated data,
multiple lanes and passes, variable tag lengths, and deterministic memory-cost
rounding. The package now spans 12 target implementation lanes and reduces
each paired high-consensus gap to 12.

The sixth paired C#/F# slice is complete: `argon2id` now implements RFC 9106's
recommended hybrid password hash in both lanes, using data-independent
addresses for the first half of pass zero and data-dependent addresses
thereafter. Canonical vectors cover the address-mode transition, secret keys,
associated data, multiple lanes and passes, and variable tag lengths. The
package now spans 12 target implementation lanes and reduces each paired
high-consensus gap to 11.

The seventh paired C#/F# slice is complete: `chacha20-poly1305` now implements
the self-contained RFC 8439 ChaCha20 stream cipher, Poly1305 one-time MAC, and
combined AEAD construction in both lanes. Canonical block, stream, MAC, and
AEAD vectors cover the full construction, while multiblock round trips and
tamper tests verify counter progression and authenticate-before-decrypt
behavior. The package now spans 12 implementation lanes and reduces each
paired high-consensus gap to 10.

The eighth paired C#/F# slice is complete: `xml-lexer` now provides native,
context-sensitive scanners in both lanes while reusing their existing lexer
token models. Package-local state transitions match the shared XML grammar's
content, tag, comment, CDATA, and processing-instruction groups; tests cover
namespaces, quoted attributes, entity and character references, significant
whitespace, token positions, malformed input, and EOF behavior. The package
now spans 12 implementation lanes, reduces the high-consensus backlog to 327
slots, and leaves 9 paired gaps in each lane.

The ninth paired C#/F# slice is complete: `block-ram` now models SRAM cells,
row-addressed arrays, rising-edge single-port and true dual-port RAM, all three
read-during-write modes, same-address collision detection, and configurable
FPGA-style width/depth aspect ratios in both lanes. Package-native tests cover
cross-port visibility, edge behavior, reconfiguration clearing, defensive
copies, and invalid signals and dimensions. The package now spans 12 target
implementation lanes, reduces the high-consensus backlog to 325 slots, and
leaves 8 paired gaps in each lane.

The tenth paired C#/F# slice is complete: `nib-wasm-compiler` now compiles the
portable typed Nib `u4` function subset in both lanes. Native parsers cover
literals, parameters, nested calls, and wrapping `+%` addition, while the
existing `wasm-module-encoder`, `wasm-types`, and `wasm-leb128` packages produce
validated modules that export every declared function. Package-native tests
cover executable literals and calls, wrapping-opcode validation, malformed
source, depth and size limits, defensive results, and optional file output. The
package now spans 13 implementation lanes, reduces the high-consensus backlog
to 323 slots, and leaves 7 paired gaps in each lane.

The eleventh paired C#/F# slice is complete: `dartmouth-basic-lexer` now loads
the shared token grammar from an embedded resource in both lanes. The native
wrappers normalize case-insensitive token values, relabel only physical-line
labels as `LINE_NUM`, preserve string case without quotes, and suppress `REM`
bodies while retaining their terminating newline. Package-native tests cover
operators, numeric formats, functions, unknown input, CRLF positions, blank
lines, and multi-line remark recovery. The package now spans 12 implementation
lanes, reduces the high-consensus backlog to 321 slots, leaves 6 paired gaps in
each lane, and unlocks the dependency-safe `dartmouth-basic-parser` slice.

The twelfth paired C#/F# slice is complete: `dartmouth-basic-parser` now
combines those native lexers with each lane's grammar-driven parser and the
shared BASIC grammar embedded as a package resource. The adapters enforce
complete non-EOF token consumption so malformed statements cannot collapse to
the grammar's valid empty program, while package-native tests exercise all 17
statement forms, expression precedence, configured and one-shot APIs, empty
and bare-line programs, and syntax failures. The package now spans 12
implementation lanes, reduces the high-consensus backlog to 319 slots, and
leaves 5 paired gaps in each lane.

The thirteenth paired C#/F# slice is complete: `ed25519` now provides native
RFC 8032 key generation, deterministic signing, and verification in both
lanes, composing their existing SHA-512 packages with extended Edwards
coordinates over `2^255 - 19`. Package-native tests cover the first three RFC
vectors, deterministic key and signature derivation, wrong messages and keys,
tampered signature halves, non-canonical scalars, malformed point encodings,
and strict seed and secret-key formats. The package now spans 12 implementation
lanes, reduces the high-consensus backlog to 317 slots, and leaves 4 paired
gaps in each lane.

The fourteenth paired C#/F# slice is complete: `font-parser` now provides
dependency-free metrics-only OpenType and TrueType readers in both lanes.
Native big-endian table parsing covers global metrics and names, BMP `cmap`
format 4 glyph lookup, complete and shared `hmtx` records, optional `OS/2`
heights, and legacy `kern` format 0 pairs. Package-native tests exercise the
shared Inter fixture, in-memory synthetic fonts, malformed directories and
sentinels, immutable input ownership, unsupported mappings, shared advances,
and sorted kerning lookup. The package now spans 12 implementation lanes,
reduces the high-consensus backlog to 315 slots, and leaves 3 paired gaps in
each lane.

The fifteenth paired C#/F# slice is complete: `asciidoc-parser` now provides
native block and inline parsers over each lane's shared `document-ast` model.
Both implementations cover headings, paragraphs and breaks, source, literal,
passthrough, and recursive quote blocks, ordered and unordered nested lists,
comments, thematic breaks, emphasis, strong text, code spans, link and image
macros, cross-references, and HTTP autolinks. Mirrored package-native suites
exercise 33 test cases in each lane, including lenient unterminated blocks and
malformed inline delimiters, with more than 97% line coverage. The
package now spans 12 implementation lanes, reduces the high-consensus backlog
to 313 slots, and leaves 2 paired gaps in each lane.

The sixteenth paired C#/F# slice is complete: `fpga` now provides native
SRAM-backed lookup tables, dual-LUT slices with optional registers and carry
chains, configurable logic blocks, programmable switch matrices, I/O pads, and
immutable JSON bitstream configuration in both lanes. The ports compose the
existing native `logic-gates` and `block-ram` packages, and their package-local
suites exercise 40 C# and 38 F# cases with more than 97% line coverage. The
package now spans 12 implementation lanes, reduces the high-consensus backlog
to 311 slots, and leaves only `zstd` as a paired gap in C# and F#.

The seventeenth paired C#/F# slice is complete: `zstd` now provides native
CMP07 educational Zstandard codecs in both lanes. The ports compose their
existing native `lzss` packages and implement frame headers, 128 KiB raw, RLE,
and compressed blocks, raw literal sections, and the predefined
literal-length, match-length, and offset FSE tables. Package-native suites
exercise 29 C# and 20 F# cases with more than 90% line coverage, including
multi-block frames, malformed input, compression ratios, and cross-language
compatibility with the established Ruby implementation. The package now spans
12 implementation lanes, reduces the high-consensus backlog to 309 slots, and
closes the remaining high-consensus gaps in both C# and F#.

The first Haskell high-consensus slice is complete: `atbash-cipher` now
provides a dependency-free CR01 implementation that mirrors ASCII letters
while preserving case and passing all other characters through unchanged.
Its package-native suite exercises 11 examples with 100% expression coverage,
including complete alphabets, non-ASCII pass-through, and the cipher's
self-inverse property. The package now spans 13 implementation lanes, reduces
the high-consensus backlog to 308 slots, and leaves 33 gaps in the Haskell
lane.

The second Haskell high-consensus slice is complete: `scytale-cipher` now
provides dependency-free CR02 encryption, decryption, explicit key validation,
and brute-force candidate generation. Its package-native suite exercises 17
examples with 98% expression coverage, including reference vectors, padded and
uneven grids, mixed-character round trips, and the complete shared key range.
The package now spans 14 implementation lanes, reduces the high-consensus
backlog to 307 slots, and leaves 32 gaps in the Haskell lane.

The third Haskell high-consensus slice is complete: `feature-normalization`
now provides dependency-free ML05 standard and min-max scaler fitting and
transformation with explicit rectangular-matrix and scaler-width validation.
Its package-native suite exercises 14 examples with 99% expression coverage,
including the shared matrix, population deviation, constant columns, negative
ranges, new observations, and every validation branch. The package now spans
14 implementation lanes, reduces the high-consensus backlog to 306 slots, and
leaves 31 gaps in the Haskell lane.

The fourth Haskell high-consensus slice is complete: `loss-functions` now
provides dependency-free ML04 mean squared, mean absolute, binary
cross-entropy, and categorical cross-entropy losses together with their
prediction gradients, explicit vector validation, and finite probability
clamping. Its package-native suite exercises 15 examples covering reference
values, every derivative branch, boundary probabilities, and all validation
paths. The package now spans 14 implementation lanes, reduces the
high-consensus backlog to 305 slots, and leaves 30 gaps in the Haskell lane.

The fifth Haskell high-consensus slice is complete: `trig` now provides the
dependency-free PHY00 angle constants, range-reduced sine and cosine series,
degree/radian conversions, Newton-method square root, pole-guarded tangent,
and inverse tangent functions. Its package-native suite exercises 17 examples
with 99% expression coverage, including reference angles, identities,
large-input reduction, conversions, domain validation, tangent poles, inverse
ranges, axes, and all quadrants. The package now spans 14 implementation
lanes, reduces the high-consensus backlog to 304 slots, leaves 29 gaps in the
Haskell lane, and unlocks the dependent `wave` port.

The sixth Haskell high-consensus slice is complete: `wave` now builds on the
local `trig` layer to provide validated PHY01 sinusoidal waves, periods,
angular frequencies, phase offsets, and time-domain evaluation. Its
package-native suite exercises 17 examples with 95% expression coverage,
including construction, validation, derived quantities, the full cycle,
periodicity, phase offsets, and the zero-amplitude case. The package now spans
14 implementation lanes, reduces the high-consensus backlog to 303 slots, and
leaves 28 gaps in the Haskell lane.

The seventh Haskell high-consensus slice is complete: `matrix` now provides
immutable rectangular matrices with factories, arithmetic, multiplication,
indexed updates, reductions, element-wise math, shape operations, and exact or
tolerant comparison. Its package-native suite exercises 34 examples with 96%
expression coverage, including rectangular validation, every operation family,
empty and zero-width shapes, mismatched dimensions, invalid indices, and
half-open slices. The package now spans 14 implementation lanes, reduces the
high-consensus backlog to 302 slots, and leaves 27 gaps in the Haskell lane.

The eighth Haskell high-consensus slice is complete: `vigenere-cipher` now
provides case-preserving encryption and decryption, strict ASCII-key
validation, index-of-coincidence key-length estimation, chi-squared key
recovery, and automatic cipher breaking. Its package-native suite exercises 26
examples with 97% expression coverage, including parity vectors, punctuation,
Unicode pass-through, invalid keys, round trips, three recovery key lengths,
and short-input behavior. The package now spans 14 implementation lanes,
reduces the high-consensus backlog to 301 slots, and leaves 26 gaps in the
Haskell lane.

The ninth Haskell high-consensus slice is complete: `uuid` now provides strict
128-bit construction, parsing and rendering, standard namespaces, metadata,
and native v1, v3, v4, v5, and v7 generation. It builds name-based UUIDs on the
existing Haskell MD5 and SHA-1 ports and uses native time and randomness for
the generated versions. Its package-native suite exercises 17 examples with
87% expression coverage, including accepted and rejected text forms, integer
and byte round trips, all variants, RFC name-based vectors, Unicode names,
random uniqueness, multicast nodes, and v7 timestamps. The package now spans
14 implementation lanes, reduces the high-consensus backlog to 300 slots, and
leaves 25 gaps in the Haskell lane.

The tenth Haskell high-consensus slice is complete: `document-ast` now provides
immutable algebraic data types for the TE00 block and inline model together
with the shared GFM task-list, strikethrough, and table extensions. Exhaustive
unions and stable discriminator helpers support typed parser and renderer
traversal without external dependencies. Its package-native suite exercises
11 examples with 100% expression and alternative coverage across every node
family, payload accessor, nesting shape, and discriminator. The package now
spans 14 implementation lanes, reduces the high-consensus backlog to 299 slots,
and leaves 24 gaps in the Haskell lane.

The eleventh Haskell high-consensus slice is complete: `lz78` now provides the
CMP01 token model, an immutable byte-trie cursor, dictionary-capped encoding,
checked decoding, strict big-endian wire serialization, and deterministic
one-shot compression. Its package-native suite exercises 13 examples with 98%
expression and 96% alternative coverage, including both canonical token
vectors, end-of-stream flushing, dictionary caps, text and binary round trips,
exact wire bytes, and every malformed-input error family. The package now spans
13 implementation lanes, reduces the high-consensus backlog to 298 slots, and
leaves 23 gaps in the Haskell lane.

The twelfth Haskell high-consensus slice is complete: `deflate` now provides a
pure CMP05 encoder and strict decoder that compose the existing Haskell LZSS
and canonical Huffman packages. Its package-native suite exercises 26 examples
with 91% expression coverage, including exact Python-compatible wire vectors,
literal-only and match-heavy streams, binary round trips, parameter validation,
and malformed headers, tables, prefixes, backreferences, and output lengths.
The package now spans 13 implementation lanes, reduces the high-consensus
backlog to 297 slots, and leaves 22 gaps in the Haskell lane.

The thirteenth Haskell high-consensus slice is complete: `point2d` now provides
immutable G2D00 point/vector arithmetic and half-open axis-aligned rectangle
geometry on top of the existing pure Haskell `trig` package. Its package-native
suite exercises construction, products, norms, normalization, distance,
interpolation, axis angles, empty and negative extents, boundary containment,
union, strict positive-area intersection, and symmetric expansion. The package
now spans 12 implementation lanes, reduces the high-consensus backlog to 296
slots, leaves 21 gaps in the Haskell lane, and unlocks the dependent `affine2d`,
`bezier2d`, and `arc2d` graphics wave.

The fourteenth Haskell high-consensus slice is complete: `affine2d` now
provides the immutable G2D01 six-scalar matrix, all standard factories,
ordered composition, separate point and vector application, determinant,
checked inversion, tolerance predicates, and SVG/Canvas component ordering.
It composes the existing pure Haskell `point2d` and `trig` packages and covers
centered rotation, skew, non-commutativity, singularity thresholds, and inverse
round trips in its package-native suite. The package now spans 12 implementation
lanes, reduces the high-consensus backlog to 295 slots, and leaves 20 gaps in
the Haskell lane.

The fifteenth Haskell high-consensus slice is complete: `bezier2d` now
provides immutable G2D02 quadratic and cubic curves, numerically stable de
Casteljau evaluation and exact splitting, unnormalized derivatives, adaptive
polyline flattening, tight derivative-root bounds, and exact quadratic degree
elevation. It composes only the existing pure Haskell `point2d` package. Its
package-native suite exercises 21 examples with 100% expression and
alternative coverage, including exact reparameterized splits, tolerance-driven
subdivision, both quadratic extrema paths, and full, linear, constant, and
negative-discriminant cubic derivative cases. The package now spans 12
implementation lanes, reduces the high-consensus backlog to 294 slots, leaves
19 gaps in the Haskell lane, and unlocks the dependent `arc2d` port.

The sixteenth Haskell high-consensus slice is complete: `arc2d` now provides
G2D03 SVG endpoint and center arc forms, W3C endpoint-to-center conversion,
parametric evaluation, unnormalized tangents, analytic bounds for rotated
ellipse arcs, and cubic Bezier approximation. It composes the existing pure
Haskell `point2d`, `bezier2d`, and `trig` packages. Its package-native suite
exercises 25 examples with 99% expression and 100% alternative coverage,
including degeneracy thresholds, both sweep corrections, radius scaling,
nonzero rotation, positive and negative tight-bound extrema, zero sweep, the
quarter-circle magic controls, segmentation, and continuity. The package now
spans 12 implementation lanes, reduces the high-consensus backlog to 293 slots,
and leaves 18 gaps in the Haskell lane.

The seventeenth Haskell high-consensus slice is complete: `gradient-descent`
now provides the dependency-free ML02 stochastic-gradient-descent update with
explicit rejection of empty and length-mismatched vectors. Its package-native
suite exercises 11 examples with 100% expression and alternative coverage,
including the shared parity vector, singleton and multi-element inputs, zero
and negative learning rates, mixed gradient signs, input preservation, and all
validation paths. The package now spans 11 implementation lanes, reduces the
high-consensus backlog to 292 slots, and leaves 17 gaps in the Haskell lane.

The eighteenth Haskell high-consensus slice is complete: `perceptron` now
provides a pure sigmoid/BCE single-neuron classifier that composes the existing
Haskell `matrix`, `loss-functions`, and `activation-functions` packages. Its
package-native suite exercises 14 examples covering shared AND-gate
convergence, scalar and column labels, epoch-zero updates, deterministic
refitting, prediction guards, hyperparameter validation, feature shapes, label
counts, and finite-value checks. The package now spans 11 implementation
lanes, reduces the high-consensus backlog to 291 slots, and leaves 16 gaps in
the Haskell lane.

The nineteenth Haskell high-consensus slice is complete:
`type-checker-protocol` now provides immutable diagnostics and typed results,
a functional checker contract, and pure phase/kind hook dispatch with explicit
fall-through, exact-before-wildcard precedence, source-location helpers, and
clean diagnostic lifecycles. Its base-only package-native suite exercises 19
examples covering checker outcomes, partial typed ASTs, normalization, hook
ordering, wildcard and argument dispatch, error collection, and reusable
state, with 100% alternative and 99% expression coverage. The package now
spans 11 implementation lanes, reduces the high-consensus backlog to 290
slots, leaves 15 gaps in the Haskell lane, and establishes the shared contract
needed by later typed-frontend ports.

The twentieth Haskell high-consensus slice is complete: `paint-vm-ascii` now
provides a pure terminal renderer for the shared `paint-instructions` IR. It
maps scene coordinates through configurable character-cell scales, clips
visible filled rectangles into the scene buffer, trims terminal whitespace,
and rejects paths explicitly rather than returning incomplete output. Its
package-native suite exercises 13 examples covering shared defaults, filled
rectangles, clipping, transparent paints, default scaling, half-cell rounding,
zero-sized scenes, unsupported paths, invalid scales, scene dimensions, and
rectangle geometry. The package now spans 11 implementation lanes, reduces the
high-consensus backlog to 289 slots, and leaves 14 gaps in the Haskell lane.

The twenty-first Haskell high-consensus slice is complete:
`barcode-layout-1d` now provides the shared pure geometry layer for linear
barcodes. It validates alternating bar/space runs, expands binary and
narrow/wide patterns, computes inferred or explicit symbol spans and quiet
zones, and emits metadata-rich rectangle-only scenes through the existing
Haskell `paint-instructions` package. Its package-native suite exercises 18
examples with 97% expression and 89% alternative coverage across shared
defaults, both pattern families, custom ratios and markers, attribution,
symbol inference and descriptors, empty content, rendering geometry and
metadata, every validation family, and the deliberate text-shaping guard. The
package now spans 12 implementation lanes, reduces the high-consensus backlog
to 288 slots, leaves 13 gaps in the Haskell lane, and unlocks the dependent
Code 39, Codabar, ITF, UPC-A, EAN-13, Code 128, and `barcode-1d` ports.

The twenty-second Haskell high-consensus slice is complete: `itf` now provides
the pure Interleaved 2 of 5 encoder unlocked by `barcode-layout-1d`. It
validates non-empty even-length ASCII digit payloads, exposes typed digit-pair
patterns, interleaves first-digit bar widths with second-digit space widths,
and emits explicit start, data, and stop symbol geometry plus authoritative
symbology metadata. Its package-native suite exercises shared patterns, the
complete digit table, source attribution, exact module geometry, customized
paint output, metadata precedence, aliases, and both local and shared
validation paths. The package now spans 12 implementation lanes, reduces the
high-consensus backlog to 287 slots, and leaves 12 gaps in the Haskell lane.

The twenty-third Haskell high-consensus slice is complete: `code39` now
provides the pure linear-barcode encoder unlocked by `barcode-layout-1d`. It
normalizes lowercase input, validates the complete standard alphabet, protects
the reserved delimiter, exposes all 44 typed narrow/wide symbol patterns, and
emits attributed start, data, stop, and inter-character-gap runs through the
shared paint geometry. Its package-native suite exercises normalization,
educational errors, the complete symbol table, exact patterns and module
counts, semantic attribution, empty payloads, customized paint output,
metadata precedence, aliases, and both local and shared validation paths. The
package now spans 12 implementation lanes, reduces the high-consensus backlog
to 286 slots, and leaves 11 gaps in the Haskell lane.

The twenty-fourth Haskell high-consensus slice is complete: `codabar` now
provides the pure configurable-guard encoder unlocked by `barcode-layout-1d`.
It accepts body-only or explicitly guarded input, validates configurable `A`-`D`
start and stop choices, exposes all 20 typed binary symbol patterns, and emits
attributed start, data, stop, and inter-character-gap runs through the shared
paint geometry. Its package-native suite exercises guard insertion and
preservation, educational errors, the complete symbol table, exact module
counts, semantic attribution, empty payloads, customized paint output,
metadata precedence, aliases, and both local and shared validation paths. The
package now spans 12 implementation lanes, reduces the high-consensus backlog
to 285 slots, and leaves 10 gaps in the Haskell lane.

The twenty-fifth Haskell high-consensus slice is complete: `code128` now
provides the pure Code Set B encoder unlocked by `barcode-layout-1d`. It
validates printable ASCII, exposes the complete 107-pattern symbol table,
computes the required weighted modulo-103 checksum, and emits attributed start,
data, check, and stop runs through the shared paint geometry. Its package-native
suite exercises ASCII boundaries, educational errors, the complete pattern
table, reference values and checksums, empty payloads, exact module geometry,
customized paint output, metadata precedence, aliases, and both local and shared
validation paths. The package now spans 12 implementation lanes, reduces the
high-consensus backlog to 284 slots, and leaves 9 gaps in the Haskell lane.

The twenty-sixth Haskell high-consensus slice is complete: `upc-a` now provides
the pure retail-barcode encoder unlocked by `barcode-layout-1d`. It accepts
11-digit payloads or validated 12-digit codes, computes the required modulo-10
check digit, exposes all twenty left/right digit patterns, and emits the fixed
95-module start, digit, center, and end structure with typed source attribution
and explicit symbol spans. Its package-native suite exercises 23 examples with
91% expression and 81% alternative coverage, including reference checksums,
all standard patterns, computed and supplied checks, ASCII and length guards,
exact module geometry, metadata precedence, aliases, and shared validation
paths. The package now spans 12 implementation lanes, reduces the
high-consensus backlog to 283 slots, leaves 8 gaps in the Haskell lane, and
unlocks the dependent `ean-13` port.

The twenty-seventh Haskell high-consensus slice is complete: `ean-13` now
provides the pure retail-barcode encoder unlocked by `barcode-layout-1d`. It
accepts 12-digit payloads or validated 13-digit codes, computes the required
weighted modulo-10 check digit, exposes all thirty L/G/R digit patterns and all
ten leading-digit parity sequences, and emits the fixed 95-module guard and
visible-digit structure with typed source attribution and explicit symbol
spans. Its package-native suite exercises 26 examples with 92% expression and
78% alternative coverage, including reference checksums, every standard digit
and parity pattern, computed and supplied checks, ASCII and length guards,
exact module geometry, metadata precedence, aliases, and shared validation
paths. The package now spans 12 implementation lanes, reduces the
high-consensus backlog to 282 slots, and leaves 7 gaps in the Haskell lane.

The twenty-eighth Haskell high-consensus slice is complete: `sql-csv-source`
now provides the filesystem adapter for the existing pure
`sql-execution-engine`. It loads every CSV table through an explicit IO
boundary, preserves header order in an immutable data-source snapshot, handles
quoted commas, escaped quotes, embedded newlines, and CRLF, validates malformed
records, and coerces null, boolean, integer, finite real, and text values into
the shared SQL types. Its package-native suite exercises 19 examples with 89%
expression and 86% alternative coverage, including parsing failures, missing
directories and tables, typed scans, filters, ordering, null predicates, joins,
grouping, aggregates, limits, and total result wrappers. The package now spans
13 implementation lanes, reduces the high-consensus backlog to 281 slots, and
leaves 6 gaps in the Haskell lane.

The twenty-ninth Haskell high-consensus slice is complete: `zstd` now provides
the pure CMP07 educational Zstandard frame codec on top of the existing native
`lzss` package. It emits standard single-segment frames and 128 KiB raw, RLE,
and compressed blocks, encodes raw literal sections and the predefined
literal-length, match-length, and offset FSE tables, and strictly validates
headers, modes, truncation, backreferences, trailing data, and output limits.
Its package-native suite exercises 16 examples with 86% expression and 77%
alternative coverage, including exact cross-language compressed bytes, all
block families, multi-block RLE, dictionary-id and content-size header forms,
checksums, deterministic binary data, compression ratios, and malformed
frames. The package now spans 13 implementation lanes, reduces the
high-consensus backlog to 280 slots, and leaves 5 gaps in the Haskell lane.

The thirtieth Haskell high-consensus slice is complete: `barcode-1d` now
coordinates the six existing native Haskell symbology packages for Code 39,
Codabar, Code 128, EAN-13, ITF, and UPC-A. It normalizes user-facing
symbology names, preserves typed encoder and layout failures, forwards shared
paint options and Codabar guards, emits backend-neutral `PaintScene` values,
and renders them through the pure ASCII Paint VM without claiming an absent
native raster backend. Its package-native suite exercises 13 examples with
93% expression and 100% alternative coverage, including every route, default
selection, normalized spellings, unsupported names, custom geometry, custom
guards, typed failures, and ASCII backend errors. The package now spans 12
implementation lanes, reduces the high-consensus backlog to 279 slots, and
leaves 4 gaps in the Haskell lane.

The thirty-first Haskell high-consensus slice is complete: `http-core` now
provides the dependency-free NET03 semantic model with ordered duplicate
headers, bounded versions and status codes, body-framing hints, request and
response heads, content helpers, raw request-target/query handling, and
path-only route patterns. Its package-native suite exercises 19 examples with
96% expression and 95% alternative coverage across valid and malformed
versions, ASCII-only header matching, length overflow, content parameters,
message delegates, raw queries, repeated slashes, captures, and route
mismatches. The port also repairs Haskell scaffold naming/test conventions and
build-graph dependency discovery so the follow-on `http1` edge is visible to
incremental CI. The package now spans 12 established lanes, reduces the
high-consensus backlog to 278 slots, and leaves 3 gaps in the Haskell lane.

The merged thirty-second Haskell slice is `http1`, the direct NET04 consumer
of that merged `http-core` foundation. It closes one ready protocol edge,
validates the repaired Cabal dependency graph, and unlocks the remaining
Haskell high-consensus tail. Its security review also exposed a shared NET04
contract gap: established implementations need fail-closed framing for
transfer-encoding/content-length ambiguity, strict wire grammar, bounded head
resources, response-method context, and redacted typed errors. The Haskell
slice establishes that contract; `http1-safe-framing-backfill` tracks the
remaining established lanes as a separate dependency-shaped tranche.

The same slice exposed legacy Haskell scaffold capability metadata:
`required_capabilities.json` is still emitted as an incomplete one-field
object, and the merged `http-core` package retains that shape. The current
`http1` package carries a schema-valid v1 pure-computation manifest;
`haskell-scaffold-capability-schema` tracks generator golden/schema coverage
and the existing-package backfill.

The selected thirty-third Haskell slice is that scaffold capability-schema
repair. It is a generator-level prerequisite for the remaining `event-loop`
and `brotli` ports: the Go, TypeScript, and native Haskell scaffold paths must
emit schema-valid v1 manifests instead of propagating the legacy one-field
shape or omitting explicit scaffold metadata. The backfill covers all ten
existing invalid Haskell manifests. Eight are explicit pure-computation
profiles; `conduit` and `conduit-hello` carry proposed FFI, network, time,
environment, and standard-output declarations under the canonical taxonomy,
pending the Layer 5 review required for nonempty profiles.

That audit also discovered four independently owned follow-ups:
`haskell-capability-policy-audit` classifies the effective profiles of every
Haskell package and program that currently relies on an absent legacy
manifest; `haskell-scaffold-convention-reconciliation` brings the older
TypeScript Haskell Cabal/Hspec templates to full canonical parity; and
`scaffold-description-injection-hardening` closes structural delimiter
injection across all generated metadata and source-comment contexts.
`capability-schema-category-action-constraints` encodes the taxonomy's valid
category-action pairs in both schemas and every enforcement backend. Merged PR
#9363 completed that restriction-only contract and directly unblocked the
future OCaml analyzer, `adj-lang-cli` profile reconciliation, native Matter
controller review, and scaffold manifest repairs.

The implementation audit also separated a legacy migration instead of
silently forcing unlike files through the new schema. The tracked tree
currently has 2,885 `required_capabilities.json` paths: 150 top-level
dependency arrays, 82 metadata objects without a `capabilities` key, 164 object
manifests with string-list capabilities, and 2,489 objects whose capability
list is empty or contains structured entries. Of the last group, 2,316
currently validate as schema-v1 manifests. The structured entries total 373:
359 use valid current-vocabulary pairs and 14 use separately owned legacy
vocabulary such as `hardware`, `audio`, `ffi:export`, `process:spawn`,
`network:listen`, and `fs:read_write`; no current-vocabulary entry forms an
invalid cross-pair. The pending
`capability-manifest-legacy-shape-and-vocabulary-migration` item must classify
those semantic owners before migration, must not reinterpret build-dependency
arrays as security manifests, and must obtain Layer-5 review before changing
any nonempty authority profile.

The same post-merge audit found six additional repository-wide generator
blockers not owned by that migration: three valid underscore-bearing Go package
names rejected by the generator's narrow package-name rule and three reviewed
`ffi:call` manifests rejected by its duplicated vocabulary. The new
`capability-generator-all-mode-reconciliation` item follows legacy
classification, reconciles those rules without reopening the closed taxonomy,
and makes `--all --dry-run` a tested green repository contract.

Recommended family order:

1. Leaf algorithms and data structures.
2. Hashing, crypto, compression, and deterministic codecs.
3. Shared JSON/document/IR models and serializers.
4. Grammar-generated lexer/parser pairs.
5. SQL/storage packages in dependency order.
6. Compiler and VM families with their local IR, validator, encoder, and runtime
   dependencies in the same wave.

## Priority 3: Expand The Portable Core

After the high-consensus set is complete:

1. Complete packages already present in 8-9 languages.
2. Complete packages present in 5-7 languages.
3. Recompute the matrix after every merged wave.
4. Prefer families with existing cross-language fixtures and low dependency
   fan-out.
5. Add missing shared conformance fixtures before porting when current tests are
   language-specific and cannot prove equivalent behavior.

This phase covers 121 package identities and 911 current missing slots.

The DT02 graph-substrate slice adds pure Go and Rust
`multi-directed-graph` implementations alongside the existing Python and
TypeScript references. Both ports provide stable edge identity, parallel
directed edges, copied property bags, multiplicity-aware DAG algorithms,
package-native tests, capability declarations, and downstream integration with
their `neural-network` and `neural-graph-vm` packages. This moves the portable
package from two to four established implementation lanes; the remaining
language ports stay eligible for later dependency-shaped waves.

## Priority 4: Classify Sparse And Singleton Families

The singleton inventory is led by 572 Rust, 86 Python, and 84 TypeScript
packages. Classify families before opening implementation PRs.

The July 30-August 2 inventories added thirty-one Rust singleton identities that now
have explicit classification work in the loop state: `axiom-to-semantic-ir` is a
likely portable deterministic lowering; `http1-client` needs its portable
protocol core separated from native transport behavior; and
`venture-browser-core` needs a portable-core versus native-boundary review.
`smart-home-discovery-service`, `hue-integration`, and
`smart-home-runtime-store`, `smart-home-automation-runtime`,
`smart-home-zwave-integration`, the newly merged `smart-home-zwave-host` serial
host, `smart-home-mqtt-integration`, and `smart-home-zigbee-integration` are
native/service/storage-boundary applicability cases. The Zigbee adapter is pure
logic above coordinator transport and carries only an opaque `VaultRef`, so its
likely portable adapter core should be separated conceptually from a future
native coordinator host. The MQTT host also needs explicit capability metadata
and Vault-mediated credentials before that classification is complete; its deterministic
discovery/topic/entity/value/payload transforms are candidates for a separately
fixture-driven portable core. `venture-browser-macos` is an Apple-native
AppKit/CoreText/Metal host whose expected classification is `native-source`,
not a blind 14-lane port.

The twentieth identity, `venture-browser-windows`, is a mixed split rather than
a complete `native-source` exception. It duplicates deterministic session,
chrome, status, navigation, link, JSON projection, and Mosaic event-bridge logic
from the macOS host; an explicit follow-up moves that behavior into
`venture-browser-core` or a shared fixture-driven bridge. The residual Windows
DLL, WinUI adapter, Direct2D BGRA rendering, native text shaping, and C ABI remain
native-source. The same owner adds the missing capability profile and Windows
ABI, generated-project, pointer-lifetime, panic-containment, and pixel-buffer
validation; other lanes must not duplicate the native shell.

The twenty-first identity, `smart-home-camera-media`, currently mixes a portable
authorization and lease-policy core with host authority. Its first implementation
returns the secret snapshot/stream URI to the lease holder, accepts caller-asserted
identity and time, does not bind a lease to an endpoint generation, owns OS entropy,
and leaves endpoint/lease maps unbounded. The hardening owner replaces redemption
with a host-owned service that installs identity/time/nonce/executor authority once,
revalidates current grants, bounds snapshot bytes, owns broker-minted stream sessions,
retains failed teardown for reported retry, rejects URL userinfo and default plaintext,
binds leases to generations, and imposes global and per-principal quotas. An explicit
policy opt-in exists only for loopback fixtures, where query strings remain forbidden;
secure query tokens remain confined to the executor. A later fixture owner expands
only the resulting authority-free policy core.

The twenty-second identity, `smart-home-onvif-integration`, is also a mixed split.
Correlated discovery and SOAP parsing, deterministic UsernameToken construction,
origin policy, profile projection, and hostile input handling form a portable core.
UDP/DNS/TCP/TLS, trusted time and randomness, Vault credential leases, process I/O,
and reviewed endpoint allowlists remain native. Before extraction, the host must
stop following discovery- or device-controlled XAddr values across origins with a
fresh credential digest, fail closed on insecure non-loopback transport, and replace
ambient username/password environment variables with Vault-mediated credentials.
The nonempty native profiles then require separately tracked Layer 5 evidence.
Its current installation path also mutates bridge and camera endpoint state before
every profile, device, and entity has passed validation. A separate dependency-
ordered item adds validate-and-plan preflight plus atomic commit or rollback so a
late URI, quota, or registry failure cannot leave partial runtime state or rotate
previously valid endpoint generations.

The twenty-third identity, `smart-home-shelly-integration`, is another mixed
split. Gen2/Gen3 device-info and status parsing, authentication-required
classification, component projection, stable identifiers, capability/state
normalization, RPC envelope validation, and command planning are deterministic
portable-core candidates. mDNS, DNS/TCP, plaintext LAN HTTP, trusted time,
console I/O, origin allowlists, runtime effect application, and future Vault
credential delivery remain in the native Rust host. Before cross-lane expansion,
the host must install authenticated session/clock authority once, narrow public
arbitrary RPC access, bind discovery to reviewed private origins, defend DNS
rebinding, redact and bound device-controlled data, make installation and
command effects transactional or compensating, and declare truthful nonempty
runtime/test capability profiles. Authentication-enabled devices remain
fail-closed until a reviewed Vault-mediated flow exists; Layer 5 approval stays
a separate external gate. The same dependency audit found capability drift in
the shared Rust network substrates: `tcp-client` and `http1-client` claim empty
profiles despite concrete network calls, while `udp-client` and
`smart-home-discovery` lack manifests. A separate high-leverage owner corrects
those native boundaries before downstream approval.

The twenty-fourth identity, `smart-home-wled-integration`, follows the same
mixed pattern. `/json/si` DTO validation, master and segment projection, stable
identifiers, capability-bit interpretation, state normalization, brightness,
RGB and mirek conversion, and JSON command planning form the portable candidate.
mDNS, DNS/TCP, plaintext LAN HTTP, trusted time, console I/O, pairing/origin
policy, and runtime effects remain native. Before extraction, the host must stop
accepting caller-asserted identity/time and arbitrary public state updates, bind
discovery to reviewed private origins, defend DNS rebinding, bound/redact device
data, reject identity/segment collisions, reconcile returned device state, and
make runtime/device effects transactional or compensating. Truthful nonempty
host profiles and external Layer 5 evidence remain separate owners. A shared
follow-up consolidates the duplicated Shelly/WLED DNS, TCP, request encoding,
bounded response, chunked decoding, and error projection behind a native LAN-
HTTP executor while keeping `smart-home-local-http` a pure request planner.

The newest identity, `smart-home-nanoleaf-local-integration`, is another mixed
split rather than a blind port. Credential syntax and credential-free origin
configuration, bounded snapshot and state validation, stable identifiers,
capability and state normalization, RGB/HSV and mirek conversion, command
planning, verification, and hostile inputs are deterministic portable-core
candidates. mDNS, DNS/TCP, LAN HTTP execution, physical-presence pairing, token
and Vault handling, trusted time, endpoint approval, CLI I/O, authorization,
and SmartHomeRuntime mutation remain native-host responsibilities. Its portable
owner depends on the shared confirmed command-effect lifecycle so that fixtures
describe confirmed device state rather than optimistic runtime acceptance.

The fourteenth identity, `smart-home-home-assistant-migration`, is a mixed
boundary rather than an automatic fifteen-lane port. Its deterministic export
parsing, normalization, planning, diagnostics, IDs, fingerprints, and receipts
are portable-core candidates; runtime application, atomic filesystem writes,
CLI behavior, three Rust-only runtime dependencies, and the missing capability
manifest require explicit host-boundary and authority classification.

The fifteenth identity, `smart-home-home-assistant-export`, is another mixed
boundary. Its deterministic normalization and export-core logic are portable
candidates, while TLS/WebSocket transport, environment-token intake,
filesystem output, wall-clock metadata, console reporting, and its missing
capability manifest remain host-owned authority that must be classified before
any cross-language extraction.

The sixteenth identity, `smart-home-home-assistant-history`, is also a mixed
split candidate. Deterministic history DTO validation, ordering, fingerprints,
state projection, event planning, diagnostics, and receipts belong in a
fixture-driven portable core. WebSocket/TLS collection, runtime application,
artifact I/O, clock and console effects, and CLI orchestration stay in a Rust
native host. That host must declare reviewed capabilities, use Vault-mediated
token delivery and secure transport, enforce closed resource ceilings, redact
all error paths, and replace artifacts through no-follow durable atomic writes.
The urgent capability, Vault, transport, limit, redaction, and write hardening
depends only on the merged capability-taxonomy contract; it must not wait for
portable-core extraction. A later host refactor can depend on both the hardened
boundary and the extracted core.

The seventeenth identity, `smart-home-home-assistant-definitions`, is the same
kind of deliberate split rather than a blind fourteen-lane port. Its safe-
subset validation and normalization, state and time-pattern triggers,
conditions, scene/action mapping, ordering, uniqueness, fingerprints, reports,
and diagnostics are fixture-driven portable-core candidates. WebSocket and
HTTP/TLS collection, administrator credentials, artifact I/O, wall clock,
console output, and CLI orchestration stay in the Rust native host. The current
host lacks `required_capabilities.json`, accepts plaintext transports, reads an
ambient administrator token, persists server-provided error text, and creates
a predictable link-following temporary output. Repository-local transport,
Vault, limit, redaction, and durable no-follow write hardening must land without
claiming capability approval; a separately blocked Layer 5 item owns the
hardware-key-backed approval evidence, and the later host refactor depends on
both the portable core and hardened boundary.

The eighteenth identity, `smart-home-home-assistant-dashboard-migration`, is
another mixed split. Lovelace DTO validation and normalization,
reviewed-topology projection, standard-card compilation, ordered layout
flattening, entity mapping, diagnostics, summaries, fingerprints, blocking
decisions, receipts, and artifacts are fixture-driven portable-core candidates.
WebSocket/TLS collection, administrator credentials, filesystem and wall-clock
effects, console output, and CLI orchestration stay in the Rust host. That host
currently lacks capability metadata, accepts arbitrary plaintext endpoints,
reads an ambient token, persists server-controlled error text and resource URLs,
has no closed input or nesting limits, and uses predictable link-following
temporary output. Repository-local hardening is therefore independent of broad
classification; a separate blocked item owns Layer 5 approval, and a later host
refactor depends on both hardening and core extraction.

The nineteenth identity, `smart-home-dashboard-core`, is not a host-boundary
exception. It contains deterministic versioned dashboard, view, card, and
resource DTO parsing; identifier and duplicate validation; dry-run rejection;
applied-migration projection; and summaries without runtime capabilities. A
dedicated item owns its explicit empty capability profile, closed language-
neutral fixtures, and dependency-shaped cross-lane expansion. The Home
Assistant dashboard portable-core extraction depends on this native dashboard
contract so it does not duplicate the target representation. Controller
transport, dashboard serving, filesystem I/O, and other host effects remain
outside that portable contract.

The new `smart-home-matter-integration` is also a mixed split candidate rather
than a native-source exception: endpoint projection, report normalization, and
command planning are deterministic zero-capability logic, while D23 keeps the
residual `SmartHomeRuntime` integration Rust-canonical. The backlog now tracks
language-neutral fixtures and hardening for the portable core separately from a
future controller host that owns mDNS, commissioning, certificates, PASE/CASE
sessions, subscriptions, retries, Vault leases, and reviewed host capabilities.

### Likely portable Rust-led families

- `closure-*` compiler passes
- `dsp-*` algorithms
- `iir-*` IR passes and deterministic target emitters
- portable `image-codec-*` packages
- `state-machine-*` tokenization, serialization, and compilation
- language runtimes and frontends such as `r-*` and `twig-*`, when their
  dependency stacks are ready
- deterministic portions of `vault-*`, `adjudication-*`, and `smart-home-*`
- the new Axiom, IDL, and Q frontend/runtime stacks in dependency order
- deterministic SIR lowerings such as `idl-to-semantic-ir`,
  `q-to-semantic-ir`, and `scilab-to-semantic-ir`
- `html-to-layout`, after its document and layout dependencies are classified

`sir-bench` remains a likely tool/harness exception, while
`chief-of-staff-vault-runtime` needs an explicit domain/native applicability
review before any port is selected.

### Likely native, wrapper, or target-specific Rust-led families

- `*-bridge`, `*-capi`, `*-jni`, `*-napi`, and `*-native`
- `silicon-rust-*` bindings
- board firmware and physical transport packages
- OS paint/window backends
- CUDA, Metal, Vulkan, Direct2D, GDI, OpenCL, and similar accelerator/platform
  implementations

### Python-led families requiring classification

- CPU/ISA simulators and gate-level models
- JVM/CLR/BEAM artifact and runtime packages
- Prolog and logic-runtime stacks
- Tetrad, Twig, and Oct compiler backends
- native data-structure wrappers

### TypeScript-led families requiring classification

- the 57-package `forme-*` web/static-site family
- browser, IndexedDB, Vite, Canvas, Web Audio, and UI packages
- layout and document-to-paint packages
- Mosaic web emitters

For each family, add or identify a portable contract spec, dependency order,
reference implementation, shared fixtures, and explicit exception list before
the first port PR.

## Priority 5: Conformance And Regression Prevention

1. Give each portable family a language-neutral fixture corpus or oracle.
2. Add package-level conformance runners where directory presence currently
   masks API or semantic drift.
   Start with NET03 `http-core`: align raw request-target, query, and route
   behavior across the established ports that currently expose only the basic
   header/head surface.
3. Extend the parity reporter with explicit applicability data rather than
   hard-coding exceptions in reporting logic.
4. Fail CI on unclassified new package buckets.
5. Keep the canonical-collision CI gate enabled.
6. Add a policy check: a new portable singleton must include either another
   language implementation or a declared parity work item/classification.
7. Track package maturity separately from structural presence: manifest,
   source, tests, BUILD, README, CHANGELOG, conformance status, and last verified
   revision.

## Cross-Cutting Stream A: Build-Tool Parity

The build tool is itself a cross-language program and must follow the same
rules as portable packages: directory presence is not behavioral parity.

Executable front doors currently exist in 12 of the 15 established lanes:

- C# and F# under `code/programs/dotnet/`
- Elixir, Go, Haskell, Lua, Perl, Python, Ruby, Rust, Swift, and TypeScript
  under their respective `code/programs/<language>/build-tool` directories

Dart, Java, and Kotlin have no implementation. The F# entry point is currently
a thin facade over the C# engine rather than an independent native engine.
C and C++ are emerging lanes without build tools; OCaml must receive one before
it can graduate from emerging status. WASM is an execution target, Mosaic and
Twig are domain languages, and Starlark is a build language, so none enters this
program-level requirement without a separate applicability decision.

The existing implementations have materially drifted. The Go program is the
broadest reference, but even it has unfinished OS-aware structured command and
Windows execution work. Other ports differ on package discovery, ecosystem
resolvers, diff selection, hashing, Starlark, plan read/write, sharding,
toolchain detection, parallelism, and failure propagation. Specifications 12,
`build-plan-v1`, `build-plan-sharding`, 15, and B05 must be reconciled with
current code before more directory-only ports are added.

Delivery order:

1. Define `build-tool-conformance.md`, a versioned fixture schema, and stable
   machine-readable results for discovery, resolution, graph order, diff
   selection, hashing/cache, Starlark, plan interchange, sharding, execution,
   validation, platform BUILD selection, and toolchain detection. Completed by
   PR #9151.
2. Add a cross-implementation runner. The runner may orchestrate processes in
   Python, but the fixtures and canonical JSON are the behavior oracle. Start
   with non-execution domains and fail closed on every execution case.
   Completed by PR #9157 with the process-free bootstrap runner, implementation
   inventory, and closed discovery, resolution, graph, and plan corpus.
3. Expand the bootstrap's closed input/result schemas and positive plus
   adversarial corpus from discovery, resolution, graph, and plan into the
   remaining non-execution domains: diff selection, hashing/cache, Starlark,
   sharding, validation, toolchain detection, and CLI. The current selected
   tranche now provides a 111-case, 11-domain process-free corpus, including
   conservative unknown-path handling, typed cache states, inline-only
   Starlark loads, prerequisite-closed shard verification, the OCaml-aware
   toolchain registry, and fail-closed BUILD-file validation.
4. Define the process-free trusted-execution policy core: closed execution
   input/result records, reviewed corpus and adapter digests, explicit operator
   authorization, stable backend-unavailable results, and a backend interface
   with no host-execution fallback. This tranche validates authority but never
   executes fixture code. Completed by PR #9178.
5. Implement the Linux OCI backend first. The completed first tranche defines a
   closed identity for exact rootless Podman, `crun`, Conmon, OCI manifest/config,
   seccomp, shim, and invariant-probe artifacts; proves local non-remote
   rootless operation, cgroup v2 delegation, seccomp, exact binaries, and the
   already-present image; and constructs the exact no-pull, private-namespace,
   read-only, capability-free probe-container argv. It invokes neither that
   argv nor a fixture, and Linux remains unavailable. Completed by PR #9189.
6. Before trusted Linux execution, bind operator approval to the complete
   reviewed authority bundle, bind case selection to the exact hashed
   root-handle/no-follow snapshot, and normalize dependency-skip/result-state
   semantics. Then execute the runner-owned Linux invariant probe with
   aggregate cgroup CPU metering, combined streaming output/result accounting,
   hard writable-workspace semantics, cancellation, and verified
   whole-container kill/reap/removal. The completed first prerequisite
   replaced corpus-only approval with a
   process-free, domain-separated external bundle over the reviewed source
   revision, policy, schemas, authority verifier, Linux preflight backend, and
   external backend identity. This first closed profile binds no corpus or
   adapter and approves bytes for capability inspection only; it exposes no
   process handoff. PR #9208 added a separately domain-bound atomic exact-byte
   backend/import-closure loader and a one-shot isolated loadability worker; it
   runs no capability command. PR #9231 completed the separately authorized
   protected command broker: it retains verified statically linked Podman,
   `crun`, Conmon, and state descriptors; permits only the two closed preflight
   operations; confines pathname-backed execution to retained Podman with
   Landlock; streams one combined output ceiling; and owns delegated-cgroup
   descendant cleanup. PR #9270 closed anonymous executable memfds, `dlopen`,
   executable mappings, and descriptor-replenishment paths with separately
   reviewed kernel enforcement and Linux integration probes. A verifiable
   immutable runner-image TCB attestation is also required before the
   subsequent invariant-probe authority profile; this covers the protected
   interpreter, standard library/native extensions, libc, loader, container
   configuration defaults, and other immutable host dependencies outside the
   three explicitly bound runtime executables. Run
   protected probe enforcement to produce containment evidence, then add a
   distinct trusted-execution profile binding that evidence, the exact case
   snapshot, and one adapter before trusted-case enforcement. Only protected
   evidence can mark Linux
   ready.

   The attestation item is not implementable on the repository's mutable
   GitHub-hosted `ubuntu-latest` runner without circular self-attestation.
   `build-tool-immutable-runner-attester-provisioning` is therefore an explicit
   externally blocked prerequisite: infrastructure owners must select and
   provision the immutable or measured runner/image subject, measurement root,
   out-of-band attester trust root, protected no-secret workflow, receipt
   storage, and rotation/revocation procedure. The repository attestation item
   remains pending until that reviewed subject exists; neither item runs a
   capability probe, fixture, adapter, or invariant probe. Unrelated
   deliverable parity edges continue meanwhile.
7. Implement the native Windows boundary with AppContainer or LPAC plus Job
   Objects and root-handle reparse-safe filesystem operations. Keep macOS
   non-passing until a signed helper or isolated VM can prove the same
   filesystem, network, resource, and tree-termination guarantees;
   `sandbox-exec` alone is not sufficient evidence.
8. Add closed execution-semantics cases for command ordering, failure
   propagation, dependency skips, dry-run, jobs, resource locks, legacy shell
   behavior, and direct argv. Keep escape, network, environment, link-race,
   cancellation, and resource-exhaustion checks as runner-owned non-oracular
   probes.
9. Make the Go reference pass the contract, including structured Starlark
   context/commands and the B05 Windows executor contract.
10. Remediate existing ports in fixture-failure order: C#/F#, Python, Ruby,
   Swift, TypeScript, Elixir, Rust, Perl, Haskell, and Lua. Each independent
   engine is its own PR; a reviewed shared-engine exception must be explicit.
11. Add language-native Java and Kotlin implementations using shared JVM fixture
   infrastructure but separate engines, then add Dart.
12. Add the OCaml implementation after the lane foundation is stable.
13. Decide and document whether C and C++ require native build tools before
   either emerging lane graduates.
14. Gate completion in CI: every supported implementation runs the same
   conformance corpus on its applicable operating systems.

Merged PR #9368 completed the repository-owned
`build-tool-execution-status-normalization` slice. It replaces the duplicated
`dependency-skipped` schema term with normative `dep-skipped` and closes
contradictory command exit codes, package return codes, dry-run states, overall
outcomes, duplicate result identities, fail-stop ordering, and dependency
propagation. This bounded schema-and-validator repair lands before execution
cases, trusted execution, the Go oracle, OCaml's build substrate, or adapter
work consumes those records.

Merged PR #9371 completed `build-tool-execution-case-snapshot`. It defines the
typed selector and held-snapshot boundary, opens one bounded direct-member
corpus snapshot through a retained no-follow root, and preserves the exact
hashed bytes for later execution while rejecting path, rename, symlink,
hardlink, case/Unicode-alias, and post-digest substitution. This process-free
slice grants no execution authority and marks no backend or adapter ready.

Merged PR #9375 completed `dart-scaffold-capability-schema` with green Linux,
macOS, Windows, and CodeQL checks. The collision-checked post-merge inventory
found thirteen Dart-only 14-of-15 gaps. The serial loop selected
`dart-trig-wave` because PHY00 `trig` is a dependency-free numeric leaf and
PHY01 `wave` is its direct consumer. PR #9383 merged that pair with green Linux,
macOS, Windows, and CodeQL checks, and PR #9390 then merged the closed shared
PHY00/PHY01 oracle plus its always-on Dart consumer. The refreshed
collision-clean inventory has 11 remaining Dart-only frontier gaps and one
newly owned Rust Home Assistant definitions singleton. The next build-tool
execution successor remains `build-tool-bootstrap-execution-fixture`, which is
still blocked on the Linux OCI enforcement chain.

Final Dart review hardened the selected implementation for subnormal square
roots, infinity and signed zero, non-finite wave parameters and time, angular-
frequency overflow, zero-amplitude evaluation, and extreme finite products.
The shared PHY00/PHY01 corpus is now merged. The serial loop selected the
existing-lane PHY00 square-root audit next because it is the foundational
dependency and its tiny-normal, minimum-subnormal, maximum-finite, infinity,
NaN, and signed-zero cases are now normative. PR #9395 merged that foundational
repair with all 15 checks green. The collision-clean post-merge inventory then
selected `phy01-nonfinite-validation-backfill`; PR #9400 merged that explicit
consumer repair with all 15 checks green. The refreshed `20afefa7a` inventory
then selected `phy00-atan-tiny-signed-zero-cross-lane-audit` because its only
dependency is merged and the shared corpus can now express the cross-lane
negative-zero and subnormal-floor failures exactly.

The post-#9368 dependency audit also found that trusted authority requires one
held case and one selected in-image adapter even though both checked-in
inventories are empty and normal adapter work sits downstream of the execution
semantics corpus. The new `build-tool-bootstrap-execution-fixture` item follows
the snapshot, status normalization, and Linux OCI enforcing boundary, then
lands one inert schema-valid case plus one untrusted digest-bindable bootstrap
adapter without claiming conformance or execution permission. Trusted authority
and Linux trusted execution explicitly depend on that bootstrap; the full
cross-platform corpus remains downstream of every platform sandbox boundary.

Pull-request CI may validate the policy, schemas, digests, and fake-backend
tests, but it must not authorize execution from branch-modifiable code or
fixtures. Real sandbox probes run only from reviewed immutable revisions in a
protected `push`/manual workflow with read-only repository permissions, no
repository secrets, and an out-of-band approved authority-bundle digest. The
corpus digest remains an internal consistency identity and cannot authorize
execution. The trusted runner never pulls a container image and never falls
back to unsandboxed host execution.

The pure-domain security review also discovered three follow-on gates that must
land before final adapter parity:

- define a common inert CLI argv grammar and typed parse-result corpus; the
  current process-free CLI cases intentionally classify exit decisions only;
- add adversarial Starlark metering fixtures for fuel, recursion, allocation,
  load depth/count/cycles, and diagnostic output before any native evaluator
  can claim final Starlark conformance.
- model deterministic inline inputs and semantic oracles for dependency,
  standalone-prerequisite, Starlark-declaration, identity, toolchain, and path
  validation checks before admitting them to the closed v1 schema.

The 41622fa7 dependency audit made those three gates direct prerequisites of
`build-tool-go-oracle`; the merged pure-domain umbrella alone cannot make the
oracle ready while its CLI, Starlark-metering, and validation-oracle children
remain pending. The same audit split `ocaml-build-substrate-process-free-core`
from the execution-coupled OCaml substrate. Repository-owned OCaml discovery,
opam/Dune resolution, hashing, validation, shard cost, affected-node behavior,
and workflow markers can now land before external execution infrastructure,
while opam-switch serialization, canonical execution conformance, and OCaml
promotion remain gated on the Go oracle and trusted-execution contract.

The Linux OCI review discovered eight additional dependency gates:

- replace corpus-only approval with a domain-separated external authority
  profile. The first profile binds the exact source, policy, schemas, verifier,
  Linux preflight backend, and external identity for capability inspection
  only; later profiles add launcher, seccomp, image, shim, corpus, and adapter
  bytes only after those components are enforced;
- load the process-free backend only from its exact retained approved import
  closure, and execute process-owning code only through the separately bound
  broker, using atomic beneath-root handles rather than name-based imports or
  check-then-open traversal;
- require the retained Podman identity to declare static linkage, reject any
  malformed/non-amd64 ELF or `PT_INTERP`, and confine each brokered capability
  command with a mandatory Landlock execute ruleset whose only allow-rule is
  the exact retained Podman inode, so a dynamic loader cannot become a
  trampoline and Podman
  constructor hooks, pause helpers, and every other pathname-backed helper in
  the reviewed flow cannot escape the closed command authority;
- close anonymous executable memfds, executable mappings, and other
  in-memory-code paths with separately reviewed kernel enforcement before any
  invariant-probe evidence becomes authoritative;
- bind protected Linux evidence to a verifiable immutable runner-image TCB
  receipt before an invariant-probe authority can rely on ambient interpreter,
  libc/loader, configuration, or helper dependencies;
- execute only the exact direct corpus member bytes from the approved
  root-handle/no-follow snapshot, never a reopened caller path;
- normalize `dep-skipped` terminology and enforce result-state/return-code
  invariants before execution fixtures land;
- separate identity/preflight and exact command construction from actual
  containment enforcement, aggregate resource accounting, lifecycle cleanup,
  and protected runner-owned probe evidence.

The post-#9323 security audit also found that `adj-lang-cli` has a legacy
effective zero-capability profile despite filesystem, environment, process, and
stdout use in its runtime/tests, with PR #9324 adding another E2E instance.
Keep that unrelated repair in the dedicated `adj-lang-cli-capability-profile`
item: separate harness authority from the publishable runtime profile, declare
truthful capabilities, and obtain Layer-5 approval for every nonempty profile.

## Cross-Cutting Stream B: Introduce OCaml Safely

OCaml begins as an `emerging_implementation` lane. It must not silently change
the current 15-language denominator until its package, build, security, and CI
substrate is real. Use OCaml 5.2.1, opam 2.5.2, Dune 3.17.2 with Dune language
3.16, Alcotest 1.9.0, `bisect_ppx` 2.8.3, and `ocamlformat` 0.27.0.

Merged PR #9323 delivered `ocaml-lane-contract`. OCAML01 classifies
OCaml as a known emerging bucket, keeps it outside established coverage and
missing-slot calculations, and derives the reporter's upper completion-band
bound from the established-language count. This is the denominator-safe
prerequisite for every remaining bootstrap step.

Bootstrap order:

1. Add an OCaml lane contract. Make the reporter's hard-coded `10-15`
   completion band denominator-safe, classify OCaml as emerging, and test that
   OCaml packages create neither unknown buckets nor 15-lane missing slots.
   Complete in PR #9323.
2. Add complete Go and TypeScript scaffold templates plus golden tests,
   repository ignores, capability metadata/schema support, and the
   `code/packages/ocaml/` lane README. Complete in PR #9336.
3. Provision the exact direct OCaml/opam/Dune/Alcotest/`bisect_ppx`/
   `ocamlformat` toolchain on Ubuntu, macOS, and Windows; lock the
   opam-repository/switch transitive solver state separately per runner family;
   verify lock and package-receipt digests against a fresh solve; and run both
   generated scaffold kinds without skips. Hosted-runner image metadata is
   diagnostic evidence only and is not an immutable host-image attestation.
   Complete in merged PR #9354. Final runs 30605415709, 30605415708,
   30605413650, and 30605415829 prove the contract, three fresh solves, three
   locked fixtures, repository builds, and CodeQL detection are green.
4. Add the process-free OCaml build substrate now that the cross-platform
   toolchain evidence and pure-domain corpus are complete: discovery,
   opam/Dune dependency resolution, source hashing, language detection,
   validator support, shard cost, affected-node behavior, and CI workflow
   markers. Keep opam-switch serialization and canonical execution
   conformance in the separately tracked execution-coupled tranche, which
   remains gated on `build-tool-go-oracle`.
5. Exercise the full path with a real dependency chain:
   `logic-gates`, then `graph -> directed-graph -> state-machine`. Every package
   needs native tests, formatting, measured coverage, README, changelog,
   capability metadata, opam/Dune manifests, and BUILD/BUILD_windows.
6. Add an OCaml capability analyzer over compiler-libs ASTs, covering process
   execution, dynamic loading, unsafe marshaling, and `Obj.magic` under the
   repository's explicit capability/exception policy after the shared
   category/action constraint schema is complete.
7. Implement the OCaml build tool on `directed-graph` and require two-way
   build-plan interchange plus the shared conformance corpus.
8. Promote OCaml into the implementation denominator only when Ubuntu, macOS,
   and Windows run real tests without a skip path, the representative chain and
   build tool are green, capability enforcement is active, and the generated
   16-lane backlog has been explicitly reviewed.

After promotion, start the OCaml portable-core queue with `http-core -> http1`,
then recompute dependency-shaped high-consensus work alongside the existing
Dart, Java/Kotlin, Swift, and Haskell lanes.

## Post-#10564 Refresh and Canonical-CBOR Selection

External review merged PR #10564 at `ded37da774f13dda0755bde4da2751184732a361`
after every required check passed. The collision-checked `2a6dab3ae0` refresh is
identity-neutral: 15 established lanes still contain 1,298 normalized
identities and 4,454 slots, with 173 high-consensus identities and 269 missing
slots, 848 singletons and 11,872 singleton gaps, 652 Rust singletons, zero
canonical collisions, and zero unknown buckets.

The intervening range nevertheless exposed behavior ownership that the
directory inventory cannot express:

- `semantic-ir-sys-write-portable-conformance` owns SIR28 stream selection,
  terminators, recursive unpacking, frontend lowering, and backend/runtime
  behavior. SIR28 §7 (removal of the legacy bare `print`/`puts` paths) is
  now complete across all 7 backends, including the JavaScript backend's
  load-bearing SIR23 evaluator special-case and the 6 external CAS-frontend
  crates whose test harnesses depended on it.
- `gc-core-portable-conformance` owns moving-minor pacing, remembered sets,
  barriers, tagged layouts, relocation, and fixups. Separate dependent owners
  cover VM integration, the native C ABI, and the target-specific LLVM runtime
  bridge; the native reviews are not autonomous parity targets, and the core is
  temporarily blocked while PR #10593 overlaps it.
- Python Haskell/Java/Kotlin filter exposure now waits for a dedicated
  field-aware Cabal/Gradle resolution tranche. Accepting a filter while comments
  and descriptive fields can create dependency edges would make the apparent
  language support unsafe.
- Data-store clock remediation now follows one shared injected-clock fixture;
  the earlier Lua-first dependency direction could not establish cross-lane
  TTL, expiry, reset, and provider-consistency semantics.

The loop selected `canonical-cbor-language-neutral-conformance` next. It is
dependency-free, pure, has no live PR overlap, and unlocks established-lane
canonical-CBOR parity, both Vault format roots, and the wider Vault application
and CLI chain. The tranche is not documentation-only: the C, C++, and Rust
reference encoders currently sort but retain duplicate encoded map keys and do
not bound encoder depth or output size. CBR01, the shared fixture, and all three
reference consumers must therefore define and enforce one fallible,
payload-blind contract before those implementations can serve as parity
oracles. The fourteen missing established implementations remain a separate
dependent item.

A late pre-publication refresh rebased this tranche onto `6c3b19b27b35`. The
intervening GC, HTML, Mermaid, Mosaic, SIR28, ADJ, Spanish, and handwriting
changes modify only existing package identities. Merged PR #10599 adds the
empty-capability Rust `vault-pm-audit` singleton, so the collision-checked
inventory is now 1,299 identities and 4,455 slots, with 849 singletons, 11,886
singleton gaps, 653 Rust singletons, and zero collisions or unknown buckets.
The new `vault-pm-audit-portable-conformance` owner waits on canonical-CBOR and
Vault domain/format parity; it also becomes a dependency of the application
owner. Open PR #10608 overlaps only that downstream application integration,
while open #10605 and #10607 overlap the already blocked HTML and GC owners.
None overlaps canonical CBOR.

## Post-#10612 Refresh and Python Resolver Selection

External review merged ready-for-review PR #10612 at
`431a8fcc829724507cc50f707ec9b7c0f8c39824` after all required checks passed.
The late collision-checked `44dc4b563ef` refresh covers 15 established lanes,
1,300 normalized implementation identities, and 4,456 established slots. It
reports 173 high-consensus identities with 269 missing slots, 850 singletons
with 11,900 missing singleton slots, 654 Rust singletons, zero canonical
collisions, and zero unknown buckets.

The range from the original `96f58be046bb` selection base through this late
refresh contains only identity-neutral HTML, Mermaid, Mosaic, ALGOL, ADJ, and
human-language changes. None overlaps the resolver or shared fixture paths.

The sole topology addition since the prior inventory is the Rust
`smart-home-frigate-snapshot-host` from merged PR #10602. It directly composes
Human Approval, Vault and sealed-secret reads, wall time, OS entropy,
reviewed-address-pinned TCP/TLS, credential and cookie custody, snapshot I/O,
logout, and cleanup without `required_capabilities.json`. A blocked native
authority owner now tracks that review; the package is not an all-language
port. Deterministic installed-identity, approval ordering, request and envelope
construction, response bounds, cleanup, and stable errors remain assigned to
the camera-media portable core.

The canonical-CBOR umbrella became dependency-ready, but a fourteen-language
change is not one reviewable implementation unit. The state graph now treats
that item as a blocked completion umbrella over selectable per-lane children,
pairing only the shared .NET and JVM toolchains. This preserves the Vault
dependency chain without forcing one oversized parity PR.

The loop selected `build-tool-python-haskell-gradle-field-aware-resolution`.
It is a bounded, standard-library-only repair of an affected-plan trust
boundary: Python previously scanned whole Cabal files, registered only legacy
prefixed Haskell names, and line-matched Gradle settings. The tranche consumes
the shared Haskell, Java, and Kotlin fixtures, accepts only one root Cabal
manifest and its `build-depends` fields, registers directory and declared Cabal
aliases, and lexically matches real multiline `includeBuild` calls against
same-lane discovered roots without opening referenced targets. The downstream
Python language-filter owner remains pending until exact Python-versus-Go graph
equality is proven.

## Post-#10751 Refresh and Haskell/JVM Filter Selection

External review merged ready-for-review PR #10751 as
`c4e8e8399f1a102651cec4002cde84ca1c1aa133` after all 20 checks reached a
terminal success, neutral, or expected skipped state with no failures. The
collision-checked `38ecfd9ff014` late refresh covers 15 established lanes, 1,301
normalized implementation identities, and 4,457 established slots. It reports
173 high-consensus identities with 269 missing slots, 851 singletons with
11,914 missing singleton slots, 655 Rust singletons, zero canonical collisions,
and zero unknown buckets. The tranche was selected at `a4ad42e1f71a`; the
intervening curriculum, HTML, Mermaid, Mosaic, ALGOL, Vault, and Semantic-IR
changes modify only existing identities and do not overlap this tranche.

The sole topology addition is Rust `smart-home-controller-runtime` from merged
PR #10791. It is an authority-free orchestration package over an injected
storage backend and caller-supplied timestamps, so a new portable-conformance
owner tracks its clone-mutate-persist-publish transaction, atomic combined
snapshot, CAS, rollback, serialization, and stable-error behavior. Concrete
storage, HTTP, scheduling, and worker authority remains with native hosts.

The audit also found that Python still omits the established C#, F#, and Dart
lanes from discovery, dependency resolution, and filtering. Separate pending
.NET and Dart field-aware owners now track the shared project-file and pubspec
fixture work; those manifest families do not widen this Haskell/JVM tranche.

The loop selected `build-tool-python-haskell-language-filter`. The merged
field-aware resolver work now makes 206 Haskell, 129 Java, and 133 Kotlin build
roots safe to select explicitly. This tranche consumes the shared discovery
registry, recognizes only exact `packages|programs/<language>` buckets,
preserves `<language>/programs/<name>` identities, and derives CLI choices from
the canonical registry. Canonical-CBOR lane children remain ready, but each
advances only one dependency of the twelve-child completion umbrella.

## Post-#10828 Refresh and .NET Resolver Selection

External review merged ready-for-review PR #10828 as
`399a87fff348db21258b14a6410fef6612d4ed41` after all 20 required, neutral, and
expected skipped checks reached a terminal success state. The collision-checked
refresh covers 15 established lanes, 1,302 normalized implementation identities,
and 4,458 established slots. It reports 173 high-consensus identities with 269
missing slots, 852 singletons with 11,928 missing singleton slots, 656 Rust
singletons, zero canonical collisions, and zero unknown buckets.

The sole topology addition since the prior inventory is Rust `task-mosaic-app`
from merged PR #10859. This TaskApp-specific cdylib/rlib maps the authored MIL
slot and event vocabulary to the separately owned pure `task-core` engine,
uses `mosaic-app-runtime` for deterministic transactions and snapshots, and
exports the reviewed fixed `mosaic-app-capi` ABI to generated native hosts. A
blocked native-wrapper applicability owner now tracks complete mapping,
rollback, stable errors, ABI symbol/lifetime/panic behavior, platform evidence,
and missing empty capability metadata. Reusable runtime and domain semantics
remain with their existing portable owners; fifteen application-specific ABI
copies would not be honest parity.

The dependency/leverage pass selected
`build-tool-python-dotnet-field-aware-resolution-and-filter`. Its Haskell/JVM
filter prerequisite is now merged, and the existing language-neutral .NET
fixtures define one closed `ProjectReference` grammar for both established
lanes. A single bounded standard-library tranche therefore makes 198 C# roots
with 238 edges and 197 F# roots with 239 edges safe to discover, resolve, and
filter while preserving package/program identities and the shared `dotnet`
toolchain mapping. The 83-root Dart manifest family and Windows atomic plan
replacement remain separate owners. Canonical-CBOR lane children remain ready,
but none independently unlocks their twelve-child completion umbrella.

A late pre-publication rebase moved the tranche to `7bb2b558c85`. The
intervening Mermaid, language-ladder, ALGOL, ADJ, and final ADJ state-only
changes, plus Vault named-target integration, modify only existing identities
and do not overlap this work. The collision-checked counts remain exactly 1,302
identities, 4,458 slots, 852 singletons, 11,928 singleton gaps, 656 Rust
singletons, zero collisions, and zero unknown buckets.

## Post-#10936 Refresh and Dart Resolver Selection

External review merged ready-for-review PR #10936 as
`ec9182513348ad5850a94bb28cb6ffd503dbd224` after all 20 required, neutral, and
expected skipped checks reached a terminal success state. The collision-checked
refresh at `86f47eb4c343e4b9b29039fc62282919d42e1bf8` covers 15 established
lanes, 1,303 normalized implementation identities, and 4,459 established
slots. It reports 173 high-consensus identities with 269 missing slots, 853
singletons with 11,942 missing singleton slots, 656 Rust singletons, zero
canonical collisions, and zero unknown buckets.

A late pre-publication rebase onto
`8f1b3a4f04fbf483f6af73ecb4f245fa3d542cff` added only unrelated curriculum,
Wasm-specification, and ADJ fact rows. The collision-checked inventory was
regenerated there with the same counts and no newly unowned parity topology.
Two further unrelated human-language and ALGOL commits advanced the final base
to `3786127cf65f09be93a374dfcb8c9b3f3d7c0dda`; a second collision-checked
refresh again produced exactly the same topology and counts.

The sole established-lane topology addition is TypeScript `image-codec-png`
from merged PR #11088. IC18 makes the bounded byte-array codec portable, but it
also exposes a prerequisite contract gap: CMP09 records TypeScript as the only
ZIP lane that exports raw RFC 1951 encode, counted decode, output caps, and
CRC-32. New pending owners therefore first fixture and expose those ZIP-owned
primitives across all established lanes, then consume them with the all-lane
`pixel-container` contract and a language-neutral PNG corpus to close the
fourteen missing codec slots. New Mosaic standard-foundation and control
packages remain domain-language identities outside the established denominator
and continue under their existing UI38 owners.

The dependency/leverage pass selected
`build-tool-python-dart-field-aware-resolution-and-filter`. Python's build tool
now handles every established implementation lane except Dart, while the shared
fixture already closes the permitted root `dependencies` and
`dev_dependencies` grammar and the Go oracle resolves all 83 Dart roots. This
bounded tranche adds safe discovery, alias resolution, graph equality, and a
real `--language dart` filter without widening the independently owned Windows
plan-overwrite path. The new ZIP primitive owner is broader fifteen-lane work,
and direct PNG parity remains blocked on that prerequisite.

## Post-#11111 Refresh and Windows Atomic Plan Selection

External review merged ready-for-review PR #11111 as
`91a6026f3e18d3320be96fa6b533059441395e39` at
2026-08-13T06:50:57Z after every required CI and CodeQL check reached terminal
success or an expected skip. The collision-checked refresh, recertified after
rebasing at `286c1549b11c516e4ccce83281ea675102849f68`, covers 15 established lanes,
1,305 normalized implementation identities, and 4,461 established slots. It
reports 173 high-consensus identities with 269 missing slots, 855 singletons
with 11,970 missing singleton slots, 658 Rust singletons, zero canonical
collisions, and zero unknown buckets.

The two new identities are Rust `wasm-wast-parser` and the currently
fixture-only Rust `wasm-conformance` root. Their deterministic parser and
directive/report behavior are portable candidates, while corpus fetching,
filesystem traversal, and baseline persistence are native authority. New
pending owners therefore separate the language-neutral WAT/WAST corpus, its
established-lane ports, the portable conformance directive/report core, and a
blocked host-authority review. External PR #11146 edits both Rust roots, so all
four owners remain selection-blocked until that PR is terminal and a fresh
inventory removes the overlap.

The leverage pass also decomposed the broad ZIP raw-RFC1951 item. A new neutral
foundation owns the fallible raw encode, capped decode, exact counted decode,
incremental CRC-32, malformed-stream, interoperability, and stable-error
corpus. The completion umbrella now depends on that foundation and remains
blocked until it is split into reviewable toolchain-shaped lane children.
TypeScript stays the reference consumer, and direct PNG parity remains blocked
on completion of the all-lane ZIP umbrella.

The selected independent item is
`build-tool-python-plan-atomic-overwrite-windows`. Python already writes a
temporary sibling but uses a rename operation that cannot replace an existing
destination on Windows, so a second `--emit-plan` to the same path fails with
`WinError 183` and cleans up the newly written temporary plan. A bounded
standard-library tranche adds a language-neutral repeated-write fixture,
proves exact replacement and temporary cleanup, switches Python to the
portable replace primitive, and leaves the broader ZIP, OCaml, WebAssembly,
and lane-port programs untouched.

The neutral overwrite fixture also exposes a residual multi-writer gap. Go and
several other build-tool implementations still rename over an existing path or
truncate the destination directly. A new pending completion owner depends on
the Python slice, inventories every `plan_v1_write` front door, and will be
decomposed by shared engine or toolchain before selection. This PR deliberately
does not change those independent implementations.

## Post-#11162 Refresh and ZIP Raw-Conformance Selection

External review merged ready-for-review PR #11162 as
`d6cfc5a758af417620b6b0ad48652794fef8b45d` at
2026-08-13T09:07:42Z after every required CI, CodeQL, and neutral auxiliary
check reached terminal success or an expected skip. The final reviewed head is
`e722f96ee5e7b4a305a34146171cbb35579ce8cd`. A fresh collision-checked report
at `5574bc77277b1b047d09d6e696f9427e2a8c4ba7` covers 15 established lanes,
1,305 normalized implementation identities, and 4,461 established slots. It
reports 173 high-consensus identities with 269 missing slots, 855 singletons
with 11,970 missing singleton slots, 658 Rust singletons, zero canonical
collisions, and zero unknown buckets.

The intervening ADJ fact-library commit is outside the established
implementation denominator, so the topology and every stored count remain
unchanged. No newly unowned package identity was discovered. The residual
cross-writer plan-publication gap remains with the existing
`build-tool-plan-atomic-overwrite-remaining-writers` owner, which is now
dependency-ready but must be decomposed by shared writer engine before
selection. The four WebAssembly parser and conformance owners remain blocked
while external PR #11146 overlaps their Rust oracle and host surfaces.

After selection, PR #11146 merged externally as `93a27bfec2`. A late
collision-checked refresh at `e57bec074440fd7b3b4198597b42d3c059f659c4`
remains exactly 1,305 identities and 4,461 slots with zero collisions and zero
unknown buckets. The temporary overlap flags are therefore cleared from the
WAST neutral contract and portable conformance core. Their dependency chain,
the established-lane umbrella decomposition requirement, and the separate host
authority review still prevent overlapping or over-broad selection.

The dependency/leverage pass selected
`zip-raw-rfc1951-language-neutral-conformance`. This dependency-free foundation
closes the fallible raw encode, capped decode, exact counted decode,
incremental CRC-32, malformed-stream, foreign-interoperability, and stable
payload-free error contract before any lane port begins. It also corrects the
RFC 1951 distance-header boundary: dynamic blocks may advertise all 32 distance
code-length slots, while reserved symbols 30 and 31 fail only if decoded.
Completing this tranche makes the reviewable ZIP lane children ready and, once
their umbrella closes, unlocks the fourteen missing established PNG codec
slots. It does not widen into those lane ports, the remaining build-plan
writers, OCaml promotion, or the externally owned WebAssembly work.

## Post-#11202 Refresh and Dart ZIP Raw-Conformance Selection

External review merged ready-for-review PR #11202 as
`0eabbd60beba51de34693b35980cd0f2366497b9` at
2026-08-13T10:31:33Z after every required CI and CodeQL check reached terminal
success or an expected skip. The final reviewed head is
`2909069e3fe7665bd08fe46ccaa2daec3094c5fc`. A fresh collision-checked report
at the merge revision covers 15 established lanes, 1,305 normalized
implementation identities, and 4,461 established slots. It reports 173
high-consensus identities with 269 missing slots, 855 singletons with 11,970
missing singleton slots, 658 Rust singletons, zero canonical collisions, and
zero unknown buckets.

The merge is topology-neutral: it tightens the established TypeScript ZIP
package and adds the closed neutral fixture without creating, removing, or
reclassifying an established package identity. No newly unowned gap was
discovered. The completion umbrella is now decomposed into twelve reviewable
toolchain-shaped children: .NET, Dart, Elixir, Go, Haskell, JVM, Lua, Perl,
Python, Ruby, Rust, and Swift. Every child depends only on the merged neutral
contract, and the umbrella remains blocked until all twelve have merged.
Direct PNG parity continues to depend on that completed umbrella.

The dependency/leverage pass selected `zip-raw-rfc1951-dart-lane-parity`.
Dart's existing pure in-memory ZIP codec already handles stored, fixed,
dynamic, and multi-block streams, bounded output, CRC-32, and foreign ZIP
interoperability. This bounded tranche therefore focuses on the closed raw
API, exact counted consumption, stable payload-blind errors, hardened malformed
stream rejection, all 34 neutral cases, and explicit empty capability
metadata. No live PR overlaps the Dart ZIP package, shared fixture, state, or
roadmap, and the historical stale cross-lane ZIP branch does not touch Dart.
The remaining eleven ZIP children stay pending and independently reviewable.

## Post-#11218 Refresh and Rust ZIP Raw-Conformance Selection

External review merged ready-for-review PR #11218 as
`0338e4f2558f9df9eebdcbba43d67187de4aafe2` at
2026-08-13T18:11:16Z after every required CI and CodeQL check reached terminal
success or an expected skip. The final reviewed head is
`a35a466ab45426195f6656cf157f3264dabde6fb`. A fresh collision-checked report
at live main `d9857363b1bfd5c99ffaecb01d1007e4eeb307af` still covers 15 established
lanes, 1,305 normalized implementation identities, and 4,461 established
slots. It reports 173 high-consensus identities with 269 missing slots, 855
singletons with 11,970 missing singleton slots, 658 Rust singletons, zero
canonical collisions, and zero unknown buckets.

The Dart merge changes only the established ZIP root, and the six later main
commits change only existing ALGOL, RISC-V, human-language, ADJ, and Mermaid
roots. None creates, removes, nor reclassifies a package root, so no new owner
is required before selection.
The Dart child is now merged, the ten other non-Rust ZIP children remain
pending, and the portable ZIP umbrella stays blocked on all twelve children.

The dependency/leverage pass selected `zip-raw-rfc1951-rust-lane-parity`.
Rust is the only remaining child whose established DEFLATE decoder already
handles stored, fixed, dynamic, multi-block, and full-window streams. The
reviewable slice therefore adds strict counted and capped decoding, typed
payload-blind errors, complete malformed-Huffman validation, ZIP-owned raw
entry points, all 34 neutral fixtures, and explicit empty capability metadata
without duplicating the codec. No live PR overlaps the Rust ZIP or DEFLATE
packages, shared fixture, state, or roadmap.

## Post-#11334 Refresh and Go ZIP Raw-Conformance Selection

External review merged ready-for-review PR #11334 as
`d4da597ab710e300598bb213e1a475c69eb01d71` at
2026-08-13T19:06:30Z after every CI, CodeQL, and auxiliary check reached
terminal success or an expected skip. The final reviewed head is
`b8c519514d4ed05ceae10955481d455104d5ff6f`. A fresh collision-checked report
at the merge revision covers 15 established lanes, 1,307 normalized
implementation identities, and 4,463 established slots. It reports 173
high-consensus identities with 269 missing slots, 857 singletons with 11,998
missing singleton slots, 658 Rust singletons, zero canonical collisions, and
zero unknown buckets.

The Rust merge itself is topology-neutral, but external PR #11224 merged
between the prior inventory and #11334 and added two TypeScript singleton
roots: `path-raster` and `script-ductus`. That accounts exactly for the two new
identities, two new slots, two new singletons, and 28 new singleton gaps. Both
are portable pure in-memory engines rather than native-authority exceptions.
The refreshed backlog therefore adds neutral-contract and blocked
established-lane completion owners for each. Path rasterization will close
P2D08 geometry, coverage, determinism, hostile-input, and work-bound fixtures
before fourteen lane ports reuse the all-lane pixel container. Script ductus
will close bounded TrueType parsing, authored-stroke provenance, glyph
verification, filmstrip, and XML-safety fixtures; every port must consume one
shared authored curriculum corpus instead of copying cultural claims. Both
completion umbrellas must be decomposed before selection.

The Rust ZIP child is now merged, leaving ten pending children: .NET, Elixir,
Go, Haskell, JVM, Lua, Perl, Python, Ruby, and Swift. The portable ZIP umbrella
and direct PNG parity remain blocked on those children. No live PR owns ZIP,
DEFLATE, the neutral fixture, state, or roadmap paths.

The dependency/leverage pass selected `zip-raw-rfc1951-go-lane-parity`. Go is
the cleanest single-toolchain child with no live or historical branch overlap;
the stale cross-lane ZIP residue touches only .NET, Haskell, and JVM paths.
Go's existing ZIP-owned codec already emits fixed blocks and decodes stored and
fixed blocks, so this coherent tranche adds dynamic Huffman decoding, exact
counted consumption, caller caps, typed payload-blind errors, strict malformed
stream validation, all 34 neutral fixtures, compressed-payload cavity
rejection, and explicit empty capability metadata without duplicating DEFLATE.

Before publication, the tranche rebased cleanly over eleven unrelated mainline
commits through `0d608f5054c9723e9df05f1c59941bee2ba5c3bf`. The refreshed
collision gate reports 1,308 identities, 4,464 slots, 858 singletons, 12,012
singleton gaps, 659 Rust singletons, zero collisions, and zero unknown buckets.
The sole topology addition is Rust `chief-of-staff-trust-checker` from PR
#11341. It is a zero-authority policy evaluator over an injected trusted
approval provider, so a new portable-conformance owner now covers bounded
resources, maximum-tier reduction, canonical tier/timeout behavior, assurance,
redacted failures, receipts, explicit empty capability metadata, and eventual
established-lane ports after the existing Chief tool-API contract. Native
notification, biometric, hardware-key, clock, and platform adapters remain
outside that portable core. This dependency-shaped discovery does not displace
the already in-progress Go ZIP child.

Ready-for-review PR #11354 publishes the validated Go tranche from
`ca344c4f104f6989a6b820b7e168f2b328f51619`. GitHub reports the PR open,
non-draft, and mergeable; CI, CodeQL, and auxiliary checks are queued, so the
loop returns to monitor-only behavior until those checks reach a terminal state.

## Post-#11354 Refresh and Ruby ZIP Raw-Conformance Selection

External review merged ready-for-review PR #11354 as
`08ae46bfee81a4df0c05e0422ed0657ba1bcad4d` at
2026-08-13T20:59:19Z after every required CI, CodeQL, and auxiliary check
reached terminal success or an expected skip/neutral conclusion. The final
reviewed head is `6d0e5ec4f91f0de4c0619acdb40ed1bba174ae66`.
A fresh collision-checked report at live main
`264347eee15b09d17174a6774923415683aea04e` covers 15 established lanes,
1,310 normalized identities, and 4,466 slots. It reports 173 high-consensus
identities with 269 gaps, 860 singletons with 12,040 singleton gaps, 661 Rust
singletons, zero collisions, and zero unknown buckets.

The Go merge itself is topology-neutral. External PR #11362 added the two new
Rust roots `modbus-protocol` and `smart-home-modbus-tcp-integration`, accounting
exactly for two identities, two slots, two singletons, and 28 singleton gaps.
The pure bounded Modbus read codec now has a portable conformance/port owner.
The mixed smart-home integration is split between an injected-transport core
owner for validation, register decoding, deterministic projection, and
authorization ordering, and a blocked native-authority review for DNS/TCP,
timeouts, plaintext origin policy, CLI I/O, capability truthfulness, and
partial-mutation risk. No Modbus write authority will be manufactured. Later
human-language and ADJ merges through the live revision add no package roots;
the late pre-publication refresh includes HL12 correction #11378, Spanish
curriculum step #11377, the existing-root ALGOL change #11380, and the ADJ loop
state update #11382 without changing the parity topology.

The Go ZIP child is now merged, leaving nine pending children: .NET, Elixir,
Haskell, JVM, Lua, Perl, Python, Ruby, and Swift. No live PR owns parity state,
the roadmap, ZIP, or the neutral fixture. The only stale parity-adjacent branch
still touches .NET, Haskell, and JVM ZIP paths, so it remains unowned residue.

The dependency/leverage pass selected `zip-raw-rfc1951-ruby-lane-parity`.
Ruby is the smallest clean standalone package/toolchain without live or stale
overlap. Its ZIP-owned decoder still rejects dynamic Huffman streams, making
this a genuine codec-contract port: strict dynamic trees, counted consumption,
caller output caps, stable payload-blind errors, all 34 neutral fixtures, ZIP
suffix-cavity rejection, and explicit empty capability metadata.

Ready-for-review PR #11386 publishes the validated Ruby tranche from
`89964cc523610b44367ae5db0ff79b36fa6f39e1`. GitHub reports it open,
non-draft, and mergeable. CI, CodeQL, and auxiliary checks are queued, so the
loop returns to monitor-only behavior until those checks reach a terminal
state. Eight ZIP raw-profile children remain pending under the blocked
completion umbrella: .NET, Elixir, Haskell, JVM, Lua, Perl, Python, and Swift.

## Post-#11386 Refresh and Elixir ZIP Raw-Conformance Selection

External review merged ready-for-review PR #11386 as
`2df1499600b500cc4e34b3631fc840bd9abaf261` at
2026-08-13T21:55:35Z from final reviewed head
`42004110d93a2dc61738853f5102598c465457fb`. All 20 reported checks reached a
terminal success, expected skip, or neutral conclusion; the remote branch was
deleted, and this loop did not exercise merge authority.

A fresh collision-checked report, refreshed after selection at live main
`d73dadad9531aa88bdc830a53912a208fe4ce76a`, still covers 15 established lanes,
1,310 normalized identities, and 4,466 slots. It reports 173 high-consensus
identities with 269 gaps, 860 singletons with 12,040 singleton gaps, 661 Rust
singletons, zero collisions, and zero unknown buckets. The Ruby merge and the
intervening ALGOL, wasm-wast-parser, Spanish-curriculum, Mermaid, RISC-V,
ADJ-facts, and HTML-parser changes modify only existing package roots, so the
normalized root set and every inventory metric are unchanged. No newly unowned
identity or authority boundary was discovered, and no additional state owner
was required before reprioritization.

The Ruby ZIP child is now merged, leaving eight dependency-ready children:
.NET, Elixir, Haskell, JVM, Lua, Perl, Python, and Swift. No live PR or remote
parity branch owns ZIP, the neutral fixture, state, or this roadmap. The stale
historical catch-up branch still touches only .NET, Haskell, and JVM ZIP paths
and remains unowned residue that must not be reused or cherry-picked.

The dependency/leverage pass selected
`zip-raw-rfc1951-elixir-lane-parity`. Elixir is the smallest clean standalone
remaining package/toolchain without live or historical overlap. Its ZIP-owned
decoder explicitly rejects dynamic Huffman streams, making this a genuine
codec-contract port: strict dynamic trees, counted consumption, caller output
caps, stable payload-blind errors, all 34 neutral fixtures, ZIP suffix-cavity
and declared-size rejection, and explicit empty capability metadata.

## Post-#11405 Refresh and Perl ZIP Raw-Conformance Selection

External review merged ready-for-review PR #11405 as
`2fb32016e437d5edb847ba70b023381dd04c8c0b` at
2026-08-13T23:28:13Z from final reviewed head
`8394e9eb31649b6998493cd213685dfb44281f23`. All 19 reported checks reached
terminal success or an expected skip, the remote branch was deleted, and this
loop did not exercise merge authority.

A fresh collision-checked report, refreshed through identity-neutral live main
`60fb1384864579944a70229e7ac71fd917e35cee`, covers 15 established
lanes, 1,312 normalized identities, and 4,468 slots. It reports 173
high-consensus identities with 269 gaps, 862 singletons with 12,068 singleton
gaps, 663 Rust singletons, zero canonical collisions, and zero unknown buckets.
The Elixir merge and the thirteen later commits through that live revision are
topology-neutral. External PR #11394 added exactly two
Rust singleton roots, `bacnet-protocol` and
`smart-home-bacnet-ip-integration`, accounting for the two new identities,
two new slots, two new singletons, and 28 new singleton gaps.

The backlog now owns the pure bounded Who-Is/I-Am BVLC/NPDU/APDU codec through
`bacnet-protocol-portable-conformance`. The mixed integration is split between
an injected-transport portable core for deterministic planning, reply
isolation, deduplication, discovery projection, authorization ordering, and
bounded failure behavior, and a blocked native-authority review for UDP bind,
broadcast, peer and forwarded-origin policy, timeouts, CLI I/O, capability
truthfulness, and partial-mutation risk. No BACnet property access, control,
write, BBMD, foreign-device registration, BACnet/SC, or all-language socket
authority will be manufactured.

The Elixir ZIP child is now merged, leaving seven pending children: .NET,
Haskell, JVM, Lua, Perl, Python, and Swift. No live PR or remote parity branch
owns ZIP, the neutral fixture, state, or this roadmap. The stale historical
catch-up branch still touches only .NET, Haskell, and JVM ZIP paths and remains
unowned residue that must not be reused or cherry-picked.

The dependency/leverage pass selected `zip-raw-rfc1951-perl-lane-parity`.
Perl is the smallest clean standalone remaining package/toolchain without live
or historical overlap. Its ZIP-owned codec already has exact bit positions,
the complete length/distance tables, and overlapping back-references, but it
still rejects dynamic Huffman streams and trims declared-size mismatches. This
coherent tranche adds strict dynamic trees, exact counted consumption, caller
output caps, stable payload-blind errors, all 34 neutral fixtures, compressed
suffix-cavity and declared-size rejection, and explicit empty capability
metadata without duplicating DEFLATE.

## Post-#11435 Refresh and Python ZIP Raw-Conformance Selection

External review merged ready-for-review PR #11435 as
`1cf98c1ad74502336c30f21f096967aec2a42a87` at
2026-08-14T02:57:08Z from final reviewed head
`e604023a121b35b558a01e5041df7a1ca78579f4`. All 19 reported checks reached
terminal success or an expected skip, the remote branch was deleted, and this
loop did not exercise merge authority. The Perl ZIP merge modifies only the
existing Perl `zip` identity and is topology-neutral.

A fresh collision-checked report at exact live main
`1cf98c1ad74502336c30f21f096967aec2a42a87` covers 15 established lanes,
1,315 normalized identities, and 4,471 slots. It reports 173 high-consensus
identities with 269 gaps, 865 singletons with 12,110 singleton gaps, 666 Rust
singletons, zero canonical collisions, and zero unknown buckets. Compared with
the stored `60fb1384864579944a70229e7ac71fd917e35cee` snapshot, the exact delta
is three identities, three slots, three singletons, and 42 singleton gaps;
every other stored metric is unchanged.

External PR #11429 added the pure Rust `knxnet-ip-protocol` singleton and the
mixed `smart-home-knxnet-ip-integration` singleton. External PR #11460 added
the mixed `smart-home-esphome-discovery-integration` singleton. The backlog now
owns the bounded KNXnet/IP Search Request/Response codec through a portable
fixture-and-port owner. KNXnet/IP discovery is split between an injected
portable core for configuration, deterministic request planning, isolated
response parsing, deduplication, projection, and authorization ordering and a
blocked native-authority review for UDP bind, multicast, origin policy,
timeouts, CLI I/O, capability truthfulness, and partial-mutation risk. ESPHome
discovery has the same portable/native split around injected mDNS results,
bounded TXT and Noise metadata validation, deterministic projection, the
shared multicast transport, CLI I/O, and runtime authority. All three new roots
lack capability manifests; the two native reviews depend on
`rust-network-substrate-capability-truthfulness`. No tunneling, routing,
configuration, KNX Secure, ESPHome protobuf session, key provisioning, entity
read, subscription, or action authority will be manufactured by parity work.

With Perl merged, six ZIP raw-profile children remained: .NET, Haskell, JVM,
Lua, Python, and Swift. No live PR or remote parity branch owns the neutral
fixture or any of those paths. The stale historical catch-up branch still
touches only .NET, Haskell, and JVM ZIP paths and remains unowned residue that
must not be reused or cherry-picked.

The dependency/leverage pass selected
`zip-raw-rfc1951-python-lane-parity`. Python is the smallest effective clean
standalone tranche: its Python 3.13 test, coverage, Ruff, MyPy, JSON, and raw
zlib oracle tooling is already available, while Lua lacks equivalent installed
fixture/oracle tooling and Swift has a costlier foreign-stream boundary. The
Python ZIP-owned codec still rejects dynamic Huffman streams and trims declared
size mismatches, making this a genuine contract port: strict dynamic trees,
exact counted consumption, caller output caps, stable payload-blind errors, all
34 neutral fixtures, compressed suffix-cavity and declared-size rejection, and
explicit empty capability metadata without duplicating DEFLATE.

Before implementation publication, the branch was refreshed onto live main
`7b82442d0e37ec87477846e1dae89c8f72c362d6`. The five intervening Mermaid,
ADJ, WebAssembly, and human-language commits modify only existing roots and do
not intersect the parity state, roadmap, neutral fixture, or Python ZIP path.
The collision-checked report remains exactly 1,315 identities, 4,471 slots,
865 singletons, 12,110 singleton gaps, 666 Rust singletons, zero collisions,
and zero unknown buckets, so no new owner preceded continued Python work.

Final publication refreshes advanced through
`3f3e77ebebeb77cc8458a86a74ab8be11ecf5be8` after later RISC-V, HTML-parser,
ALGOL, ADJ, WebAssembly-spec, and Mermaid merges. Those commits also modify
only existing, disjoint roots; the same collision-free inventory counts remain
current and no new owner was required.

## Post-#11494 Refresh and Lua ZIP Raw-Conformance Selection

External review merged ready-for-review PR #11494 as
`75b8490ed7ec2bb2a59a5b5519feec033641097a` at
2026-08-14T04:28:18Z from final reviewed head
`62f0041c99d57b00ad593423ea9603eeafbaa2d6`. Its 20 reported checks ended
with 14 successes, five expected skips, and one neutral umbrella conclusion;
the remote branch was deleted and this loop did not exercise merge authority.
The Python ZIP merge modifies only the existing Python `zip` identity and is
topology-neutral.

A fresh collision-checked report at exact live main
`a74ce9183f5986cbc80ae0c94d72da4042416b7a` covers 15 established lanes,
1,317 normalized identities, and 4,473 slots. It reports 173 high-consensus
identities with 269 gaps, 867 singletons with 12,138 singleton gaps, 668 Rust
singletons, zero canonical collisions, and zero unknown buckets. Compared with
the stored `3f3e77ebebeb77cc8458a86a74ab8be11ecf5be8` snapshot, the exact delta
is two identities, two slots, two singletons, and 28 singleton gaps; every
other stored metric is unchanged. Later RISC-V PR #11501 and HTML-parser PR
#11503 modify only existing roots and are topology-neutral.

External PR #11481 added the mixed Rust
`chief-of-staff-notification-approval` singleton. Its bounded versioned Tier 1
stdin/stdout transcript is now owned by
`chief-notification-approval-protocol-portable-conformance`; absolute helper
selection, shell-free process creation, cleared environment, pipes, worker
threads, deadlines, kill/reap behavior, and native notification UI are owned
by the blocked `chief-notification-approval-native-authority-review`. External
PR #11487 added the mixed Rust
`smart-home-google-cast-discovery-integration` singleton. Its injected-result
configuration, Cast TXT validation, stable identity, bounded deduplication,
authorization ordering, and D23 projection are owned by
`smart-home-google-cast-discovery-portable-core-conformance`; the shared mDNS
socket, timeouts, CLI I/O, capability truthfulness, and partial-mutation risk
are owned by the blocked
`smart-home-google-cast-discovery-native-authority-review`. Both roots lack
capability manifests. Parity does not manufacture cross-language process/UI
adapters or extend Cast discovery into TCP sessions, credentials, application
launch, queues, or media control.

With Python merged, five ZIP raw-profile children remain: .NET, Haskell, JVM,
Lua, and Swift. No open PR or live remote branch owns the shared fixture,
state, roadmap, or any remaining ZIP path. The ancient no-PR catch-up branch
still overlaps only .NET, Haskell, and JVM and must not be reused or
cherry-picked.

The dependency/leverage pass selected `zip-raw-rfc1951-lua-lane-parity`.
Lua is the smallest clean standalone remaining child and has a complete local
Lua 5.4/LuaRocks test toolchain plus a pure-Lua independent raw-DEFLATE oracle.
Its ZIP-owned codec already implements exact bit positions, stored/fixed and
multi-block framing, symbol 285, and overlapping back-references, but still
rejects dynamic Huffman streams and trims declared-size mismatches. This
coherent tranche adds strict dynamic trees, exact counted consumption, caller
output caps, stable payload-blind errors, all 34 neutral fixtures, compressed
suffix-cavity and declared-size rejection, and explicit empty capability
metadata without duplicating DEFLATE. Swift remains the clean fallback; .NET,
Haskell, and JVM stay deferred while the historical overlap is independently
re-derived rather than reused.

### Post-#11506 Late-Main Refresh

Before publication, the Lua branch was rebased cleanly onto exact live main
`d11ab15e7455487161485ec71c330d10d63cfaa8`. External PR #11506 added the
mixed Rust `chief-of-staff-biometric-approval` singleton from final reviewed
head `fe646ae53304b9cf50a777a7a0b9aa7317f4f910`, merged as
`88ae29a4fb9c88c10c74466a79bba1a3a356d959` at
2026-08-14T05:08:13Z. Every other commit since the preceding inventory
modifies existing, disjoint roots and is topology-neutral.

A final pre-publication refresh advanced the inventory pin to
`c6bdc3694a85711a213340ac5f12370703051ff6` after PR #11515, PR #11358, and
PR #11516 merged. Their ALGOL, human-language curriculum, and Mermaid changes
add no package root and touch no parity or Lua ZIP surface, so the exact report
metrics and ownership classification below remain unchanged.

The refreshed collision-checked report covers 15 established lanes, 1,318
normalized identities, and 4,474 slots. It reports 173 high-consensus
identities with 269 gaps, 868 singletons with 12,152 singleton gaps, 669 Rust
singletons, zero canonical collisions, and zero unknown buckets. The exact
delta from `a74ce9183f5986cbc80ae0c94d72da4042416b7a` is one identity, one slot,
one singleton, 14 singleton gaps, and one Rust singleton, entirely explained
by the new biometric root.

The bounded `CHIEF-TIER2-BIOMETRIC/1` request and response transcript is now
owned by `chief-biometric-approval-protocol-portable-conformance`, downstream
of the shared notification and trust-checker protocol work. Absolute helper
provenance, shell-free process creation, environment isolation, pipes,
threads, monotonic deadlines, kill/reap behavior, native UI and authenticator
semantics, and external credential custody are owned by the blocked
`chief-biometric-approval-native-authority-review`. The Rust package has no
capability manifest, so its current default falsely presents concrete process,
environment, and timing authority as zero; the native review must add a
nonempty hardware-key-reviewed profile. The source-backed minimum is
`proc:exec`, `proc:signal`, `env:write`, `time:read`, and `time:sleep`; native
biometric sensor, UI, and credential authority remains with the separately
reviewed helper rather than being fabricated in every language.

These two owners were recorded before publication. They do not alter the
dependency-shaped Lua ZIP selection: the rebased branch remains path-disjoint,
pure in memory, and the only active parity tranche.

## Post-#11525 Refresh

External review merged Lua ZIP raw-RFC-1951 parity PR #11525 from final
reviewed head `f1b5f88692456abc7e8b52f8016c43e18bb30eb8` as
`14f15adda03c69074b1178fe124efc0907630821` at
2026-08-14T06:27:41Z. Thirteen checks succeeded and six non-applicable checks
skipped; no check failed or remained pending. GitHub deleted the source branch,
and the parity loop did not exercise merge authority.

The late collision-checked refresh is pinned to exact live main
`4425641359dfa2d23b17bbee7e866c10817b75af`. It covers 15 established lanes,
1,319 normalized identities, and 4,475 slots. It reports 173 high-consensus
identities with 269 gaps, 869 singletons with 12,166 singleton gaps, 670 Rust
singletons, zero canonical collisions, and zero unknown buckets. The Lua merge
changes only the existing `lua/zip` identity and is topology-neutral.

The exact root delta from the prior `c6bdc3694a85711a213340ac5f12370703051ff6`
inventory is one identity, one slot, one singleton, 14 singleton gaps, and one
Rust singleton. External PR #11528 introduced the sole new root,
`rust/chief-of-staff-hardware-key-approval`, from final reviewed head
`149b13105ccff2c7ee0d604195253afef7a58abe`, merged as
`223b14a14e87807ee59b534192540244347be1bd` at
2026-08-14T06:29:40Z. Later human-language and WASM documentation changes are
topology-neutral.

The bounded `CHIEF-TIER3-HARDWARE-KEY/1` transcript is now owned by
`chief-hardware-key-approval-protocol-portable-conformance`, downstream of the
Tier 2 protocol owner. Absolute helper provenance, process and pipe isolation,
deadlines, termination, native UI and FIDO2/WebAuthn/YubiKey semantics, and
credential and assertion custody are owned by the blocked
`chief-hardware-key-approval-native-authority-review`. The daemon receives only
a canonical strength label over a private pipe, not a cryptographically bound
hardware assertion, so the reviewed helper and OS process identity remain the
attestation boundary. The Rust package has no capability manifest; the native
review must replace that false zero-authority default with independently
hardware-key-signed evidence for its concrete process, environment, and timing
effects without attributing the external helper's credential authority to the
Rust crate.

These two ownership records were added before reprioritization. Lua is merged,
leaving exactly four pending ZIP raw-profile children: .NET, Haskell, JVM, and
Swift. No other new owner was discovered in this refresh.

### Post-#11517/#11539 Late-Main Refresh

Before reprioritization, live main advanced to
`97094db1b26beb22b506bdf8432919bea1dca873`. External PR #11517 added the
Rust `coap-protocol` and `smart-home-coap-integration` roots from final
reviewed head `061dc1b5f01c1ce75f0f8854d1eaa8d7bb1987bb`, merged as
`ea036d82100a7530792f7944638be464c4288a05` at
2026-08-14T06:33:47Z after 13 checks succeeded and six non-applicable checks
skipped. PR #11539 adds only ADJ facts and documentation and is
topology-neutral.

The regenerated collision-clean report now covers 1,321 normalized identities
and 4,477 slots across the same 15 established lanes. It reports 173
high-consensus identities with 269 gaps, 871 singletons with 12,194 singleton
gaps, 672 Rust singletons, zero canonical collisions, and zero unknown
buckets. The exact delta from the preceding `4425641359dfa2d23b17bbee7e866c10817b75af`
snapshot is two identities, two slots, two singletons, 28 singleton gaps, and
two Rust singletons, entirely explained by the two CoAP roots.

The socket-free bounded CoAP v1 Confirmable-GET framing contract is owned by
`coap-protocol-portable-conformance`, including strict URI-Path and option
encoding, token and message-ID correlation, piggybacked and separate response
framing, bounded datagrams, and stable payload-blind failures. Its absent
capability manifest defaults to a truthful zero-authority runtime but must be
made explicit when the portable fixture and ports land.

Injected endpoint and profile validation, deterministic exchange planning,
text and JSON scalar decoding, complete-snapshot validation, stable D23
projection, and authorization-before-transport ordering are owned by
`smart-home-coap-portable-core-conformance`. Concrete UDP bind/connect/send/
receive and timeouts, plaintext peer trust and replay risk, direct CLI I/O,
and sequential runtime-mutation risk are owned by the blocked
`smart-home-coap-native-authority-review`. That integration's absent manifest
falsely presents concrete network and console authority as zero; the native
review must add minimum `net:listen`, `net:connect`, and `stdout:write`
runtime evidence plus a loopback UDP test profile, coordinated with the shared
network-substrate truthfulness owner. Read-only local telemetry remains the
boundary: parity does not manufacture writes, subscriptions, discovery,
multicast, blockwise transfer, DTLS, OSCORE, or all-language socket adapters.

All three CoAP ownership records were added before selection. The four pending
ZIP raw-profile children remain .NET, Haskell, JVM, and Swift.

The dependency/leverage pass selected `zip-raw-rfc1951-swift-lane-parity`.
Swift is the only remaining ZIP child with no live or historical branch
overlap: the ancient unmerged catch-up residue still touches the complete
.NET, Haskell, and JVM ZIP trees and must not be reused or cherry-picked.
Swift 6.3.3 is available, the package is a standalone pure in-memory library
with only the local LZSS dependency, and no downstream Swift package imports
it. Its decoder already owns stored/fixed and multi-block framing plus a 256
MiB guard, but rejects dynamic Huffman blocks, lacks counted caller-capped raw
APIs and stable errors, and trims declared-size mismatches. This coherent
tranche consumes all 34 neutral fixtures, hardens exact compressed and
uncompressed ZIP boundaries, adds an independent test-only raw-DEFLATE oracle,
and records explicit empty production capabilities without adding native
authority. The newly recorded CoAP owners remain high-leverage pending work but
do not shorten the four-child ZIP blocker; Swift is therefore the next serial
delivery.

## Swift ZIP RFC 1951 Implementation

The Swift tranche was rebased cleanly onto live `origin/main`
`09877072d243dcadc76f1ea6b7afeaff7124768e`. The six intervening ALGOL,
Mermaid, HTML, ADJ, WASM, and human-language commits are path-disjoint and add
no package root, so the collision-clean inventory remains 1,321 identities,
4,477 slots, 871 singletons, 672 Rust singletons, zero collisions, and zero
unknown buckets.

Implementation revision `0911c7e129f3c9cf0d60836654f4bf7f7a68dd7d`
adds the strict public raw inflater/deflater surface, canonical dynamic-Huffman
decoding, exact consumed-byte reporting, caller-lowerable output bounds, stable
payload-blind failures, strict method-8 ZIP cavity and declared-size checks,
all 34 neutral fixtures, and explicit empty capability metadata. Both BUILD
front doors pass under Swift 6.3.3 with 36 tests; the release build treats
warnings as errors, and LLVM reports 93.76% production line coverage. The
neutral fixture, capability, and parity suites pass 26 tests plus 678 subtests,
the Swift build-tool dry run validates all 164 discovered Swift roots, and the
state DAG, dependency, credential, authority, package-manifest, and diff checks
are clean. Production remains pure in-memory with only the local LZSS
dependency; Foundation, fixture reads, Process, and zlib remain test-only.

### PR #11553 CI build repair

After the Swift package itself passed on all three runners, repository-wide
BUILD validation exposed legacy Windows metadata drift. The bounded repair
restores the exact Perl, Python, and Swift standalone dependency declarations,
keeps Windows `cmd /C` metadata comments executable-safe, fixes the affected
virtual-environment path and LF-sensitive byte fixtures, and repairs the Perl
Nib type checker's missing `shift_expr` precedence node.

The first Direct2D workaround duplicated Windows-only Rust dependencies into
Unix BUILD files. That made the Linux-generated plan validate on Windows but
incorrectly scheduled the native Direct2D/GDI dependency chain on Unix. Build
plan v1 now carries optional Linux, Darwin, and Windows dependency graphs plus
prerequisite-closed affected sets, unions their toolchain requirements, assigns
shared shards from the platform union, and selects only the current runner's
state for execution. The Windows Rust bridge is therefore never omitted from
the matrix but is built and ordered only where it applies; older v1 plans
continue to use the top-level graph. The branch was rebased cleanly onto
`952c9ce7f162467039818f00347186f04dc7bb9c`; auto-merge remains disabled until
the replacement CI run is entirely acceptable and GitHub reports no conflict.

The following Windows run built Swift ZIP and the native Direct2D bridge, then
exercised six inherited affected-package gaps. The focused follow-up restores
the complete grammar-tools CLI dependency set, removes two `cmd.exe`-literal
editable-extra quotes, and adds the missing symbolic-ir Windows front door.
Windows peer close tests now accept either EOF or `ConnectionReset`; IRC clamps
Windows to its supported single reactor instead of requesting unavailable
`SO_REUSEPORT`; and the exposed GDI/Direct2D/C bridge code is warning-clean under
strict Windows Clippy. The three Python front doors, full embeddable and IRC
tests, paint backend tests, strict Clippy gates, and release C bridge build all
pass locally. Auto-merge remains disabled until the replacement head has no CI
failure or merge conflict.

## Post-#11553 Refresh and Haskell ZIP Raw-Conformance Selection

External auto-merge completed ready-for-review Swift ZIP PR #11553 at final
reviewed head `cd14d81f5618878a4aee18f5b735a6819baf291e`, merged as
`10e9e80610f639b13b5d6123aa70a822f5194e9d` at
2026-08-20T09:36:56Z. Its final 32 reported checks ended with 28 successes,
three expected skips, one neutral conclusion, and no failure or pending job.
The parity loop did not exercise merge authority, and the remote branch was
deleted.

The collision-checked schema-3 inventory at exact live main
`afa7975ba76037907138e9208481a360cfe4f14a` covers the same 15 established
lanes, 1,365 normalized implementation identities, and 4,549 established
slots. It reports 174 high-consensus identities with 271 gaps, 903 singletons
with 12,642 singleton gaps, 713 Rust singletons, zero canonical collisions,
and zero unknown buckets. Compared with the stored `09877072d2` snapshot, the
net delta is 44 identities, 72 slots, one high-consensus identity and two
high-consensus gaps, 32 singletons and 448 singleton gaps, and 41 Rust
singletons. The exact topology movement is 49 added identities and five
retired Rust `iir-to-*` identities (`armv7`, `ge225`, `intel4004`,
`intel8008`, and `riscv`).

Every new root was classified before selection. Two family owners cover 19
historical target backend/encoder/simulator identities plus seven simulator
lane expansions, and 13 symbolic-CAS IIR compiler/VM identities. Separate
portable owners cover Chief channel epoch activation; the sole missing Swift
paint-vm-ascii slot; the five remaining ct-compare slots; Python-expanded SIR
array/symbolic runtimes; NUT, SNMP, and SSDP framing; Chief Vault dispatch;
Bitwarden/CSV Vault imports; the Vault local-agent protocol; password policy;
and removable-storage semantics. HomeKit and IPP discovery plus NUT, SNMP, and
SSDP smart-home integrations each have an injected portable-core owner and a
blocked native network/CLI authority review. The Vault agent host,
crash-injection package, and removable filesystem adapter likewise have
explicit blocked native reviews. These records preserve the loop's
portable-only selection boundary while preventing any new singleton from
remaining unowned.

No open PR touches the parity state, roadmap, CMP09, neutral RFC 1951 fixture,
or any remaining .NET/Haskell/JVM ZIP package. One ancient no-PR branch,
`worktree-feat+zstd-and-catchups`, still overlaps those three complete ZIP
trees but is more than ten thousand commits behind main; it is residue, not
ownership, and must not be reused or cherry-picked.

The dependency/leverage pass selected
`zip-raw-rfc1951-haskell-lane-parity` on a fresh branch from `afa7975`. It is
one isolated implementation lane, directly advances the three-child ZIP
blocker, and ultimately unlocks fourteen PNG slots. GHC 9.4.8 and Cabal
3.16.1 are available; the package depends only on local LZSS; no downstream
Haskell package imports ZIP; and production remains a pure in-memory byte
transform. The existing decoder already handles stored/fixed multi-block
streams and hardened output accumulation, but rejects dynamic Huffman,
lacks counted caller-capped typed raw APIs, trims container size mismatches,
and skips Windows validation. This tranche consumes all 34 neutral cases,
adds exact dynamic-tree and suffix boundaries, explicit empty capabilities,
real cross-platform Cabal front doors, and strict ZIP size/CRC integration
without duplicating DEFLATE or adding native authority.

A late clean rebase onto `9873bf0170673631ea338541d235b76aaba78ff6`
added one Rust-only `smart-home-ipp-scanner-discovery-integration` identity and
did not overlap any ZIP, fixture, state, roadmap, or CMP09 file. The refreshed
collision gate therefore records 1,366 identities, 4,550 slots, 904
singletons with 12,656 missing singleton slots, and 714 Rust singletons; the
15-language denominator, 174 high-consensus identities with 271 gaps, zero
collisions, and zero unknown buckets are unchanged. Before publication the
new scanner root received an injected portable-core owner plus a blocked
native DNS-SD/HTTP/filesystem/CLI authority review. This late discovery does
not displace the already selected dependency-ready Haskell ZIP tranche.
Ready-for-review PR #12234 publishes that tranche from validated implementation
head `b1bbe5ec1a`. Its first Ubuntu runs exposed Cabal 3.10.3.0 applying the
command-line coverage switch to the dependency plan, disabling per-component
builds and then rejecting `vector`'s internal libraries. Coverage now lives in
the `zip` package stanza instead: exact Cabal 3.10.3.0 and 3.16.1 runs both
produce HPC reports while dependency per-component builds remain enabled.

## Post-#12234 Refresh and .NET ZIP Raw-Conformance Selection

External auto-merge completed ready-for-review Haskell ZIP PR #12234 at final
reviewed head `94831785259007b7d1cbeea2f2cffe429e34411d`, merged as
`9aaeaf55b4ebc161508de52380afdded8c134dd2` at
2026-08-20T12:38:31Z. Its final 29 reported checks ended with 23 successes,
six expected skips, and no failure or pending job. The parity loop did not
exercise manual merge authority, and GitHub deleted the remote branch.

The collision-checked schema-3 inventory at exact live main
`9aaeaf55b4ebc161508de52380afdded8c134dd2` remains unchanged from the stored
`9873bf0170673631ea338541d235b76aaba78ff6` topology: 15 established lanes,
1,366 normalized implementation identities, 4,550 established slots, 174
high-consensus identities with 271 gaps, 904 singletons with 12,656 singleton
gaps, 714 Rust singletons, zero canonical collisions, and zero unknown
buckets. The exact root comparison found no added or retired identity, no lane
expansion, and no new work requiring classification before selection.

A fresh ownership audit of every open PR found no direct overlap with the
parity state, roadmap, CMP09, neutral RFC 1951 fixture, or either remaining
.NET/JVM ZIP child. The only overlapping remote is still the ancient no-PR
`worktree-feat+zstd-and-catchups` branch, now more than ten thousand commits
behind main. It is residue rather than active ownership and must not be reused
or cherry-picked. Two live PRs continue to touch global Go build-tool
validation/execution surfaces, so this tranche deliberately avoids those
files and will rebase and recertify if either merges.

The dependency/leverage pass selected
`zip-raw-rfc1951-dotnet-lane-parity` on fresh branch
`codex/dotnet-zip-raw-rfc1951` from `9aaeaf55`. It advances both C# and F# and
leaves only the JVM pair before the ZIP portable-conformance umbrella can
close and release its fourteen dependent PNG slots. Both .NET ZIP packages
already own pure in-memory stored/fixed RFC 1951 codecs, local LZSS project
references, explicit empty capability profiles, and real Unix/Windows
`dotnet test` front doors. The installed .NET 9.0.315 toolchain can validate
both immediately; no downstream .NET package imports either ZIP package.
Compared with JVM, the combined source/test surface is smaller and avoids a
missing standalone Gradle plus a Kotlin coverage-gate follow-up.

This coherent tranche consumes every one of the 34 neutral cases in both
lanes, adds dynamic canonical Huffman decoding, caller output limits, exact
input consumption, the closed payload-blind error taxonomy, strict ZIP
compressed/uncompressed-size boundaries, independent foreign-codec
interoperability, and package documentation/metadata updates. Production
remains a memory-only byte transform with no filesystem, process, network,
environment, clock, entropy, FFI, or credential authority.

A late refresh onto live `origin/main`
`6694384516b8bd2d2b7696daf2604fa5d0118d8f` adds one Rust-only
`smart-home-thread-border-agent-discovery-integration` identity and otherwise
does not overlap this tranche. The schema-3 collision gate now reports 15
established lanes, 1,367 identities, 4,551 slots, 174 high-consensus packages
with 271 gaps, 905 singletons with 12,670 missing singleton slots, 715 Rust
singletons, zero collisions, and zero unknown buckets. The new mixed package
has an injected portable-core owner for bounded MeshCoP DNS-SD record parsing,
deduplication, authorization ordering, and D23 projection, plus a blocked
native-authority review for mDNS sockets, peer/origin policy, runtime mutation,
CLI effects, truthful capabilities, and transactional commits. Commissioning,
credentials, Thread sessions, datasets, and control remain explicitly outside
portable discovery parity. Recording both owners before publication preserves
the selection boundary and leaves the dependency-ready .NET ZIP child as the
highest-leverage active unit.

Implementation revision `fe37c45b438cb82664420f5ed5981e6b61ac343f`
publishes the same strict raw profile in C# and F#. Both lanes consume all 34
neutral cases, decode stored, fixed, dynamic, multi-block, foreign, and full
32 KiB-window streams, report exact compressed-byte consumption, enforce the
14 payload-blind typed errors and caller output limits, and map raw failures
back into the historical `InvalidDataException` ZIP-container contract. The
Windows wrappers are repeatable when setup directories already exist and
safely quote checkout-derived environment paths.

The exact BUILD front doors pass 30 C# tests at 91.84% line and 77.89% branch
coverage and 21 F# tests at 91.64% line and 73.43% branch coverage after the
superseded fixed-only decoder in each lane was removed. Strict
builds, both 0.2.0 package builds, dependency vulnerability audits, fresh
uncached C#/F# build-tool plans, Go build-tool tests/vet/build, the neutral
fixture and capability suites, state-DAG validation, authority and credential
scans, collision reporting, and diff checks all pass. Production remains pure
in-memory; fixture reads and the independent Python zlib oracle are test-only.

## Post-#12271 Refresh and JVM ZIP Raw-Conformance Selection

After all required checks became terminal and acceptable, the loop enabled
squash auto-merge for ready-for-review .NET ZIP PR #12271. GitHub merged final
reviewed head `b3c4eac6b8020717fd8be985b933a5f2815efc27` as
`993c78f1d65304f696cce4aeb765de40ac2f34cf` at
2026-08-20T14:10:41Z. Its final 29 reported checks ended with 23 successes,
six expected skips, and no failure or pending job. The parity loop did not
exercise manual merge authority, and GitHub deleted the source remote branch.

The collision-checked schema-3 inventory at exact merge main remains unchanged
from `6694384516b8bd2d2b7696daf2604fa5d0118d8f`: 15 established lanes,
1,367 normalized implementation identities, 4,551 established slots, 174
high-consensus identities with 271 gaps, 905 singletons with 12,670 singleton
gaps, 715 Rust singletons, zero canonical collisions, and zero unknown buckets.
The exact root comparison found no added or retired identity, no lane expansion,
and no newly discovered work requiring classification before selection.
Before publication, the branch rebased cleanly over unrelated Gujarati,
Telugu, and French human-language merges through
`d6baf53b3ef32353fc8e4fcd9b8087349aad788e`; the exact collision gate at that
current main retained every count above with zero collisions or unknown buckets.

The dependency/leverage pass therefore selected the sole remaining ZIP child,
`zip-raw-rfc1951-jvm-lane-parity`, on fresh branch
`codex/jvm-zip-raw-rfc1951` from `993c78f1d6`. Completing Java and Kotlin closes
the last two established-lane gaps in the strict raw RFC 1951 profile, unlocks
the ZIP portable-conformance umbrella, and in turn releases fourteen dependent
PNG parity slots. A fresh ownership audit found no open ZIP/parity PR. The only
overlapping remote remains the ancient no-PR
`worktree-feat+zstd-and-catchups` residue; it will not be reused or
cherry-picked.

This coherent tranche consumes CMP09 and every neutral fixture case in both JVM
lanes, adds dynamic canonical Huffman decoding, caller output limits, exact
input consumption, the closed payload-blind error taxonomy, strict ZIP size and
CRC boundaries, package documentation and metadata, and real local package,
coverage, BUILD, parity, diff, and security evidence. Production remains a
pure in-memory byte transform with explicitly empty capability profiles.

## Post-#12283 Refresh and ZIP Portable-Closure Selection

Ready-for-review JVM ZIP PR #12283 completed all 29 reported checks with 23
successes, six expected skips, and no failure or pending job. After GitHub
reported the branch clean, the loop enabled squash auto-merge; GitHub merged
final reviewed head `4c5edae41dccabd2ac77828779659e76e6d6d7f2` as
`d3699381f3df7a4b6cd86f6ccc97fe1817f70684` at
2026-08-20T15:41:22Z without manual merge authority and deleted the source
branch. Live main then advanced to `f65567daa53a484eac081a750e554096b4241210`
through an unrelated Gujarati-ductus change.

The canonical schema-3 collision inventory at that exact live main remains 15
established lanes, 1,367 normalized implementation identities, 4,551
established slots, 174 high-consensus identities with 271 gaps, 905 singletons
with 12,670 missing slots, 715 Rust singletons, zero canonical collisions, and
zero unknown buckets. The exact root comparison found no added or retired
identity, no new root, and no lane expansion, so no topology-driven owner was
needed. A paginated audit of every open PR found no direct overlap with the
parity state, roadmap, CMP09, neutral fixture, ZIP packages, or dependent PNG
packages. The sole overlapping no-PR remote is the ancient
`worktree-feat+zstd-and-catchups` residue; it remains quarantined and will not
be reused or cherry-picked.

With all twelve toolchain-shaped children merged, the dependency/leverage pass
selected the newly ready `zip-raw-rfc1951-portable-conformance` umbrella on
fresh branch `codex/zip-raw-rfc1951-portable-closure`. The closure is a bounded
aggregate proof rather than a codec rewrite: a closed registry binds exactly
the parity reporter's 15 established lanes to their production raw/count/cap/
error surfaces, the same 34-case neutral corpus, real BUILD entrypoints, and
truthful empty capability profiles. C, C++, and OCaml remain outside this
denominator until their promotion gates pass. Direct PNG work remains blocked
until this closure follows the full pending-to-merged lifecycle.

The same audit decomposed the broad PNG owner before selection: a neutral
TypeScript-reference foundation, twelve toolchain-shaped lane children, and a
final completion umbrella now represent the fourteen missing slots. The
foundation must first tighten TypeScript's caller pixel ceiling and zlib CINFO
validation. Existing Go paint-codec-png delegates to the standard library and
legacy Rust png duplicates CRC/DEFLATE while mixing filesystem authority with
an empty capability declaration; explicit portable reconciliation owners and
a blocked Rust authority review now prevent either package from becoming
silent parity evidence.

A pre-publication rebase onto live main
`0aa390af2441d94b762f45603c955c7052b32db8` added one Rust-only
`smart-home-matter-operational-discovery-integration` root and otherwise did
not overlap the closure. The refreshed collision gate now reports 1,368
implementation identities, 4,552 established slots, 906 singletons, 12,684
singleton gaps, and 716 Rust singletons; every other metric remains unchanged,
including zero collisions and zero unknown buckets. Before publication the
state classified an injected portable Matter DNS-SD parsing, normalization,
authorization-ordering, and D23 projection owner separately from a blocked
native mDNS/runtime/CLI authority review. Commissioning, fabric credentials,
PASE/CASE, certificate validation, secure sessions, Interaction Model I/O, and
control remain explicitly outside portable discovery parity. The ZIP closure
therefore remains the highest-leverage active item. A final rebase onto
`88b12d1280e6e931ab3b7dffa68c820459bbeeef` added only existing Latin
curriculum, TypeScript human-language-data tests, and Rust HTML-parser
diagnostics; the collision-checked topology and every inventory count remained
unchanged. The final publication rebase onto
`12c26b50ae755ec88704c457d8264558433b96f0` added only Gujarati and
Spanish curriculum evidence and likewise left the parity topology unchanged.

## Post-#12307 Refresh and PNG Neutral-Foundation Selection

Ready-for-review ZIP closure PR #12307 completed all 29 reported checks with
23 successes, six expected skips, and no failure or pending job. Both required
gates succeeded. After GitHub reported the branch clean and mergeable, the loop
enabled squash auto-merge; GitHub merged final reviewed head
`adb93159e901a352b37f534b2c8f4a345ebf1f42` as
`2780825f2a7b5bf93936b6927175e975b611cb5c` at
2026-08-20T18:18:50Z without manual merge authority.

The canonical schema-3 collision inventory at that exact live main remains 15
established lanes, 1,368 normalized implementation identities, 4,552
established slots, 174 high-consensus identities with 271 gaps, 906 singletons
with 12,684 missing slots, 716 Rust singletons, zero canonical collisions, and
zero unknown buckets. The closure added no package root: the exact comparison
found no added or retired identity and no existing-identity lane expansion, so
no topology-driven owner was required before selection.

Before publication, `origin/main` advanced through the unrelated Arabic
curriculum repair in #12316 and Gujarati ductus repair in #12317 to
`929de1d639ea9e76e2f38fcf9fbfaf55d596a5d5`. The branch was rebased onto that
exact revision; neither incoming change adds a package root, and the collision
inventory remains unchanged at the counts above.

Merging the ZIP umbrella made exactly one dependency-shaped item newly ready:
`image-codec-png-language-neutral-conformance`. The loop selected it on fresh
branch `codex/png-language-neutral-conformance` from exact merge main. This
foundation will first correct the TypeScript reference's CINFO and caller pixel
ceiling boundaries, then bind IC18 to a checked language-neutral corpus for
chunk ordering and CRC, zlib framing and Adler-32, split IDAT, supported colour
types, bit depths and filters, independent interoperability, exact consumption,
stable payload-blind errors, and caller-lowerable resource ceilings. It will not
start any non-TypeScript lane port. Once this full lifecycle merges, all twelve
toolchain-shaped PNG children become dependency-ready.

## Post-#12319 Refresh and Go PNG Selection

Ready-for-review PNG neutral-foundation PR #12319 completed all 30 reported
checks with 23 successes, six expected skips, and one neutral CodeQL
conclusion. Both CI gates and every Linux, macOS, and Windows build succeeded.
After GitHub reported final reviewed head
`e9ae38873f3606d764e2786e824d6fd1c297a8dc` clean and mergeable, the loop
enabled squash auto-merge; GitHub merged that exact head as
`a09bb28bd19ab24760322f7fcf940184c0912409` at 2026-08-20T20:03:24Z without
manual merge authority and deleted the source branch.

The canonical schema-3 collision inventory at exact merge main remains 15
established lanes, 1,368 normalized implementation identities, 4,552
established slots, 174 high-consensus identities with 271 gaps, 906 singletons
with 12,684 missing slots, 716 Rust singletons, zero canonical collisions, and
zero unknown buckets. The intervening Tamil, Gujarati, and Punjabi curriculum
commits modify only existing roots, while #12319 modifies the established
TypeScript image-codec-png root. The exact topology comparison therefore found
no added or retired identity, no new package root, and no existing-identity
lane expansion; no additional classification owner was needed.

Merging the neutral foundation made all twelve toolchain-shaped PNG children
dependency-ready. The quick leverage and delivery-risk pass selected
`image-codec-png-go-lane-parity` on fresh branch
`codex/image-codec-png-go-lane-parity` from exact merge main. Go has the strongest safe
dependency leverage: it unlocks both the final PNG umbrella and the separate
unblocked paint-codec-png reconciliation. Go 1.26.4 is installed, and its
pixel-container and ZIP raw-deflate, counted-inflate, and CRC-32 prerequisites
pass native tests, race/coverage, vet, and build front doors. No live PR or
remote branch owns the selected PNG surface.

The tranche will add exactly the Go image-codec-png package, consume every
representable closed `image-codec-png-v1` case through its public API, and
implement exact IC18 chunk, zlib, filter, transparency, resource-limit, and
stable-error behavior without duplicating DEFLATE or CRC. Go's typed
`PixelContainer` cannot represent the neutral corpus's fractional encoder
dimensions, so the fixture consumer will validate that schema boundary before
routing every representable case through the public codec. Production remains
an in-memory byte transform with explicit empty capabilities. The existing Go
paint adapter continues to use the standard library until its separately owned
reconciliation, every other lane remains pending, and the completion umbrella
stays blocked until all twelve children merge.

The Go security pass also found one neutral-contract omission before source
publication: IC18 requires APNG refusal, but the merged TypeScript reference and
82-case corpus currently skip the APNG `acTL`, `fcTL`, and `fdAT` chunks as
ordinary ancillary data. The queue now records
`image-codec-png-apng-neutral-rejection-conformance` as a separate pending
reference-and-fixture correction and makes every not-yet-started PNG child wait
for it. The in-progress Go child remains coherent by refusing all three chunk
types as `unsupported-feature` with a focused valid-CRC regression; it does not
modify the merged neutral corpus or broaden into another lane.

Before the first implementation commit was recertified, `origin/main` advanced
to `7020ebbb1a85976dc5aec170ac61df15d7d46e25` through #12321's human-language
gentle-ramp snapshot sharding. Its 30 changed paths are confined to the
human-language workflow, snapshots, verifier, and existing TypeScript
human-language-data root, with no intersection with parity or PNG surfaces and
no added package identity. The Go branch rebased cleanly onto that exact main;
the stored main inventory is repinned there with all counts unchanged.

The implemented Go package now delegates CRC-32, raw RFC 1951 encoding, and
counted exact-cap decoding to `go/zip`; supports the required 8-bit colour,
filter, suggested-palette, transparency, split-IDAT, zlib, checksum, ordering,
and stable-error behavior; and enforces both encoder and decoder edge/product
ceilings before allocation. Its public fixture consumer passes all 82 neutral
cases and the ordered 29-code taxonomy, while focused valid-CRC APNG controls
cover the newly classified neutral omission. A temporary removal of the APNG
branch made all three regressions fail, proving that evidence load-bearing.

Fresh race tests report 99.0% Go statement coverage; format, vet, trimpath
build, module tidiness, eight neutral-fixture validator tests, seven capability
tests, ten parity-reporter tests, Ruff, strict MyPy, Bandit, state/DAG, diff,
credential, and authority checks pass. ZIP, pixel-container, and LZSS pass race
tests at 90.9%, 92.9%, and 97.7% coverage plus their native quality gates. A
fresh repository build-tool binary discovers 4,973 packages, selects the exact
four-node Go dependency closure on Linux, Darwin, and Windows, and executes the
real front door successfully. The branch inventory remains collision-clean at
1,368 identities and advances only the intended established Go slot to 4,553.

The validated branch was published as ready-for-review PR #12324 from exact
head `1b5e34acf32d31feac5acdabf460b0f870dc1606`. The state now records the Go
child as `pr-open`, with that PR as the loop's sole active parity publication;
all remaining PNG children and the newly classified APNG neutral correction
remain pending while its required checks run.

### Post-#12324 refresh and APNG neutral-correction selection

Go PNG PR #12324 completed 30 terminal acceptable checks (24 successes, five
expected skips, and one neutral CodeQL conclusion), including both CI gates and
Linux, macOS, and Windows builds. After the loop invoked GitHub's squash
auto-merge path, GitHub immediately merged final reviewed head
`4d292e615d31af4221c95632f60706c76cb838df` as
`cbce24c513e02c4a3d41e3b1ca731240bbddd5a4` at
2026-08-20T21:43:31Z and deleted the source branch.

The exact-merge collision inventory remains at 15 established lanes and 1,368
canonical implementation identities. The new Go root advances the established
slot count from 4,552 to 4,553, moves `image-codec-png` from the singleton band
to the 2-4 band, and leaves zero collisions and zero unknown buckets. It is one
owned expansion of an existing identity, so no additional classification owner
is required.

Two items are dependency-ready after the Go merge: the APNG neutral correction
and the Go paint-adapter reconciliation. The neutral correction is selected
because it unlocks all eleven remaining PNG lane children, while the adapter
reconciliation unlocks no lane. The bounded tranche clarifies IC18's named APNG
refusal, adds deterministic valid-CRC `acTL`, `fcTL`, and `fdAT` rejection
vectors without changing schema version or error taxonomy, makes the TypeScript
oracle reject those names only after normal chunk and CRC validation, and
updates both the TypeScript and Go consumers plus pinned corpus summaries from
82 to 85 cases. It adds no animation parser, dependency, capability, or new
implementation lane.

Before final validation, `origin/main` advanced to
`9237b4e1be43946aa38c1109387704e3b62a0ec1` through Gujarati `pa` ductus work.
That commit changes only existing human-language and script-ductus surfaces,
adds no package root, and has no path or semantic overlap with the APNG tranche.
The branch rebased cleanly, and the collision inventory remains unchanged at
1,368 identities, 4,553 slots, zero collisions, and zero unknown buckets.
During the final validation pass, main advanced once more to
`377e5add479229badcab758ee166d43edda782e7` through Italian curriculum recovery.
That second commit is likewise root-neutral and disjoint from every owned APNG,
fixture, package, state, and roadmap implementation surface.

The completed correction grows the deterministic corpus from 82 to 85 cases
without changing its schema or ordered 29-error taxonomy. Independent Python
validation proves the three APNG chunks have valid framing and CRCs; removing
the TypeScript rejection produced exactly three portable-suite failures before
the final source change. The final TypeScript suite passes 145 tests with 100%
statement, branch, function, and line coverage, while the Go consumer passes
the expanded public-surface corpus with 99.0% race coverage. Build, lint, type,
module, audit, capability, parity, state-DAG, diff, and authority scans pass.
A freshly compiled repository build tool discovers 4,973 packages, emits the
same eight affected nodes for Linux, Darwin, and Windows, and builds all eight
PNG/pixel-container/ZIP/LZSS nodes on the real local Windows front door with
4,965 unrelated packages skipped.

Ready-for-review PR #12334 was opened from clean validated head
`e7bcb9026ef107d6409e1bb35502dfd290e0de7e` after a normal first push. Three
independent final audits found no code, security, portability, topology, or
ownership blocker across 15 live open PRs and 22 no-PR remote branches. The
APNG item is now `pr-open` and is the loop's only active parity publication;
its initial seven CI checks are queued, so all other parity work remains pending
and auto-merge remains disabled until CI is terminal acceptable and conflict
status is clear.

### Post-#12334 refresh and .NET PNG lane selection

APNG correction PR #12334 completed 30 terminal acceptable checks: 25
successes, four expected skips, and one neutral CodeQL conclusion. Both CI
gates and the Linux, macOS, and Windows builds succeeded, and GitHub reported
the final reviewed head `e8b61f8a108fcf42c07864aa10a637b401bfb781`
clean and mergeable. After the loop requested squash auto-merge, GitHub merged
that head as `7d640e6f77fa4b8905890dfc202cab485858fefb` at
2026-08-20T23:35:27Z and deleted the source branch. The public timeline records
the merge but does not independently expose an auto-merge-enabled event.

The exact-merge collision inventory remains at 15 established lanes, 1,368
canonical identities, and 4,553 implementation slots, with zero collisions and
zero unknown buckets. The five commits since the previous inventory modified
only existing package roots, so no new or retired identity and no additional
classification owner is required. The APNG merge makes all eleven remaining
PNG lane children dependency-ready; the Go paint reconciliation also remains
ready.

The .NET PNG child is selected because it closes two established slots, C# and
F#, with one installed .NET 9 toolchain. Both lanes already provide pure,
tested PixelContainer and ZIP-owned raw RFC 1951, counted-inflate, and CRC-32
surfaces with BUILD and BUILD_windows front doors. This tranche is bounded to
native IC18 packages in those two lanes, complete 85-case public fixture
consumers, focused allocation and error-precedence tests, project metadata,
documentation, changelogs, and empty capability profiles. It does not change
IC18, the neutral fixture, the TypeScript or Go references, paint adapters, or
any other implementation lane.

### .NET PNG implementation and validation

The selected tranche now provides native, pure in-memory IC18 codecs in both
C# and F#. Each implementation consumes its own lane's PixelContainer and
ZIP-owned raw RFC 1951, counted-inflate, and CRC-32 APIs; neither duplicates
the compression substrate or acquires filesystem, process, network,
environment, FFI, or credential authority. Public fixture consumers exercise
all 85 neutral cases, while focused tests load-bear allocation limits, APNG
CRC precedence, exact DEFLATE consumption, Adler-32 validation, filter choice,
the typed PixelContainer boundary, and the stable 29-error taxonomy.

After a conflict-free rebase, exact main is
`c8d204fc58bffd25a7059cbff157e9dbaaa0f5fa`. Its intervening Arabic curriculum
commit adds no package root, so the stored main inventory remains 15 lanes,
1,368 identities, and 4,553 slots. The prospective tree adds exactly the two
selected implementation slots, reaching 4,555 slots with zero canonical
collisions and zero unknown buckets.

Fresh Release tests pass 13/13 in C# at 98.47% line and 98.94% branch coverage,
and 7/7 in F# at 98.99% line and 91.44% branch coverage. C# format verification,
strict JSON, the nine neutral-fixture tests, seven capability-taxonomy tests,
ten parity-reporter tests, state uniqueness/dependency/cycle validation,
production-authority scans, and diff checks pass. The Go build tool's own tests
and vet pass; repository discovery finds 4,975 packages and selects exactly the
eight C#/F# PNG, PixelContainer, ZIP, and LZSS nodes. Its first full Windows run
found that Coverlet's include property needed the repository-standard quoting;
after that focused BUILD fix, all eight nodes built successfully through their
real Windows front doors.

Ready-for-review PR #12341 was opened from clean validated head
`a69cbe67ce0c967321e772e9e25dbfa615f3c462` after a normal first push from exact
main `c8d204fc58bffd25a7059cbff157e9dbaaa0f5fa`. Independent ownership and
security audits found no scope, topology, authority, or publication blocker.
The .NET item is now `pr-open` and PR #12341 is the loop's sole active parity
publication. GitHub reports the head mergeable while its initial CI and CodeQL
checks are queued, so all other parity work remains pending and auto-merge is
disabled until the required checks are terminal acceptable and conflict status
is clear.

### Post-#12341 refresh and JVM PNG lane selection

.NET PNG PR #12341 completed all 29 required checks with 23 successes and six
expected skips. Both CI gates and the Linux, macOS, and Windows builds
succeeded, and GitHub reported final reviewed head
`c89a92887db8a28d3cbf84db595dab6d72013eed` clean and mergeable. After the loop
requested squash auto-merge, GitHub merged that head as
`d58c4ee7ff81f6edccfcdb30d95d499007f17363` at 2026-08-21T01:37:45Z and
deleted the source branch.

The refreshed collision-checked schema-3 inventory at exact live main
`4f15cd2aee15562bff204076608205b5d32f2c80` contains 15 established lanes,
1,368 canonical identities, and 4,555 implementation slots. High-consensus
coverage remains 174 packages with 271 gaps; the 2-4 band contains 167
packages with 2,098 gaps; 905 singletons have 12,670 gaps, including 716 Rust
singletons. Canonical collisions and unknown language buckets remain zero.
The only new package roots since the prior inventory are the reviewed C# and
F# PNG implementations, while later human-language commits modify existing
roots only, so no new classification owner is required.

The JVM PNG child is selected because it closes the only remaining two-slot
PNG gap, Java and Kotlin, with one installed JDK 21 toolchain. Both lanes
already provide passing PixelContainer and ZIP-owned raw RFC 1951,
counted-inflate, and CRC-32 dependencies, BUILD and BUILD_windows front doors,
and empty ZIP capability profiles; cached Gradle 8.11.1 runs all four
prerequisite suites. The bounded tranche adds independent native IC18 packages
in Java and Kotlin, complete 85-case public fixture consumers, direct resource
and precedence tests, Gradle metadata, documentation, changelogs, cross-host
BUILD front doors, and empty capability manifests. It does not change IC18,
the neutral fixture, adapters, or any unrelated lane. The remaining
single-slot PNG children and the Go paint reconciliation stay pending.

### JVM PNG implementation and validation

Implementation commit `b881d8119c` adds independent native Java and Kotlin
PNG codecs. Both packages expose IC18's stable limits, ordered 29-code error
taxonomy, direct encode/decode helpers, and the lane ImageCodec adapter. Their
production code uses only in-memory PixelContainer and ZIP-owned raw RFC 1951,
counted-inflate, and CRC-32 surfaces; JDK ImageIO, zlib, and fixture filesystem
access remain test-only. Checked long arithmetic, exact inflate consumption,
Adler and CRC checks, chunk and APNG precedence, palette/transparency rules,
and deterministic best-of-five filtering are enforced before untrusted-size
allocations.

Each public fixture adapter passes all 85 language-neutral cases. Focused
tests additionally exercise non-finite, non-positive, fractional, and raised
pixel limits, malformed explicitly supplied PixelContainers, APNG precedence,
foreign ImageIO decoding, and independent JDK inflate inspection of emitted
filter rows. Java reaches 98.53% line and 93.95% branch coverage; Kotlin
reaches 96.28% line and 82.51% branch coverage. Both literal `BUILD` and
`BUILD_windows` entry points pass with JDK 21 and Gradle 8.11.1. The shared
fixture generator and nine validator tests, seven capability-taxonomy tests,
ten parity reporter tests, diff checks, and independent authority/security
review are green.

The prospective collision-checked schema-3 inventory remains 1,368 canonical
identities and advances from 4,555 to 4,557 slots with no collisions or unknown
buckets. `image-codec-png` moves from four to six established lanes, shifting
from the 2-4 to the 5-9 completion band, without creating a new identity or
classification owner. Exact repository build-resolver validation remains the
final local gate before publication. That gate now passes: the Go build tool's
tests and vet are green, its dry run selects exactly PNG, PixelContainer, ZIP,
and LZSS in each JVM lane, and its real Windows executor builds all eight
affected nodes successfully.

Ready-for-review PR #12360 was opened from clean validated head
`4f4b60bd701bcab5933d529b5917891962856503` after a normal first push from exact
main `4f15cd2aee15562bff204076608205b5d32f2c80`. Independent implementation,
security, build, inventory, and live-ownership audits found no blocker or
competing owner. GitHub reports the PR mergeable while its initial CI and
CodeQL checks are queued, so the JVM item is now `pr-open`, every other parity
item remains pending, and auto-merge stays disabled until the exact-head checks
are terminal acceptable and conflict status is clear.

### Post-#12360 refresh and Dart PNG lane selection

JVM PNG PR #12360 completed all 29 checks with 23 successes and six expected
skips. Its first D18F attempt failed before repository tests when Hex setup lost
its connection to builds.hex.pm; the unchanged rerun passed. Both CI gates and
the Linux, macOS, and Windows builds succeeded. After the loop requested squash
auto-merge, GitHub merged final reviewed head
`0288bf4a878f7c45cd47ec3e0d43dd44d3158992` as
`3cb13f2b8fb8a27c7c0bf3d6608bbc64aebc9273` at 2026-08-21T03:48:51Z and
deleted the source branch.

The refreshed collision-checked schema-3 inventory at that exact live main
contains 15 established lanes, 1,368 canonical identities, and 4,557
implementation slots. High-consensus coverage remains 174 packages with 271
gaps; the 5-9 band contains 123 packages with 935 gaps; the 2-4 band contains
166 packages with 2,087 gaps; and 905 singletons have 12,670 gaps, including
716 Rust singletons. Canonical collisions and unknown language buckets remain
zero. The only new roots are the reviewed Java and Kotlin PNG implementations;
intervening human-language commits modify existing roots only, so no new
classification owner is required.

The Dart PNG child is selected next. Every remaining PNG lane child closes one
umbrella slot, and Dart wins the safety and leverage tie-break: SDK 3.12.2 is
installed, the established lane is comparatively thin, its pure PixelContainer
and ZIP-owned raw RFC 1951/counting/CRC dependency chain already exists, and a
fresh live ownership audit finds no competing Dart PNG or prerequisite work.
The bounded tranche adds one native IC18 implementation, all 85 neutral cases
through public APIs, focused allocation and precedence regressions, package and
BUILD metadata, documentation, changelog, and an empty capability manifest.
It does not change IC18, the neutral corpus, adapters, or unrelated lanes.

### Dart PNG implementation and validation

The Dart child now implements the native IC18 encoder, decoder, and
`ImageCodec` adapter over the established PixelContainer and ZIP-owned raw RFC
1951/CRC substrate. Production remains pure and in-memory. The public fixture
consumer executes all 85 schema-1 cases, pins the 29-code taxonomy and limits,
and adds focused numeric-limit, malformed-container, nonzero-buffer-offset,
APNG-precedence, and Adler-boundary regressions. Encoder output is inspected
with the platform zlib decoder and decoded by the independent test-only
`image` package.

Dart 3.12.2 passes 93 tests, fatal-info analysis, the literal BUILD front door,
and a 90% coverage gate at 285/287 production lines (99.30%) and 19/19
functions. The established PixelContainer, LZSS, and ZIP prerequisites pass
18, 54, and 66 tests. The Go build tool passes tests, vet, and trimpath build;
its collision-checked 4,978-package plan selects and then builds exactly Dart
PNG, PixelContainer, ZIP, and LZSS, with 4,974 packages skipped. Neutral
fixture, capability-taxonomy, and parity-reporter suites pass 9, 7, and 10
tests. Production dependencies are current, the empty capability profile is
schema-valid, authority and credential scans are clean, and `git diff --check`
passes.

The prospective schema-3 inventory keeps 1,368 canonical identities and adds
one reviewed slot for 4,558 total. `image-codec-png` now spans C#, Dart, F#,
Go, Java, Kotlin, and TypeScript; the 5-9 band has 123 packages with 934 gaps.
Canonical collisions and unknown language buckets remain zero. The branch
rebased cleanly through `2e248b9070a4e7a89cea4bef83ad5caa5917148c` after two
unrelated human-language-only main advances.

### Dart PNG publication

Ready-for-review PR #12376 was opened from clean validated head
`63e5a7adbe3ea06ba3e732564dfe5136ec29b112` after a normal first push from
exact `origin/main` `2e248b9070a4e7a89cea4bef83ad5caa5917148c`.
Independent implementation, security, build, inventory, and live-ownership
audits found no blocker. The exact 12-path diff was clean, mergeable, and had no
open or stale ownership overlap. Final reviewed head
`283ebd81b8a533b4ceb7a5a0055d09d0e5940477` completed all 29 checks with 23
successes and six expected skips; both CI gates and the Linux, macOS, and
Windows builds succeeded. After the loop requested squash auto-merge, GitHub
merged it as `49cfa0700c29fa434fdce3e26cc237ccfcf5a3ce` at
2026-08-21T05:04:51Z and deleted the source branch.

### Post-#12376 refresh and Swift PNG lane selection

The refreshed collision-checked schema-3 inventory at exact live main
`12dbe729e721280d38b4522aa6ea66890d10541b` contains 15 established lanes,
1,368 canonical identities, and 4,558 implementation slots. High-consensus
coverage remains 174 packages with 271 gaps; the 5-9 band contains 123
packages with 934 gaps; the 2-4 band contains 166 packages with 2,087 gaps;
and 905 singletons have 12,670 gaps, including 716 Rust singletons. Canonical
collisions and unknown language buckets remain zero. The only new root since
the prior inventory is the reviewed Dart image-codec-png package. Two later
Gujarati human-language commits modify existing TypeScript roots only, so no
new classification owner is required.

The Swift PNG child is selected next. Each remaining clean PNG child closes
one umbrella slot; Swift wins the safety and dependency-critical tie-break
because Swift 6.3.3 is installed, the lane already has PixelContainer,
ZIP-owned raw RFC 1951/counting/CRC, and LZSS prerequisites with cross-platform
BUILD front doors, and complete live open-PR and no-PR branch audits find no
Swift PNG or prerequisite overlap. Rust is deferred while three live PRs touch
its PixelContainer or ZIP conformance surfaces, Haskell retains a quarantined
stale ZIP/LZSS branch, and the Go paint adapter unlocks no parity child. The
bounded tranche adds one native Swift IC18 package, all 85 neutral cases
through public APIs, focused resource and precedence regressions, SwiftPM and
BUILD metadata, documentation, changelog, and an empty capability manifest.
It does not change IC18, the neutral corpus, adapters, or unrelated lanes.

### Swift PNG implementation and validation

The Swift package implements the native IC18 encoder and bounded decoder over
the established PixelContainer and ZIP-owned raw RFC 1951/counting/CRC APIs.
Swift 6.3.3 passes all eight XCTest methods, including all 85 neutral cases
through public APIs and focused maxPixels, malformed mutable PixelContainer,
product-before-allocation, APNG-precedence, Adler-boundary, taxonomy, and
ImageCodec regressions. Encoder output is independently inflated and parsed by
Python and accepted by the real Windows WIC PNG decoder; the same test target
uses ImageIO on macOS. Production coverage is 434/440 lines (98.64%) and 27/28
functions (96.43%). Format lint, SwiftPM manifest resolution, the warnings-as-
errors release build, and the PixelContainer, LZSS, and ZIP prerequisite BUILD
front doors pass.

A freshly compiled Go build tool passes tests, vet, and trimpath build. Its
full collision-checked plan evaluates 45 Starlark files, discovers 4,979
packages, and selects exactly Swift ImageCodecPNG, PixelContainer, ZIP, and
LZSS on Linux, Darwin, and Windows; real Windows execution builds those four
with 161 unrelated Swift packages skipped. Neutral fixture, capability-
taxonomy, and parity-reporter suites pass 9, 7, and 10 tests. The prospective
schema-3 inventory retains 1,368 identities and adds one reviewed slot for
4,559 total. `image-codec-png` spans eight lanes, the 5-9 band has 123 packages
and 933 gaps, and canonical collisions and unknown buckets remain zero.
Production imports only repository PixelContainer and ZIP; its empty capability
profile is valid, test-only Foundation/Python/WIC/ImageIO authority is
documented, and production authority, credential, dependency, diff, security,
build, and ownership audits are clean.

### Swift PNG publication

Ready-for-review PR #12383 was opened from clean validated head
`9d6cdda29a481e89125c5653747462613cde249b` after a normal first push from exact
`origin/main` `12dbe729e721280d38b4522aa6ea66890d10541b`. Independent
implementation, security, build, inventory, and live-ownership audits found no
blocker. The exact 11-path diff is clean, GitHub reports the branch mergeable,
and required checks queued immediately after publication. The loop remains in
monitor-only mode until the exact lifecycle-update head is terminal.

### Post-#12383 refresh and Lua PNG lane selection

Final reviewed Swift head `5e4de3817ca0089978d4b23ac83bb481934e5e2b`
completed all 30 checks with 24 successes, five expected skips, and one neutral
CodeQL result. Both CI gates and the Linux, macOS, and Windows builds succeeded.
After the loop requested squash auto-merge, GitHub merged PR #12383 as
`a81801eb17fd3f980f23211522011cb69de2d005` at 2026-08-21T06:25:58Z and
deleted the source branch.

The refreshed collision-checked schema-3 inventory at that exact merge main
contains 15 established lanes, 1,368 canonical identities, and 4,559
implementation slots. High-consensus coverage remains 174 packages with 271
gaps; the 5-9 band contains 123 packages with 933 gaps; the 2-4 band contains
166 packages with 2,087 gaps; and 905 singletons have 12,670 gaps, including
716 Rust singletons. `image-codec-png` now spans eight lanes. Canonical
collisions and unknown language buckets remain zero, and the reviewed Swift
package is the only new root.

The Lua PNG child is selected next. It closes another umbrella slot, is tied
for the thinnest collision-free established lane, has installed Lua 5.4.5 and
LuaRocks 3.9.2 toolchains, and can reuse established PixelContainer, ZIP-owned
raw RFC 1951/counting/CRC, and LZSS packages with BUILD front doors. Complete
open-PR and no-PR ownership audits find no Lua PNG or prerequisite overlap.
Rust remains deferred behind three live prerequisite overlaps, Haskell retains
a quarantined stale ZIP/LZSS branch, and the clean Go paint adapter adds no
parity slot or dependency unlock. This bounded tranche adds one native Lua IC18
package, all 85 neutral cases through public APIs, focused binary-string,
allocation, and precedence regressions, LuaRocks and BUILD metadata,
documentation, changelog, and an empty capability manifest. It does not change
IC18, the neutral corpus, adapters, or unrelated lanes.

### Lua PNG implementation and allocation hardening

Tests-first implementation exposed a Lua-specific allocation hazard that the
small neutral vectors alone could not reveal. Both PixelContainer RGBA storage
and ZIP counted inflate previously retained one boxed Lua number per byte. At
the normative 33,554,432-pixel ceiling those two buffers could exceed 4 GiB
before PNG decoder overhead, contradicting IC18's bounded-resource contract.
The selected dependency-shaped tranche therefore hardens both prerequisites:
PixelContainer preserves its mutable 1-indexed `data` table interface over
compact 4 KiB byte strings, while ZIP retains completed inflate output as byte
strings plus only one bounded numeric tail. Focused compact-storage and
multi-megabyte overlapping-back-reference regressions keep that boundary
load-bearing. The Windows PixelContainer and LZSS BUILD fronts are also made
directly executable through LuaRocks' extensionless Busted wrapper.

The native Lua PNG package now passes 94 Busted tests: all 85 neutral cases
through public APIs plus focused max-pixel, malformed/sparse buffer, unsigned
chunk-length, APNG precedence, compact output, taxonomy, and Adler-boundary
regressions. Test-only LibDeflate independently inflates encoder output and
Windows System.Drawing accepts each encoded PNG and recovers exact RGBA bytes.
Production line coverage is 98.58%. PixelContainer passes 36 tests; ZIP passes
69 full tests and 40 focused portable coverage cases at 90.44%; BMP, PPM, QOI,
point-operation, and geometric-transform downstream suites pass 31, 28, 33,
28, and 41 tests respectively. Syntax and LuaCheck gates report zero warnings.

The prospective collision-checked schema-3 inventory contains 15 established
lanes, 1,368 identities, and 4,560 implementation slots. `image-codec-png`
spans nine lanes, the 5-9 band remains 123 packages with 932 gaps, and
collisions and unknown buckets remain zero. The neutral PNG and capability
taxonomy suites pass 9 and 7 tests. Production imports only repository
PixelContainer and ZIP; all filesystem, LibDeflate, process, and real-image
tool authority is confined to tests and the empty production capability
manifest remains valid.

The repository Go build tool passes tests, vet, and trimpath compilation. Its
real parallel Windows plan evaluates 45 Starlark files, discovers 259 Lua
packages, and builds all 15 affected prerequisite, codec, compression, and
image-downstream nodes with 244 unrelated packages skipped. That first full
execution exposed extensionless LuaRocks launchers and repeated test-tool
installation as parallel Windows hazards. BUILD front doors now invoke the
Lua modules directly without quoted wrapper paths, CI installs shared Lua test
tools once during toolchain setup, and the complete rerun passes.

Ready-for-review PR #12396 was opened from validated implementation head
`f1a2c61e7aec307b76cd292d9e8f4f0312ced003` after a clean rebase onto exact
`origin/main` `5eb6fc0589a08d53c4db3fe7f6c9e2e23d0d897c`. The intervening
human-language-only commits were disjoint from this tranche, the source branch
had no prior remote or PR owner, and the complete post-rebase validation above
was repeated before the normal first push. Final reviewed head
`e02e491307a510b4783e8a6e4dd7e3106a5187f8` completed all 29 checks with 23
successes and six expected skips; both CI gates and every operating-system
build succeeded. After the loop requested squash auto-merge, GitHub merged the
PR as `20c704d960e3d62e81571f8834ea1bdeabb33621` at
2026-08-21T08:15:48Z and deleted the source branch.

### Post-#12396 refresh and Go paint PNG reconciliation

The exact live-main inventory at `f6ef9169c3d5a89da0e1e65b18f0811a8b46f194`
contains 15 established lanes, 1,368 canonical identities, and 4,560
implementation slots. High-consensus coverage remains 174 packages with 271
gaps; the 5-9 band contains 123 packages with 932 gaps; the 2-4 band contains
166 packages with 2,087 gaps; and 905 singletons have 12,670 gaps, including
716 Rust singletons. `image-codec-png` now spans nine lanes. Canonical
collisions and unknown language buckets remain zero. The reviewed Lua package
is the only new root since the prior inventory; two later Gujarati
human-language commits modify established TypeScript roots only.

The Go paint PNG reconciliation is selected next. The existing adapter still
delegates to the standard-library PNG decoder, bypassing IC18's 16,384-edge,
33,554,432-pixel, bounded-inflate, exact-consumption, and APNG-refusal
contracts even though the repository Go IC18 core is merged. This is the
highest immediate security and downstream leverage among ready unowned work:
Go 1.26.4 and the compact byte-backed PixelContainer, ZIP, and LZSS chain are
installed, open-PR and no-PR audits find no overlap, and `barcode-1d` is a real
downstream. The bounded tranche preserves the adapter's public aliases and
panic/error compatibility while delegating production behavior to
`image-codec-png`, adding typed-error and portable regression evidence,
raising coverage, and validating the downstream. It adds no package slot, but
closes an eligible owned security gap before more resource-sensitive lane
ports. Rust remains deferred behind live prerequisite overlap, Haskell retains
a quarantined stale ZIP/LZSS owner, Elixir and Perl lack complete local
toolchains, and boxed-byte Ruby, Perl, and Haskell paths need allocation-safe
prerequisite decisions before claiming IC18's ceiling.

### Go paint PNG implementation and validation

The paint adapter is now a thin API-compatible delegate to the repository Go
`image-codec-png` package. Production no longer imports `image/png`, `image`,
`image/color`, `bytes`, or standard-library compression. `PngCodec`, `Codec`,
`Encode`, `EncodePNG`, `Decode`, and `DecodePNG` remain intact; deterministic
output matches the canonical encoder byte-for-byte, the test-only standard
library accepts it, typed `*PngError` values propagate through every decode
alias, and the nonthrowing ImageCodec encode path preserves its typed panic.

The adapter test reads the schema-1 `image-codec-png-v1` corpus directly and
makes representative edge-limit, pixel-limit, valid-CRC APNG, IDAT-cavity, and
invalid-filter cases load-bearing through paint's public APIs. Race-enabled
tests pass at 100.0% statement coverage, followed by `go vet`, trimpath build,
module verification, and dependency listing. `barcode-1d` carries the local
image-codec-png, ZIP, and LZSS replacements that Go modules do not propagate
from dependencies; its race-enabled tests, vet, trimpath build, and module
verification pass.

The repository Go build tool passes all tests, vet, and trimpath compilation.
Its collision-checked Windows plan discovers 306 Go packages, selects exactly
18 changed, prerequisite, and downstream nodes, and builds all 18 with 288
unrelated packages skipped. Neutral PNG fixture, capability taxonomy, and
parity reporter suites pass 9, 7, and 10 tests. The exact inventory remains 15
lanes, 1,368 identities, and 4,560 slots with zero collisions and zero unknown
buckets. JSON/state-graph, diff, dependency, production-authority, and
credential scans are clean, and the empty production capability manifest
remains truthful.

Ready-for-review PR #12403 was opened from clean validated head
`20149865d2825d8746c2d1bfdbb5f6c4b2874db1` after a normal first push from
exact `origin/main` `f6ef9169c3d5a89da0e1e65b18f0811a8b46f194`.
The source branch had no prior remote or PR owner, all live ownership surfaces
were disjoint, and GitHub reports the PR open, non-draft, and mergeable while
required CI and CodeQL checks are queued. It is the loop's sole active parity
publication, so the loop remains monitor-only while those checks are pending.

Final reviewed head `73398d3178ae9bdd8ca0f9fc365e37daced0cba5`
completed all 30 checks with 24 successes, five expected skips, and one neutral
CodeQL result. Both CI gates and every operating-system build succeeded. After
the loop requested squash auto-merge, GitHub merged PR #12403 as
`66b4424e8976369519a5a394f6ef579d1844565f` at 2026-08-21T09:51:46Z and
deleted the source branch.

### Post-#12403 refresh and Python PNG selection

The exact live-main collision inventory at
`0055b8b25f384357d7a9bf57b14f097a930c189b` remains 15 established lanes,
1,368 canonical identities, and 4,560 implementation slots. High-consensus
coverage is 174 packages with 271 gaps; the 5-9 band contains 123 packages
with 932 gaps; the 2-4 band contains 166 packages with 2,087 gaps; and 905
singletons have 12,670 gaps, including 716 Rust singletons. Canonical
collisions and unknown language buckets remain zero. The Go reconciliation
changed an existing root, and the later Italian, Marwadi, Gujarati, and
Punjabi human-language merges add no package root.

The Python PNG child is selected next. Python 3.13.14 and uv 0.11.28 are
installed, and its established PixelContainer and counted ZIP inflater both
use compact byte-backed storage, keeping IC18's 33,554,432-pixel ceiling
honest. Exact Windows prerequisite fronts pass 23 PixelContainer tests at
100.0% coverage, 51 LZSS tests at 97.89%, and 76 ZIP tests at 93.40% branch
coverage with Ruff, formatting, and strict MyPy clean. Live open-PR and stale
remote audits find no Python PNG or prerequisite overlap. The bounded child
will consume all 85 neutral cases through public APIs, reuse repository ZIP
raw inflate/deflate and CRC, add focused allocation and precedence evidence,
and carry empty production capabilities. Its four-node build graph is
`image-codec-png -> {pixel-container, zip -> lzss}`. The prospective inventory
adds one slot, moves `image-codec-png` to ten lanes, and therefore yields 175
high-consensus packages with 276 gaps and 122 packages with 926 gaps in the
5-9 band. Boxed-byte Haskell, Ruby, Perl, and Elixir prerequisites need
separate compact-output hardening before their ceilings are credible; Rust
remains deferred behind live prerequisite ownership and legacy filesystem
authority.

The tests-first security review found one Python prerequisite gap before the
PNG ceiling could be treated as load-bearing: ZIP's educational `raw_deflate`
path materialized boxed LZSS tokens and exhaustively searched a 32 KiB window,
so a legal noisy 128 MiB scanline stream could amplify into multiple gigabytes
and impractical match work. This dependency-shaped tranche therefore hardens
large raw-DEFLATE input in ZIP itself. Stored blocks cover incompressible data,
fixed-Huffman blocks use constant-size streaming match state for repetitive
data, and a foreign-zlib 2 MiB noisy-input regression forbids the boxed token
path. Small inputs retain the educational tokenizer.

### Python PNG implementation and validation

The native package exposes the ordered 29-code payload-blind `PngError`
taxonomy, Adler-32, deterministic RGBA8 encoding, bounded decoding, and the
`PngCodec` adapter through repository PixelContainer and ZIP primitives only.
All 85 neutral cases run through the public API with exact structural and
error-precedence assertions. Test-only zlib independently inflates encoder
output, and Pillow accepts complete output while recovering exact RGBA bytes.

The dependency closure is green on Windows: PixelContainer passes 23 tests at
100.0% coverage, LZSS passes 51 at 97.89%, ZIP passes 77 at 93.87% branch
coverage, and PNG passes 106 at 99.13%. Ruff, format, and strict MyPy gates are
clean. ZIP 0.2.1's deterministic 2 MiB noisy-input regression proves the large
raw-DEFLATE path bypasses boxed LZSS tokens and remains foreign-zlib compatible;
the existing 100 KiB repetitive regression keeps compression load-bearing.
LZSS's BUILD fronts now pin Python 3.13 instead of resolving an unsupported
host default.

The Go build tool passes its tests, vet, and trimpath build. The exact Windows
diff has three changed packages and selects eight affected nodes: `deflate`,
`heap`, `huffman-tree`, `image_codec_png`, `lzss`, `pixel_container`, `zip`,
and `zstd`. A clean execution builds all eight and skips 4,973. Neutral PNG
fixture, capability taxonomy, and parity reporter suites pass 9, 7, and 10
tests. The prospective collision-checked inventory is 15 lanes, 1,368
identities, 4,561 slots, and zero collisions or unknown buckets;
`image-codec-png` reaches ten lanes, moving the high-consensus band to 175
packages with 276 gaps and the 5-9 band to 122 packages with 926 gaps.
Production authority, state graph, diff, dependency, and credential audits are
clean, and the empty capability manifest remains truthful.

Final reviewed head `f600037fe3631ddccd8e3c18452f8db96bbf52df`
completed all 30 checks with 24 successes, five expected skips, one neutral
CodeQL result, and no failures or pending work. After the loop requested squash
auto-merge, GitHub merged PR #12420 as
`43b52d3b5ba230dd55a4fae35d0bee746b4a7d94` at 2026-08-21T11:37:49Z and
deleted the source branch.

### Post-#12420 refresh and Python heap BUILD-front selection

The collision inventory selected at merge
`43b52d3b5ba230dd55a4fae35d0bee746b4a7d94` and revalidated after rebasing on
`87b0df7caf6314bd2b9b4e887e35c39a2bad97b2` remains 15 established lanes and
1,368 canonical identities with 4,561 implementation slots. The intervening
Chinese and Marwadi human-language commits modify existing package roots only.
`image-codec-png` now spans ten lanes. The high-consensus band contains 175
packages with 276 gaps; the 5-9 band contains 122 packages with 926 gaps; the
2-4 band remains 166 packages with 2,087 gaps; and 905 singletons have 12,670
gaps, including 716 Rust singletons. Canonical collisions and unknown language
buckets remain zero.

The Python PNG downstream run discovered a concrete build-front contract gap.
`python/heap` creates `.venv` without `--clear` or a Python pin: the first
Windows invocation selected unsupported Python 3.10 on this host, and the
immediate second invocation deterministically failed because the environment
already existed. `python-heap-build-front-idempotence` is selected as the next
bounded item. Both BUILD fronts will recreate the environment, pin Python 3.13,
and invoke that environment explicitly for Ruff, formatting, MyPy, and pytest.
The validation gate runs each recipe twice in one checkout and then exercises
the heap-dependent compression closure. Live audits across open PRs and remote
branches find no heap, state, or roadmap owner.

The same audit found 17 additional Python `BUILD_windows` recipes that create
`.venv` without `--clear`. They are recorded separately as
`python-uv-build-front-idempotence-audit`, dependent on this reference repair,
so the present tranche does not silently expand into unrelated packages.
Direct clean-tree validation also found that `huffman-tree` creates Python
3.10 despite declaring Python >=3.12, then cannot install `heap`; its Brotli,
Deflate, Huffman Compression, and related compression fronts share the
unpinned uv pattern. That dependency-shaped follow-up is recorded separately
as `python-huffman-compression-build-front-python313`. Until it lands, local
downstream validation supplies `UV_PYTHON=3.13` explicitly and does not claim
that the existing dependent fronts are standalone-clean.
Remaining PNG lanes stay deferred: Haskell, Ruby, Perl, and Elixir still need
compact-output prerequisite decisions, Elixir and complete Perl tooling are
absent locally, and Rust has live PixelContainer/ZIP ownership plus a legacy
filesystem-authority reconciliation.

### Python heap BUILD-front implementation and validation

Both heap front doors now recreate `.venv` with `--clear`, pin Python 3.13,
and invoke the environment's interpreter explicitly for Ruff, format, MyPy,
and pytest. A repository regression pins the complete Unix and Windows recipes.
The exact Windows front passes twice consecutively in one checkout: every
quality gate is clean and all 52 tests pass at 99.12% coverage on both runs.
The canonical Unix recipe is structurally pinned for Linux and macOS CI.

Direct Windows downstream validation passes when supplying the separately
recorded pre-existing `UV_PYTHON=3.13` requirement: Huffman Tree passes 36 tests
at 98.62% coverage, Brotli 75 at 97.70%, Deflate 20 at 98.14%, and Huffman
Compression 35 at 97.44%. The Go build tool passes all tests, vet, and trimpath
compilation. Its exact Windows diff plan discovers 494 Python packages, selects
one changed and six affected nodes, and a real run builds heap, LZSS, Huffman
Tree, Brotli, Deflate, and Huffman Compression while skipping 488 packages.

The focused build-front regression, parity reporter, and capability taxonomy
pass 2, 10, and 7 tests. The schema-3 inventory remains 15 lanes, 1,368
identities, and 4,561 slots with zero collisions or unknown buckets. The state
graph is unique, dependency-complete, and acyclic across 415 items. Diff,
credential, dependency, and production-authority scans are clean; no capability
manifest or privileged production boundary changes.

Ready-for-review PR #12425 was opened from clean validated head
`882060f58f2d5013abbcd13a08781fd8b57f11b9` after a normal first push from
exact `origin/main` `87b0df7caf6314bd2b9b4e887e35c39a2bad97b2`.
The target branch and PR were absent before publication, and all live ownership
surfaces were disjoint. Final reviewed head
`86501760d38cb31c8eb2f4ac4df0ecb1a0c18fad` completed all 30 checks with 24
successes, five expected skips, one neutral CodeQL result, and no failures or
pending work. GitHub reported the branch mergeable, and the loop requested
squash auto-merge; GitHub merged PR #12425 as
`51804cdf287b4085875c0113557600b20dd4d733` at 2026-08-21T13:19:22Z and
deleted the source branch.

### Post-#12425 refresh and Python Huffman compression BUILD-front selection

The refreshed collision inventory at live main
`4b36ac52276feb29355065e86ea0121e0b731c52` remains 15 established lanes,
1,368 canonical identities, and 4,561 implementation slots. The Portuguese
human-language commit after #12425 modifies existing roots only. Canonical
collisions and unknown language buckets remain zero.

`python-huffman-compression-build-front-python313` is the direct
dependency-shaped successor to the heap repair. A clean Huffman Tree front
selected Python 3.10 despite declaring Python >=3.12 and then could not install
the repaired heap package. Brotli, Deflate, and Huffman Compression share the
same unpinned uv recipe; their Windows fronts pass only when the caller supplies
`UV_PYTHON=3.13`. This tranche pins Python 3.13 in both canonical and Windows
fronts for those four packages, preserves their existing dependency order and
quality gates, and adds a repository regression for the complete recipes.
Live audits across 23 open PRs and 20 unowned remote branches find no overlap
on the compression packages, parity state, or roadmap. The broader uv
idempotence audit remains pending for classification of the 17 other fronts.

### Python Huffman compression BUILD-front implementation and validation

The canonical and Windows recipes for Huffman Tree, Brotli, Deflate, and
Huffman Compression now pin Python 3.13 while preserving `--clear`, exact
repository-local prerequisite order, editable development installation, and
their existing pytest gate. A focused repository regression pins all eight
complete recipes and passes alongside the heap reference regression.

Exact Windows fronts run on Python 3.13.14 and pass Huffman Tree's 36 tests at
98.62% coverage, Brotli's 75 at 97.70%, Deflate's 20 at 98.14%, and Huffman
Compression's 35 at 97.44%. The Go build tool passes tests, vet, and trimpath
compilation. Its exact Windows diff plan evaluates 45 Starlark files,
discovers 494 Python packages, selects four changed and six affected nodes,
and a real run builds heap, LZSS, Huffman Tree, Brotli, Deflate, and Huffman
Compression with 488 skipped. No `UV_PYTHON` override is required.

Parity reporter and capability taxonomy suites pass 10 and 7 tests. After a
clean rebase on latest human-language main
`4bf6d6253a76ade124348248830ff58406a08be5`, the schema-3 inventory remains
15 lanes, 1,368 identities, and 4,561 slots with zero collisions or unknown
buckets. The 415-item state graph is unique, dependency-complete, and acyclic.
Diff, dependency, credential, and production-authority scans are clean; this
tranche changes no production source, dependency metadata, capability
manifest, or privileged boundary.

Ready-for-review PR #12440 was opened from clean validated head
`aea1d5bf9d8abc7d361fb209bf0af481de9a97e7` after a normal first push from
exact `origin/main` `4bf6d6253a76ade124348248830ff58406a08be5`.
The target branch and PR were absent before publication, and exact live audits
found zero overlap across 22 open PRs and 20 no-PR remotes. Final reviewed head
`47992e6c1a328561681d87e163b129f90d01122a` completed all 30 checks with 24
successes, five expected skips, one neutral CodeQL result, and no failures or
pending work. GitHub reported the branch mergeable, and the loop requested
squash auto-merge; GitHub merged PR #12440 as
`4f840fb693ec3569943ad6d27b46edcc3f69a5ae` at 2026-08-21T14:25:00Z and
deleted the source branch.

### Post-#12440 refresh and Python uv BUILD-front audit selection

The refreshed collision inventory at exact live main
`a2855409cfa8ebd5923f76a8fb1a0dce8e6ac4b6` remains 15 established lanes,
1,368 canonical identities, and 4,561 implementation slots. The sole commit
after #12440 changes tests under an existing human-language package root, so
the parity topology is unchanged: 175 high-consensus packages have 276 gaps,
122 packages in the 5-9 band have 926 gaps, 905 singletons have 12,670 gaps,
and canonical collisions and unknown language buckets remain zero.

`python-uv-build-front-idempotence-audit` is selected as the next bounded
owner. Exact Git-visible scanning finds 17 remaining Python `BUILD_windows`
fronts that create `.venv` without `--clear`; sixteen declare Python >=3.12,
while `ls00` declares >=3.11. A representative clean `caesar-cipher` run
selects unsupported Python 3.10, its immediate repeat fails because uv refuses
to overwrite the existing environment, and package installation then rejects
the selected interpreter. Nine fronts install repository-local siblings, so
the audit will classify Python pinning, canonical/Windows symmetry, exact
dependency order, and downstream reach before recording homogeneous
dependency-shaped backfills. It will not bulk-edit the 17 packages.

Live ownership audits across 16 open PRs and 24 no-PR remotes find no exact
overlap on the 17 package surfaces, parity state, or roadmap. PR #12162 is a
semantic neighbor changing 56 different Python `BUILD_windows` files; none is
in this corpus, so its paths and validation scope remain outside this tranche.
Required uv 0.11.28 and Python 3.13.14 toolchains are installed. Remaining PNG
children retain compact-output, toolchain, or live Rust ownership blockers;
the OCaml process-free build core overlaps current build-tool owners.

### Python uv BUILD-front audit implementation and validation

The version-1 audit specification now defines fail-closed Git-visible corpus
discovery, deterministic JSON and Markdown records, stable issue classes,
ordered local dependencies, weak dependency components, and a payload-bounded
runtime observation protocol. The reporter scans 481 Python package roots and
classifies the exact 17 non-idempotent fronts. Every canonical and Windows venv
command omits both `--clear` and a Python pin; sixteen packages require Python
>=3.12, `ls00` requires >=3.11, nine install local siblings, and all sibling
orders are platform-symmetric. The corpus forms eight weak dependency
components. No package recipe changes in this audit.

Every literal `BUILD_windows` front ran twice in one disposable clean worktree
at selection revision `a2855409cfa8ebd5923f76a8fb1a0dce8e6ac4b6` with uv
0.11.28. Sixteen generated-pattern fronts selected Python 3.10.11 and failed
their first run at command two because the interpreter did not satisfy the
package floor. `ls00`, whose venv command omits `--no-project`, discovered
Python 3.13.14 from project context and completed its first run. Every unchanged
second run failed command one with exit 2 because `.venv` already existed. The
versioned receipt records only command indexes, exit codes, stable diagnostic
classes, and interpreter versions; it contains no host paths or payload logs.

Nine machine-checked pending backfills cover all 17 fronts exactly once. They
separate hash-functions from its dependents, RESP/TCP from the data-store
closure, graph and trie closures, Caesar's unsplit Windows install, three
homogeneous standalone structures, and the legacy `ls00` shape. The data-store
owner also carries its Windows-invalid quoted editable requirement. This
preserves dependency order while avoiding a 17-package bulk patch.

The focused reporter suite passes 13 tests with 97% branch coverage for the
reporter. Ruff format/lint and strict MyPy are clean on Python 3.13. Parity
reporter and capability taxonomy suites pass 10 and 7 tests. The Go build tool
passes all tests, vet, and trimpath compilation; its exact diff plan validates
494 Python packages and skips all 494 because this audit changes no package
recipe, while the forced Windows dry plan validates and selects all 494. The
audit changes no package recipe. After a clean rebase onto human-language-only
main
`54db2ed5ba676e5921010179e4046cb025ee6b0a`, the schema-3 inventory remains 15
lanes, 1,368 identities, and 4,561 slots with zero collisions or unknown
buckets. The expanded 424-item state graph is unique, dependency-complete, and
acyclic. Diff, credential, dependency, and production-authority scans are
clean.

Ready-for-review PR #12447 was opened from clean validated head
`c4ebd2113e42cfd915d30916a034a5bffdb928d6` after a normal first push from
exact `origin/main` `54db2ed5ba676e5921010179e4046cb025ee6b0a`.
The target branch and prior PR were absent before publication, and a late audit
found zero exact overlap across 17 open PRs. Final reviewed head
`b636516654b718a1de3950ba358f1e4432641ecb` completed all 29 checks with 23
successes, five expected skips, one neutral result, and no failures or pending
work. GitHub reported the branch mergeable; the loop requested squash
auto-merge, and GitHub merged PR #12447 as
`fc0ff4496e81e3a79c5011be2ec4b03465211676` at 2026-08-22T07:02:42Z.

### Post-#12447 refresh and Python hash-functions BUILD-front selection

The collision-checked schema-3 inventory selected at main
`56374fe9ef977ecac3180315239f1c54da76eec7` and revalidated after the later
Arabic lesson-only main `352edb288887b47098958106f907f0113cd3b891`
contains 15 established lanes,
1,369 canonical identities, and 4,562 implementation slots. The 175-package
high-consensus band still has 276 missing slots, the 5-9 band has 122 packages
and 926 gaps, the 2-4 band has 166 packages and 2,087 gaps, and 906 singletons
have 12,684 gaps, including 717 Rust singletons. Canonical collisions and
unknown language buckets remain zero. The reporter's ten focused tests pass.

The one new canonical identity since the prior inventory is the Rust-only
`smart-home-kodi-jsonrpc-integration` merged in PR #12308. It owns concrete
private-endpoint TCP/HTTP JSON-RPC transport and D23 runtime authorization, has
no `required_capabilities.json`, and is therefore split into two explicit
owners. The selectable portable-core owner will define language-neutral
fixtures for bounded envelopes and snapshots, stable D23 projection,
allowlisted command plans and postconditions, injected transport, normalized
failures, and redaction. The selection-blocked native authority review owns
TCP/HTTP, endpoint trust, timeouts and response limits, CLI effects, and D23
runtime authorization and mutation. No cross-lane coverage claim is made
before these boundaries are classified.

`python-hash-functions-build-front-idempotence` is the next eligible bounded
owner. The uv audit's machine-checked decomposition places `hash-functions`
before `bloom-filter`, `hash-map`, and `hyperloglog`, which in turn precede the
in-memory data-store closure. Repairing this single leaf front therefore has
the highest immediate dependency leverage while preserving one coherent
package boundary. Both fronts will recreate `.venv`, pin Python 3.13, install
through that named environment, and invoke its interpreter explicitly for
Ruff, formatting, MyPy, and pytest. Validation will execute the exact Windows
front twice, pin the canonical recipe structurally for Linux/macOS CI, and
inspect the exact affected build plan. Live open-PR and remote-branch audits
found no overlapping owner on `python/hash-functions`, parity state, or this
roadmap, and the selected branch was absent before creation.

### Python hash-functions BUILD-front implementation and validation

Both `hash-functions` fronts now recreate `.venv` with `--clear`, pin Python
3.13, install through that environment, and invoke its interpreter explicitly
for Ruff, formatting, MyPy, and pytest. The original repair validated MyPy's
default mode but did not retain the governing profile's `--strict` flag; the
separately owned strict-MyPy correction fixes that historical claim. The
package's previously dormant quality gates exposed only mechanical test hygiene
and formatting debt, which is clean without changing hash behavior. A
repository regression pins both complete recipes. The audit contract and tests
preserve the original 17-owner decomposition while treating the live
non-idempotent corpus as a shrinking
set, so each selected owner can advance through the normal lifecycle without
invalidating the merged audit.

The exact Windows front passes twice consecutively in clean validation
worktrees on Python 3.13.14 with every quality gate clean and all 88 tests at
100% coverage. The canonical recipe is structurally pinned for exact Linux and
macOS CI execution because this Windows host has neither WSL nor a container
runtime. The Go build tool passes all tests, vet, and trimpath compilation. Its
exact Windows dry plan evaluates 45 Starlark files, discovers 494 Python
packages, and selects one changed and eight affected nodes:
`hash-functions`, `bloom-filter`, `hash-map`, `hyperloglog`,
`in-memory-data-store-protocol`, `resp-protocol`,
`in-memory-data-store-engine`, and `in-memory-data-store`, with 486 skipped.

The exact build execution exposed an out-of-scope prerequisite front before it
could complete that plan. `in-memory-data-store-protocol` creates its Windows
venv through ambient `python` and invokes the `pip` console launcher directly;
the launcher recursively re-entered the environment and did not reach tests.
This distinct non-uv recipe is now owned by
`python-in-memory-data-store-protocol-build-front-python313`, and the existing
data-store owner depends on it. The protocol source passes five tests at 100%
coverage when install and pytest use its named Python 3.13 interpreter.

The remaining real downstream Windows fronts pass under the explicit
`UV_PYTHON=3.13` and `UV_VENV_CLEAR=1` compatibility settings already owned by
their pending backfills: RESP Protocol passes 129 tests at 100% coverage,
Bloom Filter 47 at 96.19%, Hash Map 116 at 96.04%, HyperLogLog 61 at 98.89%,
the store engine 57 at 96.67%, and the composed store five at 100%. Focused
build-front, uv-audit, parity reporter, and capability taxonomy suites pass 2,
13, 10, and 7 tests. The schema-3 inventory remains 15 lanes, 1,369 identities,
and 4,562 slots with zero collisions or unknown buckets. State, diff,
credential, dependency, and production-authority checks are clean; the tranche
changes no production behavior, dependency metadata, capability manifest, or
privileged boundary.

### Python hash-functions BUILD-front publication

Ready-for-review PR #12482 opened from independently reviewed head
`6be901dbae2716252eec572208e05052106ecc5b` after a normal first push from exact
`origin/main` `352edb288887b47098958106f907f0113cd3b891`. The target branch and exact
implementation surfaces were absent before publication. Final head
`2dac3e26949c0e7c297d8816691aa4e5ee706f11` completed all 30 checks with 24
successes, five expected skips, one neutral CodeQL result, and no failures or
pending work. GitHub reported the branch clean and mergeable; the loop
requested squash auto-merge, and GitHub merged PR #12482 as
`e9d55a4265f4894228f2b595b6ce4c17daf1eb70` at 2026-08-24T02:03:40Z.

### Post-#12482 refresh and Python hash-collections selection

The collision-checked schema-3 inventory at exact merged main
`e9d55a4265f4894228f2b595b6ce4c17daf1eb70` remains identity-neutral: 15
established lanes, 1,369 canonical identities, 4,562 implementation slots,
175 high-consensus identities with 276 missing slots, 122 identities in five
to nine lanes with 926 missing slots, 166 identities in two to four lanes with
2,087 missing slots, and 906 singletons with 12,684 missing slots. There are
zero canonical collisions and zero unknown language buckets. No identity or
per-lane slot changed since the preceding inventory, so no new owner was
needed.

The dependency/leverage pass selects
`python-hash-collections-build-front-idempotence`. Its two prerequisites are
merged, and its homogeneous generated-standard repair closes the six fronts
for Bloom Filter, Hash Map, and HyperLogLog. All three directly consume the
newly repaired hash-functions package, while HyperLogLog continues into the
in-memory data-store closure. Independent live audits across nine open PRs and
32 remote heads find zero overlap on the three package roots, audit fixtures,
state, or roadmap; the selected remote branch was absent before creation.

### Python hash-collections BUILD-front implementation and validation

All six complete recipes are now pinned by one focused regression. Each front
clears and recreates its package-local environment with Python 3.13, installs
the local hash-functions prerequisite before the package, and invokes Ruff,
formatting, strict MyPy, and pytest through that exact interpreter. The
generated-standard contract explicitly covers local dependencies without a
PEP 561 marker by analyzing them with MyPy's `--follow-untyped-imports`; no
dependency metadata or capability boundary changes.

On Windows with uv 0.12.1 and Python 3.13.14, every literal front passes twice
consecutively: Bloom Filter passes 47 tests at 96.15% coverage, Hash Map passes
116 tests at 96.04%, and HyperLogLog passes 61 tests at 98.88%. Dormant Ruff
and MyPy findings required only formatting, unused-test cleanup, stronger type
annotations, and the behavior-equivalent register maximum expression. The
immediate HyperLogLog downstream store engine passes 57 tests at 96.67%
coverage against editable local dependencies.

Focused recipe and uv-audit suites pass two and 13 tests; parity reporter and
capability taxonomy suites pass ten and seven tests, and the collision gate is
clean. The Go build tool passes all tests, vet, and compilation. Its exact dry
plan evaluates 45 Starlark files, discovers 494 Python packages, and selects
the three changed packages plus five prerequisites and dependents. A full
eight-node local execution reached the unchanged in-memory-data-store-protocol
prerequisite and reproduced its separately owned recursive pip-launcher hang;
the runaway validation process was stopped after five minutes. The selected
fronts and immediate runtime downstream were executed directly, while the
canonical Unix recipes remain structurally pinned for Linux and macOS CI.

### Python hash-collections BUILD-front publication

Ready-for-review PR #12492 opened from independently reviewed head
`6a3b5fb04a5016e66164787db6de8aac1e85b118` after a normal first push from
exact `origin/main` `d491ed654f31c43f01e429a96eb5cbaaae4cc85c`.
Live audits found zero overlap across 11 open PRs and 32 remote source heads;
the target remote branch was absent before publication. GitHub reports the
branch mergeable, while required checks are queued, so the loop returns to
monitor-only behavior.

Final head `972100da6667bfc03dd3e9d2e0faf6f09644ab4c` completed all 30
checks with 24 successes, five expected skips, one neutral CodeQL result, and
no failures or pending work. GitHub reported the branch clean and mergeable;
the loop requested squash auto-merge, and GitHub merged PR #12492 as
`6440352ce3d282209d5c5e6aa08e2ec2ee80463b` at 2026-08-24T03:03:55Z.

### Post-#12492 refresh and Python data-store protocol selection

The collision-checked schema-3 inventory at exact merged main
`6440352ce3d282209d5c5e6aa08e2ec2ee80463b` remains identity-neutral: 15
established lanes, 1,369 canonical identities, 4,562 implementation slots,
175 high-consensus identities with 276 gaps, 122 identities in five to nine
lanes with 926 gaps, 166 identities in two to four lanes with 2,087 gaps, and
906 singletons with 12,684 gaps. Canonical collisions and unknown buckets are
zero, and no new portable or unowned identity appeared.

The dependency/leverage pass selects
`python-in-memory-data-store-protocol-build-front-python313`. It is the exact
one-package prerequisite that blocked the prior eight-node build: the Windows
front resolves ambient Python and invokes a recursive pip launcher, while the
canonical front leaves its interpreter unpinned. The bounded repair will pin
and clear a named Python 3.13 environment, route install and all quality gates
through that interpreter, and validate the protocol before the final
data-store owner. RESP/TCP remains the other independently owned prerequisite.
Live audits find zero overlap across nine open PRs and 30 remote source heads
on the selected package, state, or roadmap; the target branch was absent before
creation.

The same contract pass found one previously unowned retroactive quality-gate
gap: the merged generated-standard hash-functions fronts still invoke plain
MyPy although the governing repair profile now requires strict mode and the
merged state notes claim it. New pending owner
`python-hash-functions-build-front-strict-mypy` records the exact two-front
correction and validation-claim repair. It does not displace the selected
protocol item because the protocol's recursive pip launcher is the reproduced
dependency-closure blocker.

### Python data-store protocol BUILD-front implementation

Both protocol BUILD fronts now recreate a named `.venv`, pin Python 3.13,
install through that environment, and invoke its interpreter explicitly for
Ruff, formatting, strict MyPy, and pytest. The Windows recipe separates the
dependency-free editable package install from the quality-tool install, which
removes the recursive generated-pip path without changing package dependencies.
A repository regression pins both complete recipes. Strict checking also made
the existing `EngineResponse.bulk_string` normalization contract explicit for
`bytearray` and `memoryview`; runtime behavior is unchanged.

The exact Windows recipe passes twice consecutively on Python 3.13.14 with all
five tests at 100% coverage and every quality gate clean. The immediate
store-engine downstream passes 57 tests at 96.67% coverage against editable
local dependencies. Focused recipe, uv-audit, parity reporter, and capability
taxonomy suites pass two, 13, ten, and seven tests. The Go build tool passes
tests, vet, and compilation.

With `uv` explicitly available to the child shell and the compatibility
variables already owned by pending legacy-front repairs, the build tool's real
494-package affected execution builds `hash-functions`, `hyperloglog`,
`resp-protocol`, and `in-memory-data-store-protocol`. It then reaches the
unchanged `in-memory-data-store-engine` front and reproduces its separately
owned Windows-invalid quoted editable requirement; the composed store is
dependency-skipped. The protocol blocker is therefore closed without widening
this tranche into `python-data-store-build-front-idempotence`.

After a clean rebase onto current `origin/main`
`de94b5ab33e04e13d0581f3ecf6a846049cb7d6d`, the collision-checked inventory
remains unchanged at 15 established lanes, 1,369 implementation identities,
4,562 package slots, and zero collisions or unknown buckets.

### Python data-store protocol BUILD-front publication

Ready-for-review PR #12495 opened from independently reviewed head
`f8fe85c86c50225945784a2440d6179fad9ebecd` after a normal first push from
exact `origin/main` `de94b5ab33e04e13d0581f3ecf6a846049cb7d6d`.
Ten open PRs and 31 remote source heads have zero selected-package or full-diff
overlap. Independent security/ownership, implementation, and state/roadmap
reviews all pass with no required fix. GitHub reports the PR open, non-draft,
and mergeable. Final head `d1c3d2d6d1109b1e1fbbd97e55444e347ae23bb3`
completed all 30 checks with 24 successes, five expected skips, one neutral
CodeQL result, and no failures or pending work. GitHub's REST state reported the
branch clean and mergeable; the loop requested squash auto-merge, and GitHub
merged PR #12495 as `ec2f913facaf2ecf00ebc1a0393d289220cde39d` at
2026-08-24T03:44:58Z.

### Post-#12495 refresh and Hash Functions strict-MyPy selection

The collision-checked schema-3 inventory at exact merged main
`ec2f913facaf2ecf00ebc1a0393d289220cde39d` remains identity-neutral: 15
established lanes, 1,369 implementation identities, 4,562 package slots, 175
high-consensus identities with 276 gaps, 122 identities in five to nine lanes
with 926 gaps, 166 identities in two to four lanes with 2,087 gaps, and 906
singletons with 12,684 gaps. Collisions and unknown buckets remain zero, and no
new unowned portable identity appeared. Merged Roku PR #12488 did expand the
Rust native host's concrete TCP/HTTP, timeout, CLI, and runtime-command
authority without a capability profile, so new selection-blocked owner
`smart-home-roku-ecp-native-authority-review` records that excluded native
review before selection. The existing portable Roku owner now cites #12488's
deterministic media-control behavior as its Rust reference.

The dependency audit adds the already registered
`python-hash-functions-build-front-strict-mypy` owner to the final data-store
owner's prerequisites before selection. This closes the missing DAG edge: the
merged Hash Functions state claims strict checking, but both active recipes
still invoke plain MyPy. Strict MyPy already passes all nine source/test files,
so the selected correction is limited to two BUILD commands, their exact
recipe expectations, package documentation, and the historical validation
claim. It makes no production-source, dependency, capability, or behavioral
test change. RESP/TCP remains the next dependency-leveraged prerequisite.

Live audits across six open PRs and 27 remote source heads find zero overlap on
Hash Functions, its recipe regression, parity state, or roadmap. The target
remote branch and PR were absent before creation.

### Hash Functions strict-MyPy implementation and validation

The canonical and Windows Hash Functions BUILD recipes now add only MyPy's
`--strict` flag to the repaired commands, and the exact-recipe regression pins
both spellings. No package source, behavioral test, dependency, capability,
production-authority, or privileged-boundary surface changes.

The exact Windows front passes twice consecutively with uv 0.11.28, Python
3.13.14, Ruff, formatting, strict MyPy across all nine source/test files, 88
tests, and 100% coverage. Exact dependent Windows fronts pass Bloom Filter (47
tests, 96.15% coverage), Hash Map (116 tests, 96.04%), HyperLogLog (61 tests,
98.88%), and the already repaired data-store protocol (five tests, 100%); the
immediate engine downstream passes 57 tests at 96.67% under its local sibling
closure. Focused recipe, uv-audit, parity-reporter, and capability-taxonomy
suites pass 2, 13, 10, and 7 tests. The Go build tool passes tests, vet, and
trimpath compilation.

The exact build-tool dry plan evaluates 45 Starlark files and 494 Python
packages, selecting the expected one changed and eight affected packages. With
the uv 0.11.28 executable directory on `PATH` and explicit `UV_PYTHON=3.13`
plus `UV_VENV_CLEAR=1` only for separately owned legacy fronts, the real
affected execution builds `hash-functions`, `bloom-filter`, `hash-map`,
`hyperloglog`, `in-memory-data-store-protocol`, and `resp-protocol`; it then
reproduces the unchanged engine front's separately owned Windows-invalid
quoted editable requirement and dependency-skips the composed store. That
failure remains within `python-data-store-build-front-idempotence` and does not
widen this strict-MyPy correction.

Ready-for-review PR #12501 was opened from independently reviewed head
`ca3ab61352b1e7ac19194aa4cc3488bd27933139` after a normal first push from exact
`origin/main` `ec2f913facaf2ecf00ebc1a0393d289220cde39d`. Eight open PRs and 30
non-main remote heads have zero exact implementation, state, roadmap,
dependency-fixture, or recipe-test overlap. Independent implementation/claim,
security/ownership, and roadmap/state reviews pass with no remaining defect.
Final head `71d82e170b90100d8ec070305eaebaba603e15f9` completed all 30
checks with 24 successes, five expected skips, one neutral CodeQL result, and
no failures or pending work. GitHub reported the branch clean and mergeable;
the loop requested squash auto-merge, and GitHub merged PR #12501 as
`8ca09a9928a113c358a625fe16938b02fb1e38c5` at 2026-08-24T04:44:15Z.

### Post-#12501 refresh and RESP/TCP selection

The collision-checked schema-3 inventory at exact merged main
`8ca09a9928a113c358a625fe16938b02fb1e38c5` remains identity-neutral: 15
established lanes, 1,369 implementation identities, 4,562 package slots, 175
high-consensus identities with 276 gaps, 122 identities in five to nine lanes
with 926 gaps, 166 identities in two to four lanes with 2,087 gaps, and 906
singletons with 12,684 gaps. There are 717 Rust singletons, zero collisions,
and zero unknown buckets. OCaml remains correctly emerging with no packages;
no new identity or unowned portable gap appeared.

The reference audit did discover a contract discrepancy within existing
`tcp-server`: DT24, the README, and source define a protocol-agnostic raw-byte
server with no RESP import, while package metadata, both BUILD fronts, and the
uv fixture encode RESP as a runtime prerequisite. New pending owner
`python-tcp-server-resp-dependency-contract-reconciliation` records that follow-up and
depends on the current backfill. The selected generated-standard repair must
preserve the fixture's explicit RESP-before-TCP order, so it does not widen
into dependency-metadata or historical-fixture redesign.

`python-resp-tcp-build-front-idempotence` is dependency-ready and is the sole
remaining prerequisite that unlocks the final data-store repair. It repairs
exactly the RESP then TCP fronts under the generated-standard profile: pinned
Python 3.13 environments recreated on every run, named-interpreter installs
and quality gates, complete-recipe regressions, immediate repeat validation,
and the real dependent closure. Nine open PRs and 31 non-main remote heads
have zero exact overlap; the target branch and prior PR were absent before
creation.

### RESP/TCP BUILD-front implementation and validation

All four complete recipes are now pinned by two focused regressions. On
Windows with uv 0.11.28 and Python 3.13.14, every `BUILD_windows` line passes
twice consecutively from a cleared package-local environment: RESP Protocol
passes 129 tests at 100% coverage and TCP Server passes 23 tests at 100%, with
Ruff, formatting, and strict MyPy clean in both runs. TCP preserves the
fixture-required RESP prerequisite order and uses `--follow-untyped-imports`
for that untyped sibling. Two focused tests make the listening-socket and
client-unregister cleanup exception paths load-bearing, raising TCP's coverage
from the fragile pre-repair margin to 100%. The canonical Unix recipes are
structurally pinned for exact Linux and macOS CI execution.

The live uv audit falls from 13 to 11 non-idempotent fronts. The focused
recipe/uv suites pass 15 tests plus 22 subtests, and the parity/capability
suites pass 17 tests plus 719 subtests. The Go build tool passes all package
tests, vet, and trimpath compilation. A freshly compiled binary validates 494
Python BUILD fronts and selects exactly seven affected packages:
`hash-functions`, `hyperloglog`, `in-memory-data-store-protocol`,
`resp-protocol`, `tcp-server`, `in-memory-data-store-engine`, and
`in-memory-data-store`.

With the uv 0.11.28 executable directory on `PATH` and explicit
`UV_PYTHON=3.13` plus `UV_VENV_CLEAR=1` only for separately owned legacy
fronts, real affected execution builds the first five packages, then
reproduces the unchanged engine front's Windows-invalid `".[dev]"` parse
error and dependency-skips the store. Direct local downstream runs with that
separately owned requirement spelled unquoted pass the engine's 57 tests at
96.67% coverage and the store's five tests at 100%. After non-overlapping
publication-time rebases, the collision-checked schema-3 inventory at exact
current main `19c2cfb6399a2d3513f5db7d1f656dce906a8a3b` remains at 15 lanes,
1,369 identities, and 4,562 slots with zero collisions or unknown buckets; the
430-node, 612-edge state remains unique and dependency-complete. Dependency,
capability, credential,
production-authority, and diff checks are clean. The tranche does not change
runtime dependencies, capability metadata, protocol behavior, or a privileged
boundary.

Independent implementation/claim and security/ownership reviews pass with no
actionable defect. The state/readiness review's stale-main and stale-inventory
findings were corrected by the final rebase and exact-main inventory refresh.

Ready-for-review PR #12512 was opened from head
`edaf6f5cf952c633b2b0c327bedbb04d3d99544d` after a normal first push from
exact `origin/main` `19c2cfb6399a2d3513f5db7d1f656dce906a8a3b`. GitHub reports the
PR mergeable while required checks are queued or in progress; auto-merge
remains disabled until every required check is terminal and acceptable.

Final head `0c082d97645ce1f3b1856b641693023f3f8740a4` completed all 30
checks with 24 successes, five expected skips, one neutral CodeQL result, and
no failures or pending work. GitHub reported the branch clean and mergeable;
the loop requested squash auto-merge, and GitHub merged PR #12512 as
`087ec0e50e951a72d181124e9e0e1effcc2dfecc` at 2026-08-24T05:54:41Z.

### Post-#12512 refresh and data-store selection

The collision-checked schema-3 inventory at exact current main
`4d00a4560805a40ac2147d465f3ecb7b91a4f18b` remains identity-neutral: 15
established lanes, 1,369 implementation identities, 4,562 package slots, 175
high-consensus identities with 276 gaps, 122 identities in five to nine lanes
with 926 gaps, 166 identities in two to four lanes with 2,087 gaps, and 906
singletons with 12,684 gaps. There are 717 Rust singletons, zero collisions,
and zero unknown buckets. OCaml remains correctly emerging with no packages;
no new identity or package marker appeared.

The same audit found one separate privileged contract and registered it before
selection as
`python-in-memory-data-store-filesystem-authority-review`. The composed store
accepts a caller-supplied AOF path and performs reads, directory creation,
append writes, flushes, and `fsync` while declaring an empty capability
profile. That owner is selection-blocked pending least-privilege filesystem
review and Layer-5 approval (or an injected authority-bearing adapter); it must
not widen the build-front repair.

`python-data-store-build-front-idempotence` is now dependency-ready after all
five prerequisites merged. It repairs exactly the engine then composed-store
fronts, closes the final hash/RESP/data-store dependency chain, and owns the
repeatedly reproduced Windows-invalid quoted editable requirement. Seven open
PRs and 30 non-main remote heads have zero exact overlap; the target branch
and prior PR were absent before creation. The ready TCP dependency-contract
reconciliation does not block this closure.

Pre-selection quality diagnostics show the engine already passes Ruff,
formatting, strict MyPy, 57 tests, and 96.67% coverage. The composed store
passes Ruff, formatting, five tests, and 100% coverage, while strict MyPy
exposes four findings across its response conversion and response-shape test;
the selected owner must bound any cleanup to behavior-preserving typing and
test narrowing within the two owned roots.

The implemented repair pins all four complete recipes. On Windows with uv
0.11.28 and Python 3.13.14, each BUILD_windows front passes twice from a
cleared environment: the engine passes 57 tests at 96.67% coverage and the
store passes five tests at 100%, with Ruff, formatting, strict MyPy, and
dependency checks clean throughout. Kind-specific response narrowing and
runtime-proved decoded-frame test narrowing resolve the store's four dormant
type errors, allowing removal of the broad missing-import suppression without
changing behavior.

The uv audit shrinks exactly from 11 to nine fronts across seven components.
Focused recipe, audit, parity, and capability suites pass. Both packages build
wheel and source distributions, install with their explicit closure into a
fresh Python 3.13 environment, and pass engine and RESP smoke tests. The Go
build tool passes tests, vet, and trimpath compilation; its fresh binary
validates 4,982 packages, selects exactly the six-package hash/protocol/data-
store closure, and completes the real serial build. The rebased schema-3
inventory and 431-node, 613-edge state remain collision-free, complete, and
acyclic. Security review finds no new dependency, credential, network,
process, capability, or production-authority surface; the pre-existing AOF
filesystem authority remains isolated in its separately blocked owner.

### Post-#12520 refresh and Wemo authority registration

Final head `1df0eee904d1828a9b4858130d843fb15e331b73` completed all 30
reported checks with 24 successes, five expected skips, one neutral CodeQL
result, and no failures or pending work. GitHub reported PR #12520 clean and
mergeable; the loop requested squash auto-merge, and GitHub merged it as
`e5eb4be0c234456a3a5c0ae627ed88a723b066b6` at 2026-08-24T07:22:29Z.

The collision-checked schema-3 inventory at that exact merged main remains
identity-neutral: 15 established lanes, 1,369 implementation identities, 4,562
package slots, 175 high-consensus identities with 276 gaps, 122 identities in
five to nine lanes with 926 gaps, 166 identities in two to four lanes with
2,087 gaps, and 906 singletons with 12,684 gaps. There are 717 Rust
singletons, zero collisions, and zero unknown buckets. OCaml remains correctly
emerging with no packages; no package marker, identity, or slot changed. The
live uv audit now reports the expected nine fronts across seven components.

The same audit found one newly unowned privileged contract introduced by
merged Wemo PR #12515. The Rust Wemo host now performs recognized outlet and
switch mutations with idempotent pre-read/set/readback execution, concrete
SSDP/TCP/HTTP endpoint policy, catalog-command capability, CLI effects, and
runtime mutation, but the crate has no `required_capabilities.json`. New owner
`smart-home-wemo-upnp-native-authority-review` records that excluded work as
selection-blocked behind the portable Wemo core and the shared Rust network-
substrate capability review. The existing portable owner now includes
recognized-model, endpoint-policy, and pre-read/readback fixtures without
making native network or CLI authority portable.

The dependency/leverage pass selects
`python-graph-build-front-idempotence`. Graph followed by directed-graph is the
largest zero-overlap ready uv-debt component: 97 Python package manifests and
118 unique direct BUILD consumers reference the pair, versus six manifest
consumers for trie/radix. Eight live open PRs and 29 non-main source heads have
zero exact overlap on the package roots, focused recipe/audit tests, state,
roadmap, or repair profile; the target branch and prior PR are absent. The
dependency-ready OCaml process-free build substrate remains strategically
important, but three live build-tool PRs currently overlap its validator,
main, and changelog surfaces, so the conflict-free graph chain is the safer
serial delivery choice without changing OCaml's durable priority.

Pre-selection diagnostics pass 132 graph tests at 95.04% coverage and 129
directed-graph tests at 97.62%. Strict MyPy is already clean for graph. The
generated-standard quality profile also exposes bounded Ruff and formatting
debt in both packages plus seven directed-graph strict-MyPy findings; the
selected repair must resolve those findings without changing behavior,
runtime dependencies, capabilities, or the graph-before-directed dependency
order.

The implemented repair pins all four canonical and Windows recipes to Python
3.13, recreates each named environment with `--clear`, preserves graph before
directed-graph installation, and invokes that interpreter for Ruff lint and
format checks, strict MyPy, and pytest. Exact recipe regressions and the uv
audit now report seven live fronts rather than nine. Both Windows fronts pass
twice consecutively on uv 0.11.28 and Python 3.13.14: graph passes 134 tests at
96.14% coverage and directed-graph passes 129 at 97.63%, with every quality
gate clean. State-machine passes 161 downstream tests at 99.02% coverage and
tree passes 137 at 100%; both selected packages build wheel and source
distributions and retain `py.typed` in their wheels.

A real Windows build-tool execution validates the 4,982-package, 9,489-edge
plan and attempts all 205 affected packages. It builds both selected roots and
196 packages total, skips 289 unchanged packages, and exposes four unrelated
pre-existing failures plus five dependent skips. Exact replay classifies them
under three newly registered pending owners: an unpinned Python 3.13 front for
Markov chain, truthful Windows symlink-capability testing for ir-to-jvm-class-
file and unix-tools, and scaffold-generator temporary-root cleanup after a
current-directory change. Those owners preserve the evidence without widening
this dependency-shaped repair.

### Post-#12538 refresh and Markov chain selection

Final head `0db73720dea50544fdcf90f4c437bfa5b2c1c841` completed all 30
reported checks with 24 successes, five expected skips, one neutral aggregate
CodeQL result, and no failures or pending work. GitHub reported PR #12538 clean
and mergeable; the loop enabled squash auto-merge, and GitHub merged it as
`6f4f3dad8c3740577813e13990069c452b7246ce` at
2026-08-24T09:49:20Z without a manual merge command.

The collision-checked schema-3 inventory remains identity-neutral after that
merge and the subsequent Tamil and Devanagari curriculum-only commits through
`78efa57321d11eb7a72674f2c16dca56eea6e519`: 15 established lanes,
1,369 implementation identities, 4,562 package slots, 175 high-consensus
identities with 276 gaps, 122 identities in five to nine lanes with 926 gaps,
166 identities in two to four lanes with 2,087 gaps, and 906 singletons with
12,684 gaps. There are 717 Rust singletons, zero canonical collisions, and
zero unknown buckets. OCaml remains correctly emerging with no packages, and
the graph repair reduces the live uv audit exactly from nine fronts to seven
across six dependency components.

The intervening merges add no implementation identity but expose work that
must be owned before selection. PR #12153 makes orphan-crate BUILD coverage a
Go-only validation contract, so the roadmap now records a language-neutral
orphan-crate validation corpus followed by every remaining supported build-tool
engine. Its new `code/BUILD-EXEMPTIONS` ledger also records five exact
host-qualified fronts: `jit-loader-macos`, `twig-module-driver`,
`sir-conformance`, `paint-vm-cairo`, and `paint-vm-skia`. Each has its own
pending owner; selection remains blocked until the named macOS, Linux,
libcairo, large-stack, or Skia toolchain evidence is available.

PR #12527 expands BACnet beyond the existing discovery-only owners with fixed
Device-object ReadProperty telemetry, an injected snapshot core, and connected
UDP reads. Three new owners preserve that boundary: first a neutral bounded
ReadProperty codec contract, then the authority-free injected telemetry core,
then a selection-blocked native UDP/runtime authority review. The existing
BACnet discovery owners continue to exclude property reads and native telemetry
rather than being silently widened. The OCaml process-free substrate also owns
the stale `OCAML03-ci-toolchain.md` status correction when that tranche becomes
collision-free.

The dependency/leverage pass selects
`python-markov-chain-build-front-python313`. It is the only owner newly
unblocked by the graph merge and directly closes a reproduced failure in the
real 205-package affected closure: both Markov fronts clear their environment
but omit a Python pin, so ambient Python 3.10 rejects graph's Python >=3.12
floor. Replaying with `UV_PYTHON=3.13` passes all 91 tests at 97.62% coverage,
isolating build-front interpreter selection rather than graph behavior. Seven
open PRs and 30 non-main remote heads have zero exact overlap on the Markov
root, its focused regression, state, roadmap, or governing uv spec; the target
branch and prior PR are absent.

Trie/radix and the remaining independent uv components stay ready follow-ups.
Windows symlink portability overlaps live PR #12162, while the strategically
important OCaml process-free substrate still overlaps live build-tool PRs
#12162 and #12149 in `validator.go`, validator tests, and `main.go`. Markov is
therefore the highest-leverage collision-free serial continuation without
changing OCaml's durable priority.

The implemented Markov repair now pins both BUILD fronts to Python 3.13,
recreates the named environment, preserves graph before directed-graph
installation, and runs Ruff lint, Ruff formatting, strict MyPy, and pytest
through the explicit interpreter. The Windows recipe passes twice
consecutively on uv 0.11.28 and Python 3.13.14; each run passes 91 tests at
97.62% coverage. Bounded generic annotations and formatting satisfy the new
gates without changing behavior, dependencies, capabilities, or version
metadata. Wheel and source distributions build successfully, `py.typed`
survives in the wheel, and a fresh isolated install of that exact wheel passes
a deterministic generation smoke.

The Go build tool passes its full test suite, vet, and trimpath compilation.
Repository-wide BUILD validation succeeds across 494 discovered packages. A
real Windows diff execution identifies exactly one changed package, expands it
to the three-package graph, directed-graph, and Markov closure, and builds all
three without failures. Focused recipe, uv-audit, parity-reporter, capability-
taxonomy, state-graph, diff, credential, dependency, and production-authority
checks are clean. The live uv audit intentionally remains at seven fronts
across six components because Markov was discovered through the previous real
affected closure rather than the already-clear audit corpus.

### Post-#12549 refresh and Starlark metering selection

Final head `cfc70cf12374e60f4c87d67631647f5ff851b2af` completed all 30
reported checks with 24 successes, five expected skips, one neutral aggregate
CodeQL result, and no failures or pending work. GitHub reported PR #12549 clean
and mergeable; the loop enabled squash auto-merge, and GitHub merged it as
`0978453133ed2e40b94c549a3737440a8b67116e` at
2026-08-24T11:08:17Z without a manual merge command.

The collision-checked schema-3 inventory remains identity-neutral through the
subsequent tracked-artifact validator and Spanish-curriculum merges at exact
main `35ead36e61adab3bcf8f86a9cf527c633d96012a`: 15 established
lanes, 1,369 implementation identities, 4,562 package slots, 175
high-consensus identities with 276 gaps, 122 identities in five to nine lanes
with 926 gaps, 166 identities in two to four lanes with 2,087 gaps, and 906
singletons with 12,684 gaps. There are 717 Rust singletons, zero canonical
collisions, and zero unknown buckets. OCaml remains correctly emerging with no
packages, and the live uv audit remains at seven fronts across six components.

The intervening work adds no implementation identity but exposes three owners
that are recorded before selection. The new Go-only tracked-`node_modules`
gate requires a closed language-neutral tracked-artifact validation corpus and
a remaining-engine rollout; neither Git enumeration nor host filesystem access
belongs in that process-free oracle. The six manifest-driven human-language
writer CLIs also require a selection-blocked filesystem-authority and
capability review covering their exact outputs and containment boundary; that
native generator-shell review is not a portable all-language port.

The dependency/leverage pass selects `build-tool-starlark-metering-corpus`.
It is ready behind the merged pure-domain corpus, process-free, and has zero
exact overlap across 11 live open PRs and 33 non-main remote heads; its target
branch and prior PR are absent. The fixtures must close fuel, recursion,
aggregate allocation, range and value size, module count and load depth,
load-cycle, and print/trace-output behavior before any native evaluator claims
final Starlark conformance. This owner is a direct prerequisite of the Go
build-tool oracle and has seven unfinished descendants, so it advances the
durable every-language build-tool contract more broadly than the collision-free
trie/radix uv repair.

The validation corpus, CLI parser corpus, and OCaml process-free substrate have
higher raw strategic reach but remain unsafe to select while PRs #12162 and
#12149 overlap their canonical Go `main.go` and validator surfaces. Trie
followed by radix-tree remains the highest-leverage collision-free uv follow-up
and would remove the only remaining two-node dependency component. Selecting
the neutral Starlark corpus now therefore preserves implementation-PR
serialization without changing the durable OCaml or Python priorities.

Before publication, main advanced again through the already-owned HTML parser,
Mermaid XY layout, IPP printer-status runtime, and Devanagari curriculum work.
The branch rebased conflict-free onto exact main
`9eb5934aefd3c5fb216ec8171e803c6f91a6f938`. The refreshed inventory adds one
identity and one slot: the pure Rust-only `ipp-protocol` singleton introduced
by the status runtime. The roadmap now owns that zero-authority codec through a
neutral fixture and established-lane port, and makes the existing injected IPP
printer core consume it. Exact-main totals are therefore 1,370 implementation
identities, 4,563 slots, 907 singletons with 12,698 missing slots, and 718 Rust
singletons; all high-consensus and middle-band totals remain unchanged, with
zero collisions and zero unknown buckets. This late identity does not overlap
or displace the already selected Starlark contract slice.

### Post-#12562 refresh and Python trie selection

Final head `876c19a0c247e4ace99f5017462da0aa11e9e80e` completed all 29
reported checks with 23 successes, six expected skips, and no failures or
pending work. GitHub reported PR #12562 clean and mergeable; the loop enabled
squash auto-merge, and GitHub merged it as
`707a5d39f8d5c68f769200a9cfc466bd1c231693` at
2026-08-24T12:02:53Z without a manual merge command.

The collision-checked schema-3 inventory remains identity-neutral through the
subsequent Devanagari curriculum-only commit at exact main
`69d816d78994217c72e7d956d5f59872406b6282`: 15 established lanes,
1,370 implementation identities, 4,563 package slots, 175 high-consensus
identities with 276 gaps, 122 identities in five to nine lanes with 926 gaps,
166 identities in two to four lanes with 2,087 gaps, and 907 singletons with
12,698 gaps. There are 718 Rust singletons, zero canonical collisions, and
zero unknown buckets. OCaml remains correctly emerging with no packages.

The implementation audit records one new owner before selection. The Starlark
metering corpus now supplies ten neutral cases and a Python reference oracle,
but no native build-tool engine consumes the exact fuel, recursion, allocation,
range, scalar, load-graph, or combined-output limits. The pending
`build-tool-starlark-metering-remaining-engines` umbrella therefore follows the
canonical Go oracle and must be decomposed into independent engine children;
future Dart and JVM engines must consume the same process-free contract.

The dependency/leverage pass selects
`python-trie-build-front-idempotence`. Trie followed by radix-tree is ready
behind the merged uv audit, owns the only remaining two-package dependency
component, and removes two of the seven measured live fronts across six
components. Six Python manifests consume the pair. Nine live open PRs and 32
non-main remote heads have zero exact or semantic overlap on the package roots,
focused regression and audit tests, state, roadmap, or governing uv spec; the
target branch and prior PR are absent.

The build-tool validation and CLI corpora and the OCaml process-free substrate
remain strategically stronger but overlap live PRs #12162 and #12149 in Go
validator and main surfaces. IPP portability is collision-free but is a broader
all-lane tranche with fewer immediate descendants. The dependency-shaped trie
repair is therefore the safest serial continuation while those higher-leverage
owners remain collision-blocked.

The selected repair now gives both trie and radix-tree complete, immediately
repeatable Python 3.13 recipes. Each front removes its named environment first,
uses that interpreter for Ruff lint and format checks, strict MyPy, and pytest,
and preserves the required trie-before-radix install order. On Windows with uv
0.11.28 and Python 3.13.14, both canonical recipes pass twice consecutively:
trie passes 95 tests at 99.41% coverage and radix-tree passes 84 tests at
97.97% coverage. Wheel and source-distribution builds pass for both packages,
and the focused recipe, live-audit, and Markov regression suites pass all 18
tests. The measured uv debt therefore falls exactly from seven fronts to five.

The required affected-closure run discovers one further owner rather than
widening this tranche. The Go build tool passes its complete tests, vet, and
trimpath compilation; its exact diff plan evaluates 45 Starlark files,
discovers 494 Python packages, and selects exactly trie, radix-tree, and LZ78.
The real execution builds both owned packages. LZ78 alone fails before tests
because its existing Windows front selects ambient Python 3.10.11 while trie
requires Python 3.12 or newer; an explicit Python 3.13 environment passes all
48 LZ78 tests at 97.26% coverage. The new pending
`python-lz78-build-front-python313` owner follows this repair and keeps that
pinning correction, its complete-recipe regression, and any dormant LZ78
quality debt in an independently reviewable dependency-shaped slice.

### Post-#12572 refresh and validation-oracle selection

Final head `8f4bd8ea3dca1f0ec0fdf1c66f3ea25a2a63f3f8` completed all 30
reported checks with 24 successes, five expected skips, one neutral aggregate
CodeQL result, and no failures or pending work. GitHub reported PR #12572 clean
and mergeable; the loop enabled squash auto-merge, and GitHub merged it as
`42b29dd5f03b93841660848628c49d9a1130664b` at
2026-08-24T12:55:12Z without a manual merge command.

The collision-checked schema-3 inventory remains identity-neutral through the
subsequent human-language and already-owned HTML diagnostic changes at exact
main `2bf4896ee63c359e6f04c57cdc9da6a14fcf84b2`: 15 established lanes,
1,370 implementation identities, 4,563 package slots, 175 high-consensus
identities with 276 gaps, 122 identities in five to nine lanes with 926 gaps,
166 identities in two to four lanes with 2,087 gaps, and 907 singletons with
12,698 gaps. There are 718 Rust singletons, zero canonical collisions, and
zero unknown buckets. OCaml remains correctly emerging with no packages. The
exact implementation, fixture, and authority audit exposes no unowned gap;
LZ78's affected-closure Python pin is already a pending registered owner.

The dependency/leverage pass selects
`build-tool-validation-oracle-corpus`. It is ready behind the merged pure-domain
corpus, directly unlocks the Go oracle plus orphan-crate and tracked-artifact
validation corpora, and has 12 unfinished build-tool and OCaml descendants.
Nine live open PRs and 35 non-main remote heads have zero exact overlap on the
governing conformance spec, closed schema, neutral fixtures, Python reference
runner, runner/schema tests, state, or roadmap; the target branch and prior PR
are absent.

PRs #12162 and #12149 still block implementation-coupled Go validator and
OCaml integration surfaces, but they do not touch this language-neutral corpus
slice. This narrows the earlier conservative roadmap classification: neutral
validation and CLI fixtures may proceed while native implementation edits stay
serialized behind those live branches. The newly ready LZ78 repair remains a
small collision-free follow-up, but it unlocks no state descendants; closing
the deterministic process-free validation contract therefore has materially
greater leverage toward the every-language build-tool and OCaml promotion
requirements.

The selected corpus is now implemented without widening into any native
build-tool engine. Validation v1 exposes exactly nine checks and derives every
diagnostic from one bounded normalized snapshot. Six new fixtures cover a
fully clean transitive prerequisite chain, undeclared local references,
missing standalone prerequisites, unsafe Starlark sources and unknown
dependencies, ambiguous package roots and manifests, an unsupported language,
and unsafe raw paths. Unknown graph endpoints, normalized duplicate package
identities, cycles, and self-consistent dishonest results fail closed. The
runner retains safe package roots as diagnostic paths, keeps unsafe inputs only
in bounded details, and the full pure-domain side-effect test proves no
filesystem, process, Git, or network access.

Local validation passes the 75-case corpus, all 18 schema and 48 runner tests
with 88% branch-aware runner coverage, 121 downstream execution, authority,
loader, broker, and Linux-backend tests with their expected platform skips,
and the process-free execution-contract validator. Ruff error-class lint and
format checks, compileall, Bandit, the Go build tool's full tests at 78.3%
aggregate coverage plus vet and trimpath build, a real 5,070-package
BUILD-validation plan, the collision-free schema-3 parity report and its ten
tests, capability taxonomy, Haskell capability, and OCaml toolchain-lock suites
also pass. Strict MyPy remains a non-gating legacy audit with 11 errors in
unchanged runner lines; this tranche adds none.

### Post-#12577 refresh and CLI-parser selection

Final head `0c97091ff2c1ab526f41fe518377d77050bd51be` completed all 29
reported checks with 23 successes and six expected skips. GitHub reported PR
#12577 clean and mergeable; the loop enabled squash auto-merge, and GitHub
merged it as `aacb6a163b599737e821a2083aa500ce26b7b100` at
2026-08-24T13:50:10Z without a manual merge command.

The collision-checked exact-main schema-3 inventory remains unchanged: 15
established lanes, 1,370 implementation identities, 4,563 package slots, 175
high-consensus identities with 276 gaps, 122 identities in five to nine lanes
with 926 gaps, 166 identities in two to four lanes with 2,087 gaps, and 907
singletons with 12,698 gaps. There are 718 Rust singletons, zero canonical
collisions, and zero unknown buckets. OCaml remains correctly emerging with no
packages. No package marker changed in the merge.

The post-merge implementation audit registers one newly exposed rollout before
selection: `build-tool-validation-oracle-remaining-engines`, pending behind the
canonical Go oracle. The neutral corpus now specifies all nine validation
checks, but no native engine consumes its new cases; C#, Elixir, Haskell, Lua,
Perl, Python, Ruby, Rust, Swift, and TypeScript therefore need decomposed engine
children later, with future Dart and JVM implementations routed through the
same contract. Schema and reference-runner coverage are not engine parity.

The dependency/leverage pass selects `build-tool-cli-parser-corpus`. It is the
last independent process-free parser prerequisite on the Go-oracle path, is
ready behind the merged pure-domain corpus, and has nine unfinished descendants
through build-tool and OCaml delivery. Ten live open PRs and 34 current
non-main heads have zero exact candidate overlap; only the already merged
validation-corpus branch touches the shared neutral surfaces, and the target
branch and prior PR are absent. The orphan-crate and tracked-artifact corpora
each unlock one remaining-engine umbrella, while the collision-free LZ78 repair
unlocks none. Go-oracle and OCaml implementation surfaces remain collision-
blocked by live PRs #12162 and #12149, so their neutral CLI prerequisite is the
highest-leverage safe serial continuation.

Local cross-version validation also registered
`build-tool-windows-python313-execution-snapshot-volume-identity` as a separate
pending owner behind the merged execution snapshot. Five unchanged Windows
execution-schema tests pass under Python 3.10 but fail identically on a clean
baseline and this branch under Python 3.13 because the newer interpreter's
64-bit `st_dev` does not equal the existing 32-bit volume-serial projection.
That native filesystem identity debt is outside this inert CLI-parser slice.

Before publication, the completed parser corpus rebased conflict-free onto
exact `origin/main` `561cbd2a9365bc5093fdd6189fd2e0e065e16506` after three Devanagari
curriculum commits and the merged HTML-parser diagnostic repair. None changes a
package marker or overlaps this tranche. The collision gate remains at 15
established lanes, 1,370 implementation identities, 4,563 package slots, 175
high-consensus identities with 276 gaps, 907 singletons with 12,698 gaps, 718
Rust singletons, zero collisions, and zero unknown buckets; OCaml remains
correctly emerging with zero packages.

The downstream audit also registers
`build-tool-dist-newstyle-discovery-exclusion-remaining-engines`. Go accepts the
shared language-registry fixture, while Ruby retains one failure among 307 runs
and 632 assertions and Rust retains one failure among 140 tests because both
still discover `code/packages/haskell/dist-newstyle` as a package on Windows.
That pre-existing engine discovery debt is now explicit behind the merged Ruby
and Rust identity registries and remains outside the neutral CLI-parser change.

Ready-for-review PR #12584 opened from reviewed head
`df678cc47f147aac265449723c10bc0b3554195d` on exact main
`561cbd2a9365bc5093fdd6189fd2e0e065e16506`. Nine live open PRs have no
exact changed-file overlap; the only remote-head overlap is the already merged
squash source branch for PR #12577. GitHub reports #12584 mergeable with checks
queued, so auto-merge remains disabled until every required check is terminal
and acceptable.

### Post-#12584 refresh and orphan-crate corpus selection

Final head `9ba1dbe29c90817b31dfd33448bd8e1fccc77a8a` completed all 29
reported checks with 23 successes, six expected skips, and no failures or
pending work. GitHub reported PR #12584 clean and mergeable; the loop enabled
squash auto-merge, and GitHub merged it as
`f7f929e494aa77573bac8bf19d57583bdab1b783` at 2026-08-24T15:13:38Z
without a manual merge command.

After the merge, `main` advanced only through unrelated Devanagari curriculum
work to exact revision `0ba76cb0b5b0463179c7f4d0ee338db1ddedc01f`. The
collision-checked schema-3 inventory remains 15 established lanes, 1,370
identities, and 4,563 present slots: 175 high-consensus identities have 276
gaps, 122 identities in five to nine lanes have 926 gaps, 166 identities in
two to four lanes have 2,087 gaps, and 907 singletons have 12,698 gaps. There
are 718 Rust singletons, zero canonical collisions, and zero unknown buckets.
OCaml remains correctly emerging with no packages and therefore remains
outside the established-language denominator.

The required post-merge audit registered the missing native CLI-parser rollout
behind the Go oracle. It also narrowed the existing `dist-newstyle` repair to
Ruby and Rust after reproducing separate TypeScript and Swift failures, then
registered those engines under independent owners. The orphan-crate and
tracked-artifact neutral corpora are now explicit Go-oracle prerequisites. The
resulting state graph has 458 unique owners and 637 dependency edges with one
in-progress item, no duplicate IDs, missing dependencies, self-edges, or
cycles. The orphan audit additionally records pure Go-oracle debt for removed
manifests, nonportable ledger cleanup, normalized duplicates, and REM-only
BUILD bypasses, while no-follow/reparse/file-kind handling is isolated in a
selection-blocked host-scanner security review.

The dependency/leverage pass selects
`build-tool-orphan-crate-validation-corpus` on branch
`codex/build-tool-orphan-crate-validation-corpus` from exact base
`0ba76cb0b5b0463179c7f4d0ee338db1ddedc01f`. It is ready behind the
merged validation oracle, directly unlocks every remaining engine's orphan
validator, and protects against gaps that discovery, build, test, and lint
would otherwise never see. Eleven live open PRs and 35 current non-main heads
have zero candidate-surface overlap; the target branch and prior PR are absent,
and only stale squash-source branches for merged parity PRs intersect shared
history. The higher-leverage OCaml core remains collision-unsafe while PRs
#12149 and #12162 touch Go validator/main surfaces. The Ruby-only
`dist-newstyle` repair remains a ready sibling with TypeScript and Swift owned
separately; Rust's half was later closed by the focused CI repair in PR #12633.
The live reviewed exemption ledger contains five active entries,
so the neutral contract must preserve reasoned exceptions and countable
PENDING debt rather than treating every missing BUILD as equivalent.

The implemented neutral contract now derives orphan coverage from one closed,
bounded snapshot rather than trusting per-crate coverage assertions. Four new
cases cover direct and ancestor BUILDs, all platform BUILD names, package and
program roots, sibling noncoverage, exact artifact components, empty BUILDs,
virtual workspaces, compile-only and foreign-toolchain exclusions, active
PENDING debt, unsafe redacted entries, invalid-entry non-suppression, and all
three stale-ledger states. Exact portable paths join the snapshot; NFC/casefold
identities are used only to reject collisions and duplicate aliases. The
result exposes a derived `pending_exemption_count`, and schema conditionals
forbid both the snapshot and count when the orphan check is absent.

Final local validation passes the 106-case, 269-file corpus and 75 focused
Python 3.13 schema/runner tests at 89% branch-aware runner coverage. Python
3.10 passes the complete 196-test execution, authority, loader, broker,
backend, schema, and runner family with 23 expected platform skips. The Go
build tool passes all tests at 78.3% aggregate coverage plus vet and a trimpath
build; a real forced dry plan evaluates 45 Starlark BUILD files, discovers
5,070 packages, verifies the five-entry orphan ledger, and emits 9,778 edges.
Ten parity-reporter, seven capability-taxonomy, five Haskell-capability, and 46
OCaml-lock tests pass with two expected skips. Ruff error-class lint/format,
compileall, production Bandit, strict JSON, state/DAG, diff, dependency and
capability-manifest scope, production-authority, and added-line credential
checks pass. Strict MyPy retains ten findings only in unchanged legacy lines.

Before publication, the implementation rebased three times without conflict as
`main` advanced through two HTML-parser diagnostics and additional Devanagari
curriculum work to exact `origin/main`
`9d40f14652dabc95b698d092c275bff9e57200c5`. The refreshed schema-3
inventory retains all recorded counts with zero collisions and unknown
buckets. Ten live open PRs and 35 non-main remote heads have zero exact overlap
across the 14 changed paths; the locally unavailable `gh-pages` object was
checked through GitHub's tree API and also has no overlap. Three independent
read-only reviews found and verified closure of conditional-schema,
program-root, exact-join, redaction, generated-artifact, state, roadmap, and
host-authority ownership issues.

### Post-#12591 refresh and tracked-artifact corpus selection

Final head `9362bde637976b1c48c136850983fa8246a5f042` completed all 29
reported checks with 23 successes, six expected skips, and no failures or
pending work. GitHub reported PR #12591 clean and mergeable; the loop enabled
squash auto-merge, and GitHub merged it as
`053f6501b9b4b72c89253d198b9de6ad2a52e256` at 2026-08-24T16:48:14Z
without a manual merge command.

The collision-checked schema-3 inventory on that exact merge remains 15
established lanes, 1,370 identities, and 4,563 present slots: 175
high-consensus identities have 276 gaps, 122 identities in five to nine lanes
have 926 gaps, 166 identities in two to four lanes have 2,087 gaps, and 907
singletons have 12,698 gaps. There are 718 Rust singletons, zero canonical
collisions, and zero unknown buckets. OCaml remains correctly emerging with no
packages. Exact tree comparison from the prior inventory revision found no
identity, slot, directory-spelling, marker, or unknown-bucket change and no
newly unowned portable package.

The post-merge audit registers
`build-tool-go-tracked-artifact-index-authority-review` before selection. The
current Go scanner PATH-resolves and executes Git, buffers its index output,
has no timeout or process-tree cleanup, and can expose raw stderr and indexed
paths. That native process and Git-index authority is selection-blocked and
must remain outside the neutral corpus. The reconciled state graph now has 459
unique owners and 638 dependency edges with one in-progress item, no duplicate
IDs, missing dependencies, self-edges, or cycles.

The dependency/leverage pass selects
`build-tool-tracked-artifact-validation-corpus` on branch
`codex/build-tool-tracked-artifact-validation-corpus` from exact base
`053f6501b9b4b72c89253d198b9de6ad2a52e256`. This bounded process-free
contract is ready behind the merged validation oracle, removes another direct
Go-oracle prerequisite, and unlocks every remaining engine's tracked-artifact
validator, with eleven unfinished descendants through the Go oracle,
every-language build tools, and OCaml promotion. Eleven live open PRs and 34
non-main remote heads have zero expected-surface overlap; the target branch and
prior PR are absent. The newly ready orphan remaining-engine umbrella first
requires per-engine decomposition and has no descendants, while OCaml's
process-free core still overlaps PRs #12149 and #12162 on Go validator and main
surfaces.

### Tracked-artifact corpus implementation and validation

Validation v1 now exposes `tracked_artifact_absence` as its eleventh closed
check. Its bounded snapshot contains only strictly increasing ordinals, raw
paths, and inert regular, symlink, or reparse kinds. The independent oracle
normalizes separators, applies portable path checks, and rejects every exact,
nested, case-folded, or NFKC-compatible `node_modules` component. Unsafe paths
produce `TRACKED_ARTIFACT_PATH_INVALID` at the fixed `repository` path without
echoing hostile input; safe forbidden paths produce
`TRACKED_ARTIFACT_FORBIDDEN`. Entry kind never authorizes a read or follow, and
the neutral implementation does not inspect Git, the filesystem, processes,
the environment, or the network.

Four new fixtures cover allowed ordinary and similarly named paths,
root/nested and backslash-normalized artifacts, case and Unicode compatibility
aliases, all three entry kinds, dotted traversal, absolute/drive/UNC and other
unsafe inputs, fixed-path redaction, deterministic sorting, dishonest expected
results, conditional-schema leakage, stable problem codes, and malformed
ordinal ordering. The corpus now validates 110 unique cases and 269 staged
workspace files. The complete conformance schema, runner, execution,
authority, loader, broker, and backend family passes 201 tests with 23 expected
platform skips; the reference runner has 90% branch-aware coverage.

The Go build tool passes all packages at 78.3% aggregate coverage plus vet and
trimpath compilation. A real forced dry plan validates and selects all 5,070
packages. Ten parity-reporter, seven capability-taxonomy, five
Haskell-capability, and 46 OCaml-lock tests pass with two expected skips. Ruff
error-class lint and formatting, production Bandit, strict JSON, collision
reporting, the 459-owner/638-edge acyclic state graph, and diff checks pass.
Strict MyPy reports the same eight pre-existing source findings as exact main
and no added finding.

Before publication, `main` advanced through two Devanagari curriculum commits
in existing roots and the existing Rust HTML parser to exact revision
`1d5eb7d570ceb011b0d6bd66a4be8e32cac5b395`. None changes a package identity
or overlaps the neutral corpus surfaces. The refreshed schema-3
inventory remains 15 established lanes, 1,370 identities, 4,563 slots, 175
high-consensus identities with 276 gaps, 907 singletons with 12,698 gaps, 718
Rust singletons, zero collisions, and zero unknown buckets; OCaml remains
correctly emerging with no packages. Nine open PRs still have no exact
changed-file overlap, and the target branch and prior PR remain absent.

### Post-#12598 refresh and Python tracked-artifact selection

Final head `bc18656cbf4375f418a1a75f8150391445be6498` completed all 29
reported checks with 23 successes, six expected skips, and no failures or
pending work. GitHub reported PR #12598 clean and mergeable; the loop enabled
squash auto-merge, and GitHub merged it as
`d960dabfe5e07a4d0d7073dada2fea00c05de426` at 2026-08-24T17:55:43Z
without a manual merge command.

The collision-checked schema-3 inventory on that exact merge remains 15
established lanes, 1,370 identities, and 4,563 present slots: 175
high-consensus identities have 276 gaps, 122 identities in five to nine lanes
have 926 gaps, 166 identities in two to four lanes have 2,087 gaps, and 907
singletons have 12,698 gaps. There are 718 Rust singletons, zero canonical
collisions, and zero unknown buckets. OCaml remains correctly emerging with no
packages. Exact tree comparison from the prior inventory revision found no
identity, slot, directory-spelling, marker, or unknown-bucket change and no
newly unowned portable package.

Before selection, the loop decomposed both closed validation-corpus umbrellas
into independently reviewable C#, Elixir, Haskell, Lua, Perl, Python, Ruby,
Rust, Swift, and TypeScript children. This prevents an umbrella from being
mistaken for native engine coverage and gives every extant non-Go consumer an
explicit owner. The completion umbrellas remain selection-blocked tracking
nodes until their ten engine children merge. The Go tracked-artifact
Git-index/process boundary stays separately selection-blocked and outside the
process-free oracle.

The dependency/leverage pass selects
`build-tool-python-tracked-artifact-validation-conformance` on branch
`codex/build-tool-python-tracked-artifact-validation-conformance` from exact
base `d960dabfe5e07a4d0d7073dada2fea00c05de426`. It is one engine and one
closed check, consumes all four neutral cases without Git or host-filesystem
authority, and is the smallest newly ready build-tool parity child. Eleven live
open PRs and 35 non-main remote heads have zero exact candidate-surface
overlap; the target branch is absent. The OCaml process-free core remains
dependency-ready but collision-unsafe while PRs #12149 and #12162 touch the Go
validator and main surfaces.

The implementation adds one pure Python validator over inert snapshot entries,
tests it against every shared tracked-artifact fixture, and documents the
public result shape without widening the reviewed authority boundary. After
four unrelated human-language and HTML-parser commits reached `main`, the two
implementation commits rebased without conflict onto exact revision
`cd0d11cea514288cca2e4a9c786eb98dd336b654`. The collision-checked inventory is
unchanged at 15 established lanes, 1,370 identities, 4,563 slots, zero
collisions, and zero unknown buckets. Both complete Python build-tool fronts
pass from cleared Python 3.13 environments with 423 tests and 90.06% coverage;
the shared corpus, Python 3.10 conformance family, Go oracle and forced dry
repository plan, package-parity/capability/OCaml gates, focused quality and
security checks, build artifacts, dependency check, state DAG, and diff checks
also pass. Full-package strict MyPy retains exactly the same 11 findings in
seven unchanged files as exact `main`; the new validator is strict-MyPy clean.

### Post-#12602 refresh and C# tracked-artifact selection

Python tracked-artifact consumer PR #12602 completed 30 terminal acceptable
checks (24 successes, five expected skips, and one neutral CodeQL gate) and
merged through squash auto-merge as
`b4ffbccc5bef0a436bd5af006f5c9bf0aab799ba` at 2026-08-24T19:02:44Z. The
collision-checked exact-main inventory is unchanged: 15 established lanes,
1,370 identities, 4,563 present slots, 175 high-consensus identities with 276
gaps, 122 identities in five to nine lanes with 926 gaps, 166 identities in
two to four lanes with 2,087 gaps, and 907 singletons with 12,698 gaps. There
are 718 Rust singletons, zero collisions, zero unknown buckets, and no newly
unowned portable package. OCaml remains correctly emerging with no package.

The required ownership audit found that the supported F# build-tool facade was
omitted from both remaining-engine validation decompositions and from three
future rollout notes. The state now registers independent F# orphan-crate and
tracked-artifact consumers, adds both to their completion umbrellas, and names
F# beside C# in the future CLI-parser, Starlark-metering, and validation-oracle
rollouts. This yields 481 explicit owners and 682 dependency edges; facade
reuse no longer stands in for reviewed F# coverage.

The dependency/leverage pass selects
`build-tool-csharp-tracked-artifact-validation-conformance` on branch
`codex/build-tool-csharp-tracked-artifact-validation-conformance`. All eleven
tracked-artifact engine children are dependency-ready, but C# is the best
immediate dependency-shaped slice: its existing bounded `Validator` and
`--validate-build-files` path can consume the four process-free cases without
authority, and the reviewed .NET data/result pattern directly prepares the
separately owned F# facade follow-up without combining engines. Rust has the
largest package footprint and Ruby is the smallest standalone implementation,
but neither unlocks a second explicitly supported lane. Ten live open PRs and
33 non-main remote heads have zero exact overlap on the C#/F# build-tool,
state, or roadmap surfaces; the target branch was absent before the clean
worktree was created.

### C# tracked-artifact implementation

The C# child implements one pure validator over caller-supplied inert snapshot
entries. It slash-normalizes separators, fails closed on the ten reviewed
portable-path defects without echoing hostile text, detects exact
`node_modules` components after NFKC and the full-fold expansions relevant to
an ASCII identity, treats regular, symlink, and reparse kinds identically, and
sorts complete diagnostics by the neutral oracle's canonical keys. The public
entry and diagnostic records serialize directly to the shared result shape;
the implementation adds no Git, filesystem, process, environment, or network
authority and changes no fixture, schema, capability manifest, or dependency.

All four shared fixtures pass through the C# API. Focused hostile-path and
Unicode boundary tests cover every stable problem code, both unsafe-character
branches, scalar-counted 512/513 limits, cross-plane scalar ordering, and the
dotless-i uppercase expansion used by a reserved basename. The canonical .NET
9 BUILD front door passes 26 xUnit tests. The new validator is fully covered at
79 of 79 lines and 50 of 50 branches; complete-package coverage is 49.69% line,
33.79% branch, and 68.33% method. The neutral corpus
validates 110 cases and 269 files, while 269 focused package-parity,
capability-taxonomy, OCaml-lock, and conformance tests pass with 25 expected
platform skips. The Go oracle passes all packages with coverage, vet, and
trimpath compilation. A fresh binary evaluates 45 Starlark BUILD files,
discovers 5,070 packages, preserves the five-entry orphan ledger, and validates
a diff-based dry plan with one changed and two affected packages. NuGet reports no vulnerable direct or
transitive dependency. The source-wide `dotnet format` whitespace gate retains
the exact same 18 findings in unchanged pre-existing ranges as the clean
selected-main baseline; the new ranges and complete diff are clean.

Three independent read-only reviews caught and drove corrections for UTF-16
length counting, UTF-16 ordinal path sorting, and dotless-i reserved-name
uppercase behavior, then found no remaining implementation, authority,
ownership, state-graph, or publication-scope defect. The branch rebased without
conflict over four unrelated human-language and HTML-parser commits to exact
`origin/main` `1d482dc49a5c791b326e0b073cf6469c46d67ef8`. A fresh collision-checked
report at that revision is tree-equivalent to the post-#12602 inventory: 15
established lanes, 1,370 identities, 4,563 slots, 175 high-consensus packages
with 276 gaps, 907 singletons with 12,698 gaps, 718 Rust singletons, zero
collisions, and zero unknown buckets; OCaml remains correctly emerging at zero.

### Post-#12608 refresh and cross-runtime hardening discovery

C# tracked-artifact consumer PR #12608 completed all 29 reported checks with
23 successes, six expected skips, and no failures or pending work. GitHub
reported the branch clean and mergeable; the loop enabled squash auto-merge,
and GitHub merged final head `af6521706441c9171a56aef5994d5b4fbf0318e9`
as `977b44f41124b36a59c6a73978eeb1153eb67c37` at
2026-08-24T20:05:44Z without a manual merge command.

The collision-checked exact-main inventory was regenerated after the subsequent
Tamil- and Telugu-curriculum and HTML-parser merges at
`3a758a6973ea74ff14b6617098e1ad042fa5621c`. It remains 15 established
lanes, 1,370 implementation identities, 4,563 slots, 175 high-consensus
identities with 276 gaps, 122 identities in five to nine lanes with 926 gaps,
166 identities in two to four lanes with 2,087 gaps, and 907 singletons with
12,698 gaps. There are 718 Rust singletons, zero collisions, zero unknown
buckets, no newly unowned portable package, and OCaml remains correctly
emerging at zero packages.

The required cross-runtime audit found that three corrections discovered while
reviewing C# remain pinned only by C#-local tests: Unicode-scalar 512/513 path
lengths, cross-plane Unicode-scalar diagnostic ordering, and full-uppercase
Windows reserved-basename membership for a dotless-i alias. The shared schema
also requires tracked paths to contain one through 512 characters, making the
advertised `EMPTY` and `TOO_LONG` diagnostics unreachable from every valid
fixture. The loop therefore registers
`build-tool-tracked-artifact-unicode-cross-runtime-corpus-hardening` before
selecting another engine child, and makes it a prerequisite of all nine
unmerged consumers. The F# tracked and orphan children also now explicitly
depend on the C# shared engine surfaces they expose rather than relying on an
unstated facade dependency. The Go oracle and future JVM and Dart build tools
also depend explicitly on this repair, while OCaml receives it transitively
through the Go substrate. The reconciled state has 482 owners and 699 edges;
all IDs and dependencies remain unique, present, and acyclic.

The dependency/leverage pass selects
`build-tool-tracked-artifact-unicode-cross-runtime-corpus-hardening` on branch
`codex/build-tool-tracked-artifact-unicode-corpus-hardening`. It is the only
newly exposed portable prerequisite, stays entirely within inert schema,
fixture, and pure-validator data, and directly gates nine extant engine
children, the Go oracle, the future JVM and Dart build tools, and OCaml through
the Go substrate. Nine live open PRs and 34 current non-main remote heads have
zero exact overlap on the selected schema, fixture, reference-runner, Python,
C#, state, or roadmap surfaces; the target branch and prior PR were absent
before the fresh exact-main worktree was created. F# tracked-artifact remains
the preferred next engine child after this shared contract is closed.

The implementation closes the reachability gap without widening the validator:
the schema admits only the exact zero-through-513-scalar fixture envelope while
the pure oracle continues to reject empty and longer-than-512 paths. One new
shared case proves the 512/513 astral boundary, Unicode-scalar diagnostic order
across U+E000 and U+10000, and the U+0131 dotless-i `CONIN$` reserved-basename
alias. Python and C# both consume all five tracked-artifact cases. The complete
neutral schema and runner suites pass 80 tests plus 119 subtests; the 111-case,
269-file corpus validates; Python passes 424 package tests at 90.01% total
coverage and 97% validator coverage; and C# passes all 27 release tests. The
corpus remains process-free and adds no Git, filesystem, environment, process,
or network authority.

Before publication, the branch rebased without conflict over the unrelated
Rust HTML-parser fix in PR #12613 to exact `origin/main`
`6378b33b42714a66eb09964acb2682adfe830738`. A fresh collision-checked report
at that base is unchanged at 15 established lanes, 1,370 identities, 4,563
slots, zero collisions, and zero unknown buckets. All 178 build-tool
conformance tests pass with 23 expected platform skips and 225 subtests; both
canonical Python BUILD fronts pass 424 tests at 90.06% total coverage and the
C# BUILD front passes 27 tests with no compiler warning. Three independent
read-only fixture, security/authority, and state/roadmap reviews found no
remaining implementation or publication-scope defect. F# tracked-artifact
therefore remains the correct next owner after merge.

Ready-for-review PR #12615 opened from validated head
`5c6cdd8df36fe9a490d251e707b379221b48bf72` after a normal first push. GitHub
reports it mergeable while required checks are pending, so auto-merge remains
disabled until every required check reaches a terminal acceptable conclusion.

### Post-#12615 refresh and F# tracked-artifact selection

Cross-runtime tracked-artifact hardening PR #12615 completed all 30 reported
checks with 24 successes, five expected skips, one neutral aggregate, and no
failures or pending work. GitHub reported the branch clean and mergeable; the
loop enabled squash auto-merge, and GitHub merged final head
`bea09c2a425ab559ff60e5b06374dc7b49a84d4e` as
`2f50248a87521d15b069b6daa8b6e1c25d693d85` at 2026-08-24T21:26:43Z
without a manual merge command.

The collision-checked exact-main inventory was regenerated at that merge. It
remains 15 established lanes, 1,370 implementation identities, 4,563 slots,
175 identities in ten to fifteen lanes with 276 gaps, 122 identities in five
to nine lanes with 926 gaps, 166 identities in two to four lanes with 2,087
gaps, and 907 singletons with 12,698 gaps. There are 718 Rust singletons, zero
collisions, zero unknown buckets, no newly unowned portable package or fixture
contract, and OCaml remains correctly emerging at zero packages.

The dependency/leverage pass selects
`build-tool-fsharp-tracked-artifact-validation-conformance` on branch
`codex/build-tool-fsharp-tracked-artifact-validation-conformance`. Its corpus,
C# engine, and Unicode-hardening prerequisites are all merged. This bounded
process-free facade tranche closes the second supported .NET build-tool lane,
tests the F# entry surface independently against all five neutral fixtures,
and adds no Git, filesystem, process, environment, or network authority. Ten
live open PRs have zero exact overlap on the selected F# package, state, or
roadmap surfaces; the target remote branch was absent, and `origin/main`
remained at exact selected base `2f50248a87521d15b069b6daa8b6e1c25d693d85`
before the fresh clean branch was created.

The implementation makes the shared-engine exception reviewable rather than
implicit. The governing contract now requires a language-native adapter and
independent fixture consumption for every shared-engine front door. F# exposes
one pure facade over the reviewed C# tracked-artifact validator, consumes all
five neutral cases through that F# symbol, and routes both existing smoke tests
through the actual F# `main` function. Its test-only root resolver also retains
fixture reachability when .NET artifacts are isolated outside the checkout.
No C# engine, neutral fixture, schema, project, dependency, BUILD, capability,
or production authority changed.

An isolated Release run passes all seven F# tests with 100% line, branch, and
method coverage. Both canonical BUILD commands, a warning-as-error Release
build, Fantomas 7.0.6, and the NuGet vulnerability audit pass. The unchanged C#
engine passes its canonical 27-test Release suite. The neutral corpus validates
111 cases and 269 files; schema and runner suites pass 80 tests plus 119
subtests. Package-parity, capability-taxonomy, Haskell-capability, and OCaml-lock
suites pass 68 tests with two expected Windows symlink skips. The Go build tool
passes all packages with coverage, vet, and trimpath compilation. The real F#
front door validates build files, discovers 199 F# packages, and completes a
clean no-change dry run. Two independent final reviews found no implementation,
fixture, metadata, redaction, security, or authority defect.

The broad conformance family reproduces the five unchanged Windows Python 3.13
execution-snapshot failures already owned by
`build-tool-windows-python313-execution-snapshot-volume-identity`. An isolated
C# downstream run also exposed that its fixture-root finder searches only the
compiled binary ancestry. The canonical in-tree C# suite remains green, and
the loop registers the separate pending owner
`build-tool-csharp-isolated-artifact-fixture-root-resolution` rather than
widening this F# tranche.

The branch rebased without conflict over five non-overlapping HTML-parser,
human-language, and ADJ facts commits to exact `origin/main`
`b4e68a5e90ef124f4bc3ec426f44bc655cc001cc`. A fresh collision-checked report
at that revision is unchanged at 15 established lanes, 1,370 identities, 4,563
slots, zero collisions, and zero unknown buckets.

Ready-for-review PR #12625 opened from clean validated head
`2c9fca023bba1567a57091427adef986c521971c` after a normal first push. The
branch was based on exact `origin/main`
`b4e68a5e90ef124f4bc3ec426f44bc655cc001cc`; the target branch was absent,
and all seven changed paths had zero exact overlap across twelve other live
open PRs. GitHub reports the PR non-draft and mergeable, with required checks
queued or in progress. Auto-merge remains disabled until every required check
is terminal and acceptable.

PR #12625 completed 29 terminal acceptable checks (23 successes and six
expected skips). With GitHub reporting a clean mergeable branch, the loop
enabled squash auto-merge; GitHub merged it as
`015a9da640ab2cd02e74110e31e1916d0671a8f5` at
`2026-08-24T22:32:48Z` without a manual merge command. The exact-main
collision report remains schema 3 with 15 established lanes, 1,370 identities,
4,563 slots, 718 Rust singletons, zero collisions, and zero unknown buckets.
No new package, identity, fixture, or authority owner was discovered.

The next selected owner is
`build-tool-rust-tracked-artifact-validation-conformance`. Both prerequisites
are merged, and this bounded process-free child closes one of the eight
remaining engine consumers. Rust is the widest established lane; the C# child
had temporarily outranked it only because that work unlocked the now-complete
two-step .NET chain. Ten live open PRs have zero exact overlap on the Rust
build-tool, shared spec, state, or roadmap surfaces, and the target remote
branch was absent. The strategic OCaml process-free substrate remains
collision-unsafe while live PRs #12149 and #12162 touch its Go validator and
main surfaces.

The Rust implementation review exposed three separately owned follow-ups before
publication. A trailing slash or backslash creates a final empty component that
the shared Python and C# validators currently accept despite the portable-path
contract; the Rust child rejects both forms directly, while
`build-tool-tracked-artifact-trailing-empty-segment-cross-runtime-hardening`
owns the neutral fixture and reviewed shared-engine repairs. The contract also
needs one explicit Unicode data version across runtimes, now owned by
`build-tool-tracked-artifact-unicode-version-contract`; Rust uses one exactly
pinned Unicode 17.0.0 snapshot for normalization, full default folding, and
reserved-name uppercase rather than mixing Unicode 17 normalization with an
older folding table. Both owners now gate the unfinished native consumers and
the Go, JVM, and Dart implementations.

The canonical Rust `BUILD_windows` recipe also leaves an unignored
`target_isolated/` directory after a real run. The generated validation output
was removed, and `build-tool-rust-windows-isolated-target-artifact-hygiene`
owns the durable build-front fix and repeated-clean-run proof without widening
the process-free validator tranche.

The implemented Rust validator is a pure in-memory consumer of the closed
snapshot schema. It independently loads all five neutral cases and matches the
contract's slash normalization, closed path-error precedence, hostile-input
redaction, Unicode-scalar length and sorting, NFKC plus full default folding,
full-uppercase reserved-name membership, inert entry kinds, and canonical
diagnostic ordering. One exact `oxixml-unicode` 0.1.2 dependency supplies NFC,
NFKC, full folding, and full uppercase from the same Unicode 17.0.0 snapshot;
the dependency is Apache-2.0, has no transitives, and forbids unsafe code.

Focused tests pass 2/2 at 98.98% validator line and 100% function coverage;
Clippy passes all targets with warnings denied. The first GitHub check run
exposed the registered Rust `dist-newstyle` discovery gap on Ubuntu and macOS:
actual logs showed the shared fixture fail after 141 passes because the
generated Haskell decoy was discovered. The minimal repair adds that exact,
case-sensitive component to Rust's existing artifact exclusion. The focused
regression, complete all-target suite, canonical BUILD, and canonical
BUILD_windows front now pass 142 unit tests plus three CLI integrations.
Coverage reports 94.03% discovery lines and 80.44% lines overall. The stable
`build-tool-dist-newstyle-discovery-exclusion-remaining-engines` owner is now
Ruby-only; TypeScript and Swift remain separately owned. Neutral schema and
runner suites pass 80 tests plus 119 subtests, and all 111 corpus cases across 269 files
validate. Parity, capability, Haskell-capability, and OCaml-lock suites pass 68
tests with two expected Windows symlink skips. The Go build tool passes tests,
vet, trimpath compilation, BUILD validation, and a real no-change dry run over
45 Starlark files and 5,070 packages. Cargo audit reports no vulnerability in
57 dependencies, and dependency, credential, diff, capability-manifest, and
production-authority reviews are clean.

The branch rebased without conflict over nine unrelated main commits to exact
`origin/main` `d0d6ab3846ca56175b8d2e66b901d1facf32202a`. A fresh schema-3
collision report at that revision remains 15 established lanes, 1,370
identities, 4,563 slots, 718 Rust singletons, zero collisions, and zero unknown
buckets. The state graph contains 486 unique owners with complete dependencies
and no cycle. The final 467,724,669-byte `target_isolated/` validation artifact was
removed after the exact Windows front ran; its durable cleanliness repair stays
with the separately registered owner.

Final fixture/inventory review caught one subtle NFC issue before publication:
the Unicode library's fast predicate is conservative for `NFC_QC=Maybe`
scalars. The validator now uses that predicate only as a fast path and performs
an exact normalization comparison whenever it is inconclusive. A focused
q-plus-combining-grave regression proves an already-normalized path remains
accepted while the existing decomposed e-plus-acute case remains `NON_NFC`.
Three independent read-only reviews verified the correction and found no
remaining implementation, ownership, security, authority, or publication
defect.

Ready-for-review PR #12633 opened from clean validated head
`b445baa5f43991f96db9540b2fb86b50f8057365` after a normal first push and one
focused metadata correction on the same branch. Immediately before
publication, the branch was based on exact `origin/main`
`d0d6ab3846ca56175b8d2e66b901d1facf32202a`, the target remote branch was
absent, and all seven changed paths had zero exact overlap across six other
live open PRs. GitHub reports the PR non-draft and mergeable while required
checks are queued or in progress. Auto-merge remains disabled until every
required check is terminal and acceptable.

The first CI run reached 18 successes and six expected skips, but Ubuntu and
macOS build jobs plus their aggregate gate failed on the exact registered Rust
`dist-newstyle` fixture. After inspecting the actual job logs, the branch added
only the exact generated-artifact exclusion, its changelog entry, and README
clarification. Commit `f0754ce362bae712b33380595394d2337b436450` was pushed
normally to the same PR after the focused, full-suite, Clippy, both BUILD-front,
coverage, diff, artifact-cleanliness, and security validations passed. The PR
remains mergeable while replacement checks run; auto-merge stays disabled until
all required checks are terminal and acceptable.

PR #12633 ultimately completed all 29 reported checks: 23 successes and six
expected skips. GitHub reported the branch clean and mergeable, so the loop
enabled squash auto-merge; GitHub merged it as
`293f79290187d2dd3b1a993f9b11df2198cb5787` at
`2026-08-25T00:13:59Z` without a manual merge command. The mandatory
post-merge collision report remains schema 3 with 15 established lanes, 1,370
implementation identities, 4,563 slots, 718 Rust singletons, zero collisions,
and zero unknown buckets. OCaml remains correctly emerging at zero packages,
and no package identity, slot, or newly unowned portable gap appeared.

The refreshed 486-owner state graph is complete and acyclic. Its rockspec UTF-8
umbrella previously depended only on the shared corpus despite naming three
unfinished engine children; explicit Ruby, Haskell, and Elixir dependencies now
prevent that umbrella from becoming ready early. This semantic correction
raises the edge count from 731 to 734 without adding an owner. A leverage pass
selects
`build-tool-tracked-artifact-trailing-empty-segment-cross-runtime-hardening`:
all five prerequisites are merged, it directly gates 11 owners and 20
unfinished descendants, and it closes a reproduced correctness defect in a
smaller dependency surface than the equally leveraged Unicode-version sibling.
Seven live open PRs have zero exact overlap on its neutral fixture, oracle,
Python, C#, F#, state, or roadmap surfaces. Before the first push, `main`
advanced through three non-overlapping Vault, Java-spec, and Tamil commits. The
branch rebased without conflict to exact `origin/main`
`ee13dca795810c618dcbfce660dd707838a8e5ef`; no force push was involved because
the target branch was still absent. The refreshed collision report contains
1,371 identities, 4,564 slots, 908 singletons with 12,712 missing slots, and 719
Rust singletons, while high-, middle-, and low-consensus bands remain unchanged
and collisions and unknown buckets remain zero. The new deterministic
`vault-import-otpauth` singleton is classified by extending the existing
pending `vault-external-import-portable-conformance` owner with VLT-PM49 URI
fixtures, rather than creating a duplicate umbrella.

The implementation makes the existing portable-path rule explicit: a trailing
slash or backslash creates an invalid empty component after separator
normalization. The language-neutral invalid fixture now proves both forms while
preserving redacted diagnostics. The process-free oracle, Python engine, and
shared C# engine inspect every normalized component rather than only internal
double separators; absolute and drive-qualified precedence is unchanged. The
F# facade independently consumes the expanded fixture without adding native
Git, filesystem, process, environment, or network authority.

Red phases failed exactly the expanded fixture and both new direct cases in the
neutral oracle and C# engine. Green validation passes 80 neutral tests plus 121
subtests and validates all 111 corpus cases across 269 files. Python passes 426
tests at 90.06% total and 97% validator coverage. C# passes 29 tests, with the
changed predicate exercised 130 times and both branch outcomes covered; F#
passes all seven facade tests. Warning-free .NET builds, the canonical Python
`uv` front, NuGet vulnerability audits, Bandit, scoped Ruff, collision,
capability-taxonomy, Haskell-capability, OCaml-lock, dependency-manifest,
credential, diff, production-authority, and state-DAG checks pass. Broader
formatter, Ruff, MyPy, and `dotnet format` probes reproduce only pre-existing
untouched baseline findings; `dotnet format` does not support F#.

Ready-for-review PR #12644 opened from clean independently reviewed head
`e317ff99b3e53f3d32325136a677da392242e50c` after a normal first push. The
branch was based on exact `origin/main`
`ee13dca795810c618dcbfce660dd707838a8e5ef`; later package-neutral main commits
have zero changed-path overlap and leave GitHub mergeable, so no further rebase
or force push is appropriate. Immediately before publication, six live PRs had
zero exact overlap across all 19 changed paths, and the target branch and PR
were absent. GitHub reports the PR non-draft and mergeable while required checks
are queued or in progress. Auto-merge remains disabled until every required
check is terminal and acceptable.

PR #12644 completed all 30 reported checks: 24 successes, five expected skips,
and one neutral aggregate. GitHub reported the branch clean and mergeable, so
the loop enabled squash auto-merge; GitHub merged it as
`6d9882fbff004af242851589d34dc78bee59b654` at
`2026-08-25T01:04:14Z` without a manual merge command. Main then advanced by
one non-overlapping WASM SIMD change to exact revision
`530938671414502ae74e3506527cc5081a19ddac`. The mandatory post-merge report is
unchanged at schema 3, 15 established lanes, 1,371 implementation identities,
4,564 slots, 908 singletons with 12,712 missing slots, 719 Rust singletons,
zero collisions, and zero unknown buckets. OCaml remains correctly emerging at
zero packages, and no new identity, slot, or eligible unowned portable gap was
discovered.

The state dependency audit found that both shared tracked-artifact hardening
owners name and validate the independently reviewed F# facade but depended
only on the shared C# engine. Their dependency lists now explicitly include
`build-tool-fsharp-tracked-artifact-validation-conformance`; the complete
486-owner graph remains acyclic and its edge count rises from 734 to 736.

The next selected owner is
`build-tool-tracked-artifact-unicode-version-contract`. All five explicit
prerequisites are merged, it directly gates 11 owners and 20 unfinished
descendants, and it closes the remaining cross-runtime semantic risk before
the Go, JVM, Dart, and seven unfinished native consumers proceed. Seven live
open PRs have zero exact overlap on the neutral, Python, C#, F#, state, or
roadmap surfaces, and the target remote branch was absent. Rust already pins
one Unicode 17.0.0 snapshot; the selected process-free tranche must replace
Python and .NET host-table drift with one reviewed generated Unicode 17 data
substrate for NFC, NFKC, full default folding, and full uppercase while
preserving the no-filesystem, no-process, no-environment, and no-network
authority boundary. The strategic OCaml substrate remains collision-unsafe
while live PRs #12149 and #12162 touch its Go validator and main surfaces.

The selected Unicode tranche now pins five official Unicode 17.0.0 data files
by exact byte count and SHA-256 and generates one source-embedded policy for
the neutral oracle, Python, C#, and the F# facade; Rust continues to use its
reviewed Unicode 17 tables. The generator refuses redirects, exact-origin or
final-URL drift, oversized and truncated bodies, hash drift, stale generated
sources, and stale distributable license notices. Full official normalization,
case-fold, and uppercase checks pass, as do the Todhri and outlined-letter
version sentinels. Python, C#, F#, and Rust package suites, coverage, lint,
static analysis, package audits, neutral conformance, Go downstream validation,
build-file validation, package-parity, capability, Haskell, and OCaml gates all
pass. Python wheel/sdist and C#/F# publish/NuGet artifacts declare mixed MIT and
Unicode-3.0 licensing and contain the exact full notice. Two independent
security reviews found redirect/body-cap and license-distribution defects; both
were repaired, and the final re-review passed. After conflict-free rebases over
eight non-overlapping commits, the collision-checked report at exact main
`88ff366eb5bacc5ad112beb760ec6f6801bd905b` remains 15 lanes, 1,371 identities,
4,564 slots, 908 singletons with 12,712 gaps, 719 Rust singletons, zero
collisions, and zero unknown buckets.

Ready-for-review PR #12659 now carries the bounded Unicode-version contract
from validated head `711c6ffe91d76e62be029fa0fdf2fd3cd0ec781b`. Immediately before
publication the branch was based on exact main
`88ff366eb5bacc5ad112beb760ec6f6801bd905b`; the target remote branch was absent,
and its 42 changed paths had zero exact overlap across seven other live open
PRs. GitHub reports the PR non-draft and mergeable while required checks are
queued. Auto-merge remains disabled until every required check is terminal and
acceptable.

PR #12659 completed all 30 reported checks: 24 successes, five expected skips,
and one neutral aggregate. GitHub reported the branch clean and mergeable, so
the loop enabled squash auto-merge; GitHub merged it as
`9cfe95f38364e3bc1609548c7b85b57c6b0fc72d` at
`2026-08-25T02:42:16Z` without a manual merge command. The mandatory
post-merge collision report remains schema 3 with 15 established lanes, 1,371
implementation identities, 4,564 slots, 175 high-consensus packages with 276
gaps, 908 singleton packages with 12,712 gaps, 719 Rust singletons, zero
collisions, and zero unknown buckets. OCaml remains correctly emerging at zero
packages, and exact comparison with the stored inventory finds no added or
removed identity or slot and no newly unowned portable gap.

The post-merge dependency audit found that Ruby, Swift, and TypeScript cannot
yet take their tracked-artifact children through the required complete engine
suites: each retains a separately registered `dist-newstyle` discovery
failure. Their tracked-artifact owners now explicitly depend on the Ruby,
Swift, and TypeScript discovery repairs, respectively. The complete 486-owner
graph remains dependency-complete and acyclic while its edge count rises from
736 to 739. Elixir, Haskell, Lua, and Perl remain immediately ready tracked-
artifact consumers; Ruby, Swift, and TypeScript correctly become two-step
chains rather than nominally ready children with known red validation.

The next selected owner is
`build-tool-typescript-dist-newstyle-discovery-exclusion`. Its language-
identity prerequisite is merged, the exact shared fixture already reproduces
the one generated Cabal-directory false package, and the bounded engine-only
repair unlocks the widest affected tracked-artifact lane at 446 TypeScript
package directories. Eight live open PRs have zero exact overlap on the
TypeScript build-tool, state, or roadmap surfaces, and the target remote branch
was absent before the fresh exact-main worktree was created. The strategic
OCaml process-free substrate remains collision-unsafe while PRs #12149 and
#12162 own exact Go validator and main paths.

The TypeScript repair now has red-to-green evidence at exact rebased main
`a6b3c9d3b95e4d310b57a3b194c4b035b58b8bfc`: the focused regression initially
failed in all three expected assertions, then all 284 TypeScript build-tool
tests passed across 12 files. Overall coverage is 89.43% statements, 82.06%
branches, 92.75% functions, and 89.36% lines; `discovery.ts` is 93.68%, 89.23%,
100%, and 93.4%, respectively. The neutral corpus validates 111 cases and 269
files; the focused package-parity, capability, Haskell, and OCaml suite passes
66 tests plus 806 subtests with two expected Windows symlink skips. The Go
build tool passes its full test, vet, and trimpath build gates, and a real
forced TypeScript dry validation evaluates 45 Starlark BUILD files and reports
476 of 476 packages `WOULD-BUILD`, with the orphan-crate check clean. The
schema-3 collision report and the complete 486-owner, 739-edge acyclic state
graph remain unchanged. Production dependency audit reports zero
vulnerabilities; the full audit exposes one pre-existing development-only
`nanoid` advisory below Vitest, while this branch changes neither dependency
metadata nor runtime authority.

Two read-only security reviews pass the exact-component boundary, credential,
dependency, and production-authority checks. The branch rebased twice without
conflict over four non-overlapping WASM, HTML-parser, Spanish, and human-
language debt-ceiling commits; after the final rebase, all 38 focused discovery
tests pass, the collision report is unchanged, and the 486-owner, 739-edge
graph remains complete and acyclic.

PR #12669 then completed all 30 reported checks: 24 successes, five expected
skips, and one neutral aggregate. GitHub reported the branch clean and
mergeable, so the loop enabled squash auto-merge; GitHub merged it as
`67391c12334b0193f67fba16864dbcaf8190d647` at
`2026-08-25T03:36:51Z` without a manual merge command. The mandatory
post-merge collision report remains schema 3 with 15 established lanes, 1,371
implementation identities, 4,564 slots, 175 high-consensus packages with 276
gaps, 908 singleton packages with 12,712 gaps, 719 Rust singletons, zero
collisions, and zero unknown buckets. OCaml remains correctly emerging at zero
packages, and exact comparison with the preceding inventory finds no identity
or slot additions or removals and no newly unowned portable gap.

The post-merge audit registered the development-only TypeScript `nanoid`
advisory chain as its own pending, non-blocking security owner. Production
dependencies remain clean. It also added four missing semantic prerequisites:
the TypeScript, Ruby, and Swift orphan-crate consumers now depend on their
respective discovery exclusions, and Swift Windows absolute-file-option work
depends on Swift discovery exclusion. The complete graph therefore contains
487 owners and 744 dependency edges and remains acyclic.

The next selected owner is
`build-tool-swift-dist-newstyle-discovery-exclusion`. It is the highest-
leverage collision-free tranche: the bounded exact-component repair unlocks
three direct owners and five unfinished descendants, versus two and four for
Ruby. Seven live open PRs have zero exact path overlap, the remote target
branch was absent, and the fresh worktree starts at the merged PR #12669
revision. OCaml remains a stronger strategic chain but collision-unsafe while
PRs #12149 and #12162 own its exact Go validator and entry-point surfaces.
Before publication, the branch rebased twice without conflict over five non-
overlapping Rust WASM, ADJ, Tamil, Vault, and HTML-parser commits to
`69bf8c4a87f8a0de0a14e3adcc5be77b8ceffeb6`.

That final rebase adds one inventory identity: the Rust-only
`vault-webauthn-ctap2-hid` native adapter. The refreshed collision report now
contains 1,372 implementation identities, 4,565 slots, 909 singleton packages
with 12,726 gaps, and 720 Rust singletons; all other bands are unchanged, with
zero collisions or unknown buckets and OCaml still emerging at zero packages.
The adapter's physical FIDO2 enumeration, USB HID I/O through native `hidapi`,
worker thread, timeout, and nonempty FFI capability make it a concrete native-
runtime exception rather than a portable all-language target. A new selection-
blocked owner now records the required capability, dependency-provenance,
device policy, timeout-retention, error, zeroization, and hardware-evidence
review, leaving no newly unowned eligible portable gap.

The Swift repair has red-to-green evidence on that exact base: its focused
regression first failed on the absent registry member and emitted Cabal decoy,
then all 27 Swift build-tool tests passed. `Discovery.swift` retains 91.33%
line coverage, `Hasher.swift` reaches 51.49%, and overall production line
coverage is 47.24%. Focused cases prove exact lower-case exclusion, case- and
near-name preservation, and source-hashing reuse. The neutral corpus validates
111 cases and 269 files; 148 package-parity, capability, Haskell, OCaml, and
conformance tests pass with two expected Windows symlink skips. The Go build
tool passes its full test, vet, and trimpath build gates, and a forced Swift dry
validation evaluates 45 Starlark BUILD files and reports all 165 Swift packages
`WOULD-BUILD` with the orphan-crate check clean.

Both package build fronts pass repeated clean-status runs, package metadata and
the normal release build pass, the collision report remains clean, and the
488-owner, 745-edge graph remains complete and acyclic. The warnings-as-errors
release variant reproduces only three pre-existing warnings in untouched
`BuildTool.swift` and `Executor.swift`; Swift formatting similarly has no
checked-in configuration and reports the package's existing style baseline.
There are no external Swift dependencies, capability or runtime-authority
changes, credential paths, or secret-like diff hits. An independent security
review found no production blocker after its requested explicit case and
Hasher assertions were added.

PR #12678 completed all 32 reported checks: 26 successes, five expected skips,
and one neutral aggregate. GitHub reported the branch clean and mergeable, so
the loop enabled squash auto-merge; GitHub merged it as
`834b6a01530906f64f218d41edf233e5537a7c8d` at
`2026-08-25T04:47:50Z` without a manual merge command. The post-merge
collision report at that exact revision remains schema 3 with 15 established
lanes, 1,372 implementation identities, 4,565 slots, 175 high-consensus
packages with 276 gaps, 909 singleton packages with 12,726 gaps, 720 Rust
singletons, zero collisions, and zero unknown buckets. OCaml remains correctly
emerging at zero packages. Current `origin/main` then advanced by one
package-neutral HTML-parser commit to
`42ae3e50152fc5373451cd9eee257a789f279782`, then by package-neutral Spanish
curriculum PR #12679, and finally by Rust HTML-parser PR #12680 to
`59b34f88ca0e1b4882c966374bb4bd53a2b9defa`; regenerating the report at each
revision changes no inventory count and exposes no eligible unowned gap.

The reconciled graph contains 488 owners and 745 dependency edges with no
duplicate IDs, missing prerequisites, or cycles. The Vault HID native-authority
exception and TypeScript `nanoid` development advisory retain explicit pending
owners. Completing Swift discovery unlocks its tracked-artifact, orphan-crate,
and Windows absolute-file owners, but the ready Ruby discovery repair has the
highest immediate build-tool leverage: it unlocks two direct consumers and
four unfinished descendants. The next selected owner is therefore
`build-tool-dist-newstyle-discovery-exclusion-remaining-engines`, now a bounded
Ruby-only exact-component repair. Six live open PRs have no exact overlap with
its Ruby build-tool, state, or roadmap surfaces. OCaml remains strategically
important but collision-unsafe while PRs #12149 and #12162 own its exact Go
validator and entry-point surfaces.

The Ruby repair has direct red-to-green evidence on that selected base. The
shared registry fixture and a focused exact-component regression first emitted
the Cabal decoy, while case and near-name controls already passed. Adding the
single frozen registry member closes both failures. Ruby 3.4.9 with Bundler
2.6.9 now passes all 310 runs and 635 assertions with one expected skip, 89.43%
line coverage, and 72.55% branch coverage; repeated canonical `BUILD` runs
reproduce the result. The process-free corpus validates 111 cases and 269
files, and 148 focused conformance, package-parity, capability, Haskell, and
OCaml-lock tests pass with two expected Windows symlink skips.

The Go oracle passes its full test, vet, and trimpath build gates. A real forced
Ruby dry validation evaluates 45 Starlark BUILD files, discovers 305 packages,
reports all 305 as `WOULD-BUILD`, and leaves the orphan-crate check clean with
its five reviewed exemptions. Syntax and dependency-current checks pass; a
fresh ruby-advisory-db at `eca0eccee391` reports no vulnerabilities. Direct
StandardRB lint reproduces the clean-main file's pre-existing broad baseline
and adds no offense on a changed production or test line. The collision report
and complete acyclic 488-owner/745-edge graph remain clean, and the live
open-PR audit has zero exact overlap with the six-path diff. An independent
exact-head security review found no new filesystem, process, network,
environment, credential, diagnostic, dependency, or execution-authority
surface and no publication blocker. After a conflict-free rebase over Spanish
curriculum PR #12679, the exact same 310-run Ruby suite, syntax checks, Go tests
and vet, and collision report all pass again. A second conflict-free rebase over
Rust HTML-parser PR #12680 again reproduces the complete Ruby suite and
collision report. Independent final review found no implementation, test,
graph, security, dependency, or scope defect and requested only this applied
live-main metadata refresh. A final exact-head confirmation then verified the
clean six-path diff, current merge base, complete acyclic graph, and count-free
overlap wording and found no publication blocker.

Ready-for-review PR #12683 opened from validated head
`f07299a1c88655544c99aabbbddb5a309e241e24` after a normal first push. At
publication, remote main, local `origin/main`, and the branch merge base all
equal `59b34f88ca0e1b4882c966374bb4bd53a2b9defa`; the six-path diff has zero
exact overlap with the live open-PR set. Auto-merge remains disabled until all
required checks are terminal and acceptable and GitHub reports no merge
conflict.

PR #12683 subsequently completed all 30 reported checks: 24 successes, five
expected skips, and one neutral CodeQL aggregate. GitHub reported the branch
clean and mergeable, so the loop enabled squash auto-merge; GitHub merged it as
`8bda602ea0546ae6c078faa1f833f835d43fb233` at
`2026-08-25T05:38:14Z` without a manual merge command. The required post-merge
collision report at that exact revision is unchanged: schema 3, 15 established
lanes, 1,372 implementation identities, 4,565 package slots, 175 high-consensus
packages with 276 gaps, 909 singleton packages with 12,726 gaps, 720 Rust
singletons, and zero canonical collisions or unknown language buckets. OCaml
remains correctly emerging at zero packages. Exact comparison with the stored
`59b34f88ca0e1b4882c966374bb4bd53a2b9defa` inventory finds no identity or slot
addition or removal.

The post-merge source audit found one additional owned gap before selection.
The language-neutral hashing contract requires generated, dependency, VCS,
cache, and temporary directories to be excluded, while both Ruby hasher
collection paths recursively visit all descendant files without pruning any
directory component. The new pending
`build-tool-ruby-generated-directory-hashing-exclusion` owner records that
bounded repair and depends on the now-merged Ruby discovery owner. This raises
the complete acyclic graph to 489 owners and 746 edges; there are no duplicate
IDs, missing prerequisites, cycles, or eligible unowned gaps.

The selected next owner is
`build-tool-csharp-orphan-crate-validation-conformance` on exact base
`8bda602ea0546ae6c078faa1f833f835d43fb233`. Its neutral corpus prerequisite is
merged. This process-free one-engine consumer has the highest immediate
build-tool leverage because it directly unlocks the separately reviewed F#
facade and contributes to the orphan-validation completion umbrella. Live open
PRs and the target remote branch have zero exact overlap on the C# build-tool,
state, or roadmap surfaces. The strategically larger OCaml process-free core
remains collision-unsafe while existing PRs own the exact Go validator and
entry-point files it must change.

Before the first push, `origin/main` advanced without path conflict through ADJ
facts PR #12684, Go build-tool PR #12687, Persian ductus PR #12688, vault-auth
PR #12685, Rust html-parser PR #12689, and ADJ loop-state PR #12691 to
`48c0096df92b73c66ba116e9f7023de2e8701e39`; the selected branch rebased there
normally. The refreshed schema-3 collision report remains unchanged in
topology and counts. PR #12687 also exposes a new portable build-tool contract
gap: Go alone now parses `# needs-toolchain: NAME` BUILD comments and applies
them to affected-package CI detection. The state therefore registers a neutral
toolchain-detection corpus/Go-normalization owner and a selection-blocked
remaining-engines umbrella before publication. The corpus must close directive
syntax, canonical names, duplicates, invalid values, platform BUILD precedence,
affected-only behavior, forced-full behavior, and deterministic results before
the umbrella is decomposed into engine children. The complete graph is now 491
owners and 749 edges, and no eligible behavior from that main advance remains
unowned.

Validated implementation head `4dd44395b2a508c51045d10ea8d4ef8342f76819`
exposes one process-free orphan-crate snapshot
validator and exactly consumes all four neutral fixtures. It covers direct and
ancestor BUILD ownership, closer empty BUILD precedence, fixed platform
filename ranking, exact artifact exclusions, invalid and stale exemptions,
pending counting, hostile-path redaction, deterministic Unicode ordering, and
Python-compatible blank reasons including the U+001C through U+001F controls.
The focused family passes 13 tests and both repeated canonical C# BUILD fronts
pass all 43 tests; Release coverage is 94.6% line, 42.89% branch, and 76.54%
method, with a clean warnings-as-errors build and publish. The F# downstream
facade passes seven tests. The neutral corpus validates 111 cases and 269 files,
and 146 focused repository tests plus 928 subtests pass with two expected
Windows skips. The rebased Go oracle passes all packages, vet, and trimpath
compilation; a real forced C# plan evaluates 45 Starlark BUILD files, discovers
200 packages, preserves the five reviewed orphan exemptions, and reports all
200 as `WOULD-BUILD`. NuGet vulnerability, collision, strict-JSON, acyclic
491-owner/749-edge state, diff, dependency-scope, and production-authority
checks pass. Source-wide `dotnet format` reproduces only the exact pre-existing
whitespace baseline outside changed lines. Independent implementation,
fixture/inventory, state/roadmap, and security reviews found no remaining
publication blocker after the identified Python-whitespace, closest-empty
filename-rank, Go-oracle dependency, generated-artifact, and rebase gaps were
closed.

Ready-for-review PR #12700 opened from clean validated head
`29b203f905a4982dfcdf60dee340b273ca635643` after a normal first push. At
publication, `origin/main` and the branch merge base both equaled
`48c0096df92b73c66ba116e9f7023de2e8701e39`; the six-path diff had zero exact
overlap with live open PRs. GitHub reports the PR non-draft and mergeable,
blocked only by queued checks, so auto-merge remains disabled until every
required check is terminal and acceptable and GitHub reports no conflict.

PR #12700 subsequently completed all 29 reported checks: 23 successes and six
expected skips, with no failures or pending work. GitHub reported the branch
clean and mergeable, so the loop enabled squash auto-merge; GitHub merged it as
`c68b31d8daa9b82e2b59b139364dd990d8244a27` at
`2026-08-25T06:55:04Z` without a manual merge command. The exact merged head is
`924a1bb297a00f97bc94e48d2ef41cbee8c968d9`.

The mandatory post-merge collision inventory on current `origin/main`
`fc01df1ae0b68af15f7a493d610f348c32edf60f` is structurally unchanged:
schema 3, 15 established lanes, 1,372 implementation identities, 4,565 slots,
175 high-consensus packages with 276 gaps, 122 five-to-nine-lane packages with
926 gaps, 166 two-to-four-lane packages with 2,087 gaps, 909 singletons with
12,726 gaps, 720 Rust singletons, zero collisions, and zero unknown buckets.
OCaml remains emerging at zero packages. Exact comparison with the stored
`671d62d2c1c536da2778c03347e030fae77f6c33` inventory finds no identity or
slot addition or removal.

The source and contract refresh assigns merged PR #12698's 274 successful Rust
`i8x16.shuffle` assertions across 12 modules to the existing
`wasm-conformance-portable-core-conformance` owner; it is expanded corpus
evidence, not a new package or owner. Merged PR #12686 exposes one genuinely
unowned native correctness boundary: `storage-fs` explicitly admits that a
fresh backend can reuse an issued revision after deletion of the highest
surviving record and that two instances sharing one root are not coordinated,
while STR01 and the README still overclaim restart-safe monotonicity. The new
selection-blocked
`storage-fs-persistent-revision-floor-native-authority-review` owner covers a
truthful contract, durable floor or epoch design, crash and rollback behavior,
cross-process locking, no-follow containment, atomic publication and fsync,
and restart/two-instance tests. The complete state is now an acyclic 492-owner,
749-edge graph with no eligible unowned gap.

The next selected owner is
`build-tool-fsharp-orphan-crate-validation-conformance` on exact base
`fc01df1ae0b68af15f7a493d610f348c32edf60f`. PR #12700 merged its last unmet
dependency, so this bounded process-free F# facade now advances the reviewed
shared .NET validator into another supported implementation lane and
contributes the F# edge to the orphan-validation completion umbrella. Its
target branch is absent and its exact F# program, tests, README, changelog,
state, and roadmap surfaces have zero overlap with live PRs and remote heads.
The extra-CI-toolchain corpus has greater graph leverage, but its current owner
also requires canonical Go normalization on `main.go`, which overlaps live PRs
#12149 and #12162; OCaml remains collision-unsafe on the same surfaces.

The selected F# slice now has validated implementation head
`128832d6f52459f2e8553fa096a643c9e73aedc2`. The specification
requires each established shared-engine front door to expose a language-native
orphan-snapshot adapter and consume every registered neutral fixture without
gaining discovery authority. A focused regression first failed because the F#
symbol did not exist; the no-inline facade now drives all four clean, unlisted,
invalid-exemption, and stale-exemption fixtures and compares exact diagnostics
and pending counts. The formatted Release suite passes 11 tests, the shared C#
downstream passes 43, warnings-as-errors build and publish pass, both canonical
F# BUILD commands pass twice, and the F# assembly records 100% line, branch,
and method coverage while the exercised shared engine records 92.39% line
coverage. The 111-case/269-file neutral corpus, 146 focused repository tests
plus 928 subtests with two expected Windows skips, Go test/vet/trimpath build,
and a real 199-of-199 F# forced dry plan all pass. NuGet vulnerability,
Fantomas 7.0.6, collision, state-DAG, diff, credential, artifact, dependency,
and authority checks are clean. Three independent audits confirm exact fixture
consumption, zero live-PR or remote-head path overlap, and no publication
blocker after adding the missing README paragraph break.

Ready-for-review PR #12715 opened from clean validated head
`aa6a49c93aeb533fe75846ef74ad6d8512147e6d` after a normal first push. At
publication, `origin/main` and the branch merge base both equal
`fc01df1ae0b68af15f7a493d610f348c32edf60f`; the seven-path diff has zero
exact overlap with live open PRs or remote heads. GitHub reports the PR
non-draft and mergeable, blocked only by queued checks, so auto-merge remains
disabled until every required check is terminal and acceptable and GitHub
reports no conflict.

### Post-#12715 refresh and Rust orphan-validator selection

PR #12715 subsequently completed all 29 reported checks: 23 successes and six
expected skips, with no failures or pending work. GitHub reported the branch
clean and mergeable, so the loop enabled squash auto-merge; GitHub merged it as
`07aa6098a25018e672cf4538be630aea52b30330` at
`2026-08-25T08:06:29Z` without a manual merge command. The exact merged head is
`5c716a9283e604fa1f922e6f95075fd63b166abd`.

The mandatory post-merge refresh followed every intervening main commit through
`c1659cc9a104d3406d4e4d821e468511ee793701`. The collision-checked schema-3
inventory now contains 15 established lanes, 1,373 implementation identities,
4,566 package slots, 175 high-consensus packages with 276 missing slots, 122
five-to-nine-lane packages with 926 missing slots, 166 two-to-four-lane packages
with 2,087 missing slots, 910 singletons with 12,740 missing slots, and 721 Rust
singletons. Canonical collisions and unknown language buckets remain zero;
OCaml remains correctly emerging at zero packages.

The source audit registered all newly exposed portable work before selection.
Merged Mermaid XY configuration and axis changes now have an explicit
`mermaid-xy-chart-portable-conformance` contract owner followed by a
selection-blocked established-lane rollout umbrella; the existing sequence
diagram owner is intentionally too narrow. Merged PR #12720 added the pure Rust
`java-to-semantic-ir` singleton, now owned by a language-neutral JV02 fixture
contract followed by a selection-blocked established-lane rollout. HTML-parser,
ALGOL, ADJ, human-language, and vault changes remain inside their existing
portable, curriculum, or native classifications. The resulting state contains
496 unique owners and 751 complete dependency edges, with no duplicate IDs or
missing prerequisites.

The selected next owner is
`build-tool-rust-orphan-crate-validation-conformance` on exact base
`192768b4190c852754b01c531025b8064109e709`. Its neutral corpus prerequisite is
merged. Rust is the widest established lane, and its build tool already pins one
Unicode 17 normalization, full-folding, and full-uppercase substrate required by
the orphan exemption contract, so this process-free consumer is bounded and
dependency-ready without a runtime Unicode-version expansion. Eight live open
PRs have zero exact overlap with the Rust build-tool, state, or roadmap surfaces,
and the target remote branch was absent. The higher-descendant extra-CI-toolchain
corpus and OCaml process-free substrate remain collision-unsafe while live PRs
#12149 and #12162 own their required Go entry-point and validator surfaces.

The implemented Rust consumer exposes one pure in-memory orphan snapshot
validator without adding checkout authority. It consumes all four neutral
fixtures and covers runnable ancestor ownership, nearer empty BUILD
non-masking, fixed closest-empty filename rank, exact artifact exclusions,
invalid and stale exemption precedence, pending counts, hostile-path
redaction, NFC/full-casefold duplicate identity, deterministic Unicode-scalar
ordering, Python-compatible ASCII JSON diagnostic keys, and Python-compatible
U+001C-through-U+001F blank reasons. An independent conformance audit exposed
the neutral Python oracle's non-ASCII escaping as a second-order sort key; a
test-first Unicode stale-entry regression reproduced the mismatch before the
Rust implementation was corrected, including supplementary-plane escapes. Six
focused tests and the full 149-test suite plus three CLI integrations pass;
Clippy is clean with warnings denied, the release build and both canonical
BUILD fronts pass, and LLVM coverage is 82.10% overall plus 97.42% for
`validator.rs`, with 98.89% of validator functions executed. The neutral corpus
validates 111 cases across 269 files; 146 repository guard tests plus 928
subtests pass with two expected Windows skips. The Go oracle passes tests, vet,
trimpath compilation, and a real forced Rust plan over 45 Starlark files and
1,157 packages with the five reviewed orphan exemptions clean and all packages
reported `WOULD-BUILD`. Cargo audit finds no vulnerability in 57 dependencies.

Before publication the branch rebased without conflict over merged ADJ
digraph-sound implementation and loop bookkeeping, Spanish A1 PCIC inventory,
Rust WASM SIMD rounding, Japanese learner-guide reconciliation, and HTML-parser
diagnostic work to exact `origin/main`
`c1659cc9a104d3406d4e4d821e468511ee793701`. Those commits remain inside
existing ADJ curriculum, human-language curriculum, portable WASM, and
HTML-parser owners and add no package identity. The refreshed report therefore
remains 15 established lanes, 1,373 identities, 4,566 slots, zero collisions,
and zero unknown buckets; all 10 live open PRs have zero exact candidate-path
overlap. The generated 459.6 MiB `target_isolated/` directory from the exact
Windows front was verified inside the dedicated worktree and removed with
`cargo clean`, while its separately owned hygiene item remains pending.

### Post-#12738 refresh and Python orphan-validator selection

PR #12738 completed every reported check without a failure or pending job.
GitHub reported the branch clean and mergeable, so the loop enabled squash
auto-merge; GitHub merged it as
`847c3f2fcd27f2491fcd3ae53948786e6d24e118` at
`2026-08-25T10:06:09Z` without a manual merge command. The exact merged head is
`8a969ecb9482145f86e8857ae8bbf741f2e98612`.

The mandatory post-merge refresh followed current `origin/main` through
`2445ef2bd42ddefd6b47ab3d9fc6aec5ac5c1e39`. The collision-checked schema-3
inventory is structurally unchanged: 15 established lanes, 1,373
implementation identities, 4,566 package slots, 175 high-consensus packages
with 276 missing slots, 122 five-to-nine-lane packages with 926 gaps, 166
two-to-four-lane packages with 2,087 gaps, 910 singletons with 12,740 gaps, and
721 Rust singletons. Canonical collisions and unknown language buckets remain
zero; OCaml remains correctly emerging at zero packages.

The source audit assigned every intervening change before selection. Merged PR
#12722 adds sourced Malayalam authored strokes to the existing neutral
script-ductus data owner; #12735 extends the HTML frontend's exact recovery
diagnostics; #12740 adds ADJ curriculum and E2E-harness authority evidence; and
#12741 extends the existing portable WAST parser, WebAssembly conformance, and
selection-blocked corpus-host owners. The ALGOL unit-selector repair remains
identity-neutral compiler semantics. After #12738, Spanish #12743 and Gujarati
#12747 are curriculum-only changes. Mermaid XY tick controls from #12744 extend
the existing chart conformance owner. Storage-fs #12728 closes the narrower
Unix directory-fsync error-propagation subrequirement but leaves its durable
monotone floor, cross-process coordination, crash/rollback, containment,
two-instance, documentation-truthfulness, and native-authority review pending.
No commit adds a package marker or requires a new owner.

The selected next owner is
`build-tool-python-orphan-crate-validation-conformance` on exact base
`2445ef2bd42ddefd6b47ab3d9fc6aec5ac5c1e39`. Its sole neutral-corpus
prerequisite is merged. Python is the widest remaining supported lane at 502
packages, already carries the pinned source-embedded Unicode 17 normalization,
full-folding, and uppercase substrate required by exemption validation, and can
add a bounded native adapter without filesystem, Git, process, environment, or
network authority. Eleven live PRs and every non-main remote head have zero
exact overlap with the six expected Python build-tool, state, and roadmap
paths; the target remote branch was absent. The higher-descendant extra-CI
corpus and OCaml process-free substrate remain collision-unsafe while live PRs
#12149 and #12162 own their required Go main and validator surfaces.

### Python orphan-validator implementation and exact-main refresh

The Python engine now consumes the four language-neutral orphan-crate cases
through a pure in-memory adapter. It matches the reviewed Rust oracle for
component-wise BUILD ancestry, runnable-over-empty precedence, the fixed BUILD
filename rank, exemption error and stale-entry precedence, Unicode 17 NFC plus
full-fold identities, portable-path rejection and hostile-path redaction, and
Python-ASCII-JSON diagnostic ordering. The adapter does not enumerate the
checkout, invoke Git or another process, read the environment, open a path, or
access the network.

Both literal `BUILD_windows` and `BUILD` fronts pass from independently cleared
uv 0.11.28 / Python 3.13.14 environments with 442 tests and 90.58% total
coverage in each run; the focused validator passes 40 tests at 97% coverage.
The neutral schema and runner suites pass 80 tests plus 121 subtests, and the
complete 111-case, 269-file corpus validates. The Go oracle passes tests, vet,
and trimpath compilation; its forced Python plan validates metadata and reports
all 494 discovered packages would build. Ruff, strict MyPy, compileall, focused
Bandit, uv dependency compatibility, pip-audit, wheel/sdist construction, the
package-parity collision gate, capability suites, state graph, credential, and
diff checks are clean. A source-wide Bandit probe retains only already-present
process-execution findings in unchanged CI, executor, and Git-diff modules.

Two conflict-free rebases brought the branch to exact `origin/main`
`d4e91eb24f1ed9f24e834fbff264b08dcaf60511`. PR #12755's guarded curriculum
ledger routing stays inside the existing TypeScript human-language-data owner;
PR #12751 is ADJ loop bookkeeping only; and PR #12750's positioned void-end-tag
diagnostics remain inside the existing Rust HTML-parser owner. None adds a
package marker, requires a new parity owner, or overlaps the six Python
build-tool, state, and roadmap paths. The refreshed collision report is
unchanged at 15 established lanes, 1,373 identities, 4,566 slots, 175
high-consensus packages with 276 gaps, 910 singletons with 12,740 gaps, 721
Rust singletons, zero collisions, and zero unknown buckets.

### Post-#12762 refresh and TypeScript tracked-artifact selection

PR #12762 reached 30 terminal acceptable checks: 24 succeeded, five skipped,
and one completed neutrally, with no failure or pending job. GitHub reported the
branch clean and mergeable, so the loop enabled squash auto-merge. GitHub merged
the exact reviewed head `430a99f8d48597ecb027abf4467a29f84f75bb29` as
`8149e2fe933ce4047ab417b8aa61d5af7ed49c6c` at
`2026-08-25T11:16:19Z` without a manual merge command.

The mandatory exact-main collision report remains structurally unchanged: 15
established lanes, 1,373 implementation identities, 4,566 package slots, 175
high-consensus packages with 276 missing slots, 122 five-to-nine-lane packages
with 926 gaps, 166 two-to-four-lane packages with 2,087 gaps, 910 singletons
with 12,740 gaps, and 721 Rust singletons. Canonical collisions and unknown
language buckets remain zero; OCaml remains correctly emerging at zero
packages.

Every intervening main commit is assigned before selection. PR #12746 extends
the shared script-ductus Tamil data and provenance owner; #12759 extends the
existing WAST parser, portable WebAssembly conformance, and selection-blocked
corpus-host owners with `v128.loadN_zero`; #12757 remains Rust-only ALGOL
compiler semantics under singleton classification; #12760 adds source-position
evidence to the existing portable HTML frontend; and #12761 adds shared ADJ
facts plus scratch/process E2E authority evidence to the existing ADJ CLI
capability review. No commit changes a package identity, BUILD marker, or
build-tool contract, and no new unowned gap appears.

The selected next owner is
`build-tool-typescript-tracked-artifact-validation-conformance` on exact base
`8149e2fe933ce4047ab417b8aa61d5af7ed49c6c`. All five declared prerequisites
are merged, and all 11 live open PRs plus 34 non-main remote heads have zero
exact overlap with its expected TypeScript build-tool, generator, state, or
roadmap surfaces. The dependency audit also found that the TypeScript orphan
consumer was falsely ready: ECMAScript host normalization and casing do not
supply the reviewed, source-pinned Unicode 17 NFC, NFKC, full-fold, and full
uppercase contract. The tracked-artifact consumer is therefore now an explicit
prerequisite of the TypeScript orphan owner. This bounded process-free slice
advances the widest remaining tracked-artifact lane at 446 packages and
establishes one reusable deterministic Unicode substrate without adding Git,
filesystem, process, environment, or network authority. The higher-descendant
extra-CI and OCaml owners remain collision-unsafe while live PRs #12149 and
#12162 own their required Go entry-point and validator surfaces. The reconciled
state now has 496 unique owners and 752 complete acyclic dependency edges, with
one in-progress owner and no missing prerequisite.

### TypeScript tracked-artifact implementation

The TypeScript build tool now consumes the five language-neutral
tracked-artifact fixtures through a pure in-memory snapshot adapter. It follows
the closed error precedence, redacts hostile input, normalizes separators only
lexically, counts and orders Unicode scalar values, detects exact, nested,
case, and compatibility aliases of `node_modules`, and evaluates reserved
Windows basenames with full uppercase. It adds no Git, filesystem, process,
environment, network, or credential authority.

The engine uses newly generated source-embedded Unicode 17.0.0 NFC, NFKC,
full-fold, NFKC-plus-fold, and full-uppercase tables. The exact Unicode notice
ships in the npm package and both manifest and lock metadata declare
`MIT AND Unicode-3.0`. The generator verifies pinned upstream sizes and SHA-256
identities, then executes both the generated Python and TypeScript runtimes over
all 20,034 official normalization vectors, 1,585 C/F folding rows, 1,581
unconditional uppercase mappings, derived NFKC-fold expectations, and Unicode
17 sentinels before accepting either artifact.

Seven generator tests, 34 focused validator tests, and the full 307-test Vitest
suite pass. V8 coverage is 89.61% statements, 82.16% branches, 94.18%
functions, and 89.51% lines overall; the validator reaches 100% statements,
lines, and functions plus 95.04% branches. A pinned temporary TypeScript 5.9.2
toolchain passes `tsc --noEmit`. Both literal npm BUILD steps, the complete
111-case and 269-file neutral corpus, 201 conformance tests with 23 expected
skips, the package-parity and capability suites, and Go test, vet, trimpath,
and BUILD validation pass. The npm package has an explicit 18-file allowlist,
includes the Unicode notice and generated source, and excludes local coverage
and dependency trees. Production dependency audit is clean; the sole full
audit finding is the already registered development-only `nanoid` owner.

Independent differential review compared 1,183,049 Unicode inputs with the
Python runtime without a mismatch. Independent security review found no
authority, dependency, license, credential, or packaging blocker. Its initial
package-allowlist finding and a separate persistent TypeScript full-vector
self-check finding were both corrected and revalidated. The implementation is
committed as `1fc9ee695cfceab184b0013b125802df398d6424`.

Before publication the branch rebased without conflict over Rust HTML
frame-position diagnostics, native Vault generation-zero journal recovery, and
ADJ curriculum-loop bookkeeping to exact `origin/main`
`ad36477baa5ada117c52ccb682b5b26fe4ee0510`. Those changes remain inside the
existing HTML frontend, Vault native storage/CLI, and ADJ fact/capability
owners, add no package identity or build-tool contract, and have zero exact
surface overlap. The generator, complete TypeScript suite, coverage, typecheck,
diff gate, state graph, and collision report pass again after the rebase; the
exact-main inventory remains 15 established lanes, 1,373 identities, 4,566
slots, zero collisions, and zero unknown buckets.

Ready-for-review PR #12773 opened from clean validated head
`eeb0f448217aa96bd170b2e719ddf64f8946aab8` after a normal first push. The
target remote branch and prior PR were absent before publication, and all 12
changed paths had zero exact overlap across 13 other live open PRs. GitHub
reports the branch mergeable with required checks queued; auto-merge remains
disabled until every check is terminal and acceptable.

### Post-#12773 refresh and TypeScript orphan-crate selection

PR #12773 reached 30 terminal acceptable checks: 24 succeeded, five skipped,
and one completed neutrally, with no failure or pending job. GitHub reported the
branch clean and mergeable, so the loop enabled squash auto-merge. GitHub merged
the exact reviewed head `fbe53cb7aba011d1295ecd7bc7b2bda50a7b2546` as
`3e114a1d37e919505671cad222cc8cea4f964801` at
`2026-08-25T12:17:10Z` without a manual merge command. A late fetch advanced
exact `origin/main` to `65729fcf2acae0df975a6e8c2b7db50bcad4e21f` through
PR #12772; its rejected-frameset diagnostic positions remain in the existing
portable HTML-frontend owner and do not overlap this build-tool slice.

The regenerated collision report remains structurally unchanged: 15
established lanes, 1,373 implementation identities, 4,566 package slots, 175
high-consensus packages with 276 missing slots, 122 five-to-nine-lane packages
with 926 gaps, 166 two-to-four-lane packages with 2,087 gaps, 910 singletons
with 12,740 gaps, and 721 Rust singletons. Canonical collisions and unknown
language buckets remain zero; OCaml remains correctly emerging at zero
packages.

Every intervening commit is classified before selection. PR #12767 extends the
existing Mermaid XY chart owner with legend configuration; #12733 and #12770
extend shared Chinese and Persian script-ductus data; and #12771 extends the
existing WAST parser, portable WebAssembly conformance, and corpus-host owners
with the six `v128.loadNxM_s/u` instructions. None changes a package identity
or BUILD marker. PR #12745 does expose one genuinely unowned portable contract:
contextual `>>` and `>>>` splitting with transactional rollback and memo
coherence in nested generic parsing. The state now records a neutral contract
root and a dependent established-lane umbrella. PR #12773 also exposes a
separate selection-blocked host-authority review for the Unicode generator's
PATH-resolved Node execution and temporary TypeScript runner. The Ruby
tracked-artifact consumer extends that same owner to PATH-resolved Ruby,
inherited Ruby environment state, temporary Ruby sources and runner, buffered
child output, diagnostics, and process-tree cleanup. Six orphan consumers now
explicitly depend on their same-language tracked-artifact consumer so no lane
is falsely ready without its reviewed Unicode substrate.

The selected next owner is
`build-tool-typescript-orphan-crate-validation-conformance` on exact base
`65729fcf2acae0df975a6e8c2b7db50bcad4e21f`. Its corpus, TypeScript discovery,
and TypeScript tracked-artifact prerequisites are all merged. This bounded,
process-free consumer advances the widest remaining orphan lane at 446
packages and reuses the source-pinned Unicode 17 tables without adding
filesystem enumeration, Git, process, environment, or network authority. Its
six expected paths have zero exact overlap across all 10 live open PRs and 33
non-main remote heads; the target branch was absent before the fresh worktree
was created. Extra-CI and OCaml work remain collision-unsafe while PRs #12149
and #12162 own their required Go entry-point and validator surfaces. The
reconciled graph has 499 unique owners and 762 complete acyclic dependency
edges, with exactly one in-progress owner and no active PR.

### TypeScript orphan-crate implementation

The TypeScript engine now exposes a pure `validateOrphanCrateSnapshot` adapter
over inert manifest, BUILD, and exemption records. It matches the closed
language-neutral contract for exact generated-artifact components, nearest
rooted BUILD membership, fixed missing-evidence rank, invalid and stale
precedence, hostile-path redaction, Unicode-scalar limits and ordering,
Python-equivalent whitespace and canonical detail ordering, NFC plus full
default casefold duplicate detection, and full-uppercase reserved basenames.
All four shared
fixtures plus adversarial astral, whitespace, BUILD-recognition, duplicate,
precedence, and Unicode-detail cases are consumed without adding Git,
filesystem, process, environment, clock, random, or network authority.

Red-to-green evidence first produced 16 expected missing-export failures. The
complete TypeScript suite then passed 324 tests across 12 files, with 90.5%
statement, 83.93% branch, 94.84% function, and 90.43% line coverage overall;
`validator.ts` reaches 100% statements, lines, and functions plus 97.01%
branches. TypeScript 5.9.2 typechecking, the literal BUILD recipe, the pinned
Unicode 17 generator check, the 111-case/269-file neutral corpus, conformance,
package-parity, capability, Haskell, OCaml-lock, strict-JSON, state-DAG, diff,
and collision gates pass. The Go build tool passes tests, vet, and trimpath
compilation; a real forced TypeScript dry plan evaluates 45 Starlark files,
discovers 476 packages, keeps the five-entry orphan ledger clean, and reports
all 476 WOULD-BUILD. The package tarball contains exactly 18 allowlisted files,
including the Unicode notice and no bundled dependencies. The runtime audit is
clean; the full audit retains only the separately registered development-only
`nanoid` advisory.

Independent differential review matched the Python oracle over 2,500
deterministic randomized and 139 systematic snapshots. Independent security
review found no new authority, path-disclosure, dependency, credential,
license, or packaging blocker. The implementation and defensive BUILD-filter
tests are committed as `dbcc0ece8d90507b0a3844451fd214088aa759e1`
and `ed74330505e209a36022933a7b2c7129a664691c`.

Before publication the branch rebased without conflict onto exact
`origin/main` `8ca94c2ef717427004c8a1c073fde078f0aa92f9`. PR #12774
extends the existing ADJ fact, test, and capability owner; PR #12768 extends
existing Gujarati curriculum and generated-data owners; and PR #12779 extends
the existing Rust `algol-iir-compiler`, `lang-aot`, and full-language-spec
owners; PR #12778 extends the existing Rust HTML-frontend diagnostics owner.
None changes a package identity or BUILD marker or overlaps the selected six
paths. The
refreshed collision report remains 15 established lanes, 1,373 identities,
4,566 slots, zero collisions, and zero unknown buckets. At the live audit, all
14 open PRs and 37 non-main remote heads had zero overlap with this slice;
PR #12780 temporarily owns Rust parser surfaces needed by the separate pending
contextual generic-closer owner.

Ready-for-review PR #12786 opened from clean validated head
`fe1f7adc7d20e44ef7ab753abf0c99a702330d99` after a normal first push. The
target remote branch and prior PR were absent immediately before publication.
Current main then advanced only through PR #12781's ADJ curriculum-loop state
bookkeeping at `207cd2bd13422b5047631ac0062c4f1954f5db41`; that commit is
covered by the existing ADJ owner and has no package marker or selected-path
overlap. GitHub reports PR #12786 mergeable with required checks queued, so
auto-merge remains disabled until every check is terminal and acceptable.

### Post-#12786 refresh and Ruby tracked-artifact selection

PR #12786 reached 30 terminal acceptable checks: 24 succeeded, five skipped,
and one CodeQL aggregate completed neutrally, with no failure or pending job.
GitHub reported the branch clean and mergeable, so the loop enabled squash
auto-merge. GitHub merged reviewed head
`2235050c40295ad636a2ea1727af9f023809a207` as
`655997ff5081d74dd16281d3e2c34a940695df7d` at
`2026-08-25T13:36:11Z` without a manual merge command.

The exact-main schema-3 collision report at
`143aa30a07bb5f90b84d202e7710e50e3d099fe2` remains structurally unchanged:
15 established lanes, 1,373 implementation identities, 4,566 package slots,
175 high-consensus packages with 276 missing slots, 122 five-to-nine-lane
packages with 926 gaps, 166 two-to-four-lane packages with 2,087 gaps, 910
singletons with 12,740 gaps, and 721 Rust singletons. Canonical collisions and
unknown language buckets remain zero; OCaml remains correctly emerging at zero
packages.

Every intervening commit is classified before selection. PR #12781 is ADJ
lifecycle bookkeeping; #12775 and #12787 are Japanese and Russian curriculum
content. PR #12782 extends the existing script-ductus neutral and lane owners;
#12783 extends the existing WAST parser, portable WebAssembly core, and host
authority owners; #12785 extends the existing Mermaid XY neutral and lane
owners; and #12788 extends the existing HTML-frontend diagnostics owner. None
adds or removes a package identity, BUILD marker, build-tool marker, or newly
unowned portable contract.

The later exact-main advance remains ownership-complete and non-overlapping:
#12777 changes only a test-only Vault socket retry under the existing Vault
native-authority review; #12791 adds Persian and Urdu kheh data under the
existing script-ductus owners; and #12790 extends the Dolch fact table and its
Rust CLI evidence under the existing ADJ capability owner. None adds a package,
BUILD, or build-tool marker, and none touches the selected Ruby surfaces.

The selected next owner is
`build-tool-ruby-tracked-artifact-validation-conformance`. All five declared
dependencies are merged. Ruby is the widest remaining tracked-artifact lane at
298 packages, and this consumer has two direct children plus three unfinished
descendants through the Ruby orphan validator and both completion umbrellas.
Its Ruby validator, generated Unicode, generator, license, tests, README,
CHANGELOG, state, and roadmap surfaces have zero overlap across all seven live
open PRs and current non-main remote heads; the target branch and prior PR were
absent before the fresh worktree was created. Extra-CI and OCaml remain
collision-unsafe while PRs #12149 and #12162 own required Go entry-point and
validator surfaces, and PR #12780 owns the Rust parser surfaces required by
contextual generic-closer work. The reconciled graph remains 499 unique owners
and 762 complete acyclic dependency edges, with exactly one in-progress owner
and no active PR.

### Ruby tracked-artifact implementation

The Ruby engine now exposes a pure in-memory tracked-artifact snapshot
validator and independently consumes all five language-neutral fixtures. It
implements the closed portable-path precedence, lexical slash normalization,
Unicode-scalar limits and ordering, invalid-path redaction, normalized safe
forbidden paths, NFKC plus full default folding for exact `node_modules`
components, full-uppercase reserved basenames, inert entry kinds, and canonical
diagnostic ordering. The runtime adapter adds no Git, filesystem, process,
environment, or network authority.

Generated source-embedded Unicode 17.0.0 tables provide NFC, NFKC, full
folding, NFKC-fold, and full-uppercase behavior without inheriting the host
Ruby tables. The generator pins every upstream size and SHA-256 identity,
ships the complete Unicode License v3 notice, and runs Python, emitted
TypeScript, and emitted Ruby over every official normalization, folding,
uppercase, derived NFKC-fold, and Unicode 17 sentinel family. Its 10-test suite
is now an explicit CI gate. The separate selection-blocked generator authority
owner now depends on this Ruby consumer and truthfully covers PATH-resolved
Ruby, inherited environment state, temporary sources and runners, buffered
output, diagnostics, and process-tree cleanup.

The canonical Ruby BUILD front passes 321 tests and 717 assertions with one
expected macOS-only skip, 89.57% line coverage, and 73.36% branch coverage;
Bundle check, StandardRB's substantive rules, and Ruby syntax checks pass. The
neutral corpus validates all 111 cases across 269 files, and the full build-tool
conformance family passes 201 tests with 23 expected platform skips. The
package-parity, capability, Haskell, and OCaml gates pass. The Go build tool
passes all packages, vet, trimpath compilation, BUILD validation, and a forced
Ruby dry plan over 305 packages while preserving the five-entry orphan ledger.
Ruby advisory audit and production npm audit report no vulnerability; the
unchanged development-only `nanoid` advisory remains registered separately.
Bandit reports no medium or high issue, and its low child-process notices are
owned by the generator authority review. Strict JSON and the 499-owner,
762-edge complete acyclic state graph pass.

Three independent read-only audits found no remaining correctness,
supply-chain, metadata, ownership, or integration defect. The required split
security review passed in one round, and its exact-head documentation recheck
also passed. A conflict-free rebase advanced the branch to exact `origin/main`
`3e29b2fee9572cf6a244c0bc8a889b655c2a3c22`; the intervening ADJ tracking-only
finalization and rejected-body HTML parser diagnostics remain under existing
owners and do not add identities or overlap. The refreshed schema-3 inventory
remains 15 established lanes, 1,373 identities, 4,566 slots, zero collisions,
and zero unknown buckets. All 12 live open PRs have zero exact overlap across
this branch's 11 changed paths.

Ready-for-review PR #12802 opened from clean validated head
`8593e7fce3160c2fa25d87844f7c632d929c6733` after a normal first push. Exact
`origin/main` remained at the reviewed base, and the target branch and prior PR
were absent immediately before publication. GitHub reports the PR mergeable
with required checks queued, so auto-merge remains disabled until every check
is terminal and acceptable and no merge conflict exists.

### Post-#12802 refresh and Elixir tracked-artifact selection

PR #12802 reached 30 terminal acceptable checks: 24 succeeded, five skipped,
and one CodeQL aggregate completed neutrally, with no failure or pending job.
GitHub reported reviewed head
`463d4a883535356809d16838bd426da901db4c0f` clean and mergeable, so the loop
enabled squash auto-merge. GitHub merged it as
`7a43168ec941da34b418ddefa34eff5056ac1d79` at
`2026-08-25T15:08:06Z` without a manual merge command.

The exact-main schema-3 collision report at
`e9f7e8e8c5957a90f5dc775c30ca3f5793d74f8f` remains structurally unchanged:
15 established lanes, 1,373 implementation identities, 4,566 package slots,
175 high-consensus packages with 276 missing slots, 910 singletons with 12,740
missing slots, and 721 Rust singletons. Canonical collisions and unknown
language buckets remain zero; OCaml remains correctly emerging at zero
packages.

Every intervening commit is classified before selection. Tamil zha, Malayalam
chillu l, and Latin writing stages remain under the existing script-ductus and
curriculum owners; the Chinese school cluster, exact level-snapshot shards, and
Dolch Primer completion are curriculum and ADJ data. The WebAssembly
lane-memory syntax remains inside the WAST-parser and portable WebAssembly
owners, Mermaid plot reservation remains inside the XY chart owner, ALGOL
selector preservation remains inside the existing frontend owner, HTML
self-closing diagnostics remain inside the HTML-frontend owner, and the shared
parser depth guard remains inside the contextual generic-closer/parser-hardening
owner. None adds or removes a package identity, BUILD marker, build-tool
marker, or newly unowned portable contract.

The dependency/leverage pass selects
`build-tool-elixir-tracked-artifact-validation-conformance` on branch
`codex/build-tool-elixir-tracked-artifact-validation-conformance`. All four
declared dependencies are merged. Elixir is the widest remaining
tracked-artifact lane at 279 packages and this consumer has three direct
children plus four unfinished descendants through the Elixir orphan validator,
both completion umbrellas, and the separately selection-blocked generator
authority review. Ruby orphan validation is newly ready after #12802, but
has only one unfinished descendant; Lua, Perl, Haskell, and Swift tracked
consumers cover 252, 251, 204, and 161 packages respectively. The expected
Elixir validator, generated Unicode, generator and generator-test, fixture
README, license, tests, package docs, CI, state, and roadmap surfaces have zero
exact overlap across all 13 live open PRs; the target branch and prior PR were
absent before the fresh worktree was created.

The audit found no newly unowned behavior or package gap. Stale fixture and
generator wording still names only earlier runtime consumers; full emitted
Elixir verification owns that documentation repair. Because that verification
adds a PATH-resolved Elixir process boundary, the selection-blocked Unicode
generator authority review now depends explicitly on this consumer and covers
the Elixir executable, inherited BEAM/Elixir environment, temporary modules and
runners, output, diagnostics, failure, and cleanup risks. The reconciled state
graph contains 499 unique owners and 763 complete acyclic dependency edges,
with exactly one in-progress owner and no active PR. The strategically broader
OCaml process-free substrate and extra-CI corpus remain collision-unsafe while
live PRs #12149 and #12162 own their required Go validator and main surfaces.

### Elixir tracked-artifact implementation

The Elixir engine now exposes pure
`validate_tracked_artifact_snapshot/1,2` entry points and independently consumes
all five language-neutral fixtures. The adapter implements the closed
portable-path precedence, lexical slash normalization, Unicode-scalar limits
and ordering, root-redacted invalid paths, normalized safe forbidden paths,
NFKC plus full default folding for exact `node_modules` components,
full-uppercase reserved basenames, inert entry kinds, and canonical diagnostic
ordering. It adds no Git, filesystem, process, environment, or network
authority.

Generated, source-embedded Unicode 17.0.0 tables provide NFC, NFKC, full
folding, NFKC-fold, and full uppercase without inheriting the host BEAM tables.
The generator pins exact upstream byte counts and SHA-256 identities, carries
the complete Unicode License v3 notice, and runs Python plus emitted
TypeScript, Ruby, and Elixir over all 20,034 official normalization vectors,
1,585 C/F folding rows, 1,581 unconditional uppercase mappings, derived
NFKC-fold expectations, and the two Unicode 17 sentinels. Both real generation
and byte-check mode pass, as do all 15 generator tests.

Elixir 1.18.4 passes warnings-as-errors compilation, changed-file formatting,
20 focused validator tests, and the complete 230-test suite with two expected
skips. Coverage measures `BuildTool.Validator` at 96.30% and the generated
Unicode module at 80.85%; the package-wide 52.77% result remains below Mix's
default 90% threshold because the unchanged CLI, resolver, and Git-diff modules
have no coverage instrumentation, and is recorded rather than hidden or folded
into this bounded consumer. The neutral corpus validates all 111 cases and 269
files; 201 conformance tests pass with 23 expected platform skips. The focused
package-parity, capability, Haskell-capability, and OCaml-lock families pass 68
tests with two expected Windows skips.

The Go build tool passes all packages, vet, trimpath compilation, BUILD
validation, and a forced Elixir dry plan over 285 packages. The refreshed
schema-3 inventory remains 15 established lanes, 1,373 implementation
identities, 4,566 slots, zero collisions, and zero unknown buckets. Hex reports
no retired or advisory dependency; production npm audit reports no
vulnerability, while the unchanged development-only `nanoid` advisory remains
under its registered owner. Ruff, Bandit medium/high, diff, strict JSON, and
state-graph checks pass. Two independent split security reviews passed in one
round. An independent correctness review found that mocks covered the generator
subprocess boundary without making the real Elixir vector check durable in CI;
the branch now adds a required read-only job on pinned Elixir 1.18.4 and OTP
27.3.4.11, and the stable CI gate explicitly requires its real full-vector
result. A focused post-fix review found no remaining correctness, workflow, or
security defect. Generator host authority remains separately selection-blocked
and truthfully owns PATH, environment, temporary-file, output, failure, and
process cleanup risks.

The branch rebased without conflict onto exact `origin/main`
`07a09f4ccc93eb74fa12072d493238d93a027b6b` after classifying intervening HTML
frontend diagnostics, vault-pm export handling, and Java-to-semantic-IR
lowering under their existing owners. All ten live open PRs have zero exact
overlap across this branch's 12 changed paths, and the target remote branch
remains absent.

Ready-for-review PR #12819 opened from validated head
`5771510efc0083c8186ab632bb368618c21e52ff` after a normal first push. GitHub
reports the branch mergeable, with required checks queued or in progress,
including the new Unicode 17 generated Elixir conformance job. Auto-merge
remains disabled until every required check is terminal and acceptable and the
branch remains conflict-free.

### Post-#12819 refresh and Lua tracked-artifact selection

PR #12819 reached 31 terminal acceptable checks: 25 succeeded and six were
expected skips, with no failure or pending job. GitHub reported reviewed head
`33df983e3f6eba67e05aa2f6f71bc5de1c3f33b0` clean and mergeable, so the loop
enabled squash auto-merge. GitHub merged it as
`a939778e7f7ce375bc5b4deefdb3467ca487ab54` at
`2026-08-25T16:50:05Z` without a manual merge command.

The mandatory exact-main schema-3 collision report remains structurally
unchanged: 15 established lanes, 1,373 implementation identities, 4,566
package slots, 175 high-consensus packages with 276 missing slots, 910
singletons with 12,740 gaps, and 721 Rust singletons. Canonical collisions and
unknown language buckets remain zero; OCaml remains correctly emerging at zero
packages. The state graph before newly discovered owners contained 499 unique
owners and 763 complete acyclic edges. A pre-existing historical inversion in
which merged `build-file-standalone-integrity-lua` names the still-pending
`build-file-standalone-integrity` umbrella remains unchanged; dependency
readiness is therefore evaluated for the candidate tranche rather than claimed
as a global lifecycle invariant.

Every intervening commit is classified before selection. ADJ Dolch Primer and
First/Second Grade tracking, HTML colgroup and template-table diagnostics,
vault-pm best-effort export, Kannada and Gujarati script work, and
Java-to-semantic-IR lowering remain inside existing owners. The Java execution
proof now silently skips without `python3` while its BUILD files declare no
Python requirement, so that concrete consumer is added to the existing
extra-CI toolchain corpus and remaining-engine adoption owner.

The audit also found two genuinely unowned behavior families before selecting
the next implementation. Rust alone added the complete SIMD lane-memory
load/store family across `wasm-opcodes`, `wasm-validator`, and
`wasm-execution`, even though those identities report package presence in all
15 established lanes. New neutral and established-lane owners now cover binary
sub-opcodes, memarg/lane ordering, width-specific bounds, little-endian lane
updates, stack typing, traps, and multi-memory behavior while leaving WAST
syntax and official-corpus consumption with their existing owners. Rust's
`algol-iir-compiler` also added bounded chained-Boolean selector identity and
seven-backend results without an exact delivery contract; new neutral and
selection-blocked applicable-lane owners now pin that behavior and require an
explicit interpreter-IR lane decomposition. These four owners are registered
as pending before the next item moves to in-progress.

The dependency/leverage pass selects
`build-tool-lua-tracked-artifact-validation-conformance` on branch
`codex/build-tool-lua-tracked-artifact-validation-conformance`. All four
declared prerequisites are merged. Lua is the widest remaining tracked lane at
252 packages, ahead of Perl at 251, Haskell at 204, and Swift at 161. It has
two existing direct children and three unfinished descendants through the Lua
orphan validator and both completion umbrellas. Registering its real emitted
Lua subprocess verification under the excluded Unicode-generator authority
review raises that leverage to three direct children and four unfinished
descendants without granting process authority to the pure runtime adapter.
Elixir orphan validation is newly ready but unlocks only the orphan completion
umbrella.

All nine live open PRs and all non-main remote heads have zero exact overlap
with the expected Lua validator, generated Unicode module, generator and test,
fixture README, Unicode license, package tests and docs, CI, state, and roadmap
surfaces. The target branch and prior PR were absent before the clean worktree
and branch were created. The broader extra-CI corpus and OCaml process-free
substrate remain collision-unsafe while PRs #12149 and #12162 own their
required Go validator and main surfaces; PR #12824 likewise owns the
otherwise-ready Mermaid XY surfaces. After registering the four new owners and
the Lua generator-authority edge, the graph contains 503 unique owners and 766
complete acyclic dependency edges, with exactly one in-progress owner and no
active parity PR.

### Lua tracked-artifact implementation

The Lua build tool now exposes a pure
`validate_tracked_artifact_snapshot` adapter and independently consumes all
five language-neutral fixtures. It implements the closed portable-path error
precedence, lexical slash normalization, Unicode-scalar length limits and
ordering, hostile-path redaction, normalized safe forbidden paths, NFKC plus
full default folding for exact `node_modules` components, full-uppercase
Windows reserved basenames, inert entry kinds, and deterministic canonical
diagnostic ordering. The adapter adds no Git, filesystem, process,
environment, or network authority.

Generated source-embedded Unicode 17.0.0 tables provide NFC, NFKC, full
folding, NFKC-fold, and full uppercase without inheriting host Unicode data.
The generator pins the upstream byte counts and SHA-256 identities, carries
the complete Unicode License v3 notice, and checks the emitted Lua module over
every official normalization, C/F folding, unconditional uppercase, derived
NFKC-fold, and Unicode 17 sentinel vector. A required read-only CI job builds
the repository-pinned Lua 5.4.7 toolchain and runs that real emitted-runtime
check through an explicit executable path; the stable CI gate explicitly
requires its result. The generator invokes Lua with `-E` under a minimal
environment, retains at most 8 KiB from each output stream, and terminates the
isolated process tree after every verifier exit. Windows starts the verifier
suspended, assigns it to a kill-on-close Job Object before resuming it, and
waits for job accounting to reach zero; POSIX always kills the isolated process
group. Real timeout and early-root-exit descendant probes plus a cleanup-error
ownership regression cover the boundary.

The repository-pinned Lua 5.4.7 toolchain passes syntax compilation, 19
focused validator tests, and the complete 72-test Lua build-tool suite. LuaCov
measures the validator at 98.23% and the generated Unicode module at 84.06%; the
package-wide total is 51.24% because unchanged CLI, resolver, and bundled test
framework code remains outside this bounded consumer. All 23 generator tests
and six pinned-Lua setup tests pass on Windows; the two Job Object failure-path
tests are expected platform skips on POSIX. Ruff, changed-source Luacheck, workflow
YAML parsing, generated-byte verification, and the full emitted Lua Unicode
self-check pass.

The neutral corpus validates 111 cases and 269 files. The schema and runner
conformance families pass 22 and 58 tests respectively; package-parity,
capability, Haskell-capability, and OCaml-lock suites pass another 66 tests
with two expected platform skips and 928 subtests. The Go build tool passes all
packages with coverage, vet, and trimpath compilation, then a fresh binary
evaluates 45 Starlark BUILD files and validates a forced dry plan over all 259
Lua packages.
The refreshed schema-3 inventory remains 15 established lanes, 1,373
implementation identities, 4,566 slots, 175 high-consensus packages with 276
gaps, 910 singletons with 12,740 gaps, 721 Rust singletons, zero collisions,
and zero unknown buckets. Bandit medium/high, strict JSON, state-graph, and diff
checks pass; no package dependency changed. Generator subprocess authority
remains explicitly selection-blocked under its separate review owner.

The branch rebased without conflict onto exact `origin/main`
`4faca735e6851d24bb78f96817292dbb354c0748` after classifying intervening HTML
parser diagnostics and Malayalam script data under their existing owners. The
merged SIMD lane-memory completion, ALGOL power-one selector writes, and
Mermaid XY axis theme colors remain inside the neutral/parity owners registered
before selection and the existing WAST/conformance, ALGOL, and Mermaid XY
owners. Later implied-row HTML diagnostics and JV02 classic/enhanced for-loop
lowering likewise remain inside the HTML frontend and Java-to-Semantic-IR
owners; its two Python execution proofs retain the already registered extra-CI
toolchain declaration gap. None adds or removes a package identity,
BUILD/build-tool marker, or Lua tranche path. The focused package, generator,
full-vector, Go oracle,
build-plan, lint, and syntax gates were rerun successfully after the rebase.

Independent split security review passed the pure Lua adapter and generated
Unicode substrate without a finding. Generator and CI review found and then
closed executable/environment, bounded-output, timeout-tree,
early-root-descendant, Job Object ownership, and suspended setup-failure risks.
The final reviews of validated code revision
`f459a4cf54cbddac899a1172a22180d91a4acadd` returned `SECURITY REVIEW PASSED`:
CI is read-only with checkout credentials disabled, the Unicode and Lua source
inputs remain hash-pinned, and no actionable security issue remains.

Ready-for-review PR #12843 opened from clean published head
`997ffd2bf389d66e2aeadd59089898f6696257a2`. All 33 reported checks reached
terminal acceptable conclusions: 27 succeeded and six were expected skips,
with no failure or pending job. GitHub reported the branch clean and
mergeable, so the loop enabled squash auto-merge. GitHub merged it as
`de59078b731c226e77e9475ce83669070d6be383` at
`2026-08-25T19:04:20Z` without a manual merge command.

### Post-#12843 refresh and Perl tracked-artifact selection

The exact-main schema-3 collision report at
`de59078b731c226e77e9475ce83669070d6be383` remains structurally unchanged:
15 established lanes, 1,373 implementation identities, 4,566 package slots,
175 high-consensus packages with 276 missing slots, 910 singletons with 12,740
gaps, and 721 Rust singletons. Canonical collisions and unknown language
buckets remain zero; OCaml remains correctly emerging at zero packages.

Every commit since the Lua branch's final rebase is classified before the next
selection. Tamil independent-u ductus remains curriculum and script data.
HL22's deterministic Markdown partition/join helper and its filesystem-backed
shard, unshard, and check CLI remain inside the existing TypeScript
human-language-data domain and generator-authority owner; that owner now
records plan-table target selection, enumeration, exclusive creation,
monolith replacement, containment, and disk-derived diagnostics. ALGOL
unary-plus selector writes, HTML caption recovery, and Mermaid XY axis theme
colors extend their already registered neutral/parity or frontend owners. None
adds or removes a package identity or BUILD/build-tool marker.

Merged PR #12844 does expose one genuinely unowned portable behavior family:
Rust alone now implements the relaxed-SIMD families including
`i8x16.relaxed_swizzle`, `f32x4.relaxed_min/max`, and
`f64x2.relaxed_min/max`, plus recursive N-ary `either` expected-result grading
across the WAST parser, conformance reporter, opcode, validator, and execution
stack. Two registered owners separate a neutral relaxed-SIMD contract from an
established-lane completion umbrella. The neutral owner covers all twenty
`0x100` through `0x113` instructions, canonical multi-byte LEB128 encoding,
deterministic permitted implementations, recursive N-ary accepted-result sets,
validator shapes, and diagnostics. Existing
WAT/WAST, conformance-report, corpus, and host-authority owners retain syntax,
directive execution, pinned provenance, fetching, and baseline writes. The
state graph now has 505 unique owners and 768 complete acyclic edges.

The dependency/leverage pass selects
`build-tool-perl-tracked-artifact-validation-conformance` on branch
`codex/build-tool-perl-tracked-artifact-validation-conformance`. All four
declared prerequisites are merged. Perl is the widest remaining
tracked-artifact lane at 251 packages, ahead of Haskell at 204 and Swift at
161. It directly unlocks the Perl orphan-crate consumer and advances the
tracked-artifact completion umbrella. Full emitted-Perl Unicode verification
is recorded under the excluded generator-host owner and must reuse the
hardened bounded process-tree runner without granting process authority to the
pure validator.

All eight live open PRs have zero exact overlap with the expected Perl
validator, generated Unicode, generator/test, fixture README, license, package
tests/docs, CI, state, and roadmap surfaces. The target branch and prior PR
were absent locally and remotely before the fresh worktree and branch were
created. Lua and Elixir orphan validation are ready but narrower. The broader
extra-CI corpus and OCaml process-free substrate remain collision-unsafe while
PRs #12149 and #12162 own required Go entry-point and validator surfaces, and
the ALGOL neutral contract overlaps PR #12847. After selection the ledger has
141 merged owners, 363 pending owners, exactly one in-progress owner, and no
active parity PR.

### Perl tracked-artifact implementation

The Perl build tool now exposes one pure
`validate_tracked_artifact_snapshot` adapter and exactly consumes all five
language-neutral fixtures. It preserves the closed portable-path precedence,
lexical separator normalization, hostile-path redaction, Unicode-scalar limits
and ordering, NFKC plus full-fold `node_modules` identities, full-uppercase
Windows reserved basenames, inert entry kinds, and canonical deterministic
diagnostics. Inputs are rejected above 512 Perl characters before regex or
scalar-unpack work, and the adapter adds no Git, checkout, link-following,
process, environment, or network authority.

Generated source-embedded Unicode 17.0.0 tables provide NFC, NFKC, full case
folding, NFKC-fold, and full uppercase independently of the host Perl Unicode
tables. The pinned generator checks exact upstream byte counts and SHA-256
identities, emits the complete Unicode License v3 notice, and verifies every
official normalization, C/F folding, unconditional-uppercase, derived
NFKC-fold, outlined-letter, and Todhri vector against the emitted Perl source.
A required read-only `ubuntu-24.04` job checks the runner's pinned Perl 5.38.2,
uses an explicit executable path with taint mode and an isolated module path,
and feeds its result into the stable CI gate. The verifier reuses the hardened
minimal-environment, 8-KiB-per-stream, bounded-time process-tree runner; Windows
assigns a suspended root to a kill-on-close Job Object before resuming it, and
POSIX isolates and cleans the process group.

The exact Strawberry Perl 5.38.2.2 portable toolchain passes Perl syntax, nine
focused validator subtests, and the complete 14-file, 129-test package suite.
An independently mirrored repo-shaped distribution passes dependency install,
`Makefile.PL`, and `gmake test`, confirming that the generated module ships in
the MakeMaker artifact. Focused Devel::Cover measurement reports 100% statement
and subroutine coverage for the validator and 82.0% total for that module;
generated Unicode code reaches 82.0% statement coverage. Strict changed-source
Perl::Critic passes with the two documented explicit-return/sort policies
excluded. All 26 generator tests, Ruff checks, formatting, workflow YAML
parsing, generated-byte verification, and the real full-vector Perl self-check
pass.

The neutral corpus validates 111 cases and 269 files. Schema, runner,
package-parity, capability-taxonomy, Haskell-capability, and OCaml-lock suites
pass 148 tests with two expected platform skips. The Go oracle passes all
packages with coverage, vet, and trimpath compilation; a fresh binary evaluates
45 Starlark BUILD files and validates a forced dry plan over 258 discovered
packages. The refreshed schema-3 report remains 15 established lanes, 1,373
implementation identities, 4,566 slots, 175 high-consensus packages with 276
gaps, 910 singletons with 12,740 gaps, 721 Rust singletons, zero collisions,
and zero unknown buckets.

Bandit medium/high, strict JSON, state-graph, diff, and direct CPAN-module
security checks pass; no tracked dependency changes. CPAN::Audit reports no
advisories for the exact direct module versions. Its separate upstream Perl
5.38.2 scan reports inherited interpreter notices, but this bounded consumer
uses no attacker-supplied regex or pack templates, no threads, and no
non-ASCII transliteration left-hand side. Independent split reviews found no
actionable issue in the validator, generated substrate, generator, CI, tests,
license, or metadata. Generator subprocess authority remains explicitly
selection-blocked under its separate review owner.

The branch rebased without conflict onto exact `origin/main`
`26dc47061ef57cbeb2fd2ce900c9e5f4b4a6ab54`. The four intervening commits
only advance the existing ADJ curriculum ledger, Gujarati doorway retrieval,
Persian/Urdu feh ductus, and registered ALGOL IIR selector-identity owners.
They add or remove no package identity or BUILD/build-tool marker and touch no
Perl tranche path. Focused package, generator, emitted-vector, workflow,
state-graph, and diff validation was rerun after the rebase. Independent split
reviews of content-equivalent rebased head
`7bf7926ba5280f54c71be16ce8ff22daed0520be` returned
`SECURITY REVIEW PASSED` or `REVIEW PASSED` with no actionable issue.

Ready-for-review PR #12862 opened from clean published head
`e5ba227fc3a58a6ebac8ce08e0b32169ac141079`. All 35 reported checks reached
terminal acceptable conclusions: 29 succeeded and six were expected skips,
with no failure or pending job. GitHub reported the branch clean and
mergeable, so the loop enabled squash auto-merge. GitHub merged it as
`0ed3d402a2bf61424032ad34f88223348b915a3c` at
`2026-08-25T20:44:39Z` without a manual merge command.

### Post-#12862 refresh and Haskell tracked-artifact selection

The exact-main schema-3 collision report at
`0ed3d402a2bf61424032ad34f88223348b915a3c` remains structurally unchanged:
15 established lanes, 1,373 implementation identities, 4,566 package slots,
175 high-consensus packages with 276 missing slots, 910 singletons with 12,740
gaps, and 721 Rust singletons. Canonical collisions and unknown language
buckets remain zero; OCaml remains correctly emerging at zero packages.

Every intervening commit is classified before selection. Gujarati R3 closure
and R4 bridge work remains curriculum and generated TypeScript data. Nested
anchor diagnostics extend the HTML frontend owner. Java method declarations,
calls, overload choice, rejection, and backend proofs extend the JV02 neutral
and lane owners while the existing extra-CI owner retains the Python proof
requirement. The long-vowel ADJ fact stays in the shared fact corpus and its
scratch-filesystem/process E2E extends the adj-lang-cli capability owner.
Signed-neutral ALGOL writes, the pinned Mermaid 11.16.1 XY compatibility
corpus, and `i16x8.relaxed_q15mulr_s` semantics extend their existing ALGOL,
Mermaid, relaxed-SIMD, WAST, and corpus-provenance owners. No intervening
commit adds or removes a package identity or BUILD/build-tool marker, and no
new owner is required.

The dependency/leverage pass selects
`build-tool-haskell-tracked-artifact-validation-conformance` on branch
`codex/build-tool-haskell-tracked-artifact-validation-conformance`. All four
declared prerequisites are merged. Haskell is the widest remaining
tracked-artifact lane at 204 reported packages, ahead of Swift at 161. It
directly unlocks the Haskell orphan consumer and advances both tracked-artifact
and orphan completion umbrellas, giving it more unfinished descendants than
any ready orphan-only child. Full emitted-Haskell Unicode verification is
recorded under the excluded generator-host owner and must reuse the hardened
bounded process-tree runner without granting process authority to the pure
validator.

All ten live open PRs have zero exact overlap with the expected Haskell
validator, generated Unicode module, Cabal/package metadata, generator and
test, fixture README, Unicode license, package tests/docs, CI, state, and
roadmap surfaces. The target branch and prior PR were absent locally and
remotely before the fresh worktree and branch were created. Perl, Lua, Elixir,
and Ruby orphan validation are ready but narrower. The broader extra-CI corpus
and OCaml process-free substrate remain collision-unsafe while PRs #12149 and
#12162 own required Go entry-point, validator, CI, or Swift surfaces. After
selection the graph contains 505 unique owners and 769 complete acyclic edges,
with 142 merged owners, 362 pending owners, exactly one in-progress owner, no
pr-open owner, and no active parity PR.

### Haskell tracked-artifact implementation

The Haskell build tool now exposes a pure in-memory tracked-artifact validator
and consumes all five language-neutral fixtures. It preserves the closed
portable-path precedence, lexical slash normalization, hostile-path redaction,
Unicode-scalar length and ordering, NFKC plus full-fold `node_modules`
identities, full-uppercase Windows reserved basenames, inert entry kinds, and
canonical deterministic diagnostics without adding Git, filesystem, process,
environment, or network authority to the adapter.

Generated source-embedded Unicode 17.0.0 tables provide NFC, NFKC, full case
folding, NFKC-fold, and full uppercase independently of host Unicode tables.
The pinned generator checks exact upstream byte counts and SHA-256 identities,
emits the complete Unicode License v3 notice, and verifies every official
normalization, C/F folding, unconditional-uppercase, derived NFKC-fold, and
Unicode 17 sentinel vector against the emitted Haskell source. The verifier
requires exact reviewed GHC 9.4.8 `runghc` and compiler paths, disables user
package databases and package environments, uses a minimal environment and
isolated temporary directory, bounds retained output, and unconditionally
cleans the isolated process tree.

The full Haskell BUILD front passes graph's four examples, directed-graph's
three examples, and the build tool's 44 examples. HPC reports 75% expression,
69% alternative, and 59% top-level-definition coverage across the existing
large `BuildTool` module. The complete shared resolver fixtures also exposed
and this tranche repairs two existing Haskell-engine defects: ambiguous
multi-Cabal metadata and duplicate Dart manifest-name aliases contribute no
ambiguous edge, while every discovered package node and the Haskell directory
aliases remain available and a package identity still wins over a
same-basename program identity.

All 29 generator tests and the real emitted-Haskell official-vector check pass.
The neutral corpus validates 111 cases and 269 files; conformance,
package-parity, Haskell-capability, and OCaml-lock families pass 262 tests with
25 expected platform skips. The Go oracle passes all packages with coverage,
vet, and trimpath compilation; a fresh binary evaluates 45 Starlark BUILD files
and validates a forced Haskell dry plan over all 207 discovered Haskell
packages. Cabal metadata validation, Ruff check and formatting, zero-new-hint
HLint comparison, workflow YAML, strict state JSON, the complete acyclic state
graph, collision checks, Bandit medium/high, credential-pattern review, and
diff checks pass. No dependency changed.

A required read-only `ubuntu-24.04` job pins GHC 9.4.8 and Cabal 3.10.3.0,
disables checkout credentials, executes the real full-vector verifier, and
feeds its result into the stable CI gate. Generator subprocess authority stays
under the separate selection-blocked host-authority owner.

Independent generator and CI security review passed the exact emitted-vector
implementation, pinned action and toolchain objects, package isolation,
bounded streams, process-tree cleanup, Unicode provenance, and read-only gate.
Adapter review caught one real resolver regression in the first implementation:
ambiguous Haskell and Dart metadata had removed whole packages from the graph.
The final repair suppresses only the ambiguous aliases and dependency edges,
retains all discovered nodes and Haskell directory aliases, and extends the
shared node and scheduling assertions. All 51 Haskell examples pass afterward.

The branch rebased without conflict onto exact `origin/main`
`89a236a69a925673b734cacf07a1ebca4024205d`. Grouped ALGOL selector identity,
relaxed SIMD min/max, HTML table diagnostics, Tamil dependent long-i, Gujarati
R4 bridge B, suffix-meaning ADJ work, and its later thread-ledger update remain
inside their existing neutral, frontend, conformance, curriculum, or adjacent
automation-state owners and do not overlap this tranche.
The first Mermaid Gantt grammar/parser/IR/temporal-layout/paint pipeline was
newly unowned, so `mermaid-gantt-chart-portable-conformance` and its dependent
established-lane parity umbrella were registered before publication. The state
graph now contains 507 unique owners and 770 complete acyclic edges: 142 merged,
364 pending, and exactly one in progress.

Ready-for-review PR #12878 opened from clean validated head
`af4d907c1832402c71978ee71c52d958dac9662b`; its state-tracking commit advanced
the reviewed head to `f29bd65249fb3f08a7638a88bee7063e52e6f020`. One CodeQL
language-detection attempt failed before checkout because GitHub's internal
action-download host did not resolve. The loop inspected the actual log and
reran only that failed infrastructure job. The replacement succeeded, leaving
31 successes and six expected skips with no failure or pending job. GitHub
reported the branch clean and mergeable, so the loop enabled squash auto-merge.
GitHub merged PR #12878 as
`9042d13df3ea2391db6d23a0e49066e233e06ace` at
`2026-08-25T22:54:56Z` without a manual merge command.

### Post-#12878 refresh and Swift tracked-artifact selection

The exact-main schema-3 collision report at
`ba51676b1047f365715a2fede4fbfc27a6d0ea37` remains structurally unchanged:
15 established lanes, 1,373 implementation identities, 4,566 package slots,
175 high-consensus packages with 276 missing slots, 122 identities in five to
nine lanes with 926 gaps, 166 identities in two to four lanes with 2,087 gaps,
910 singletons with 12,740 gaps, and 721 Rust singletons. Canonical collisions
and unknown language buckets remain zero. OCaml remains correctly emerging at
zero packages, and a Git-tree identity comparison with the preceding inventory
finds no changed implementation bucket.

Every intervening change is classified before selection. Java lambdas extend
the JV02 neutral and lane owners. Relaxed SIMD lane-select extends the existing
relaxed-SIMD, WAST-parser, conformance, and corpus-provenance owners. HTML form
diagnostics extend the HTML frontend owner, conditional ALGOL selector writes
extend the ALGOL IIR owner, and human-language plus ADJ changes remain
curriculum, generated-data, or adjacent automation-state surfaces. The Mermaid
Gantt pipeline remains covered by the two owners registered before PR #12878
opened.

The commit-level audit also registered two newly exposed, selection-blocked
native authority reviews before confirming the next portable selection. The
first owns residual TeX `openin_any=a` and repository-controlled SVG-ingestion
risks in human-language book builds. Its dependent owns the write-authorized
publication job's mutable third-party action references, full-SHA pinning, and
artifact provenance. These host-security and supply-chain reviews do not enter
the all-language denominator; no newly unowned eligible portable gap remains.

The dependency/leverage pass selects
`build-tool-swift-tracked-artifact-validation-conformance` on branch
`codex/build-tool-swift-tracked-artifact-validation-conformance`. All five
declared prerequisites are merged. Swift is the sole remaining tracked-artifact
engine, covers 161 reported packages, directly unlocks the Swift orphan-crate
consumer, and completes the implementation dependencies of the
tracked-artifact umbrella. Its three unfinished descendants give it more
immediate leverage than any ready orphan-only child.

All eight live open PRs have zero exact overlap with the expected Swift
build-tool, generated Unicode, generator and test, fixture documentation,
license, CI, state, and roadmap surfaces. The target branch and prior PR were
absent before the fresh clean worktree and branch were created. The
higher-descendant extra-CI corpus and strategically important OCaml
process-free substrate remain collision-unsafe while PRs #12149 and #12162 own
their required Go entry-point and validator surfaces. After lifecycle
reconciliation and selection, the graph contains 509 unique owners and 772
complete acyclic edges: 143 merged, 365 pending, and exactly one in-progress
owner, with no active parity PR.

The selection also adds Swift as a dependency of the existing excluded Unicode
generator host-authority review. The generated-table verification requires
explicit Swift 6.3.3 `swift` and `swiftc` executables, isolated temporary source
and module-cache writes, and on Windows toolchain-relative runtime, SDK, and
linker-library selection; these native powers remain outside the pure Swift
validator.

Before publication, the branch rebased without conflict onto exact main
`52adb62ee56ec11203f5d047a399d57bb2f1047a`. The intervening relaxed-SIMD
madd/nmadd merge remains owned by the relaxed-SIMD, WAST, conformance, and
provenance work; Java-to-Semantic-IR array declarations remain owned by JV02.
Neither changes package identity or build-tool scope. The refreshed report is
unchanged at 15 lanes, 1,373 identities, 4,566 slots, zero collisions, and zero
unknown buckets, and all 13 tranche paths have zero exact overlap across nine
live open PRs.

### Post-#12895 refresh and Ruby orphan-crate selection

PR #12895 completed 45 terminal acceptable checks: 38 successes, six expected
skips, and one neutral result. GitHub reported the reviewed branch clean and
mergeable, so the loop enabled squash auto-merge. GitHub merged reviewed head
`ad5495ea7cf008d0db6a91137209b935290f7d69` as
`89364dd43dc165e7a683a74df109d4fb77d40725` at
`2026-08-26T00:58:42Z` without a manual merge command.

The exact-main schema-3 collision report at
`297c1f3f72da31a856389b774f5cf7dd11cfb8d9` remains structurally unchanged:
15 established lanes, 1,373 implementation identities, 4,566 package slots,
175 high-consensus packages with 276 missing slots, 122 identities in five to
nine lanes with 926 gaps, 166 identities in two to four lanes with 2,087 gaps,
910 singletons with 12,740 gaps, and 721 Rust singletons. Canonical collisions
and unknown language buckets remain zero. OCaml remains correctly emerging at
zero packages. The intervening human-language and Rust HTML-parser merges add
no package identity or build-tool contract and do not overlap the next tranche.

The live-PR audit found one newly unowned future family before selection. The
then-open PR #12908 exposed Rust-only Wasm GC i31 parsing, validation, and
execution work, including incorrect stack-identity implementations for
`ref.i31` and `i31.get_s`. The state now registers
`wasm-gc-i31-language-neutral-conformance` for the bounded neutral fixture
contract and a dependent, selection-blocked
`wasm-gc-i31-established-lane-parity` umbrella. Generic WAST execution,
vendored-corpus provenance, fetching, NOTICE, and baseline authority remain in
their existing owners. PR #12908 subsequently merged as exact-main revision
`477e3f742304876fd5c17bcce21d8abf66f0075c`; its implementation remains
covered by those registered owners. The resulting state graph contains 511
owners and 773 dependency edges before the selected implementation lands.

The dependency/leverage pass selects
`build-tool-ruby-orphan-crate-validation-conformance` on branch
`codex/build-tool-ruby-orphan-crate-validation-conformance`. Its orphan corpus,
remaining-engine `dist-newstyle` exclusion, and Ruby tracked-artifact substrate
are merged. Ruby covers 298 reported packages versus Swift's 161, while each
consumer unlocks the same orphan-crate umbrella descendant. Every live PR has
zero exact overlap with the six Ruby tranche paths, and no prior branch or PR
exists. OCaml's process-free core remains collision-unsafe while PRs #12149 and
#12162 both own the required Go entry-point and validator surfaces.

Before publication, the Ruby branch rebased twice without conflict, finally
onto exact main `297c1f3f72da31a856389b774f5cf7dd11cfb8d9`. The intervening ADJ
bookkeeping, Wasm GC i31, Vault, and JV02 merges leave the schema-3 inventory
unchanged. Merged PR #12912 remains in the existing JV02 neutral/lane owners,
and merged PR #12911 remains in the existing Vault owners; neither overlaps
any of the six Ruby tranche paths.

### Post-#12920 refresh and Elixir orphan-crate selection

PR #12920 completed all 41 terminal acceptable checks. GitHub reported the
reviewed branch clean and mergeable, so the loop enabled squash auto-merge.
GitHub merged reviewed head
`ee7306c67eaf421076d821f3742fe81604860ed2` as
`47c7ef94e31558fb5b32d0345009513a698f0f0a` at
`2026-08-26T02:18:52Z` without a manual merge command.

The collision-checked exact-main schema-3 report remains structurally
unchanged: 15 established lanes, 1,373 implementation identities, 4,566
package slots, 175 high-consensus packages with 276 missing slots, 122
identities in five to nine lanes with 926 gaps, 166 identities in two to four
lanes with 2,087 gaps, 910 singletons with 12,740 gaps, and 721 Rust
singletons. Canonical collisions and unknown language buckets remain zero.
OCaml remains correctly emerging at zero packages.

The intervening Tamil ductus and ADJ vowel-team changes are curriculum or
adjacent automation only. Live ALGOL, JV02, and HTML-parser work remains inside
its existing owners. The commit and live-PR audits therefore found no newly
unowned eligible portable gap. PRs #12149 and #12162 still overlap the Go
entry-point and validator surfaces required by the OCaml process-free core, so
that strategically important tranche remains collision-unsafe.

The dependency/leverage pass selects
`build-tool-elixir-orphan-crate-validation-conformance` on branch
`codex/build-tool-elixir-orphan-crate-validation-conformance`. Its neutral
orphan corpus and Elixir tracked-artifact substrate are merged. Each remaining
orphan consumer unlocks the same completion umbrella, so breadth breaks the
tie: Elixir covers 279 reported packages versus Lua 252, Perl 251, Haskell
204, and Swift 161. All twelve live PRs have zero exact overlap with the six
expected Elixir validator, test, documentation, state, and roadmap paths. The
target local branch, remote branch, and prior PR were absent before creating
the fresh exact-main worktree.

### Post-#12933 refresh and Lua orphan-crate selection

PR #12933 completed all 41 terminal acceptable checks. GitHub reported the
reviewed branch clean and mergeable, so the loop enabled squash auto-merge.
GitHub merged reviewed head
`f0097f2a36d40d312b4d43687e8c92c7c4c63681` as
`e89a78ee93e7e6750a7362fd29d7a1cac10f002b` at
`2026-08-26T03:30:44Z` without a manual merge command.

The collision-checked exact-main schema-3 report remains structurally
unchanged: 15 established lanes, 1,373 implementation identities, 4,566
package slots, 175 high-consensus packages with 276 missing slots, 122
identities in five to nine lanes with 926 gaps, 166 identities in two to four
lanes with 2,087 gaps, 910 singletons with 12,740 gaps, and 721 Rust
singletons. Canonical collisions and unknown language buckets remain zero.
OCaml remains correctly emerging at zero packages.

Every intervening change is classified before selection. Merged PR #12928
broadens the existing Mermaid Gantt owner with inclusive and exclusive date
ranges, weekend boundaries, axis date formatting, tick intervals, and
deterministic calendar geometry and paint. Spanish A1 and ADJ sedimentary-rock
changes are curriculum or adjacent automation only and create no portable
package identity.

Open PR #12929 exposes a newly unowned Rust-only WebAssembly exception family,
so the refresh registers
`wasm-exceptions-tag-throw-language-neutral-conformance` for tag index spaces,
tag types, `throw`, `try_table`, uncaught-exception classification, catch-table
shapes, stable diagnostics, and bounds. A dependent, selection-blocked
`wasm-exceptions-tag-throw-established-lane-parity` owns the applicable-lane
completion work. Generic WAST parsing, conformance reporting, official-corpus
provenance, fetching, NOTICE, baseline writes, and host authority remain with
their existing owners.

The dependency/leverage pass selects
`build-tool-lua-orphan-crate-validation-conformance` on branch
`codex/build-tool-lua-orphan-crate-validation-conformance`. Its neutral orphan
corpus and Lua tracked-artifact substrate are merged. Each remaining orphan
consumer unlocks the same completion umbrella, so breadth breaks the tie: Lua
covers 252 reported packages versus Perl 251, Haskell 204, and Swift 161. All
ten live PRs have zero exact overlap with the six expected Lua validator, test,
documentation, state, and roadmap paths. The target local branch, remote
branch, and prior PR were absent before creating the fresh exact-main
worktree. OCaml's process-free core remains collision-unsafe while PRs #12149
and #12162 own required Go entry-point and validator surfaces.

Before publication, the branch rebased without conflict onto exact main
`b4dbb28496f299f335fdda14021e61ab5d126614`. Intervening ALGOL mixed numeric
folding, Japanese hiragana ductus, ADJ shipment bookkeeping, and HTML unmatched
sectioning diagnostics stay within existing owners. Merged PR #12936 extends
the existing Mermaid Gantt owner with explicit axis-end dates and
inclusive/exclusive end behavior across the same parser, IR, temporal-layout,
and paint roots. The refreshed collision report keeps all schema-3 counts
unchanged and retains zero collisions and unknown buckets.

The refresh also corrected the state graph's one historical lifecycle
inversion: the still-pending standalone-integrity umbrella now depends on its
merged Lua child, instead of the merged child depending on pending work. The
W21 exception owner now records all changed Wasm type, opcode, module,
validator, execution, runtime, WAST, and conformance surfaces.

### Post-#12940 refresh and Perl orphan-crate selection

PR #12940 completed all 40 terminal acceptable checks: 33 successes and seven
expected skips. GitHub reported the reviewed branch clean and mergeable, so the
loop enabled squash auto-merge. GitHub merged reviewed head
`29ee502daa1d841d3fb888b7cf8b35095193f420` as
`d0fc3d3b094249ed46b304294703cc6ec72d0d5c` at
`2026-08-26T04:37:28Z` without a manual merge command.

The collision-checked exact-main schema-3 report remains structurally
unchanged: 15 established lanes, 1,373 implementation identities, 4,566
package slots, 175 high-consensus packages with 276 missing slots, 122
identities in five to nine lanes with 926 gaps, 166 identities in two to four
lanes with 2,087 gaps, 910 singletons with 12,740 gaps, and 721 Rust
singletons. The all-reported union is 1,412. Canonical collisions and unknown
language buckets remain zero, and OCaml remains correctly emerging at zero
packages.

Every intervening change is classified before selection. Merged PR #12929 is
retained under the registered Wasm exception neutral and applicable-lane
owners. Merged PR #12941 extends the Mermaid Gantt owner with multi-source
`after`/`until` ranges, validated dependency lists, duplicate, unknown, and
cyclic dependency rejection, fixed-point scheduling, and resolved-range
geometry. Merged PRs #12927 and #12939 extend the ALGOL IIR owner with exact
mixed numeric comparisons and checked integer snapshots widened into real
targets, including transitive while dependencies and fail-closed overflow or
inexact values. Kannada independent-i work remains curriculum and generated
script data.

Merged PR #12937 exposes one newly unowned Semantic IR loop-control family.
The refresh therefore registers
`semantic-ir-loop-control-language-neutral-conformance` for bare
`Break`/`Continue`, `Feature::LoopControl`, nearest-loop and `ForRange`
validation, statement-flow boundaries, unlabeled-only behavior, manifest/text/
metadata representation, stable diagnostics, and resource bounds. The dependent,
selection-blocked `semantic-ir-loop-control-applicable-consumer-parity` umbrella
owns decomposition across applicable frontend, core, execution, and emission
lanes while existing frontend owners retain source-language lowering details.
This is distinct from the SIR28 system-write owner.

The dependency/leverage pass selects
`build-tool-perl-orphan-crate-validation-conformance` on branch
`codex/build-tool-perl-orphan-crate-validation-conformance`. Its neutral
orphan corpus and Perl tracked-artifact substrate are merged. Each remaining
consumer unlocks the same orphan completion umbrella, so breadth breaks the
tie: Perl covers 251 reported packages versus Haskell at 204 and Swift at 161.
All ten live open PRs have zero exact overlap with the six expected Perl
validator, test, documentation, state, and roadmap paths. The target local
branch, remote branch, and prior PR were absent before creating the fresh
exact-main worktree. OCaml's process-free core remains collision-unsafe while
PRs #12149 and #12162 own required Go entry-point and validator surfaces.

After lifecycle reconciliation, new-owner registration, and selection, the
state graph contains 517 unique owners and 778 complete acyclic edges: 147
merged, 369 pending, and exactly one pr-open owner, with active parity PR
#12958 and no merged-to-unmerged dependency edge.

Before publication, the branch rebased four times without conflict, most
recently onto exact main `53c7c6054f9cd93378726f2348a92ed6514c5948`. PRs
#12944 and #12949 extend the shared script-ductus authored-data owner; PRs
#12946 and #12951 extend the deterministic HTML frontend owner; PR #12947
supplies the first JavaScript
emitter child under the SIR16 loop-control consumer umbrella; and PR #12948 is
Language Ladder program-only batching. PR #12917 extends the existing
TypeScript `human-language-data` singleton classification and leaves its writer
shell in the separate filesystem-authority review. PR #12952 extends the
Mermaid Gantt neutral and lane owners with pinned implicit IDs and sequential
tasks. PR #12950 extends the ALGOL IIR owners with exact subtractive-positive-
zero selector identity. PR #12945 adds Chinese curriculum and generated data
without changing package source behavior. Merged PR #12943 adds Spanish A1
curriculum and generated human-language-data without new package behavior.
PR #12949 specifically adds sourced Malayalam chillu RR as one zero-lift
three-movement arch, loop, and hook run
across the existing curriculum, human-language-data, script-ductus, and
Language Ladder consumers.

None of those eleven commits touches a selected Perl path or changes a package
identity, BUILD marker, build-tool marker, or dependency surface. All eight
live open PRs have zero exact overlap with this six-path tranche. The post-rebase
schema-3 report remains 15 established lanes, 1,373 identities, 4,566 slots,
175 high-consensus packages with 276 gaps, 910 singletons with 12,740 gaps,
721 Rust singletons, zero collisions, and zero unknown buckets. The 517-owner,
778-edge state graph remains complete and acyclic with exactly one pr-open
item.

The live-PR audit also classifies four unmerged surfaces before publication.
PR #12955 remains inside the existing ALGOL IIR neutral and applicable-lane
owners with additive negative-real-zero selector identity. PR #12954 exposes a
distinct W22 contract that W21 deliberately excluded, so the state registers a
neutral same-instance `catch`/`catch_all` matching owner depending on the W21
neutral tag/throw contract, plus an applicable-lane umbrella depending on both
that W22 contract and the W21 lane substrate. Generic inert `exnref` parsing,
official WAST reporting and baseline data, and native corpus-fetch authority
remain with the existing WAST, conformance, and host-authority owners.
Open PR #12956 likewise remains in the shared script-ductus authored-data owner
with sourced Persian shin geometry and provenance, while PR #12957 remains in
the Mermaid Gantt neutral and lane owners with repeatable status tags, vertical
markers, interaction bounds, and deterministic paint projection.

Pinned Strawberry Perl 5.38.2 validation passes all 14 package test files and
136 tests. `Validator.pm` reaches 100% statement and subroutine coverage and
83.5% total coverage; the severity-5 Perl::Critic result has no regression
against exact main. The neutral corpus validates 111 cases and 269 files, all
201 build-tool conformance tests pass with 23 expected Windows skips, and the
canonical Go oracle passes module verification, test, vet, and trimpath build.
A forced Perl dry plan evaluates 45 Starlark files, discovers 258 packages,
preserves the five-entry orphan ledger, and reports all 258 as WOULD-BUILD.
Independent exact-head contract and security reviews are clean after explicit
fail-closed regressions for malformed exemption reasons and Perl-internal
surrogate or above-Unicode code points.

Ready-for-review PR #12958 opened from exact independently reviewed head
`ce9b6c85715a1ee268020477096d718a9f8093c0` after a normal first push from
exact main `53c7c6054f9cd93378726f2348a92ed6514c5948`. GitHub reports the PR
open, non-draft, and mergeable. Required checks are queued or in progress, so
auto-merge remains disabled until every check is terminal and acceptable and
GitHub reports no conflict.

### Post-#12958 refresh and Haskell orphan-crate selection

PR #12958 completed all 40 terminal acceptable checks: 33 succeeded and seven
were expected skips. GitHub reported reviewed head
`e6f2d31009ce4e0aae639a5951791853f3b0c93c` clean and mergeable, so the loop
enabled squash auto-merge. GitHub merged it as
`a0cb099a58f58ee619e909fed6a42720a704d21f` at
`2026-08-26T06:43:58Z` without a manual merge command.

The collision-checked exact-main schema-3 report, refreshed after clean rebases,
most recently onto `773ecf8b1b3485ef68c428b892c8e4a3d3d81f4b`, remains structurally
unchanged:
15 established lanes, 1,373 implementation identities, 4,566 package slots,
175 high-consensus packages with 276 missing slots, 122 identities in five to
nine lanes with 926 gaps, 166 identities in two to four lanes with 2,087 gaps,
910 singletons with 12,740 gaps, and 721 Rust singletons. The all-reported
union remains 1,412. Canonical collisions and unknown language buckets remain
zero, and OCaml remains correctly emerging at zero packages.

Every intervening change is classified before selection. Merged PR #12954 is
owned by the registered W22 catch-matching neutral and applicable-lane owners
plus the existing WAST, conformance, provenance, and host-authority owners.
Merged PR #12955 remains inside the ALGOL IIR neutral and lane owners. Merged
PR #12956 remains curriculum and shared script-ductus authored data. Merged PR
#12957 remains inside the Mermaid Gantt neutral and lane owners. Merged PR
#12959 extends the existing deterministic HTML frontend owner with unmatched
disclosure-end diagnostic positions. Merged PR #12960 remains inside the ALGOL
IIR owners; merged PR #12961 is curriculum and generated `human-language-data`
under the existing singleton classification; merged PR #12962 remains inside
the Mermaid Gantt neutral and lane owners; and merged PR #12965 remains inside
the script-ductus neutral and lane owners plus existing curriculum and
`human-language-data` consumers. None adds or removes a package identity or
BUILD/build-tool marker, and no newly unowned eligible portable gap remains.

The live-PR audit adds one newly discovered owner pair before publication.
PR #12966's Rust W23 slice introduces cross-instance Wasm tag identity,
identity-based `catch`, and unconditional cross-instance `catch_all`; W22
explicitly excluded that behavior. The state now registers a neutral W23
contract depending on the W22 neutral contract and a selection-blocked lane
umbrella depending on both neutral W23 and lane W22. PR #12963 remains inside
the JV02 portable/lane and Semantic-IR loop-control consumer owners; its
skip-on-missing-Node execution checks broaden the existing extra-CI
toolchain-declaration owners. PR #12964 remains inside the deterministic HTML
frontend owner. PR #12968 remains inside the human-language-data generator
filesystem-authority and human-language-books untrusted-input host-authority
owners. None changes a package identity or BUILD/build-tool marker.

Before the final rebase, PR #12964 merged as `68199d1105` inside that HTML
owner, and PR #12963 merged as `6e425d80b4` inside the JV02 portable/lane,
Semantic-IR loop-control consumer, and extra-CI toolchain-declaration owners.
The three new live heads are also already owned: #12971 is ALGOL IIR, #12972
is deterministic HTML frontend behavior, and #12973 is script-ductus plus
curriculum and `human-language-data` work. None changes a package identity or
BUILD/build-tool marker.

PR #12966 then merged as `773ecf8b1b` without changing the inventory. Its Rust
W23 reference behavior remains classified by the newly registered
cross-instance tag-identity neutral and lane owners plus the existing Wasm
conformance and host-authority owners.

The dependency/leverage pass selects
`build-tool-haskell-orphan-crate-validation-conformance` on branch
`codex/build-tool-haskell-orphan-crate-validation-conformance`. Its neutral
orphan corpus and Haskell tracked-artifact substrate are merged. Haskell is the
widest remaining orphan consumer at 204 reported packages versus Swift at 161;
both advance the same completion umbrella. All eight live PRs have zero exact
overlap with the expected Haskell validator, tests, README, changelog, state,
and roadmap surfaces. The target local branch, remote branch, and prior PR were
absent before the fresh exact-main worktree was created. The higher-descendant
extra-CI corpus and strategic OCaml process-free substrate remain
collision-unsafe while PRs #12149 and #12162 own their required Go entry-point
and validator surfaces. After lifecycle reconciliation and selection, the
graph contains 519 unique owners and 781 complete acyclic edges: 148 merged,
370 pending, and exactly one in-progress owner, with no active parity PR.

The Haskell implementation is one pure typed snapshot adapter and consumes all
four neutral orphan-crate fixtures. It preserves exact artifact filtering,
component-aware ancestor BUILD coverage and filename rank, nearer-empty
nonmasking, NFC and Unicode 17 full-fold identities, Windows reserved names,
exemption precedence and staleness, Python-compatible blank reasons, scalar
ordering, canonical ASCII JSON details, fixed hostile-input redaction, and
active PENDING counts without adding filesystem, Git, process, environment,
network, credential, or link authority. Independent review caught two
pre-publication edge cases: scalar limits now use bounded spine checks that do
not force hostile lazy tails, and U+007F is escaped as `\\u007f` for
Python-compatible canonical ordering.

Exact-main GHC 9.4.8 validation passes 55 examples with zero failures through
direct Cabal, literal `BUILD`, and Windows `BUILD_windows` entry points;
`cabal check` and `cabal build all` also pass. Excluding generated Unicode
tables and test modules, HPC reports 76% expressions, 69% alternatives, 86%
local declarations, and 57% top-level declarations. Installed HLint 3.8
reports the same nine inherited hints as exact main and zero new hints. The
neutral corpus validates 111 cases and 269 files, all 201 conformance tests
pass with 23 expected Windows skips, and the execution contract validates.
The Go build tool passes module verification, all tests, vet, and trimpath
build. A forced Haskell dry plan evaluates 45 Starlark BUILD files, discovers
207 packages, validates the five-entry orphan ledger, and reports all 207
`WOULD-BUILD`. The final report and 519-owner/781-edge graph remain clean.

Ready-for-review PR #12974 opened from exact independently reviewed head
`67f8d42a66c44e5ddda09d3eef091effb06e2d1d` after a normal first push from
exact main `773ecf8b1b3485ef68c428b892c8e4a3d3d81f4b`. After later review fixes,
all 40 checks on final head `47be7c15433a3548a3e36fa1da17a30f9af4c4a0`
reached terminal acceptable conclusions: 33 succeeded and seven were expected
skips. GitHub reported the branch clean and mergeable, so the loop enabled
squash auto-merge; GitHub merged it as `07eebd8125799553c3df6e97494dd437b63977be`
at 2026-08-26T08:11:54Z without a manual merge command.

The post-merge collision-checked schema-3 inventory remains structurally
unchanged at 15 established lanes, 1,373 implementation identities, 4,566
package slots, and 1,412 all-reported identities. The breadth buckets remain
175 identities in ten to fifteen lanes with 276 missing slots, 122 in five to
nine with 926 gaps, 166 in two to four with 2,087 gaps, and 910 singletons with
12,740 gaps, including 721 Rust singletons. Canonical collisions and unknown
language buckets remain zero, and OCaml remains emerging at zero packages.
Merged PRs #12968, #12971, and #12972 stay within their registered
human-language data/host-authority, ALGOL IIR, and deterministic HTML frontend
owners. Live PRs #12973 and #12976 are curriculum or generated
`human-language-data` work, while #12977 is ALGOL IIR. None creates a package
identity, BUILD marker, or newly eligible unowned parity gap.

The dependency/leverage pass therefore selects
`build-tool-swift-orphan-crate-validation-conformance` on branch
`codex/build-tool-swift-orphan-crate-validation-conformance`. Its neutral
orphan corpus, exact `dist-newstyle` exclusion, and tracked-artifact substrate
are merged. Swift covers 161 reported packages and is now the sole unfinished
dependency of the orphan-validator completion umbrella. All eight live PRs
have zero exact overlap with the expected Swift validator, tests, README,
changelog, state, and roadmap surfaces. After lifecycle reconciliation and
selection, the graph contains 519 unique owners and 781 complete acyclic edges:
149 merged, 369 pending, and exactly one in-progress owner, with no active
parity PR. The higher-descendant extra-CI corpus and strategic OCaml
process-free substrate remain collision-unsafe while PRs #12149 and #12162 own
their required Go entry-point and validator surfaces.

Before Swift implementation began, `origin/main` advanced to
`3c442dcf16c3e0ec551b86f39d928feda6893078`; the one-commit selection branch
rebased cleanly onto that exact revision. Newly merged #12975 remains inside
the Mermaid Gantt neutral and Rust lane owners, #12973 remains inside the
script-ductus plus curriculum and `human-language-data` owners, and #12976
remains inside curriculum, generated-book, and `human-language-data` owners.
None changes a package identity, BUILD marker, or selected Swift path. The
refreshed schema-3 inventory and reporter's ten focused tests remain clean and
structurally unchanged: 1,373 established implementation identities, 4,566
package slots, 1,412 all-reported identities, zero collisions, and zero unknown
buckets. No newly eligible unowned gap displaced the selected Swift validator.

The branch rebased cleanly once more during validation onto exact `origin/main`
`87c2f6564c0944d8fd47fccd041b1a9ba6e4b801`. Intervening #12977 remains
inside the ALGOL IIR neutral and Rust lane owners, and #12978 remains inside
the deterministic HTML frontend owner. Neither changes a package identity,
BUILD marker, or selected Swift path.

The Swift implementation consumes all four neutral orphan fixtures through a
pure caller-bounded snapshot API. Focused tests cover exact artifact names,
direct and ancestor runnable coverage, nearer-empty nonmasking, closest-empty
BUILD rank, path redaction and scalar bounds, Python blank reasons, NFC
full-fold duplicate reservation before field precedence, stale precedence,
pending counts, and Python-compatible ASCII JSON ordering for DEL, accented,
and supplementary-plane details. A test-first compile failed on the missing
API before implementation. Swift 6.3.3 then passes all 40 package tests in
normal, parallel, coverage, literal Unix BUILD, and exact Windows BUILD-content
modes. `Validator.swift` reaches 98.21% line and 94.69% function coverage; the
release build, package description, and zero-external-dependency graph pass.

The neutral corpus validates 111 cases and 269 files, its execution contract
validates, and all 201 conformance tests pass with 23 expected platform skips.
The Go oracle passes module verification, all tests, vet, and trimpath build;
its real forced Swift dry plan evaluates 45 Starlark BUILD files, validates the
five reviewed orphan exemptions, discovers 165 packages, and reports all 165
as `WOULD-BUILD`. Reporter tests pass 10 of 10, the inventory remains collision
clean, and the 519-owner/781-edge graph remains unique, complete, acyclic, and
free of merged-to-unmerged edges at 149 merged, 369 pending, and one
in-progress item. Diff hygiene, credential-pattern scanning, dependency scope,
and process/filesystem/environment/network authority review pass. Swift 6.3.3
strict formatting remains an inherited baseline-red gate because the repository
has no swift-format configuration; the exact-main versions of the three touched
Swift files already produce 1,021 diagnostics, so unrelated formatter churn was
not introduced.

The final live-PR audit registers the newly unowned W24 WebAssembly exception
slice before publication. PR #12981 adds real abstract `exnref` handles,
`catch_ref`, `catch_all_ref`, `throw_ref`, null-reference trapping, corrected
blocktype encoding, and the pinned `throw_ref.wast` corpus; W21 through W23
explicitly stop short of that behavior. A new neutral W24 owner therefore
depends on W23 identity conformance, while its selection-blocked established-
lane umbrella depends on both neutral W24 and the W23 lane umbrella. This grows
the graph to 521 owners and 784 edges: 149 merged, 371 pending, and exactly one
in-progress owner. The graph remains unique, complete, acyclic, and free of
merged-to-unmerged edges, and W24 does not displace the dependency-ready Swift
validator.

The other live heads remain classified by existing owners. PR #12980 extends
the Mosaic generated-wrapper review with target-specific XAML lowering;
#12982 extends script-ductus plus curriculum and `human-language-data`; #12983
extends Mermaid Gantt portable and lane conformance; #12985 is Gujarati
curriculum, generated-book, and `human-language-data` work; and the spec-only
#12986 is human-language curriculum planning. The older live #12165, #12162,
#12149, and #7821 remain inside their registered authority, build, or dependency
owners. All ten live PRs have zero exact overlap with the seven Swift tranche
paths.

Immediately before publication, the complete three-commit branch rebased
without conflict onto exact `origin/main`
`4bc1649d28948a6daa609ef1f00239063a10f12b`. PR #12981 merged as
`6604df1f9d` inside the newly registered W24 pair and existing Wasm owners;
#12982 merged as `9591aa6e44` inside script-ductus, curriculum, and
`human-language-data`; #12980 merged as `72ccc71aa8` inside the Mosaic
generated-wrapper review; and #12983 merged as `4bc1649d28` inside Mermaid
Gantt neutral and lane owners. The regenerated collision-checked inventory and
all ten reporter tests remain unchanged and clean. The new live #12987 remains
inside the ALGOL IIR neutral and Rust lane owners. The resulting seven live PRs
still have zero exact overlap with the seven Swift tranche paths.

Ready-for-review PR #12989 opened from clean independently reviewed head
`810ec4bcec5d87a564f90cdb39e60ddd1b291c0d` after a normal first push from
exact main `4bc1649d28948a6daa609ef1f00239063a10f12b`. The target remote branch and
prior PR were absent before publication, and all seven other live PRs had zero
exact overlap across the seven intended paths. GitHub reports the PR non-draft
with required checks queued or in progress, so auto-merge remains disabled and
the loop is monitor-only.

Final reviewed head `56e153b0a4fdc70089e52509f2cbbf49eab72639`
completed all 43 reported checks with 36 successes and seven expected skips.
GitHub reported the branch clean and mergeable, so the loop enabled squash
auto-merge; GitHub merged PR #12989 as
`1f3b85a73dde63076f584a0558d4d9019929362b` at 2026-08-26T09:44:31Z
without a manual merge command.

Before the selection commit, the fresh branch incorporates exact
`origin/main` `1f62ce37805fa44a2bc00a84758ff1e11f35d7d3`; late merged #12991
remains inside the existing HTML frontend owner and changes no package identity
or BUILD/build-tool marker. The exact-main collision-checked schema-3 inventory
remains unchanged at 15
established lanes, 1,373 implementation identities, 4,566 package slots, and
1,412 all-reported identities. The breadth buckets remain 175 identities in
ten to fifteen lanes with 276 gaps, 122 in five to nine with 926 gaps, 166 in
two to four with 2,087 gaps, and 910 singletons with 12,740 gaps, including 721
Rust singletons. Collisions and unknown buckets remain zero, and OCaml remains
emerging at zero packages. Merged #12986 is a human-language planning spec,
#12987 extends ALGOL IIR, #12988 extends Malayalam script and data, and #12990
extends Mermaid Gantt rendering. The eight live heads are already classified
inside Persian script-ductus, Gujarati curriculum, Mosaic generated-wrapper, ALGOL
IIR, Chief native build evidence, extra-CI or build-file authority, Swift and
Go overlap, or dependency owners; no new package identity or unowned portable
gap appears.

The dependency/leverage pass selects `canonical-cbor-jvm-lane-parity` on
branch `codex/canonical-cbor-jvm-lane-parity`. Its CBR01 neutral corpus and
three independent Rust, C, and C++ references are merged. The paired JVM slice
is a bounded shared-toolchain review unit that closes two explicitly prioritized
established lanes and unlocks the same fourteen unfinished canonical-CBOR
descendants as each single-lane sibling. All eight live PRs have zero exact
overlap with the expected Java and Kotlin canonical-CBOR, state, and roadmap
paths. The higher-descendant extra-CI corpus and strategic OCaml process-free
core remain collision-unsafe while PRs #12162 and #12149 own their required Go
validator and entry-point surfaces. After lifecycle reconciliation and
selection, the graph contains 521 owners and 784 complete acyclic edges: 150
merged, 370 pending, and exactly one in-progress owner, with no active parity
PR.

### JVM canonical-CBOR implementation and final ownership refresh

The selected `canonical-cbor-jvm-lane-parity` tranche adds independent Java
and Kotlin implementations of the closed CBR01 value model, checked canonical
encoder, and strict decoder. Both lanes consume all 55 shared
`canonical-cbor-v1` operations and expose the same 14 stable error identifiers.
They preserve the full unsigned 64-bit domain, reject non-minimal and
non-canonical inputs, order map keys by encoded length and unsigned bytes,
reject duplicate encoded keys, cap nesting at 128, and publish checked output
atomically under the 1,048,576-byte item limit. A test-first compile failed on
the missing APIs before the implementations were added.

Independent correctness and security review found two pre-publication boundary
defects and one documentation defect. Both value models now reject every
unpaired UTF-16 surrogate while retaining valid supplementary pairs. Both
encoders preflight UTF-8 length before payload allocation, reject impossible
map entry counts, and bound cumulative retained encoded-key bytes before map
sorting. The Kotlin dependency description now names its exact runtime closure:
Kotlin stdlib 2.1.20 plus transitive JetBrains annotations 13.0; Java has an
empty production runtime classpath. Production code remains pure in-memory
computation with empty capability profiles and no filesystem, process,
environment, network, credential, or link authority.

Before final publication the branch rebased cleanly onto exact `origin/main`
`2d7bfa99226e341dee63e997d22dce50a9b57cf2`.
Intervening PRs #12985 and #12996 remain inside existing human-language,
generated-data, script-ductus, and singleton-classification owners; #12993
remains inside the Mosaic generated-wrapper review; and #12995 remains inside
the ALGOL IIR neutral and applicable-lane owners. None adds a package identity,
BUILD marker, or selected-path overlap.

The later #13002 and #13001 merges remain inside existing human-language
curriculum, generated-data, and singleton-classification owners. They likewise
add no implementation package identity, BUILD marker, or selected-path overlap.

Post-rebase Java and Kotlin `gradle clean check` runs pass their strict compiler,
55-case corpus, and 95% JaCoCo gates. Java covers 268 of 276 production lines
(97.10%); Kotlin covers 224 of 228 (98.25%). Literal Unix BUILD and exact
Windows BUILD-content entry points pass in both lanes. The neutral fixture,
capability-taxonomy, and parity-reporter suites pass 21 tests plus 720 subtests.
The Go build tool passes all tests, vet, and trimpath compilation; its exact
dry plan evaluates 45 Starlark BUILD files, discovers 5,075 packages, validates
the five-entry orphan ledger, and selects exactly the two JVM packages while
skipping 5,073. Diff hygiene, dependency, credential-pattern, authority, and
independent security reviews pass.

The prospective merged-tree schema-3 inventory remains collision-clean at 15
established lanes, 1,373 implementation identities, 4,568 package slots, and
1,412 all-reported identities. Java rises to 131 packages and Kotlin to 130.
The breadth buckets become 175 identities in ten to fifteen lanes with 276
gaps, 122 in five to nine with 926 gaps, 167 in two to four with 2,099 gaps,
and 909 singletons with 12,726 gaps, including 720 Rust singletons. Canonical
collisions and unknown language buckets remain zero; OCaml remains emerging at
zero packages. Global `last_inventory` intentionally stays tied to exact-main's
pre-tranche 4,566-slot inventory until the PR merges.

The final live audit also registers two newly unowned owners exposed by open
PR #13005 before publication. The neutral Semantic IR emitted-identifier owner
covers reserved-marker separation, distinct verbatim and escaped tags,
underscore escaping, fixed-width Unicode scalar encoding, keyword validity,
collision cases, bounds, and deterministic authority-free behavior. Its
selection-blocked applicable-consumer umbrella follows after that corpus and
uses the seven Rust semantic-ir-to-* backends as reference evidence rather than
as a parity claim. The resulting graph contains 523 unique owners and 785
complete acyclic edges: 150 merged, 372 pending, and exactly one in-progress,
with no active parity PR before publication.

The complete final audit covers all eight open PRs and finds zero exact overlap
with the 26 intended branch paths. PR #13009 remains inside script-ductus plus
curriculum and `human-language-data`; PR #13011 is test-only Mosaic generated-
wrapper and artifact-builder degradation evidence inside the existing Mosaic
review; and PR #13012 adds deterministic source positions to unmatched HTML
block-container diagnostics inside `html-frontend-portable-conformance`. None
requires another package-parity owner or edge.

Ready-for-review PR #13013 opened from clean independently reviewed head
`a057ff32f8630ca6dd86445410bce81d01e81f4c` after a normal first push from
exact main `2d7bfa99226e341dee63e997d22dce50a9b57cf2`. The target remote
branch and prior PR were absent, and all eight other open PRs had zero exact
overlap. GitHub reports the PR non-draft and mergeable with checks queued or in
progress, so auto-merge remains disabled while the loop monitors CI.

## Post-#13013 Refresh and .NET Canonical-CBOR Selection

Final reviewed head `cef8a8de39f067dd1d1ee3c95636fe3903e16950`
completed all 40 reported checks with 33 successes and seven expected skips.
GitHub reported the branch clean and mergeable, so the loop enabled squash
auto-merge; GitHub merged PR #13013 as
`1d0c64a9ad9e0e8d9652d605d731de823b21fcc1` at 2026-08-26T11:06:51Z
without a manual merge command. The clean merged-head worktree was then
removed, reclaiming 724,619,264 bytes while preserving active, dirty,
detached, open-PR, and ambiguous checkouts.

The fresh exact-main schema-3 report remains collision-clean at 15 established
lanes and 1,373 implementation identities, while the merged JVM packages raise
the implementation slot count from 4,566 to 4,568. The report contains 1,412
all-reported identities. The breadth bands are now 175 identities in ten to
fifteen lanes with 276 gaps, 122 in five to nine with 926 gaps, 167 in two to
four with 2,099 gaps, and 909 singletons with 12,726 gaps, including 720 Rust
singletons. Java reports 131 packages and Kotlin 130. Canonical collisions and
unknown language buckets remain zero. OCaml remains correctly emerging at zero
packages and therefore does not yet widen the established-language
denominator. The reporter's ten focused tests pass on the exact merge.

The ownership refresh classifies every live head before selection. PR #13014
fits `storage-fs-persistent-revision-floor-native-authority-review`; #13017 is
human-language curriculum and generated-book work; #13018 remains inside the
script-ductus, curriculum, `human-language-data`, and Language Ladder owners;
and #13011 remains inside the Mosaic generated-wrapper review. The older
#12165, #12162, #12149, and #7821 remain within their registered Chief,
build-file/build-tool, Swift, or dependency owners.

Open PR #13015 exposes one newly unowned portable family. Its Rust W25 slice
adds memory64 limits, encodings, validation, execution, runtime, WAT/WAST, and
official-corpus evidence across existing package identities. The new
`wasm-memory64-language-neutral-conformance` owner therefore binds exact 32-
versus 64-bit memory index behavior, `u64` limits and memarg offsets, binary
flags, scalar load/store plus size/grow stack typing, active-data offsets,
checked effective-address arithmetic, `min <= max`, the `2^48` specification
ceiling, a bounded practical allocation ceiling, stable errors, and resource
rejection. Its selection-blocked
`wasm-memory64-established-lane-parity` child requires decomposition after the
neutral corpus closes. The pair reuses existing parser, portable-conformance,
official-corpus, provenance, fetch, NOTICE, baseline, and native-host owners;
table64 and imports, bulk memory, SIMD, atomics, and the wider `*64.wast` family
remain explicitly outside this first slice.

The dependency/leverage pass selects `canonical-cbor-dotnet-lane-parity` on
branch `codex/canonical-cbor-dotnet-lane-parity`. The CBR01 neutral corpus and
independent Rust, C, C++, Java, and Kotlin references are merged. The paired
.NET tranche is the strongest collision-safe canonical-CBOR unit because one
shared toolchain review closes two established lanes at once and directly
advances the durable all-language denominator. All nine live PRs have zero
exact overlap with the expected new C# and F# package directories or parity
state and roadmap. The target local branch, remote branch, and prior PR were
absent before the fresh exact-main branch was created. The extra-CI corpus and
strategic OCaml process-free core remain collision-unsafe while PRs #12162 and
#12149 own their required Go validator and entry-point surfaces. After the W25
registration and .NET selection, the graph contains 525 unique owners and 786
complete acyclic edges: 151 merged, 373 pending, and exactly one in-progress
owner, with no active parity PR.

The selected implementation now adds independent dependency-free C# and F#
packages for the complete closed `canonical-cbor-v1` corpus. Each native engine
supports the full CBR01 data model, shortest-form definite decoding, exact
single-item consumption, canonical length-first unsigned map-key ordering,
duplicate and out-of-order key rejection, strict Unicode, bounded nesting and
encoder output, defensive ownership, stable payload-blind errors, and
transactional `MemoryStream` append at destination length. The F# public
facade exposes safe typed views over its private value union. Neither engine
shares production code or adds filesystem, process, environment, credential,
clock, random, or network authority.

All 55 neutral cases plus adversarial direct probes pass in both lanes through
the literal Unix and Windows BUILD fronts: eight tests per package. C# reaches
98.19% line, 87.15% branch, and 100% method coverage; F# reaches 98.08% line,
95.47% branch, and 100% method coverage. Warning-free .NET 9 release builds and
release packs, capability and OCaml-lock suites, the Go build tool's tests,
vet, module verification, trimpath build, full repository plan, and forced C#
and F# dry plans all pass. The forced plans report 201 and 200 WOULD-BUILD
packages respectively. Reporter, collision, strict-JSON, dependency,
credential, production-authority, formatting, packaging, diff, and independent
security reviews are clean. The branch inventory therefore adds exactly the
two selected slots: 1,373 identities, 4,570 slots, 1,412 all-reported
identities, breadth bands 175/276, 123/936, 166/2,087, and 909/12,726, 720 Rust
singletons, zero collisions, and zero unknown buckets. Global `last_inventory`
remains pinned to the exact merged main baseline until publication merges.
The three branch commits then rebased conflict-free over all intervening W25
memory64, curriculum, Java semantic-IR, storage-fs, and HTML-parser changes
through exact `origin/main`
`bb6ae0c70e8000a28b9eaa0d6fc06c821bfbb43b`; all six current open PRs remain
exact-path disjoint from the selected package, state, and roadmap surfaces.

## Post-#13026 Refresh and TypeScript Canonical-CBOR Selection

Reviewed head `00a5c729c3ea0fdbba8d2c2063b5c3badd8ca658`
completed all 40 reported checks with terminal acceptable conclusions. GitHub
reported the branch clean and mergeable, so the loop enabled squash
auto-merge; GitHub merged PR #13026 as
`ce4c3aa0c522e354653a2a4205b8dc5534c40f89` at 2026-08-26T12:25:25Z
without a manual merge command. The exact merged-head worktree was clean and
removed afterward, reclaiming 578,113,073 bytes while preserving dirty,
detached, active-PR, and ambiguous checkouts.

The exact-main schema-3 report remains collision-clean at 15 established lanes,
1,373 implementation identities, and 1,412 all-reported identities. The C# and
F# packages raise implementation slots from 4,568 to 4,570 and move
`canonical-cbor` from the two-to-four band into five-to-nine. The breadth bands
are now 175 identities with 276 gaps in ten to fifteen lanes, 123 with 936 gaps
in five to nine, 166 with 2,087 gaps in two to four, and 909 singletons with
12,726 gaps, including 720 Rust singletons. C# reports 199 packages, F# 198,
and TypeScript 446. Canonical collisions and unknown language buckets remain
zero. C and C++ remain emerging at 158 and 142 packages; OCaml remains
correctly emerging at zero packages. All ten reporter tests pass.

Every live head is classified before selection. PR #13028 extends the existing
Rust singleton and `lang-aot` owner. PR #13027 remains inside script-ductus,
human-language-data, curriculum, and generator-authority owners. PR #13011's
five-backend native-complete degradation evidence remains inside the
selection-blocked Mosaic generated-wrapper review and does not create a
portable package or build-tool contract. PRs #12162 and #12149 temporarily
overlap Go validator and entry-point surfaces already owned by
`build-tool-go-oracle` plus the separate Swift cowsay owner; dependency-comment
parsing, `BuildContent` reconstruction, worktree `.git`-file discovery, and raw
Windows command-line behavior are now explicit in that canonical Go owner.
PR #7821 reports no live files. No open PR touches a canonical-CBOR package,
the neutral corpus or spec, parity state, or this roadmap, and no eligible
unowned gap remains.

The dependency/leverage pass selects
`canonical-cbor-typescript-lane-parity` on branch
`codex/canonical-cbor-typescript-lane-parity`. TypeScript is the largest
remaining established canonical-CBOR lane at 446 packages with 331 Rust
overlaps and zero high-consensus gaps, has the complete local Node/npm/pnpm
toolchain, and advances one of the ten remaining established-lane children
without shared production code. Its highest-risk contract points are bigint-
only full-uint64 values, fatal UTF-8 and surrogate rejection, defensive byte
and collection ownership, length-first unsigned map-key order and duplicate
identity, exact 128/129 depth, the encoder-only 1,048,576-byte cap, hostile
length preflight, exact one-item consumption, all 14 payload-blind diagnostics,
atomic append, and closed fixture grammar. The fresh exact-main worktree,
target path, branch, remote branch, and prior PR were absent before selection.
The reconciled graph remains complete and acyclic at 525 owners and 786 edges:
152 merged, 372 pending, and exactly one in-progress owner, with no active
parity PR.

### TypeScript canonical-CBOR implementation

Commit `cc6729e698cd1e7295544763d34da038e7ea0e13` adds the native
`code/packages/typescript/canonical-cbor` package without production
dependencies. Its immutable value algebra keeps the complete unsigned 64-bit
domain in `bigint`; the checked encoder stages output, orders and deduplicates
complete encoded map keys, and applies the exact depth and byte limits. The
decoder consumes exactly one item, uses fatal UTF-8, rejects hostile declared
lengths before allocation, and exposes only the fourteen static CBR01 errors.

The 55 shared cases and TypeScript adversarial coverage pass as 12 Vitest
tests. Both front doors are real: the final Unix `BUILD` passes `npm ci`,
`tsc`, and V8 coverage at 95.83% statements, 92.30% branches, 100% functions,
and 96.61% lines; the Go build tool discovers 477 TypeScript packages, resolves
exactly one changed and affected package, and executes its `BUILD_windows`
successfully with the other 476 skipped. Focused Go discovery, resolver,
validator, command-rendering, and executor suites pass, as do the ten parity
reporter tests and seven capability-taxonomy tests. npm reports zero
vulnerabilities and an empty production dependency tree.

Independent review found and drove regressions for JavaScript runtime mutation,
subclass and erased-type bypasses, replaceable null singleton state, sparse
array append capacity, unchecked decode/container inputs, allocation ordering,
and permissive generated-fixture suffixes. Private frozen storage plus
encoder-side revalidation, exact runtime boundaries, preflight checks, and
closed grammar repair every finding; the final security review reports no
remaining actionable issue. The collision-checked candidate inventory stays
at 1,373 implementation identities and 1,412 all-reported identities while
raising slots to 4,571 and reducing five-to-nine missing slots to 935, with
zero collisions and zero unknown buckets.

PR #13037 completed all 41 final checks successfully or acceptably after a
focused TypeScript-project inventory repair at reviewed head
`14c5060bfe2b53ce6003a1aa25ab1e5f9f2fb708`. GitHub reported a clean merge
state, auto-merge was enabled, and the PR merged as
`848f43c20b4d30544a3d9f9c50ae4a10c6e2b63d` without a manual merge command.

### Post-#13037 inventory and Python canonical-CBOR selection

The exact-main schema-3 report remains collision-clean at 15 established
lanes, 1,373 implementation identities, and 1,412 all-reported identities.
The TypeScript package raises implementation slots to 4,571 and advances
canonical-CBOR to six established lanes, leaving the breadth bands at 175
high-consensus identities with 276 gaps, 123 five-to-nine identities with 935
gaps, 166 two-to-four identities with 2,087 gaps, and 909 singletons with
12,726 gaps. Rust retains 720 singletons; canonical collisions and unknown
language buckets remain zero; OCaml remains emerging at zero packages. The ten
reporter tests pass.

All nine live PRs are classified under existing Mermaid Gantt, script-ductus
and human-language, Semantic IR LoopControl, Chief approval, Go build-tool,
Swift cowsay, or stale zero-file owners. None touches the Python canonical-CBOR
target, its neutral corpus or specification, parity state, or this roadmap. A
fresh breadth audit also found the `java-kotlin-high-consensus` umbrella's zstd
lead stale because zstd is already complete in all 15 lanes. The remaining
lz78, deflate, and state-machine frontier is now a bounded pending paired-JVM
child: each identity is 13/15 and missing only Java and Kotlin.

The dependency/leverage pass selects `canonical-cbor-python-lane-parity` on
branch `codex/canonical-cbor-python-lane-parity`. Python is the largest
remaining canonical-CBOR lane at 502 packages with 391 Rust overlaps. Its sole
dependency is merged; the closed 55-case CBR01 corpus and eight independent
reference implementations are available; and this zero-production-dependency
slice advances the AND-gated canonical-CBOR umbrella that unlocks three Vault
portability roots. The fresh exact-main worktree, package path, local branch,
remote branch, and prior PR were absent before selection. At selection, the
reconciled graph was complete and acyclic at 526 owners and 787 edges: 153
merged, 372 pending, and exactly one in-progress owner, with no active parity
PR.

Ready-for-review PR #13049 delivers the native Python CBR01 lane at reviewed
implementation head `e94a3d680e0c70a5d64165d6ec59fed76318e6dc` against exact
`origin/main` `5d30513d12e0ece69a083577c342184c24cf9f83`. The candidate report
raises implementation slots to 4,572, Python to 503 packages, and Rust/Python
overlaps to 392 while reducing five-to-nine missing slots to 934, with zero
collisions or unknown buckets. GitHub reports the PR open, non-draft, and
mergeable; CI and CodeQL are queued, so auto-merge remains disabled until every
required check is terminal and acceptable.

PR #13049 completed all 41 final checks successfully or acceptably at reviewed
head `18a6fe2f3923bfe33d3111e7c64de1bc6e31344c`. GitHub reported a clean
merge state, auto-merge was enabled, and the PR merged as
`c04af4ec8c9db002a651e349af726e391650c253` without a manual merge command.

### Post-#13049 inventory and paired-JVM high-consensus selection

The exact-main schema-3 report remains collision-clean at 15 established
lanes, 1,373 implementation identities, 4,572 implementation slots, and 1,412
all-reported identities. The breadth bands are 175 high-consensus identities
with 276 gaps, 123 five-to-nine identities with 934 gaps, 166 two-to-four
identities with 2,087 gaps, and 909 singletons with 12,726 gaps. Rust retains
720 singletons; canonical collisions and unknown language buckets remain zero;
OCaml remains emerging at zero packages. The expected Python canonical-CBOR
slot is the only topology change, so this refresh adds no newly discovered
owner.

Nine live PRs are classified outside this portable slice, and no other parity
PR is active. The dependency/leverage pass selects
`java-kotlin-high-consensus-lz78-deflate-state-machine` on branch
`codex/java-kotlin-lz78-deflate-state-machine-parity`. The item has no unmet
dependency, shares one JVM toolchain and cross-lane review boundary, adds six
missing implementation slots, and completes three identities that are each
already present in the other thirteen established lanes. That is the largest
bounded completion gain among the immediately ready high-consensus items and
directly advances the Java and Kotlin parity requirement. At selection, the
reconciled graph remains complete and acyclic at 526 owners and 787 edges: 154
merged, 371 pending, and exactly one in-progress owner, with no active parity
PR.

### Java/Kotlin lz78, deflate, and state-machine candidate

The paired JVM candidate adds all six selected package slots. LZ78 implements
the published token and big-endian wire vectors with dictionary and hostile
output limits. DEFLATE constructs fixed and dynamic candidates from one LZSS
stream, builds length-limited trees with package-merge, compares exact bit
costs, and emits raw RFC 1951 streams accepted by an independent JDK inflater.
State-machine covers DFA, NFA/subset conversion, minimization, PDA, and modal
behavior with defensive snapshots and explicit trace, subset, stack, and
trace-state-cell budgets.

All six Gradle suites and their JaCoCo gates pass: 34 tests total, with line
coverage from 96.25% through 99.10% in Java and 96.98% through 97.78% in
Kotlin. Both BUILD_windows command contracts pass. Shared reporter and
capability validation passes 22 tests and 744 subtests; the Go build tool passes
all packages, vet, and a trimpath build. The schema-3 candidate inventory is
collision-clean at 1,373 implementation identities and 4,578 slots, with the
high-consensus gap count reduced from 276 to 270 and lz78, deflate, and
state-machine each complete in all 15 established lanes. Independent contract,
metadata, and security reviews are clean after their findings were closed.

Ready-for-review PR #13070 publishes the candidate from reviewed head
`267c78332aa96684f0bf729a35d2957297d33767` on exact `origin/main`
`b232ffbfb8e349355a92d987209e85640e36fabb`. GitHub reports it non-draft and
mergeable. Required checks have not yet populated, so auto-merge remains
disabled and the loop is monitor-only until every check is terminal and
acceptable.

PR #13070 completed all 40 final checks at state-recording head
`b2dc18477d6b232f05af7b54417f8000b698ee3a`: 33 succeeded and seven were
expected skips, with no failures or pending work. GitHub reported a clean merge
state, auto-merge was enabled, and the PR merged as
`1e2ee726573105be54245cba117ef3099765a130` at
2026-08-26T17:32:54Z without a manual merge command. The exact merged-head
worktree was clean and removed afterward.

### Post-#13070 inventory and Dart classical-cipher selection

The exact-main schema-3 report remains collision-clean at 15 established
lanes, 1,373 implementation identities, 4,578 implementation slots, and 1,412
all-reported identities. The breadth bands are 175 high-consensus identities
with 270 gaps, 123 five-to-nine identities with 934 gaps, 166 two-to-four
identities with 2,087 gaps, and 909 singletons with 12,726 gaps. Rust retains
720 singletons; canonical collisions and unknown language buckets remain zero;
OCaml remains correctly emerging at zero packages. The six merged JVM slots
are the only package-directory topology change.

The reopened 14-of-15 frontier now contains eight exact gaps: Dart alone is
missing `atbash-cipher`, `binary-search-tree`, `fenwick-tree`,
`scytale-cipher`, `trie`, `uuid`, and `vigenere-cipher`; Swift alone is missing
`paint-vm-ascii`. All nine live open PRs have zero exact overlap with the
selected Dart package, CR01-CR03 specification, state, and roadmap surfaces.
The Swift paint item remains actively owned by PR #12149. The dependency-ready
OCaml process-free substrate is strategically important but remains
collision-unsafe while PRs #12149 and #12162 own required Go build-tool
entry-point and validator surfaces.

The dependency/leverage pass selects
`dart-current-14-of-15-atbash-scytale-vigenere` on branch
`codex/dart-classical-ciphers-parity`. Its sole scaffold dependency is merged;
all three adjacent CR01-CR03 packages are deterministic, dependency-free, and
empty-capability leaves; and every other established lane implements all
three. This one-toolchain tranche adds three slots, completes three identities
at 15/15, and reduces Dart's high-consensus backlog from 101 to 98. Haskell's
two-package event-loop/brotli tail and the single Dart trie foundation remain
the next bounded runners-up. The fresh exact-main worktree, target package
paths, descriptive local and remote branch, and prior PR were absent before
selection.

The subsequent CR01-CR03 reference audit found three pre-existing behavioral
ownership gaps before Dart implementation began. A new neutral-fixture owner
will pin the shared ASCII vectors, Vigenere insufficient-signal behavior, IC
threshold and tie-breaking, analysis bounds, and the exact long-English corpus.
Dependent Scytale conformance now owns the split among UTF-16 units, Unicode
scalars, grapheme clusters, and UTF-8 bytes plus four lanes that trim more than
the literal U+0020 padding character. Dependent Vigenere conformance owns the
Go/Python Unicode-key divergence and the Kotlin/Swift Unicode-analysis hazards.
The current Dart tranche follows the tightened portable contract—Unicode
scalars and literal-space removal for Scytale, ASCII-only Vigenere processing,
and key length 1 for insufficient analysis signal—without expanding into those
cross-lane remediations.

At selection time, the reconciled graph was complete and acyclic at 529 owners
and 789 edges: 155 merged, 373 pending, and exactly one in-progress owner, with
no active parity PR.

The Dart candidate implements the reconciled contract in specification,
tests, source, and documentation order. Atbash passes eight tests; Scytale and
Vigenere pass sixteen each. All three format and fatal-analysis gates are clean,
and LCOV reports 100% production line and function coverage: 8/8 and 3/3,
52/52 and 9/9, and 93/93 and 20/20. Production dependencies remain empty,
capability manifests remain empty and schema-valid, and no production
dependency is outdated. The Go build tool passes all packages, vet, and
trimpath compilation. Its exact Windows diff plan evaluates 45 Starlark files,
discovers 89 Dart packages, selects exactly these three independent nodes, and
the real validated execution builds all three while skipping the other 86.

The collision-checked candidate inventory adds exactly three slots to 4,581,
reduces high-consensus gaps from 270 to 267 and Dart's share from 101 to 98,
and completes Atbash, Scytale, and Vigenere at 15/15 with no collision or
unknown bucket. Parity, capability, state, strict-JSON, README-link, diff,
dependency, credential, and production-authority checks pass. Independent
contract, packaging, and security reviews are clean after redacting implicit
value-object output, pinning trailing non-space padding behavior, and making
the provisional Vigenere IC heuristic debt explicit under its neutral-fixture
owner.

Ready-for-review PR #13083 publishes the implementation from validated head
`73ca4792d31b572bd0ee7097141ea658e7658436` on exact `origin/main`
`5dcdf1c555e1300337998a705658865b47c221da`. The branch rebased cleanly over
disjoint Marathi PR #13065 and Punjabi PR #13074 curriculum changes before its
normal first push, and eleven live open PRs have zero exact changed-path
overlap. GitHub reports the PR non-draft and mergeable. Required checks are
queued, merge state is blocked only by those protections, and auto-merge
remains disabled. State is now 155 merged, 373 pending, and exactly one
`pr-open` owner across the unchanged 529-owner/789-edge acyclic graph.

PR #13083 completed all 40 final checks at state-recording head
`5beb364839614d82e3dc33a2b6321ce1f422f128`: 33 succeeded and seven were
expected skips, with no failures or pending work. GitHub reported a clean
merge state, auto-merge was enabled, and the PR merged as
`9ae79c55494056324a7518715b005ca8594c4472` at 2026-08-26T18:48:24Z without a
manual merge command. The exact merged-head worktree was removed afterward.

### Post-#13083 inventory and Dart trie selection

The exact-main schema-3 report is collision-clean at 15 established lanes,
1,373 implementation identities, 4,581 implementation slots, and 1,412
all-reported identities. The breadth bands are 175 high-consensus identities
with 267 gaps, 123 five-to-nine identities with 934 gaps, 166 two-to-four
identities with 2,087 gaps, and 909 singletons with 12,726 gaps. Rust retains
720 singletons; canonical collisions and unknown language buckets remain zero;
OCaml remains correctly emerging at zero packages. Relative to the post-#13070
snapshot, only the expected Dart Atbash, Scytale, and Vigenere slots were
added. No identity, language bucket, or unowned topology gap appeared. The
reference pass did find two semantic backlog owners before implementation:
one for a language-neutral DT13 corpus and exact key contract, followed by one
for established-lane Unicode-scalar and deterministic-order conformance.

The reopened 14-of-15 frontier now contains five exact gaps: Dart alone is
missing `binary-search-tree`, `fenwick-tree`, `trie`, and `uuid`; Swift alone
is missing `paint-vm-ascii`. Open PR #12149 continues to own the Swift package.
All seven live open PRs have zero exact changed-path overlap with the DT13
specification, the prospective Dart trie package, state, or roadmap. The
process-free OCaml substrate remains collision-unsafe while open PRs #12149 and
#12162 both own required Go build-tool entry-point and validator surfaces.

The dependency/leverage pass selects `dart-current-14-of-15-trie` on branch
`codex/dart-trie-parity`. Its Dart scaffold dependency is merged; the pure
deterministic package needs no production dependency or runtime authority; and
one slot completes DT13 at 15/15. It also establishes the package prerequisite
for the separately tracked Dart LZ78 shared-trie migration. That downstream
migration must still reconcile LZ78's byte-at-a-time cursor with DT13's
complete-string API, following the Python `TrieCursor` precedent or adapting
the consumer in its own PR. Fenwick tree is an isolated runner-up;
binary-search-tree still needs comparator, empty-result, rank, and conceptual
prerequisite reconciliation; and the unrelated Haskell event-loop/Brotli pair
has a larger authority and malformed-input surface. The classical-cipher
neutral fixture owner remains the next cross-lane conformance tranche because
it unlocks both Scytale and Vigenere remediation.

At selection time, the reconciled graph is complete and acyclic at 531 owners
and 790 edges: 156 merged, 374 pending, and exactly one in-progress owner, with
no active parity PR. The fresh worktree starts from exact
`9ae79c55494056324a7518715b005ca8594c4472`; no other registered worktree meets
the clean exact-merged-head deletion rule, so dirty, detached, open-PR, and
ambiguous worktrees remain preserved.

The Dart candidate implements the generic DT13 contract in specification,
tests, source, and documentation order. Nineteen package tests pass with
109/109 production lines covered; formatting and fatal analysis are clean.
The contract pins Unicode-scalar edges and numeric order without normalization,
nullable endpoint presence, empty keys and prefixes, upsert counts, iterative
enumeration/deletion/validation, longest-prefix fallback, pruning, and a
50,001-scalar stack-safety case. Independent contract and security findings
were resolved by making the reference pseudocode iterative and nullable-safe,
rejecting malformed UTF-16 atomically before insertion and before traversal
even after an earlier missing edge, redacting missing-key errors, and streaming
longest-prefix traversal without materializing the rest of an
attacker-controlled input.

The existing Dart LZ78 downstream suite passes all 47 tests and the Dart
scaffold generator passes all 17. The Go build tool passes all package tests
and vet; its real exact-diff execution evaluates 45 Starlark files, discovers
90 Dart packages, builds only `dart/trie`, and skips the other 89. The
collision-checked candidate report adds exactly one slot to 4,582, reduces
high-consensus gaps from 267 to 266 and Dart's share from 98 to 97, and
completes trie at 15/15 with zero canonical collisions or unknown buckets.
Capability, dependency, README-link, state, diff, and credential checks are
clean. The state graph remains complete and acyclic at 531 owners and 790
edges: 156 merged, 374 pending, and exactly one in-progress owner, with no
active parity PR.

Ready-for-review PR #13095 publishes the implementation from validated head
`0526bd71ca271cc00350efa565e379f2f69ba12d` on exact `origin/main`
`7fe720dc8a806232732390aaeb85a115ee208709`. The branch rebased cleanly over
six disjoint Mosaic, Rust/Wasm, Mermaid, Japanese, Gujarati, and Punjabi
changes before its normal first push; no force push was needed. Eleven live
open PRs had zero
exact changed-path overlap at the final ownership audit. GitHub reports the PR
non-draft and mergeable. Checks are queued or in progress, merge state is
blocked only by branch protections, and auto-merge remains disabled. State is
now 156 merged, 374 pending, and exactly one `pr-open` owner across the
unchanged complete and acyclic 531-owner/790-edge graph.

PR #13095 completed all 40 final checks at state-recording head
`fe0fd77c2af7ea8bf5df6b937b0ee08e62536625`: 33 succeeded and seven were
expected skips, with no failures or pending work. GitHub reported a clean merge
state, auto-merge was enabled, and the PR merged as
`b2a237322f3f595385eb33f80e6793734659c524` at 2026-08-26T20:29:30Z without a
manual merge command. The clean exact merged-head worktree was removed
afterward.

### Post-#13095 inventory and classical-cipher fixture selection

The exact-main schema-3 report remains collision-clean at 15 established
lanes, 1,373 implementation identities, 4,582 implementation slots, and 1,412
all-reported identities. The breadth bands are 175 high-consensus identities
with 266 gaps, 123 five-to-nine identities with 934 gaps, 166 two-to-four
identities with 2,087 gaps, and 909 singletons with 12,726 gaps. Rust retains
720 singletons; canonical collisions and unknown language buckets remain zero;
OCaml remains correctly emerging at zero packages. Relative to the post-#13083
snapshot, only the expected Dart trie slot was added. Current-main changes
after the merge are disjoint from the parity topology, so no new topology owner
is required.

The reopened 14-of-15 frontier has four exact gaps: Dart alone is missing
`binary-search-tree`, `fenwick-tree`, and `uuid`; Swift alone is missing
`paint-vm-ascii`, which open PR #12149 continues to own. The reference pass
found one semantic ownership gap before selection: DT06 does not close the
Fenwick integer domain, negative-size behavior, bounds errors, or the
monotone-prefix precondition required by `findKth`. A new
`fenwick-tree-language-neutral-fixtures-and-index-contract` owner now precedes
the Dart Fenwick port. This keeps implementation work behind a reviewed neutral
contract instead of silently choosing among divergent established lanes.

The dependency/leverage pass selects
`classical-cipher-language-neutral-fixtures-and-analysis-contract` on branch
`codex/classical-cipher-neutral-fixtures`. It has no dependencies or live-PR
path overlap and unlocks two registered children: established-lane Scytale
Unicode/padding conformance and Vigenere ASCII/analysis conformance. The DT13
neutral corpus unlocks one child; the remaining Dart package ports unlock none.
The fixture tranche will pin Atbash's ASCII substitution contract, Scytale's
Unicode-scalar grid and literal U+0020 padding behavior, and Vigenere's
ASCII-only text/key progression plus deterministic analysis bounds, threshold,
ties, and fixed long-English recovery. A closed, bounded schema and semantic
oracle will keep the static corpus payload-safe and language-neutral.

OCaml's process-free substrate remains collision-unsafe while open PRs #12149
and #12162 own required Go build-tool entry-point and validator surfaces. The
unrelated Haskell event-loop/Brotli tail must be split before selection because
Brotli carries malformed-input and output-amplification risk. No additional
registered worktree met the clean exact-merged-head deletion rule; dirty,
detached, open-PR, and ambiguous worktrees remain preserved.

At selection time, the reconciled graph is complete and acyclic at 532 owners
and 791 edges: 157 merged, 374 pending, and exactly one in-progress owner, with
no active parity PR. The fresh branch starts from exact current
`origin/main` `f7de03cfbecfe2532e18e2112961a5185615eaeb`.

The implementation defines a closed, process-free `classical-ciphers-v1`
schema and 48-case corpus across all nine operations. CR01-CR03 now pin exact
portable string units and edge behavior: Atbash's complete ASCII involution;
Scytale's Unicode-scalar full and ragged grids, U+0020-only trimming,
ascending brute force, and resource limit; and Vigenere's ASCII-only
progression, exact 90% shortest-near-maximum IC rule, ascending chi-squared
tie, insufficient-signal fallback, empty groups, unshortened keys, fixed
`SECRET` recovery, and analysis limits. The independent Python oracle also
enforces bounded strict JSON, duplicate-free objects, finite numbers, separate
schema and fixture depth caps, surrogate and duplicate-ID rejection,
fragment-local references, payload-blind stable errors, and limit checks before
repeat expansion. Its six tests execute 48 semantic subtests at 97%
branch-aware coverage; the canonical-CBOR companion and parity reporter bring
the focused gate to 20 tests plus those 48 subtests.

Existing Dart Atbash, Scytale, and Vigenere packages pass 8, 16, and 16 tests,
fatal analysis, and formatting. Their Python 3.13 references pass 33, 45 with
one expected skip, and 37 tests at 100%, 100%, and 98% coverage. Ruff lint and
source/test formatting are clean; an unchanged Vigenere README retains two
pre-existing code-block spacing notices. The Go build tool passes all package
tests, vet, and trimpath compilation. Its real forced dry run evaluates 45
Starlark files, validates the five reviewed orphan exemptions, discovers all
5,089 packages, and reports all 5,089 as `WOULD-BUILD` without changing package
files. Strict JSON/YAML, Bandit, diff, dependency, credential, raw-control-byte,
BUILD, state, and collision gates pass.

The branch rebased without conflict onto exact `origin/main`
`5bc201ae66021fed72c655484d65dda3753e1eae` after seven disjoint Mermaid,
human-language, Mosaic, ALGOL, and Java-to-semantic-IR changes. The refreshed
schema-3 inventory is unchanged at 15 established lanes, 1,373 implementation
identities, 4,582
slots, 1,412 all-reported identities, zero collisions, and zero unknown
buckets. Five live open PRs have zero exact overlap with the 11 changed paths;
the complete acyclic state remains 532 owners and 791 edges with 157 merged,
374 pending, and exactly one in-progress owner. Independent contract, security,
and ownership reviews identified and verified closure of every focused loader,
fixture, CI, and wording issue; no publication blocker remains.

Ready-for-review PR #13112 publishes the contract from validated head
`32a21b71455163ba58160dc97e89040ef53722de` on exact `origin/main`
`5bc201ae66021fed72c655484d65dda3753e1eae` after a normal first push. Every
intervening rebase was conflict-free, and no force push was needed. Five live
open PRs had zero exact overlap with the 11 changed paths at publication.
GitHub reports the PR non-draft and mergeable. Checks are queued or in
progress, merge state is blocked only by branch protections, and auto-merge
remains disabled until every required check is terminal and acceptable. State
is now 157 merged, 374 pending, and exactly one `pr-open` owner across the
unchanged complete and acyclic 532-owner/791-edge graph.

PR #13112's final state-recording head
`681cab6d2991f52d2ec7def9a1847deb03036ba2` completed all 40 reported checks:
33 passed and seven were expected skips, with no failure or pending check.
GitHub reported the branch clean and mergeable. The loop enabled squash
auto-merge, and GitHub merged the PR as
`7b43338ebe0e7caa55cda1e06bd0324ae9d1a6e7` at
2026-08-26T21:59:52Z without a manual merge command. Its clean worktree matched
the exact reviewed head and was removed after the successor worktree existed.

Before successor selection, `origin/main` advanced through disjoint Urdu
curriculum and Rust Wasm-runtime changes to exact
`523f5ac925b23a8682f199e1a1838be745d6833a`. Neither merge added an
implementation package identity. The regenerated schema-3 inventory therefore
remains collision-clean at 15 established lanes, 1,373 implementation
identities, 4,582 slots, and 1,412 all-reported identities. The four bands remain
175/266, 123/934, 166/2,087, and 909/12,726; Rust has 720 singletons, OCaml is
still emerging at zero packages, and collisions and unknown buckets remain
zero. The exact 14/15 frontier remains Dart binary-search-tree, Fenwick tree,
and UUID plus externally owned Swift paint-vm-ascii. The ragged Scytale review
finding is already captured by its registered conformance owner, so the refresh
adds no new owner.

The next selected owner is
`scytale-unicode-padding-established-lane-conformance` on
`codex/scytale-established-lane-conformance`. It is newly dependency-ready after
the neutral classical-cipher corpus merged, implements the CR02 behavior pinned
by that suite, and closes one established identity's Unicode-unit,
ragged-column, and literal U+0020-padding divergence across all 15 lanes. It is
the highest-leverage bounded successor and materially narrower than the sibling
Vigenere cryptanalysis convergence. Seven live PRs have zero exact overlap with
its package, CR02, fixture, state, or roadmap surfaces. OCaml's process-free
substrate remains collision-unsafe while PRs #12149 and #12162 own required Go
build-tool entry and validator paths. The reconciled state graph remains
complete and acyclic at 532 owners and 791 edges: 158 merged, 373 pending, and
exactly one in-progress owner, with no active parity PR.

The final implementation audit found a distinct test-infrastructure gap before
publication: the package tests directly exercise the normative Scytale
semantics, but no lane loads or generatedly consumes all 18 Scytale fixture
cases and compares each complete expected object. The new pending owner
`scytale-language-neutral-fixture-established-lane-consumers` depends on this
behavior-convergence slice and will add bounded test-only JSON consumers or
dependency-free generated adapters plus a corpus-drift gate across all 15
lanes. This keeps the current implementation claim honest while preserving the
fixture README's stronger definition of executable conformance. The addition
brings the registered graph to 533 owners and 792 edges without changing the
single in-progress owner.

The implementation now converges every established Scytale package on Unicode
scalar cells, explicit full and ragged column reconstruction, U+0020-only
padding removal, empty-before-validation behavior, stable errors, ascending
brute force, and the 4,096-scalar preflight. Package tests pass in all 15 lanes;
coverage is above 80% wherever a lane has a coverage gate, and Java/Kotlin now
gate line coverage explicitly. TypeScript's Unix and Windows BUILD fronts use
`npm ci`, compile the public API, run 26 tests at 100% coverage, and retain an
audited range-consistent `nanoid` security patch with zero reported npm
vulnerabilities. Empty schema-v1 capability manifests preserve the packages'
process-free, filesystem-free, network-free authority profile.

After a conflict-free rebase over five disjoint mainline commits to exact
`origin/main` `5fb2ef7d5d858b6144075eb95e162908f4b26cbb`, a fresh build-tool
plan evaluated 45 Starlark files, discovered 5,089 packages, selected exactly
the 15 established Scytale packages, and built all 15 while skipping 5,074.
The collision gate remains at 15 established lanes, 1,373 identities, 4,582
slots, and zero collisions or unknown buckets. Ten live PRs have zero overlap
with the 87 changed paths. Independent reviews are clean after focused Dart,
Elixir, Swift, JVM coverage, TypeScript build, README, and generated-artifact
fixes. Exact cleanup of generated plans, coverage, dependency, and compiler
caches reduced the active worktree from about 1.3 GiB to 778 MiB.

Ready-for-review PR #13128 publishes the validated implementation from initial
head `a44209c2fe836357dc466530aac741c50df794bc` on exact `origin/main`
`5fb2ef7d5d858b6144075eb95e162908f4b26cbb` after a normal first push.
The conflict-free rebase required no force push. GitHub reports the PR
non-draft; checks are queued and mergeability is still being calculated, so
auto-merge remains disabled until every required check is terminal and
acceptable and GitHub reports no conflict. State is now 158 merged, 374
pending, and exactly one `pr-open` owner across the complete acyclic
533-owner/792-edge graph.

PR #13128's replacement head `b6fe5f60a09c50b24e4753ab33d3975de7f1c495`
completed all 41 reported checks acceptably after a focused Windows build-front
repair for the intentionally absent Elixir toolchain. GitHub reported the
branch `MERGEABLE/CLEAN`; the loop enabled squash auto-merge, and GitHub merged
the PR as `0ce595048edcd1e9d9c41e6b2249d19ef3567224` at
2026-08-27T00:37:58Z without a manual merge command. The clean successor setup
removed the exact reviewed-head worktree and preserved every dirty, detached,
open-PR, and ambiguous worktree.

Before successor selection, `origin/main` advanced through disjoint Mosaic
Flutter work to exact `0a1a5902fccf90f9cabe572c60a7c461768a4bee`.
The regenerated schema-3 report remains collision-clean at 15 established
lanes, 1,373 implementation identities, 4,582 slots, and 1,412 all-reported
identities. The four bands remain 175/266, 123/934, 166/2,087, and
909/12,726; Rust has 720 singletons, OCaml is still emerging at zero packages,
and collisions and unknown buckets remain zero. Neither merge added an
identity, so no new unowned eligible gap was registered.

The next selected owner is
`scytale-language-neutral-fixture-established-lane-consumers` on
`codex/scytale-neutral-fixture-consumers`. Its sole dependency is now merged,
and it closes the explicitly registered residual debt: all 15 established
packages test CR02 semantics directly, but none executes every one of the 18
normative Scytale fixture objects and compares the complete expected record.
Dependency-free generated test adapters plus a corpus-drift gate keep the
slice bounded and production capability profiles unchanged. Seven live PRs
have zero exact overlap with the package, CR02, fixture, drift-gate, state,
roadmap, or CI surfaces. The tranche deliberately avoids Go build-tool source
owned by open PRs #12149 and #12162.

Implementation is now complete in the successor worktree. One bounded
stdlib-only generator validates the exact fixture profile, resource limits,
raw digest, strict JSON subset, unique IDs, and all 18 Scytale cases before
emitting a native complete-record test for every established lane. Its
six-test regression suite covers roster and digest drift, every-ID presence,
duplicate and non-finite input, malformed Unicode, nesting and size bounds,
and stale or missing outputs. CI runs those tests and the generator's
`--check` gate immediately after the independent classical-cipher oracle.
Production sources, dependency manifests, capability profiles, and BUILD
recipes remain unchanged.

Direct native validation passes on the 13 locally executable lanes: C#, Dart,
F#, Go, Haskell, Java, Kotlin, Lua, Python, Ruby, Rust, Swift, and TypeScript.
Elixir and Perl keep their reviewed Windows no-toolchain skips and will execute
their generated consumers in Linux CI. The Go build tool passes its complete
test, coverage, vet, module-verification, and trimpath gates; the exact diff
plan evaluates 45 Starlark files, discovers 5,089 packages, preserves the five
reviewed orphan exemptions, selects exactly the 15 established Scytale
packages in every platform override, and executes all 15 Windows BUILD fronts
successfully across the full run plus isolated Python and Lua toolchain
reruns. Formatting, lint, build-file, fixture, diff, dependency, and security
gates are clean. The dependency-free Rust package has no lockfile for
`cargo audit`; its empty dependency table and cargo metadata provide the
applicable dependency evidence.

The branch rebased without conflict onto exact `origin/main`
`b46fbb583bf06ccbdc499980c78afb62eff12afd` after four disjoint curriculum and
Mermaid commits. The refreshed schema-3 report is unchanged at 15 established
lanes, 1,373 implementation identities, 4,582 slots, 1,412 all-reported
identities, zero collisions, and zero unknown buckets.

Ready-for-review PR #13147 publishes the validated implementation from
pre-state head `78d2166ca172c2e9c6135078d4ad31e589aaf2fc` on exact `origin/main`
`b46fbb583bf06ccbdc499980c78afb62eff12afd` after a normal first push. GitHub
is calculating checks and mergeability, so auto-merge remains disabled until
every required check is terminal and acceptable and the PR is conflict-free.
The state graph now has 159 merged, 373 pending, and exactly one `pr-open`
owner across 533 owners and 792 dependency edges.

The initial PR head later completed every reported check successfully without a
merge conflict, but the final independent exact-diff security audit found four
in-scope generator gaps. Fixture text could be emitted into interpolation-aware
source literals, file reads were not bounded before allocation, the limit case
duplicated its scalar and count instead of consuming the fixture descriptor,
and six lanes accepted only a broad exception class instead of the normalized
`scytale-brute-force-limit` failure. Auto-merge was deliberately withheld while
the same branch received the focused repair.

The repaired generator now admits only source-safe case and error identifiers,
uses escaped Dart literals or Unicode-scalar constructors for Elixir, Kotlin,
Perl, and Ruby, bounds fixture and generated-output reads before allocation,
and derives every lane's limit scalar, count, and normalized failure from the
fixture. Eleven generator regressions cover the new boundaries, including
hostile control, quote, backslash, and interpolation scalars in all 15 lanes,
plus fixture-owned limit and invalid-key error identifiers. All 13 locally
available native suites pass again, including exact limit-error assertions in
C#, Dart, F#, Java, Kotlin, and Swift; Elixir and Perl remain delegated to the
fresh Linux CI cycle because their reviewed Windows toolchains are absent.

## Post-#13147 Refresh and Vigenere Conformance Selection

Focused hardening head `28d4e018de7aa8d94af20a9e56e71be8baad52d7`
completed all 41 reported checks: 39 successes and two expected skips, with no
failure or pending check. GitHub reported `MERGEABLE/CLEAN`; the loop enabled
squash auto-merge, and GitHub merged PR #13147 as
`878556189d8652d9dc6cb9c84e3d3236e7f09f54` at
2026-08-27T03:48:57Z without a manual merge command. The final generator uses
bounded reads, source-safe identifiers, per-language scalar or proven literal
encoders in all 15 lanes, and exact fixture-derived invalid-key and limit
error identifiers.

The collision-checked schema-3 inventory on exact merged `origin/main` is
structurally unchanged: 15 established lanes, 1,373 implementation identities,
4,582 package slots, and 1,412 all-reported identities. The four bands remain
175/266, 123/934, 166/2,087, and 909/12,726; Rust has 720 singletons, OCaml is
still emerging at zero packages, and collisions and unknown buckets remain
zero. Intervening Algol, Mermaid, HTML, Mosaic, and curriculum work adds no
package identity or eligible unowned gap. The exact post-merge state graph
initially remains 533 owners and 792 dependency edges. The reference pass then
records one newly discovered follow-on owner,
`vigenere-language-neutral-fixture-established-lane-consumers`, because no
established lane currently executes all 26 complete Vigenere fixture objects.
That bounded generated-consumer tranche depends on behavior convergence and
brings the graph to 534 owners and 793 edges. The later toolchain audit also
registers `vigenere-established-lane-build-front-hardening` for the Swift
failure-masking front, unconditional Haskell/Perl Windows skips, omitted
TypeScript typecheck, and PATH-specific Python `uv` invocation. That separate
owner brings the complete graph to 535 owners and 794 edges without widening
this behavior slice.

The next selected owner is
`vigenere-ascii-established-lane-conformance` on
`codex/vigenere-ascii-established-lane-conformance`. Its neutral CR03 contract
is merged and it closes observable ASCII text/key, key-length inference,
chi-squared recovery, insufficient-signal, tie-breaking, and resource-limit
drift across the established Vigenere implementations. That direct all-lane
conformance leverage outranks the independent DT13 fixture contract while
remaining one bounded package family. Six live open PRs have zero exact
overlap with its Vigenere package, CR03, classical-cipher fixture, state, or
roadmap surfaces. After selection, state is 160 merged, 374 pending, and
exactly one in-progress owner, with no active parity PR.

## Post-#13172 Refresh and Vigenere Fixture-Consumer Selection

Focused Windows build-front repair head
`e212e0a87632069e8a37e732a4f784a9e3936b1c` completed all 41 reported checks:
39 succeeded and two were expected skips, with no failure or pending check.
GitHub reported `MERGEABLE/CLEAN`; the loop enabled squash auto-merge, and
GitHub merged PR #13172 as
`580e1a1710452f1133eae07cc5252961920cac57` at
2026-08-27T13:18:07Z without a manual merge command.

The collision-checked schema-3 inventory on that exact merged `origin/main`
contains 15 established lanes, 1,374 implementation identities, 4,583 package
slots, and 1,413 all-reported identities. The four bands are 175/266,
123/934, 166/2,087, and 910/12,740; Rust has 721 singletons, canonical
collisions and unknown buckets remain zero, and OCaml remains correctly
emerging at zero packages. The sole topology delta is Rust-only
`layout-inline`, added by PR #13196. The refresh registers
`layout-inline-singleton-classification` before selection so its deterministic
Layout IR behavior receives either a portable contract or a reviewed subsystem
exception. Live PR #13209 owns its Rust source, so the classification owner
must not implement or modify that package while the PR remains active.

The fixture audit also found that none of the 15 Atbash packages executes all
six complete CR01 objects from `classical-ciphers-v1`. New pending owner
`atbash-language-neutral-fixture-established-lane-consumers` records that
bounded generated-consumer work before selection. Schema validation alone is
not executable conformance, so this is distinct from the merged neutral corpus
contract and from the completed Scytale consumers.

The dependency/leverage pass selects
`vigenere-language-neutral-fixture-established-lane-consumers` on branch
`codex/vigenere-neutral-fixture-consumers`. Its sole behavior-convergence
dependency is now merged. Executing all 26 complete normative Vigenere objects
through the bounded dependency-free generated-consumer pattern already proven
for Scytale closes executable drift evidence across all 15 established lanes.
That immediate all-lane reach outranks the ready five-front Vigenere build
hardening sibling, the six-case Atbash consumer, and the independent
single-package contracts. The live-PR path audit found no overlap with the
Vigenere packages, CR03 and classical-cipher fixtures, prospective generator
and tests, state, roadmap, CI, or root changelog surfaces.

The strategic OCaml process-free build substrate remains pending even though
its declared dependencies are merged: open PR #12149 still owns the required
Go build-tool validator and entry-point paths. OCaml therefore remains outside
the established denominator until that collision clears and the full substrate,
representative package, capability analyzer, native build tool, and promotion
chain lands. After reconciliation and selection, the complete acyclic graph has
537 owners and 795 dependency edges: 161 merged, 375 pending, and exactly one
in-progress owner, with no active parity PR. The fresh worktree and branch start
from exact `origin/main` `580e1a1710452f1133eae07cc5252961920cac57`.

## Pre-publication Rebase and Manchester Baby Classification

Before publication, the selected branch rebased conflict-free twice, finally
onto exact `origin/main` `063d4d725df6dd8f854ef6c3a74a6459d8322e3b`. The
intervening Mermaid, Algol, two Manchester Baby, Spanish, human-language,
HTML-parser, and blog changes have no exact overlap with this tranche's 57
Vigenere, fixture-generator, state, roadmap, CI, and changelog paths.

The final required collision-checked schema-3 refresh records 15 established
lanes, 1,375 implementation identities, 4,585 package slots, and 1,414
all-reported identities. The four bands are now 175/266, 123/934, 167/2,100,
and 910/12,740; Rust has 722 singletons, canonical collisions and unknown
buckets remain zero, and OCaml remains emerging at zero packages. Merged PR
#13198 added the Rust `manchester-baby-simulator` beside its existing
TypeScript package. It therefore added one slot without adding an identity,
moving that package from the singleton band into the two-to-four-lane band.
Merged PR #13218 then added the independently named Rust-only
`manchester-baby-gatelevel`, contributing one identity, one slot, and one Rust
singleton.

New pending owner `manchester-baby-simulator-two-lane-classification` records
the higher-level simulator's thirteen-lane classification gap. Separate new
pending owner `manchester-baby-gatelevel-singleton-classification` records the
gate-level package's transistor, netlist, clock, machine-state, diagnostic,
resource-limit, portability, and applicability review. The resulting complete
acyclic graph has 539 owners and 795 dependency edges: 161 merged, 377 pending,
and exactly one in-progress owner, with no active parity PR.

## Post-#13224 Refresh and Atbash Fixture-Consumer Selection

Final state-recording head `b7e1dcd82e980fd5e441eb487ce3f97e6060d015`
completed all 41 reported checks: 39 succeeded and two were expected skips,
with no failure or pending check. GitHub reported `MERGEABLE/CLEAN`; the loop
enabled squash auto-merge, and GitHub merged PR #13224 as
`0302e4e8f35a6e79d7bd385ae91fde710176a53c` at
2026-08-27T16:18:19Z without a manual merge command.

The collision-checked schema-3 inventory on exact merged `origin/main` is
topology-identical to the pre-publication report: 15 established lanes, 1,375
implementation identities, 4,585 package slots, and 1,414 all-reported
identities. The four bands remain 175/266, 123/934, 167/2,100, and 910/12,740;
Rust has 722 singletons, OCaml remains emerging at zero packages, and canonical
collisions and unknown buckets remain zero. No package-directory addition,
removal, lane move, or normalized-identity change creates a new inventory-
driven owner.

The late-main review does make the roadmap's broad TypeScript-led Forme family
actionable as a narrower pending owner,
`forme-portable-core-family-classification`. It must separate deterministic
graph, routing, content-model, and rendering kernels from browser, Vite,
filesystem, and UI hosts before any port. PR #13244 subsequently merged as
`f7d0b913d3175096197b1d325863012dff90741b`, clearing the temporary path block
without changing package topology. Live PR #13251 now owns overlapping
`forme-collect-chronological`, `forme-render-static`, and Forme-roadmap paths,
so that owner is again registered but collision-blocked. The Vigenere
BUILD-front owner now explicitly depends on the merged neutral-consumer
tranche: its five false-green or incomplete front doors remain a bounded
operational sibling.

The dependency/leverage pass selects
`atbash-language-neutral-fixture-established-lane-consumers` on branch
`codex/atbash-neutral-fixture-consumers`. Its sole neutral-contract dependency
is merged; six complete CR01 objects exist; all 15 established Atbash packages
exist; and none executes the full corpus. Reusing the strict bounded,
dependency-free generated-consumer and digest/ID drift-gate pattern proven by
Scytale and Vigenere closes executable all-lane evidence without adding
production filesystem, JSON, process, environment, or network authority. A
live open-PR scan found zero exact overlap with Atbash packages, CR01, the
classical-cipher fixture, generator and CI surfaces, state, roadmap, or the root
changelog.

Before publication the branch rebased conflict-free twice, finally onto exact
`origin/main` `a45a88e98a50fb363eb68d441926e59a523a9106`. Intervening
curriculum, ALGOL, Forme, Venture, HTML-parser, Mermaid, and Language Ladder
changes have no exact implementation overlap. The refreshed collision-checked
schema-3 inventory records 1,376 implementation identities and 4,586 slots;
the bands are 175/266, 123/934, 167/2,100, and 911/12,754, with 723 Rust
singletons, zero OCaml packages, zero collisions, and zero unknown buckets.

Merged PR #13209 added the Rust-only `browser-navigation` identity. New pending
owner `browser-navigation-singleton-classification` records its evidently
portable, host-neutral in-memory history and canonical visited-link contract,
its `url-parser` dependency order, and the need for neutral fixtures before an
applicable-lane rollout. Browser fetch, storage, rendering, windows, and native
paint remain outside that owner.

PR #13252's first hosted-Windows diff build then exposed an operational gap:
the workflow intentionally does not install Erlang/Elixir on newer Windows
runner images, but Elixir Atbash lacked a `BUILD_windows` override and invoked
the fallback Mix command. The focused repair records the same truthful Windows
platform skip used by Scytale and Vigenere while retaining real native Mix
evidence on Linux and supported local hosts. New pending owner
`elixir-windows-build-front-toolchain-classification` audits the wider build
front, pins supported-platform semantics, and requires either a validated
Windows toolchain or consistent narrow fail-closed exceptions.

The strategic OCaml process-free substrate still has every declared dependency
merged but remains collision-unsafe while open PR #12149 owns the required Go
validator and entry-point paths. Native Dart and JVM build tools remain blocked
behind the pending execution-semantics and trusted-platform isolation chain.
After reconciliation and selection, the complete acyclic graph has 542 owners
and 797 dependency edges: 162 merged, 379 pending, and exactly one pr-open
owner, with active parity PR #13252. The fresh worktree and branch start from
exact `origin/main` `0302e4e8f35a6e79d7bd385ae91fde710176a53c`.

## Post-#13252 Refresh and Elixir Windows BUILD-Front Implementation

Final repair head `70d5f2847e6df41d5b72bd941ebf24bbf7c1fc89`
completed all 41 reported checks: 39 succeeded and two were expected skips,
with no failure or pending check. GitHub reported `MERGEABLE/CLEAN`; the loop
enabled squash auto-merge, and GitHub merged PR #13252 as
`1606285f832b9e2cc0743a5339b4309925f323c5` at
2026-08-27T19:30:54Z without a manual merge command.

The collision-checked schema-3 inventory on exact selected `origin/main`
`c2981e655697bd9a9ba82c9b78f20edc9fed31d1` records 15 established
lanes, 1,377 implementation identities, 4,588 package slots, and 1,416
all-reported identities. The four bands are 175/266, 123/934, 168/2,113,
and 911/12,754; Rust has 724 singletons, OCaml remains emerging at zero
packages, and canonical collisions and unknown buckets remain zero. Merged
IBM 704 work adds the Python/Rust simulator slot and the separately named
Rust-only gate-level identity. Pending owners
`ibm704-simulator-two-lane-classification` and
`ibm704-gatelevel-singleton-classification` record both topology deltas before
selection.

The dependency/leverage pass selects
`elixir-windows-build-front-toolchain-classification` on branch
`codex/elixir-windows-build-front-toolchain-contract`. Its build-tool contract
dependency is merged, and one pinned toolchain boundary turns an entire lane
from latent missing-tool or false-green behavior into native Windows evidence.
The collision audit finds no live PR overlap with the selected workflow,
Elixir BUILD-front, audit, fixture, or Go executor/reporter surfaces. The
strategic OCaml process-free substrate remains collision-blocked while open PR
#12149 owns its required Go validator and entry-point paths.

The implementation audits all 285 Elixir package/program BUILD roots. It
provisions Elixir 1.18.4 and OTP 27.3.4.11 through an exact setup-action commit
on the explicit `windows-2025` job; keeps 278 roots as real native evidence;
repairs the inherited POSIX-only `http1` and `zip` fronts with CMD-safe
overrides; and removes the temporary Atbash, Scytale, and Vigenere skips. Seven
reviewed NIF, transitive-NIF, or Metal exclusions use a closed declarative
protocol whose stable code is reported as `UNSUPPORTED`; downstream consumers
become `DEP-UNSUPPORTED`, and neither result is a passing shell command or a
success-cache entry. A language-neutral schema/contract fixture plus a bounded,
Git-visible, symlink-rejecting audit pins the corpus, workflow, runner,
toolchain, action digest, front syntax, counts, and exact exception registry.

The same audit discovers three tracked Mix projects outside BUILD discovery:
`activation_functions`, `matrix`, and `perceptron`. New pending owner
`elixir-activation-matrix-perceptron-build-front-coverage` depends on this
toolchain tranche and requires missing tests before those roots become honest
cross-platform build evidence.

Before publication the branch rebased conflict-free twice, finally onto exact
`origin/main` `0f22582f5a0ddaceaae4f8f6ba56f3b2d5a4fd30`.
Real Windows plan emission found one additional build-tool edge case: deleting
the three temporary cipher overrides initially failed to select their canonical
fallbacks. Platform change detection now treats deletion of a selected override
as a platform-scoped package change. The regenerated closure contains all three
ciphers and completed 16 native builds, seven exact unsupported results, one
dependency-unsupported result, and 261 unaffected skips with no failure.

The final schema-3 inventory records 1,379 implementation identities, 4,590
slots, and 1,418 all-reported identities. The bands are 175/266, 123/934,
168/2,113, and 913/12,782; Rust has 726 singletons, OCaml remains emerging at
zero packages, and collisions and unknown buckets remain zero. Merged PR #13264
adds the Rust-only `browser-bookmarks` portable core and separately named
`browser-bookmarks-file` native adapter. New pending owners
`browser-bookmarks-singleton-classification` and
`browser-bookmarks-file-applicable-lane-classification` record those gaps in
dependency order before publication.

A final live-PR path audit finds no package, fixture, audit, state, roadmap, or
changelog overlap. PR #13285 concurrently edits `ci.yml` for Forme workflow
modernization and PR #12149 edits the Go entry point for a Swift native-dep
validator path; their current hunks are line-distinct from this tranche, but the
loop must recheck conflicts if either merges first. With the new owners
registered, the selected graph contains 547 owners and 799 dependency edges:
163 merged, 383 pending, and exactly one pr-open owner. Ready-for-review PR
#13287 was opened from validated head
`b17e72130edc097b7f0cbe2483431513408b91f5`; it is the sole active parity PR.
GitHub reports it non-draft and mergeable, while required checks are still
queued or in progress, so auto-merge remains disabled until all checks are
terminal and acceptable and the branch is conflict-free.

## Post-#13287 Refresh and OCaml Process-Free Substrate Selection

All 43 final checks for PR #13287 reached terminal acceptable conclusions and
GitHub reported final head `fffa84a555b74e0e4f95c6f7f8eac9fc9a023f4e`
conflict-free. The loop enabled squash auto-merge, and GitHub merged the PR at
2026-08-28T04:12:40Z as
`548477ac8975894fd60a1f08878d81e0dfa0c692` without a manual merge command.
The active parity slot is therefore clear.

The collision-checked schema-3 inventory on current exact `origin/main`
`e6d99bc6997d0c4fae3b00b76653e0a824d6dc98` records 15 established
lanes, 1,383 implementation identities, 4,595 package slots, and 1,422
all-reported identities. The four bands are 175/265, 123/934, 168/2,113,
and 917/12,838; Rust has 729 singletons, OCaml remains emerging at zero
packages, and canonical collisions and unknown buckets remain zero. The final
main advance after the Elixir merge is human-language-only and does not change
these counts.

Four new singleton identities and one new implementation slot explain the
topology delta. New pending owners classify the Rust-only `ge225-gatelevel`,
pure device-independent `text-flow`, and mixed neutral/native
`venture-browser-visual-fixtures` packages. Live PR #13357 currently blocks the
GE-225 owner. TypeScript-only `forme-theme-classless` belongs to the existing
Forme portable-core family owner. Merged external PR #12149 added the Swift
`paint-vm-ascii` package and closed the last package-directory gap, but the
package still lacks the required explicit empty capability profile, so its
existing owner remains pending rather than being credited as complete.

The dependency/leverage pass selects
`ocaml-build-substrate-process-free-core` on branch
`codex/ocaml-build-substrate-process-free-core`. Its infrastructure, CI
toolchain, and pure-domain-corpus prerequisites are all merged; former overlap
blockers #12149 and #12162 are merged; and every current open PR has zero
exact overlap with the prospective Go build-tool, OCaml specification,
workflow, state, or roadmap surfaces. This tranche adds repository-owned OCaml
package/program discovery, field-aware opam/Dune local dependency resolution,
source hashing, build-file validation, shard cost, affected-node behavior, and
workflow markers. It remains process-free: opam-switch serialization,
execution conformance, native OCaml build-tool implementation, package
promotion, and entry into the established-language denominator stay with their
existing downstream owners.

Immediately before publication, the implementation rebased without conflict
onto exact `origin/main` `72e33f497aba09efdcf195cc625f297f5a5ddf47`.
Merged PR #13352 remains inside the registered ALGOL IIR neutral and lane
owners, #13355 remains inside the Mermaid sequence-diagram owner, and #13357
clears the GE-225 owner's former live-path block while leaving its neutral and
cross-lane work pending. None changes an implementation identity or
BUILD/build-tool marker. The refreshed collision-checked report therefore
remains 15 established lanes, 1,383 identities, 4,595 slots, zero collisions,
and zero unknown buckets.

Ready-for-review PR #13366 was opened at 2026-08-28T05:02:29Z from the
validated two-commit branch. GitHub reports it non-draft and mergeable. Its
required checks are queued and `mergeStateStatus` is `BLOCKED`, so auto-merge
remains disabled until every required check is terminal and acceptable and the
branch remains conflict-free.

## Post-#13366 Refresh and Extra CI Toolchain Corpus Selection

All 48 final checks for PR #13366 reached terminal acceptable conclusions. A
single Linux locked-fixtures job failed while `curl` fetched the pinned OCaml
compiler archive; its rerun passed without a repository change. GitHub then
reported final head `a6eefa89f743922efd3cda335d715f2e9443bd06`
conflict-free. The loop enabled squash auto-merge, and GitHub merged PR #13366
at 2026-08-28T06:18:27Z as
`038431ce25f4af12a5498eb65d70e82664ce94b1` without a manual merge
command. The active parity slot is clear.

The collision-checked schema-3 inventory on that exact merged `origin/main`
remains topology-identical to the pre-publication report: 15 established
lanes, 1,383 implementation identities, 4,595 package slots, and 1,422
all-reported identities. The completion bands remain 175/265, 123/934,
168/2,113, and 917/12,838; Rust has 729 singletons, canonical collisions and
unknown buckets remain zero, and OCaml remains correctly emerging at zero
packages. No implementation package directory was added or removed on main,
and no exact-main eligible unowned package or build-tool gap was discovered.
The live-PR audit pre-registers one prospective topology delta: PR #13377
would add Rust `cdc6600-simulator` beside the existing Python package, so
`cdc6600-simulator-two-lane-classification` records the future portable-core
review and remains collision-blocked while that PR is open. Merged PR #13368's
new filesystem asset resolver is classified as an applicable-lane child of the
existing Forme family owner, and PR #13371 blocks the Venture visual-fixture
owner while it changes that surface.

The dependency/leverage pass selects
`build-tool-extra-ci-toolchain-declaration-corpus` on branch
`codex/build-tool-extra-ci-toolchain-declaration-corpus`. Its sole direct
dependency, the pure-domain corpus, is merged. The tranche defines a closed,
process-free fixture snapshot for exact `# needs-toolchain:` syntax, canonical
names, empty and unknown values, stable deduplication, multiple declarations,
platform BUILD precedence, affected-only selection, forced-full behavior, and
deterministic complete toolchain maps. It then binds the canonical Go behavior
to those neutral fixtures without consulting a checkout, process,
environment, or network. The owner removes the immediately actionable blocker
from the Go-oracle path and feeds every current engine plus later Dart, JVM,
and promoted OCaml engines. The separate execution-semantics prerequisite
remains gated by reviewed platform isolation work, so this slice claims no
execution authority or OCaml promotion.

A live open-PR path audit found no overlap with the shared fixture schema,
cases, process-free runner, Go discovery and toolchain tests, state, roadmap,
or documentation surfaces. After lifecycle reconciliation and selection, the
complete acyclic graph has 551 owners and 799 dependency edges: 165 merged,
385 pending, and exactly one in-progress owner, with no active parity PR.

Before implementation, the selected branch rebased onto later `origin/main`
revision `a20237bf01d079ea02bcce12009fed2156a3c654`. Those intervening merges did
not overlap the selected paths; a second collision-checked report preserved the
same topology and zero-collision result.

Before publication, the branch rebased again onto
`d751dfa23a4cdfbefe1c9f637de67a2c8003d02d`. Merged PR #13368 added the already
classified TypeScript-only `forme-resolve-asset-refs-fs` identity, clearing the
Forme live-PR block and moving the exact inventory to 1,384 implementation
identities, 4,596 slots, 1,423 all-reported identities, 918 singletons, and
12,852 singleton gaps. Every other completion band is unchanged; collisions,
unknown buckets, and OCaml packages remain zero. The existing Forme family
owner covers the new filesystem-authority child, so this refresh adds no
eligible unowned gap and does not change the selected tranche.

## Post-#13388 Refresh and C# Declaration Consumer Selection

All 41 final checks for PR #13388 reached terminal acceptable conclusions: 35
successes and six expected skips. GitHub reported final head
`96eaf214e75b3dff458ef104f3f9fb31a9505acb` conflict-free. The loop enabled
squash auto-merge, and GitHub merged the PR at 2026-08-28T08:05:32Z as
`b8b2282996b6983f4d2b2de87889300900bd37dd` without a manual merge command.
The active parity slot is therefore clear.

The collision-checked schema-3 inventory on current exact `origin/main`
`5ffefdda23ad1c3ce0e3d285d859d3ed6bed2fc2` records 15 established lanes,
1,386 implementation identities, 4,599 package slots, and 1,425 all-reported
identities. The four bands are 175/265, 123/934, 169/2,126, and 919/12,866;
Rust has 730 singletons, OCaml remains emerging at zero packages, and canonical
collisions and unknown buckets remain zero. Merged PR #13377 adds one Rust
`cdc6600-simulator` slot beside Python, moving that identity from singleton to
the two-to-four band and clearing its existing owner's collision block. Merged
PR #13391 adds independently named Rust-only `cdc6600-gatelevel`, which is now
recorded by its dedicated pending classification owner. Merged PRs #13395 and
#13401 add and harden TypeScript-only `forme-load-assets-fs`; the existing Forme
family owner covers its mixed deterministic asset/MIME and filesystem authority
boundaries.

The live topology audit registers every prospective package change before
selection. Open PR #13405 would add Rust `pdp11-simulator` beside the existing
Python package, moving the identity from singleton into the two-to-four-lane
band without adding a new identity; the dedicated PDP-11 two-lane owner is
blocked while that PR remains open. Open PR #13409 prospectively adds
TypeScript `forme-emit-site-fs`, so the existing Forme family owner is blocked
again without creating a duplicate. Merged PR #13390 extends the existing
text-flow and Venture visual-fixture owners. No exact-main or live topology gap
remains unowned, and current live PRs have zero exact overlap with the selected
C# build-tool surfaces.

The final pre-implementation refresh includes merged #13394, #13402, #13403,
and #13392. Their human-language and existing-package changes leave the package
inventory unchanged. The #13403 merge clears the existing Venture
visual-fixtures owner's live-path block without creating a new identity or
owner.

The dependency pass decomposes
`build-tool-extra-ci-toolchain-declaration-remaining-engines` into eleven
independently reviewable C#, F#, Elixir, Haskell, Lua, Perl, Python, Ruby, Rust,
Swift, and TypeScript children. F# depends on the C# child because its native
facade directly references that engine; every child depends on the merged
process-free corpus. Independent C# review also found that the neutral Python
reference and current Go parser accept lone carriage returns outside CRLF. A
new cross-runtime hardening owner pins CRLF-positive and lone-CR-negative corpus
cases and blocks the ten unselected engine children until the reference and Go
oracle are repaired; the selected C# consumer already rejects lone CR directly.
Exactly one item is selected on branch
`codex/build-tool-csharp-extra-ci-toolchain-declaration-conformance`: make the
C# engine consume the neutral snapshots with canonical registry and language
mapping, selected platform-front precedence, exact stable declarations,
affected and forced-full scheduling, deterministic unsupported diagnostics,
and bounded inert inputs. The tranche adds no process, Git, environment,
network, execution, or OCaml promotion authority and leaves F# for the next
separately reviewed facade slice.

## Post-#13418 Refresh and CRLF Declaration Grammar Selection

All 40 final checks for PR #13418 reached terminal acceptable conclusions: 33
successes and seven expected skips. GitHub reported final head
`0f4f64103081f39ce49536384f356688476a1d7e` conflict-free. The loop enabled
squash auto-merge, and GitHub merged the PR at 2026-08-28T10:01:38Z as
`bb74da875cf8453c08ffe409522a7975b0b3134c` without a manual merge command.
The active parity slot is therefore clear.

The collision-checked schema-3 inventory on exact current `origin/main`
`b7178c248b66a0f84982c9ce0abd026979552d02` records 15 established lanes,
1,388 implementation identities, 4,602 package slots, and 1,427 all-reported
identities. The four completion bands are 175/265, 123/934, 170/2,139, and
920/12,880; Rust has 731 singletons, OCaml remains emerging at zero packages,
and canonical collisions and unknown buckets remain zero.

Merged PR #13405 adds Rust `pdp11-simulator` beside Python and clears the
existing two-lane functional-simulator owner's live-PR block. Merged PRs
#13409 and #13420 add `forme-emit-site-fs` and its routed-blog consumer within
the existing Forme family, clearing that owner's live-PR block without a
duplicate singleton owner. Merged PR #13422 adds the independently named
Rust-only `pdp11-gatelevel` identity. New pending owner
`pdp11-gatelevel-singleton-classification` records its deterministic
gate-backed state, clock, trace, diagnostic, topology, resource-limit, and
functional-differential review before selection. With that addition, no
eligible exact-main topology gap remains unowned.

Before validation, the selected branch rebased conflict-free over merged PRs
#13421, #13408, #13417, and #13427. Their Venture cascade work, ALGOL exponent
composition, and Sanskrit and Hindi curriculum content do not overlap this
tranche and do not change the collision inventory counts. PR #13421 stays
classified by the existing Venture portable-host-bridge owner rather than
creating a duplicate owner.

The dependency/leverage pass selects
`build-tool-extra-ci-toolchain-crlf-grammar-cross-runtime-hardening` on branch
`codex/build-tool-extra-ci-toolchain-crlf-grammar-hardening`. It is the sole
ready dependency shared by F# and nine other unselected existing-engine
declaration consumers. The tranche makes the neutral grammar byte-exact:
only a CR immediately preceding an LF terminator is stripped, while a final
lone CR or CR before trailing ASCII whitespace remains content and makes the
lookalike inert. A dedicated language-neutral case, the Python reference, and
the canonical Go oracle move together. The change remains process-free and
adds no checkout, process, environment, network, filesystem-write, execution,
or OCaml-promotion authority.

After lifecycle reconciliation and selection, the complete graph contains
566 owners: 167 merged, 398 pending, and exactly one in-progress owner. There
is no active parity PR.

## Post-#13434 Refresh and F# Declaration-Facade Selection

All 41 final checks for PR #13434 reached terminal acceptable conclusions: 34
successes, six expected skips, and one neutral result. GitHub reported final
head `c2a3408d06a9f8c558c5d1ade4bcec8ad649f2f8` conflict-free. The loop enabled
squash auto-merge, and GitHub merged the PR at 2026-08-28T11:12:06Z as
`e76f211644452f24dade7031c3573540e6b672c9` without a manual merge command.
The active parity slot is therefore clear.

The collision-checked schema-3 inventory, refreshed after a conflict-free
rebase onto exact current `origin/main`
`0668a379a94d8071388d8f2cba2a52327f0c8ab3`, remains 15 established lanes,
1,388 implementation identities, 4,602 package slots, and 1,427 all-reported
identities. The four completion bands remain 175/265, 123/934, 170/2,139, and
920/12,880; Rust has 731 singletons, OCaml remains emerging at zero packages,
and canonical collisions and unknown buckets remain zero.

The intervening Intel 4004 functional and gate-level audits, Forme root-site
generation, Mosaic Flutter elevation work, FLOW-MATIC fixture expansion,
Mermaid sequence fixtures, and human-language owner sharding add no package
identity. They remain within existing owners, so no duplicate or newly unowned
topology work is added.

Five later exact-main commits converge the Venture CSS cascade, add Kannada
vocalic-r curriculum evidence, harden HTML frameset whitespace, add the
Macsyma universal JIT, and track human-validated language mocks. They remain
within their existing package, frontend, curriculum, language-parity, and
human-language owners, add no build-tool identity or fixture contract, and
have no exact overlap with this tranche.

The dependency/leverage pass selects
`build-tool-fsharp-extra-ci-toolchain-declaration-conformance` on branch
`codex/build-tool-fsharp-extra-ci-toolchain-declaration-conformance`. Its
neutral corpus, CRLF grammar, and C# shared-engine dependencies are all merged.
The tranche must expose the bounded snapshot evaluator through an F# symbol
and independently consume every neutral toolchain-detection fixture; inherited
C# coverage or CLI delegation alone does not count. This is the narrowest
ready shared-engine lane proof and advances the remaining-engine umbrella
without widening filesystem, Git, process, environment, network, execution,
credential, or OCaml-promotion authority.

The implemented F# symbol is a no-inline, process-free wrapper over the
reviewed bounded C# snapshot evaluator. Its independent test dynamically
discovers all 11 neutral toolchain-detection fixtures and compares each exact
outcome, all 16 canonical toolchain flags, and diagnostics. After the final
rebase, all 12 F# tests pass with 100% line, branch, and method coverage for
the F# module. Fantomas 7.0.6, warning-as-error Release build, Release publish,
the exact BUILD commands, 57 C# downstream tests, the 119-case/283-file neutral
corpus, 84 schema and runner tests, the complete Go suite and vet, trimpath
compilation, NuGet vulnerability inspection, and diff checks pass. A fresh Go
build-tool binary evaluates all 45 Starlark BUILD files, discovers 5,112
packages, keeps the five-entry orphan ledger clean, and completes the actual
two-package affected build closure. The facade adds no discovery, Git,
filesystem, process, environment, network, execution, credential, secret,
dependency, or logging authority.

After publication, the complete graph contains 566 owners: 168 merged, 397
pending, and exactly one `pr-open` owner. Ready-for-review PR #13453 is the
sole active parity PR at validated head
`814cb3c1b3d616f8687e9a06a8865541b41093f4`. GitHub reports it mergeable;
required CI and CodeQL checks are queued, so auto-merge remains disabled until
every required check is terminal and acceptable and the branch remains
conflict-free.

## Post-#13453 Refresh and Elixir Declaration-Engine Selection

PR #13453 reached 40 terminal acceptable checks: 33 successes and seven
expected skips. GitHub merged validated head
`511385c52d4c76e6272a875ae34d8476700aa062` as
`0ad7611d3701cacf40555556a7870346acb96c7c` at 2026-08-28T18:50:45Z without a
manual merge command. The state therefore advances the F# owner from
`pr-open` to `merged` and clears the sole active parity PR.

The collision-checked schema-3 inventory at exact current `origin/main`
`ee5068fe7b8161fa549a0d9891372706baa80c40` remains unchanged at 15 established
lanes, 1,388 implementation identities, 4,602 package slots, and 1,427
all-reported identities. The four completion bands remain 175/265, 123/934,
170/2,139, and 920/12,880; Rust has 731 singletons, OCaml remains emerging at
zero packages, and canonical collisions and unknown buckets remain zero. The
14 commits since the prior inventory touched existing package roots without
adding or removing an identity or slot, and the ownership audit found no new
eligible unowned topology or build-tool contract gap.

The nine unfinished native declaration consumers have equal graph leverage:
each advances the one remaining-engine umbrella. The quick readiness pass
selects `build-tool-elixir-extra-ci-toolchain-declaration-conformance` on branch
`codex/build-tool-elixir-extra-ci-toolchain-declaration-conformance`. Elixir is
the first child after the completed shared C#/F# pair and provides the strongest
independent-runtime diversity tie-break while preserving the same process-free
boundary. Its corpus and CRLF grammar dependencies are merged. The local lane
uses exact Elixir 1.18.4 on Erlang/OTP 27.3.4.11 from SHA-256-verified official
release archives.

The selected engine exposes `BuildTool.ToolchainDetection.evaluate_snapshot/5`
and dynamically consumes all 11 neutral cases. The pure evaluator owns the
complete 16-key registry, shared language mappings, platform precedence,
affected and forced-full scheduling, forced workflow toolchains, exact CRLF
declaration grammar, deterministic unsupported diagnostics, and per-file plus
aggregate resource ceilings. Production discovery retains only the raw BUILD
front it already selected and feeds the same evaluator for `--detect-languages`
and emitted plans. No new filesystem enumeration, Git, process, environment,
network, credential, secret, dependency, execution, or logging authority is
introduced.

After lifecycle reconciliation and selection, the complete graph contains 566
owners and 833 dependency edges: 169 merged, 396 pending, and exactly one
`in-progress` owner. There is no active parity PR.

The implementation was rebased without conflict over merged Kannada long-o PR
#13469, whose human-language and TypeScript changes have zero exact overlap with
the Elixir tranche. The test-first regression failed four focused cases on the
absent native module. After implementation and rebase, the full Elixir suite
passes 243 tests with two expected skips; the new module records 96.00% line
coverage. Warning-as-error test and production compilation, production escript
construction, exact Windows BUILD commands, scoped format checks, Hex advisory
inspection, direct affected and forced-full production snapshots, the Elixir
Windows-front contract, neutral conformance suites, complete Go build-tool
suite, vet, trimpath build, collision gate, state DAG audit, and diff hygiene
pass. The repository's pre-existing aggregate Elixir coverage gate remains
below its configured 90% threshold at 58.63%, while this added module clears
focused coverage. Seven live PRs have zero exact changed-path overlap.

Ready-for-review PR #13475 is now the sole active parity PR at validated head
`c01387d3399f86c230a290f2e5402e6bf84d0560`. It was opened after a normal first
push from exact `origin/main` `ee5068fe7b8161fa549a0d9891372706baa80c40`;
all 11 changed paths have zero exact overlap across seven other live pull
requests. The complete graph is now 169 merged, 396 pending, and exactly one
`pr-open` owner. GitHub reports the PR mergeable and non-draft, with CI and
CodeQL checks queued or in progress, so auto-merge remains disabled.

## Autonomous Loop Protocol

Only one parity PR should be active at a time.

1. Fetch `origin/main` and verify the prior PR state.
2. If CI fails, inspect the actual GitHub Actions logs, make a focused fix, run
   local verification, and push to the same PR.
3. If the branch conflicts with `main`, update it carefully and verify the full
   PR diff contains only intended work.
4. If checks are pending, keep monitoring.
5. If the PR is merged, regenerate the report from the new `origin/main`, update
   priorities with any newly discovered work, and select the highest-impact
   unblocked item.
6. Create a fresh `codex/` branch, implement one coherent dependency-shaped
   work item, validate it, push it, and open the next PR.
7. Continue until the report has no unclassified or eligible portable gaps.

Every PR must state what changed, why the selected slice is next, tests run,
remaining gaps, and any packages deliberately classified as non-portable.

## Completion Definition

The parity program is complete when:

- every package identity is classified;
- every `portable` package has a tested pure implementation in every established
  implementation language, or an explicit reviewed `not-applicable` exception;
- native, wrapper, web-only, and target-specific packages have honest tested
  coverage in their applicable lanes;
- canonical identity collisions are zero;
- the reporter and conformance checks run in CI;
- adding a new package cannot silently create an unplanned singleton;
- the generated matrix contains no eligible unowned gap.
