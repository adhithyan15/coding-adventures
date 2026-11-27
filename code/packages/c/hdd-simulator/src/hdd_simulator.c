/* Fixed-geometry synchronous HDD teaching model, profile DB02a. */
#include "hdd_simulator.h"

#include <stdlib.h>
#include <string.h>

struct HddSimulator {
    uint64_t tick;
    size_t head_track;
    uint8_t sectors[HDD_SECTOR_COUNT][HDD_SECTOR_BYTES];
};

static const char *const status_messages[] = {
    "success",
    "invalid API argument",
    "LBA out of range",
    "sector data must be bytes",
    "sector data must match bytes_per_sector",
    "clock overflow",
    "allocation failed"
};

const char *hdd_status_message(HddStatus status) {
    if ((unsigned)status >= sizeof status_messages / sizeof status_messages[0])
        return "unknown status";
    return status_messages[(unsigned)status];
}

HddStatus hdd_create(HddSimulator **out) {
    HddSimulator *simulator;
    if (out == NULL) return HDD_INVALID_ARGUMENT;
    *out = NULL;
    simulator = (HddSimulator *)calloc(1, sizeof *simulator);
    if (simulator == NULL) return HDD_ALLOCATION_ERROR;
    *out = simulator;
    return HDD_OK;
}

void hdd_destroy(HddSimulator *simulator) {
    free(simulator);
}

static HddStatus calculate_timing(const HddSimulator *simulator, size_t lba,
    HddTiming *timing) {
    const size_t target_track = lba / 8;
    const size_t target_sector = lba % 4;
    const size_t distance = target_track > simulator->head_track
        ? target_track - simulator->head_track
        : simulator->head_track - target_track;
    const uint64_t seek = (uint64_t)(distance * 3);
    const uint64_t phase = ((simulator->tick % UINT64_C(4)) + seek) % UINT64_C(4);
    const uint64_t rotation = ((uint64_t)target_sector + UINT64_C(4) - phase) % UINT64_C(4);
    const uint64_t total = seek + rotation + UINT64_C(1);
    if (simulator->tick > HDD_MAX_TICK || total > HDD_MAX_TICK - simulator->tick)
        return HDD_CLOCK_OVERFLOW;
    timing->start_tick = simulator->tick;
    timing->seek_ticks = seek;
    timing->rotation_ticks = rotation;
    timing->transfer_ticks = UINT64_C(1);
    timing->total_ticks = total;
    timing->end_tick = simulator->tick + total;
    return HDD_OK;
}

HddStatus hdd_read_sector(HddSimulator *simulator, size_t lba,
    uint8_t *out, size_t out_length, HddTiming *timing_out) {
    HddTiming timing;
    HddStatus status;
    const size_t target_track = lba / 8;
    if (simulator == NULL) return HDD_INVALID_ARGUMENT;
    if (lba >= HDD_SECTOR_COUNT) return HDD_INVALID_LBA;
    if (out == NULL) return HDD_INVALID_DATA;
    if (out_length != HDD_SECTOR_BYTES) return HDD_INVALID_DATA_LENGTH;
    if (timing_out == NULL) return HDD_INVALID_ARGUMENT;
    status = calculate_timing(simulator, lba, &timing);
    if (status != HDD_OK) return status;
    memcpy(out, simulator->sectors[lba], HDD_SECTOR_BYTES);
    simulator->head_track = target_track;
    simulator->tick = timing.end_tick;
    *timing_out = timing;
    return HDD_OK;
}

HddStatus hdd_write_sector(HddSimulator *simulator, size_t lba,
    const uint8_t *data, size_t data_length, HddTiming *timing_out) {
    HddTiming timing;
    uint8_t copy[HDD_SECTOR_BYTES];
    HddStatus status;
    const size_t target_track = lba / 8;
    if (simulator == NULL) return HDD_INVALID_ARGUMENT;
    if (lba >= HDD_SECTOR_COUNT) return HDD_INVALID_LBA;
    if (data == NULL) return HDD_INVALID_DATA;
    if (data_length != HDD_SECTOR_BYTES) return HDD_INVALID_DATA_LENGTH;
    if (timing_out == NULL) return HDD_INVALID_ARGUMENT;
    status = calculate_timing(simulator, lba, &timing);
    if (status != HDD_OK) return status;
    memcpy(copy, data, sizeof copy);
    memcpy(simulator->sectors[lba], copy, sizeof copy);
    simulator->head_track = target_track;
    simulator->tick = timing.end_tick;
    *timing_out = timing;
    return HDD_OK;
}

HddStatus hdd_get_snapshot(const HddSimulator *simulator, HddSnapshot *out) {
    HddSnapshot snapshot;
    if (simulator == NULL) return HDD_INVALID_ARGUMENT;
    if (out == NULL) return HDD_INVALID_ARGUMENT;
    snapshot.tick = simulator->tick;
    snapshot.head_track = simulator->head_track;
    memcpy(snapshot.sectors, simulator->sectors, sizeof snapshot.sectors);
    *out = snapshot;
    return HDD_OK;
}

#ifdef HDD_TESTING
/* Private seam used only when the native test translation unit includes this
 * implementation. It cannot be linked into an ordinary package build. */
static int hdd_test_set_state(HddSimulator *simulator, uint64_t tick,
    size_t head_track) {
    if (simulator == NULL || tick > HDD_MAX_TICK || head_track >= 4) return 0;
    simulator->tick = tick;
    simulator->head_track = head_track;
    return 1;
}
#endif
