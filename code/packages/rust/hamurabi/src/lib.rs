//! # hamurabi
//!
//! Pure, deterministic simulation core for the seminal 1968 mainframe resource
//! allocation game.
//!
//! ## Historical Lineage
//!
//! - **1964 — The Sumerian Game:** Designed by Mabel Addis (the first credited
//!   female video game designer and writer) and programmed by William McKay for
//!   an IBM 7090 mainframe at the Board of Cooperative Educational Services
//!   (BOCES) in Westchester County, New York. It introduced the economic feedback
//!   loop: governing the city-state of Lagash through annual decisions on land
//!   acquisition, grain allocation for consumption, and agriculture.
//! - **1968 — FOCAL Hamurabi:** Implemented by Richard Merrill on the Digital
//!   Equipment Corporation (DEC) PDP-8 in FOCAL-69, shortening the historical
//!   Sumerian ruler name to fit the 8-character symbol constraint of PDP-8
//!   teletype terminals.
//! - **1973 — 101 BASIC Computer Games:** Adapted to Dartmouth BASIC by David H.
//!   Ahl and published in DEC's landmark book, becoming the archetypal computer
//!   simulation game that influenced an entire generation of programmers.
//!
//! ## Architectural Guarantees
//!
//! In accordance with the repository's verification architecture (`VER00`,
//! `VER04m`, `VER27`, `VER29`):
//! - Strictly zero ambient system authority: `"capabilities": []` declared in
//!   `required_capabilities.json`.
//! - Pure state transition: `step_year(state: &HamurabiState, order: &TurnOrder) -> Result<HamurabiState, HamurabiError>`.
//! - Explicit pseudorandomness: all simulation rolls enter deterministically
//!   through explicit PCG32 state stored directly inside [`HamurabiState`].
//! - Pure integer arithmetic: uses exclusively bounded integer types (`u8`, `u32`,
//!   `u64`, `i32`) with zero floating-point operations, ensuring 100% bit-exact
//!   identical execution traces across all 18 programming languages.
//! - Zero heap allocation: state transitions operate purely on value records
//!   with zero dynamic memory allocation on the simulation hot path (`VER07`).
//! - Complete 26-CWE weakness closure (`VER15`).

#![forbid(unsafe_code)]

#[allow(dead_code)]
pub mod capabilities;

/// LCG multiplier constant for 64-bit PCG32 (Knuth / Numerical Recipes).
const PCG_MULTIPLIER: u64 = 6364136223846793005;

/// Stream ID for PCG32 PRNG stream in Hamurabi simulation.
const HAMURABI_STREAM_ID: u64 = 54;

/// Rating classification awarded to ruler upon completing a 10-year term.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rating {
    /// Average starvation <= 3% AND acres per person >= 10.
    Charlemagne,
    /// Average starvation <= 10% AND acres per person >= 9.
    Acre,
    /// Average starvation <= 33% AND acres per person >= 7.
    Nero,
    /// Average starvation > 33% OR acres per person < 7.
    Disaster,
}

/// Simulation lifecycle status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameStatus {
    /// Active, playable year (years 1..=10).
    Playing,
    /// Completed 10-year term successfully (transitioned to terminal year 11).
    Won { rating: Rating },
    /// Impeached and expelled from office due to extreme starvation (> 45%).
    Impeached {
        starved_this_turn: u32,
        percent: u32,
    },
    /// Entire population died of starvation or disaster.
    GameOverExtinction,
}

/// Closed enumeration of order rejection errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HamurabiError {
    /// Attempted to execute an order when status is not Playing.
    GameAlreadyEnded,
    /// Arithmetic overflow or invalid bounds on input parameters.
    InvalidOrderBounds,
    /// Player ordered to buy land but lacks sufficient bushels.
    InsufficientBushelsForLand,
    /// Player ordered to sell more land than the city owns.
    InsufficientAcresToSell,
    /// Player ordered to feed more bushels than available after land trade.
    InsufficientBushelsToFeed,
    /// Player ordered to plant more acres than owned after land trade.
    InsufficientAcresToPlant,
    /// Player ordered to plant more acres than population can tend (1 person tends 10 acres).
    InsufficientPopulationToPlant,
    /// Player lacks grain to seed fields (1 bushel seeds 2 acres).
    InsufficientGrainForSeed,
}

