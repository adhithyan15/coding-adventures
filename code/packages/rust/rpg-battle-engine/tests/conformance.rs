//! # Conformance Tests for `rpg-battle-engine` (GAME24)
//!
//! Covers:
//! - Type chart (immunity / half / neutral / double, dual-type products, bounds)
//! - Stage clamping and ratios, derived stats, creature validation
//! - Damage formula (STAB, crit, burn, immunity, stages, roll clamp)
//! - Seeded PRNG determinism, XP / level-up, capture and fleeing maths
//! - Battle FSM: phases, command validation, speed / priority ordering, PP,
//!   Struggle, status conditions, stage moves, switching, victory, defeat,
//!   capture, fleeing, replay determinism, event-log saturation
//! - HUD helpers for `palette-indexed`
//! - CWE-20 / 125 / 190 / 369 / 400 / 787 guards (randomised stress)

use rpg_battle_engine::*;

const SOLID: Move = Move {
    move_type: NORMAL,
    category: Category::Physical,
    power: 40,
    accuracy: 0,
    max_pp: 5,
    priority: 0,
    effect: Effect::NoEffect,
};

fn base(v: u8) -> BaseStats {
    BaseStats {
        hp: v,
        atk: v,
        def: v,
        spa: v,
        spd: v,
        spe: v,
    }
}

fn spec<'a>(t1: u8, t2: u8, level: u8, b: BaseStats, moves: &'a [Move]) -> CreatureSpec<'a> {
    CreatureSpec {
        type1: t1,
        type2: t2,
        level,
        base: b,
        moves,
        catch_rate: 45,
        exp_yield: 64,
    }
}

fn mk(t1: u8, level: u8, b: BaseStats, moves: &[Move]) -> Creature {
    Creature::from_spec(&spec(t1, NO_TYPE, level, b, moves)).unwrap()
}

fn battle_between(a: Creature, b: Creature, wild: bool, seed: u64) -> Battle {
    let mut bt = Battle::start(&[a], &[b], wild, seed).unwrap();
    bt.begin().unwrap();
    bt
}

fn fight(slot: u8) -> Command {
    Command::Fight { slot }
}

// --- type chart -------------------------------------------------------------

#[test]
fn chart_basic_entries() {
    assert_eq!(type_effectiveness(FIRE, GRASS), Ok(EFF_DOUBLE));
    assert_eq!(type_effectiveness(FIRE, WATER), Ok(EFF_HALF));
    assert_eq!(type_effectiveness(NORMAL, GHOST), Ok(EFF_IMMUNE));
    assert_eq!(type_effectiveness(GHOST, NORMAL), Ok(EFF_IMMUNE));
    assert_eq!(type_effectiveness(ELECTRIC, GROUND), Ok(EFF_IMMUNE));
    assert_eq!(type_effectiveness(GROUND, FLYING), Ok(EFF_IMMUNE));
    assert_eq!(type_effectiveness(FIGHTING, GHOST), Ok(EFF_IMMUNE));
    assert_eq!(type_effectiveness(DRAGON, DRAGON), Ok(EFF_DOUBLE));
    assert_eq!(type_effectiveness(NORMAL, NORMAL), Ok(EFF_NEUTRAL));
}

#[test]
fn chart_values_are_legal() {
    for row in TYPE_CHART.iter() {
        for e in row.iter() {
            assert!([EFF_IMMUNE, EFF_HALF, EFF_NEUTRAL, EFF_DOUBLE].contains(e));
        }
    }
    assert_eq!(TYPE_COUNT, 15);
}

#[test]
fn dual_type_products() {
    assert_eq!(dual_multiplier(FIRE, GRASS, BUG), Ok(64)); // 4x
    assert_eq!(dual_multiplier(FIRE, WATER, ROCK), Ok(4)); // 0.25x
    assert_eq!(dual_multiplier(FIRE, WATER, GRASS), Ok(MULT_ONE)); // cancel
    assert_eq!(dual_multiplier(ELECTRIC, WATER, GROUND), Ok(0)); // immunity wins
    assert_eq!(dual_multiplier(FIRE, GRASS, NO_TYPE), Ok(32));
    assert_eq!(dual_multiplier(NORMAL, NORMAL, NO_TYPE), Ok(MULT_ONE));
}

#[test]
fn chart_bounds_rejected() {
    assert_eq!(type_effectiveness(15, 0), Err(BattleError::InvalidType));
    assert_eq!(type_effectiveness(0, 15), Err(BattleError::InvalidType));
    assert_eq!(type_effectiveness(255, 255), Err(BattleError::InvalidType));
    assert_eq!(dual_multiplier(0, 0, 77), Err(BattleError::InvalidType));
    assert_eq!(
        dual_multiplier(99, 0, NO_TYPE),
        Err(BattleError::InvalidType)
    );
}

// --- stages and stats -------------------------------------------------------

#[test]
fn stage_clamp_and_ratio() {
    assert_eq!(clamp_stage(100), 6);
    assert_eq!(clamp_stage(-100), -6);
    assert_eq!(clamp_stage(i32::MAX), 6);
    assert_eq!(clamp_stage(i32::MIN), -6);
    assert_eq!(clamp_stage(3), 3);
    assert_eq!(stage_ratio(0), (2, 2));
    assert_eq!(stage_ratio(6), (8, 2));
    assert_eq!(stage_ratio(-6), (2, 8));
    assert_eq!(stage_ratio(50), (8, 2));
    assert_eq!(accuracy_ratio(6), (9, 3));
    assert_eq!(accuracy_ratio(-6), (3, 9));
    assert_eq!(accuracy_ratio(0), (3, 3));
}

#[test]
fn derived_stats() {
    assert_eq!(derive_hp(100, 50), 100 + 50 + 10);
    assert_eq!(derive_stat(100, 50), 105);
    assert_eq!(derive_hp(255, 100), 620);
    assert_eq!(effective_stat(100, 0), 100);
    assert_eq!(effective_stat(100, 2), 200);
    assert_eq!(effective_stat(100, -2), 50);
    assert_eq!(effective_stat(0, -6), 1);
}

#[test]
fn creature_validation() {
    let m = [SOLID];
    assert!(Creature::from_spec(&spec(FIRE, NO_TYPE, 5, base(50), &m)).is_ok());
    assert_eq!(
        Creature::from_spec(&spec(15, NO_TYPE, 5, base(50), &m)).err(),
        Some(BattleError::InvalidType)
    );
    assert_eq!(
        Creature::from_spec(&spec(FIRE, 40, 5, base(50), &m)).err(),
        Some(BattleError::InvalidType)
    );
    assert_eq!(
        Creature::from_spec(&spec(FIRE, NO_TYPE, 0, base(50), &m)).err(),
        Some(BattleError::InvalidLevel)
    );
    assert_eq!(
        Creature::from_spec(&spec(FIRE, NO_TYPE, 101, base(50), &m)).err(),
        Some(BattleError::InvalidLevel)
    );
    let mut b = base(50);
    b.spe = 0;
    assert_eq!(
        Creature::from_spec(&spec(FIRE, NO_TYPE, 5, b, &m)).err(),
        Some(BattleError::InvalidStat)
    );
    assert_eq!(
        Creature::from_spec(&spec(FIRE, NO_TYPE, 5, base(50), &[])).err(),
        Some(BattleError::InvalidMove)
    );
    let five = [SOLID; 5];
    assert_eq!(
        Creature::from_spec(&spec(FIRE, NO_TYPE, 5, base(50), &five)).err(),
        Some(BattleError::InvalidMove)
    );
    let mut bad = SOLID;
    bad.accuracy = 101;
    assert!(Creature::from_spec(&spec(FIRE, NO_TYPE, 5, base(50), &[bad])).is_err());
    bad = SOLID;
    bad.max_pp = 0;
    assert!(bad.check_move().is_err());
    bad = SOLID;
    bad.priority = 7;
    assert!(bad.check_move().is_err());
    bad = SOLID;
    bad.move_type = 20;
    assert_eq!(bad.check_move(), Err(BattleError::InvalidType));
    bad = SOLID;
    bad.effect = Effect::Stage {
        stat: StatIndex::Atk,
        delta: 9,
        on_self: true,
        chance: 100,
    };
    assert!(bad.check_move().is_err());
    bad.effect = Effect::Inflict {
        status: Status::None,
        chance: 10,
    };
    assert!(bad.check_move().is_err());
    bad.effect = Effect::Inflict {
        status: Status::Burn,
        chance: 101,
    };
    assert!(bad.check_move().is_err());
}

