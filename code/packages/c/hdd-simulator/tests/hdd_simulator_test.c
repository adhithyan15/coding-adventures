#define _CRT_SECURE_NO_WARNINGS
#define HDD_TESTING
#include "../src/hdd_simulator.c"

#include "bounded_json.h"
#include "iso_test.h"
#include "hdd_simulator_fuzz_support.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define FIXTURE_PATH "../../../specs/fixtures/database-media-v1/hdd-simulator.json"
#define MAX_FIXTURE_BYTES ((size_t)65536)
#define HDD_FUZZ_MAX_OPERATIONS ((size_t)64)
#define HDD_FUZZ_MAX_PROBES ((size_t)8)
#define HDD_FUZZ_MAX_COMMANDS ((size_t)64)
#define HDD_FUZZ_BUFFER_BYTES ((size_t)256)

typedef struct {
    uint64_t tick;
    size_t head;
    uint8_t media[HDD_SECTOR_COUNT][HDD_SECTOR_BYTES];
} HddModel;

static int same_model_state(const HddModel *left, const HddModel *right) {
    return left->tick == right->tick && left->head == right->head &&
        memcmp(left->media, right->media, sizeof left->media) == 0;
}

static int model_timing(HddModel *model, size_t lba, HddTiming *timing) {
    uint64_t seek = 0, rotation = 0;
    uint64_t tick = model->tick;
    size_t head = model->head;
    const size_t target = lba / 8;
    const size_t distance = target > head ? target - head : head - target;
    while (seek < (uint64_t)(distance * 3)) {
        if (tick == HDD_MAX_TICK) return 0;
        tick++;
        seek++;
    }
    while ((tick % UINT64_C(4)) != (uint64_t)(lba % 4)) {
        if (tick == HDD_MAX_TICK) return 0;
        tick++;
        rotation++;
    }
    if (tick == HDD_MAX_TICK) return 0;
    timing->start_tick = tick - seek - rotation;
    timing->seek_ticks = seek;
    timing->rotation_ticks = rotation;
    timing->transfer_ticks = 1;
    timing->total_ticks = seek + rotation + 1;
    tick++;
    timing->end_tick = tick;
    model->tick = tick;
    model->head = target;
    return 1;
}

static int same_snapshot(const HddSnapshot *a, const HddSnapshot *b) {
    return a->tick == b->tick && a->head_track == b->head_track &&
        memcmp(a->sectors, b->sectors, sizeof a->sectors) == 0;
}

static int same_timing(const HddTiming *a, const HddTiming *b) {
    return a->start_tick == b->start_tick && a->seek_ticks == b->seek_ticks &&
        a->rotation_ticks == b->rotation_ticks &&
        a->transfer_ticks == b->transfer_ticks && a->total_ticks == b->total_ticks &&
        a->end_tick == b->end_tick;
}

static const BJsonValue *member(const BJsonValue *object, const char *key) {
    size_t i;
    const size_t wanted = strlen(key);
    const BJsonValue *found = NULL;
    for (i = 0; i < bj_size(object); i++) {
        const char *name = NULL;
        size_t length = 0;
        const BJsonValue *value = NULL;
        if (!bj_member(object, i, &name, &length, &value)) return NULL;
        if (length == wanted && memcmp(name, key, wanted) == 0) {
            if (found != NULL) return NULL; /* duplicate required key */
            found = value;
        }
    }
    return found;
}

static int object_keys_are(const BJsonValue *object, const char *const *keys, size_t count) {
    size_t i;
    if (bj_kind(object) != BJ_OBJECT || bj_size(object) != count) return 0;
    for (i = 0; i < count; i++) if (member(object, keys[i]) == NULL) return 0;
    return 1;
}

static int integer(const BJsonValue *value, int64_t *out) {
    return bj_kind(value) == BJ_INTEGER && bj_integer(value, out);
}

static int string_is(const BJsonValue *value, const char *expected) {
    const char *text = NULL;
    size_t length = 0;
    const size_t wanted = strlen(expected);
    return bj_kind(value) == BJ_STRING && bj_string(value, &text, &length) &&
        length == wanted && memcmp(text, expected, wanted) == 0;
}

static int decode_hex(const BJsonValue *value, uint8_t *out, size_t capacity, size_t *decoded_length) {
    const char *text = NULL;
    size_t length = 0, i;
    if (!bj_string(value, &text, &length) || (length % 2) != 0 || length / 2 > capacity) return 0;
    for (i = 0; i < length / 2; i++) {
        unsigned high, low;
        char a = text[i * 2], b = text[i * 2 + 1];
        high = a >= '0' && a <= '9' ? (unsigned)(a - '0') :
            a >= 'a' && a <= 'f' ? (unsigned)(a - 'a' + 10) :
            a >= 'A' && a <= 'F' ? (unsigned)(a - 'A' + 10) : 16U;
        low = b >= '0' && b <= '9' ? (unsigned)(b - '0') :
            b >= 'a' && b <= 'f' ? (unsigned)(b - 'a' + 10) :
            b >= 'A' && b <= 'F' ? (unsigned)(b - 'A' + 10) : 16U;
        if (high > 15 || low > 15) return 0;
        out[i] = (uint8_t)(high * 16U + low);
    }
    *decoded_length = length / 2;
    return 1;
}

static int hex_bytes(const BJsonValue *value, uint8_t *out, size_t expected_length) {
    size_t decoded = 0;
    return decode_hex(value, out, expected_length, &decoded) && decoded == expected_length;
}

static int parse_timing(const BJsonValue *value, HddTiming *timing) {
    static const char *const keys[] = {"start_tick", "seek_ticks", "rotation_ticks",
        "transfer_ticks", "total_ticks", "end_tick"};
    const char *names[] = {"start_tick", "seek_ticks", "rotation_ticks",
        "transfer_ticks", "total_ticks", "end_tick"};
    uint64_t *fields[] = {&timing->start_tick, &timing->seek_ticks,
        &timing->rotation_ticks, &timing->transfer_ticks,
        &timing->total_ticks, &timing->end_tick};
    size_t i;
    if (!object_keys_are(value, keys, 6)) return 0;
    for (i = 0; i < 6; i++) {
        int64_t n;
        if (!integer(member(value, names[i]), &n) || n < 0) return 0;
        *fields[i] = (uint64_t)n;
    }
    return 1;
}

