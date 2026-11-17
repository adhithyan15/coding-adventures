#include "iso_test.h"
#include "hamurabi.hpp"

#include <cstdio>
#include <fstream>
#include <sstream>
#include <string>
#include <vector>

using namespace ca::hamurabi;

static std::ifstream open_corpus() {
    std::vector<std::string> paths = {
        "../../../specs/fixtures/hamurabi-v1/cases.txt",
        "../../specs/fixtures/hamurabi-v1/cases.txt",
        "code/specs/fixtures/hamurabi-v1/cases.txt"
    };
    for (const auto &path : paths) {
        std::ifstream in(path);
        if (in.good()) {
            return in;
        }
    }
    return std::ifstream();
}

static std::string unescape_report(const std::string &src) {
    std::string dst;
    dst.reserve(src.size());
    for (std::size_t i = 0; i < src.size(); ++i) {
        if (src[i] == '\\' && i + 1 < src.size() && src[i + 1] == 'n') {
            dst.push_back('\n');
            ++i;
        } else if (src[i] != '\r' && src[i] != '\n') {
            dst.push_back(src[i]);
        }
    }
    return dst;
}

static HamurabiState parse_state_fields(const std::string &line) {
    std::istringstream iss(line);
    std::string kw;
    iss >> kw;

    unsigned int year = 0, pop = 0, bushels = 0, acres = 0, price = 0, harvest = 0, rats = 0;
    unsigned int starved_ly = 0, starved_tot = 0, pct_sum = 0, immigrants = 0, plague = 0;
    iss >> year >> pop >> bushels >> acres >> price >> harvest >> rats
        >> starved_ly >> starved_tot >> pct_sum >> immigrants >> plague;

    std::string status_token;
    iss >> status_token;
    GameStatus status;
    if (status_token == "Playing") {
        status.kind = GameStatusKind::Playing;
    } else if (status_token == "GameOverExtinction") {
        status.kind = GameStatusKind::GameOverExtinction;
    } else if (status_token == "Won(Charlemagne)") {
        status.kind = GameStatusKind::Won;
        status.rating = Rating::Charlemagne;
    } else if (status_token == "Won(Acre)") {
        status.kind = GameStatusKind::Won;
        status.rating = Rating::Acre;
    } else if (status_token == "Won(Nero)") {
        status.kind = GameStatusKind::Won;
        status.rating = Rating::Nero;
    } else if (status_token == "Won(Disaster)") {
        status.kind = GameStatusKind::Won;
        status.rating = Rating::Disaster;
    } else if (status_token.rfind("Impeached(", 0) == 0) {
        status.kind = GameStatusKind::Impeached;
        std::string status_token2;
        iss >> status_token2;
        std::string full_status = status_token + " " + status_token2;
        unsigned int st_num = 0, pct_num = 0;
        std::sscanf(full_status.c_str(), "Impeached(starved=%u, percent=%u)", &st_num, &pct_num);
        status.starved_this_turn = st_num;
        status.percent = pct_num;
    }

    std::uint64_t rng_state = 0, rng_inc = 0;
    iss >> rng_state >> rng_inc;

    HamurabiState s;
    s.year = static_cast<std::uint8_t>(year);
    s.population = pop;
    s.bushels = bushels;
    s.acres = acres;
    s.land_price = price;
    s.harvest_per_acre = harvest;
    s.rats_eaten = rats;
    s.starved_last_year = starved_ly;
    s.starved_total = starved_tot;
    s.percent_starved_sum = pct_sum;
    s.immigrants_last_year = immigrants;
    s.plague_struck = (plague != 0);
    s.status = status;
    s.rng_state = rng_state;
    s.rng_inc = rng_inc;
    return s;
}

static HamurabiError parse_error_name(const std::string &name) {
    if (name == "GameAlreadyEnded") return HamurabiError::GameAlreadyEnded;
    if (name == "InvalidOrderBounds") return HamurabiError::InvalidOrderBounds;
    if (name == "InsufficientBushelsForLand") return HamurabiError::InsufficientBushelsForLand;
    if (name == "InsufficientAcresToSell") return HamurabiError::InsufficientAcresToSell;
    if (name == "InsufficientBushelsToFeed") return HamurabiError::InsufficientBushelsToFeed;
    if (name == "InsufficientAcresToPlant") return HamurabiError::InsufficientAcresToPlant;
    if (name == "InsufficientPopulationToPlant") return HamurabiError::InsufficientPopulationToPlant;
    if (name == "InsufficientGrainForSeed") return HamurabiError::InsufficientGrainForSeed;
    return HamurabiError::InvalidOrderBounds;
}

int main() {
    auto in = open_corpus();
    ISO_CHECK_MSG(in.good(), "open hamurabi-v1 cases.txt");
    if (!in.good()) {
        return ISO_TEST_RESULT();
    }

    std::string line;
    HamurabiState current_state;
    int cases_seen = 0;

    while (std::getline(in, line)) {
        if (line.empty() || line[0] == '#' || line[0] == '\r') {
            continue;
        }

        if (line.rfind("COUNT", 0) == 0) {
            continue;
        } else if (line.rfind("CASE", 0) == 0) {
            cases_seen++;
        } else if (line.rfind("INIT", 0) == 0) {
            current_state = parse_state_fields(line);
        } else if (line.rfind("TURN", 0) == 0) {
            std::istringstream iss(line);
            std::string kw;
            int turn_num = 0, acres_delta = 0;
            unsigned int bushels_feed = 0, acres_plant = 0;
            iss >> kw >> turn_num >> acres_delta >> bushels_feed >> acres_plant;

            TurnOrder order{
                .acres_delta = acres_delta,
                .bushels_to_feed = bushels_feed,
                .acres_to_plant = acres_plant
            };

            auto res = step_year(current_state, order);
            ISO_CHECK(res.is_ok());
            HamurabiState next_state = res.value();

            // Next line: STATE
            if (std::getline(in, line)) {
                HamurabiState expected_state = parse_state_fields(line);
                ISO_CHECK(next_state == expected_state);
            }

            // Next line: REPORT
            if (std::getline(in, line)) {
                if (line.rfind("REPORT ", 0) == 0) {
                    std::string expected_report = unescape_report(line.substr(7));
                    std::string actual_report = format_report(next_state);
                    ISO_CHECK_STR_EQ(actual_report.c_str(), expected_report.c_str());
                }
            }

            current_state = next_state;
        } else if (line.rfind("REFUSAL", 0) == 0) {
            std::istringstream iss(line);
            std::string kw;
            int acres_delta = 0;
            unsigned int bushels_feed = 0, acres_plant = 0;
            std::string err_name;
            iss >> kw >> acres_delta >> bushels_feed >> acres_plant >> err_name;

            TurnOrder order{
                .acres_delta = acres_delta,
                .bushels_to_feed = bushels_feed,
                .acres_to_plant = acres_plant
            };

            HamurabiError expected_err = parse_error_name(err_name);

            auto val_res = validate_order(current_state, order);
            ISO_CHECK(val_res.is_err());
            ISO_CHECK(val_res.error() == expected_err);

            auto step_res = step_year(current_state, order);
            ISO_CHECK(step_res.is_err());
            ISO_CHECK(step_res.error() == expected_err);
        }
    }

    ISO_CHECK_EQ_INT(cases_seen, 31);
    return ISO_TEST_RESULT();
}