impl std::fmt::Display for HamurabiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GameAlreadyEnded => write!(f, "Game has already ended"),
            Self::InvalidOrderBounds => write!(
                f,
                "Order parameters exceed valid numerical bounds or overflow"
            ),
            Self::InsufficientBushelsForLand => {
                write!(f, "Insufficient bushels to buy specified acreage")
            }
            Self::InsufficientAcresToSell => write!(f, "Cannot sell more acres than the city owns"),
            Self::InsufficientBushelsToFeed => {
                write!(f, "Insufficient bushels in store to feed population")
            }
            Self::InsufficientAcresToPlant => {
                write!(f, "Cannot plant more acres than owned after land trade")
            }
            Self::InsufficientPopulationToPlant => {
                write!(f, "Insufficient population to tend planted acreage")
            }
            Self::InsufficientGrainForSeed => {
                write!(f, "Insufficient grain to seed planted acreage")
            }
        }
    }
}

impl std::error::Error for HamurabiError {}

/// Immutable value record capturing the complete simulation state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HamurabiState {
    /// Current year of reign: 1..=10 are playable; 11 is terminal summary state.
    pub year: u8,
    /// Current population of the city-state (initial: 100).
    pub population: u32,
    /// Bushels of grain in storage (initial: 2800).
    pub bushels: u32,
    /// Acres of farmland owned by the city (initial: 1000).
    pub acres: u32,
    /// Current market price of land in bushels per acre (17..=26).
    pub land_price: u32,
    /// Yield from the previous harvest in bushels per acre (1..=5).
    pub harvest_per_acre: u32,
    /// Bushels consumed by rats during previous year (initial: 200).
    pub rats_eaten: u32,
    /// Citizens starved during previous year (initial: 0).
    pub starved_last_year: u32,
    /// Cumulative citizens starved across the entire reign.
    pub starved_total: u32,
    /// Sum of yearly starvation percentages (for 10-year average computation).
    pub percent_starved_sum: u32,
    /// New citizens arriving during previous year (initial: 5).
    pub immigrants_last_year: u32,
    /// Whether a plague occurred in the previous year.
    pub plague_struck: bool,
    /// Current game lifecycle status.
    pub status: GameStatus,
    /// Internal 64-bit state of the PCG32 generator.
    pub rng_state: u64,
    /// Internal 64-bit sequence increment of PCG32 (always odd).
    pub rng_inc: u64,
}

/// Player orders for a single year.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TurnOrder {
    /// Acres of land to buy (positive) or sell (negative). 0 indicates no trade.
    pub acres_delta: i32,
    /// Bushels of grain allocated to feed the population (20 bushels per person).
    pub bushels_to_feed: u32,
    /// Acres of owned land to sow with grain (1 bushel seeds 2 acres).
    pub acres_to_plant: u32,
}

// ── PRNG Internal Primitives (PCG32 XSH RR 64/32) ──────────────────────────

#[inline]
fn pcg32_step(state: &mut u64, inc: u64) -> u32 {
    let old_state = *state;
    *state = old_state.wrapping_mul(PCG_MULTIPLIER).wrapping_add(inc);
    let xorshifted = (((old_state >> 18) ^ old_state) >> 27) as u32;
    let rot = (old_state >> 59) as u32;
    xorshifted.rotate_right(rot)
}

fn pcg32_next_int_in_range(state: &mut u64, inc: u64, min: u32, max: u32) -> u32 {
    debug_assert!(min <= max, "min must be <= max");
    let span = (max - min) as u64 + 1;
    let threshold = ((1u64 << 32) - span) % span;
    loop {
        let r = pcg32_step(state, inc) as u64;
        if r >= threshold {
            return min + (r % span) as u32;
        }
    }
}

// ── Simulation Engine API ───────────────────────────────────────────────────