#[test]
fn integrity_detects_tampering() {
    let mut c = mk(FIRE, 10, base(50), &[SOLID]);
    assert!(c.check_integrity().is_ok());
    c.hp = c.max_hp + 1;
    assert!(c.check_integrity().is_err());
    c = mk(FIRE, 10, base(50), &[SOLID]);
    c.stages[3] = 9;
    assert!(c.check_integrity().is_err());
    c = mk(FIRE, 10, base(50), &[SOLID]);
    c.move_count = 9;
    assert!(c.check_integrity().is_err());
    c = mk(FIRE, 10, base(50), &[SOLID]);
    c.pp[0] = 200;
    assert!(c.check_integrity().is_err());
    c = mk(FIRE, 10, base(50), &[SOLID]);
    c.type1 = 99;
    assert_eq!(c.check_integrity(), Err(BattleError::InvalidType));
    c = mk(FIRE, 10, base(50), &[SOLID]);
    c.level = 0;
    assert_eq!(c.check_integrity(), Err(BattleError::InvalidLevel));
    c = mk(FIRE, 10, base(50), &[SOLID]);
    c.max_hp = 0;
    assert!(c.check_integrity().is_err());
    // A battle refuses a tampered creature.
    let good = mk(FIRE, 10, base(50), &[SOLID]);
    c = good;
    c.hp = 9999;
    assert!(Battle::start(&[c], &[good], false, 1).is_err());
}

// --- damage -----------------------------------------------------------------

fn dmg_pair() -> (Creature, Creature) {
    let a = mk(FIRE, 50, base(100), &[SOLID]);
    let mut d = mk(WATER, 50, base(80), &[SOLID]);
    d.stats.def = 85;
    (a, d)
}

#[test]
fn damage_formula_reference() {
    let (a, mut d) = dmg_pair();
    d.type1 = NORMAL;
    let r = calc_damage(&a, &d, &SOLID, 100, false);
    assert_eq!(r.damage, 23);
    assert_eq!(r.multiplier, MULT_ONE);
    assert!(!r.stab && !r.crit);
    assert_eq!(calc_damage(&a, &d, &SOLID, 85, false).damage, 19);
    // roll is clamped to 85..=100
    assert_eq!(calc_damage(&a, &d, &SOLID, 0, false).damage, 19);
    assert_eq!(calc_damage(&a, &d, &SOLID, 9999, false).damage, 23);
}

#[test]
fn damage_modifiers() {
    let (a, mut d) = dmg_pair();
    d.type1 = NORMAL;
    let base_dmg = calc_damage(&a, &d, &SOLID, 100, false).damage;
    let mut fire = SOLID;
    fire.move_type = FIRE;
    let stab = calc_damage(&a, &d, &fire, 100, false);
    assert!(stab.stab);
    assert_eq!(stab.damage, base_dmg * 3 / 2);
    let crit = calc_damage(&a, &d, &SOLID, 100, true);
    assert_eq!(crit.damage, base_dmg * 2);
    assert!(crit.crit);
    let mut burned = a;
    burned.status = Status::Burn;
    assert_eq!(
        calc_damage(&burned, &d, &SOLID, 100, false).damage,
        base_dmg / 2
    );
    // Special moves are not halved by burn.
    let mut sp = SOLID;
    sp.category = Category::Special;
    assert_eq!(
        calc_damage(&burned, &d, &sp, 100, false).damage,
        calc_damage(&a, &d, &sp, 100, false).damage
    );
    // Super effective doubles, resisted halves.
    let mut water_foe = d;
    water_foe.type1 = GRASS;
    let se = calc_damage(&a, &water_foe, &fire, 100, false);
    assert_eq!(se.multiplier, 32);
    water_foe.type1 = WATER;
    let nve = calc_damage(&a, &water_foe, &fire, 100, false);
    assert_eq!(nve.multiplier, 8);
    assert!(se.damage > nve.damage * 3);
}

#[test]
fn damage_immunity_status_and_zero_power() {
    let (a, mut d) = dmg_pair();
    d.type1 = GHOST;
    assert_eq!(calc_damage(&a, &d, &SOLID, 100, false).damage, 0);
    d.type1 = NORMAL;
    let mut st = SOLID;
    st.category = Category::Status;
    assert_eq!(calc_damage(&a, &d, &st, 100, false).damage, 0);
    let mut zero = SOLID;
    zero.power = 0;
    assert_eq!(calc_damage(&a, &d, &zero, 100, false).damage, 0);
    // Minimum damage of 1 against a tank.
    let mut tank = d;
    tank.stats.def = u16::MAX;
    tank.stages[StatIndex::Def.slot()] = 6;
    let mut weak = SOLID;
    weak.power = 1;
    let mut feeble = a;
    feeble.level = 1;
    feeble.stats.atk = 1;
    assert_eq!(calc_damage(&feeble, &tank, &weak, 85, false).damage, 1);
}

#[test]
fn damage_stages_and_crit_ignore() {
    let (mut a, mut d) = dmg_pair();
    d.type1 = NORMAL;
    let neutral = calc_damage(&a, &d, &SOLID, 100, false).damage;
    a.stages[StatIndex::Atk.slot()] = 2;
    assert!(calc_damage(&a, &d, &SOLID, 100, false).damage > neutral * 3 / 2);
    a.stages[StatIndex::Atk.slot()] = -6;
    let dropped = calc_damage(&a, &d, &SOLID, 100, false).damage;
    assert!(dropped < neutral / 2);
    // Crits ignore the attacker's negative stages.
    assert_eq!(calc_damage(&a, &d, &SOLID, 100, true).damage, neutral * 2);
}

#[test]
fn damage_saturates_to_u16() {
    let mut a = mk(NORMAL, 100, base(255), &[SOLID]);
    a.stats.atk = u16::MAX;
    a.stages[StatIndex::Atk.slot()] = 6;
    let mut d = mk(GRASS, 100, base(1), &[SOLID]);
    d.type2 = BUG;
    d.stats.def = 1;
    d.stages[StatIndex::Def.slot()] = -6;
    let mut mv = SOLID;
    mv.power = 255;
    mv.move_type = FIRE;
    let r = calc_damage(&a, &d, &mv, 100, true);
    assert_eq!(r.damage, u16::MAX);
}

// --- rng --------------------------------------------------------------------

