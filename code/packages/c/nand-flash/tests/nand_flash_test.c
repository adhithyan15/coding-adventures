#include "nand_flash.h"
#include "nand_flash_fuzz_support.h"
#include "iso_test.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static void test_shared_trace(void) {
    FILE *file = fopen("../../../specs/fixtures/database-media-v1/nand-flash.json", "rb");
    char *bytes;
    long length;
    size_t mismatch_operation = SIZE_MAX;
    NandFuzzTrace trace;
    NandFixtureDecodeResult decoded;
    NandReplayResult replay;
    ISO_CHECK_MSG(file != NULL, "open canonical NAND trace");
    if (file == NULL) return;
    if (fseek(file, 0, SEEK_END) != 0 ||
        (length = ftell(file)) <= 0 ||
        length > (long)NAND_FUZZ_MAX_INPUT_BYTES ||
        fseek(file, 0, SEEK_SET) != 0) {
        fclose(file);
        ISO_CHECK_MSG(0, "canonical NAND trace fits the 64 KiB adapter cap");
        return;
    }
    bytes = (char *)malloc((size_t)length + 1);
    ISO_CHECK(bytes != NULL);
    if (bytes == NULL) { fclose(file); return; }
    if (fread(bytes, 1, (size_t)length, file) != (size_t)length) {
        fclose(file); free(bytes); ISO_CHECK(0); return;
    }
    fclose(file);
    bytes[length] = '\0';
    decoded = nand_fixture_decode((const uint8_t *)bytes, (size_t)length, &trace);
    ISO_CHECK(decoded == NAND_FIXTURE_ACCEPTED);
    if (decoded != NAND_FIXTURE_ACCEPTED) { free(bytes); return; }
    replay = nand_fixture_replay(&trace, &mismatch_operation);
    ISO_CHECK(replay == NAND_REPLAY_MATCH);
    /* NAND erase isolation operation 11 after-state is the E02 witness. */
    ISO_CHECK(mismatch_operation != 11);
    free(bytes);
}

static char *fixture_replace(const char *source, const char *old_text,
    const char *new_text) {
    const char *found = strstr(source, old_text);
    size_t source_length = strlen(source), old_length = strlen(old_text);
    size_t replacement_length = strlen(new_text), prefix;
    char *copy;
    if (found == NULL) return NULL;
    prefix = (size_t)(found - source);
    if (replacement_length >= SIZE_MAX - (source_length - old_length)) return NULL;
    copy = (char *)malloc(source_length - old_length + replacement_length + 1);
    if (copy == NULL) return NULL;
    memcpy(copy, source, prefix);
    memcpy(copy + prefix, new_text, replacement_length);
    memcpy(copy + prefix + replacement_length, found + old_length,
        source_length - prefix - old_length);
    copy[source_length - old_length + replacement_length] = '\0';
    return copy;
}

static void fixture_rejected(const char *text) {
    NandFuzzTrace trace;
    ISO_CHECK(nand_fixture_decode((const uint8_t *)text, strlen(text), &trace) ==
        NAND_FIXTURE_REJECTED);
}