/// Initializes a new, deterministic 10-year reign for ruler Hamurabi.
///
/// Seeds PCG32 with `(seed, stream_id = 54)` and performs the initial draw
/// for Year 1 land price (range 17..=26).
#[must_use]
pub fn new_game(seed: u64) -> HamurabiState {
    let inc = (HAMURABI_STREAM_ID << 1) | 1;
    let mut rng_state = 0u64;
    // Standard PCG initseq warm-up
    rng_state = rng_state.wrapping_mul(PCG_MULTIPLIER).wrapping_add(inc);
    rng_state = rng_state.wrapping_add(seed);
    rng_state = rng_state.wrapping_mul(PCG_MULTIPLIER).wrapping_add(inc);

    // Initial draw: land price for year 1
    let land_price = pcg32_next_int_in_range(&mut rng_state, inc, 17, 26);

    HamurabiState {
        year: 1,
        population: 100,
        bushels: 2800,
        acres: 1000,
        land_price,
        harvest_per_acre: 3,
        rats_eaten: 200,
        starved_last_year: 0,
        starved_total: 0,
        percent_starved_sum: 0,
        immigrants_last_year: 5,
        plague_struck: false,
        status: GameStatus::Playing,
        rng_state,
        rng_inc: inc,
    }
}

/// Validates player orders against current state constraints without advancing state.
///
/// Evaluates checks in the exact canonical order specified by `GAME01`:
/// 1. Status check (`GameStatus::Playing`)
/// 2. Bounds and overflow checks
/// 3. Land trade resource availability
/// 4. Food allocation feasibility
/// 5. Planting acreage and labor limits
/// 6. Seed grain availability
pub fn validate_order(state: &HamurabiState, order: &TurnOrder) -> Result<(), HamurabiError> {
    // 1. Status Check
    if state.status != GameStatus::Playing {
        return Err(HamurabiError::GameAlreadyEnded);
    }

    // State invariant checks (VER04m state_inv & CWE-400 / CWE-835)
    // - Enforce odd rng_inc to prevent infinite loop / short period in PCG32 LCG
    if (state.rng_inc & 1) != 1 {
        return Err(HamurabiError::InvalidOrderBounds);
    }
    // - Enforce reign year in valid turn bounds [1, 10]
    if state.year < 1 || state.year > 10 {
        return Err(HamurabiError::InvalidOrderBounds);
    }
    // - Enforce land price within canonical range [17, 26]
    if state.land_price < 17 || state.land_price > 26 {
        return Err(HamurabiError::InvalidOrderBounds);
    }
    // - Enforce harvest per acre within canonical range [1, 5]
    if state.harvest_per_acre < 1 || state.harvest_per_acre > 5 {
        return Err(HamurabiError::InvalidOrderBounds);
    }

    // 2. Bounds & Overflow Guard on Land Trade
    if order.acres_delta > 0 {
        let buy_count = order.acres_delta as u64;
        let cost = match buy_count.checked_mul(state.land_price as u64) {
            Some(c) => c,
            None => return Err(HamurabiError::InvalidOrderBounds),
        };
        if cost > state.bushels as u64 {
            return Err(HamurabiError::InsufficientBushelsForLand);
        }
    } else if order.acres_delta < 0 {
        let sell_count = match (order.acres_delta as i64).checked_neg() {
            Some(s) if s >= 0 => s as u64,
            _ => return Err(HamurabiError::InvalidOrderBounds),
        };
        if sell_count > state.acres as u64 {
            return Err(HamurabiError::InsufficientAcresToSell);
        }
    }

    // 3. Land Trade Balances
    let acres_after_trade = if order.acres_delta >= 0 {
        match (state.acres as u64).checked_add(order.acres_delta as u64) {
            Some(a) if a <= u32::MAX as u64 => a as u32,
            _ => return Err(HamurabiError::InvalidOrderBounds),
        }
    } else {
        let sell_count = (-(order.acres_delta as i64)) as u64;
        (state.acres as u64 - sell_count) as u32
    };

    let bushels_after_trade = if order.acres_delta > 0 {
        let cost = (order.acres_delta as u64) * (state.land_price as u64);
        (state.bushels as u64 - cost) as u32
    } else if order.acres_delta < 0 {
        let sell_count = (-(order.acres_delta as i64)) as u64;
        let revenue = match sell_count.checked_mul(state.land_price as u64) {
            Some(r) => r,
            None => return Err(HamurabiError::InvalidOrderBounds),
        };
        match (state.bushels as u64).checked_add(revenue) {
            Some(b) if b <= u32::MAX as u64 => b as u32,
            _ => return Err(HamurabiError::InvalidOrderBounds),
        }
    } else {
        state.bushels
    };

    // 4. Food Allocation Check
    if order.bushels_to_feed > bushels_after_trade {
        return Err(HamurabiError::InsufficientBushelsToFeed);
    }
    let bushels_after_feed = bushels_after_trade - order.bushels_to_feed;

    // 5. Planting Land & Labor Checks
    if order.acres_to_plant > acres_after_trade {
        return Err(HamurabiError::InsufficientAcresToPlant);
    }
    let max_labor_acres = match (state.population as u64).checked_mul(10) {
        Some(m) => m,
        None => return Err(HamurabiError::InvalidOrderBounds),
    };
    if (order.acres_to_plant as u64) > max_labor_acres {
        return Err(HamurabiError::InsufficientPopulationToPlant);
    }

    // 6. Planting Seed Check: each bushel seeds 2 acres (ceil(acres / 2))
    let seed_needed = (order.acres_to_plant as u64).div_ceil(2);
    if seed_needed > bushels_after_feed as u64 {
        return Err(HamurabiError::InsufficientGrainForSeed);
    }
    let bushels_after_seed = bushels_after_feed - (seed_needed as u32);

    // 7. Maximum Harvest Overflow Guard (CWE-190)
    // Maximum harvest per acre is 5. Enforce that bushels_after_seed + acres_to_plant * 5 <= u32::MAX.
    let max_harvest = match (order.acres_to_plant as u64).checked_mul(5) {
        Some(h) => h,
        None => return Err(HamurabiError::InvalidOrderBounds),
    };
    let max_stored = match (bushels_after_seed as u64).checked_add(max_harvest) {
        Some(s) => s,
        None => return Err(HamurabiError::InvalidOrderBounds),
    };
    if max_stored > (u32::MAX as u64) {
        return Err(HamurabiError::InvalidOrderBounds);
    }

    Ok(())
}

