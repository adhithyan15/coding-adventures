#include "hamurabi.h"

#include <stdio.h>
#include <string.h>

#define PCG_MULTIPLIER 6364136223846793005ULL
#define HAMURABI_STREAM_ID 54ULL

/*
 * PCG32 XSH-RR PRNG step.
 * Guarded against CWE-190 / shift UB: rot is masked to [0, 31],
 * so shift amount is strictly in [0, 31].
 */
static inline uint32_t pcg32_step(uint64_t *state, uint64_t inc) {
    uint64_t old_state = *state;
    *state = old_state * PCG_MULTIPLIER + inc;
    uint32_t xorshifted = (uint32_t)(((old_state >> 18) ^ old_state) >> 27);
    uint32_t rot = (uint32_t)(old_state >> 59);
    uint32_t r = rot & 31u;
    uint32_t l = (0u - rot) & 31u;
    return (r == 0u) ? xorshifted : ((xorshifted >> r) | (xorshifted << l));
}

static uint32_t pcg32_next_int_in_range(uint64_t *state, uint64_t inc, uint32_t min, uint32_t max) {
    uint64_t span = (uint64_t)(max - min) + 1ULL;
    uint64_t threshold = ((1ULL << 32) - span) % span;
    for (;;) {
        uint64_t r = (uint64_t)pcg32_step(state, inc);
        if (r >= threshold) {
            return min + (uint32_t)(r % span);
        }
    }
}

hamurabi_error hamurabi_new_game(uint64_t seed, hamurabi_state *out_state) {
    if (out_state == NULL) {
        return HAMURABI_ERROR_NULL_POINTER;
    }
    uint64_t inc = (HAMURABI_STREAM_ID << 1) | 1ULL;
    uint64_t rng_state = 0ULL;
    rng_state = rng_state * PCG_MULTIPLIER + inc;
    rng_state += seed;
    rng_state = rng_state * PCG_MULTIPLIER + inc;

    uint32_t land_price = pcg32_next_int_in_range(&rng_state, inc, 17, 26);

    out_state->year = 1;
    out_state->population = 100;
    out_state->bushels = 2800;
    out_state->acres = 1000;
    out_state->land_price = land_price;
    out_state->harvest_per_acre = 3;
    out_state->rats_eaten = 200;
    out_state->starved_last_year = 0;
    out_state->starved_total = 0;
    out_state->percent_starved_sum = 0;
    out_state->immigrants_last_year = 5;
    out_state->plague_struck = false;
    out_state->status.kind = HAMURABI_STATUS_PLAYING;
    out_state->status.rating = HAMURABI_RATING_CHARLEMAGNE;
    out_state->status.starved_this_turn = 0;
    out_state->status.percent_starved = 0;
    out_state->rng_state = rng_state;
    out_state->rng_inc = inc;
    return HAMURABI_OK;
}

