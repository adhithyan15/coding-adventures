//! # Unit and Conformance Tests for Hamurabi Simulation Core
//!
//! Covers:
//! - Initial game state verification (`new_game`)
//! - All 8 `HamurabiError` error arms (refusal conditions and canonical order)
//! - Integer overflow guards in debug and release profiles (CWE-190)
//! - Full 10-year deterministic game transcripts
//! - Plague events and population halving
//! - Impeachment condition (> 45% starvation in a single year)
//! - Rating classifications (`Charlemagne`, `Acre`, `Nero`, `Disaster`)
//! - Teletype report formatting (`format_report`)

use hamurabi::{
    format_report, new_game, step_year, validate_order, GameStatus, HamurabiError, Rating,
    TurnOrder,
};

#[test]
fn test_initial_state() {
    let state = new_game(1337);
    assert_eq!(state.year, 1);
    assert_eq!(state.population, 100);
    assert_eq!(state.bushels, 2800);
    assert_eq!(state.acres, 1000);
    assert!(state.land_price >= 17 && state.land_price <= 26);
    assert_eq!(state.harvest_per_acre, 3);
    assert_eq!(state.rats_eaten, 200);
    assert_eq!(state.starved_last_year, 0);
    assert_eq!(state.starved_total, 0);
    assert_eq!(state.percent_starved_sum, 0);
    assert_eq!(state.immigrants_last_year, 5);
    assert!(!state.plague_struck);
    assert_eq!(state.status, GameStatus::Playing);
    assert_eq!(state.rng_inc & 1, 1);
}

#[test]
fn test_all_eight_error_arms() {
    let state = new_game(1337);

    // Arm 1: GameAlreadyEnded
    let mut ended_state = state.clone();
    ended_state.status = GameStatus::Won {
        rating: Rating::Charlemagne,
    };
    let normal_order = TurnOrder {
        acres_delta: 0,
        bushels_to_feed: 2000,
        acres_to_plant: 100,
    };
    assert_eq!(
        validate_order(&ended_state, &normal_order),
        Err(HamurabiError::GameAlreadyEnded)
    );
    assert_eq!(
        step_year(&ended_state, &normal_order),
        Err(HamurabiError::GameAlreadyEnded)
    );

    // Arm 2: InvalidOrderBounds (corrupted land price or arithmetic overflow on trade)
    let mut bad_state = state.clone();
    bad_state.land_price = 10; // valid range is 17..=26
    assert_eq!(
        validate_order(&bad_state, &normal_order),
        Err(HamurabiError::InvalidOrderBounds)
    );

    // Also test overflow in bushels_after_trade:
    let mut rich_land_state = state.clone();
    rich_land_state.acres = 2_000_000_000;
    rich_land_state.bushels = 4_000_000_000;
    let overflow_sell = TurnOrder {
        acres_delta: -1_000_000_000, // 1B * 20 = 20B bushels, overflows u32
        bushels_to_feed: 0,
        acres_to_plant: 0,
    };
    assert_eq!(
        validate_order(&rich_land_state, &overflow_sell),
        Err(HamurabiError::InvalidOrderBounds)
    );

    // Arm 3: InsufficientBushelsForLand (trying to buy more than bushels allow)
    let buy_too_much = TurnOrder {
        acres_delta: 200, // at >= 17 bushels/acre, 200 * 17 = 3400 > 2800
        bushels_to_feed: 0,
        acres_to_plant: 0,
    };
    assert_eq!(
        validate_order(&state, &buy_too_much),
        Err(HamurabiError::InsufficientBushelsForLand)
    );

    // Arm 4: InsufficientAcresToSell (selling more acres than owned)
    let sell_too_much = TurnOrder {
        acres_delta: -1001, // city owns 1000 acres
        bushels_to_feed: 0,
        acres_to_plant: 0,
    };
    assert_eq!(
        validate_order(&state, &sell_too_much),
        Err(HamurabiError::InsufficientAcresToSell)
    );

    // Arm 5: InsufficientBushelsToFeed (feeding more bushels than available)
    let feed_too_much = TurnOrder {
        acres_delta: 0,
        bushels_to_feed: 2801, // only 2800 in store
        acres_to_plant: 0,
    };
    assert_eq!(
        validate_order(&state, &feed_too_much),
        Err(HamurabiError::InsufficientBushelsToFeed)
    );

    // Arm 6: InsufficientAcresToPlant (planting more acres than owned)
    let plant_too_much_acres = TurnOrder {
        acres_delta: 0,
        bushels_to_feed: 1000,
        acres_to_plant: 1001, // only 1000 acres owned
    };
    assert_eq!(
        validate_order(&state, &plant_too_much_acres),
        Err(HamurabiError::InsufficientAcresToPlant)
    );

    // Arm 7: InsufficientPopulationToPlant (1 person tends 10 acres; 100 people tend 1000 acres)
    let mut small_pop_state = state.clone();
    small_pop_state.population = 50; // can only tend 500 acres
    let plant_too_much_pop = TurnOrder {
        acres_delta: 0,
        bushels_to_feed: 1000,
        acres_to_plant: 501,
    };
    assert_eq!(
        validate_order(&small_pop_state, &plant_too_much_pop),
        Err(HamurabiError::InsufficientPopulationToPlant)
    );

    // Arm 8: InsufficientGrainForSeed (1 bushel seeds 2 acres)
    // 800 acres needs 400 bushels seed. Bushels left after feed: 2800 - 2500 = 300.
    let seed_starved_order = TurnOrder {
        acres_delta: 0,
        bushels_to_feed: 2500,
        acres_to_plant: 800, // needs 400 bushels, but only 300 left
    };
    assert_eq!(
        validate_order(&state, &seed_starved_order),
        Err(HamurabiError::InsufficientGrainForSeed)
    );
}

