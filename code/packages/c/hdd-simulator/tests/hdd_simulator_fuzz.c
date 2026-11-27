#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include "hdd_simulator_fuzz_support.h"

#ifndef HDD_FUZZ_MODE
#define HDD_FUZZ_MODE 1
#endif

/* Implemented by the ordinary native-test translation unit. This keeps the
 * strict fixture adapter and HDD_TESTING seam identical in tests and fuzzing. */
int LLVMFuzzerTestOneInput(const uint8_t *data, size_t size) {
    const HddFuzzResult result = hdd_fuzz_process(data, size, HDD_FUZZ_MODE);
    /* Expected malformed/out-of-domain inputs are not failures. */
    if (result == HDD_FUZZ_RUNTIME_MISMATCH) abort();
    return 0;
}