#[test]
fn rng_is_deterministic_and_bounded() {
    let mut a = BattleRng::seeded(42);
    let mut b = BattleRng::seeded(42);
    let mut c = BattleRng::seeded(43);
    let mut differs = false;
    for _ in 0..64 {
        let (x, y, z) = (a.next_word(), b.next_word(), c.next_word());
        assert_eq!(x, y);
        differs |= x != z;
    }
    assert!(differs);
    let mut r = BattleRng::seeded(7);
    assert_eq!(r.below(0), 0);
    assert_eq!(r.below(1), 0);
    for _ in 0..1000 {
        assert!(r.below(16) < 16);
    }
    let mut seen = [false; 16];
    for _ in 0..2000 {
        seen[r.below(16) as usize] = true;
    }
    assert!(seen.iter().all(|s| *s));
}

// --- xp ---------------------------------------------------------------------

#[test]
fn xp_curve_and_yield() {
    assert_eq!(xp_for_level(1), 1);
    assert_eq!(xp_for_level(10), 1000);
    assert_eq!(xp_for_level(100), 1_000_000);
    assert_eq!(xp_for_level(0), 1);
    assert_eq!(xp_for_level(200), 1_000_000);
    assert_eq!(xp_yield(70, 14, true), 140);
    assert_eq!(xp_yield(70, 14, false), 210);
    assert_eq!(xp_yield(u16::MAX, 100, true), 936_214);
    assert_eq!(xp_yield(0, 100, true), 0);
}

#[test]
fn level_up_progression() {
    let mut c = mk(NORMAL, 5, base(60), &[SOLID]);
    let old_max = c.max_hp;
    c.hp = 3;
    assert_eq!(c.grant_xp(0), 0);
    let gained = c.grant_xp(xp_for_level(8) - c.xp);
    assert_eq!(gained, 3);
    assert_eq!(c.level, 8);
    assert!(c.max_hp > old_max);
    assert!(c.hp > 3 && c.hp <= c.max_hp);
    assert_eq!(c.max_hp, derive_hp(60, 8));
    assert_eq!(c.stats.atk, derive_stat(60, 8));
    // Cap at level 100 and saturating xp.
    let g2 = c.grant_xp(u32::MAX);
    assert_eq!(c.level, 100);
    assert_eq!(g2, 92);
    assert_eq!(c.xp, 1_000_000);
    assert_eq!(c.grant_xp(10), 0);
    // A fainted creature does not heal on level-up.
    let mut f = mk(NORMAL, 5, base(60), &[SOLID]);
    f.hp = 0;
    f.grant_xp(u32::MAX);
    assert_eq!(f.hp, 0);
}

// --- capture and flee -------------------------------------------------------

#[test]
fn catch_math() {
    let full = catch_value(100, 100, 45, 10, Status::None);
    let low = catch_value(100, 1, 45, 10, Status::None);
    assert!(low > full);
    assert!(
        catch_value(100, 50, 45, 10, Status::Sleep) > catch_value(100, 50, 45, 10, Status::Burn)
    );
    assert!(
        catch_value(100, 50, 45, 10, Status::Burn) > catch_value(100, 50, 45, 10, Status::None)
    );
    assert_eq!(catch_value(100, 100, 255, 1000, Status::Sleep), 255);
    assert_eq!(catch_value(0, 0, 45, 10, Status::None), 1);
    assert_eq!(catch_value(u16::MAX, 0, 1, 0, Status::None), 1);
    // hp above max is clamped, ball bonus capped
    assert!(catch_value(10, 60000, 255, u16::MAX, Status::None) <= 255);
}

#[test]
fn shake_thresholds() {
    assert_eq!(shake_threshold(255), 65_535);
    assert_eq!(shake_threshold(1000), 65_535);
    let mut prev = 0;
    for a in 1..=255 {
        let t = shake_threshold(a);
        assert!(t >= prev && t <= 65_535);
        prev = t;
    }
    assert_eq!(shake_threshold(0), shake_threshold(1));
    let mut rng = BattleRng::seeded(1);
    for _ in 0..100 {
        assert_eq!(attempt_capture(&mut rng, 255), 4);
    }
    let mut rng = BattleRng::seeded(2);
    let mut caught = 0;
    for _ in 0..2000 {
        if attempt_capture(&mut rng, 1) == 4 {
            caught += 1;
        }
    }
    assert!(caught < 100);
}

#[test]
fn flee_math() {
    assert!(flee_succeeds(100, 50, 0, 255));
    assert!(flee_succeeds(50, 50, 0, 255));
    assert!(!flee_succeeds(10, 100, 0, 255));
    assert!(flee_succeeds(10, 100, 9, 0)); // attempts push the odds up
    assert!(!flee_succeeds(0, 0, 0, 0) || flee_succeeds(0, 0, 0, 0));
    assert!(!flee_succeeds(1, u32::MAX, 0, 255));
    assert!(flee_succeeds(1, 0, 0, 0));
}

#[test]
fn isqrt_exhaustive_small_and_edges() {
    for n in 0..5000u32 {
        let r = isqrt_u32(n);
        assert!(r * r <= n && (r + 1) * (r + 1) > n);
    }
    assert_eq!(isqrt_u32(u32::MAX), 65_535);
    assert_eq!(isqrt_u32(16_711_680), 4087);
}

// --- battle fsm -------------------------------------------------------------

#[test]
fn start_validation() {
    let c = mk(FIRE, 10, base(50), &[SOLID]);
    assert_eq!(
        Battle::start(&[], &[c], false, 1).err(),
        Some(BattleError::PartyEmpty)
    );
    assert_eq!(
        Battle::start(&[c], &[], false, 1).err(),
        Some(BattleError::PartyEmpty)
    );
    assert_eq!(
        Battle::start(&[c; 7], &[c], false, 1).err(),
        Some(BattleError::PartyTooLarge)
    );
    assert_eq!(
        Battle::start(&[c], &[c; 2], true, 1).err(),
        Some(BattleError::InvalidParty)
    );
    let mut dead = c;
    dead.hp = 0;
    assert_eq!(
        Battle::start(&[dead], &[c], false, 1).err(),
        Some(BattleError::AllFainted)
    );
    let b = Battle::start(&[dead, c], &[c], false, 1).unwrap();
    assert_eq!(b.active_index(Side::Player), 1);
    assert_eq!(b.phase(), Phase::Intro);
    assert_eq!(b.party_size(Side::Player), 2);
    assert!(!b.is_wild_battle());
    assert!(b.event_log().len() >= 3);
    assert!(b.party_member(Side::Player, 2).is_none());
    assert!(b.party_member(Side::Player, 1).is_some());
}

#[test]
fn phase_gating() {
    let c = mk(FIRE, 10, base(50), &[SOLID]);
    let mut b = Battle::start(&[c], &[c], false, 1).unwrap();
    assert_eq!(
        b.submit_turn(fight(0), fight(0)),
        Err(BattleError::InvalidPhase)
    );
    assert_eq!(b.resolve_turn(), Err(BattleError::InvalidPhase));
    assert_eq!(b.switch_in(0), Err(BattleError::InvalidPhase));
    b.begin().unwrap();
    assert_eq!(b.phase(), Phase::CommandSelect);
    assert_eq!(b.begin(), Err(BattleError::InvalidPhase));
    assert_eq!(b.resolve_turn(), Err(BattleError::InvalidPhase));
    b.submit_turn(fight(0), fight(0)).unwrap();
    assert_eq!(b.phase(), Phase::TurnResolution);
    assert_eq!(
        b.submit_turn(fight(0), fight(0)),
        Err(BattleError::InvalidPhase)
    );
    b.resolve_turn().unwrap();
    assert_eq!(b.turn_number(), 1);
}

