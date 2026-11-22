//! Hostile-input and deterministic fuzz test suite for `rpg-battle-engine`.

#![forbid(unsafe_code)]

use rpg_battle_engine::{
    attempt_capture, calc_damage, catch_value, clamp_stage, derive_hp, derive_stat,
    dual_multiplier, effective_stat, flee_succeeds, hp_bar_blend_color, hp_bar_pixels,
    hp_bar_state, isqrt_u32, shake_threshold, type_effectiveness, xp_for_level, xp_yield,
    BaseStats, Battle, BattleError, BattleRng, Category, Command, Creature, CreatureSpec, Effect,
    Move, Phase, Side, StatIndex, Status, MAX_BALL_BONUS_X10, MAX_LEVEL, MAX_MOVES, MAX_PARTY,
    MAX_PRIORITY, NO_TYPE, STRUGGLE_MOVE, TYPE_COUNT,
};

struct SplitMix64(u64);

impl SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    fn next_u16(&mut self) -> u16 {
        (self.next_u64() >> 48) as u16
    }

    fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
}

fn valid_move() -> Move {
    Move {
        move_type: 0,
        category: Category::Physical,
        power: 40,
        accuracy: 100,
        max_pp: 35,
        priority: 0,
        effect: Effect::NoEffect,
    }
}

fn valid_creature(type1: u8, level: u8) -> Creature {
    let moves = [valid_move()];
    Creature::from_spec(&CreatureSpec {
        type1,
        type2: NO_TYPE,
        level,
        base: BaseStats {
            hp: 45,
            atk: 55,
            def: 45,
            spa: 50,
            spd: 50,
            spe: 60,
        },
        moves: &moves,
        catch_rate: 190,
        exp_yield: 64,
    })
    .expect("valid creature")
}

