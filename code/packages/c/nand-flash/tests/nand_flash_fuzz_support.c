#include "nand_flash_fuzz_support.h"

#include "bounded_json.h"
#include "nand_flash.h"

#include <limits.h>
#include <stdlib.h>
#include <string.h>

#define NAND_SAFE_INTEGER INT64_C(9007199254740991)
#define NAND_DB01_MAX_STORAGE ((size_t)16777216)
#define HISTORY_PREFIX 0x4Eu
#define HISTORY_RECORD_BYTES 8u

typedef struct {
    NandFlashStatus status;
    uint8_t bytes[NAND_FUZZ_PAGE_BYTES];
} ModelRead;

static const BJsonValue *member(const BJsonValue *value, const char *wanted) {
    size_t i;
    if (bj_kind(value) != BJ_OBJECT) return NULL;
    for (i = 0; i < bj_size(value); i++) {
        const char *name;
        size_t length;
        const BJsonValue *child;
        if (bj_member(value, i, &name, &length, &child) &&
            length == strlen(wanted) && memcmp(name, wanted, length) == 0) {
            return child;
        }
    }
    return NULL;
}

static int key_is(const char *key, size_t length, const char *expected) {
    return length == strlen(expected) && memcmp(key, expected, length) == 0;
}

/* Indexed object walking makes duplicate detection possible. A first-match
 * lookup alone would let an adversarial second value hide from the schema. */
static int exact_keys(const BJsonValue *value, const char *const *keys,
    size_t key_count) {
    size_t i, j;
    unsigned seen = 0;
    if (bj_kind(value) != BJ_OBJECT || key_count > sizeof(seen) * CHAR_BIT ||
        bj_size(value) != key_count) return 0;
    for (i = 0; i < bj_size(value); i++) {
        const char *name;
        size_t length;
        const BJsonValue *child;
        if (!bj_member(value, i, &name, &length, &child)) return 0;
        for (j = 0; j < key_count; j++) {
            if (key_is(name, length, keys[j])) break;
        }
        if (j == key_count || (seen & (1u << j)) != 0) return 0;
        seen |= 1u << j;
    }
    /* The object has exactly key_count members and each distinct member was
     * matched to one allowed name, so every required name was seen. */
    return 1;
}

static int integer_value(const BJsonValue *value, int64_t *out) {
    return bj_kind(value) == BJ_INTEGER && bj_integer(value, out);
}

static int nonnegative_integer(const BJsonValue *value, uint64_t maximum,
    uint64_t *out) {
    int64_t integer;
    if (!integer_value(value, &integer) || integer < 0 ||
        (uint64_t)integer > maximum) return 0;
    *out = (uint64_t)integer;
    return 1;
}

static int fits_size_t(uint64_t value) {
    return (uint64_t)(size_t)value == value;
}

static int string_is(const BJsonValue *value, const char *expected) {
    const char *text;
    size_t length;
    return bj_string(value, &text, &length) && length == strlen(expected) &&
        memcmp(text, expected, length) == 0;
}

/* Hex accepted by the portable fixture is exact-length and lowercase. */
static int decode_hex(const BJsonValue *value, uint8_t *out, size_t bytes) {
    const char *text;
    size_t length, i;
    if (!bj_string(value, &text, &length) || bytes > SIZE_MAX / 2 ||
        length != bytes * 2) return 0;
    for (i = 0; i < bytes; i++) {
        unsigned high, low;
        char a = text[i * 2], b = text[i * 2 + 1];
        if (a >= '0' && a <= '9') high = (unsigned)(a - '0');
        else if (a >= 'a' && a <= 'f') high = (unsigned)(a - 'a') + 10u;
        else return 0;
        if (b >= '0' && b <= '9') low = (unsigned)(b - '0');
        else if (b >= 'a' && b <= 'f') low = (unsigned)(b - 'a') + 10u;
        else return 0;
        if (out != NULL) out[i] = (uint8_t)((high << 4) | low);
    }
    return 1;
}