static int fixture_error_status(const BJsonValue *value, HddStatus *status) {
    if (string_is(value, "InvalidLbaError")) *status = HDD_INVALID_LBA;
    else if (string_is(value, "InvalidSectorDataError")) *status = HDD_INVALID_DATA_LENGTH;
    else if (string_is(value, "ClockOverflowError")) *status = HDD_CLOCK_OVERFLOW;
    else return 0;
    return 1;
}

static int nonempty_string(const BJsonValue *value) {
    const char *text = NULL;
    size_t length = 0;
    return bj_kind(value) == BJ_STRING && bj_string(value, &text, &length) && length > 0;
}

#ifndef HDD_FUZZ_BUILD
static int read_file(char *buffer, size_t capacity, size_t *length) {
    FILE *file = fopen(FIXTURE_PATH, "rb");
    size_t count;
    if (file == NULL) return 0;
    count = fread(buffer, 1, capacity, file);
    if (ferror(file) || count == capacity || fgetc(file) != EOF) {
        fclose(file);
        return 0;
    }
    fclose(file);
    *length = count;
    return 1;
}
#endif

static int fixture_case(const BJsonValue *case_value) {
    static const char *const case_keys[] = {"name", "operations"};
    static const char *const operation_success_keys[] = {"op", "lba", "data_hex", "timing", "state"};
    static const char *const operation_lba_error_keys[] = {"op", "lba", "error", "state_unchanged"};
    static const char *const operation_data_error_keys[] = {"op", "lba", "data_hex", "error", "state_unchanged"};
    static const char *const state_keys[] = {"tick", "head_track"};
    HddSimulator *simulator = NULL;
    size_t i;
    int ok = 1;
    const BJsonValue *ops;
    if (!object_keys_are(case_value, case_keys, 2) || !nonempty_string(member(case_value, "name"))) return 0;
    ops = member(case_value, "operations");
    if (bj_kind(ops) != BJ_ARRAY || bj_size(ops) == 0 || hdd_create(&simulator) != HDD_OK) return 0;
    for (i = 0; ok && i < bj_size(ops); i++) {
        const BJsonValue *op = bj_at(ops, i);
        const BJsonValue *op_name = member(op, "op"), *lba_value = member(op, "lba");
        int64_t lba_int;
        size_t lba;
        HddSnapshot before, after;
        HddStatus actual, expected;
        if (bj_kind(op_name) != BJ_STRING || hdd_get_snapshot(simulator, &before) != HDD_OK) { ok = 0; break; }
        if (!integer(lba_value, &lba_int)) {
            if (bj_kind(lba_value) != BJ_BOOL) { ok = 0; break; }
            lba_int = -1; /* JSON boolean must be rejected as an LBA. */
        }
        lba = lba_int < 0 || lba_int >= (int64_t)HDD_SECTOR_COUNT
            ? HDD_SECTOR_COUNT : (size_t)lba_int;
        if (string_is(op_name, "read") || string_is(op_name, "write")) {
            const BJsonValue *data = member(op, "data_hex"), *state = member(op, "state");
            const BJsonValue *error = member(op, "error");
            uint8_t bytes[HDD_SECTOR_BYTES];
            uint8_t expected_bytes[HDD_SECTOR_BYTES] = {0};
            HddTiming timing, expected_timing;
            size_t error_payload_length = 0;
            const int is_read = string_is(op_name, "read");
            if (error != NULL) {
                const int is_data_error = string_is(error, "InvalidSectorDataError");
                int unchanged = 0;
                if (!fixture_error_status(error, &expected) ||
                    !object_keys_are(op, is_data_error ? operation_data_error_keys : operation_lba_error_keys,
                        is_data_error ? 5 : 4) ||
                    bj_kind(member(op, "state_unchanged")) != BJ_BOOL ||
                    !bj_boolean(member(op, "state_unchanged"), &unchanged) || !unchanged) { ok = 0; break; }
                if (is_data_error) {
                    if (is_read || !decode_hex(data, expected_bytes, sizeof expected_bytes - 1, &error_payload_length) ||
                        error_payload_length == HDD_SECTOR_BYTES) { ok = 0; break; }
                }
                {
                    uint8_t original_bytes[sizeof bytes];
                    memset(bytes, 0xA5, sizeof bytes);
                    if (is_data_error) memcpy(bytes, expected_bytes, sizeof bytes);
                    memset(&timing, 0xA5, sizeof timing);
                    memcpy(original_bytes, bytes, sizeof bytes);
                    if (is_data_error) actual = hdd_write_sector(simulator, lba, bytes,
                        error_payload_length, &timing);
                    else if (is_read) actual = hdd_read_sector(simulator, lba, bytes, sizeof bytes, &timing);
                    else actual = hdd_write_sector(simulator, lba, bytes, sizeof bytes, &timing);
                    if (actual != expected || hdd_get_snapshot(simulator, &after) != HDD_OK ||
                        !same_snapshot(&before, &after) || memcmp(original_bytes, bytes, sizeof bytes) != 0) ok = 0;
                    {
                        HddTiming untouched;
                        memset(&untouched, 0xA5, sizeof untouched);
                        if (memcmp(&timing, &untouched, sizeof timing) != 0) ok = 0;
                    }
                }
                continue;
            }
            if (!object_keys_are(op, operation_success_keys, 5) ||
                !hex_bytes(data, expected_bytes, sizeof expected_bytes) || !parse_timing(member(op, "timing"), &expected_timing) ||
                !object_keys_are(state, state_keys, 2)) { ok = 0; break; }
            if (is_read) memset(bytes, 0xA5, sizeof bytes);
            else memcpy(bytes, expected_bytes, sizeof bytes);
            memset(&timing, 0xA5, sizeof timing);
            actual = is_read ? hdd_read_sector(simulator, lba, bytes, sizeof bytes, &timing)
                : hdd_write_sector(simulator, lba, bytes, sizeof bytes, &timing);
            if (actual != HDD_OK || !same_timing(&timing, &expected_timing)) { ok = 0; break; }
            if (is_read) {
                if (memcmp(bytes, expected_bytes, sizeof bytes) != 0) { ok = 0; break; }
            } else if (memcmp(bytes, expected_bytes, sizeof bytes) != 0) {
                ok = 0; break;
            }
            if (hdd_get_snapshot(simulator, &after) != HDD_OK) { ok = 0; break; }
            {
                int64_t tick, track;
                if (!integer(member(state, "tick"), &tick) || !integer(member(state, "head_track"), &track) ||
                    tick < 0 || track < 0 || after.tick != (uint64_t)tick || after.head_track != (size_t)track) ok = 0;
            }
            if (!is_read) memcpy(before.sectors[lba], expected_bytes, HDD_SECTOR_BYTES);
            if (memcmp(after.sectors, before.sectors, sizeof after.sectors) != 0) ok = 0;
        } else ok = 0;
    }
    hdd_destroy(simulator);
    if (!ok) {
        const char *name = NULL; size_t name_length = 0;
        if (bj_string(member(case_value, "name"), &name, &name_length))
            fprintf(stderr, "fixture case failed: %.*s operation %lu\n", (int)name_length, name, (unsigned long)i);
    }
    return ok;
}