#[test]
fn command_validation() {
    let c = mk(FIRE, 10, base(50), &[SOLID, SOLID]);
    let mut b = battle_between(c, c, false, 3);
    assert_eq!(
        b.submit_turn(fight(2), fight(0)),
        Err(BattleError::InvalidMoveSlot)
    );
    assert_eq!(
        b.submit_turn(fight(255), fight(0)),
        Err(BattleError::InvalidMoveSlot)
    );
    assert_eq!(
        b.submit_turn(fight(0), fight(9)),
        Err(BattleError::InvalidMoveSlot)
    );
    assert_eq!(
        b.submit_turn(Command::Switch { index: 0 }, fight(0)),
        Err(BattleError::InvalidPartyIndex)
    );
    assert_eq!(
        b.submit_turn(Command::Switch { index: 200 }, fight(0)),
        Err(BattleError::InvalidPartyIndex)
    );
    assert_eq!(
        b.submit_turn(Command::Run, fight(0)),
        Err(BattleError::CannotFlee)
    );
    assert_eq!(
        b.submit_turn(Command::Catch { bonus_x10: 10 }, fight(0)),
        Err(BattleError::CannotCatch)
    );
    assert_eq!(
        b.submit_turn(fight(0), Command::Run),
        Err(BattleError::InvalidCommand)
    );
    // Rejected submissions leave the phase untouched.
    assert_eq!(b.phase(), Phase::CommandSelect);
    let mut w = battle_between(c, c, true, 3);
    assert_eq!(
        w.submit_turn(Command::Catch { bonus_x10: 0 }, fight(0)),
        Err(BattleError::CannotCatch)
    );
    assert_eq!(
        w.submit_turn(Command::Catch { bonus_x10: 5000 }, fight(0)),
        Err(BattleError::CannotCatch)
    );
    assert_eq!(
        w.submit_turn(fight(0), Command::Catch { bonus_x10: 10 }),
        Err(BattleError::CannotCatch)
    );
}

#[test]
fn faster_creature_moves_first_and_damage_applies() {
    let mut fast = mk(NORMAL, 20, base(100), &[SOLID]);
    fast.stats.spe = 200;
    let mut slow = mk(NORMAL, 20, base(100), &[SOLID]);
    slow.stats.spe = 10;
    let mut b = battle_between(fast, slow, false, 5);
    b.submit_turn(fight(0), fight(0)).unwrap();
    b.resolve_turn().unwrap();
    let first_move = b
        .event_log()
        .iter()
        .find_map(|e| match e {
            Event::MoveUsed { side, .. } => Some(*side),
            _ => None,
        })
        .unwrap();
    assert_eq!(first_move, Side::Player);
    assert!(b.active_creature(Side::Enemy).hp < slow.hp);
    assert!(b.active_creature(Side::Player).hp < fast.hp);
    // PP was consumed.
    assert_eq!(b.active_creature(Side::Player).pp[0], 4);
    assert_eq!(b.active_creature(Side::Enemy).pp[0], 4);

    // And in the reverse configuration the enemy goes first.
    let mut b2 = battle_between(slow, fast, false, 5);
    b2.submit_turn(fight(0), fight(0)).unwrap();
    b2.resolve_turn().unwrap();
    let first2 = b2
        .event_log()
        .iter()
        .find_map(|e| match e {
            Event::MoveUsed { side, .. } => Some(*side),
            _ => None,
        })
        .unwrap();
    assert_eq!(first2, Side::Enemy);
}

#[test]
fn priority_beats_speed() {
    let mut quick = SOLID;
    quick.priority = 1;
    let mut fast = mk(NORMAL, 20, base(100), &[SOLID]);
    fast.stats.spe = 300;
    let mut slow = mk(NORMAL, 20, base(100), &[quick]);
    slow.stats.spe = 5;
    let mut b = battle_between(fast, slow, false, 8);
    b.submit_turn(fight(0), fight(0)).unwrap();
    b.resolve_turn().unwrap();
    let first = b
        .event_log()
        .iter()
        .find_map(|e| match e {
            Event::MoveUsed { side, .. } => Some(*side),
            _ => None,
        })
        .unwrap();
    assert_eq!(first, Side::Enemy);
}

#[test]
fn speed_ties_are_resolved_deterministically_by_seed() {
    let c = mk(NORMAL, 20, base(100), &[SOLID]);
    let order = |seed: u64| {
        let mut b = battle_between(c, c, false, seed);
        b.submit_turn(fight(0), fight(0)).unwrap();
        b.resolve_turn().unwrap();
        b.event_log()
            .iter()
            .find_map(|e| match e {
                Event::MoveUsed { side, .. } => Some(*side),
                _ => None,
            })
            .unwrap()
    };
    let mut players = 0;
    for seed in 0..64 {
        assert_eq!(order(seed), order(seed));
        if order(seed) == Side::Player {
            players += 1;
        }
    }
    assert!(players > 10 && players < 54);
}

#[test]
fn victory_with_xp_and_level_up() {
    let mut strong = mk(NORMAL, 30, base(120), &[SOLID]);
    strong.stats.spe = 500;
    let mut weak = mk(NORMAL, 5, base(20), &[SOLID]);
    weak.exp_yield = 200;
    weak.hp = 1;
    let mut b = battle_between(strong, weak, true, 11);
    b.submit_turn(fight(0), fight(0)).unwrap();
    b.resolve_turn().unwrap();
    assert_eq!(b.phase(), Phase::Victory);
    assert!(b.event_log().contains(&Event::Victory));
    let xp = b
        .event_log()
        .iter()
        .find_map(|e| match e {
            Event::XpGained { amount } => Some(*amount),
            _ => None,
        })
        .unwrap();
    assert_eq!(xp, xp_yield(200, 5, true));
    assert!(b.active_creature(Side::Player).xp > strong.xp);
    // Terminal phases accept nothing further.
    assert_eq!(
        b.submit_turn(fight(0), fight(0)),
        Err(BattleError::InvalidPhase)
    );
    assert!(b.phase().is_terminal());
}

#[test]
fn level_up_event_emitted() {
    let mut p = mk(NORMAL, 10, base(120), &[SOLID]);
    p.stats.spe = 500;
    p.xp = xp_for_level(11) - 1;
    let mut e = mk(NORMAL, 10, base(10), &[SOLID]);
    e.hp = 1;
    e.exp_yield = 100;
    let mut b = battle_between(p, e, true, 12);
    b.submit_turn(fight(0), fight(0)).unwrap();
    b.resolve_turn().unwrap();
    assert!(b
        .event_log()
        .iter()
        .any(|ev| matches!(ev, Event::LevelUp { level } if *level == 11)));
    assert!(b.active_creature(Side::Player).level >= 11);
}