static int parse_state(const BJsonValue *value, size_t blocks,
    size_t pages_per_block, size_t page_size_bytes, NandFuzzState *out) {
    static const char *const keys[] = {"pages", "erase_counts", "programmed"};
    const BJsonValue *pages, *counts, *markers;
    size_t b, p;
    if (!exact_keys(value, keys, 3)) return 0;
    pages = member(value, "pages");
    counts = member(value, "erase_counts");
    markers = member(value, "programmed");
    if (bj_kind(pages) != BJ_ARRAY || bj_kind(counts) != BJ_ARRAY ||
        bj_kind(markers) != BJ_ARRAY || bj_size(pages) != blocks ||
        bj_size(counts) != blocks || bj_size(markers) != blocks) return 0;
    for (b = 0; b < blocks; b++) {
        uint64_t count;
        const BJsonValue *block_pages = bj_at(pages, b);
        const BJsonValue *block_markers = bj_at(markers, b);
        if (bj_kind(block_pages) != BJ_ARRAY ||
            bj_kind(block_markers) != BJ_ARRAY ||
            bj_size(block_pages) != pages_per_block ||
            bj_size(block_markers) != pages_per_block ||
            !nonnegative_integer(bj_at(counts, b), (uint64_t)NAND_SAFE_INTEGER,
                &count)) return 0;
        if (out != NULL) out->erase_counts[b] = count;
        for (p = 0; p < pages_per_block; p++) {
            int flag;
            uint8_t *page = out == NULL ? NULL : out->pages[b][p];
            if (!decode_hex(bj_at(block_pages, p), page, page_size_bytes) ||
                !bj_boolean(bj_at(block_markers, p), &flag)) return 0;
            if (out != NULL) out->programmed[b][p] = (uint8_t)(flag != 0);
        }
    }
    return 1;
}

static int states_equal(const NandFuzzState *left, const NandFuzzState *right,
    size_t blocks, size_t pages_per_block, size_t page_size_bytes) {
    size_t b, p;
    for (b = 0; b < blocks; b++) {
        if (left->erase_counts[b] != right->erase_counts[b]) return 0;
        for (p = 0; p < pages_per_block; p++) {
            if (left->programmed[b][p] != right->programmed[b][p] ||
                memcmp(left->pages[b][p], right->pages[b][p],
                    page_size_bytes) != 0) return 0;
        }
    }
    return 1;
}

int nand_fuzz_states_equal(const NandFuzzState *left,
    const NandFuzzState *right, size_t blocks, size_t pages_per_block,
    size_t page_size_bytes) {
    if (left == NULL || right == NULL || blocks > NAND_FUZZ_BLOCKS ||
        pages_per_block > NAND_FUZZ_PAGES ||
        page_size_bytes > NAND_FUZZ_PAGE_BYTES) return 0;
    return states_equal(left, right, blocks, pages_per_block, page_size_bytes);
}

static NandFlashStatus model_read(const NandFuzzState *state, size_t blocks,
    size_t pages_per_block, size_t block, size_t page, uint8_t *out,
    size_t length, size_t page_size_bytes) {
    if (block >= blocks) return NAND_FLASH_ADDRESS_BLOCK_ERROR;
    if (page >= pages_per_block) return NAND_FLASH_ADDRESS_PAGE_ERROR;
    if (out == NULL) return NAND_FLASH_PAYLOAD_ERROR;
    if (length != page_size_bytes) return NAND_FLASH_PAYLOAD_SIZE_ERROR;
    memcpy(out, state->pages[block][page], page_size_bytes);
    return NAND_FLASH_OK;
}

static const char *model_message(NandFlashStatus status) {
    switch (status) {
    case NAND_FLASH_OK: return "success";
    case NAND_FLASH_INVALID_ARGUMENT: return "invalid API argument";
    case NAND_FLASH_GEOMETRY_ERROR: return "geometry values are outside supported bounds";
    case NAND_FLASH_ADDRESS_BLOCK_ERROR: return "block index out of range";
    case NAND_FLASH_ADDRESS_PAGE_ERROR: return "page index out of range";
    case NAND_FLASH_PAYLOAD_ERROR: return "page data must be bytes";
    case NAND_FLASH_PAYLOAD_SIZE_ERROR: return "page data must match page_size_bytes";
    case NAND_FLASH_PROGRAM_ERROR: return "page has already been programmed since erase";
    case NAND_FLASH_ERASE_LIMIT_ERROR: return "block erase limit reached";
    case NAND_FLASH_ALLOCATION_ERROR: return "allocation failed";
    }
    return "unknown NAND status";
}

