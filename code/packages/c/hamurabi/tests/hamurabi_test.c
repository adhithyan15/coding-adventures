#include "iso_test.h"
#include "hamurabi.h"

#include <limits.h>
#include <string.h>

static void test_new_game_baseline(void) {
    hamurabi_state state;
    hamurabi_error err = hamurabi_new_game(42, &state);
    ISO_CHECK_EQ_INT(err, HAMURABI_OK);
    ISO_CHECK_EQ_INT(state.year, 1);
    ISO_CHECK_EQ_INT(state.population, 100);
    ISO_CHECK_EQ_INT(state.bushels, 2800);
    ISO_CHECK_EQ_INT(state.acres, 1000);
    ISO_CHECK(state.land_price >= 17 && state.land_price <= 26);
    ISO_CHECK_EQ_INT(state.harvest_per_acre, 3);
    ISO_CHECK_EQ_INT(state.rats_eaten, 200);
    ISO_CHECK_EQ_INT(state.starved_last_year, 0);
    ISO_CHECK_EQ_INT(state.starved_total, 0);
    ISO_CHECK_EQ_INT(state.percent_starved_sum, 0);
    ISO_CHECK_EQ_INT(state.immigrants_last_year, 5);
    ISO_CHECK_EQ_INT(state.plague_struck, 0);
    ISO_CHECK_EQ_INT(state.status.kind, HAMURABI_STATUS_PLAYING);
    ISO_CHECK_EQ_INT(state.rng_inc, 109);

    /* Deterministic for same seed */
    hamurabi_state state2;
    err = hamurabi_new_game(42, &state2);
    ISO_CHECK_EQ_INT(err, HAMURABI_OK);
    ISO_CHECK_EQ_INT(state.land_price, state2.land_price);
    ISO_CHECK_EQ_INT(state.rng_state, state2.rng_state);
}

static void test_cwe_476_null_pointer_guards(void) {
    hamurabi_state state;
    hamurabi_new_game(1, &state);
    hamurabi_turn_order order = {0, 2000, 1000};
    hamurabi_state out;
    char buf[256];
    size_t out_len = 0;

    ISO_CHECK_EQ_INT(hamurabi_new_game(1, NULL), HAMURABI_ERROR_NULL_POINTER);
    ISO_CHECK_EQ_INT(hamurabi_validate_order(NULL, &order), HAMURABI_ERROR_NULL_POINTER);
    ISO_CHECK_EQ_INT(hamurabi_validate_order(&state, NULL), HAMURABI_ERROR_NULL_POINTER);
    ISO_CHECK_EQ_INT(hamurabi_step_year(NULL, &order, &out), HAMURABI_ERROR_NULL_POINTER);
    ISO_CHECK_EQ_INT(hamurabi_step_year(&state, NULL, &out), HAMURABI_ERROR_NULL_POINTER);
    ISO_CHECK_EQ_INT(hamurabi_step_year(&state, &order, NULL), HAMURABI_ERROR_NULL_POINTER);
    ISO_CHECK_EQ_INT(hamurabi_format_report(NULL, buf, sizeof(buf), &out_len), HAMURABI_ERROR_NULL_POINTER);
    ISO_CHECK_EQ_INT(hamurabi_format_report(&state, NULL, sizeof(buf), &out_len), HAMURABI_ERROR_NULL_POINTER);
}

static void test_cwe_190_int32_min_negation(void) {
    hamurabi_state state;
    hamurabi_new_game(1, &state);
    hamurabi_turn_order order = {INT32_MIN, 0, 0};

    /* Selling -INT32_MIN (> 2 billion acres) when city has only 1000 must cleanly fail */
    hamurabi_error err = hamurabi_validate_order(&state, &order);
    ISO_CHECK_EQ_INT(err, HAMURABI_ERROR_INSUFFICIENT_ACRES_TO_SELL);

    hamurabi_state out;
    err = hamurabi_step_year(&state, &order, &out);
    ISO_CHECK_EQ_INT(err, HAMURABI_ERROR_INSUFFICIENT_ACRES_TO_SELL);
}