/* The reference model owns every sector and advances clock ticks explicitly.
 * It checks JSON expectations before the runtime adapter replays the trace. */
static int model_case(const BJsonValue *case_value) {
    static const char *const keys[] = {"name", "operations"};
    static const char *const good_keys[] = {"op", "lba", "data_hex", "timing", "state"};
    static const char *const bad_lba_keys[] = {"op", "lba", "error", "state_unchanged"};
    static const char *const bad_data_keys[] = {"op", "lba", "data_hex", "error", "state_unchanged"};
    static const char *const state_keys[] = {"tick", "head_track"};
    const BJsonValue *ops = member(case_value, "operations");
    HddModel model = {0};
    size_t i;
    if (!object_keys_are(case_value, keys, 2) || !nonempty_string(member(case_value, "name")) ||
        bj_kind(ops) != BJ_ARRAY || bj_size(ops) == 0) return 0;
    for (i = 0; i < bj_size(ops); i++) {
        const BJsonValue *op = bj_at(ops, i), *lba_value = member(op, "lba");
        const BJsonValue *error = member(op, "error"), *name = member(op, "op");
        int reading = string_is(name, "read"), writing = string_is(name, "write");
        int bool_lba = bj_kind(lba_value) == BJ_BOOL;
        int64_t number = -1;
        size_t lba = HDD_SECTOR_COUNT;
        HddModel before = model, next;
        HddTiming calculated, declared;
        uint8_t bytes[HDD_SECTOR_BYTES] = {0};
        if ((!reading && !writing) || (!bool_lba && !integer(lba_value, &number))) return 0;
        if (!bool_lba && number >= 0 && number < (int64_t)HDD_SECTOR_COUNT) lba = (size_t)number;
        if (error != NULL) {
            HddStatus status;
            int unchanged = 0;
            if (!fixture_error_status(error, &status) ||
                bj_kind(member(op, "state_unchanged")) != BJ_BOOL ||
                !bj_boolean(member(op, "state_unchanged"), &unchanged) || !unchanged) return 0;
            if (status == HDD_INVALID_LBA) {
                if (lba != HDD_SECTOR_COUNT || !object_keys_are(op, bad_lba_keys, 4)) return 0;
            } else if (status == HDD_INVALID_DATA_LENGTH) {
                size_t payload_length = 0;
                if (!writing || lba == HDD_SECTOR_COUNT || !object_keys_are(op, bad_data_keys, 5) ||
                    !decode_hex(member(op, "data_hex"), bytes, sizeof bytes, &payload_length) ||
                    payload_length == HDD_SECTOR_BYTES) return 0;
            } else if (status == HDD_CLOCK_OVERFLOW) {
                next = before;
                if (lba == HDD_SECTOR_COUNT || !object_keys_are(op, bad_lba_keys, 4) ||
                    model_timing(&next, lba, &calculated) || !same_model_state(&next, &before)) return 0;
            } else return 0;
            continue;
        }
        if (lba == HDD_SECTOR_COUNT || !object_keys_are(op, good_keys, 5) ||
            !hex_bytes(member(op, "data_hex"), bytes, sizeof bytes) ||
            !parse_timing(member(op, "timing"), &declared) ||
            !object_keys_are(member(op, "state"), state_keys, 2)) return 0;
        next = before;
        if (!model_timing(&next, lba, &calculated) || !same_timing(&calculated, &declared)) return 0;
        if (reading) {
            uint8_t declared_bytes[HDD_SECTOR_BYTES];
            if (!hex_bytes(member(op, "data_hex"), declared_bytes, sizeof declared_bytes) ||
                memcmp(declared_bytes, before.media[lba], sizeof declared_bytes) != 0) return 0;
        } else memcpy(next.media[lba], bytes, sizeof bytes);
        {
            const BJsonValue *state = member(op, "state");
            int64_t tick, head;
            if (!integer(member(state, "tick"), &tick) || !integer(member(state, "head_track"), &head) ||
                tick < 0 || head < 0 || (uint64_t)tick != next.tick || (size_t)head != next.head) return 0;
        }
        model = next;
    }
    return 1;
}

static int model_probe(const BJsonValue *probe) {
    static const char *const success_keys[] = {"name", "start_tick", "head_track", "target_track", "target_sector", "timing"};
    static const char *const error_keys[] = {"name", "start_tick", "head_track", "target_track", "target_sector", "error"};
    const BJsonValue *error = member(probe, "error");
    int64_t start, head, target, sector;
    HddModel initial = {0}, next;
    HddTiming calculated, declared;
    size_t lba;
    if (!object_keys_are(probe, error ? error_keys : success_keys, 6) ||
        !nonempty_string(member(probe, "name")) || !integer(member(probe, "start_tick"), &start) || start < 0 ||
        (uint64_t)start > HDD_MAX_TICK || !integer(member(probe, "head_track"), &head) || head < 0 || head >= 4 ||
        !integer(member(probe, "target_track"), &target) || target < 0 || target >= 4 ||
        !integer(member(probe, "target_sector"), &sector) || sector < 0 || sector >= 4) return 0;
    initial.tick = (uint64_t)start;
    initial.head = (size_t)head;
    lba = (size_t)target * 8 + (size_t)sector;
    next = initial;
    if (error) return string_is(error, "ClockOverflowError") &&
        !model_timing(&next, lba, &calculated) && same_model_state(&initial, &next);
    if (!parse_timing(member(probe, "timing"), &declared) || !model_timing(&next, lba, &calculated)) return 0;
    return same_timing(&calculated, &declared);
}