#[test]
fn hostile_type_ids_stages_hp_isqrt_and_mutated_move_count() {
    // OOB type IDs across 0..=255.
    for atk in 0..=u8::MAX {
        for def in 0..=u8::MAX {
            let res = type_effectiveness(atk, def);
            if (atk as usize) < TYPE_COUNT && (def as usize) < TYPE_COUNT {
                assert!(res.is_ok());
            } else {
                assert_eq!(res, Err(BattleError::InvalidType));
            }
        }
    }
    assert_eq!(
        dual_multiplier(TYPE_COUNT as u8, 0, NO_TYPE),
        Err(BattleError::InvalidType)
    );
    assert_eq!(
        dual_multiplier(0, 0, TYPE_COUNT as u8),
        Err(BattleError::InvalidType)
    );

    // i32::MIN / i32::MAX stages clamp cleanly.
    assert_eq!(clamp_stage(i32::MIN), -6);
    assert_eq!(clamp_stage(i32::MAX), 6);
    assert!(effective_stat(0, i32::MIN) >= 1);
    assert!(effective_stat(u16::MAX, i32::MAX) >= 1);

    // 0 / u16::MAX HP, stats, ball bonuses, and u32::MAX isqrt_u32.
    assert_eq!(isqrt_u32(0), 0);
    assert_eq!(isqrt_u32(u32::MAX), 65_535);
    assert_eq!(catch_value(0, 0, 0, 0, Status::None), 1);
    assert_eq!(
        catch_value(u16::MAX, u16::MAX, u8::MAX, u16::MAX, Status::Sleep),
        255
    );
    assert_eq!(shake_threshold(0), shake_threshold(1));
    assert_eq!(shake_threshold(u32::MAX), 65_535);
    assert_eq!(hp_bar_pixels(0, 0, 0), 0);
    assert_eq!(hp_bar_pixels(u16::MAX, u16::MAX, u16::MAX), u16::MAX);
    let _ = hp_bar_state(u16::MAX, 0);
    let _ = hp_bar_blend_color(u16::MAX, 0);
    assert!(flee_succeeds(0, 0, u8::MAX, u32::MAX));
    assert_eq!(xp_for_level(0), 1);
    assert_eq!(xp_for_level(u8::MAX), 1_000_000);
    let _ = xp_yield(u16::MAX, u8::MAX, false);

    // Invalid CreatureSpec rejected.
    let mv = [valid_move()];
    assert_eq!(
        Creature::from_spec(&CreatureSpec {
            type1: TYPE_COUNT as u8,
            type2: NO_TYPE,
            level: 5,
            base: BaseStats {
                hp: 30,
                atk: 30,
                def: 30,
                spa: 30,
                spd: 30,
                spe: 30
            },
            moves: &mv,
            catch_rate: 100,
            exp_yield: 50,
        }),
        Err(BattleError::InvalidType)
    );
    assert_eq!(
        Creature::from_spec(&CreatureSpec {
            type1: 0,
            type2: NO_TYPE,
            level: 0,
            base: BaseStats {
                hp: 30,
                atk: 30,
                def: 30,
                spa: 30,
                spd: 30,
                spe: 30
            },
            moves: &mv,
            catch_rate: 100,
            exp_yield: 50,
        }),
        Err(BattleError::InvalidLevel)
    );
    assert_eq!(
        Creature::from_spec(&CreatureSpec {
            type1: 0,
            type2: NO_TYPE,
            level: MAX_LEVEL + 1,
            base: BaseStats {
                hp: 30,
                atk: 30,
                def: 30,
                spa: 30,
                spd: 30,
                spe: 30
            },
            moves: &mv,
            catch_rate: 100,
            exp_yield: 50,
        }),
        Err(BattleError::InvalidLevel)
    );

    // Mutated move_count = 255 caught by check_integrity and safe under all_pp_spent().
    let mut c = valid_creature(0, 5);
    c.move_count = u8::MAX;
    assert_eq!(c.check_integrity(), Err(BattleError::InvalidCreature));
    let _ = c.all_pp_spent();
    assert!(matches!(
        Battle::start(&[c], &[valid_creature(1, 5)], true, 1),
        Err(BattleError::InvalidCreature)
    ));

    // Battle bounds: empty/oversized parties, invalid commands, invalid phase transitions.
    let p = valid_creature(0, 10);
    let e = valid_creature(1, 5);
    assert!(matches!(
        Battle::start(&[], &[e], true, 1),
        Err(BattleError::PartyEmpty)
    ));
    let too_many = [p; MAX_PARTY + 1];
    assert!(matches!(
        Battle::start(&too_many, &[e], false, 1),
        Err(BattleError::PartyTooLarge)
    ));
    assert!(matches!(
        Battle::start(&[p], &[e, e], true, 1),
        Err(BattleError::InvalidParty)
    ));

    let mut battle = Battle::start(&[p], &[e], true, 42).unwrap();
    assert_eq!(battle.resolve_turn(), Err(BattleError::InvalidPhase));
    battle.begin().unwrap();
    assert_eq!(
        battle.submit_turn(Command::Fight { slot: u8::MAX }, Command::Fight { slot: 0 }),
        Err(BattleError::InvalidMoveSlot)
    );
    assert_eq!(
        battle.submit_turn(
            Command::Catch {
                bonus_x10: MAX_BALL_BONUS_X10 + 1
            },
            Command::Fight { slot: 0 }
        ),
        Err(BattleError::CannotCatch)
    );
    assert_eq!(
        battle.submit_turn(
            Command::Switch { index: u8::MAX },
            Command::Fight { slot: 0 }
        ),
        Err(BattleError::InvalidPartyIndex)
    );
}