static void test_validation_error_variants(void) {
    hamurabi_state state;
    hamurabi_new_game(1, &state);

    /* GameAlreadyEnded */
    hamurabi_state ended_state = state;
    ended_state.status.kind = HAMURABI_STATUS_GAME_OVER_EXTINCTION;
    hamurabi_turn_order valid_order = {0, 2000, 1000};
    ISO_CHECK_EQ_INT(hamurabi_validate_order(&ended_state, &valid_order), HAMURABI_ERROR_GAME_ALREADY_ENDED);

    /* InvalidOrderBounds: state invariant violations */
    hamurabi_state invalid_state = state;
    invalid_state.rng_inc = 108; /* even inc */
    ISO_CHECK_EQ_INT(hamurabi_validate_order(&invalid_state, &valid_order), HAMURABI_ERROR_INVALID_ORDER_BOUNDS);

    invalid_state = state;
    invalid_state.year = 12;
    ISO_CHECK_EQ_INT(hamurabi_validate_order(&invalid_state, &valid_order), HAMURABI_ERROR_INVALID_ORDER_BOUNDS);

    invalid_state = state;
    invalid_state.land_price = 10;
    ISO_CHECK_EQ_INT(hamurabi_validate_order(&invalid_state, &valid_order), HAMURABI_ERROR_INVALID_ORDER_BOUNDS);

    /* InsufficientBushelsForLand */
    hamurabi_turn_order buy_too_much = {1000, 0, 0};
    ISO_CHECK_EQ_INT(hamurabi_validate_order(&state, &buy_too_much), HAMURABI_ERROR_INSUFFICIENT_BUSHELS_FOR_LAND);

    /* InsufficientAcresToSell */
    hamurabi_turn_order sell_too_much = {-1001, 0, 0};
    ISO_CHECK_EQ_INT(hamurabi_validate_order(&state, &sell_too_much), HAMURABI_ERROR_INSUFFICIENT_ACRES_TO_SELL);

    /* InsufficientBushelsToFeed */
    hamurabi_turn_order feed_too_much = {0, 2801, 0};
    ISO_CHECK_EQ_INT(hamurabi_validate_order(&state, &feed_too_much), HAMURABI_ERROR_INSUFFICIENT_BUSHELS_TO_FEED);

    /* InsufficientAcresToPlant */
    hamurabi_turn_order plant_too_much = {0, 2000, 1001};
    ISO_CHECK_EQ_INT(hamurabi_validate_order(&state, &plant_too_much), HAMURABI_ERROR_INSUFFICIENT_ACRES_TO_PLANT);

    /* InsufficientPopulationToPlant */
    hamurabi_state low_pop = state;
    low_pop.population = 10; /* max labor = 100 */
    hamurabi_turn_order plant_over_pop = {0, 200, 150};
    ISO_CHECK_EQ_INT(hamurabi_validate_order(&low_pop, &plant_over_pop), HAMURABI_ERROR_INSUFFICIENT_POPULATION_TO_PLANT);

    /* InsufficientGrainForSeed */
    hamurabi_turn_order seed_short = {0, 2700, 300}; /* remaining 100 bushels, need ceil(300/2) = 150 */
    ISO_CHECK_EQ_INT(hamurabi_validate_order(&state, &seed_short), HAMURABI_ERROR_INSUFFICIENT_GRAIN_FOR_SEED);
}

static void test_format_report_and_ver07(void) {
    hamurabi_state state;
    hamurabi_new_game(1, &state);

    char tiny_buf[10];
    size_t out_len = 0;
    hamurabi_error err = hamurabi_format_report(&state, tiny_buf, sizeof(tiny_buf), &out_len);
    ISO_CHECK_EQ_INT(err, HAMURABI_ERROR_BUFFER_TOO_SMALL);

    char full_buf[1024];
    err = hamurabi_format_report(&state, full_buf, sizeof(full_buf), &out_len);
    ISO_CHECK_EQ_INT(err, HAMURABI_OK);
    ISO_CHECK(out_len > 50);
    ISO_CHECK_EQ_UINT(out_len, strlen(full_buf));
    ISO_CHECK(strstr(full_buf, "HAMURABI: I BEG TO REPORT TO YOU,") != NULL);
}

static void test_error_strings(void) {
    ISO_CHECK(hamurabi_error_string(HAMURABI_OK) != NULL);
    ISO_CHECK(hamurabi_error_string(HAMURABI_ERROR_NULL_POINTER) != NULL);
    ISO_CHECK(hamurabi_error_string(HAMURABI_ERROR_BUFFER_TOO_SMALL) != NULL);
    ISO_CHECK(hamurabi_error_string(HAMURABI_ERROR_GAME_ALREADY_ENDED) != NULL);
    ISO_CHECK(hamurabi_error_string(HAMURABI_ERROR_INVALID_ORDER_BOUNDS) != NULL);
    ISO_CHECK(hamurabi_error_string(HAMURABI_ERROR_INSUFFICIENT_BUSHELS_FOR_LAND) != NULL);
    ISO_CHECK(hamurabi_error_string(HAMURABI_ERROR_INSUFFICIENT_ACRES_TO_SELL) != NULL);
    ISO_CHECK(hamurabi_error_string(HAMURABI_ERROR_INSUFFICIENT_BUSHELS_TO_FEED) != NULL);
    ISO_CHECK(hamurabi_error_string(HAMURABI_ERROR_INSUFFICIENT_ACRES_TO_PLANT) != NULL);
    ISO_CHECK(hamurabi_error_string(HAMURABI_ERROR_INSUFFICIENT_POPULATION_TO_PLANT) != NULL);
    ISO_CHECK(hamurabi_error_string(HAMURABI_ERROR_INSUFFICIENT_GRAIN_FOR_SEED) != NULL);
}

int main(void) {
    test_new_game_baseline();
    test_cwe_476_null_pointer_guards();
    test_cwe_190_int32_min_negation();
    test_validation_error_variants();
    test_format_report_and_ver07();
    test_error_strings();
    return ISO_TEST_RESULT();
}