hamurabi_error hamurabi_validate_order(const hamurabi_state *state, const hamurabi_turn_order *order) {
    if (state == NULL || order == NULL) {
        return HAMURABI_ERROR_NULL_POINTER;
    }

    /* 1. Status Check */
    if (state->status.kind != HAMURABI_STATUS_PLAYING) {
        return HAMURABI_ERROR_GAME_ALREADY_ENDED;
    }

    /* State invariant checks (VER04m state_inv & CWE-400 / CWE-835) */
    if ((state->rng_inc & 1ULL) != 1ULL) {
        return HAMURABI_ERROR_INVALID_ORDER_BOUNDS;
    }
    if (state->year < 1 || state->year > 10) {
        return HAMURABI_ERROR_INVALID_ORDER_BOUNDS;
    }
    if (state->land_price < 17 || state->land_price > 26) {
        return HAMURABI_ERROR_INVALID_ORDER_BOUNDS;
    }
    if (state->harvest_per_acre < 1 || state->harvest_per_acre > 5) {
        return HAMURABI_ERROR_INVALID_ORDER_BOUNDS;
    }

    /* 2. Bounds & Overflow Guard on Land Trade (CWE-190) */
    if (order->acres_delta > 0) {
        uint64_t buy_count = (uint64_t)order->acres_delta;
        uint64_t cost = buy_count * (uint64_t)state->land_price;
        if (cost > (uint64_t)state->bushels) {
            return HAMURABI_ERROR_INSUFFICIENT_BUSHELS_FOR_LAND;
        }
    } else if (order->acres_delta < 0) {
        /* Guard against INT32_MIN negation overflow (CWE-190) */
        int64_t d = (int64_t)order->acres_delta;
        uint64_t sell_count = (uint64_t)(-d);
        if (sell_count > (uint64_t)state->acres) {
            return HAMURABI_ERROR_INSUFFICIENT_ACRES_TO_SELL;
        }
    }

    /* 3. Land Trade Balances */
    uint64_t acres_after_trade_64;
    if (order->acres_delta >= 0) {
        acres_after_trade_64 = (uint64_t)state->acres + (uint64_t)order->acres_delta;
        if (acres_after_trade_64 > 0xFFFFFFFFULL) {
            return HAMURABI_ERROR_INVALID_ORDER_BOUNDS;
        }
    } else {
        int64_t d = (int64_t)order->acres_delta;
        uint64_t sell_count = (uint64_t)(-d);
        acres_after_trade_64 = (uint64_t)state->acres - sell_count;
    }
    uint32_t acres_after_trade = (uint32_t)acres_after_trade_64;

    uint64_t bushels_after_trade_64;
    if (order->acres_delta > 0) {
        uint64_t cost = (uint64_t)order->acres_delta * (uint64_t)state->land_price;
        bushels_after_trade_64 = (uint64_t)state->bushels - cost;
    } else if (order->acres_delta < 0) {
        int64_t d = (int64_t)order->acres_delta;
        uint64_t sell_count = (uint64_t)(-d);
        uint64_t revenue = sell_count * (uint64_t)state->land_price;
        bushels_after_trade_64 = (uint64_t)state->bushels + revenue;
        if (bushels_after_trade_64 > 0xFFFFFFFFULL) {
            return HAMURABI_ERROR_INVALID_ORDER_BOUNDS;
        }
    } else {
        bushels_after_trade_64 = (uint64_t)state->bushels;
    }
    uint32_t bushels_after_trade = (uint32_t)bushels_after_trade_64;

    /* 4. Food Allocation Check */
    if (order->bushels_to_feed > bushels_after_trade) {
        return HAMURABI_ERROR_INSUFFICIENT_BUSHELS_TO_FEED;
    }
    uint32_t bushels_after_feed = bushels_after_trade - order->bushels_to_feed;

    /* 5. Planting Land & Labor Checks */
    if (order->acres_to_plant > acres_after_trade) {
        return HAMURABI_ERROR_INSUFFICIENT_ACRES_TO_PLANT;
    }
    uint64_t max_labor_acres = (uint64_t)state->population * 10ULL;
    if ((uint64_t)order->acres_to_plant > max_labor_acres) {
        return HAMURABI_ERROR_INSUFFICIENT_POPULATION_TO_PLANT;
    }

    /* 6. Planting Seed Check: each bushel seeds 2 acres (ceil(acres / 2)) */
    uint64_t seed_needed = ((uint64_t)order->acres_to_plant + 1ULL) / 2ULL;
    if (seed_needed > (uint64_t)bushels_after_feed) {
        return HAMURABI_ERROR_INSUFFICIENT_GRAIN_FOR_SEED;
    }
    uint32_t bushels_after_seed = bushels_after_feed - (uint32_t)seed_needed;

    /* 7. Maximum Harvest Overflow Guard (CWE-190) */
    uint64_t max_harvest = (uint64_t)order->acres_to_plant * 5ULL;
    uint64_t max_stored = (uint64_t)bushels_after_seed + max_harvest;
    if (max_stored > 0xFFFFFFFFULL) {
        return HAMURABI_ERROR_INVALID_ORDER_BOUNDS;
    }

    return HAMURABI_OK;
}

