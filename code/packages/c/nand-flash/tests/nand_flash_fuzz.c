#include "nand_flash_fuzz_support.h"

#include <stdlib.h>

/* Mode 0 dispatches by the stable 0x4E history prefix. Modes 1 and 2 are
 * separate campaign builds so libFuzzer cannot drift between input domains. */
#ifndef NAND_FUZZ_MODE
#define NAND_FUZZ_MODE 0
#endif

int LLVMFuzzerTestOneInput(const uint8_t *data, size_t size) {
#if NAND_FUZZ_MODE == 2
    NandHistoryResult history = nand_history_replay(data, size, NULL);
    if (history == NAND_HISTORY_RUNTIME_MISMATCH) abort();
#elif NAND_FUZZ_MODE == 1
    NandFuzzTrace trace;
    NandFixtureDecodeResult decoded = nand_fixture_decode(data, size, &trace);
    if (decoded == NAND_FIXTURE_ACCEPTED &&
        nand_fixture_replay(&trace, NULL) == NAND_REPLAY_RUNTIME_MISMATCH) abort();
#else
    if (size > 0 && data[0] == 0x4Eu) {
        NandHistoryResult history = nand_history_replay(data, size, NULL);
        if (history == NAND_HISTORY_RUNTIME_MISMATCH) abort();
    } else {
        NandFuzzTrace trace;
        NandFixtureDecodeResult decoded = nand_fixture_decode(data, size, &trace);
        if (decoded == NAND_FIXTURE_ACCEPTED &&
            nand_fixture_replay(&trace, NULL) == NAND_REPLAY_RUNTIME_MISMATCH) abort();
    }
#endif
    return 0;
}
