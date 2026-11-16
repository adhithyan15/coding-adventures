# rpg-battle-engine

Deterministic, zero-allocation turn-based creature battle state machine for Rust.

Part of the **Coding Adventures** polyglot monorepo historical game engine collection (`GAME24`).

---

## Overview

Console creature-battle RPGs (Pokémon, Dragon Quest, Final Fantasy) share one core: a small deterministic rules engine that takes two commands and produces an ordered list of events. `rpg-battle-engine` models that core with fixed-size `Copy` state so a `(seed, command stream)` pair always replays identically, with no heap allocation on any battle-step or damage path.

1. **Type chart:** 15 elemental types (`NORMAL` … `DRAGON`), `0x` / `0.5x` / `1x` / `2x` per-type entries (`TYPE_CHART`), combined for dual-typed defenders with `dual_multiplier` in fixed point where `MULT_ONE = 16` (so `4x` = 64, `0.25x` = 4).
2. **Creatures:** base stats, level-derived HP and stats, stage modifiers clamped to `-6..=+6` for Atk/Def/SpA/SpD/Spe/Acc/Eva, status conditions (`Burn`, `Paralysis`, `Poison`, `Sleep`, `Freeze`), PP, XP and level-up.
3. **Damage:** `calc_damage` is a level-scaled integer formula with STAB (`1.5x`), critical hits (`2x`, ignoring unfavourable stages), burn halving, an `85..=100` roll, and the type multiplier.
4. **Battle FSM:** `Phase::{Intro, CommandSelect, TurnResolution, SwitchPrompt, Victory, Defeat, Fled}` driven by `Battle::{start, begin, submit_turn, resolve_turn, switch_in}`. Turn order is switch, then run, then catch, then move priority, then effective speed (paralysis quarters speed), with seeded coin-flip tie-breaks. Includes PP accounting, Struggle with recoil, accuracy/evasion, secondary effects, trainer follow-up creatures, capture (`catch_value`, `shake_threshold`, `attempt_capture`) and fleeing (`flee_succeeds`).
5. **HUD / palette integration:** `hp_bar_state`, `hp_bar_pixels`, `hp_bar_color`, `hp_bar_palette_index`, `hp_bar_blend_color` (uses `palette-indexed::fade_color`), and `fade_trigger_for_phase` / `fade_params` to start `palette-indexed` (`GAME20`) fades on battle entry and exit.

---

## Architectural Guarantees

- **`CAP001` (Zero Ambient Authority):** `"capabilities": []` in `required_capabilities.json`.
- **`CAP002` (Zero Unsafe):** `#![forbid(unsafe_code)]`.
- **Zero Allocation Hot Path (`VER07`):** damage, a full turn (`submit_turn` + `resolve_turn`), capture, fleeing and HUD helpers allocate nothing; verified with the counting allocator in `tests/microbenchmarks.rs`. Turn events go to a fixed `MAX_EVENTS = 40` log; overflow is counted in `dropped_events`, never grown.
- **Strict Validation:** CWE-20 (type ids, levels, base stats, move definitions, party sizes, move slots, party indices, command legality per side and battle kind), CWE-125/787 (every roster/array access is bounds-checked or clamped), CWE-190 (HP/PP/XP arithmetic saturates; wide intermediates in `u64`), CWE-369 (divisors guarded; `rng.below(0) == 0`), CWE-400 (fixed capacities; bounded work per turn).
- **Tamper Detection:** `Creature` fields are public, so `Battle::start` re-validates every creature with `Creature::check_integrity`.
- **26-CWE Weakness Closure (`VER15`):** `assurance_report.json` records the closure/exclusion matrix.

---

## Usage

```rust
use rpg_battle_engine::*;

let tackle = Move {
    move_type: NORMAL, category: Category::Physical, power: 40,
    accuracy: 100, max_pp: 35, priority: 0, effect: Effect::NoEffect,
};
let base = BaseStats { hp: 45, atk: 49, def: 49, spa: 65, spd: 65, spe: 45 };
let mine = Creature::from_spec(&CreatureSpec {
    type1: GRASS, type2: POISON, level: 12, base, moves: &[tackle],
    catch_rate: 45, exp_yield: 64,
})?;
let wild = Creature::from_spec(&CreatureSpec {
    type1: FIRE, type2: NO_TYPE, level: 8, base, moves: &[tackle],
    catch_rate: 190, exp_yield: 62,
})?;

let mut battle = Battle::start(&[mine], &[wild], true, 0xC0FFEE)?;
battle.begin()?;
while battle.phase() == Phase::CommandSelect {
    let enemy = battle.ai_choose();
    battle.submit_turn(Command::Fight { slot: 0 }, enemy)?;
    battle.resolve_turn()?;
    for event in battle.event_log() {
        // drive text boxes, animations, HP bars ...
        let _ = event;
    }
}
# Ok::<(), BattleError>(())
```

## Core Types & Functions

- `Battle`, `Phase`, `Command`, `Event`, `Side`, `BattleError`
- `Creature`, `CreatureSpec`, `BaseStats`, `CombatStats`, `Move`, `Category`, `Effect`, `Status`, `StatIndex`
- `TYPE_CHART`, `type_effectiveness`, `dual_multiplier`, `MULT_ONE`
- `clamp_stage`, `stage_ratio`, `accuracy_ratio`, `derive_hp`, `derive_stat`, `effective_stat`
- `calc_damage`, `DamageResult`, `STRUGGLE_MOVE`
- `xp_for_level`, `xp_yield`, `Creature::grant_xp`
- `catch_value`, `shake_threshold`, `attempt_capture`, `flee_succeeds`
- `BattleRng` (seeded PCG32)
- `HpBarState`, `hp_bar_state`, `hp_bar_pixels`, `hp_bar_color`, `hp_bar_palette_index`, `hp_bar_blend_color`, `FadeTrigger`, `fade_trigger_for_phase`, `fade_params`

## Testing

```
cargo test -p rpg-battle-engine -- --nocapture
cargo test -p rpg-battle-engine --release -- --nocapture
```