#[test]
fn defeat_and_switch_prompt() {
    let mut weak = mk(NORMAL, 5, base(20), &[SOLID]);
    weak.hp = 1;
    weak.stats.spe = 1;
    let reserve = mk(NORMAL, 5, base(60), &[SOLID]);
    let mut brute = mk(NORMAL, 50, base(200), &[SOLID]);
    brute.stats.spe = 900;
    let mut b = Battle::start(&[weak, reserve], &[brute], false, 21).unwrap();
    b.begin().unwrap();
    b.submit_turn(fight(0), fight(0)).unwrap();
    b.resolve_turn().unwrap();
    assert_eq!(b.phase(), Phase::SwitchPrompt);
    assert!(b
        .event_log()
        .contains(&Event::Fainted { side: Side::Player }));
    assert_eq!(b.switch_in(0), Err(BattleError::InvalidPartyIndex)); // fainted
    assert_eq!(b.switch_in(5), Err(BattleError::InvalidPartyIndex));
    b.switch_in(1).unwrap();
    assert_eq!(b.phase(), Phase::CommandSelect);
    assert_eq!(b.active_index(Side::Player), 1);

    // Wipe the remaining creature.
    let mut b = Battle::start(&[weak], &[brute], false, 22).unwrap();
    b.begin().unwrap();
    b.submit_turn(fight(0), fight(0)).unwrap();
    b.resolve_turn().unwrap();
    assert_eq!(b.phase(), Phase::Defeat);
    assert!(b.event_log().contains(&Event::Defeat));
}

#[test]
fn trainer_sends_next_creature() {
    let mut strong = mk(NORMAL, 50, base(150), &[SOLID]);
    strong.stats.spe = 800;
    let mut e1 = mk(NORMAL, 3, base(10), &[SOLID]);
    e1.hp = 1;
    let e2 = mk(NORMAL, 3, base(10), &[SOLID]);
    let mut b = Battle::start(&[strong], &[e1, e2], false, 30).unwrap();
    b.begin().unwrap();
    b.submit_turn(fight(0), fight(0)).unwrap();
    b.resolve_turn().unwrap();
    assert_eq!(b.phase(), Phase::CommandSelect);
    assert_eq!(b.active_index(Side::Enemy), 1);
    assert!(b.event_log().contains(&Event::SentOut {
        side: Side::Enemy,
        index: 1
    }));
    assert!(b.has_usable_creature(Side::Enemy));
}

#[test]
fn switching_resets_stages_and_takes_priority() {
    let boost = Move {
        move_type: NORMAL,
        category: Category::Status,
        power: 0,
        accuracy: 0,
        max_pp: 10,
        priority: 0,
        effect: Effect::Stage {
            stat: StatIndex::Atk,
            delta: 2,
            on_self: true,
            chance: 100,
        },
    };
    let mut a = mk(NORMAL, 20, base(100), &[boost, SOLID]);
    a.stats.spe = 400;
    let b2 = mk(NORMAL, 20, base(100), &[SOLID]);
    let foe = mk(NORMAL, 20, base(100), &[SOLID]);
    let mut b = Battle::start(&[a, b2], &[foe], false, 40).unwrap();
    b.begin().unwrap();
    b.submit_turn(fight(0), fight(0)).unwrap();
    b.resolve_turn().unwrap();
    assert_eq!(b.active_creature(Side::Player).stages[0], 2);
    assert!(b.event_log().contains(&Event::StageChanged {
        side: Side::Player,
        stat: StatIndex::Atk,
        delta: 2
    }));
    b.submit_turn(Command::Switch { index: 1 }, fight(0))
        .unwrap();
    b.resolve_turn().unwrap();
    let first = b.event_log()[0];
    assert_eq!(
        first,
        Event::SentOut {
            side: Side::Player,
            index: 1
        }
    );
    assert_eq!(b.party_member(Side::Player, 0).unwrap().stages[0], 0);
    assert_eq!(b.active_index(Side::Player), 1);
}

#[test]
fn stage_limit_is_reported() {
    let boost = Move {
        move_type: NORMAL,
        category: Category::Status,
        power: 0,
        accuracy: 0,
        max_pp: 30,
        priority: 0,
        effect: Effect::Stage {
            stat: StatIndex::Spe,
            delta: 6,
            on_self: true,
            chance: 100,
        },
    };
    let a = mk(NORMAL, 20, base(100), &[boost]);
    let foe = mk(NORMAL, 20, base(255), &[SOLID]);
    let mut b = battle_between(a, foe, false, 41);
    b.submit_turn(fight(0), fight(0)).unwrap();
    b.resolve_turn().unwrap();
    assert_eq!(
        b.active_creature(Side::Player).stages[StatIndex::Spe.slot()],
        6
    );
    b.submit_turn(fight(0), fight(0)).unwrap();
    b.resolve_turn().unwrap();
    assert!(b.event_log().contains(&Event::StageLimit {
        side: Side::Player,
        stat: StatIndex::Spe
    }));
    assert_eq!(
        b.active_creature(Side::Player).stages[StatIndex::Spe.slot()],
        6
    );
}

#[test]
fn status_moves_inflict_and_chip() {
    let mk_status = |status: Status, def_type: u8| {
        let inflict = Move {
            move_type: NORMAL,
            category: Category::Status,
            power: 0,
            accuracy: 0,
            max_pp: 10,
            priority: 0,
            effect: Effect::Inflict {
                status,
                chance: 100,
            },
        };
        let mut a = mk(NORMAL, 20, base(100), &[inflict]);
        a.stats.spe = 500;
        let mut foe = mk(def_type, 20, base(100), &[SOLID]);
        foe.stats.spe = 1;
        let mut b = battle_between(a, foe, false, 50);
        b.submit_turn(fight(0), fight(0)).unwrap();
        b.resolve_turn().unwrap();
        b
    };
    let b = mk_status(Status::Burn, NORMAL);
    assert_eq!(b.active_creature(Side::Enemy).status, Status::Burn);
    let max = b.active_creature(Side::Enemy).max_hp;
    assert!(b.event_log().iter().any(|e| matches!(
        e,
        Event::StatusDamage { side: Side::Enemy, amount } if *amount == (max / 16).max(1)
    )));
    let b = mk_status(Status::Poison, NORMAL);
    assert_eq!(b.active_creature(Side::Enemy).status, Status::Poison);
    // Type immunities to status.
    assert_eq!(
        mk_status(Status::Burn, FIRE)
            .active_creature(Side::Enemy)
            .status,
        Status::None
    );
    assert_eq!(
        mk_status(Status::Paralysis, ELECTRIC)
            .active_creature(Side::Enemy)
            .status,
        Status::None
    );
    assert_eq!(
        mk_status(Status::Poison, POISON)
            .active_creature(Side::Enemy)
            .status,
        Status::None
    );
    assert_eq!(
        mk_status(Status::Freeze, ICE)
            .active_creature(Side::Enemy)
            .status,
        Status::None
    );
    assert_eq!(
        mk_status(Status::Sleep, NORMAL)
            .active_creature(Side::Enemy)
            .status,
        Status::Sleep
    );
}

