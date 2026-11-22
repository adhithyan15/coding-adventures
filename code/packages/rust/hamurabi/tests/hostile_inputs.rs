//! Hostile-input and deterministic fuzz test suite for `hamurabi`.

#![forbid(unsafe_code)]

use hamurabi::{
    format_report, new_game, step_year, validate_order, GameStatus, HamurabiError, HamurabiState,
    TurnOrder,
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
}

#[test]
fn hostile_extremes_and_invariant_guards() {
    let base = new_game(1968);

    // Hostile orders
    let hostile_orders = [
        TurnOrder {
            acres_delta: i32::MIN,
            bushels_to_feed: u32::MAX,
            acres_to_plant: u32::MAX,
        },
        TurnOrder {
            acres_delta: i32::MAX,
            bushels_to_feed: u32::MAX,
            acres_to_plant: u32::MAX,
        },
        TurnOrder {
            acres_delta: -1001,
            bushels_to_feed: 0,
            acres_to_plant: 0,
        },
        TurnOrder {
            acres_delta: 0,
            bushels_to_feed: 2801,
            acres_to_plant: 0,
        },
    ];
    for ord in &hostile_orders {
        assert!(validate_order(&base, ord).is_err());
        assert!(step_year(&base, ord).is_err());
    }

    // Hostile corrupted states (even rng_inc, zero population, extreme bushels/acres, bad year/price/harvest)
    let mut bad_rng_inc = base.clone();
    bad_rng_inc.rng_inc = 2; // even
    let ok_order = TurnOrder {
        acres_delta: 0,
        bushels_to_feed: 2000,
        acres_to_plant: 500,
    };
    assert!(matches!(
        step_year(&bad_rng_inc, &ok_order),
        Err(HamurabiError::InvalidOrderBounds)
    ));

    let mut zero_pop = base.clone();
    zero_pop.population = 0;
    assert!(step_year(&zero_pop, &ok_order).is_err());

    let mut max_vals = base.clone();
    max_vals.bushels = u32::MAX;
    max_vals.acres = u32::MAX;
    max_vals.population = u32::MAX;
    assert!(step_year(&max_vals, &ok_order).is_err());

    for bad_year in [0u8, 11, 255] {
        let mut s = base.clone();
        s.year = bad_year;
        assert!(step_year(&s, &ok_order).is_err());
        let _ = format_report(&s);
    }
    for bad_price in [0u32, 16, 27, u32::MAX] {
        let mut s = base.clone();
        s.land_price = bad_price;
        assert!(step_year(&s, &ok_order).is_err());
    }
    for bad_harvest in [0u32, 6, u32::MAX] {
        let mut s = base.clone();
        s.harvest_per_acre = bad_harvest;
        assert!(step_year(&s, &ok_order).is_err());
    }
}

#[test]
fn hostile_fuzz_campaign_2000_cases() {
    let mut rng = SplitMix64(0x4A30_4AB1_0000_0001);

    for i in 0..2_000 {
        let state = HamurabiState {
            year: (rng.next_u32() % 15) as u8,
            population: if i % 10 == 0 {
                0
            } else if i % 11 == 0 {
                u32::MAX
            } else {
                rng.next_u32() % 5_000
            },
            bushels: if i % 7 == 0 {
                u32::MAX
            } else {
                rng.next_u32() % 100_000
            },
            acres: if i % 9 == 0 {
                u32::MAX
            } else {
                rng.next_u32() % 50_000
            },
            land_price: if i % 5 == 0 {
                rng.next_u32()
            } else {
                17 + (rng.next_u32() % 10)
            },
            harvest_per_acre: if i % 6 == 0 {
                rng.next_u32()
            } else {
                1 + (rng.next_u32() % 5)
            },
            rats_eaten: rng.next_u32(),
            starved_last_year: rng.next_u32(),
            starved_total: rng.next_u32(),
            percent_starved_sum: rng.next_u32(),
            immigrants_last_year: rng.next_u32(),
            plague_struck: (i & 1) == 0,
            status: if i % 8 == 0 {
                GameStatus::Impeached {
                    starved_this_turn: 50,
                    percent: 50,
                }
            } else {
                GameStatus::Playing
            },
            rng_state: rng.next_u64(),
            rng_inc: if i % 13 == 0 {
                rng.next_u64() & !1
            } else {
                rng.next_u64() | 1
            },
        };

        let order = TurnOrder {
            acres_delta: match i % 5 {
                0 => i32::MIN,
                1 => i32::MAX,
                _ => rng.next_i32() % 2_000,
            },
            bushels_to_feed: if i % 7 == 0 {
                u32::MAX
            } else {
                rng.next_u32() % 20_000
            },
            acres_to_plant: if i % 9 == 0 {
                u32::MAX
            } else {
                rng.next_u32() % 10_000
            },
        };

        let _ = validate_order(&state, &order);
        let _ = step_year(&state, &order);
        if i % 25 == 0 {
            let _ = format_report(&state);
        }
    }
}