static int model_fixture(const BJsonValue *root) {
    const BJsonValue *cases = member(root, "cases"), *probes = member(root, "timing_probes");
    size_t i;
    for (i = 0; i < bj_size(cases); i++) if (!model_case(bj_at(cases, i))) return 0;
    for (i = 0; i < bj_size(probes); i++) if (!model_probe(bj_at(probes, i))) return 0;
    return 1;
}

static HddFuzzResult process_fixture_bytes(const char *text, size_t length) {
    BJsonDoc *doc = NULL;
    BJsonError error;
    const BJsonValue *root, *profile, *initial, *cases;
    static const char *const root_keys[] = {"schema", "profile", "initial_state", "cases", "timing_probes"};
    static const char *const profile_keys[] = {"surfaces", "tracks_per_surface", "sectors_per_track",
        "bytes_per_sector", "seek_ticks_per_track", "transfer_ticks_per_sector"};
    static const char *const initial_keys[] = {"tick", "head_track", "sector_count", "sector_fill_byte"};
    static const char *const probe_keys[] = {"name", "start_tick", "head_track", "target_track", "target_sector", "timing"};
    static const char *const probe_error_keys[] = {"name", "start_tick", "head_track", "target_track", "target_sector", "error"};
    const BJsonValue *probes;
    size_t i, operation_count = 0;
    int valid = 0;
    BJsonOptions options = bj_options();
    options.max_bytes = MAX_FIXTURE_BYTES;
    options.max_depth = 64;
    options.max_nodes = 16384;
    options.max_alloc_bytes = 8U * 1024U * 1024U;
    if (length > MAX_FIXTURE_BYTES || bj_parse_with_options(text, length, &options, &doc, &error) != BJ_OK)
        return HDD_FUZZ_MALFORMED;
    root = bj_root(doc);
    profile = member(root, "profile"); initial = member(root, "initial_state");
    cases = member(root, "cases"); probes = member(root, "timing_probes");
    if (!object_keys_are(root, root_keys, 5) || !string_is(member(root, "schema"),
        "coding-adventures.database-media.hdd-simulator.v1") ||
        !object_keys_are(profile, profile_keys, 6) || !object_keys_are(initial, initial_keys, 4) ||
        bj_kind(cases) != BJ_ARRAY || bj_size(cases) == 0 ||
        bj_kind(probes) != BJ_ARRAY || bj_size(probes) == 0) goto done;
    {
        const int64_t profile_values[] = {2, 4, 4, 8, 3, 1};
        const char *profile_names[] = {"surfaces", "tracks_per_surface", "sectors_per_track",
            "bytes_per_sector", "seek_ticks_per_track", "transfer_ticks_per_sector"};
        const int64_t initial_values[] = {0, 0, 32, 0};
        const char *initial_names[] = {"tick", "head_track", "sector_count", "sector_fill_byte"};
        for (i = 0; i < 6; i++) { int64_t n; if (!integer(member(profile, profile_names[i]), &n) || n != profile_values[i]) goto done; }
        for (i = 0; i < 4; i++) { int64_t n; if (!integer(member(initial, initial_names[i]), &n) || n != initial_values[i]) goto done; }
    }
    if (!model_fixture(root)) goto done;
    for (i = 0; i < bj_size(cases); i++) {
        const BJsonValue *ops = member(bj_at(cases, i), "operations");
        if (bj_kind(ops) != BJ_ARRAY) goto done;
        if (bj_size(ops) > HDD_FUZZ_MAX_OPERATIONS - operation_count) {
            valid = 2;
            goto done;
        }
        operation_count += bj_size(ops);
    }
    if (bj_size(probes) > HDD_FUZZ_MAX_PROBES) { valid = 2; goto done; }
    valid = 3; /* Schema and all declared results agree with the independent model. */
    for (i = 0; i < bj_size(cases); i++) if (!fixture_case(bj_at(cases, i))) goto done;
    for (i = 0; i < bj_size(probes); i++) {
        const BJsonValue *probe = bj_at(probes, i);
        const BJsonValue *probe_error = member(probe, "error");
        int64_t start, head, target, sector;
        HddSimulator *simulator = NULL;
        HddTiming actual, expected;
        uint8_t bytes[HDD_SECTOR_BYTES];
        HddTiming untouched;
        HddSnapshot before, after;
        int should_overflow = probe_error != NULL;
        if (!object_keys_are(probe, should_overflow ? probe_error_keys : probe_keys, 6) ||
            !nonempty_string(member(probe, "name")) ||
            !integer(member(probe, "start_tick"), &start) || start < 0 ||
            !integer(member(probe, "head_track"), &head) || head < 0 ||
            !integer(member(probe, "target_track"), &target) || target < 0 ||
            !integer(member(probe, "target_sector"), &sector) || sector < 0 ||
            head >= 4 || target >= 4 || sector >= 4 ||
            (should_overflow && !string_is(probe_error, "ClockOverflowError")) ||
            (!should_overflow && !parse_timing(member(probe, "timing"), &expected)) ||
            hdd_create(&simulator) != HDD_OK) { hdd_destroy(simulator); goto done; }
        if (!hdd_test_set_state(simulator, (uint64_t)start, (size_t)head)) {
            hdd_destroy(simulator); goto done;
        }
        memset(bytes, 0xA5, sizeof bytes);
        memset(&actual, 0xA5, sizeof actual);
        memset(&untouched, 0xA5, sizeof untouched);
        if (hdd_get_snapshot(simulator, &before) != HDD_OK) { hdd_destroy(simulator); goto done; }
        {
            HddStatus status = hdd_read_sector(simulator, (size_t)target * 8 + (size_t)sector,
                bytes, sizeof bytes, &actual);
            if (should_overflow ? status != HDD_CLOCK_OVERFLOW
                : status != HDD_OK || !same_timing(&actual, &expected)) {
                hdd_destroy(simulator); goto done;
            }
        }
        if (hdd_get_snapshot(simulator, &after) != HDD_OK ||
            (should_overflow && (memcmp(&actual, &untouched, sizeof actual) != 0 ||
                memcmp(bytes, "\xA5\xA5\xA5\xA5\xA5\xA5\xA5\xA5", sizeof bytes) != 0 ||
                !same_snapshot(&before, &after)))) {
            hdd_destroy(simulator); goto done;
        }
        if (!should_overflow) {
            HddSnapshot expected_after = before;
            const uint8_t zero_sector[HDD_SECTOR_BYTES] = {0};
            expected_after.tick = expected.end_tick;
            expected_after.head_track = (size_t)target;
            if (memcmp(bytes, zero_sector, sizeof bytes) != 0 ||
                !same_snapshot(&expected_after, &after)) {
                hdd_destroy(simulator); goto done;
            }
        }
        hdd_destroy(simulator);
    }
    valid = 1;
done:
    bj_doc_free(&doc);
    if (valid == 1) return HDD_FUZZ_ACCEPTED;
    if (valid == 2) return HDD_FUZZ_OUTSIDE_DOMAIN;
    if (valid == 3) return HDD_FUZZ_RUNTIME_MISMATCH;
    return HDD_FUZZ_MALFORMED;
}