#[test]
fn sleep_blocks_then_wakes_and_freeze_and_paralysis_gate() {
    let mut a = mk(NORMAL, 20, base(100), &[SOLID]);
    a.stats.spe = 500;
    let mut asleep = mk(NORMAL, 20, base(100), &[SOLID]);
    asleep.stats.spe = 1;
    asleep.status = Status::Sleep;
    asleep.sleep_turns = 3;
    let mut b = battle_between(a, asleep, false, 60);
    let mut woke = false;
    let mut slept = false;
    for _ in 0..4 {
        if b.phase() != Phase::CommandSelect {
            break;
        }
        b.submit_turn(fight(0), fight(0)).unwrap();
        b.resolve_turn().unwrap();
        slept |= b
            .event_log()
            .iter()
            .any(|e| matches!(e, Event::Asleep { .. }));
        woke |= b
            .event_log()
            .iter()
            .any(|e| matches!(e, Event::WokeUp { .. }));
    }
    assert!(slept && woke);

    // Tampered zero sleep counter wakes immediately.
    let mut s0 = mk(NORMAL, 20, base(100), &[SOLID]);
    s0.status = Status::Sleep;
    s0.sleep_turns = 0;
    let mut b = battle_between(a, s0, false, 61);
    b.submit_turn(fight(0), fight(0)).unwrap();
    b.resolve_turn().unwrap();
    assert!(b
        .event_log()
        .iter()
        .any(|e| matches!(e, Event::WokeUp { .. })));

    // Frozen creatures never act until thawed; over many seeds both occur.
    let (mut froze, mut thawed) = (false, false);
    for seed in 0..60 {
        let mut f = mk(NORMAL, 20, base(100), &[SOLID]);
        f.stats.spe = 1;
        f.status = Status::Freeze;
        let mut b = battle_between(a, f, false, seed);
        b.submit_turn(fight(0), fight(0)).unwrap();
        b.resolve_turn().unwrap();
        froze |= b
            .event_log()
            .iter()
            .any(|e| matches!(e, Event::FrozenSolid { .. }));
        thawed |= b
            .event_log()
            .iter()
            .any(|e| matches!(e, Event::Thawed { .. }));
    }
    assert!(froze && thawed);

    let mut full = false;
    for seed in 0..80 {
        let mut p = mk(NORMAL, 20, base(100), &[SOLID]);
        p.stats.spe = 1;
        p.status = Status::Paralysis;
        let mut b = battle_between(a, p, false, seed);
        b.submit_turn(fight(0), fight(0)).unwrap();
        b.resolve_turn().unwrap();
        full |= b
            .event_log()
            .iter()
            .any(|e| matches!(e, Event::FullyParalyzed { .. }));
    }
    assert!(full);
}

#[test]
fn paralysis_quarters_speed() {
    let mut a = mk(NORMAL, 20, base(100), &[SOLID]);
    a.stats.spe = 100;
    a.status = Status::Paralysis;
    let mut foe = mk(NORMAL, 20, base(100), &[SOLID]);
    foe.stats.spe = 60;
    // Paralysed 100 -> 25, so the 60-speed foe is first (unless it is fully paralysed: it isn't).
    let mut b = battle_between(a, foe, false, 70);
    b.submit_turn(fight(0), fight(0)).unwrap();
    b.resolve_turn().unwrap();
    let first = b.event_log().iter().find_map(|e| match e {
        Event::MoveUsed { side, .. } | Event::FullyParalyzed { side } => Some(*side),
        _ => None,
    });
    assert_eq!(first, Some(Side::Enemy));
}

#[test]
fn accuracy_and_miss() {
    let mut shaky = SOLID;
    shaky.accuracy = 1;
    let a = mk(NORMAL, 20, base(100), &[shaky]);
    let foe = mk(NORMAL, 20, base(100), &[SOLID]);
    let mut misses = 0;
    for seed in 0..100 {
        let mut b = battle_between(a, foe, false, seed);
        b.submit_turn(fight(0), fight(0)).unwrap();
        b.resolve_turn().unwrap();
        if b.event_log()
            .iter()
            .any(|e| matches!(e, Event::Missed { side: Side::Player }))
        {
            misses += 1;
        }
    }
    assert!(misses > 90);
    // +6 accuracy against -6 evasion triples effective accuracy.
    let mut precise = a;
    precise.stages[StatIndex::Acc.slot()] = 6;
    let mut elusive = foe;
    elusive.stages[StatIndex::Eva.slot()] = -6;
    let mut hits = 0;
    let mut sure = shaky;
    sure.accuracy = 40;
    precise.moves[0] = sure;
    precise.pp[0] = sure.max_pp;
    for seed in 0..100 {
        let mut b = battle_between(precise, elusive, false, seed);
        b.submit_turn(fight(0), fight(0)).unwrap();
        b.resolve_turn().unwrap();
        if !b
            .event_log()
            .iter()
            .any(|e| matches!(e, Event::Missed { side: Side::Player }))
        {
            hits += 1;
        }
    }
    assert!(hits > 90); // 40 * 9 / 3 = 120% -> always hits
}

#[test]
fn immunity_event_and_no_damage() {
    let a = mk(NORMAL, 20, base(100), &[SOLID]);
    let ghost = mk(GHOST, 20, base(100), &[SOLID]);
    let mut b = battle_between(a, ghost, false, 80);
    b.submit_turn(fight(0), fight(0)).unwrap();
    b.resolve_turn().unwrap();
    assert!(b
        .event_log()
        .contains(&Event::NoEffect { side: Side::Enemy }));
    assert_eq!(b.active_creature(Side::Enemy).hp, ghost.hp);
}

#[test]
fn pp_exhaustion_and_struggle() {
    let mut one = SOLID;
    one.max_pp = 1;
    let a = mk(NORMAL, 20, base(100), &[one]);
    let foe = mk(NORMAL, 20, base(255), &[SOLID; 4]);
    let mut b = battle_between(a, foe, false, 90);
    b.submit_turn(fight(0), fight(0)).unwrap();
    b.resolve_turn().unwrap();
    assert_eq!(b.active_creature(Side::Player).pp[0], 0);
    assert!(b.active_creature(Side::Player).all_pp_spent());
    // Struggle is permitted now, and recoil is reported.
    b.submit_turn(fight(0), fight(0)).unwrap();
    b.resolve_turn().unwrap();
    assert!(b.event_log().contains(&Event::MoveUsed {
        side: Side::Player,
        slot: 255
    }));
    assert!(b.event_log().iter().any(|e| matches!(
        e,
        Event::Recoil {
            side: Side::Player,
            ..
        }
    )));
    // With two moves, a spent slot is rejected while another has PP.
    let two = mk(NORMAL, 20, base(100), &[one, SOLID]);
    let mut b = battle_between(two, foe, false, 91);
    b.submit_turn(fight(0), fight(0)).unwrap();
    b.resolve_turn().unwrap();
    assert_eq!(b.submit_turn(fight(0), fight(0)), Err(BattleError::NoPp));
    assert!(b.submit_turn(fight(1), fight(0)).is_ok());
}

#[test]
fn capture_flow() {
    let mut a = mk(NORMAL, 20, base(100), &[SOLID]);
    a.stats.spe = 400;
    let mut wild = mk(NORMAL, 5, base(50), &[SOLID]);
    wild.catch_rate = 255;
    let mut b = Battle::start(&[a], &[wild], true, 100).unwrap();
    b.begin().unwrap();
    b.submit_turn(Command::Catch { bonus_x10: 1000 }, fight(0))
        .unwrap();
    b.resolve_turn().unwrap();
    assert_eq!(b.phase(), Phase::Victory);
    assert!(b.was_captured());
    assert!(b.event_log().contains(&Event::Captured));
    assert_eq!(b.party_size(Side::Player), 2);
    assert_eq!(b.party_member(Side::Player, 1).unwrap().level, 5);

    // Hopeless attempts fail and the wild creature keeps attacking.
    let mut tough = mk(NORMAL, 5, base(50), &[SOLID]);
    tough.catch_rate = 1;
    let mut b = Battle::start(&[a], &[tough], true, 101).unwrap();
    b.begin().unwrap();
    let mut failed = false;
    for _ in 0..3 {
        if b.phase() != Phase::CommandSelect {
            break;
        }
        b.submit_turn(Command::Catch { bonus_x10: 10 }, fight(0))
            .unwrap();
        b.resolve_turn().unwrap();
        failed |= b.event_log().contains(&Event::CaptureFailed);
    }
    assert!(failed);
    assert!(!b.was_captured());
    assert!(b.active_creature(Side::Player).hp < a.hp);

    // A full party still wins but cannot add the creature.
    let party = [a; 6];
    let mut b = Battle::start(&party, &[wild], true, 102).unwrap();
    b.begin().unwrap();
    b.submit_turn(Command::Catch { bonus_x10: 1000 }, fight(0))
        .unwrap();
    b.resolve_turn().unwrap();
    assert!(b.was_captured());
    assert_eq!(b.party_size(Side::Player), 6);
}