static int parse_expected_status(const BJsonValue *operation,
    NandFlashStatus *status) {
    static const char *const keys[] = {"type", "message"};
    const BJsonValue *error = member(operation, "error");
    if (error == NULL) return 0;
    if (!exact_keys(error, keys, 2)) return 0;
    if (string_is(member(error, "type"), "ProgramError"))
        *status = NAND_FLASH_PROGRAM_ERROR;
    else if (string_is(member(error, "type"), "EraseLimitError"))
        *status = NAND_FLASH_ERASE_LIMIT_ERROR;
    else return 0;
    return string_is(member(error, "message"), model_message(*status));
}

static int parse_geometry(const BJsonValue *value, size_t *blocks,
    size_t *pages_per_block, size_t *page_size_bytes, int *has_limit,
    uint64_t *max_erase_cycles) {
    static const char *const keys[] = {"block_count", "pages_per_block",
        "page_size_bytes", "max_erase_cycles"};
    uint64_t block_count, page_count, page_bytes;
    size_t all_pages;
    const BJsonValue *limit;
    if (!exact_keys(value, keys, 4) ||
        !nonnegative_integer(member(value, "block_count"), 1024,
            &block_count) || block_count == 0 ||
        !nonnegative_integer(member(value, "pages_per_block"), 256,
            &page_count) || page_count == 0 ||
        !nonnegative_integer(member(value, "page_size_bytes"), 65536,
            &page_bytes) || page_bytes == 0 ||
        !fits_size_t(block_count) || !fits_size_t(page_count) ||
        !fits_size_t(page_bytes) ||
        (size_t)block_count > SIZE_MAX / (size_t)page_count) return 0;
    all_pages = (size_t)block_count * (size_t)page_count;
    if (all_pages > NAND_DB01_MAX_STORAGE / (size_t)page_bytes ||
        all_pages * (size_t)page_bytes > NAND_DB01_MAX_STORAGE) return 0;
    limit = member(value, "max_erase_cycles");
    if (bj_kind(limit) == BJ_NULL) {
        *has_limit = 0;
        *max_erase_cycles = 0;
    } else if (nonnegative_integer(limit, (uint64_t)NAND_SAFE_INTEGER,
                   max_erase_cycles)) {
        *has_limit = 1;
    } else return 0;
    *blocks = (size_t)block_count;
    *pages_per_block = (size_t)page_count;
    *page_size_bytes = (size_t)page_bytes;
    return 1;
}

