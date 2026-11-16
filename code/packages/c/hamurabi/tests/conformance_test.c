#ifdef _MSC_VER
#define _CRT_SECURE_NO_WARNINGS
#endif

#include "iso_test.h"
#include "hamurabi.h"

#include <ctype.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static FILE *open_corpus(void) {
    const char *paths[] = {
        "../../../specs/fixtures/hamurabi-v1/cases.txt",
        "../../specs/fixtures/hamurabi-v1/cases.txt",
        "code/specs/fixtures/hamurabi-v1/cases.txt",
        NULL
    };
    for (int i = 0; paths[i] != NULL; i++) {
        FILE *f = fopen(paths[i], "r");
        if (f != NULL) {
            return f;
        }
    }
    return NULL;
}

static void unescape_report(const char *src, char *dst, size_t dst_cap) {
    size_t j = 0;
    for (size_t i = 0; src[i] != '\0' && j + 1 < dst_cap; i++) {
        if (src[i] == '\\' && src[i + 1] == 'n') {
            dst[j++] = '\n';
            i++;
        } else if (src[i] != '\r' && src[i] != '\n') {
            dst[j++] = src[i];
        }
    }
    dst[j] = '\0';
}

static void parse_state_fields(const char *line, hamurabi_state *out) {
    const char *p = line;
    while (*p && !isspace((unsigned char)*p)) p++;

    unsigned int year = 0, pop = 0, bushels = 0, acres = 0, price = 0, harvest = 0, rats = 0;
    unsigned int starved_ly = 0, starved_tot = 0, pct_sum = 0, immigrants = 0, plague = 0;
    int n = 0;
    sscanf(p, "%u %u %u %u %u %u %u %u %u %u %u %u%n",
           &year, &pop, &bushels, &acres, &price, &harvest, &rats,
           &starved_ly, &starved_tot, &pct_sum, &immigrants, &plague, &n);
    p += n;
    while (*p && isspace((unsigned char)*p)) p++;

    hamurabi_game_status status;
    memset(&status, 0, sizeof(status));
    if (strncmp(p, "Playing", 7) == 0) {
        status.kind = HAMURABI_STATUS_PLAYING;
        p += 7;
    } else if (strncmp(p, "GameOverExtinction", 18) == 0) {
        status.kind = HAMURABI_STATUS_GAME_OVER_EXTINCTION;
        p += 18;
    } else if (strncmp(p, "Won(Charlemagne)", 16) == 0) {
        status.kind = HAMURABI_STATUS_WON;
        status.rating = HAMURABI_RATING_CHARLEMAGNE;
        p += 16;
    } else if (strncmp(p, "Won(Acre)", 9) == 0) {
        status.kind = HAMURABI_STATUS_WON;
        status.rating = HAMURABI_RATING_ACRE;
        p += 9;
    } else if (strncmp(p, "Won(Nero)", 9) == 0) {
        status.kind = HAMURABI_STATUS_WON;
        status.rating = HAMURABI_RATING_NERO;
        p += 9;
    } else if (strncmp(p, "Won(Disaster)", 13) == 0) {
        status.kind = HAMURABI_STATUS_WON;
        status.rating = HAMURABI_RATING_DISASTER;
        p += 13;
    } else if (strncmp(p, "Impeached(", 10) == 0) {
        status.kind = HAMURABI_STATUS_IMPEACHED;
        unsigned int st_num = 0, pct_num = 0;
        int imp_n = 0;
        sscanf(p, "Impeached(starved=%u, percent=%u)%n", &st_num, &pct_num, &imp_n);
        status.starved_this_turn = st_num;
        status.percent_starved = pct_num;
        p += imp_n;
    }

    unsigned long long s_val = 0, inc_val = 0;
    sscanf(p, "%llu %llu", &s_val, &inc_val);

    out->year = (uint8_t)year;
    out->population = pop;
    out->bushels = bushels;
    out->acres = acres;
    out->land_price = price;
    out->harvest_per_acre = harvest;
    out->rats_eaten = rats;
    out->starved_last_year = starved_ly;
    out->starved_total = starved_tot;
    out->percent_starved_sum = pct_sum;
    out->immigrants_last_year = immigrants;
    out->plague_struck = (plague != 0);
    out->status = status;
    out->rng_state = (uint64_t)s_val;
    out->rng_inc = (uint64_t)inc_val;
}

static hamurabi_error parse_error_name(const char *name) {
    if (strcmp(name, "GameAlreadyEnded") == 0) return HAMURABI_ERROR_GAME_ALREADY_ENDED;
    if (strcmp(name, "InvalidOrderBounds") == 0) return HAMURABI_ERROR_INVALID_ORDER_BOUNDS;
    if (strcmp(name, "InsufficientBushelsForLand") == 0) return HAMURABI_ERROR_INSUFFICIENT_BUSHELS_FOR_LAND;
    if (strcmp(name, "InsufficientAcresToSell") == 0) return HAMURABI_ERROR_INSUFFICIENT_ACRES_TO_SELL;
    if (strcmp(name, "InsufficientBushelsToFeed") == 0) return HAMURABI_ERROR_INSUFFICIENT_BUSHELS_TO_FEED;
    if (strcmp(name, "InsufficientAcresToPlant") == 0) return HAMURABI_ERROR_INSUFFICIENT_ACRES_TO_PLANT;
    if (strcmp(name, "InsufficientPopulationToPlant") == 0) return HAMURABI_ERROR_INSUFFICIENT_POPULATION_TO_PLANT;
    if (strcmp(name, "InsufficientGrainForSeed") == 0) return HAMURABI_ERROR_INSUFFICIENT_GRAIN_FOR_SEED;
    return HAMURABI_OK;
}

