#ifndef HAMURABI_H
#define HAMURABI_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/*
 * Rating classification awarded to ruler upon completing a 10-year term.
 */
typedef enum {
    HAMURABI_RATING_CHARLEMAGNE = 0,
    HAMURABI_RATING_ACRE = 1,
    HAMURABI_RATING_NERO = 2,
    HAMURABI_RATING_DISASTER = 3
} hamurabi_rating;

/*
 * Simulation lifecycle status kind.
 */
typedef enum {
    HAMURABI_STATUS_PLAYING = 0,
    HAMURABI_STATUS_WON = 1,
    HAMURABI_STATUS_IMPEACHED = 2,
    HAMURABI_STATUS_GAME_OVER_EXTINCTION = 3
} hamurabi_status_kind;

/*
 * Simulation status value record.
 */
typedef struct {
    hamurabi_status_kind kind;
    hamurabi_rating rating;      /* Valid when kind == HAMURABI_STATUS_WON */
    uint32_t starved_this_turn;  /* Valid when kind == HAMURABI_STATUS_IMPEACHED */
    uint32_t percent_starved;    /* Valid when kind == HAMURABI_STATUS_IMPEACHED */
} hamurabi_game_status;

/*
 * Error enumeration for input validation and simulation errors.
 */
typedef enum {
    HAMURABI_OK = 0,
    HAMURABI_ERROR_NULL_POINTER = -1,
    HAMURABI_ERROR_BUFFER_TOO_SMALL = -2,
    HAMURABI_ERROR_GAME_ALREADY_ENDED = 1,
    HAMURABI_ERROR_INVALID_ORDER_BOUNDS = 2,
    HAMURABI_ERROR_INSUFFICIENT_BUSHELS_FOR_LAND = 3,
    HAMURABI_ERROR_INSUFFICIENT_ACRES_TO_SELL = 4,
    HAMURABI_ERROR_INSUFFICIENT_BUSHELS_TO_FEED = 5,
    HAMURABI_ERROR_INSUFFICIENT_ACRES_TO_PLANT = 6,
    HAMURABI_ERROR_INSUFFICIENT_POPULATION_TO_PLANT = 7,
    HAMURABI_ERROR_INSUFFICIENT_GRAIN_FOR_SEED = 8
} hamurabi_error;

/*
 * Complete immutable simulation state.
 */
typedef struct {
    uint8_t year;
    uint32_t population;
    uint32_t bushels;
    uint32_t acres;
    uint32_t land_price;
    uint32_t harvest_per_acre;
    uint32_t rats_eaten;
    uint32_t starved_last_year;
    uint32_t starved_total;
    uint32_t percent_starved_sum;
    uint32_t immigrants_last_year;
    bool plague_struck;
    hamurabi_game_status status;
    uint64_t rng_state;
    uint64_t rng_inc;
} hamurabi_state;

/*
 * Player orders for a single year.
 */
typedef struct {
    int32_t acres_delta;
    uint32_t bushels_to_feed;
    uint32_t acres_to_plant;
} hamurabi_turn_order;

/*
 * Initializes a new deterministic 10-year reign for Hamurabi.
 * Enforces CWE-476: returns HAMURABI_ERROR_NULL_POINTER if out_state is NULL.
 */
hamurabi_error hamurabi_new_game(uint64_t seed, hamurabi_state *out_state);

/*
 * Validates player orders without advancing simulation state.
 * Enforces CWE-476: returns HAMURABI_ERROR_NULL_POINTER if state or order is NULL.
 */
hamurabi_error hamurabi_validate_order(const hamurabi_state *state, const hamurabi_turn_order *order);

/*
 * Advances the simulation by exactly one year according to player orders.
 * Enforces CWE-476: returns HAMURABI_ERROR_NULL_POINTER if state, order, or out_state is NULL.
 */
hamurabi_error hamurabi_step_year(const hamurabi_state *state, const hamurabi_turn_order *order, hamurabi_state *out_state);

/*
 * Formats teletype status report into caller-supplied buffer (VER07 zero heap alloc).
 * Enforces CWE-476 (NULL checks) and CWE-125/CWE-787 (buffer bounds check).
 * Returns HAMURABI_ERROR_BUFFER_TOO_SMALL if buf_cap is insufficient.
 */
hamurabi_error hamurabi_format_report(const hamurabi_state *state, char *buf, size_t buf_cap, size_t *out_len);

/*
 * Returns a static constant human-readable description for an error code.
 */
const char *hamurabi_error_string(hamurabi_error err);

#ifdef __cplusplus
}
#endif

#endif /* HAMURABI_H */