hamurabi_error hamurabi_step_year(const hamurabi_state *state, const hamurabi_turn_order *order, hamurabi_state *out_state) {
    if (state == NULL || order == NULL || out_state == NULL) {
        return HAMURABI_ERROR_NULL_POINTER;
    }

    hamurabi_error err = hamurabi_validate_order(state, order);
    if (err != HAMURABI_OK) {
        return err;
    }

    uint64_t pcg_state = state->rng_state;
    uint64_t pcg_inc = state->rng_inc;

    /* Intermediate balances from validated trade */
    uint32_t acres_after_trade;
    uint32_t bushels_after_trade;
    if (order->acres_delta >= 0) {
        acres_after_trade = state->acres + (uint32_t)order->acres_delta;
        uint64_t cost = (uint64_t)order->acres_delta * (uint64_t)state->land_price;
        bushels_after_trade = (uint32_t)((uint64_t)state->bushels - cost);
    } else {
        int64_t d = (int64_t)order->acres_delta;
        uint32_t sell_count = (uint32_t)(-d);
        acres_after_trade = state->acres - sell_count;
        uint64_t revenue = (uint64_t)sell_count * (uint64_t)state->land_price;
        bushels_after_trade = (uint32_t)((uint64_t)state->bushels + revenue);
    }

    uint32_t bushels_after_feed = bushels_after_trade - order->bushels_to_feed;
    uint64_t seed_needed = ((uint64_t)order->acres_to_plant + 1ULL) / 2ULL;
    uint32_t bushels_after_seed = bushels_after_feed - (uint32_t)seed_needed;

    /* Step 1: Harvest Yield Draw [1, 5] */
    uint32_t harvest_per_acre = pcg32_next_int_in_range(&pcg_state, pcg_inc, 1, 5);
    uint64_t total_harvest = (uint64_t)order->acres_to_plant * (uint64_t)harvest_per_acre;
    uint64_t bushels_stored = (uint64_t)bushels_after_seed + total_harvest;

    /* Step 2: Rat Infestation Draws */
    uint32_t rat_roll = pcg32_next_int_in_range(&pcg_state, pcg_inc, 1, 100);
    uint32_t rats_eaten = 0;
    if (rat_roll <= 40) {
        uint32_t rat_factor = pcg32_next_int_in_range(&pcg_state, pcg_inc, 1, 5);
        uint64_t eaten = bushels_stored / (uint64_t)rat_factor;
        rats_eaten = (eaten > 0xFFFFFFFFULL) ? 0xFFFFFFFFU : (uint32_t)eaten;
    }
    uint64_t bushels_remaining = (bushels_stored > (uint64_t)rats_eaten) ? (bushels_stored - (uint64_t)rats_eaten) : 0ULL;
    uint32_t bushels_final = (bushels_remaining > 0xFFFFFFFFULL) ? 0xFFFFFFFFU : (uint32_t)bushels_remaining;

    /* Step 3: Plague Draw [1, 100] <= 15 */
    uint32_t plague_roll = pcg32_next_int_in_range(&pcg_state, pcg_inc, 1, 100);
    bool plague_struck = (plague_roll <= 15);

    /* Step 4: Starvation & Impeachment Evaluation */
    uint32_t people_fed = order->bushels_to_feed / 20u;
    uint32_t starved = 0;
    uint32_t survivors = 0;
    if (people_fed >= state->population) {
        starved = 0;
        survivors = state->population;
    } else {
        starved = state->population - people_fed;
        survivors = people_fed;
    }

    uint32_t percent_starved = 0;
    if (state->population > 0) {
        uint64_t p = ((uint64_t)starved * 100ULL) / (uint64_t)state->population;
        percent_starved = (p > 100ULL) ? 100U : (uint32_t)p;
    }

    uint64_t new_starved_total_64 = (uint64_t)state->starved_total + (uint64_t)starved;
    uint32_t new_starved_total = (new_starved_total_64 > 0xFFFFFFFFULL) ? 0xFFFFFFFFU : (uint32_t)new_starved_total_64;

    uint64_t new_percent_sum_64 = (uint64_t)state->percent_starved_sum + (uint64_t)percent_starved;
    uint32_t new_percent_sum = (new_percent_sum_64 > 0xFFFFFFFFULL) ? 0xFFFFFFFFU : (uint32_t)new_percent_sum_64;

    hamurabi_game_status new_status;
    memset(&new_status, 0, sizeof(new_status));
    if ((uint64_t)starved * 100ULL > (uint64_t)state->population * 45ULL) {
        new_status.kind = HAMURABI_STATUS_IMPEACHED;
        new_status.starved_this_turn = starved;
        new_status.percent_starved = percent_starved;
    } else if (survivors == 0) {
        new_status.kind = HAMURABI_STATUS_GAME_OVER_EXTINCTION;
    } else {
        new_status.kind = HAMURABI_STATUS_PLAYING;
    }

    /* Step 5: Plague Resolution */
    if (plague_struck && new_status.kind == HAMURABI_STATUS_PLAYING) {
        survivors /= 2u;
    }

    /* Step 6: Immigration Draw [1, 5] */
    uint32_t imm_factor = pcg32_next_int_in_range(&pcg_state, pcg_inc, 1, 5);
    uint32_t immigrants = 0;
    if (starved == 0 && new_status.kind == HAMURABI_STATUS_PLAYING) {
        uint64_t acres_term = (uint64_t)acres_after_trade * 20ULL;
        uint64_t inner = acres_term + (uint64_t)bushels_final;
        uint64_t numerator = (uint64_t)imm_factor * inner;
        uint64_t denominator = 100ULL * (uint64_t)state->population;
        if (denominator != 0) {
            uint64_t q = numerator / denominator;
            uint64_t imm_val = q + 1ULL;
            immigrants = (imm_val > 0xFFFFFFFFULL) ? 0xFFFFFFFFU : (uint32_t)imm_val;
        }
    }

    uint64_t final_pop_64 = (uint64_t)survivors + (uint64_t)immigrants;
    uint32_t final_population = (final_pop_64 > 0xFFFFFFFFULL) ? 0xFFFFFFFFU : (uint32_t)final_pop_64;

    /* Step 7: Next Year Land Price Draw [17, 26] */
    uint32_t next_land_price = pcg32_next_int_in_range(&pcg_state, pcg_inc, 17, 26);

    /* Step 8: Turn Advance & Rating */
    uint8_t next_year = (uint8_t)(state->year + 1);
    if (next_year == 11 && new_status.kind == HAMURABI_STATUS_PLAYING) {
        uint32_t avg_starved = new_percent_sum / 10u;
        uint32_t acres_per_person = (final_population > 0) ? (acres_after_trade / final_population) : 0u;
        hamurabi_rating rating;
        if (avg_starved <= 3 && acres_per_person >= 10) {
            rating = HAMURABI_RATING_CHARLEMAGNE;
        } else if (avg_starved <= 10 && acres_per_person >= 9) {
            rating = HAMURABI_RATING_ACRE;
        } else if (avg_starved <= 33 && acres_per_person >= 7) {
            rating = HAMURABI_RATING_NERO;
        } else {
            rating = HAMURABI_RATING_DISASTER;
        }
        new_status.kind = HAMURABI_STATUS_WON;
        new_status.rating = rating;
    }

    out_state->year = next_year;
    out_state->population = final_population;
    out_state->bushels = bushels_final;
    out_state->acres = acres_after_trade;
    out_state->land_price = next_land_price;
    out_state->harvest_per_acre = harvest_per_acre;
    out_state->rats_eaten = rats_eaten;
    out_state->starved_last_year = starved;
    out_state->starved_total = new_starved_total;
    out_state->percent_starved_sum = new_percent_sum;
    out_state->immigrants_last_year = immigrants;
    out_state->plague_struck = plague_struck;
    out_state->status = new_status;
    out_state->rng_state = pcg_state;
    out_state->rng_inc = pcg_inc;

    return HAMURABI_OK;
}