#[test]
fn test_cwe_190_overflow_guard() {
    let state = new_game(42);

    // Huge acres_delta to buy: acres_delta * land_price must not wrap in checked arithmetic
    let huge_buy = TurnOrder {
        acres_delta: i32::MAX,
        bushels_to_feed: 0,
        acres_to_plant: 0,
    };
    // i32::MAX * land_price > 2800, so InsufficientBushelsForLand is returned cleanly
    assert_eq!(
        validate_order(&state, &huge_buy),
        Err(HamurabiError::InsufficientBushelsForLand)
    );

    // Huge acres_to_plant: u32::MAX
    let huge_plant = TurnOrder {
        acres_delta: 0,
        bushels_to_feed: 0,
        acres_to_plant: u32::MAX,
    };
    // Should be caught by InsufficientAcresToPlant (u32::MAX > 1000)
    assert_eq!(
        validate_order(&state, &huge_plant),
        Err(HamurabiError::InsufficientAcresToPlant)
    );
}

#[test]
fn test_impeachment_over_45_percent_starvation() {
    let state = new_game(100);
    // 100 population, allocate 0 bushels to feed -> 0 fed, 100 starved (100% > 45%)
    let starve_order = TurnOrder {
        acres_delta: 0,
        bushels_to_feed: 0,
        acres_to_plant: 0,
    };
    let next_state = step_year(&state, &starve_order).expect("step_year failed");
    match next_state.status {
        GameStatus::Impeached {
            starved_this_turn,
            percent,
        } => {
            assert_eq!(starved_this_turn, 100);
            assert_eq!(percent, 100);
        }
        other => panic!("expected Impeached status, got {:?}", other),
    }

    let report = format_report(&next_state);
    assert!(report.contains("YOU STARVED 100 PEOPLE IN ONE YEAR!!!"));
    assert!(report.contains("IMPEACHED AND THROWN OUT OF OFFICE"));
}

#[test]
fn test_deterministic_peaceful_turn() {
    let state = new_game(1337);
    let order = TurnOrder {
        acres_delta: 0,
        bushels_to_feed: 2000,
        acres_to_plant: 800,
    };
    let next = step_year(&state, &order).expect("peaceful step should succeed");
    assert_eq!(next.year, 2);
    assert_eq!(next.acres, 1000);
    assert_eq!(next.starved_last_year, 0);
    assert_eq!(next.status, GameStatus::Playing);
}