static int decode_operation(const BJsonValue *value, size_t blocks,
    size_t pages_per_block, size_t page_size_bytes, NandFuzzOperation *out) {
    static const char *const read_ok[] = {"op", "block", "page", "result"};
    static const char *const read_error[] = {"op", "block", "page", "error"};
    static const char *const program_ok[] = {"op", "block", "page", "data", "result"};
    static const char *const program_error[] = {"op", "block", "page", "data", "error"};
    static const char *const erase_ok[] = {"op", "block", "result"};
    static const char *const erase_error[] = {"op", "block", "error"};
    uint64_t block, page = 0;
    int is_error = member(value, "error") != NULL;
    NandFlashStatus status = NAND_FLASH_OK;
    if (bj_kind(value) != BJ_OBJECT ||
        !nonnegative_integer(member(value, "block"), 1023, &block) ||
        block >= blocks || !fits_size_t(block)) return 0;
    if (string_is(member(value, "op"), "read")) {
        if (!nonnegative_integer(member(value, "page"), 255, &page) ||
            page >= pages_per_block || !fits_size_t(page) ||
            !(is_error ? exact_keys(value, read_error, 4) :
                         exact_keys(value, read_ok, 4))) return 0;
        if (is_error || !decode_hex(member(value, "result"),
                out == NULL ? NULL : out->expected_result, page_size_bytes)) return 0;
        if (out != NULL) out->kind = NAND_FUZZ_READ;
    } else if (string_is(member(value, "op"), "program")) {
        if (!nonnegative_integer(member(value, "page"), 255, &page) ||
            page >= pages_per_block || !fits_size_t(page) ||
            !(is_error ? exact_keys(value, program_error, 5) :
                         exact_keys(value, program_ok, 5)) ||
            !decode_hex(member(value, "data"),
                out == NULL ? NULL : out->data, page_size_bytes)) return 0;
        if (is_error) {
            if (!parse_expected_status(value, &status) ||
                status != NAND_FLASH_PROGRAM_ERROR) return 0;
        } else if (bj_kind(member(value, "result")) != BJ_NULL) return 0;
        if (out != NULL) out->kind = NAND_FUZZ_PROGRAM;
    } else if (string_is(member(value, "op"), "erase_block")) {
        if (!(is_error ? exact_keys(value, erase_error, 3) :
                         exact_keys(value, erase_ok, 3))) return 0;
        if (is_error) {
            if (!parse_expected_status(value, &status) ||
                status != NAND_FLASH_ERASE_LIMIT_ERROR) return 0;
        } else if (bj_kind(member(value, "result")) != BJ_NULL) return 0;
        if (out != NULL) out->kind = NAND_FUZZ_ERASE;
    } else return 0;
    if (out != NULL) {
        out->block = (size_t)block;
        out->page = (size_t)page;
        out->expected_status = (int)status;
    } else if (is_error && !parse_expected_status(value, &status)) return 0;
    return 1;
}

static int erased_state(const NandFuzzState *state, size_t blocks,
    size_t pages_per_block, size_t page_size_bytes) {
    size_t b, p, i;
    for (b = 0; b < blocks; b++) {
        if (state->erase_counts[b] != 0) return 0;
        for (p = 0; p < pages_per_block; p++) {
            if (state->programmed[b][p] != 0) return 0;
            for (i = 0; i < page_size_bytes; i++) {
                if (state->pages[b][p][i] != 0xFFu) return 0;
            }
        }
    }
    return 1;
}

static int model_operation(NandFuzzState *state, NandFuzzOperation *operation,
    int has_limit, uint64_t max_erase_cycles, size_t pages_per_block,
    size_t page_size_bytes) {
    size_t b = operation->block, p = operation->page;
    NandFlashStatus expected = NAND_FLASH_OK;
    if (operation->kind == NAND_FUZZ_READ) {
        if (memcmp(operation->expected_result, state->pages[b][p],
                page_size_bytes) != 0) return 0;
    } else if (operation->kind == NAND_FUZZ_PROGRAM) {
        if (state->programmed[b][p]) expected = NAND_FLASH_PROGRAM_ERROR;
        else {
            memcpy(state->pages[b][p], operation->data, page_size_bytes);
            state->programmed[b][p] = 1;
        }
    } else {
        if ((has_limit && state->erase_counts[b] >= max_erase_cycles) ||
            state->erase_counts[b] >= (uint64_t)NAND_SAFE_INTEGER) {
            expected = NAND_FLASH_ERASE_LIMIT_ERROR;
        } else {
            size_t page, i;
            state->erase_counts[b]++;
            for (page = 0; page < pages_per_block; page++) {
                state->programmed[b][page] = 0;
                for (i = 0; i < page_size_bytes; i++)
                    state->pages[b][page][i] = 0xFFu;
            }
        }
    }
    if ((NandFlashStatus)operation->expected_status != expected) return 0;
    operation->expected_status = (int)expected;
    return 1;
}

