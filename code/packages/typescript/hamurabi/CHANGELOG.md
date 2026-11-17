# Changelog

All notable changes to the `@coding-adventures/hamurabi` package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-15

### Added
- Initial release of `@coding-adventures/hamurabi`: deterministic simulation core for the seminal 1968 Hamurabi mainframe game (GAME01).
- Pure TypeScript implementation with zero ambient dependencies and zero ambient authority (`"capabilities": []`).
- Exact PCG32 PRNG with stream ID 54 (`inc = 109n`) and 64-bit `BigInt` modular arithmetic.
- Full domain types and interfaces:
  - `HamurabiState`: 15-field immutable value record (`Object.freeze()`).
  - `TurnOrder`: player decisions per turn (`acres_delta`, `bushels_to_feed`, `acres_to_plant`).
  - `GameStatus`: `Playing`, `Won(Rating)`, `Impeached(starved, percent)`, `GameOverExtinction`.
  - `Rating`: `Charlemagne`, `Acre`, `Nero`, `Disaster`.
  - `HamurabiError`: closed 8-variant error enum for order refusal.
  - `Result<T, E>`: pure discriminated union for success and refusal.
- Core simulation APIs:
  - `new_game(seed: bigint | number): HamurabiState`
  - `validate_order(state: HamurabiState, order: TurnOrder): Result<void, HamurabiError>`
  - `step_year(state: HamurabiState, order: TurnOrder): Result<HamurabiState, HamurabiError>`
  - `format_report(state: HamurabiState): string`
- Security hardening and verification:
  - Complete 26-CWE weakness closure (`VER15`).
  - Strict input hardening against `NaN`, `Infinity`, fractional floats, unsafe numbers, and type confusion (CWE-681, CWE-190, CWE-843).
  - Negative zero (`-0`) normalization to `0`.
  - Prototype pollution guards against `__proto__` injection (CWE-1321).
  - Strict enforcement of odd `rng_inc` preventing LCG period collapse / infinite loops (CWE-400, CWE-835).
  - Full conformance suite testing all 31 cases in `cases.json` bit-for-bit with BigInt 64-bit precision preservation.