#[test]
fn test_full_10_year_deterministic_game() {
    let mut state = new_game(4242);

    for expected_year in 1..=10 {
        assert_eq!(state.year, expected_year);
        assert_eq!(state.status, GameStatus::Playing);

        let feed_needed = state.population * 20;

        // If bushels are running low, sell land to ensure food supply
        let acres_delta = if state.bushels < feed_needed + 500 {
            let deficit = (feed_needed + 500) - state.bushels;
            let acres_to_sell = deficit.div_ceil(state.land_price).min(state.acres / 3);
            -(acres_to_sell as i32)
        } else if state.bushels > feed_needed + 2000 {
            // Surplus grain: buy land
            let surplus = state.bushels - (feed_needed + 1000);
            let acres_to_buy = (surplus / state.land_price).min(50);
            acres_to_buy as i32
        } else {
            0
        };

        let acres_after_trade = if acres_delta >= 0 {
            state.acres + acres_delta as u32
        } else {
            state.acres - (-acres_delta as u32)
        };

        let bushels_after_trade = if acres_delta > 0 {
            state.bushels - (acres_delta as u32 * state.land_price)
        } else if acres_delta < 0 {
            state.bushels + (-acres_delta as u32 * state.land_price)
        } else {
            state.bushels
        };

        let bushels_to_feed = feed_needed.min(bushels_after_trade);
        let bushels_remaining = bushels_after_trade - bushels_to_feed;

        let max_plantable = acres_after_trade.min(state.population * 10);
        let max_from_seed = bushels_remaining * 2;
        let acres_to_plant = max_plantable.min(max_from_seed);

        let order = TurnOrder {
            acres_delta,
            bushels_to_feed,
            acres_to_plant,
        };

        match step_year(&state, &order) {
            Ok(next) => state = next,
            Err(e) => panic!("year {} failed: {:?}", expected_year, e),
        }
    }

    // At end of year 10 transition, state.year must be 11
    assert_eq!(state.year, 11);
    match state.status {
        GameStatus::Won { rating } => {
            assert!(matches!(
                rating,
                Rating::Charlemagne | Rating::Acre | Rating::Nero | Rating::Disaster
            ));
            let report = format_report(&state);
            assert!(report.contains("IN YOUR 10-YEAR TERM OF OFFICE"));
        }
        GameStatus::Impeached { .. } => panic!("ruler was impeached with careful play"),
        GameStatus::GameOverExtinction => panic!("population went extinct"),
        GameStatus::Playing => panic!("game should not be Playing at year 11"),
    }
}

#[test]
fn test_ratings_evaluation() {
    let mut won_charlemagne = new_game(1);
    won_charlemagne.year = 11;
    won_charlemagne.status = GameStatus::Won {
        rating: Rating::Charlemagne,
    };
    let rep_c = format_report(&won_charlemagne);
    assert!(rep_c.contains("A FANTASTIC PERFORMANCE!"));

    let mut won_acre = new_game(1);
    won_acre.year = 11;
    won_acre.status = GameStatus::Won {
        rating: Rating::Acre,
    };
    let rep_a = format_report(&won_acre);
    assert!(rep_a.contains("YOUR PERFORMANCE COULD HAVE BEEN WORSE"));

    let mut won_nero = new_game(1);
    won_nero.year = 11;
    won_nero.status = GameStatus::Won {
        rating: Rating::Nero,
    };
    let rep_n = format_report(&won_nero);
    assert!(rep_n.contains("NERO WOULD HAVE BEEN PROUD OF YOU"));

    let mut won_disaster = new_game(1);
    won_disaster.year = 11;
    won_disaster.status = GameStatus::Won {
        rating: Rating::Disaster,
    };
    let rep_d = format_report(&won_disaster);
    assert!(rep_d.contains("OFFICIAL NATIONAL DISASTER"));
}

#[test]
fn test_plague_population_reduction() {
    let mut found_plague = false;
    for seed in 0..1000 {
        let state = new_game(seed);
        let order = TurnOrder {
            acres_delta: 0,
            bushels_to_feed: 2000,
            acres_to_plant: 0,
        };
        if let Ok(next) = step_year(&state, &order) {
            if next.plague_struck {
                found_plague = true;
                assert!(next.population >= 50);
                let report = format_report(&next);
                assert!(report.contains("A HORRIBLE PLAGUE STRUCK! HALF THE PEOPLE DIED."));
                break;
            }
        }
    }
    assert!(
        found_plague,
        "should find a plague occurrence in 1000 seeds"
    );
}