NandFixtureDecodeResult nand_fixture_decode(const uint8_t *bytes, size_t length,
    NandFuzzTrace *out) {
    static const char *const root_keys[] = {"format", "description", "geometry",
        "initial_state", "operations", "states_after_operations"};
    BJsonOptions options = bj_options();
    BJsonDoc *document = NULL;
    BJsonError error;
    const BJsonValue *root, *operations, *states;
    NandFuzzTrace candidate;
    NandFuzzState model;
    size_t i;
    int in_domain;
    if (out == NULL || bytes == NULL || length == 0 ||
        length > NAND_FUZZ_MAX_INPUT_BYTES) return NAND_FIXTURE_REJECTED;
    memset(&candidate, 0, sizeof(candidate));
    options.max_bytes = NAND_FUZZ_MAX_INPUT_BYTES;
    options.max_depth = 64;
    options.max_nodes = 16384;
    options.max_alloc_bytes = 8u * 1024u * 1024u;
    if (bj_parse_with_options((const char *)bytes, length, &options,
            &document, &error) != BJ_OK || document == NULL) {
        bj_doc_free(&document);
        return NAND_FIXTURE_REJECTED;
    }
    root = bj_root(document);
    if (!exact_keys(root, root_keys, 6) ||
        !string_is(member(root, "format"), "coding-adventures-nand-flash-v1") ||
        bj_kind(member(root, "description")) != BJ_STRING ||
        !parse_geometry(member(root, "geometry"), &candidate.blocks,
            &candidate.pages_per_block, &candidate.page_size_bytes,
            &candidate.has_erase_limit, &candidate.max_erase_cycles)) {
        bj_doc_free(&document);
        return NAND_FIXTURE_REJECTED;
    }
    operations = member(root, "operations");
    states = member(root, "states_after_operations");
    if (bj_kind(operations) != BJ_ARRAY || bj_kind(states) != BJ_ARRAY ||
        bj_size(operations) == 0 || bj_size(states) != bj_size(operations)) {
        bj_doc_free(&document);
        return NAND_FIXTURE_REJECTED;
    }
    in_domain = candidate.blocks <= NAND_FUZZ_BLOCKS &&
        candidate.pages_per_block <= NAND_FUZZ_PAGES &&
        candidate.page_size_bytes <= NAND_FUZZ_PAGE_BYTES &&
        bj_size(operations) <= NAND_FUZZ_MAX_OPERATIONS;
    if (!parse_state(member(root, "initial_state"), candidate.blocks,
            candidate.pages_per_block, candidate.page_size_bytes,
            in_domain ? &candidate.initial : NULL)) {
        bj_doc_free(&document);
        return NAND_FIXTURE_REJECTED;
    }
    if (in_domain && !erased_state(&candidate.initial, candidate.blocks,
            candidate.pages_per_block, candidate.page_size_bytes)) {
        bj_doc_free(&document);
        return NAND_FIXTURE_REJECTED;
    }
    for (i = 0; i < bj_size(operations); i++) {
        NandFuzzOperation *operation = in_domain ? &candidate.operations[i] : NULL;
        NandFuzzState *after = in_domain ? &candidate.after[i] : NULL;
        if (!decode_operation(bj_at(operations, i), candidate.blocks,
                candidate.pages_per_block, candidate.page_size_bytes,
                operation) ||
            !parse_state(bj_at(states, i), candidate.blocks,
                candidate.pages_per_block, candidate.page_size_bytes, after)) {
            bj_doc_free(&document);
            return NAND_FIXTURE_REJECTED;
        }
        if (in_domain && i == 0) model = candidate.initial;
        if (in_domain && (!model_operation(&model, operation,
                candidate.has_erase_limit, candidate.max_erase_cycles,
                candidate.pages_per_block,
                candidate.page_size_bytes) ||
                !states_equal(&model, after, candidate.blocks,
                    candidate.pages_per_block, candidate.page_size_bytes))) {
            bj_doc_free(&document);
            return NAND_FIXTURE_REJECTED;
        }
    }
    if (!in_domain) {
        bj_doc_free(&document);
        return NAND_FIXTURE_OUTSIDE_DOMAIN;
    }
    candidate.operation_count = bj_size(operations);
    *out = candidate;
    bj_doc_free(&document);
    return NAND_FIXTURE_ACCEPTED;
}

