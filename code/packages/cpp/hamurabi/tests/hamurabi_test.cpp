#include "iso_test.h"
#include "hamurabi.hpp"

#include <cstdint>
#include <limits>
#include <string>

using namespace ca::hamurabi;

static void test_new_game_baseline() {
    auto state = new_game(42);
    ISO_CHECK_EQ_INT(state.year, 1);
    ISO_CHECK_EQ_UINT(state.population, 100);
    ISO_CHECK_EQ_UINT(state.bushels, 2800);
    ISO_CHECK_EQ_UINT(state.acres, 1000);
    ISO_CHECK(state.land_price >= 17 && state.land_price <= 26);
    ISO_CHECK_EQ_UINT(state.harvest_per_acre, 3);
    ISO_CHECK_EQ_UINT(state.rats_eaten, 200);
    ISO_CHECK_EQ_UINT(state.starved_last_year, 0);
    ISO_CHECK_EQ_UINT(state.starved_total, 0);
    ISO_CHECK_EQ_UINT(state.percent_starved_sum, 0);
    ISO_CHECK_EQ_UINT(state.immigrants_last_year, 5);
    ISO_CHECK_EQ_INT(state.plague_struck, 0);
    ISO_CHECK(state.status.kind == GameStatusKind::Playing);
    ISO_CHECK_EQ_UINT(state.rng_inc, 109);

    auto state2 = new_game(42);
    ISO_CHECK(state == state2);
}

static void test_cwe_190_int32_min_negation() {
    auto state = new_game(1);
    TurnOrder order{.acres_delta = std::numeric_limits<std::int32_t>::min()};

    auto val = validate_order(state, order);
    ISO_CHECK(val.is_err());
    ISO_CHECK(val.error() == HamurabiError::InsufficientAcresToSell);

    auto res = step_year(state, order);
    ISO_CHECK(res.is_err());
    ISO_CHECK(res.error() == HamurabiError::InsufficientAcresToSell);
}

static void test_validation_error_variants() {
    auto state = new_game(1);

    // GameAlreadyEnded
    auto ended_state = state;
    ended_state.status.kind = GameStatusKind::GameOverExtinction;
    TurnOrder valid_order{.acres_delta = 0, .bushels_to_feed = 2000, .acres_to_plant = 1000};
    auto v1 = validate_order(ended_state, valid_order);
    ISO_CHECK(v1.is_err() && v1.error() == HamurabiError::GameAlreadyEnded);

    // InvalidOrderBounds
    auto inv_state = state;
    inv_state.rng_inc = 108; // even inc
    auto v2 = validate_order(inv_state, valid_order);
    ISO_CHECK(v2.is_err() && v2.error() == HamurabiError::InvalidOrderBounds);

    inv_state = state;
    inv_state.year = 11;
    auto v3 = validate_order(inv_state, valid_order);
    ISO_CHECK(v3.is_err() && v3.error() == HamurabiError::InvalidOrderBounds);

    inv_state = state;
    inv_state.land_price = 15;
    auto v4 = validate_order(inv_state, valid_order);
    ISO_CHECK(v4.is_err() && v4.error() == HamurabiError::InvalidOrderBounds);

    // InsufficientBushelsForLand
    TurnOrder buy_too_much{.acres_delta = 1000};
    auto v5 = validate_order(state, buy_too_much);
    ISO_CHECK(v5.is_err() && v5.error() == HamurabiError::InsufficientBushelsForLand);

    // InsufficientAcresToSell
    TurnOrder sell_too_much{.acres_delta = -1001};
    auto v6 = validate_order(state, sell_too_much);
    ISO_CHECK(v6.is_err() && v6.error() == HamurabiError::InsufficientAcresToSell);

    // InsufficientBushelsToFeed
    TurnOrder feed_too_much{.bushels_to_feed = 2801};
    auto v7 = validate_order(state, feed_too_much);
    ISO_CHECK(v7.is_err() && v7.error() == HamurabiError::InsufficientBushelsToFeed);

    // InsufficientAcresToPlant
    TurnOrder plant_too_much{.acres_to_plant = 1001};
    auto v8 = validate_order(state, plant_too_much);
    ISO_CHECK(v8.is_err() && v8.error() == HamurabiError::InsufficientAcresToPlant);

    // InsufficientPopulationToPlant
    auto low_pop = state;
    low_pop.population = 10;
    TurnOrder plant_over_pop{.bushels_to_feed = 200, .acres_to_plant = 150};
    auto v9 = validate_order(low_pop, plant_over_pop);
    ISO_CHECK(v9.is_err() && v9.error() == HamurabiError::InsufficientPopulationToPlant);

    // InsufficientGrainForSeed
    TurnOrder seed_short{.bushels_to_feed = 2700, .acres_to_plant = 300};
    auto v10 = validate_order(state, seed_short);
    ISO_CHECK(v10.is_err() && v10.error() == HamurabiError::InsufficientGrainForSeed);
}

static void test_format_report() {
    auto state = new_game(1);
    std::string rep = format_report(state);
    ISO_CHECK(rep.find("HAMURABI: I BEG TO REPORT TO YOU,") != std::string::npos);
    ISO_CHECK(rep.find("IN YEAR 1") != std::string::npos);
}

static void test_error_strings() {
    ISO_CHECK(std::string(error_message(HamurabiError::GameAlreadyEnded)).length() > 0);
    ISO_CHECK(std::string(error_message(HamurabiError::InvalidOrderBounds)).length() > 0);
    ISO_CHECK(std::string(error_message(HamurabiError::InsufficientBushelsForLand)).length() > 0);
    ISO_CHECK(std::string(error_message(HamurabiError::InsufficientAcresToSell)).length() > 0);
    ISO_CHECK(std::string(error_message(HamurabiError::InsufficientBushelsToFeed)).length() > 0);
    ISO_CHECK(std::string(error_message(HamurabiError::InsufficientAcresToPlant)).length() > 0);
    ISO_CHECK(std::string(error_message(HamurabiError::InsufficientPopulationToPlant)).length() > 0);
    ISO_CHECK(std::string(error_message(HamurabiError::InsufficientGrainForSeed)).length() > 0);
}

int main() {
    test_new_game_baseline();
    test_cwe_190_int32_min_negation();
    test_validation_error_variants();
    test_format_report();
    test_error_strings();
    return ISO_TEST_RESULT();
}