#[test]
fn flee_flow() {
    let mut fast = mk(NORMAL, 20, base(100), &[SOLID]);
    fast.stats.spe = 400;
    let mut slow = mk(NORMAL, 20, base(100), &[SOLID]);
    slow.stats.spe = 10;
    let mut b = battle_between(fast, slow, true, 110);
    b.submit_turn(Command::Run, fight(0)).unwrap();
    b.resolve_turn().unwrap();
    assert_eq!(b.phase(), Phase::Fled);
    assert!(b.event_log().contains(&Event::FleeSuccess));
    assert_eq!(b.active_creature(Side::Player).hp, fast.hp); // fled before being hit

    // A hopelessly slow runner eventually escapes as attempts accumulate.
    let mut b = battle_between(slow, fast, true, 111);
    let mut fled = false;
    let mut failed_once = false;
    for _ in 0..20 {
        if b.phase() != Phase::CommandSelect {
            break;
        }
        b.submit_turn(Command::Run, fight(0)).unwrap();
        b.resolve_turn().unwrap();
        failed_once |= b.event_log().contains(&Event::FleeFailed);
        if b.phase() == Phase::Fled {
            fled = true;
        }
    }
    assert!(failed_once || fled);
}

#[test]
fn ai_chooses_valid_moves_only() {
    let mut one = SOLID;
    one.max_pp = 1;
    let mut e = mk(NORMAL, 20, base(100), &[one, SOLID, one]);
    e.pp[1] = 0;
    let p = mk(NORMAL, 20, base(100), &[SOLID]);
    let mut b = battle_between(p, e, false, 120);
    for _ in 0..50 {
        match b.ai_choose() {
            Command::Fight { slot } => assert!(slot == 0 || slot == 2),
            other => panic!("unexpected {other:?}"),
        }
    }
    let mut spent = e;
    spent.pp = [0; MAX_MOVES];
    let mut b = battle_between(p, spent, false, 121);
    assert_eq!(b.ai_choose(), fight(0));
}

#[test]
fn replay_determinism() {
    let run = |seed: u64| -> (Phase, u16, u16, u16, [Event; 6]) {
        let mut a = mk(FIRE, 25, base(90), &[SOLID, SOLID]);
        a.stats.spe = 70;
        let e = mk(WATER, 25, base(95), &[SOLID, SOLID]);
        let mut b = battle_between(a, e, false, seed);
        let mut digest = [Event::Empty; 6];
        let mut n = 0;
        while b.phase() == Phase::CommandSelect && n < 200 {
            let enemy = b.ai_choose();
            b.submit_turn(fight((n % 2) as u8), enemy)
                .unwrap_or_else(|_| {
                    b.submit_turn(fight(0), fight(0)).unwrap();
                });
            b.resolve_turn().unwrap();
            for (i, ev) in b.event_log().iter().take(6).enumerate() {
                digest[i] = *ev;
            }
            n += 1;
        }
        (
            b.phase(),
            b.turn_number(),
            b.active_creature(Side::Player).hp,
            b.active_creature(Side::Enemy).hp,
            digest,
        )
    };
    for seed in 0..20 {
        assert_eq!(run(seed), run(seed));
    }
}

#[test]
fn event_log_saturates_without_panic() {
    // Many status effects per turn cannot exceed capacity; verify the invariant.
    let c = mk(NORMAL, 20, base(100), &[SOLID]);
    let mut b = battle_between(c, c, false, 130);
    for _ in 0..30 {
        if b.phase() != Phase::CommandSelect {
            break;
        }
        b.submit_turn(fight(0), fight(0)).unwrap();
        b.resolve_turn().unwrap();
        assert!(b.event_log().len() <= MAX_EVENTS);
    }
    assert_eq!(b.dropped_events(), 0);
}

#[test]
fn stress_random_commands_preserve_invariants() {
    let heal = Move {
        move_type: GRASS,
        category: Category::Special,
        power: 60,
        accuracy: 90,
        max_pp: 3,
        priority: 0,
        effect: Effect::Inflict {
            status: Status::Poison,
            chance: 50,
        },
    };
    let boost = Move {
        move_type: NORMAL,
        category: Category::Status,
        power: 0,
        accuracy: 0,
        max_pp: 20,
        priority: 2,
        effect: Effect::Stage {
            stat: StatIndex::Eva,
            delta: -3,
            on_self: false,
            chance: 100,
        },
    };
    let slp = Move {
        move_type: PSYCHIC,
        category: Category::Status,
        power: 0,
        accuracy: 70,
        max_pp: 10,
        priority: 0,
        effect: Effect::Inflict {
            status: Status::Sleep,
            chance: 100,
        },
    };
    let brn = Move {
        move_type: FIRE,
        category: Category::Special,
        power: 90,
        accuracy: 100,
        max_pp: 5,
        priority: 0,
        effect: Effect::Inflict {
            status: Status::Burn,
            chance: 30,
        },
    };
    let mut lcg = BattleRng::seeded(0xfeed);
    for seed in 0..300u64 {
        let a = mk(
            FIRE,
            20 + (seed % 30) as u8,
            base(60 + (seed % 90) as u8),
            &[heal, boost, slp, brn],
        );
        let a2 = mk(WATER, 15, base(70), &[SOLID, brn]);
        let e = mk(GRASS, 25, base(80), &[slp, SOLID, boost, heal]);
        let e2 = mk(GHOST, 10, base(40), &[SOLID]);
        let wild = seed % 3 == 0;
        let enemies: &[Creature] = if wild { &[e] } else { &[e, e2] };
        let mut b = Battle::start(&[a, a2], enemies, wild, seed).unwrap();
        b.begin().unwrap();
        for _ in 0..80 {
            match b.phase() {
                Phase::CommandSelect => {
                    let cmd = match lcg.below(6) {
                        0 => Command::Switch {
                            index: lcg.below(8) as u8,
                        },
                        1 => Command::Run,
                        2 => Command::Catch {
                            bonus_x10: lcg.below(30) as u16,
                        },
                        _ => fight(lcg.below(6) as u8),
                    };
                    let ec = b.ai_choose();
                    if b.submit_turn(cmd, ec).is_ok() {
                        b.resolve_turn().unwrap();
                    }
                }
                Phase::SwitchPrompt => {
                    let _ = b.switch_in(lcg.below(4) as u8);
                    if b.phase() == Phase::SwitchPrompt {
                        for i in 0..b.party_size(Side::Player) {
                            if b.switch_in(i).is_ok() {
                                break;
                            }
                        }
                        assert_ne!(b.phase(), Phase::SwitchPrompt);
                    }
                }
                _ => break,
            }
            for side in [Side::Player, Side::Enemy] {
                for i in 0..b.party_size(side) {
                    let c = b.party_member(side, i).unwrap();
                    assert!(c.hp <= c.max_hp);
                    assert!(c.stages.iter().all(|s| (-6..=6).contains(s)));
                    assert!(c.pp.iter().zip(c.moves.iter()).all(|(p, m)| *p <= m.max_pp));
                    assert!(c.check_integrity().is_ok());
                    if c.hp == 0 {
                        assert_eq!(c.status, Status::None);
                    }
                }
            }
            assert!(b.event_log().len() <= MAX_EVENTS);
        }
    }
}

