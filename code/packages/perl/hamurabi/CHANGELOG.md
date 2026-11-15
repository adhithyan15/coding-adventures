# Changelog

All notable changes to `CodingAdventures::Hamurabi` (Perl) will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Pure deterministic simulation core for 1968 mainframe resource allocation simulation (`GAME01`, `HA01`, Warrant `hamurabi v1`).
- `HamurabiError` exception class with all 8 canonical error variants (`GameAlreadyEnded`, `InvalidOrderBounds`, `InsufficientBushelsForLand`, `InsufficientAcresToSell`, `InsufficientBushelsToFeed`, `InsufficientAcresToPlant`, `InsufficientPopulationToPlant`, `InsufficientGrainForSeed`).
- `new_game($seed)` constructor initializing Year 1 Sumerian reign with PCG32 (`stream_id = 54`, `inc = 109`).
- `validate_order($state, $order)` pure validation function verifying all constraints in strict canonical order without state mutation.
- `step_year($state, $order)` atomic two-phase simulation advance implementing exact locked PCG32 draw sequence.
- `format_report($state)` authentic 1970s mainframe teletype report generator.
- 26-CWE weakness closure and strict Perl hardening (rejecting non-integers, floats, garbage strings, undef, refs; zero backtracking regexes; zero string eval; zero command execution; `capabilities: []`).
- Full test suite covering all 31 conformance cases from `cases.json` plus adversarial CWE security tests.

### Security

- Enforced strict presence and u32 bounds validation for all state fields and u64 validation for `rng_state` in `_validate_order_impl`, rejecting missing keys, undef, non-numeric, floats, and refs to prevent infinite loop / NaN in PCG32.
- Added strict lifecycle status validation and plague_struck boolean validation.
- Enforced required u64 integer seed validation in `new_game($seed)`, rejecting undef with `InvalidOrderBounds`.
- Updated `required_capabilities.json` to schema v1 manifest format with `"package": "perl/hamurabi"`, `"capabilities": []`, and explicit justification.
