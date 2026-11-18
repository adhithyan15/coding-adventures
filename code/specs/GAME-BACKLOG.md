# Game Platform Backlog

This file tracks delivery order across the historical game roadmap. `GAME00` remains the era and
system catalog; this backlog records what is implemented, where it runs, and what evidence exists.
Game work should grow reusable simulation and device primitives that support multiple titles.

## How to read status

Capability, language support, and verification are separate dimensions. A package directory or a
roadmap tier is not proof of a complete capability or cross-language parity. “Present” means a
source package exists; “incomplete” means the stated contract or host path has known gaps; “planned”
means no implementation has been verified here. Language lanes must be audited against the
repository's supported build lanes before claiming parity. Verification names identify evidence
that must be produced, not a promise that the evidence exists already.

| Workstream | Capability status | Language status | Verification status / limit |
| --- | --- | --- | --- |
| `GAME10` fixed-step clock | Rust core and v1 integer contract present; host pacing/render integration is separate and incomplete | Rust implementation; Python is an independent fixture validator, not a port; TS, Go, and other supported lanes remain | 23 vectors (9 success, 14 exact errors); durable Rust consumer is P0. Existing Warrant claims accept any `Err`; assurance report has property counts but empty evidence and null exhaustive/fuzz/proof entries. No formal proof claim. |
| `GAME01` Hamurabi | Multi-language package set present; Rust and other lanes need consumer-by-consumer audit | Rust, Python, TypeScript, Go, C, C++, C#, Dart, Elixir, F#, Haskell, Java, Kotlin, Lua, OCaml, Perl, Ruby, Swift packages exist | Rust `conformance.rs` currently hardcodes a transcript instead of consuming the shared 31-vector corpus. Package count is not parity evidence. Audit all registered consumers and close gaps with exact corpus execution. |
| `GAME02` Lunar Lander | Fixed-point contract and several cores present; host/presentation integration is incomplete | Rust, Python, TypeScript, Go, Ruby, Perl, Lua, Elixir packages exist; audit them against supported lanes | Verify each language against shared vectors; do not infer parity from directory presence. Expand generated/property coverage after corpus consumers. |
| Console / retro primitives | Palette, tile, sprite, and selected device work remain decomposed roadmap items; console capability is missing | No complete portable console platform established | Define narrow independent contracts and oracles first. Start with reusable palette/tile/sprite primitives, then a bounded NES register/timing slice. Do not claim a full console. |
| Flash / SWF runtime | Dedicated Flash-on-LangVM target remains incomplete | LangVM integration incomplete | Decompose parser, bytecode/runtime, display-list, and host integration; verify each against independent format/runtime vectors. |
| 3D, physics, flight, networking, tools | Broad GAME30+ roadmap, with missing 3D and flight capabilities and substantial host/device integration gaps | Per-package lane support is not yet reconciled into a verified matrix | Decompose only after foundational contracts; define deterministic physics/math oracles, then device/network/tooling contracts. F64 contracts are parked and must stay explicit rather than implied by fixed-point work. |

## Prioritized delivery

### P0 — Make GAME10 v1 a durable conformance boundary

- [ ] Consume all 23 `game-clock-v1` cases from a Rust package test through public APIs. Dispatch all
  five operation tags; compare complete configs, complete states, all step-result fields, and exact
  error variants. Unknown operations and skipped cases fail closed.
- [ ] For every rejected frame advance, verify the supplied state remains unchanged. Exercise the
  public mutating wrapper against an untouched clone as well as the immutable API.
- [ ] Add negative controls proving the conformance checker rejects an always-`Err` adapter, a
  changed expected success field, and a changed expected error code.
- [ ] Register the exact fixture path as an external build input for every required consumer. Test
  exact v1 selection and reject a v10 lookalike. Keep the Rust package's debug/release and capability
  gates intact.
- [ ] Classify the result as finite package-corpus conformance only. Do not upgrade Warrant or
  assurance claims without separate evidence.