static void test_fixture_decoder_classes(void) {
    static const char outside_domain[] =
        "{\"format\":\"coding-adventures-nand-flash-v1\",\"description\":\"outside\","
        "\"geometry\":{\"block_count\":1,\"pages_per_block\":1,\"page_size_bytes\":9,\"max_erase_cycles\":0},"
        "\"initial_state\":{\"pages\":[[\"ffffffffffffffffff\"]],\"erase_counts\":[0],\"programmed\":[[false]]},"
        "\"operations\":[{\"op\":\"read\",\"block\":0,\"page\":0,\"result\":\"ffffffffffffffffff\"}],"
        "\"states_after_operations\":[{\"pages\":[[\"ffffffffffffffffff\"]],\"erase_counts\":[0],\"programmed\":[[false]]}]}";
    FILE *file = fopen("../../../specs/fixtures/database-media-v1/nand-flash.json", "rb");
    char *text, *changed;
    long file_length;
    NandFuzzTrace trace, altered;
    size_t mismatch = SIZE_MAX;
    NandFixtureDecodeResult decoded;
    ISO_CHECK(file != NULL);
    if (file == NULL) return;
    if (fseek(file, 0, SEEK_END) != 0) { fclose(file); ISO_CHECK(0); return; }
    file_length = ftell(file);
    ISO_CHECK(file_length > 0 && file_length <= (long)NAND_FUZZ_MAX_INPUT_BYTES);
    if (file_length <= 0 || file_length > (long)NAND_FUZZ_MAX_INPUT_BYTES) {
        fclose(file); return;
    }
    if (fseek(file, 0, SEEK_SET) != 0) { fclose(file); ISO_CHECK(0); return; }
    text = (char *)malloc((size_t)file_length + 1);
    ISO_CHECK(text != NULL);
    if (text == NULL) { fclose(file); return; }
    if (fread(text, 1, (size_t)file_length, file) != (size_t)file_length) {
        fclose(file); free(text); ISO_CHECK(0); return;
    }
    fclose(file); text[file_length] = '\0';
    decoded = nand_fixture_decode((const uint8_t *)text, (size_t)file_length,
        &trace);
    ISO_CHECK(decoded == NAND_FIXTURE_ACCEPTED);
    if (decoded != NAND_FIXTURE_ACCEPTED) { free(text); return; }
    ISO_CHECK(nand_fixture_decode((const uint8_t *)outside_domain,
        sizeof outside_domain - 1, &trace) == NAND_FIXTURE_OUTSIDE_DOMAIN);
    altered = trace;
    altered.operations[0].expected_result[0] ^= 1u;
    ISO_CHECK(nand_fixture_replay(&altered, &mismatch) ==
        NAND_REPLAY_RUNTIME_MISMATCH && mismatch == 0);

    changed = fixture_replace(text, "\"block_count\": 2",
        "\"block_count\": 2, \"block_count\": 2");
    ISO_CHECK(changed != NULL);
    if (changed != NULL) { fixture_rejected(changed); free(changed); }
    changed = fixture_replace(text,
        "\"format\": \"coding-adventures-nand-flash-v1\",",
        "\"format\": \"coding-adventures-nand-flash-v1\", \"extra\": 0,");
    ISO_CHECK(changed != NULL);
    if (changed != NULL) { fixture_rejected(changed); free(changed); }
    changed = fixture_replace(text, "\"block\": 0", "\"block\": true");
    ISO_CHECK(changed != NULL);
    if (changed != NULL) { fixture_rejected(changed); free(changed); }
    changed = fixture_replace(text, "\"op\": \"read\"",
        "\"op\": \"nope\"");
    ISO_CHECK(changed != NULL);
    if (changed != NULL) { fixture_rejected(changed); free(changed); }
    changed = fixture_replace(text, "\"result\": \"ffff\"",
        "\"result\": \"fffg\"");
    ISO_CHECK(changed != NULL);
    if (changed != NULL) { fixture_rejected(changed); free(changed); }
    changed = fixture_replace(text, "\"states_after_operations\"",
        "\"states_xfter_operations\"");
    ISO_CHECK(changed != NULL);
    if (changed != NULL) { fixture_rejected(changed); free(changed); }
    changed = (char *)malloc((size_t)file_length + 1);
    ISO_CHECK(changed != NULL);
    if (changed != NULL) {
        const char *description;
        memcpy(changed, text, (size_t)file_length + 1);
        description = strstr(changed, "\"description\": \"");
        ISO_CHECK(description != NULL);
        if (description != NULL) ((char *)description)[16] = (char)0xFF;
        fixture_rejected(changed);
        free(changed);
    }
    changed = (char *)malloc((size_t)file_length + 4);
    ISO_CHECK(changed != NULL);
    if (changed != NULL) {
        memcpy(changed, text, (size_t)file_length);
        memcpy(changed + file_length, " {}", 4);
        fixture_rejected(changed);
        free(changed);
    }
    changed = fixture_replace(text, "\"result\": \"ffff\"",
        "\"result\": \"fffe\"");
    ISO_CHECK(changed != NULL);
    if (changed != NULL) { fixture_rejected(changed); free(changed); }
    changed = fixture_replace(text, "\"ProgramError\"",
        "\"EraseLimitError\"");
    ISO_CHECK(changed != NULL);
    if (changed != NULL) { fixture_rejected(changed); free(changed); }
    changed = fixture_replace(text, "\"states_after_operations\"",
        "\"states_after_operations\" ");
    ISO_CHECK(changed != NULL);
    if (changed != NULL) {
        char *state = strstr(changed, "\"states_after_operations\"");
        char *hex = state == NULL ? NULL : strstr(state, "\"ffff\"");
        ISO_CHECK(hex != NULL);
        if (hex != NULL) hex[3] = 'e';
        fixture_rejected(changed);
        free(changed);
    }
    free(text);
}