/// Advances the simulation by exactly one year according to player orders.
///
/// Executes the strict two-phase transition:
/// - **Phase A:** Validation & constraint checking in canonical order.
/// - **Phase B:** Strictly locked PRNG draw sequence (harvest, rats, plague, starvation, immigration, next land price).
///
/// Returns the updated immutable [`HamurabiState`] on success, or [`HamurabiError`] on refusal.
/// The original simulation state is never modified on error.
pub fn step_year(state: &HamurabiState, order: &TurnOrder) -> Result<HamurabiState, HamurabiError> {
    validate_order(state, order)?;

    let mut pcg_state = state.rng_state;
    let pcg_inc = state.rng_inc;

    // Intermediate balances from validated trade
    let acres_after_trade = if order.acres_delta >= 0 {
        state.acres + (order.acres_delta as u32)
    } else {
        let sell_count = (-(order.acres_delta as i64)) as u32;
        state.acres - sell_count
    };

    let bushels_after_trade = if order.acres_delta > 0 {
        let cost = (order.acres_delta as u64) * (state.land_price as u64);
        (state.bushels as u64 - cost) as u32
    } else if order.acres_delta < 0 {
        let sell_count = (-(order.acres_delta as i64)) as u64;
        let revenue = sell_count * (state.land_price as u64);
        (state.bushels as u64 + revenue) as u32
    } else {
        state.bushels
    };

    let bushels_after_feed = bushels_after_trade - order.bushels_to_feed;
    let seed_needed = (order.acres_to_plant as u64).div_ceil(2);
    let bushels_after_seed = bushels_after_feed - (seed_needed as u32);

    // Step 1: Harvest Yield Draw
    // Draw 1: pcg.next_int_in_range(1, 5) -> harvest_per_acre (u32)
    let harvest_per_acre = pcg32_next_int_in_range(&mut pcg_state, pcg_inc, 1, 5);
    let total_harvest = (order.acres_to_plant as u64) * (harvest_per_acre as u64);
    let bushels_stored = (bushels_after_seed as u64) + total_harvest;

    // Step 2: Rat Infestation Draws
    // Draw 2: pcg.next_int_in_range(1, 100) -> rat_roll (u32)
    let rat_roll = pcg32_next_int_in_range(&mut pcg_state, pcg_inc, 1, 100);
    let rats_eaten = if rat_roll <= 40 {
        // Draw 3: pcg.next_int_in_range(1, 5) -> rat_factor (u32)
        let rat_factor = pcg32_next_int_in_range(&mut pcg_state, pcg_inc, 1, 5);
        let eaten = bushels_stored / (rat_factor as u64);
        eaten.min(u32::MAX as u64) as u32
    } else {
        0
    };
    let bushels_final =
        (bushels_stored.saturating_sub(rats_eaten as u64)).min(u32::MAX as u64) as u32;

    // Step 3: Plague Draw
    // Draw 4: pcg.next_int_in_range(1, 100) -> plague_roll (u32)
    let plague_roll = pcg32_next_int_in_range(&mut pcg_state, pcg_inc, 1, 100);
    let plague_struck = plague_roll <= 15;

    // Step 4: Starvation & Impeachment Evaluation
    let people_fed = order.bushels_to_feed / 20;
    let (starved, mut survivors) = if people_fed >= state.population {
        (0u32, state.population)
    } else {
        (state.population - people_fed, people_fed)
    };

    let percent_starved = if state.population > 0 {
        ((starved as u64 * 100) / (state.population as u64)).min(100) as u32
    } else {
        0
    };
    let new_starved_total = state.starved_total.saturating_add(starved);
    let new_percent_sum = state.percent_starved_sum.saturating_add(percent_starved);

    let mut new_status = if (starved as u64) * 100 > (state.population as u64) * 45 {
        GameStatus::Impeached {
            starved_this_turn: starved,
            percent: percent_starved,
        }
    } else if survivors == 0 {
        GameStatus::GameOverExtinction
    } else {
        GameStatus::Playing
    };

    // Step 5: Plague Resolution
    if plague_struck && new_status == GameStatus::Playing {
        survivors /= 2;
    }

    // Step 6: Immigration Draw
    // Draw 5: pcg.next_int_in_range(1, 5) -> imm_factor (u32)
    let imm_factor = pcg32_next_int_in_range(&mut pcg_state, pcg_inc, 1, 5);
    let immigrants = if starved == 0 && new_status == GameStatus::Playing {
        let acres_term = (acres_after_trade as u64).saturating_mul(20);
        let inner = acres_term.saturating_add(bushels_final as u64);
        let numerator = (imm_factor as u64).saturating_mul(inner);
        let denominator = 100 * (state.population as u64);
        numerator
            .checked_div(denominator)
            .map_or(0, |q| q.saturating_add(1).min(u32::MAX as u64) as u32)
    } else {
        0
    };

    let final_population = survivors.saturating_add(immigrants);

    // Step 7: Next Year Land Price Draw
    // Draw 6: pcg.next_int_in_range(17, 26) -> next_land_price (u32)
    let next_land_price = pcg32_next_int_in_range(&mut pcg_state, pcg_inc, 17, 26);

    // Step 8: Turn Advance & Rating
    let next_year = state.year.saturating_add(1);
    if next_year == 11 && new_status == GameStatus::Playing {
        let avg_starved = new_percent_sum / 10;
        let acres_per_person = acres_after_trade.checked_div(final_population).unwrap_or(0);
        let rating = if avg_starved <= 3 && acres_per_person >= 10 {
            Rating::Charlemagne
        } else if avg_starved <= 10 && acres_per_person >= 9 {
            Rating::Acre
        } else if avg_starved <= 33 && acres_per_person >= 7 {
            Rating::Nero
        } else {
            Rating::Disaster
        };
        new_status = GameStatus::Won { rating };
    }

    Ok(HamurabiState {
        year: next_year,
        population: final_population,
        bushels: bushels_final,
        acres: acres_after_trade,
        land_price: next_land_price,
        harvest_per_acre,
        rats_eaten,
        starved_last_year: starved,
        starved_total: new_starved_total,
        percent_starved_sum: new_percent_sum,
        immigrants_last_year: immigrants,
        plague_struck,
        status: new_status,
        rng_state: pcg_state,
        rng_inc: pcg_inc,
    })
}