### P1 — Close existing evidence gaps and Warrant limits

- [ ] Make Rust Hamurabi consume the shared 31-vector corpus; audit every registered language
  consumer before describing parity. Fix cases where tests merely hardcode a transcript.
- [ ] Audit GAME10's three Warrant claims and assurance report. Add meaningful positive and negative
  evidence, generated/property and fuzz coverage, and Warrant evidence only through a separately
  reviewed design. A passing finite corpus does not repair a claim that accepts every error.
- [ ] Add generated/property tests for arithmetic invariants, validation precedence, conservation,
  and atomic failure; add bounded fuzzing where package tooling supports it. Record scope and limits.
- [ ] Audit GAME01 and GAME02 lane manifests, package builds, shared-fixture coverage, and host
  boundaries. Keep capability, language, and verification status distinct in updates.

### P2 — Port the clock contract

- [ ] Implement GAME10 public APIs and exact v1 corpus consumption in Python, TypeScript, and Go.
- [ ] Add fixture external-input/build-selection edges and negative v10 selection tests in each
  affected build lane. Preserve canonical decimal `u64`, bounded numeric `u32`, strict booleans, and
  exact error behavior across runtimes.
- [ ] Audit remaining supported language lanes and add ports only where those lanes are part of the
  repository's supported package surface.

### P3 — Complete Lunar Lander language coverage

- [ ] Verify current Rust, Python, TypeScript, Go, Ruby, Perl, Lua, and Elixir packages against the
  normative fixed-point contract and shared vectors.
- [ ] Add missing consumers or ports only after identifying the repository's required language
  lanes. Keep host rendering and input integration tracked separately from the simulation core.
- [ ] Parked F64 contracts remain an explicit gap; do not describe fixed-point coverage as F64
  coverage.

### P4 — Build reusable 2D and console foundations; finish Flash in slices

- [ ] Define independent oracles and portable contracts for palette, tile, and sprite primitives;
  verify them before building more game showcases on top.
- [ ] Specify and implement a narrow NES register/timing slice with deterministic state transitions,
  then grow device coverage by explicit module boundaries. Full console/PPU/APU/cartridge claims
  require corresponding independent tests.
- [ ] Decompose Flash completion into SWF parsing, VM/runtime, display list, and host integration
  slices, each with format or behavior evidence.

### P5 — Decompose 3D, physics, networking, and tooling

- [ ] Before GAME30+ implementation, define deterministic fixed-point contracts and tests for
  foundational vector/matrix/quaternion and collision/physics capabilities. Decide and specify F64
  behavior separately where needed.
- [ ] Add networking and replay tools as explicit protocol/tool contracts with independent vectors,
  then build on verified simulation foundations.
- [ ] Reconcile GAME00's GAME30–GAME45 rows with actual packages, language lanes, device backends,
  and host integration. Roadmap presence alone is not completion evidence.

## Evidence rules

- Unit tests show the tested examples pass; generated/property tests cover their stated input
  generator and bounds; fuzz runs cover their recorded corpus, duration, and configuration; SAT or
  model checking covers only the encoded model. Report each type separately.
- Differential tests require an independent oracle or implementation and an explicit comparison
  surface. Reusing the implementation's own expected-value function is not differential evidence.
- Warrant evidence must satisfy the actual claim, including positive cases where the claim describes
  success. A universal `Err` path cannot establish successful behavior.
- Never claim exhaustive coverage, formal proof, cross-language parity, or complete host integration
  from package existence or finite examples. State missing domains and unverified lanes.

## Duplicate-work boundary

Reuse the existing `VER-*` assurance and shared test infrastructure; this backlog does not create a
second generic verification roadmap. Reuse `MUS00`–`MUS08` audio/synthesis and transcription work;
game-specific audio adapters belong here only when a concrete integration gap exists. Prioritize
game-owned contracts, portable primitives, and evidence that those capabilities are actually
consumed.