#[test]
fn test_canonical_10_year_charlemagne_fixture() {
    let mut state = new_game(2);
    assert_eq!(state.year, 1);
    assert_eq!(state.population, 100);
    assert_eq!(state.bushels, 2800);
    assert_eq!(state.acres, 1000);
    assert_eq!(state.land_price, 21);

    // Turn 1
    let next = step_year(
        &state,
        &TurnOrder {
            acres_delta: 0,
            bushels_to_feed: 2000,
            acres_to_plant: 1000,
        },
    )
    .unwrap();
    assert_eq!(next.year, 2);
    assert_eq!(next.population, 103);
    assert_eq!(next.bushels, 4300);
    assert_eq!(next.acres, 1000);
    assert_eq!(next.land_price, 23);
    assert_eq!(next.harvest_per_acre, 4);
    assert_eq!(next.rats_eaten, 0);
    assert_eq!(next.immigrants_last_year, 3);
    assert!(!next.plague_struck);
    assert_eq!(
        format_report(&next),
        "HAMURABI: I BEG TO REPORT TO YOU,\nIN YEAR 2, 0 PEOPLE STARVED, 3 CAME TO THE CITY.\nPOPULATION IS NOW 103.\nTHE CITY NOW OWNS 1000 ACRES.\nYOU HARVESTED 4 BUSHELS PER ACRE.\nRATS ATE 0 BUSHELS.\nYOU NOW HAVE 4300 BUSHELS IN STORE.\nLAND IS TRADING AT 23 BUSHELS PER ACRE.\n"
    );
    state = next;

    // Turn 2
    let next = step_year(
        &state,
        &TurnOrder {
            acres_delta: 0,
            bushels_to_feed: 2060,
            acres_to_plant: 1000,
        },
    )
    .unwrap();
    assert_eq!(next.year, 3);
    assert_eq!(next.population, 108);
    assert_eq!(next.bushels, 3740);
    state = next;

    // Turn 3
    let next = step_year(
        &state,
        &TurnOrder {
            acres_delta: 0,
            bushels_to_feed: 2160,
            acres_to_plant: 1000,
        },
    )
    .unwrap();
    assert_eq!(next.year, 4);
    assert_eq!(next.population, 115);
    assert_eq!(next.bushels, 2080);
    state = next;

    // Turn 4
    let next = step_year(
        &state,
        &TurnOrder {
            acres_delta: -38,
            bushels_to_feed: 2300,
            acres_to_plant: 962,
        },
    )
    .unwrap();
    assert_eq!(next.year, 5);
    assert_eq!(next.population, 124);
    assert_eq!(next.bushels, 3983);
    assert_eq!(next.acres, 962);
    state = next;

    // Turn 5
    let next = step_year(
        &state,
        &TurnOrder {
            acres_delta: 0,
            bushels_to_feed: 2480,
            acres_to_plant: 962,
        },
    )
    .unwrap();
    assert_eq!(next.year, 6);
    assert_eq!(next.population, 133);
    assert_eq!(next.bushels, 1984);
    state = next;

    // Turn 6
    let next = step_year(
        &state,
        &TurnOrder {
            acres_delta: -71,
            bushels_to_feed: 2660,
            acres_to_plant: 891,
        },
    )
    .unwrap();
    assert_eq!(next.year, 7);
    assert_eq!(next.population, 138);
    assert_eq!(next.bushels, 2829);
    assert_eq!(next.acres, 891);
    state = next;

    // Turn 7
    let next = step_year(
        &state,
        &TurnOrder {
            acres_delta: -26,
            bushels_to_feed: 2760,
            acres_to_plant: 865,
        },
    )
    .unwrap();
    assert_eq!(next.year, 8);
    assert_eq!(next.population, 76);
    assert_eq!(next.bushels, 1047);
    assert_eq!(next.acres, 865);
    assert!(next.plague_struck);
    state = next;

    // Turn 8
    let next = step_year(
        &state,
        &TurnOrder {
            acres_delta: -43,
            bushels_to_feed: 1520,
            acres_to_plant: 760,
        },
    )
    .unwrap();
    assert_eq!(next.year, 9);
    assert_eq!(next.population, 81);
    assert_eq!(next.bushels, 871);
    assert_eq!(next.rats_eaten, 871);
    state = next;

    // Turn 9
    let next = step_year(
        &state,
        &TurnOrder {
            acres_delta: -71,
            bushels_to_feed: 1620,
            acres_to_plant: 751,
        },
    )
    .unwrap();
    assert_eq!(next.year, 10);
    assert_eq!(next.population, 93);
    assert_eq!(next.bushels, 3979);
    state = next;

    // Turn 10
    let next = step_year(
        &state,
        &TurnOrder {
            acres_delta: 0,
            bushels_to_feed: 1860,
            acres_to_plant: 751,
        },
    )
    .unwrap();
    assert_eq!(next.year, 11);
    assert_eq!(next.population, 55);
    assert_eq!(next.bushels, 4747);
    assert_eq!(
        next.status,
        GameStatus::Won {
            rating: Rating::Charlemagne
        }
    );
    let final_report = format_report(&next);
    assert!(final_report
        .contains("CHARLEMAGNE, DISRAELI, AND JEFFERSON COMBINED COULD NOT HAVE DONE BETTER!"));
}

