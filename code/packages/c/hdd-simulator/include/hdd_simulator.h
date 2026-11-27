#ifndef CA_HDD_SIMULATOR_H
#define CA_HDD_SIMULATOR_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define HDD_SECTOR_COUNT ((size_t)32)
#define HDD_SECTOR_BYTES ((size_t)8)
#define HDD_MAX_TICK UINT64_C(9007199254740991)

typedef struct HddSimulator HddSimulator;
typedef enum {
    HDD_OK = 0,
    HDD_INVALID_ARGUMENT,
    HDD_INVALID_LBA,
    HDD_INVALID_DATA,
    HDD_INVALID_DATA_LENGTH,
    HDD_CLOCK_OVERFLOW,
    HDD_ALLOCATION_ERROR
} HddStatus;

typedef struct {
    uint64_t start_tick;
    uint64_t seek_ticks;
    uint64_t rotation_ticks;
    uint64_t transfer_ticks;
    uint64_t total_ticks;
    uint64_t end_tick;
} HddTiming;

typedef struct {
    uint64_t tick;
    size_t head_track;
    uint8_t sectors[HDD_SECTOR_COUNT][HDD_SECTOR_BYTES];
} HddSnapshot;

HddStatus hdd_create(HddSimulator **out);
void hdd_destroy(HddSimulator *simulator);
const char *hdd_status_message(HddStatus status);
HddStatus hdd_read_sector(HddSimulator *simulator, size_t lba,
    uint8_t *out, size_t out_length, HddTiming *timing_out);
HddStatus hdd_write_sector(HddSimulator *simulator, size_t lba,
    const uint8_t *data, size_t data_length, HddTiming *timing_out);
HddStatus hdd_get_snapshot(const HddSimulator *simulator, HddSnapshot *out);

#ifdef __cplusplus
}
#endif
#endif