static void history_record(uint8_t *history, size_t index, uint8_t opcode,
    uint8_t block, uint8_t page, uint8_t flags, uint8_t length,
    uint8_t a, uint8_t b, uint8_t c) {
    uint8_t *record = history + 2 + index * 8;
    record[0] = opcode; record[1] = block; record[2] = page;
    record[3] = flags; record[4] = length;
    record[5] = a; record[6] = b; record[7] = c;
}

static void test_binary_history_classes(void) {
    uint8_t history[2 + 8 * 8];
    size_t mismatch = SIZE_MAX;
    memset(history, 0, sizeof history);
    history[0] = 0x4E; history[1] = 5;
    history_record(history, 0, 1, 0, 0, 0, 8, 0x12, 0x34, 0x56);
    history_record(history, 1, 1, 0, 0, 0, 8, 0xFF, 0xEE, 0xDD);
    history_record(history, 2, 0, 0, 0, 0, 8, 0, 0, 0);
    history_record(history, 3, 2, 0, 0, 0, 0, 0, 0, 0);
    history_record(history, 4, 3, 0, 0, 0, 0, 0, 0, 0);
    ISO_CHECK(nand_history_replay(history, 2 + 5 * 8, &mismatch) == NAND_HISTORY_MATCH);

    history[1] = 4;
    history_record(history, 0, 1, 0, 0, 1, 8, 0, 0, 0);
    history_record(history, 1, 1, 0, 1, 0, 7, 0, 0, 0);
    history_record(history, 2, 0, 1, 0, 0, 8, 0, 0, 0);
    history_record(history, 3, 4, 0, 0, 2, 0, 0, 0, 0);
    ISO_CHECK(nand_history_replay(history, 2 + 4 * 8, &mismatch) == NAND_HISTORY_MATCH);

    /* Address/null collisions preserve the public API's stable precedence. */
    history_record(history, 0, 0, 255, 255, 2, 255, 0, 0, 0);
    history_record(history, 1, 1, 255, 255, 1, 7, 0, 0, 0);
    history_record(history, 2, 3, 255, 0, 2, 0, 0, 0, 0);
    history_record(history, 3, 4, 255, 255, 2, 0, 0, 0, 0);
    ISO_CHECK(nand_history_replay(history, 2 + 4 * 8, &mismatch) == NAND_HISTORY_MATCH);

    history[1] = 3;
    history_record(history, 0, 2, 1, 0, 0, 0, 0, 0, 0);
    history_record(history, 1, 2, 1, 1, 0, 0, 0, 0, 0);
    history_record(history, 2, 2, 1, 0, 0, 0, 0, 0, 0);
    ISO_CHECK(nand_history_replay(history, 2 + 3 * 8, &mismatch) == NAND_HISTORY_MATCH);
    ISO_CHECK(nand_history_replay(history, 2 + 3 * 8 - 1, &mismatch) == NAND_HISTORY_REJECTED);
    ISO_CHECK(nand_history_replay(history, 2 + 3 * 8 + 1, &mismatch) == NAND_HISTORY_REJECTED);
    history[1] = 65;
    ISO_CHECK(nand_history_replay(history, 2, &mismatch) == NAND_HISTORY_REJECTED);
    history[1] = 1; history[2] = 5;
    ISO_CHECK(nand_history_replay(history, 2 + 8, &mismatch) == NAND_HISTORY_REJECTED);
}