/// Formats the annual teletype report or final evaluation in authentic 1970s teletype styling.
#[must_use]
pub fn format_report(state: &HamurabiState) -> String {
    match state.status {
        GameStatus::Impeached {
            starved_this_turn, ..
        } => {
            format!(
                "YOU STARVED {} PEOPLE IN ONE YEAR!!!\n\
                 DUE TO THIS EXTREME MISMANAGEMENT YOU HAVE BEEN IMPEACHED AND THROWN OUT OF OFFICE!\n",
                starved_this_turn
            )
        }
        GameStatus::GameOverExtinction => {
            "EVERYONE HAS DIED. THE CITY OF LAGASH IS EXTINCT!\n".to_string()
        }
        GameStatus::Won { rating } => {
            let avg_starved = state.percent_starved_sum / 10;
            let acres_per_person = state.acres.checked_div(state.population).unwrap_or(0);
            let rating_msg = match rating {
                Rating::Charlemagne => {
                    "A FANTASTIC PERFORMANCE! CHARLEMAGNE, DISRAELI, AND JEFFERSON COMBINED COULD NOT HAVE DONE BETTER!"
                }
                Rating::Acre => {
                    "YOUR PERFORMANCE COULD HAVE BEEN WORSE, BUT YOU LEFT THEM REASONABLE ACREAGE."
                }
                Rating::Nero => {
                    "YOUR PERFORMANCE LEAVES MUCH TO BE DESIRED. NERO WOULD HAVE BEEN PROUD OF YOU."
                }
                Rating::Disaster => {
                    "YOU DID SO POORLY YOU HAVE BEEN DECLARED AN OFFICIAL NATIONAL DISASTER!"
                }
            };
            format!(
                "IN YOUR 10-YEAR TERM OF OFFICE, {} PERCENT OF THE\n\
                 POPULATION STARVED PER YEAR ON AVERAGE.\n\
                 A TOTAL OF {} PEOPLE DIED!!\n\
                 YOU STARTED WITH 10 ACRES PER PERSON AND ENDED WITH\n\
                 {} ACRES PER PERSON.\n\n\
                 {}\n",
                avg_starved, state.starved_total, acres_per_person, rating_msg
            )
        }
        GameStatus::Playing => {
            let mut out = format!(
                "HAMURABI: I BEG TO REPORT TO YOU,\n\
                 IN YEAR {}, {} PEOPLE STARVED, {} CAME TO THE CITY.\n",
                state.year, state.starved_last_year, state.immigrants_last_year
            );
            if state.plague_struck {
                out.push_str("A HORRIBLE PLAGUE STRUCK! HALF THE PEOPLE DIED.\n");
            }
            out.push_str(&format!(
                "POPULATION IS NOW {}.\n\
                 THE CITY NOW OWNS {} ACRES.\n\
                 YOU HARVESTED {} BUSHELS PER ACRE.\n\
                 RATS ATE {} BUSHELS.\n\
                 YOU NOW HAVE {} BUSHELS IN STORE.\n\
                 LAND IS TRADING AT {} BUSHELS PER ACRE.\n",
                state.population,
                state.acres,
                state.harvest_per_acre,
                state.rats_eaten,
                state.bushels,
                state.land_price
            ));
            out
        }
    }
}