#ifndef HDD_FUZZ_BUILD
static int run_fixture_bytes(const char *text, size_t length) {
    return process_fixture_bytes(text, length) == HDD_FUZZ_ACCEPTED;
}
#endif

static const char *model_status_message(HddStatus status) {
    switch (status) {
        case HDD_OK: return "success";
        case HDD_INVALID_ARGUMENT: return "invalid API argument";
        case HDD_INVALID_LBA: return "LBA out of range";
        case HDD_INVALID_DATA: return "sector data must be bytes";
        case HDD_INVALID_DATA_LENGTH: return "sector data must match bytes_per_sector";
        case HDD_CLOCK_OVERFLOW: return "clock overflow";
        default: return "unknown status";
    }
}

static int same_model_snapshot(const HddSnapshot *snapshot, const HddModel *model) {
    return snapshot->tick == model->tick && snapshot->head_track == model->head &&
        memcmp(snapshot->sectors, model->media, sizeof model->media) == 0;
}

static HddFuzzResult process_history(const uint8_t *bytes, size_t length) {
    HddModel model = {0};
    HddSimulator *simulator = NULL;
    size_t count, i;
    uint64_t tick = 0;
    size_t head;
    if (bytes == NULL || length < 11 || bytes[0] != 0x48U || bytes[1] > HDD_FUZZ_MAX_COMMANDS)
        return HDD_FUZZ_MALFORMED;
    count = bytes[1];
    for (i = 0; i < 8; i++) tick |= (uint64_t)bytes[2 + i] << (i * 8);
    head = bytes[10];
    if (tick > HDD_MAX_TICK || head >= 4 ||
        count > (SIZE_MAX - 11) / 8 || length != 11 + count * 8) return HDD_FUZZ_MALFORMED;
    /* Reject malformed records before opening a device or executing a prefix. */
    for (i = 0; i < count; i++) {
        const uint8_t *record = bytes + 11 + i * 8;
        if (record[0] > 1U || (record[2] & 0xFCU) != 0) return HDD_FUZZ_MALFORMED;
    }
    model.tick = tick;
    model.head = head;
    if (hdd_create(&simulator) != HDD_OK || !hdd_test_set_state(simulator, tick, head)) {
        hdd_destroy(simulator);
        return HDD_FUZZ_RUNTIME_MISMATCH;
    }
    for (i = 0; i < count; i++) {
        const uint8_t *record = bytes + 11 + i * 8;
        const unsigned opcode = record[0], lba = record[1], flags = record[2], data_length = record[3];
        const int is_read = opcode == 0U, is_write = opcode == 1U;
        const int null_buffer = (flags & 1U) != 0, null_timing = (flags & 2U) != 0;
        uint8_t payload[HDD_FUZZ_BUFFER_BYTES], output[HDD_FUZZ_BUFFER_BYTES];
        uint8_t original_payload[HDD_FUZZ_BUFFER_BYTES];
        HddTiming timing, expected_timing, timing_sentinel;
        HddSnapshot snapshot;
        HddStatus expected_status, actual_status;
        HddModel before = model, after = model;
        size_t j;
        int timing_ok = 0;
        if ((!is_read && !is_write) || (flags & ~3U) != 0) {
            hdd_destroy(simulator);
            return HDD_FUZZ_MALFORMED;
        }
        for (j = 0; j < HDD_FUZZ_BUFFER_BYTES; j++) {
            payload[j] = record[4 + (j % 4)];
            output[j] = 0xA5U;
        }
        memcpy(original_payload, payload, sizeof payload);
        memset(&timing, 0xA5, sizeof timing);
        memset(&expected_timing, 0, sizeof expected_timing);
        memset(&timing_sentinel, 0xA5, sizeof timing_sentinel);
        if (lba >= HDD_SECTOR_COUNT) expected_status = HDD_INVALID_LBA;
        else if (null_buffer) expected_status = HDD_INVALID_DATA;
        else if (data_length != HDD_SECTOR_BYTES) expected_status = HDD_INVALID_DATA_LENGTH;
        else if (null_timing) expected_status = HDD_INVALID_ARGUMENT;
        else if (!model_timing(&after, lba, &expected_timing)) expected_status = HDD_CLOCK_OVERFLOW;
        else {
            expected_status = HDD_OK;
            timing_ok = 1;
            if (is_write) memcpy(after.media[lba], payload, HDD_SECTOR_BYTES);
        }
        actual_status = is_read
            ? hdd_read_sector(simulator, lba, null_buffer ? NULL : output, data_length,
                null_timing ? NULL : &timing)
            : hdd_write_sector(simulator, lba, null_buffer ? NULL : payload, data_length,
                null_timing ? NULL : &timing);
        if (actual_status != expected_status ||
            strcmp(hdd_status_message(actual_status), model_status_message(expected_status)) != 0 ||
            (timing_ok ? !same_timing(&timing, &expected_timing)
                : memcmp(&timing, &timing_sentinel, sizeof timing) != 0)) {
            hdd_destroy(simulator);
            return HDD_FUZZ_RUNTIME_MISMATCH;
        }
        if (is_read) {
            uint8_t expected_output[HDD_FUZZ_BUFFER_BYTES];
            memset(expected_output, 0xA5, sizeof expected_output);
            if (timing_ok) memcpy(expected_output, before.media[lba], HDD_SECTOR_BYTES);
            if (memcmp(output, expected_output, sizeof output) != 0) {
                hdd_destroy(simulator);
                return HDD_FUZZ_RUNTIME_MISMATCH;
            }
        } else if (memcmp(payload, original_payload, sizeof payload) != 0) {
            hdd_destroy(simulator);
            return HDD_FUZZ_RUNTIME_MISMATCH;
        }
        if (timing_ok) model = after;
        if (hdd_get_snapshot(simulator, &snapshot) != HDD_OK || !same_model_snapshot(&snapshot, &model)) {
            hdd_destroy(simulator);
            return HDD_FUZZ_RUNTIME_MISMATCH;
        }
    }
    hdd_destroy(simulator);
    return HDD_FUZZ_ACCEPTED;
}

