#ifndef CA_HDD_SIMULATOR_FUZZ_SUPPORT_H
#define CA_HDD_SIMULATOR_FUZZ_SUPPORT_H

#include <stddef.h>
#include <stdint.h>

typedef enum {
    HDD_FUZZ_MALFORMED = 0,
    HDD_FUZZ_OUTSIDE_DOMAIN,
    HDD_FUZZ_ACCEPTED,
    HDD_FUZZ_RUNTIME_MISMATCH
} HddFuzzResult;

HddFuzzResult hdd_fuzz_process(const uint8_t *bytes, size_t length, int mode);

#endif