static int runtime_state_matches(const NandFlash *device,
    const NandFuzzState *expected, size_t blocks, size_t pages_per_block,
    size_t page_size_bytes) {
    size_t b, p;
    for (b = 0; b < blocks; b++) {
        uint64_t count = UINT64_MAX;
        if (nand_flash_erase_count(device, b, &count) != NAND_FLASH_OK ||
            count != expected->erase_counts[b]) return 0;
        for (p = 0; p < pages_per_block; p++) {
            uint8_t actual[NAND_FUZZ_PAGE_BYTES];
            bool marker = !expected->programmed[b][p];
            if (nand_flash_read(device, b, p, actual, page_size_bytes) !=
                    NAND_FLASH_OK ||
                memcmp(actual, expected->pages[b][p], page_size_bytes) != 0 ||
                nand_flash_is_programmed(device, b, p, &marker) !=
                    NAND_FLASH_OK ||
                marker != (expected->programmed[b][p] != 0)) return 0;
        }
    }
    return 1;
}

NandReplayResult nand_fixture_replay(const NandFuzzTrace *trace,
    size_t *mismatch_operation) {
    NandFlash *device = NULL;
    size_t i;
    NandFlashStatus status;
    if (trace == NULL || trace->operation_count == 0 ||
        trace->operation_count > NAND_FUZZ_MAX_OPERATIONS) {
        return NAND_REPLAY_RUNTIME_MISMATCH;
    }
    status = nand_flash_create(trace->blocks, trace->pages_per_block,
        trace->page_size_bytes, trace->has_erase_limit != 0,
        trace->max_erase_cycles, &device);
    if (status != NAND_FLASH_OK || device == NULL) {
        nand_flash_destroy(device);
        return NAND_REPLAY_RUNTIME_MISMATCH;
    }
    if (!runtime_state_matches(device, &trace->initial, trace->blocks,
            trace->pages_per_block, trace->page_size_bytes)) {
        nand_flash_destroy(device);
        return NAND_REPLAY_RUNTIME_MISMATCH;
    }
    for (i = 0; i < trace->operation_count; i++) {
        const NandFuzzOperation *operation = &trace->operations[i];
        NandFlashStatus expected = (NandFlashStatus)operation->expected_status;
        const char *message;
        if (operation->kind == NAND_FUZZ_READ) {
            uint8_t output[NAND_FUZZ_PAGE_BYTES];
            memset(output, 0xA5, sizeof(output));
            status = nand_flash_read(device, operation->block, operation->page,
                output, trace->page_size_bytes);
            if (status == NAND_FLASH_OK &&
                memcmp(output, operation->expected_result,
                    trace->page_size_bytes) != 0) goto mismatch;
            if (status != NAND_FLASH_OK) {
                size_t byte;
                for (byte = 0; byte < trace->page_size_bytes; byte++)
                    if (output[byte] != 0xA5u) goto mismatch;
            }
        } else if (operation->kind == NAND_FUZZ_PROGRAM) {
            status = nand_flash_program(device, operation->block,
                operation->page, operation->data, trace->page_size_bytes);
        } else if (operation->kind == NAND_FUZZ_ERASE) {
            status = nand_flash_erase_block(device, operation->block);
        } else goto mismatch;
        if (status != expected) goto mismatch;
        message = nand_flash_status_message(status);
        if (message == NULL || strcmp(message, model_message(expected)) != 0 ||
            !runtime_state_matches(device, &trace->after[i], trace->blocks,
                trace->pages_per_block, trace->page_size_bytes)) goto mismatch;
        continue;
mismatch:
        if (mismatch_operation != NULL) *mismatch_operation = i;
        nand_flash_destroy(device);
        return NAND_REPLAY_RUNTIME_MISMATCH;
    }
    nand_flash_destroy(device);
    return NAND_REPLAY_MATCH;
}

static void initialize_model(NandFuzzState *state) {
    size_t b, p, i;
    memset(state, 0, sizeof(*state));
    for (b = 0; b < NAND_FUZZ_BLOCKS; b++)
        for (p = 0; p < NAND_FUZZ_PAGES; p++)
            for (i = 0; i < NAND_FUZZ_PAGE_BYTES; i++)
                state->pages[b][p][i] = 0xFFu;
}

