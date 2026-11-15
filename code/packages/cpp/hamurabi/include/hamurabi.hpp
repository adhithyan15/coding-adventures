#ifndef CA_HAMURABI_HPP
#define CA_HAMURABI_HPP

#include <bit>
#include <cstdint>
#include <stdexcept>
#include <string>
#include <utility>

namespace ca::hamurabi {

enum class Rating {
    Charlemagne,
    Acre,
    Nero,
    Disaster
};

enum class GameStatusKind {
    Playing,
    Won,
    Impeached,
    GameOverExtinction
};

struct GameStatus {
    GameStatusKind kind = GameStatusKind::Playing;
    Rating rating = Rating::Charlemagne;
    std::uint32_t starved_this_turn = 0;
    std::uint32_t percent = 0;

    constexpr bool operator==(const GameStatus &) const = default;
};

enum class HamurabiError {
    GameAlreadyEnded,
    InvalidOrderBounds,
    InsufficientBushelsForLand,
    InsufficientAcresToSell,
    InsufficientBushelsToFeed,
    InsufficientAcresToPlant,
    InsufficientPopulationToPlant,
    InsufficientGrainForSeed
};

constexpr const char *error_message(HamurabiError err) noexcept {
    switch (err) {
    case HamurabiError::GameAlreadyEnded:
        return "Game has already ended";
    case HamurabiError::InvalidOrderBounds:
        return "Order parameters exceed valid numerical bounds or overflow";
    case HamurabiError::InsufficientBushelsForLand:
        return "Insufficient bushels to buy specified acreage";
    case HamurabiError::InsufficientAcresToSell:
        return "Cannot sell more acres than the city owns";
    case HamurabiError::InsufficientBushelsToFeed:
        return "Insufficient bushels in store to feed population";
    case HamurabiError::InsufficientAcresToPlant:
        return "Cannot plant more acres than owned after land trade";
    case HamurabiError::InsufficientPopulationToPlant:
        return "Insufficient population to tend planted acreage";
    case HamurabiError::InsufficientGrainForSeed:
        return "Insufficient grain to seed planted acreage";
    }
    return "Unknown error";
}

struct HamurabiState {
    std::uint8_t year = 1;
    std::uint32_t population = 100;
    std::uint32_t bushels = 2800;
    std::uint32_t acres = 1000;
    std::uint32_t land_price = 0;
    std::uint32_t harvest_per_acre = 3;
    std::uint32_t rats_eaten = 200;
    std::uint32_t starved_last_year = 0;
    std::uint32_t starved_total = 0;
    std::uint32_t percent_starved_sum = 0;
    std::uint32_t immigrants_last_year = 5;
    bool plague_struck = false;
    GameStatus status{};
    std::uint64_t rng_state = 0;
    std::uint64_t rng_inc = 0;

    constexpr bool operator==(const HamurabiState &) const = default;
};

struct TurnOrder {
    std::int32_t acres_delta = 0;
    std::uint32_t bushels_to_feed = 0;
    std::uint32_t acres_to_plant = 0;

    constexpr bool operator==(const TurnOrder &) const = default;
};

template <typename T>
class Result {
public:
    static Result success(T val) {
        return Result(true, std::move(val), HamurabiError::GameAlreadyEnded);
    }
    static Result failure(HamurabiError err) {
        return Result(false, T{}, err);
    }

    constexpr bool is_ok() const noexcept { return ok_; }
    constexpr bool is_err() const noexcept { return !ok_; }
    constexpr HamurabiError error() const noexcept { return error_; }
    constexpr const T &value() const noexcept { return value_; }
    const T &unwrap() const {
        if (!ok_) throw std::runtime_error(error_message(error_));
        return value_;
    }

private:
    constexpr Result(bool ok, T val, HamurabiError err)
        : ok_(ok), value_(std::move(val)), error_(err) {}

    bool ok_ = false;
    T value_{};
    HamurabiError error_ = HamurabiError::GameAlreadyEnded;
};

template <>
class Result<void> {
public:
    static constexpr Result success() noexcept {
        return Result(true, HamurabiError::GameAlreadyEnded);
    }
    static constexpr Result failure(HamurabiError err) noexcept {
        return Result(false, err);
    }

