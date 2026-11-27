#ifndef CA_NAND_FLASH_FUZZ_SUPPORT_H
#define CA_NAND_FLASH_FUZZ_SUPPORT_H

#include <stddef.h>
#include <stdint.h>

#define NAND_FUZZ_BLOCKS 2u
#define NAND_FUZZ_PAGES 2u
#define NAND_FUZZ_PAGE_BYTES 8u
#define NAND_FUZZ_MAX_OPERATIONS 64u
#define NAND_FUZZ_MAX_INPUT_BYTES 65536u

typedef enum {
    NAND_FIXTURE_REJECTED = 0,
    NAND_FIXTURE_OUTSIDE_DOMAIN,
    NAND_FIXTURE_ACCEPTED
} NandFixtureDecodeResult;

typedef enum {
    NAND_REPLAY_MATCH = 0,
    NAND_REPLAY_RUNTIME_MISMATCH
} NandReplayResult;

typedef struct {
    uint8_t pages[NAND_FUZZ_BLOCKS][NAND_FUZZ_PAGES][NAND_FUZZ_PAGE_BYTES];
    uint64_t erase_counts[NAND_FUZZ_BLOCKS];
    uint8_t programmed[NAND_FUZZ_BLOCKS][NAND_FUZZ_PAGES];
} NandFuzzState;

typedef enum {
    NAND_FUZZ_READ = 0,
    NAND_FUZZ_PROGRAM,
    NAND_FUZZ_ERASE
} NandFuzzOperationKind;

typedef struct {
    NandFuzzOperationKind kind;
    size_t block;
    size_t page;
    uint8_t data[NAND_FUZZ_PAGE_BYTES];
    uint8_t expected_result[NAND_FUZZ_PAGE_BYTES];
    int expected_status;
} NandFuzzOperation;

typedef struct {
    size_t blocks;
    size_t pages_per_block;
    size_t page_size_bytes;
    int has_erase_limit;
    uint64_t max_erase_cycles;
    size_t operation_count;
    NandFuzzState initial;
    NandFuzzOperation operations[NAND_FUZZ_MAX_OPERATIONS];
    NandFuzzState after[NAND_FUZZ_MAX_OPERATIONS];
} NandFuzzTrace;

/* The decoder owns no input bytes. ACCEPTED means both schema and independent
 * model expectations are valid and fit the fixed replay envelope. */
NandFixtureDecodeResult nand_fixture_decode(const uint8_t *bytes, size_t length,
    NandFuzzTrace *out);
NandReplayResult nand_fixture_replay(const NandFuzzTrace *trace,
    size_t *mismatch_operation);

/* A malformed history is refused before device creation. Runtime mismatches
 * are separate so the fuzz entry point crashes only on product disagreement. */
typedef enum {
    NAND_HISTORY_REJECTED = 0,
    NAND_HISTORY_MATCH,
    NAND_HISTORY_RUNTIME_MISMATCH
} NandHistoryResult;
NandHistoryResult nand_history_replay(const uint8_t *bytes, size_t length,
    size_t *mismatch_operation);

/* Test the mismatch classifier without changing the public NAND runtime. */
int nand_fuzz_states_equal(const NandFuzzState *left,
    const NandFuzzState *right, size_t blocks, size_t pages_per_block,
    size_t page_size_bytes);

#endif