static NandFlashStatus history_expected(NandFuzzState *state, uint8_t opcode,
    size_t block, size_t page, const uint8_t *payload, size_t length,
    int null_output, int has_erase_limit, uint64_t max_erase_cycles,
    uint8_t *read_output, uint64_t *count_output, bool *marker_output) {
    NandFlashStatus status;
    size_t i;
    if (opcode == 0) {
        status = model_read(state, NAND_FUZZ_BLOCKS, NAND_FUZZ_PAGES,
            block, page, null_output ? NULL : read_output, length,
            NAND_FUZZ_PAGE_BYTES);
    } else if (opcode == 1) {
        if (block >= NAND_FUZZ_BLOCKS) return NAND_FLASH_ADDRESS_BLOCK_ERROR;
        if (page >= NAND_FUZZ_PAGES) return NAND_FLASH_ADDRESS_PAGE_ERROR;
        if (payload == NULL) return NAND_FLASH_PAYLOAD_ERROR;
        if (length != NAND_FUZZ_PAGE_BYTES) return NAND_FLASH_PAYLOAD_SIZE_ERROR;
        if (state->programmed[block][page]) return NAND_FLASH_PROGRAM_ERROR;
        memcpy(state->pages[block][page], payload, NAND_FUZZ_PAGE_BYTES);
        state->programmed[block][page] = 1;
        return NAND_FLASH_OK;
    } else if (opcode == 2) {
        if (block >= NAND_FUZZ_BLOCKS) return NAND_FLASH_ADDRESS_BLOCK_ERROR;
        if ((has_erase_limit && state->erase_counts[block] >= max_erase_cycles) ||
            state->erase_counts[block] >= (uint64_t)NAND_SAFE_INTEGER)
            return NAND_FLASH_ERASE_LIMIT_ERROR;
        state->erase_counts[block]++;
        for (i = 0; i < NAND_FUZZ_PAGES; i++) {
            memset(state->pages[block][i], 0xFF, NAND_FUZZ_PAGE_BYTES);
            state->programmed[block][i] = 0;
        }
        return NAND_FLASH_OK;
    } else if (opcode == 3) {
        if (null_output) return NAND_FLASH_INVALID_ARGUMENT;
        if (block >= NAND_FUZZ_BLOCKS) return NAND_FLASH_ADDRESS_BLOCK_ERROR;
        if (count_output != NULL) *count_output = state->erase_counts[block];
        return NAND_FLASH_OK;
    } else {
        if (null_output) return NAND_FLASH_INVALID_ARGUMENT;
        if (block >= NAND_FUZZ_BLOCKS) return NAND_FLASH_ADDRESS_BLOCK_ERROR;
        if (page >= NAND_FUZZ_PAGES) return NAND_FLASH_ADDRESS_PAGE_ERROR;
        if (marker_output != NULL)
            *marker_output = state->programmed[block][page] != 0;
        return NAND_FLASH_OK;
    }
    return status;
}

static int history_state_matches(const NandFlash *device,
    const NandFuzzState *state) {
    return runtime_state_matches(device, state, NAND_FUZZ_BLOCKS,
        NAND_FUZZ_PAGES, NAND_FUZZ_PAGE_BYTES);
}