static void check_state_equal(const hamurabi_state *actual, const hamurabi_state *expected) {
    ISO_CHECK_EQ_INT(actual->year, expected->year);
    ISO_CHECK_EQ_UINT(actual->population, expected->population);
    ISO_CHECK_EQ_UINT(actual->bushels, expected->bushels);
    ISO_CHECK_EQ_UINT(actual->acres, expected->acres);
    ISO_CHECK_EQ_UINT(actual->land_price, expected->land_price);
    ISO_CHECK_EQ_UINT(actual->harvest_per_acre, expected->harvest_per_acre);
    ISO_CHECK_EQ_UINT(actual->rats_eaten, expected->rats_eaten);
    ISO_CHECK_EQ_UINT(actual->starved_last_year, expected->starved_last_year);
    ISO_CHECK_EQ_UINT(actual->starved_total, expected->starved_total);
    ISO_CHECK_EQ_UINT(actual->percent_starved_sum, expected->percent_starved_sum);
    ISO_CHECK_EQ_UINT(actual->immigrants_last_year, expected->immigrants_last_year);
    ISO_CHECK_EQ_INT(actual->plague_struck, expected->plague_struck);
    ISO_CHECK_EQ_INT(actual->status.kind, expected->status.kind);
    if (expected->status.kind == HAMURABI_STATUS_WON) {
        ISO_CHECK_EQ_INT(actual->status.rating, expected->status.rating);
    }
    if (expected->status.kind == HAMURABI_STATUS_IMPEACHED) {
        ISO_CHECK_EQ_UINT(actual->status.starved_this_turn, expected->status.starved_this_turn);
        ISO_CHECK_EQ_UINT(actual->status.percent_starved, expected->status.percent_starved);
    }
    ISO_CHECK(actual->rng_state == expected->rng_state);
    ISO_CHECK(actual->rng_inc == expected->rng_inc);
}

int main(void) {
    FILE *f = open_corpus();
    ISO_CHECK_MSG(f != NULL, "open hamurabi-v1 cases.txt");
    if (f == NULL) {
        return ISO_TEST_RESULT();
    }

    char line[4096];
    hamurabi_state current_state;
    memset(&current_state, 0, sizeof(current_state));

    int cases_seen = 0;

    while (fgets(line, sizeof(line), f) != NULL) {
        if (line[0] == '#' || line[0] == '\r' || line[0] == '\n') {
            continue;
        }

        if (strncmp(line, "COUNT", 5) == 0) {
            continue;
        } else if (strncmp(line, "CASE", 4) == 0) {
            char case_id[128];
            char seed_str[64];
            int num_turns = 0, has_refusal = 0;
            sscanf(line, "CASE %127s %63s %d %d", case_id, seed_str, &num_turns, &has_refusal);
            cases_seen++;
        } else if (strncmp(line, "INIT", 4) == 0) {
            parse_state_fields(line, &current_state);
        } else if (strncmp(line, "TURN", 4) == 0) {
            int turn_num = 0;
            int acres_delta = 0;
            unsigned int bushels_feed = 0, acres_plant = 0;
            sscanf(line, "TURN %d %d %u %u", &turn_num, &acres_delta, &bushels_feed, &acres_plant);

            hamurabi_turn_order order;
            order.acres_delta = acres_delta;
            order.bushels_to_feed = bushels_feed;
            order.acres_to_plant = acres_plant;

            /* Advance state */
            hamurabi_state next_state;
            hamurabi_error err = hamurabi_step_year(&current_state, &order, &next_state);
            ISO_CHECK_EQ_INT(err, HAMURABI_OK);

            /* Next line should be STATE */
            if (fgets(line, sizeof(line), f) != NULL) {
                hamurabi_state expected_state;
                parse_state_fields(line, &expected_state);
                check_state_equal(&next_state, &expected_state);
            }

            /* Next line should be REPORT */
            if (fgets(line, sizeof(line), f) != NULL) {
                if (strncmp(line, "REPORT ", 7) == 0) {
                    char expected_report[2048];
                    unescape_report(line + 7, expected_report, sizeof(expected_report));

                    char actual_report[2048];
                    size_t act_len = 0;
                    hamurabi_error rep_err = hamurabi_format_report(&next_state, actual_report, sizeof(actual_report), &act_len);
                    ISO_CHECK_EQ_INT(rep_err, HAMURABI_OK);
                    ISO_CHECK_STR_EQ(actual_report, expected_report);
                }
            }

            current_state = next_state;
        } else if (strncmp(line, "REFUSAL", 7) == 0) {
            int acres_delta = 0;
            unsigned int bushels_feed = 0, acres_plant = 0;
            char err_name[128];
            sscanf(line, "REFUSAL %d %u %u %127s", &acres_delta, &bushels_feed, &acres_plant, err_name);

            hamurabi_turn_order order;
            order.acres_delta = acres_delta;
            order.bushels_to_feed = bushels_feed;
            order.acres_to_plant = acres_plant;

            hamurabi_error expected_err = parse_error_name(err_name);

            hamurabi_error val_err = hamurabi_validate_order(&current_state, &order);
            ISO_CHECK_EQ_INT(val_err, expected_err);

            hamurabi_state dummy;
            hamurabi_error step_err = hamurabi_step_year(&current_state, &order, &dummy);
            ISO_CHECK_EQ_INT(step_err, expected_err);
        }
    }

    fclose(f);
    ISO_CHECK_EQ_INT(cases_seen, 31);
    return ISO_TEST_RESULT();
}