static void test_boundaries_and_copy(void) {
    NandFlash *f = NULL; uint8_t source[2] = {0x12, 0x34}, out[2] = {0, 0}; bool mark; uint64_t count;
    ISO_CHECK(nand_flash_create(0, 1, 1, false, 0, &f) == NAND_FLASH_GEOMETRY_ERROR && f == NULL);
    ISO_CHECK(nand_flash_create(1, 257, 1, false, 0, &f) == NAND_FLASH_GEOMETRY_ERROR && f == NULL);
    ISO_CHECK(nand_flash_create(1, 1, 65537, false, 0, &f) == NAND_FLASH_GEOMETRY_ERROR && f == NULL);
    ISO_CHECK(nand_flash_create(1024, 256, 65536, false, 0, &f) == NAND_FLASH_GEOMETRY_ERROR && f == NULL);
    ISO_CHECK(nand_flash_create(1, 1, 1, true, UINT64_C(9007199254740992), &f) == NAND_FLASH_GEOMETRY_ERROR && f == NULL);
    ISO_CHECK(nand_flash_create(SIZE_MAX, 1, 1, false, 0, &f) == NAND_FLASH_GEOMETRY_ERROR && f == NULL);
    ISO_CHECK(nand_flash_create(1, 1, 1, true, 0, &f) == NAND_FLASH_OK);
    ISO_CHECK(nand_flash_erase_block(f, 0) == NAND_FLASH_ERASE_LIMIT_ERROR);
    nand_flash_destroy(f); f = NULL;
    ISO_CHECK(nand_flash_create(1, 1, 2, false, 0, &f) == NAND_FLASH_OK);
    ISO_CHECK(nand_flash_block_count(NULL) == 0 && nand_flash_pages_per_block(NULL) == 0 && nand_flash_page_size_bytes(NULL) == 0);
    ISO_CHECK(nand_flash_block_count(f) == 1 && nand_flash_pages_per_block(f) == 1 && nand_flash_page_size_bytes(f) == 2);
    ISO_CHECK(nand_flash_erase_count(NULL, 0, &count) == NAND_FLASH_INVALID_ARGUMENT);
    ISO_CHECK(nand_flash_is_programmed(NULL, 0, 0, &mark) == NAND_FLASH_INVALID_ARGUMENT);
    ISO_CHECK(nand_flash_status_message((NandFlashStatus)999)[0] == 'u');
    ISO_CHECK(nand_flash_read(NULL, SIZE_MAX, SIZE_MAX, NULL, SIZE_MAX) == NAND_FLASH_INVALID_ARGUMENT);
    ISO_CHECK(nand_flash_read(f, 9, 0, NULL, 0) == NAND_FLASH_ADDRESS_BLOCK_ERROR);
    ISO_CHECK(nand_flash_read(f, 0, 9, NULL, 0) == NAND_FLASH_ADDRESS_PAGE_ERROR);
    ISO_CHECK(nand_flash_read(f, 0, 0, NULL, 0) == NAND_FLASH_PAYLOAD_ERROR);
    ISO_CHECK(nand_flash_read(f, 0, 0, out, 1) == NAND_FLASH_PAYLOAD_SIZE_ERROR && out[0] == 0);
    ISO_CHECK(nand_flash_program(f, 0, 0, source, 1) == NAND_FLASH_PAYLOAD_SIZE_ERROR);
    ISO_CHECK(nand_flash_program(f, 0, 0, source, 2) == NAND_FLASH_OK);
    source[0] = 0xFF;
    ISO_CHECK(nand_flash_read(f, 0, 0, out, 2) == NAND_FLASH_OK && out[0] == 0x12 && out[1] == 0x34);
    out[0] = 0;
    ISO_CHECK(nand_flash_is_programmed(f, 0, 0, &mark) == NAND_FLASH_OK && mark);
    ISO_CHECK(nand_flash_program(f, 0, 0, source, 2) == NAND_FLASH_PROGRAM_ERROR);
    ISO_CHECK(nand_flash_program(f, 0, 0, NULL, 0) == NAND_FLASH_PAYLOAD_ERROR);
    ISO_CHECK(nand_flash_program(f, 0, 0, source, 1) == NAND_FLASH_PAYLOAD_SIZE_ERROR);
    ISO_CHECK(nand_flash_is_programmed(f, 0, 0, NULL) == NAND_FLASH_INVALID_ARGUMENT);
    ISO_CHECK(nand_flash_erase_count(f, 9, &count) == NAND_FLASH_ADDRESS_BLOCK_ERROR);
    ISO_CHECK(nand_flash_erase_block(f, 0) == NAND_FLASH_OK);
    ISO_CHECK(nand_flash_erase_count(f, 0, &count) == NAND_FLASH_OK && count == 1);
    ISO_CHECK(nand_flash_is_programmed(f, 0, 0, &mark) == NAND_FLASH_OK && !mark);
    nand_flash_destroy(f);
}