hamurabi_error hamurabi_format_report(const hamurabi_state *state, char *buf, size_t buf_cap, size_t *out_len) {
    if (state == NULL || buf == NULL) {
        return HAMURABI_ERROR_NULL_POINTER;
    }

    char temp[1024];
    int n = 0;

    switch (state->status.kind) {
    case HAMURABI_STATUS_IMPEACHED:
        n = snprintf(temp, sizeof(temp),
            "YOU STARVED %u PEOPLE IN ONE YEAR!!!\n"
            "DUE TO THIS EXTREME MISMANAGEMENT YOU HAVE BEEN IMPEACHED AND THROWN OUT OF OFFICE!\n",
            (unsigned int)state->status.starved_this_turn);
        break;

    case HAMURABI_STATUS_GAME_OVER_EXTINCTION:
        n = snprintf(temp, sizeof(temp),
            "EVERYONE HAS DIED. THE CITY OF LAGASH IS EXTINCT!\n");
        break;

    case HAMURABI_STATUS_WON: {
        uint32_t avg_starved = state->percent_starved_sum / 10u;
        uint32_t acres_per_person = (state->population > 0) ? (state->acres / state->population) : 0u;
        const char *rating_msg;
        switch (state->status.rating) {
        case HAMURABI_RATING_CHARLEMAGNE:
            rating_msg = "A FANTASTIC PERFORMANCE! CHARLEMAGNE, DISRAELI, AND JEFFERSON COMBINED COULD NOT HAVE DONE BETTER!";
            break;
        case HAMURABI_RATING_ACRE:
            rating_msg = "YOUR PERFORMANCE COULD HAVE BEEN WORSE, BUT YOU LEFT THEM REASONABLE ACREAGE.";
            break;
        case HAMURABI_RATING_NERO:
            rating_msg = "YOUR PERFORMANCE LEAVES MUCH TO BE DESIRED. NERO WOULD HAVE BEEN PROUD OF YOU.";
            break;
        case HAMURABI_RATING_DISASTER:
        default:
            rating_msg = "YOU DID SO POORLY YOU HAVE BEEN DECLARED AN OFFICIAL NATIONAL DISASTER!";
            break;
        }
        n = snprintf(temp, sizeof(temp),
            "IN YOUR 10-YEAR TERM OF OFFICE, %u PERCENT OF THE\n"
            "POPULATION STARVED PER YEAR ON AVERAGE.\n"
            "A TOTAL OF %u PEOPLE DIED!!\n"
            "YOU STARTED WITH 10 ACRES PER PERSON AND ENDED WITH\n"
            "%u ACRES PER PERSON.\n\n"
            "%s\n",
            (unsigned int)avg_starved,
            (unsigned int)state->starved_total,
            (unsigned int)acres_per_person,
            rating_msg);
        break;
    }

    case HAMURABI_STATUS_PLAYING:
    default: {
        if (state->plague_struck) {
            n = snprintf(temp, sizeof(temp),
                "HAMURABI: I BEG TO REPORT TO YOU,\n"
                "IN YEAR %u, %u PEOPLE STARVED, %u CAME TO THE CITY.\n"
                "A HORRIBLE PLAGUE STRUCK! HALF THE PEOPLE DIED.\n"
                "POPULATION IS NOW %u.\n"
                "THE CITY NOW OWNS %u ACRES.\n"
                "YOU HARVESTED %u BUSHELS PER ACRE.\n"
                "RATS ATE %u BUSHELS.\n"
                "YOU NOW HAVE %u BUSHELS IN STORE.\n"
                "LAND IS TRADING AT %u BUSHELS PER ACRE.\n",
                (unsigned int)state->year,
                (unsigned int)state->starved_last_year,
                (unsigned int)state->immigrants_last_year,
                (unsigned int)state->population,
                (unsigned int)state->acres,
                (unsigned int)state->harvest_per_acre,
                (unsigned int)state->rats_eaten,
                (unsigned int)state->bushels,
                (unsigned int)state->land_price);
        } else {
            n = snprintf(temp, sizeof(temp),
                "HAMURABI: I BEG TO REPORT TO YOU,\n"
                "IN YEAR %u, %u PEOPLE STARVED, %u CAME TO THE CITY.\n"
                "POPULATION IS NOW %u.\n"
                "THE CITY NOW OWNS %u ACRES.\n"
                "YOU HARVESTED %u BUSHELS PER ACRE.\n"
                "RATS ATE %u BUSHELS.\n"
                "YOU NOW HAVE %u BUSHELS IN STORE.\n"
                "LAND IS TRADING AT %u BUSHELS PER ACRE.\n",
                (unsigned int)state->year,
                (unsigned int)state->starved_last_year,
                (unsigned int)state->immigrants_last_year,
                (unsigned int)state->population,
                (unsigned int)state->acres,
                (unsigned int)state->harvest_per_acre,
                (unsigned int)state->rats_eaten,
                (unsigned int)state->bushels,
                (unsigned int)state->land_price);
        }
        break;
    }
    }

    if (n < 0 || (size_t)n >= sizeof(temp)) {
        return HAMURABI_ERROR_INVALID_ORDER_BOUNDS;
    }

    size_t needed = (size_t)n + 1u;
    if (buf_cap < needed) {
        return HAMURABI_ERROR_BUFFER_TOO_SMALL;
    }

    memcpy(buf, temp, needed);
    if (out_len != NULL) {
        *out_len = (size_t)n;
    }
    return HAMURABI_OK;
}

