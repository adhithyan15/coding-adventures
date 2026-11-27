#include "hdd_simulator.h"

int main(void) {
    HddSimulator *simulator = NULL;
    HddSnapshot snapshot;
    HddTiming timing;
    uint8_t bytes[HDD_SECTOR_BYTES];
    HddStatus status = hdd_create(&simulator);
    if (status != HDD_OK) return 1;
    status = hdd_read_sector(simulator, 0, bytes, sizeof bytes, &timing);
    if (status == HDD_OK) status = hdd_get_snapshot(simulator, &snapshot);
    hdd_destroy(simulator);
    return status == HDD_OK && snapshot.tick == timing.end_tick ? 0 : 1;
}