NandHistoryResult nand_history_replay(const uint8_t *bytes, size_t length,
    size_t *mismatch_operation) {
    NandFuzzState model;
    NandFlash *device = NULL;
    size_t count, i;
    if (bytes == NULL || length < 2 || length > 2 +
            NAND_FUZZ_MAX_OPERATIONS * HISTORY_RECORD_BYTES ||
        bytes[0] != HISTORY_PREFIX || bytes[1] > NAND_FUZZ_MAX_OPERATIONS)
        return NAND_HISTORY_REJECTED;
    count = bytes[1];
    if (length != 2 + count * HISTORY_RECORD_BYTES) return NAND_HISTORY_REJECTED;
    for (i = 0; i < count; i++) {
        const uint8_t *record = bytes + 2 + i * HISTORY_RECORD_BYTES;
        if (record[0] > 4 || (record[3] & 0xFCu) != 0)
            return NAND_HISTORY_REJECTED;
    }
    if (nand_flash_create(NAND_FUZZ_BLOCKS, NAND_FUZZ_PAGES,
            NAND_FUZZ_PAGE_BYTES, true, 2, &device) != NAND_FLASH_OK ||
        device == NULL) {
        nand_flash_destroy(device);
        return NAND_HISTORY_RUNTIME_MISMATCH;
    }
    initialize_model(&model);
    if (!history_state_matches(device, &model)) goto history_mismatch_before;
    for (i = 0; i < count; i++) {
        const uint8_t *record = bytes + 2 + i * HISTORY_RECORD_BYTES;
        uint8_t opcode = record[0], block = record[1], page = record[2];
        uint8_t flags = record[3], payload[256], output[256];
        uint8_t expected_output[256];
        size_t payload_length = record[4], byte;
        int null_payload = (flags & 1u) != 0;
        int null_output = (flags & 2u) != 0;
        uint64_t count_output = UINT64_C(0xD00DFEEDCAFEBABE);
        uint64_t expected_count_output = UINT64_C(0xD00DFEEDCAFEBABE);
        bool marker_output = true;
        bool expected_marker_output = true;
        NandFlashStatus expected, got;
        const uint8_t *source = null_payload ? NULL : payload;
        uint8_t *destination = null_output ? NULL : output;
        uint64_t *count_pointer = null_output ? NULL : &count_output;
        bool *marker_pointer = null_output ? NULL : &marker_output;
        memset(payload, 0xC3, sizeof(payload));
        for (byte = 0; byte < sizeof(payload); byte++)
            payload[byte] = (uint8_t)(record[5 + (byte % 3)] ^
                (uint8_t)(byte * 29u + (byte >> 2)));
        memset(output, 0xA5, sizeof(output));
        memset(expected_output, 0xA5, sizeof(expected_output));
        if (opcode == 0) {
            expected = history_expected(&model, opcode, block, page, NULL,
                payload_length, null_output, 1, 2,
                expected_output, NULL, NULL);
            got = nand_flash_read(device, block, page, destination,
                payload_length);
        } else if (opcode == 1) {
            expected = history_expected(&model, opcode, block, page, source,
                payload_length, 0, 1, 2, NULL, NULL, NULL);
            got = nand_flash_program(device, block, page, source,
                payload_length);
        } else if (opcode == 2) {
            expected = history_expected(&model, opcode, block, page, NULL,
                payload_length, 0, 1, 2, NULL, NULL, NULL);
            got = nand_flash_erase_block(device, block);
        } else if (opcode == 3) {
            expected = history_expected(&model, opcode, block, page, NULL,
                payload_length, null_output, 1, 2, NULL,
                &expected_count_output, NULL);
            got = nand_flash_erase_count(device, block, count_pointer);
        } else {
            expected = history_expected(&model, opcode, block, page, NULL,
                payload_length, null_output, 1, 2, NULL, NULL,
                &expected_marker_output);
            got = nand_flash_is_programmed(device, block, page, marker_pointer);
        }
        if (got != expected || nand_flash_status_message(got) == NULL ||
            strcmp(nand_flash_status_message(got), model_message(expected)) != 0)
            goto history_mismatch;
        if (opcode == 0) {
            if (!null_output && memcmp(output, expected_output,
                    sizeof(output)) != 0) goto history_mismatch;
        } else if (opcode == 3) {
            if (expected == NAND_FLASH_OK &&
                count_output != expected_count_output)
                goto history_mismatch;
            if (expected != NAND_FLASH_OK && !null_output &&
                count_output != UINT64_C(0xD00DFEEDCAFEBABE))
                goto history_mismatch;
        } else if (opcode == 4) {
            if (expected == NAND_FLASH_OK &&
                marker_output != expected_marker_output) goto history_mismatch;
            if (expected != NAND_FLASH_OK && !null_output && marker_output != 1)
                goto history_mismatch;
        }
        if (!history_state_matches(device, &model)) goto history_mismatch;
        continue;
history_mismatch:
        if (mismatch_operation != NULL) *mismatch_operation = i;
        nand_flash_destroy(device);
        return NAND_HISTORY_RUNTIME_MISMATCH;
    }
    nand_flash_destroy(device);
    return NAND_HISTORY_MATCH;

history_mismatch_before:
    if (mismatch_operation != NULL) *mismatch_operation = 0;
    nand_flash_destroy(device);
    return NAND_HISTORY_RUNTIME_MISMATCH;
}