const char *hamurabi_error_string(hamurabi_error err) {
    switch (err) {
    case HAMURABI_OK:
        return "Success";
    case HAMURABI_ERROR_NULL_POINTER:
        return "NULL pointer argument";
    case HAMURABI_ERROR_BUFFER_TOO_SMALL:
        return "Output buffer capacity too small";
    case HAMURABI_ERROR_GAME_ALREADY_ENDED:
        return "Game has already ended";
    case HAMURABI_ERROR_INVALID_ORDER_BOUNDS:
        return "Order parameters exceed valid numerical bounds or overflow";
    case HAMURABI_ERROR_INSUFFICIENT_BUSHELS_FOR_LAND:
        return "Insufficient bushels to buy specified acreage";
    case HAMURABI_ERROR_INSUFFICIENT_ACRES_TO_SELL:
        return "Cannot sell more acres than the city owns";
    case HAMURABI_ERROR_INSUFFICIENT_BUSHELS_TO_FEED:
        return "Insufficient bushels in store to feed population";
    case HAMURABI_ERROR_INSUFFICIENT_ACRES_TO_PLANT:
        return "Cannot plant more acres than owned after land trade";
    case HAMURABI_ERROR_INSUFFICIENT_POPULATION_TO_PLANT:
        return "Insufficient population to tend planted acreage";
    case HAMURABI_ERROR_INSUFFICIENT_GRAIN_FOR_SEED:
        return "Insufficient grain to seed planted acreage";
    }
    return "Unknown error";
}