#[test]
fn hostile_fuzz_1000_cases() {
    let mut rng = SplitMix64(0x2026_0024_BA77_1E00);
    let cats = [Category::Physical, Category::Special, Category::Status];
    let statuses = [
        Status::None,
        Status::Burn,
        Status::Paralysis,
        Status::Poison,
        Status::Sleep,
        Status::Freeze,
    ];
    let stat_idx = [
        StatIndex::Atk,
        StatIndex::Def,
        StatIndex::SpA,
        StatIndex::SpD,
        StatIndex::Spe,
        StatIndex::Acc,
        StatIndex::Eva,
    ];

    let mut brng = BattleRng::seeded(0);
    let mut active_battle = Battle::start(
        &[valid_creature(3, 12), valid_creature(2, 10)],
        &[valid_creature(1, 8)],
        true,
        0x1234,
    )
    .unwrap();
    active_battle.begin().unwrap();

    for i in 0..1000u32 {
        // Fuzz Creature::from_spec with hostile values.
        let moves = [
            Move {
                move_type: rng.next_u8(),
                category: cats[(rng.next_u8() as usize) % cats.len()],
                power: rng.next_u8(),
                accuracy: rng.next_u8(),
                max_pp: rng.next_u8(),
                priority: (rng.next_u8() as i8).clamp(-MAX_PRIORITY - 2, MAX_PRIORITY + 2),
                effect: Effect::NoEffect,
            },
            STRUGGLE_MOVE,
        ];
        let spec = CreatureSpec {
            type1: rng.next_u8(),
            type2: rng.next_u8(),
            level: rng.next_u8(),
            base: BaseStats {
                hp: rng.next_u8(),
                atk: rng.next_u8(),
                def: rng.next_u8(),
                spa: rng.next_u8(),
                spd: rng.next_u8(),
                spe: rng.next_u8(),
            },
            moves: &moves[..((rng.next_u8() as usize) % 3)],
            catch_rate: rng.next_u8(),
            exp_yield: rng.next_u16(),
        };
        let _ = Creature::from_spec(&spec);

        // Fuzz damage, capture, stats, and isqrt.
        let mut att = valid_creature(
            rng.next_u8() % (TYPE_COUNT as u8),
            (rng.next_u8() % 100).max(1),
        );
        let mut def = valid_creature(
            rng.next_u8() % (TYPE_COUNT as u8),
            (rng.next_u8() % 100).max(1),
        );
        att.status = statuses[(rng.next_u8() as usize) % statuses.len()];
        def.status = statuses[(rng.next_u8() as usize) % statuses.len()];
        att.move_count = rng.next_u8();
        let _ = att.all_pp_spent();
        for idx in stat_idx {
            let _ = att.stage_of(idx);
        }
        let _ = calc_damage(
            &att,
            &def,
            &moves[0],
            rng.next_u32(),
            rng.next_u8() & 1 != 0,
        );
        let a = catch_value(
            rng.next_u16(),
            rng.next_u16(),
            rng.next_u8(),
            rng.next_u16(),
            att.status,
        );
        let _ = attempt_capture(&mut brng, a);
        let _ = isqrt_u32(rng.next_u32());
        let _ = derive_hp(rng.next_u8(), rng.next_u8());
        let _ = derive_stat(rng.next_u8(), rng.next_u8());
        let _ = effective_stat(rng.next_u16(), rng.next_i32());

        // Fuzz Battle state machine transitions.
        if active_battle.phase().is_terminal() || i.is_multiple_of(100) {
            active_battle = Battle::start(
                &[valid_creature(3, 12), valid_creature(2, 10)],
                &[valid_creature(1, 8)],
                rng.next_u8() & 1 != 0,
                rng.next_u64(),
            )
            .unwrap();
            let _ = active_battle.begin();
        }
        match active_battle.phase() {
            Phase::CommandSelect => {
                let cmd = match rng.next_u8() % 4 {
                    0 => Command::Fight {
                        slot: rng.next_u8() % (MAX_MOVES as u8 + 2),
                    },
                    1 => Command::Switch {
                        index: rng.next_u8() % (MAX_PARTY as u8 + 2),
                    },
                    2 => Command::Catch {
                        bonus_x10: rng.next_u16(),
                    },
                    _ => Command::Run,
                };
                let foe_cmd = active_battle.ai_choose();
                if active_battle.submit_turn(cmd, foe_cmd).is_ok() {
                    let _ = active_battle.resolve_turn();
                }
            }
            Phase::SwitchPrompt => {
                let _ = active_battle.switch_in(rng.next_u8() % (MAX_PARTY as u8 + 2));
                let _ = active_battle.switch_in(1);
            }
            Phase::Intro => {
                let _ = active_battle.begin();
            }
            Phase::TurnResolution => {
                let _ = active_battle.resolve_turn();
            }
            _ => {}
        }
        let _ = active_battle.party_member(Side::Player, rng.next_u8());
    }
}
