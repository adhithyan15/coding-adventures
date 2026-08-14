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
`feature-normalization`, and this branch closes `document-ast`, so the remaining
seven current gaps are Dart-only:

- deterministic data structures: `binary-search-tree`, `fenwick-tree`, and
  `trie`;
- leaf ciphers: `atbash-cipher`, `scytale-cipher`, and `vigenere-cipher`;
- utility leaf: `uuid`.

Merged PR #9375 closed the generator-level prerequisite found by the post-#9363
fixture audit. Dart's native scaffold generator now emits byte-stable,
schema-v1 empty library profiles and truthful generated-program stdout
profiles, while declaring its own reviewed runtime authority. Existing
nonempty Dart profiles remain owned by the legacy migration review. Close the
remaining seven-package frontier as small coherent PRs on top of that scaffold
contract.

Merged PR #9383 completed the first child item: the zero-dependency PHY00
`trig` leaf and its direct PHY01 `wave` consumer. This closed two Dart-only
14-of-15 gaps while exercising the scaffold and capability contract on a real
dependency chain.

Merged PR #9477 completed the ML child item with the independent `matrix`,
`loss-functions`, and `feature-normalization` leaves. The post-merge leverage
pass selected `document-ast` next: its types-only TE00 model has 68 exact
cross-repository consumers and unlocks substantially more follow-on parity work
than the remaining cipher and data-structure leaves. This branch delivers that
sealed, immutable 24-node model with exhaustive discriminator, containment,
value-semantics, and coverage checks. The cipher trio, trie,
binary-search-tree, fenwick-tree, and UUID are explicit remaining child items;
the existing Dart LZ78 private-trie migration is tracked separately.

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

The 172 packages present in at least ten implementation languages need 275
ports to reach all 15. After Priority 1, select work in this order:

| Language lane | Current high-consensus gaps | Pairing rule |
|---|---:|---|
| C# | 0 | Complete; paired native package wave |
| Dart | 105 | Close the reopened 14-of-15 set, then dependencies before consumers |
| Elixir | 0 | Complete; retain as a reference lane and run conformance fixtures |
| F# | 0 | Complete; paired native package wave |
| Go | 0 | Complete; primary build-tool and portable-core reference lane |
| Haskell | 2 | Finish the generic `event-loop` and pure `brotli` gaps |
| Java | 58 | Move with Kotlin |
| Kotlin | 58 | Move with Java |
| Lua | 0 | Complete; retain as a reference lane and remediate build-tool drift |
| Perl | 0 | Complete; retain as a reference lane and remediate build-tool drift |
| Python | 1 | Classify the remaining self-hosted `python-parser` carefully |
| Ruby | 0 | Complete; retain as a reference lane and run conformance fixtures |
| Rust | 0 | Complete; reference lane for broad and singleton families |
| Swift | 51 | Data structures and generated frontends before native app surfaces |
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
   tranche now provides a 30-case, 11-domain process-free corpus, including
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
