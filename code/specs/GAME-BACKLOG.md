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
| `GAME11`–`GAME15c` early arcade | Rust packages exist for collision, arcade SFX, Pong, Breakout, discrete logic, CRT beam, and timer | Rust source present; other supported language lanes need inventory | No unified portable-evidence audit yet; collision, Pong, Breakout, and CRT Warrant drafts are parked. |
| `GAME20`–`GAME25` tile/sprite/RPG | Rust packages exist for indexed palette, tilemap, sprite OAM, RPG grid/battle, and Monster RPG | Rust source present; verify required lanes rather than inferring from GAME00's portable label | Shared independent corpus and cross-language execution audit remain open. |
| `GAME26`–`GAME29b` console devices | GAME26 GBA bridge, GAME27 NES, GAME28 Game Boy, and GAME29 GBA devices are missing from the Rust package tree | No verified portable console-device lane | Start with a narrow independently specified NES register/timing slice. Do not claim a complete console. |
| `SWF01`–`SWF05` Flash foundations | Rust packages exist for SWF container, shape lowering, ActionScript compiler, AVM lifter, and display list | Rust source present; LangVM/host integration is incomplete | Existing packages lack an audited shared corpus across public APIs. SWF06 audio bridge, SWF07 player host, and SWF08 conformance corpus are missing. |
| `GAME30`–`GAME35` 2.5D/3D | No Rust packages found for math3d, raycast, BSP, rasterizer, dungeon, or Quake bridge | No verified lane matrix | Capability is missing; fixed-point/F64 contracts and independent rendering/geometry oracles must precede claims. |
| `GAME40`–`GAME45` flight | No Rust packages found for rigidbody, aero, terrain, HUD, GPU bridge, or flight sim | No verified lane matrix | Capability is missing; host/device integration and deterministic physics evidence remain unbuilt. |
| Networking and tooling | Not decomposed into owned game contracts | No verified lane matrix | Define explicit protocol and tool inputs/outputs only after deterministic simulation foundations. |
| `GAME01` Hamurabi | Multi-language package set present; Rust and other lanes need consumer-by-consumer audit | Rust, Python, TypeScript, Go, C, C++, C#, Dart, Elixir, F#, Haskell, Java, Kotlin, Lua, OCaml, Perl, Ruby, Swift packages exist | Rust `conformance.rs` hardcodes a transcript instead of consuming the shared 31-vector corpus. Go live-corpus execution and sampled Python / TypeScript routes exist, but their exact routes differ and do not establish full normal package CI parity. OCaml README says it consumes `cases.txt`, while its test embeds a literal and BUILD/dune do not refresh that fixture. Package count is not parity evidence. |
| `GAME02` Lunar Lander | Fixed-point contract and several cores present; host/presentation integration is incomplete | Rust, Python, TypeScript, Go, Ruby, Perl, Lua, Elixir packages exist; audit them against supported lanes | Verify each language against shared vectors; do not infer parity from directory presence. Expand generated/property coverage after corpus consumers. |
| Parked F64 specifications | Five game Warrant drafts depend on float support | Warrant does not yet implement F64 | WX-FL1 in `VER-BACKLOG.md` gates restoration; no proof/evidence claim until then. |

## Prioritized delivery

Priority reflects dependencies visible on 2026-11-17: GAME10 already has an exact v1 corpus and a
bounded integration target; existing GAME01 consumer gaps are next because the portable package
set already exists; reusable deterministic contracts precede new device, graphics, and host stacks.
This order avoids duplicating repository-wide `VER-*` and `MUS00`–`MUS08` work.

### P0 — `GBL-001`: Make GAME10 v1 a durable conformance boundary (active; [PR #19602](https://github.com/adhithyan15/coding-adventures/pull/19602))

- [x] Consume all 23 `game-clock-v1` cases from a Rust package test through public APIs. Dispatch all
  five operation tags; compare complete configs, complete states, all step-result fields, and exact
  error variants. Unknown operations and skipped cases fail closed.
- [x] For every rejected frame advance, verify the supplied state remains unchanged. Exercise the
  public mutating wrapper against an untouched clone as well as the immutable API.
- [x] Add negative controls proving the conformance checker rejects an always-`Err` adapter, a
  changed expected success field, and a changed expected error code.
- [x] Register the exact fixture path as an external build input for every required consumer. Test
  exact v1 selection and reject a v10 lookalike. Keep the Rust package's debug/release and capability
  gates intact.
- [x] Classify the result as finite package-corpus conformance only. Do not upgrade Warrant or
  assurance claims without separate evidence.

### P1 — Close existing evidence gaps and Warrant limits

