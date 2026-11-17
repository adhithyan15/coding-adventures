# Changelog

All notable changes to `rpg-battle-engine` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Fixed
- `code/specs/rpg-battle-engine.warrant` was invalid Warrant (variant `None` collides with the builtin Option constructor); renamed to `NoStatus` in the document. Document only.

## [0.1.0] - 2026-11-15

### Added
- Initial implementation of `GAME24: rpg-battle-engine`.
- 15-type elemental effectiveness chart (`TYPE_CHART`) with `0x` / `0.5x` / `1x` / `2x` entries and fixed-point dual-type multipliers (`dual_multiplier`, `MULT_ONE = 16`).
- `Creature` / `CreatureSpec` / `BaseStats` with level-derived HP and stats, stage modifiers clamped to `-6..=+6` (Atk/Def/SpA/SpD/Spe/Acc/Eva), status conditions (`Burn`, `Paralysis`, `Poison`, `Sleep`, `Freeze`), PP, XP and level-up (`xp_for_level`, `xp_yield`, `Creature::grant_xp`).
- Level-scaled integer damage formula `calc_damage` with STAB, critical hits, burn, `85..=100` roll and type multiplier.
- Seeded PCG32 `BattleRng` for fully reproducible battles.
- Battle FSM `Battle` / `Phase::{Intro, CommandSelect, TurnResolution, SwitchPrompt, Victory, Defeat, Fled}` with switch/run/catch/priority/speed turn ordering, PP accounting, Struggle, accuracy/evasion, secondary effects, trainer follow-up creatures, capture (`catch_value`, `shake_threshold`, `attempt_capture`) and fleeing (`flee_succeeds`), plus an allocation-free fixed-capacity event log.
- HUD / palette helpers integrating with `palette-indexed` (`GAME20`): `hp_bar_state`, `hp_bar_pixels`, `hp_bar_color`, `hp_bar_palette_index`, `hp_bar_blend_color`, `fade_trigger_for_phase`, `fade_params`.
- Security controls: CWE-20/125/190/369/400/787 guards, safe Rust only (`#![forbid(unsafe_code)]`), zero ambient capabilities (`CAP001`).
- Conformance suite (`tests/conformance.rs`), zero-allocation microbenchmarks (`tests/microbenchmarks.rs`), runtime capability gate (`tests/runtime_capability.rs`).
- Formal verification artifacts: `rpg-battle-engine.warrant`, `assurance-profile.json`, `assurance_report.json`, `effect_certificate.json`.