static void test_exhaustive_short_histories(void) {
    unsigned sequence;
    /* Four command shapes over six steps enumerate 4096 histories. */
    for (sequence = 0; sequence < 4096; sequence++) {
        unsigned encoded = sequence, step; uint8_t byte = 255; bool marker = false;
        uint64_t erases = 0; NandFlash *f = NULL;
        ISO_CHECK(nand_flash_create(1, 1, 1, true, 1, &f) == NAND_FLASH_OK);
        if (f == NULL) continue;
        for (step = 0; step < 6; step++) {
            unsigned op = encoded & 3u; NandFlashStatus got, want;
            encoded >>= 2;
            if (op <= 1) {
                want = marker ? NAND_FLASH_PROGRAM_ERROR : NAND_FLASH_OK;
                got = nand_flash_program(f, 0, 0, (const uint8_t[]){(uint8_t)(op == 0 ? 255 : 0)}, 1);
                if (want == NAND_FLASH_OK) { marker = true; byte = (uint8_t)(op == 0 ? 255 : 0); }
            } else if (op == 2) {
                want = erases == 1 ? NAND_FLASH_ERASE_LIMIT_ERROR : NAND_FLASH_OK;
                got = nand_flash_erase_block(f, 0);
                if (want == NAND_FLASH_OK) { erases++; marker = false; byte = 255; }
            } else {
                want = NAND_FLASH_ADDRESS_PAGE_ERROR; got = nand_flash_program(f, 0, 1, &byte, 1);
            }
            ISO_CHECK(got == want);
            { uint8_t actual = 0; bool actual_marker; uint64_t actual_erases;
              ISO_CHECK(nand_flash_read(f, 0, 0, &actual, 1) == NAND_FLASH_OK && actual == byte);
              ISO_CHECK(nand_flash_is_programmed(f, 0, 0, &actual_marker) == NAND_FLASH_OK && actual_marker == marker);
              ISO_CHECK(nand_flash_erase_count(f, 0, &actual_erases) == NAND_FLASH_OK && actual_erases == erases); }
        }
        nand_flash_destroy(f);
    }
}