- [ ] `GBL-002` Make Rust Hamurabi consume the shared 31-vector corpus; audit every registered language
  consumer before describing parity. Fix cases where tests merely hardcode a transcript.
- [ ] `GBL-003` Repair GAME10's three Warrant claims and assurance report. Add meaningful positive and negative
  evidence, generated/property and fuzz coverage, and Warrant evidence only through a separately
  reviewed design. A passing finite corpus does not repair a claim that accepts every error.
- [ ] `GBL-004` Add generated/property tests for arithmetic invariants, validation precedence,
  conservation, and atomic failure. Record generators and bounds.
- [ ] `GBL-007` Add bounded fuzzing where package tooling supports it, and record corpus, runtime,
  and configuration limits.
- [ ] `GBL-005` Audit GAME01 and GAME02 lane manifests, package builds, shared-fixture coverage, and host
  boundaries. Keep capability, language, and verification status distinct in updates.
- [ ] `GBL-006` Repair OCaml Hamurabi fixture wiring: its test embeds a literal while its README
  describes `cases.txt`, and its BUILD/dune do not refresh that shared fixture.

### P2 — Port the clock contract

- [ ] `GBL-010` Implement GAME10 public APIs and exact v1 corpus consumption in Python, TypeScript, and Go.
- [ ] `GBL-011` Add fixture external-input/build-selection edges and negative v10 selection tests in each
  affected build lane. Preserve canonical decimal `u64`, bounded numeric `u32`, strict booleans, and
  exact error behavior across runtimes.
- [ ] `GBL-012` Audit remaining supported language lanes and add ports only where those lanes are part of the
  repository's supported package surface.

### P3 — Complete Lunar Lander language coverage

- [ ] `GBL-020` Verify current Rust, Python, TypeScript, Go, Ruby, Perl, Lua, and Elixir packages against the
  normative fixed-point contract and shared vectors.
- [ ] `GBL-021` Add missing consumers or ports only after identifying the repository's required language
  lanes. Keep host rendering and input integration tracked separately from the simulation core.
- [ ] `GBL-022` Specify and verify parked F64 contracts separately; do not describe fixed-point coverage as F64
  coverage.

### P4 — Build reusable 2D and console foundations; finish Flash in slices

- [ ] `GBL-030` Audit public API and shared-corpus evidence for existing GAME11–GAME25 packages;
  record language support separately from package presence.
- [ ] `GBL-031` Define an independent oracle and portable contract for indexed palette primitives.
- [ ] `GBL-032` Define an independent oracle and portable contract for tilemap primitives.
- [ ] `GBL-038` Define an independent oracle and portable contract for sprite/OAM primitives.
- [ ] `GBL-039` Specify and implement a narrow NES register/timing slice with deterministic state transitions,
  then grow device coverage by explicit module boundaries. Full console/PPU/APU/cartridge claims
  require corresponding independent tests.
- [ ] `GBL-033` Implement the missing SWF06 audio bridge.
- [ ] `GBL-044` Implement the missing SWF07 player host.
- [ ] `GBL-045` Implement the missing SWF08 conformance corpus.
- [ ] `GBL-046` Audit SWF01–SWF05 public APIs and LangVM host integration separately.
- [ ] `GBL-034` Audit the five parked Warrant drafts: `code/specs/collision2d.warrant.draft`,
  `code/specs/game-breakout.warrant.draft`, `code/specs/game-pong.warrant.draft`,
  `code/specs/paint-batch-tiles.warrant.draft`, and
  `code/specs/sim-crt-vector-beam.warrant.draft`. Their F64 dependency waits on `VER04p` floats
  (`WX-FL1`; see `VER-BACKLOG.md`); keep this dependency explicit and do not claim proof meanwhile.
- [ ] `GBL-035` Decompose Flash completion into SWF parsing, VM/runtime, display list, and host integration
  slices, each with format or behavior evidence.

### P5 — Decompose 3D, physics, networking, and tooling

- [ ] `GBL-040` Before GAME30–GAME35 implementation, define deterministic fixed-point contracts and tests for
  foundational vector/matrix/quaternion and collision/physics capabilities.
- [ ] `GBL-043` Decide and specify F64 behavior separately where needed, tracking its dependency on
  VER04p / WX-FL1 without treating that Warrant work as game-platform proof.
- [ ] `GBL-041` Add networking and replay tools as explicit protocol/tool contracts with independent vectors,
  then build on verified simulation foundations.
- [ ] `GBL-042` Reconcile GAME00's GAME30–GAME35 and GAME40–GAME45 rows with actual packages, language lanes, device backends,
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