/* Called by the optional fuzz driver; malformed and outside-domain values are
 * expected outcomes, while only accepted-model/runtime disagreement aborts. */
HddFuzzResult hdd_fuzz_process(const uint8_t *bytes, size_t length, int mode) {
    if (mode == 1) return process_fixture_bytes((const char *)bytes, length);
    if (mode == 2) return process_history(bytes, length);
    return HDD_FUZZ_MALFORMED;
}

#ifndef HDD_FUZZ_BUILD
static void test_public_basics(void) {
    HddSimulator *simulator = NULL;
    HddSnapshot snap, saved;
    HddTiming timing;
    uint8_t source[HDD_SECTOR_BYTES] = {1,2,3,4,5,6,7,8};
    uint8_t output[HDD_SECTOR_BYTES];
    const uint8_t expected[HDD_SECTOR_BYTES] = {1,2,3,4,5,6,7,8};
    const uint8_t zeros[HDD_SECTOR_BYTES] = {0};
    ISO_CHECK_EQ_INT(hdd_create(&simulator), HDD_OK);
    ISO_CHECK(simulator != NULL);
    ISO_CHECK_EQ_INT(hdd_get_snapshot(simulator, &snap), HDD_OK);
    ISO_CHECK(snap.tick == 0);
    ISO_CHECK_EQ_UINT(snap.head_track, 0);
    memset(output, 0xA5, sizeof output);
    ISO_CHECK_EQ_INT(hdd_read_sector(simulator, 31, output, sizeof output, &timing), HDD_OK);
    ISO_CHECK(timing.seek_ticks == 9);
    ISO_CHECK(timing.rotation_ticks == 2);
    ISO_CHECK(timing.total_ticks == 12);
    ISO_CHECK(timing.end_tick == 12);
    ISO_CHECK_MEM_EQ(output, snap.sectors[31], sizeof output);
    ISO_CHECK_EQ_INT(hdd_get_snapshot(simulator, &saved), HDD_OK);
    ISO_CHECK_EQ_INT(hdd_write_sector(simulator, 3, source, sizeof source, &timing), HDD_OK);
    memset(source, 0, sizeof source);
    ISO_CHECK_EQ_INT(hdd_get_snapshot(simulator, &snap), HDD_OK);
    ISO_CHECK_MEM_EQ(snap.sectors[3], expected, HDD_SECTOR_BYTES);
    ISO_CHECK_MEM_EQ(saved.sectors[3], zeros, HDD_SECTOR_BYTES);
    ISO_CHECK_STR_EQ(hdd_status_message(HDD_INVALID_ARGUMENT), "invalid API argument");
    ISO_CHECK_STR_EQ(hdd_status_message(HDD_INVALID_LBA), "LBA out of range");
    ISO_CHECK_STR_EQ(hdd_status_message(HDD_INVALID_DATA), "sector data must be bytes");
    ISO_CHECK_STR_EQ(hdd_status_message(HDD_INVALID_DATA_LENGTH), "sector data must match bytes_per_sector");
    ISO_CHECK_STR_EQ(hdd_status_message(HDD_CLOCK_OVERFLOW), "clock overflow");
    ISO_CHECK_STR_EQ(hdd_status_message(HDD_ALLOCATION_ERROR), "allocation failed");
    ISO_CHECK_STR_EQ(hdd_status_message((HddStatus)999), "unknown status");
    hdd_destroy(simulator);
    hdd_destroy(NULL);
}

static void test_precedence_atomicity_and_clock(void) {
    HddSimulator *simulator = NULL;
    HddSnapshot before, after;
    HddTiming timing, original;
    uint8_t data[HDD_SECTOR_BYTES] = {0}, out[HDD_SECTOR_BYTES];
    uint8_t sentinel[HDD_SECTOR_BYTES];
    memset(&timing, 0x5A, sizeof timing); original = timing;
    ISO_CHECK_EQ_INT(hdd_create(&simulator), HDD_OK);
    ISO_CHECK_EQ_INT(hdd_read_sector(NULL, 32, NULL, 0, NULL), HDD_INVALID_ARGUMENT);
    ISO_CHECK_EQ_INT(hdd_read_sector(simulator, 32, NULL, 0, NULL), HDD_INVALID_LBA);
    ISO_CHECK_EQ_INT(hdd_read_sector(simulator, 0, NULL, 0, NULL), HDD_INVALID_DATA);
    ISO_CHECK_EQ_INT(hdd_read_sector(simulator, 0, out, 0, NULL), HDD_INVALID_DATA_LENGTH);
    ISO_CHECK_EQ_INT(hdd_read_sector(simulator, 0, out, sizeof out, NULL), HDD_INVALID_ARGUMENT);
    ISO_CHECK_EQ_INT(hdd_write_sector(simulator, 0, NULL, 1, NULL), HDD_INVALID_DATA);
    ISO_CHECK_EQ_INT(hdd_write_sector(simulator, 0, data, 1, NULL), HDD_INVALID_DATA_LENGTH);
    ISO_CHECK_EQ_INT(hdd_write_sector(simulator, 0, data, sizeof data, NULL), HDD_INVALID_ARGUMENT);
    ISO_CHECK_EQ_INT(hdd_get_snapshot(simulator, &before), HDD_OK);
    ISO_CHECK(hdd_test_set_state(simulator, HDD_MAX_TICK, 0));
    memset(&timing, 0x5A, sizeof timing); original = timing;
    ISO_CHECK_EQ_INT(hdd_read_sector(simulator, 0, out, sizeof out, &timing), HDD_CLOCK_OVERFLOW);
    ISO_CHECK(memcmp(&timing, &original, sizeof timing) == 0);
    ISO_CHECK_EQ_INT(hdd_get_snapshot(simulator, &after), HDD_OK);
    ISO_CHECK(after.tick == HDD_MAX_TICK);
    ISO_CHECK_EQ_UINT(after.head_track, 0);
    ISO_CHECK_MEM_EQ(after.sectors, before.sectors, sizeof after.sectors);
    ISO_CHECK(hdd_test_set_state(simulator, HDD_MAX_TICK - 1, 0));
    ISO_CHECK_EQ_INT(hdd_read_sector(simulator, 2, out, sizeof out, &timing), HDD_OK);
    ISO_CHECK(timing.end_tick == HDD_MAX_TICK);
    ISO_CHECK(hdd_test_set_state(simulator, HDD_MAX_TICK - 2, 0));
    ISO_CHECK_EQ_INT(hdd_read_sector(simulator, 31, out, sizeof out, &timing), HDD_CLOCK_OVERFLOW);
    ISO_CHECK_EQ_INT(hdd_get_snapshot(simulator, &after), HDD_OK);
    ISO_CHECK(after.tick == HDD_MAX_TICK - 2);
    memset(data, 0xC7, sizeof data);
    memset(sentinel, 0xC7, sizeof sentinel);
    memset(&timing, 0x5A, sizeof timing); original = timing;
    ISO_CHECK_EQ_INT(hdd_write_sector(simulator, 31, data, sizeof data, &timing), HDD_CLOCK_OVERFLOW);
    ISO_CHECK_MEM_EQ(data, sentinel, sizeof data);
    ISO_CHECK(memcmp(&timing, &original, sizeof timing) == 0);
    ISO_CHECK_EQ_INT(hdd_get_snapshot(simulator, &after), HDD_OK);
    ISO_CHECK(after.tick == HDD_MAX_TICK - 2);
    hdd_destroy(simulator);
}