static void test_seeded_model_and_short_histories(void) {
    NandFlash *f = NULL; uint8_t model[2][2] = {{255,255},{255,255}}; bool markers[2][2] = {{false,false},{false,false}};
    uint64_t counts[2] = {0,0}; unsigned seed = 0xA51C3u; size_t step;
    ISO_CHECK(nand_flash_create(2, 2, 1, true, 2, &f) == NAND_FLASH_OK); if (f == NULL) return;
    /* LCG sequence is fixed; the oracle is a deliberately separate tiny model. */
    for (step = 0; step < 700; step++) {
        unsigned r; size_t b, p; uint8_t data; NandFlashStatus got, expected;
        seed = seed * 1664525u + 1013904223u; r = seed; b = (r >> 8) & 1u; p = (r >> 9) & 1u;
        if (r % 3u == 0) {
            expected = counts[b] >= 2 ? NAND_FLASH_ERASE_LIMIT_ERROR : NAND_FLASH_OK;
            got = nand_flash_erase_block(f, b);
            if (expected == NAND_FLASH_OK) { counts[b]++; model[b][0] = model[b][1] = 255; markers[b][0] = markers[b][1] = false; }
        } else if (r % 3u == 1) {
            expected = markers[b][p] ? NAND_FLASH_PROGRAM_ERROR : NAND_FLASH_OK;
            data = (uint8_t)r; got = nand_flash_program(f, b, p, &data, 1);
            if (expected == NAND_FLASH_OK) { model[b][p] = data; markers[b][p] = true; }
        } else {
            uint8_t actual = 0; expected = NAND_FLASH_OK; got = nand_flash_read(f, b, p, &actual, 1);
            ISO_CHECK(actual == model[b][p]);
        }
        ISO_CHECK(got == expected);
        { uint64_t actual_count; ISO_CHECK(nand_flash_erase_count(f, b, &actual_count) == NAND_FLASH_OK && actual_count == counts[b]); }
        for (b = 0; b < 2; b++) for (p = 0; p < 2; p++) {
            uint8_t actual; bool mark;
            ISO_CHECK(nand_flash_read(f, b, p, &actual, 1) == NAND_FLASH_OK && actual == model[b][p]);
            ISO_CHECK(nand_flash_is_programmed(f, b, p, &mark) == NAND_FLASH_OK && mark == markers[b][p]);
        }
    }
    nand_flash_destroy(f);
}

static void test_malformed_command_stream(void) {
    NandFlash *f = NULL; unsigned seed = 0xD81B29u; size_t i;
    uint8_t sentinel[2] = {0xA5, 0x5A}, page[2]; bool marker; uint64_t count;
    ISO_CHECK(nand_flash_create(1, 1, 2, false, 0, &f) == NAND_FLASH_OK);
    if (f == NULL) return;
    /* Deterministic fuzz of bad coordinates, missing buffers and false lengths.
     * Every command is refused and the valid page remains erased and untouched. */
    for (i = 0; i < 1000; i++) {
        unsigned r; size_t b, p; NandFlashStatus got;
        seed = seed * 1103515245u + 12345u; r = seed;
        b = (r & 1u) ? 1u : 0u; p = b == 0 ? 1u : ((r & 2u) ? 1u : 0u);
        if ((r & 4u) != 0) got = nand_flash_read(f, b, p, sentinel, (r & 8u) ? 1u : 2u);
        else got = nand_flash_program(f, b, p, (r & 16u) ? NULL : sentinel, (r & 32u) ? 1u : 2u);
        ISO_CHECK(got != NAND_FLASH_OK);
        ISO_CHECK(nand_flash_read(f, 0, 0, page, sizeof page) == NAND_FLASH_OK && page[0] == 255 && page[1] == 255);
        ISO_CHECK(nand_flash_is_programmed(f, 0, 0, &marker) == NAND_FLASH_OK && !marker);
        ISO_CHECK(nand_flash_erase_count(f, 0, &count) == NAND_FLASH_OK && count == 0);
    }
    nand_flash_destroy(f);
}

int main(void) {
    test_shared_trace(); test_boundaries_and_copy(); test_seeded_model_and_short_histories();
    test_exhaustive_short_histories(); test_malformed_command_stream();
    test_fixture_decoder_classes(); test_binary_history_classes();
    return ISO_TEST_RESULT();
}