#[test]
fn test_adversarial_even_rng_inc_refusal() {
    let mut state = new_game(1);
    state.rng_inc = 0; // even inc -> would cause LCG period collapse / infinite loop
    let order = TurnOrder {
        acres_delta: 0,
        bushels_to_feed: 2000,
        acres_to_plant: 1000,
    };
    assert_eq!(
        validate_order(&state, &order),
        Err(HamurabiError::InvalidOrderBounds)
    );
    assert_eq!(
        step_year(&state, &order),
        Err(HamurabiError::InvalidOrderBounds)
    );

    state.rng_inc = 2; // another even inc
    assert_eq!(
        validate_order(&state, &order),
        Err(HamurabiError::InvalidOrderBounds)
    );
}

#[test]
fn test_adversarial_state_inv_bounds() {
    let valid_order = TurnOrder {
        acres_delta: 0,
        bushels_to_feed: 2000,
        acres_to_plant: 1000,
    };

    // Hostile year = 255
    let mut state = new_game(1);
    state.year = 255;
    assert_eq!(
        validate_order(&state, &valid_order),
        Err(HamurabiError::InvalidOrderBounds)
    );

    // Hostile year = 0
    state.year = 0;
    assert_eq!(
        validate_order(&state, &valid_order),
        Err(HamurabiError::InvalidOrderBounds)
    );

    // Hostile year = 11 (valid playing reign is 1..=10)
    state.year = 11;
    assert_eq!(
        validate_order(&state, &valid_order),
        Err(HamurabiError::InvalidOrderBounds)
    );

    // Hostile land_price = 0
    let mut state = new_game(1);
    state.land_price = 0;
    assert_eq!(
        validate_order(&state, &valid_order),
        Err(HamurabiError::InvalidOrderBounds)
    );

    // Hostile land_price = 100 (valid is 17..=26)
    state.land_price = 100;
    assert_eq!(
        validate_order(&state, &valid_order),
        Err(HamurabiError::InvalidOrderBounds)
    );

    // Hostile harvest_per_acre = 99 (valid is 1..=5)
    let mut state = new_game(1);
    state.harvest_per_acre = 99;
    assert_eq!(
        validate_order(&state, &valid_order),
        Err(HamurabiError::InvalidOrderBounds)
    );

    // Hostile harvest_per_acre = 0
    state.harvest_per_acre = 0;
    assert_eq!(
        validate_order(&state, &valid_order),
        Err(HamurabiError::InvalidOrderBounds)
    );
}

#[test]
fn test_adversarial_starved_total_overflow() {
    let mut state = new_game(1);
    state.starved_total = u32::MAX;
    state.percent_starved_sum = u32::MAX - 10;
    let order = TurnOrder {
        acres_delta: 0,
        bushels_to_feed: 1000, // Starves 50 people
        acres_to_plant: 0,
    };
    let next = step_year(&state, &order).expect("saturating addition must not panic");
    assert_eq!(next.starved_total, u32::MAX);
    assert_eq!(next.percent_starved_sum, u32::MAX);
}

#[test]
fn test_adversarial_harvest_overflow_refusal() {
    let mut state = new_game(1);
    state.population = 500_000_000;
    state.acres = 1_000_000_000;
    state.bushels = 2_000_000_000;
    let order = TurnOrder {
        acres_delta: 0,
        bushels_to_feed: 0,
        acres_to_plant: 900_000_000, // 900M * 5 = 4.5B > u32::MAX
    };
    assert_eq!(
        validate_order(&state, &order),
        Err(HamurabiError::InvalidOrderBounds)
    );
    assert_eq!(
        step_year(&state, &order),
        Err(HamurabiError::InvalidOrderBounds)
    );
}