static void test_all_timing_states(void) {
    size_t head, target, sector, surface, phase;
    for (head = 0; head < 4; head++) for (target = 0; target < 4; target++)
        for (sector = 0; sector < 4; sector++) for (surface = 0; surface < 2; surface++)
        for (phase = 0; phase < 4; phase++) {
            HddSimulator *simulator = NULL;
            HddTiming got;
            uint8_t bytes[HDD_SECTOR_BYTES];
            const size_t distance = head > target ? head - target : target - head;
            const uint64_t seek = (uint64_t)(distance * 3);
            const uint64_t rotation = (sector + 4 - ((phase + seek) % 4)) % 4;
            const uint64_t total = seek + rotation + 1;
            const size_t lba = target * 8 + surface * 4 + sector;
            ISO_CHECK_EQ_INT(hdd_create(&simulator), HDD_OK);
            ISO_CHECK(hdd_test_set_state(simulator, (uint64_t)phase, head));
            ISO_CHECK_EQ_INT(hdd_read_sector(simulator, lba, bytes, sizeof bytes, &got), HDD_OK);
            ISO_CHECK(got.start_tick == phase);
            ISO_CHECK(got.seek_ticks == seek);
            ISO_CHECK(got.rotation_ticks == rotation);
            ISO_CHECK(got.transfer_ticks == 1);
            ISO_CHECK(got.total_ticks == total);
            ISO_CHECK(got.end_tick == phase + total);
            hdd_destroy(simulator);
        }
}

static void test_seeded_model(void) {
    HddSimulator *simulator = NULL;
    uint8_t media[HDD_SECTOR_COUNT][HDD_SECTOR_BYTES] = {{0}};
    uint64_t tick = 0;
    size_t head = 0, step;
    uint32_t seed = UINT32_C(0xB02A17);
    ISO_CHECK_EQ_INT(hdd_create(&simulator), HDD_OK);
    for (step = 0; step < 3000; step++) {
        size_t lba, target, i;
        int writing;
        HddTiming got;
        uint8_t bytes[HDD_SECTOR_BYTES], input[HDD_SECTOR_BYTES];
        seed = seed * UINT32_C(1664525) + UINT32_C(1013904223);
        lba = (size_t)(seed % 32U); writing = (int)((seed >> 8) & 1U); target = lba / 8;
        for (i = 0; i < sizeof input; i++) input[i] = (uint8_t)(seed >> ((i % 4) * 8));
        {
            size_t distance = head > target ? head - target : target - head;
            uint64_t seek = (uint64_t)(distance * 3);
            uint64_t rotation = (lba % 4 + 4 - ((tick % 4 + seek) % 4)) % 4;
            uint64_t total = seek + rotation + 1;
            if (writing) {
                ISO_CHECK_EQ_INT(hdd_write_sector(simulator, lba, input, sizeof input, &got), HDD_OK);
                memcpy(media[lba], input, sizeof input);
            } else {
                ISO_CHECK_EQ_INT(hdd_read_sector(simulator, lba, bytes, sizeof bytes, &got), HDD_OK);
                ISO_CHECK_MEM_EQ(bytes, media[lba], sizeof bytes);
            }
            ISO_CHECK(got.start_tick == tick);
            ISO_CHECK(got.seek_ticks == seek);
            ISO_CHECK(got.rotation_ticks == rotation);
            ISO_CHECK(got.total_ticks == total);
            tick += total; head = target;
        }
        {
            HddSnapshot snap;
            ISO_CHECK_EQ_INT(hdd_get_snapshot(simulator, &snap), HDD_OK);
            ISO_CHECK(snap.tick == tick);
            ISO_CHECK_EQ_UINT(snap.head_track, head);
            ISO_CHECK_MEM_EQ(snap.sectors, media, sizeof media);
        }
    }
    hdd_destroy(simulator);
}