    constexpr bool is_ok() const noexcept { return ok_; }
    constexpr bool is_err() const noexcept { return !ok_; }
    constexpr HamurabiError error() const noexcept { return error_; }
    void unwrap() const {
        if (!ok_) throw std::runtime_error(error_message(error_));
    }

private:
    constexpr Result(bool ok, HamurabiError err) noexcept
        : ok_(ok), error_(err) {}

    bool ok_ = false;
    HamurabiError error_ = HamurabiError::GameAlreadyEnded;
};

namespace detail {
constexpr std::uint64_t PCG_MULTIPLIER = 6364136223846793005ULL;
constexpr std::uint64_t HAMURABI_STREAM_ID = 54ULL;

inline std::uint32_t pcg32_step(std::uint64_t &state, std::uint64_t inc) noexcept {
    std::uint64_t old_state = state;
    state = old_state * PCG_MULTIPLIER + inc;
    auto xorshifted = static_cast<std::uint32_t>(((old_state >> 18) ^ old_state) >> 27);
    auto rot = static_cast<unsigned int>(old_state >> 59);
    return std::rotr(xorshifted, rot);
}

inline std::uint32_t pcg32_next_int_in_range(std::uint64_t &state, std::uint64_t inc, std::uint32_t min, std::uint32_t max) noexcept {
    const std::uint64_t span = static_cast<std::uint64_t>(max - min) + 1ULL;
    const std::uint64_t threshold = ((1ULL << 32) - span) % span;
    for (;;) {
        const std::uint64_t r = pcg32_step(state, inc);
        if (r >= threshold) {
            return min + static_cast<std::uint32_t>(r % span);
        }
    }
}
} // namespace detail

inline HamurabiState new_game(std::uint64_t seed) noexcept {
    std::uint64_t inc = (detail::HAMURABI_STREAM_ID << 1) | 1ULL;
    std::uint64_t rng_state = 0ULL;
    rng_state = rng_state * detail::PCG_MULTIPLIER + inc;
    rng_state += seed;
    rng_state = rng_state * detail::PCG_MULTIPLIER + inc;

    std::uint32_t land_price = detail::pcg32_next_int_in_range(rng_state, inc, 17, 26);

    HamurabiState s;
    s.year = 1;
    s.population = 100;
    s.bushels = 2800;
    s.acres = 1000;
    s.land_price = land_price;
    s.harvest_per_acre = 3;
    s.rats_eaten = 200;
    s.starved_last_year = 0;
    s.starved_total = 0;
    s.percent_starved_sum = 0;
    s.immigrants_last_year = 5;
    s.plague_struck = false;
    s.status = GameStatus{.kind = GameStatusKind::Playing};
    s.rng_state = rng_state;
    s.rng_inc = inc;
    return s;
}

inline Result<void> validate_order(const HamurabiState &state, const TurnOrder &order) noexcept {
    /* 1. Status Check */
    if (state.status.kind != GameStatusKind::Playing) {
        return Result<void>::failure(HamurabiError::GameAlreadyEnded);
    }

    /* State invariant checks */
    if ((state.rng_inc & 1ULL) != 1ULL) {
        return Result<void>::failure(HamurabiError::InvalidOrderBounds);
    }
    if (state.year < 1 || state.year > 10) {
        return Result<void>::failure(HamurabiError::InvalidOrderBounds);
    }
    if (state.land_price < 17 || state.land_price > 26) {
        return Result<void>::failure(HamurabiError::InvalidOrderBounds);
    }
    if (state.harvest_per_acre < 1 || state.harvest_per_acre > 5) {
        return Result<void>::failure(HamurabiError::InvalidOrderBounds);
    }

    /* 2. Bounds & Overflow Guard on Land Trade */
    if (order.acres_delta > 0) {
        std::uint64_t buy_count = static_cast<std::uint64_t>(order.acres_delta);
        std::uint64_t cost = buy_count * static_cast<std::uint64_t>(state.land_price);
        if (cost > static_cast<std::uint64_t>(state.bushels)) {
            return Result<void>::failure(HamurabiError::InsufficientBushelsForLand);
        }
    } else if (order.acres_delta < 0) {
        std::int64_t d = static_cast<std::int64_t>(order.acres_delta);
        std::uint64_t sell_count = static_cast<std::uint64_t>(-d);
        if (sell_count > static_cast<std::uint64_t>(state.acres)) {
            return Result<void>::failure(HamurabiError::InsufficientAcresToSell);
        }
    }

    /* 3. Land Trade Balances */
    std::uint64_t acres_after_trade_64;
    if (order.acres_delta >= 0) {
        acres_after_trade_64 = static_cast<std::uint64_t>(state.acres) + static_cast<std::uint64_t>(order.acres_delta);
        if (acres_after_trade_64 > 0xFFFFFFFFULL) {
            return Result<void>::failure(HamurabiError::InvalidOrderBounds);
        }
    } else {
        std::int64_t d = static_cast<std::int64_t>(order.acres_delta);
        std::uint64_t sell_count = static_cast<std::uint64_t>(-d);
        acres_after_trade_64 = static_cast<std::uint64_t>(state.acres) - sell_count;
    }
    std::uint32_t acres_after_trade = static_cast<std::uint32_t>(acres_after_trade_64);

    std::uint64_t bushels_after_trade_64;
    if (order.acres_delta > 0) {
        std::uint64_t cost = static_cast<std::uint64_t>(order.acres_delta) * static_cast<std::uint64_t>(state.land_price);
        bushels_after_trade_64 = static_cast<std::uint64_t>(state.bushels) - cost;
    } else if (order.acres_delta < 0) {
        std::int64_t d = static_cast<std::int64_t>(order.acres_delta);
        std::uint64_t sell_count = static_cast<std::uint64_t>(-d);
        std::uint64_t revenue = sell_count * static_cast<std::uint64_t>(state.land_price);
        bushels_after_trade_64 = static_cast<std::uint64_t>(state.bushels) + revenue;
        if (bushels_after_trade_64 > 0xFFFFFFFFULL) {
            return Result<void>::failure(HamurabiError::InvalidOrderBounds);
        }
    } else {
        bushels_after_trade_64 = static_cast<std::uint64_t>(state.bushels);
    }
    std::uint32_t bushels_after_trade = static_cast<std::uint32_t>(bushels_after_trade_64);

    /* 4. Food Allocation Check */
    if (order.bushels_to_feed > bushels_after_trade) {
        return Result<void>::failure(HamurabiError::InsufficientBushelsToFeed);
    }
    std::uint32_t bushels_after_feed = bushels_after_trade - order.bushels_to_feed;

    /* 5. Planting Land & Labor Checks */
    if (order.acres_to_plant > acres_after_trade) {
        return Result<void>::failure(HamurabiError::InsufficientAcresToPlant);
    }
    std::uint64_t max_labor_acres = static_cast<std::uint64_t>(state.population) * 10ULL;
    if (static_cast<std::uint64_t>(order.acres_to_plant) > max_labor_acres) {
        return Result<void>::failure(HamurabiError::InsufficientPopulationToPlant);
    }

    /* 6. Planting Seed Check */
    std::uint64_t seed_needed = (static_cast<std::uint64_t>(order.acres_to_plant) + 1ULL) / 2ULL;
    if (seed_needed > static_cast<std::uint64_t>(bushels_after_feed)) {
        return Result<void>::failure(HamurabiError::InsufficientGrainForSeed);
    }
    std::uint32_t bushels_after_seed = bushels_after_feed - static_cast<std::uint32_t>(seed_needed);

    /* 7. Maximum Harvest Overflow Guard */
    std::uint64_t max_harvest = static_cast<std::uint64_t>(order.acres_to_plant) * 5ULL;
    std::uint64_t max_stored = static_cast<std::uint64_t>(bushels_after_seed) + max_harvest;
    if (max_stored > 0xFFFFFFFFULL) {
        return Result<void>::failure(HamurabiError::InvalidOrderBounds);
    }

    return Result<void>::success();
}

inline Result<HamurabiState> step_year(const HamurabiState &state, const TurnOrder &order) noexcept {
    auto val = validate_order(state, order);
    if (val.is_err()) {
        return Result<HamurabiState>::failure(val.error());
    }

    std::uint64_t pcg_state = state.rng_state;
    std::uint64_t pcg_inc = state.rng_inc;

    std::uint32_t acres_after_trade;
    std::uint32_t bushels_after_trade;
    if (order.acres_delta >= 0) {
        acres_after_trade = state.acres + static_cast<std::uint32_t>(order.acres_delta);
        std::uint64_t cost = static_cast<std::uint64_t>(order.acres_delta) * static_cast<std::uint64_t>(state.land_price);
        bushels_after_trade = static_cast<std::uint32_t>(static_cast<std::uint64_t>(state.bushels) - cost);
    } else {
        std::int64_t d = static_cast<std::int64_t>(order.acres_delta);
        std::uint32_t sell_count = static_cast<std::uint32_t>(-d);
        acres_after_trade = state.acres - sell_count;
        std::uint64_t revenue = static_cast<std::uint64_t>(sell_count) * static_cast<std::uint64_t>(state.land_price);
        bushels_after_trade = static_cast<std::uint32_t>(static_cast<std::uint64_t>(state.bushels) + revenue);
    }

    std::uint32_t bushels_after_feed = bushels_after_trade - order.bushels_to_feed;
    std::uint64_t seed_needed = (static_cast<std::uint64_t>(order.acres_to_plant) + 1ULL) / 2ULL;
    std::uint32_t bushels_after_seed = bushels_after_feed - static_cast<std::uint32_t>(seed_needed);

    /* Step 1: Harvest Yield Draw [1, 5] */
    std::uint32_t harvest_per_acre = detail::pcg32_next_int_in_range(pcg_state, pcg_inc, 1, 5);
    std::uint64_t total_harvest = static_cast<std::uint64_t>(order.acres_to_plant) * static_cast<std::uint64_t>(harvest_per_acre);
    std::uint64_t bushels_stored = static_cast<std::uint64_t>(bushels_after_seed) + total_harvest;

    /* Step 2: Rat Infestation Draws */
    std::uint32_t rat_roll = detail::pcg32_next_int_in_range(pcg_state, pcg_inc, 1, 100);
    std::uint32_t rats_eaten = 0;
    if (rat_roll <= 40) {
        std::uint32_t rat_factor = detail::pcg32_next_int_in_range(pcg_state, pcg_inc, 1, 5);
        std::uint64_t eaten = bushels_stored / static_cast<std::uint64_t>(rat_factor);
        rats_eaten = (eaten > 0xFFFFFFFFULL) ? 0xFFFFFFFFU : static_cast<std::uint32_t>(eaten);
    }
    std::uint64_t bushels_remaining = (bushels_stored > static_cast<std::uint64_t>(rats_eaten)) ? (bushels_stored - static_cast<std::uint64_t>(rats_eaten)) : 0ULL;
    std::uint32_t bushels_final = (bushels_remaining > 0xFFFFFFFFULL) ? 0xFFFFFFFFU : static_cast<std::uint32_t>(bushels_remaining);

    /* Step 3: Plague Draw [1, 100] <= 15 */
    std::uint32_t plague_roll = detail::pcg32_next_int_in_range(pcg_state, pcg_inc, 1, 100);
    bool plague_struck = (plague_roll <= 15);

    /* Step 4: Starvation & Impeachment Evaluation */
    std::uint32_t people_fed = order.bushels_to_feed / 20u;
    std::uint32_t starved = 0;
    std::uint32_t survivors = 0;
    if (people_fed >= state.population) {
        starved = 0;
        survivors = state.population;
    } else {
        starved = state.population - people_fed;
        survivors = people_fed;
    }

    std::uint32_t percent_starved = 0;
    if (state.population > 0) {
        std::uint64_t p = (static_cast<std::uint64_t>(starved) * 100ULL) / static_cast<std::uint64_t>(state.population);
        percent_starved = (p > 100ULL) ? 100U : static_cast<std::uint32_t>(p);
    }

    std::uint64_t new_starved_total_64 = static_cast<std::uint64_t>(state.starved_total) + static_cast<std::uint64_t>(starved);
    std::uint32_t new_starved_total = (new_starved_total_64 > 0xFFFFFFFFULL) ? 0xFFFFFFFFU : static_cast<std::uint32_t>(new_starved_total_64);

    std::uint64_t new_percent_sum_64 = static_cast<std::uint64_t>(state.percent_starved_sum) + static_cast<std::uint64_t>(percent_starved);
    std::uint32_t new_percent_sum = (new_percent_sum_64 > 0xFFFFFFFFULL) ? 0xFFFFFFFFU : static_cast<std::uint32_t>(new_percent_sum_64);

    GameStatus new_status;
    if (static_cast<std::uint64_t>(starved) * 100ULL > static_cast<std::uint64_t>(state.population) * 45ULL) {
        new_status.kind = GameStatusKind::Impeached;
        new_status.starved_this_turn = starved;
        new_status.percent = percent_starved;
    } else if (survivors == 0) {
        new_status.kind = GameStatusKind::GameOverExtinction;
    } else {
        new_status.kind = GameStatusKind::Playing;
    }

    /* Step 5: Plague Resolution */
    if (plague_struck && new_status.kind == GameStatusKind::Playing) {
        survivors /= 2u;
    }

    /* Step 6: Immigration Draw [1, 5] */
    std::uint32_t imm_factor = detail::pcg32_next_int_in_range(pcg_state, pcg_inc, 1, 5);
    std::uint32_t immigrants = 0;
    if (starved == 0 && new_status.kind == GameStatusKind::Playing) {
        std::uint64_t acres_term = static_cast<std::uint64_t>(acres_after_trade) * 20ULL;
        std::uint64_t inner = acres_term + static_cast<std::uint64_t>(bushels_final);
        std::uint64_t numerator = static_cast<std::uint64_t>(imm_factor) * inner;
        std::uint64_t denominator = 100ULL * static_cast<std::uint64_t>(state.population);
        if (denominator != 0) {
            std::uint64_t q = numerator / denominator;
            std::uint64_t imm_val = q + 1ULL;
            immigrants = (imm_val > 0xFFFFFFFFULL) ? 0xFFFFFFFFU : static_cast<std::uint32_t>(imm_val);
        }
    }

    std::uint64_t final_pop_64 = static_cast<std::uint64_t>(survivors) + static_cast<std::uint64_t>(immigrants);
    std::uint32_t final_population = (final_pop_64 > 0xFFFFFFFFULL) ? 0xFFFFFFFFU : static_cast<std::uint32_t>(final_pop_64);

    /* Step 7: Next Year Land Price Draw [17, 26] */
    std::uint32_t next_land_price = detail::pcg32_next_int_in_range(pcg_state, pcg_inc, 17, 26);

    /* Step 8: Turn Advance & Rating */
    auto next_year = static_cast<std::uint8_t>(state.year + 1);
    if (next_year == 11 && new_status.kind == GameStatusKind::Playing) {
        std::uint32_t avg_starved = new_percent_sum / 10u;
        std::uint32_t acres_per_person = (final_population > 0) ? (acres_after_trade / final_population) : 0u;
        Rating rating;
        if (avg_starved <= 3 && acres_per_person >= 10) {
            rating = Rating::Charlemagne;
        } else if (avg_starved <= 10 && acres_per_person >= 9) {
            rating = Rating::Acre;
        } else if (avg_starved <= 33 && acres_per_person >= 7) {
            rating = Rating::Nero;
        } else {
            rating = Rating::Disaster;
        }
        new_status.kind = GameStatusKind::Won;
        new_status.rating = rating;
    }

    HamurabiState next_st;
    next_st.year = next_year;
    next_st.population = final_population;
    next_st.bushels = bushels_final;
    next_st.acres = acres_after_trade;
    next_st.land_price = next_land_price;
    next_st.harvest_per_acre = harvest_per_acre;
    next_st.rats_eaten = rats_eaten;
    next_st.starved_last_year = starved;
    next_st.starved_total = new_starved_total;
    next_st.percent_starved_sum = new_percent_sum;
    next_st.immigrants_last_year = immigrants;
    next_st.plague_struck = plague_struck;
    next_st.status = new_status;
    next_st.rng_state = pcg_state;
    next_st.rng_inc = pcg_inc;

    return Result<HamurabiState>::success(next_st);
}

inline std::string format_report(const HamurabiState &state) {
    switch (state.status.kind) {
    case GameStatusKind::Impeached: {
        return "YOU STARVED " + std::to_string(state.status.starved_this_turn) + " PEOPLE IN ONE YEAR!!!\n"
               "DUE TO THIS EXTREME MISMANAGEMENT YOU HAVE BEEN IMPEACHED AND THROWN OUT OF OFFICE!\n";
    }
    case GameStatusKind::GameOverExtinction: {
        return "EVERYONE HAS DIED. THE CITY OF LAGASH IS EXTINCT!\n";
    }
    case GameStatusKind::Won: {
        std::uint32_t avg_starved = state.percent_starved_sum / 10u;
        std::uint32_t acres_per_person = (state.population > 0) ? (state.acres / state.population) : 0u;
        const char *rating_msg = nullptr;
        switch (state.status.rating) {
        case Rating::Charlemagne:
            rating_msg = "A FANTASTIC PERFORMANCE! CHARLEMAGNE, DISRAELI, AND JEFFERSON COMBINED COULD NOT HAVE DONE BETTER!";
            break;
        case Rating::Acre:
            rating_msg = "YOUR PERFORMANCE COULD HAVE BEEN WORSE, BUT YOU LEFT THEM REASONABLE ACREAGE.";
            break;
        case Rating::Nero:
            rating_msg = "YOUR PERFORMANCE LEAVES MUCH TO BE DESIRED. NERO WOULD HAVE BEEN PROUD OF YOU.";
            break;
        case Rating::Disaster:
        default:
            rating_msg = "YOU DID SO POORLY YOU HAVE BEEN DECLARED AN OFFICIAL NATIONAL DISASTER!";
            break;
        }
        return "IN YOUR 10-YEAR TERM OF OFFICE, " + std::to_string(avg_starved) + " PERCENT OF THE\n"
               "POPULATION STARVED PER YEAR ON AVERAGE.\n"
               "A TOTAL OF " + std::to_string(state.starved_total) + " PEOPLE DIED!!\n"
               "YOU STARTED WITH 10 ACRES PER PERSON AND ENDED WITH\n"
               + std::to_string(acres_per_person) + " ACRES PER PERSON.\n\n"
               + rating_msg + "\n";
    }
    case GameStatusKind::Playing:
    default: {
        std::string out = "HAMURABI: I BEG TO REPORT TO YOU,\n"
                          "IN YEAR " + std::to_string(state.year) + ", "
                          + std::to_string(state.starved_last_year) + " PEOPLE STARVED, "
                          + std::to_string(state.immigrants_last_year) + " CAME TO THE CITY.\n";
        if (state.plague_struck) {
            out += "A HORRIBLE PLAGUE STRUCK! HALF THE PEOPLE DIED.\n";
        }
        out += "POPULATION IS NOW " + std::to_string(state.population) + ".\n"
               "THE CITY NOW OWNS " + std::to_string(state.acres) + " ACRES.\n"
               "YOU HARVESTED " + std::to_string(state.harvest_per_acre) + " BUSHELS PER ACRE.\n"
               "RATS ATE " + std::to_string(state.rats_eaten) + " BUSHELS.\n"
               "YOU NOW HAVE " + std::to_string(state.bushels) + " BUSHELS IN STORE.\n"
               "LAND IS TRADING AT " + std::to_string(state.land_price) + " BUSHELS PER ACRE.\n";
        return out;
    }
    }
}

} // namespace ca::hamurabi

#endif // CA_HAMURABI_HPP