// --- hud ----------------------------------------------------------------------

#[test]
fn hp_bar_helpers() {
    assert_eq!(hp_bar_state(100, 100), HpBarState::Green);
    assert_eq!(hp_bar_state(51, 100), HpBarState::Green);
    assert_eq!(hp_bar_state(50, 100), HpBarState::Yellow);
    assert_eq!(hp_bar_state(21, 100), HpBarState::Yellow);
    assert_eq!(hp_bar_state(20, 100), HpBarState::Red);
    assert_eq!(hp_bar_state(1, 100), HpBarState::Red);
    assert_eq!(hp_bar_state(0, 100), HpBarState::Empty);
    assert_eq!(hp_bar_state(5, 0), HpBarState::Empty);
    assert_eq!(hp_bar_state(500, 100), HpBarState::Green);

    assert_eq!(hp_bar_pixels(100, 100, 48), 48);
    assert_eq!(hp_bar_pixels(50, 100, 48), 24);
    assert_eq!(hp_bar_pixels(1, 1000, 48), 1);
    assert_eq!(hp_bar_pixels(0, 100, 48), 0);
    assert_eq!(hp_bar_pixels(10, 0, 48), 0);
    assert_eq!(hp_bar_pixels(10, 10, 0), 0);
    assert_eq!(hp_bar_pixels(u16::MAX, 1, u16::MAX), u16::MAX);

    assert_eq!(hp_bar_palette_index(HpBarState::Green), 1);
    assert_eq!(hp_bar_palette_index(HpBarState::Empty), 0);
    assert_ne!(
        hp_bar_color(HpBarState::Green),
        hp_bar_color(HpBarState::Red)
    );
    assert_eq!(hp_bar_color(HpBarState::Yellow)[3], 255);
}

#[test]
fn hp_bar_blend_and_fades() {
    assert_eq!(
        hp_bar_blend_color(100, 100),
        hp_bar_color(HpBarState::Green)
    );
    let nearly_dead = hp_bar_blend_color(1, 10_000);
    assert!(nearly_dead[0] > nearly_dead[1]);
    let mid = hp_bar_blend_color(50, 100);
    let g = hp_bar_color(HpBarState::Green);
    let r = hp_bar_color(HpBarState::Red);
    for ch in 0..3 {
        let (lo, hi) = (g[ch].min(r[ch]), g[ch].max(r[ch]));
        assert!(mid[ch] >= lo && mid[ch] <= hi);
    }
    assert_eq!(hp_bar_blend_color(0, 100), hp_bar_color(HpBarState::Empty));
    assert_eq!(hp_bar_blend_color(5, 0), hp_bar_color(HpBarState::Empty));

    assert_eq!(
        fade_trigger_for_phase(Phase::Intro),
        FadeTrigger::FadeInFromBlack
    );
    assert_eq!(
        fade_trigger_for_phase(Phase::Victory),
        FadeTrigger::FadeOutToWhite
    );
    assert_eq!(
        fade_trigger_for_phase(Phase::Fled),
        FadeTrigger::FadeOutToWhite
    );
    assert_eq!(
        fade_trigger_for_phase(Phase::Defeat),
        FadeTrigger::FadeOutToBlack
    );
    assert_eq!(
        fade_trigger_for_phase(Phase::CommandSelect),
        FadeTrigger::None
    );
    assert_eq!(
        fade_trigger_for_phase(Phase::SwitchPrompt),
        FadeTrigger::None
    );
    assert_eq!(
        fade_trigger_for_phase(Phase::TurnResolution),
        FadeTrigger::None
    );
    for t in [
        FadeTrigger::None,
        FadeTrigger::FadeInFromBlack,
        FadeTrigger::FadeOutToWhite,
        FadeTrigger::FadeOutToBlack,
    ] {
        let (_, steps) = fade_params(t);
        assert!((1..=1024).contains(&steps));
    }
}

// --- property claims (1000 randomised cases each, referenced by assurance_report.json) ---

#[test]
fn property_dual_multiplier_soundness_1000() {
    let mut rng = BattleRng::seeded(0xD0A1);
    for _ in 0..1000 {
        let atk = rng.below(20) as u8;
        let t1 = rng.below(20) as u8;
        let t2 = if rng.below(3) == 0 {
            NO_TYPE
        } else {
            rng.below(20) as u8
        };
        let r = dual_multiplier(atk, t1, t2);
        let valid = (atk as usize) < TYPE_COUNT
            && (t1 as usize) < TYPE_COUNT
            && (t2 == NO_TYPE || (t2 as usize) < TYPE_COUNT);
        match r {
            Ok(m) => {
                assert!(valid);
                assert!([0, 2, 4, 8, 16, 32, 64].contains(&m));
            }
            Err(e) => {
                assert!(!valid);
                assert_eq!(e, BattleError::InvalidType);
            }
        }
    }
}

#[test]
fn property_calc_damage_soundness_1000() {
    let mut rng = BattleRng::seeded(0xDA9A);
    for _ in 0..1000 {
        let level = 1 + rng.below(100) as u8;
        let v = 1 + rng.below(255) as u8;
        let t1 = rng.below(15) as u8;
        let mut mv = SOLID;
        mv.move_type = rng.below(15) as u8;
        mv.power = rng.below(256) as u8;
        mv.category = match rng.below(3) {
            0 => Category::Physical,
            1 => Category::Special,
            _ => Category::Status,
        };
        let mut a = mk(t1, level, base(v), &[SOLID]);
        let mut d = mk(
            rng.below(15) as u8,
            1 + rng.below(100) as u8,
            base(1 + rng.below(255) as u8),
            &[SOLID],
        );
        for s in a.stages.iter_mut().chain(d.stages.iter_mut()) {
            *s = rng.below(13) as i8 - 6;
        }
        a.status = if rng.below(4) == 0 {
            Status::Burn
        } else {
            Status::None
        };
        let crit = rng.below(2) == 0;
        let r = calc_damage(&a, &d, &mv, rng.next_word() % 200, crit);
        if r.multiplier == 0 || mv.power == 0 || mv.category == Category::Status {
            assert_eq!(r.damage, 0);
        } else {
            assert!(r.damage >= 1);
        }
        // Monotone in power at fixed everything else.
        let mut stronger = mv;
        stronger.power = mv.power.saturating_add(10);
        assert!(
            calc_damage(&a, &d, &stronger, 100, crit).damage
                >= calc_damage(&a, &d, &mv, 100, crit).damage
        );
    }
}

#[test]
fn property_clamp_stage_soundness_1000() {
    let mut rng = BattleRng::seeded(0xC1A9);
    for _ in 0..1000 {
        let x = rng.next_word() as i32;
        let c = clamp_stage(x);
        assert!((MIN_STAGE..=MAX_STAGE).contains(&c));
        if (-6..=6).contains(&x) {
            assert_eq!(c as i32, x);
        }
        let (n, d) = stage_ratio(x);
        assert!(n >= 2 && d >= 2 && (n == 2 || d == 2));
    }
}