static void test_fixture_and_malformed(void) {
    char *buffer = (char *)malloc(MAX_FIXTURE_BYTES + 1);
    size_t length = 0;
    BJsonDoc *doc = NULL;
    BJsonError error;
    ISO_CHECK(buffer != NULL);
    if (buffer == NULL) return;
    ISO_CHECK(read_file(buffer, MAX_FIXTURE_BYTES, &length));
    ISO_CHECK(run_fixture_bytes(buffer, length));
    ISO_CHECK(process_fixture_bytes(buffer, length) == HDD_FUZZ_ACCEPTED);
    {
        char *changed = (char *)malloc(length + 1);
        char *field;
        ISO_CHECK(changed != NULL);
        if (changed != NULL) {
            memcpy(changed, buffer, length + 1);
            field = strstr(changed, "\"rotation_ticks\": 0");
            ISO_CHECK(field != NULL);
            if (field != NULL) {
                field[strlen("\"rotation_ticks\": ")] = '1';
                ISO_CHECK(process_fixture_bytes(changed, length) == HDD_FUZZ_MALFORMED);
            }
            ISO_CHECK(process_fixture_bytes("{\"schema\":1,\"schema\":2}",
                sizeof "{\"schema\":1,\"schema\":2}" - 1) == HDD_FUZZ_MALFORMED);
            free(changed);
        }
    }
    ISO_CHECK(bj_parse("{} {}", 5, &doc, &error) != BJ_OK);
    ISO_CHECK(bj_parse("{\"schema\":1,\"schema\":2}", sizeof "{\"schema\":1,\"schema\":2}" - 1, &doc, &error) == BJ_OK);
    bj_doc_free(&doc);
    ISO_CHECK(!run_fixture_bytes("{\"schema\":1,\"schema\":2}", sizeof "{\"schema\":1,\"schema\":2}" - 1));
    ISO_CHECK(bj_parse("{\"schema\":\"unterminated", 23, &doc, &error) != BJ_OK);
    ISO_CHECK(!run_fixture_bytes("{}", 2));
    ISO_CHECK(!run_fixture_bytes("{\"schema\":\"wrong\",\"profile\":{},\"initial_state\":{},\"cases\":[{}],\"timing_probes\":[]}",
        sizeof "{\"schema\":\"wrong\",\"profile\":{},\"initial_state\":{},\"cases\":[{}],\"timing_probes\":[]}" - 1));
    ISO_CHECK(!run_fixture_bytes("{\"schema\":\"coding-adventures.database-media.hdd-simulator.v1\",\"profile\":{},\"initial_state\":{},\"cases\":[],\"timing_probes\":[]}",
        sizeof "{\"schema\":\"coding-adventures.database-media.hdd-simulator.v1\",\"profile\":{},\"initial_state\":{},\"cases\":[],\"timing_probes\":[]}" - 1));
    ISO_CHECK(!run_fixture_bytes(buffer, MAX_FIXTURE_BYTES + 1));
    free(buffer);
}

static void test_large_fixture_domain_classification(void) {
    char *json = (char *)malloc(MAX_FIXTURE_BYTES);
    size_t used, i;
    ISO_CHECK(json != NULL);
    if (json == NULL) return;
    used = (size_t)snprintf(json, MAX_FIXTURE_BYTES,
        "{\"schema\":\"coding-adventures.database-media.hdd-simulator.v1\","
        "\"profile\":{\"surfaces\":2,\"tracks_per_surface\":4,\"sectors_per_track\":4,"
        "\"bytes_per_sector\":8,\"seek_ticks_per_track\":3,\"transfer_ticks_per_sector\":1},"
        "\"initial_state\":{\"tick\":0,\"head_track\":0,\"sector_count\":32,\"sector_fill_byte\":0},"
        "\"cases\":[{\"name\":\"bounded-set\",\"operations\":[");
    ISO_CHECK(used < MAX_FIXTURE_BYTES);
    if (used >= MAX_FIXTURE_BYTES) { free(json); return; }
    for (i = 0; i < 65; i++) {
        const unsigned start = i == 0 ? 0U : (unsigned)(4 * i - 3);
        const unsigned rotation = i == 0 ? 0U : 3U;
        const unsigned total = i == 0 ? 1U : 4U;
        const unsigned end = start + total;
        int n = snprintf(json + used, MAX_FIXTURE_BYTES - used,
            "%s{\"op\":\"read\",\"lba\":0,\"data_hex\":\"0000000000000000\","
            "\"timing\":{\"start_tick\":%u,\"seek_ticks\":0,\"rotation_ticks\":%u,"
            "\"transfer_ticks\":1,\"total_ticks\":%u,\"end_tick\":%u},"
            "\"state\":{\"tick\":%u,\"head_track\":0}}",
            i == 0 ? "" : ",", start, rotation, total, end, end);
        ISO_CHECK(n > 0 && (size_t)n < MAX_FIXTURE_BYTES - used);
        if (n <= 0 || (size_t)n >= MAX_FIXTURE_BYTES - used) { free(json); return; }
        used += (size_t)n;
    }
    {
        const char *tail = "]}],\"timing_probes\":[{\"name\":\"domain\",\"start_tick\":0,"
            "\"head_track\":0,\"target_track\":0,\"target_sector\":0,\"timing\":{"
            "\"start_tick\":0,\"seek_ticks\":0,\"rotation_ticks\":0,\"transfer_ticks\":1,"
            "\"total_ticks\":1,\"end_tick\":1}}]}";
        size_t tail_length = strlen(tail);
        ISO_CHECK(tail_length < MAX_FIXTURE_BYTES - used);
        if (tail_length < MAX_FIXTURE_BYTES - used) {
            memcpy(json + used, tail, tail_length);
            used += tail_length;
            json[used] = '\0';
            ISO_CHECK(process_fixture_bytes(json, used) == HDD_FUZZ_OUTSIDE_DOMAIN);
        }
    }
    free(json);
}

static void test_binary_history_adapter(void) {
    uint8_t history[11 + 5 * 8] = {0x48, 5};
    size_t offset = 11;
    const uint8_t commands[][8] = {
        {0, 0, 0, 8, 0, 0, 0, 0},
        {1, 3, 0, 8, 1, 2, 3, 4},
        {0, 3, 0, 8, 0, 0, 0, 0},
        {0, 32, 0, 8, 0, 0, 0, 0},
        {0, 0, 1, 8, 0, 0, 0, 0}
    };
    size_t i;
    for (i = 0; i < 5; i++) memcpy(history + offset + i * 8, commands[i], 8);
    ISO_CHECK(process_history(history, sizeof history) == HDD_FUZZ_ACCEPTED);
    ISO_CHECK(process_history(history, sizeof history - 1) == HDD_FUZZ_MALFORMED);
    history[0] = 0x49;
    ISO_CHECK(process_history(history, sizeof history) == HDD_FUZZ_MALFORMED);
    history[0] = 0x48;
    history[11 + 2] = 4;
    ISO_CHECK(process_history(history, sizeof history) == HDD_FUZZ_MALFORMED);
    history[11 + 2] = 0;
    history[10] = 4;
    ISO_CHECK(process_history(history, sizeof history) == HDD_FUZZ_MALFORMED);
}

int main(void) {
    test_public_basics();
    test_precedence_atomicity_and_clock();
    test_all_timing_states();
    test_seeded_model();
    test_fixture_and_malformed();
    test_large_fixture_domain_classification();
    test_binary_history_adapter();
    return ISO_TEST_RESULT();
}
#endif
