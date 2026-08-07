/*
 * board_vm_protocol_test.c — unit tests for the board VM wire protocol codec.
 *
 * Mirrors the Rust crate's `#[cfg(test)]` suite (golden vectors, CRC check
 * value, ULEB128 boundaries, frame/wire round-trips, error paths) and adds
 * defensive truncation / byte-flip fuzz sweeps over the decoders.
 */
#include "board_vm_protocol.h"
#include "iso_test.h"

#include <string.h>

/* Deterministic xorshift PRNG so the fuzz sweeps are reproducible. */
static uint32_t g_rng = 0x1234ABCDu;
static uint32_t rng_next(void) {
    uint32_t x = g_rng;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    g_rng = x;
    return x;
}

static void test_crc_check_vector(void) {
    /* Standard CRC-16/CCITT-FALSE check value for "123456789". */
    static const uint8_t msg[9] = { '1', '2', '3', '4', '5', '6', '7', '8', '9' };
    ISO_CHECK_EQ_UINT(bvm_crc16_ccitt_false(msg, sizeof msg), 0x29B1u);
}

static void test_uleb128_boundaries(void) {
    static const uint8_t expected[9] = {
        0x00, 0x7F, 0x80, 0x01, 0xFF, 0x7F, 0x80, 0x80, 0x01
    };
    uint8_t out[12];
    bvm_encoder_t enc = bvm_encoder_new(out, sizeof out);
    ISO_CHECK(bvm_encoder_write_uleb128(&enc, 0) == BVM_OK);
    ISO_CHECK(bvm_encoder_write_uleb128(&enc, 127) == BVM_OK);
    ISO_CHECK(bvm_encoder_write_uleb128(&enc, 128) == BVM_OK);
    ISO_CHECK(bvm_encoder_write_uleb128(&enc, 16383) == BVM_OK);
    ISO_CHECK(bvm_encoder_write_uleb128(&enc, 16384) == BVM_OK);
    ISO_CHECK_EQ_UINT(bvm_encoder_len(&enc), sizeof expected);
    ISO_CHECK_MEM_EQ(out, expected, sizeof expected);
}

static void test_hello_payload_golden(void) {
    bvm_hello_t hello;
    uint8_t payload[16];
    size_t len = 0;
    bvm_hello_t decoded;

    hello.min_version = 1;
    hello.max_version = 1;
    hello.host_name = "bvm";
    hello.host_name_len = 3;
    hello.host_nonce = 0x1234ABCDu;

    ISO_CHECK(bvm_encode_hello(&hello, payload, sizeof payload, &len) == BVM_OK);
    ISO_CHECK_EQ_UINT(len, sizeof BVM_GOLDEN_HELLO_PAYLOAD_BVM_V1);
    ISO_CHECK_MEM_EQ(payload, BVM_GOLDEN_HELLO_PAYLOAD_BVM_V1,
                     sizeof BVM_GOLDEN_HELLO_PAYLOAD_BVM_V1);

    ISO_CHECK(bvm_decode_hello(payload, len, &decoded) == BVM_OK);
    ISO_CHECK_EQ_UINT(decoded.min_version, 1u);
    ISO_CHECK_EQ_UINT(decoded.max_version, 1u);
    ISO_CHECK_EQ_UINT(decoded.host_name_len, 3u);
    ISO_CHECK(memcmp(decoded.host_name, "bvm", 3) == 0);
    ISO_CHECK_EQ_UINT(decoded.host_nonce, 0x1234ABCDu);
}

static void test_raw_frame_with_crc(void) {
    static const uint8_t payload[10] = {
        0x01, 0x01, 0x03, 'b', 'v', 'm', 0xCD, 0xAB, 0x34, 0x12
    };
    bvm_frame_t frame;
    uint8_t out[32];
    size_t len = 0;
    bvm_frame_t decoded;

    frame.flags = BVM_FLAG_RESPONSE_REQUIRED;
    frame.message_type = BVM_MSG_HELLO;
    frame.request_id = 0x1234u;
    frame.payload = payload;
    frame.payload_len = sizeof payload;

    ISO_CHECK(bvm_encode_frame(&frame, out, sizeof out, &len) == BVM_OK);
    ISO_CHECK_EQ_UINT(len, sizeof BVM_GOLDEN_HELLO_RAW_FRAME_BVM_V1);
    ISO_CHECK_MEM_EQ(out, BVM_GOLDEN_HELLO_RAW_FRAME_BVM_V1,
                     sizeof BVM_GOLDEN_HELLO_RAW_FRAME_BVM_V1);

    ISO_CHECK(bvm_decode_frame(out, len, &decoded) == BVM_OK);
    ISO_CHECK_EQ_UINT(decoded.flags, BVM_FLAG_RESPONSE_REQUIRED);
    ISO_CHECK_EQ_UINT(decoded.message_type, BVM_MSG_HELLO);
    ISO_CHECK_EQ_UINT(decoded.request_id, 0x1234u);
    ISO_CHECK_EQ_UINT(decoded.payload_len, sizeof payload);
    ISO_CHECK_MEM_EQ(decoded.payload, payload, sizeof payload);
}

static void test_hello_wire_frame_golden(void) {
    bvm_frame_t frame;
    uint8_t raw[32];
    uint8_t wire[32];
    size_t wire_len = 0;

    frame.flags = BVM_FLAG_RESPONSE_REQUIRED;
    frame.message_type = BVM_MSG_HELLO;
    frame.request_id = 0x1234u;
    frame.payload = BVM_GOLDEN_HELLO_PAYLOAD_BVM_V1;
    frame.payload_len = sizeof BVM_GOLDEN_HELLO_PAYLOAD_BVM_V1;

    ISO_CHECK(bvm_encode_stream_frame(&frame, raw, sizeof raw, wire, sizeof wire,
                                      &wire_len) == BVM_OK);
    ISO_CHECK_MEM_EQ(raw, BVM_GOLDEN_HELLO_RAW_FRAME_BVM_V1,
                     sizeof BVM_GOLDEN_HELLO_RAW_FRAME_BVM_V1);
    ISO_CHECK_EQ_UINT(wire_len, sizeof BVM_GOLDEN_HELLO_WIRE_FRAME_BVM_V1);
    ISO_CHECK_MEM_EQ(wire, BVM_GOLDEN_HELLO_WIRE_FRAME_BVM_V1,
                     sizeof BVM_GOLDEN_HELLO_WIRE_FRAME_BVM_V1);
}

static void test_rejects_bad_crc(void) {
    uint8_t raw[18] = {
        0x01, 0x01, 0x01, 0x34, 0x12, 0x0A, 0x01, 0x01, 0x03, 'b', 'v', 'm',
        0xCD, 0xAB, 0x34, 0x12, 0x19, 0x49
    };
    bvm_frame_t decoded;
    raw[8] ^= 0x01u; /* corrupt a payload byte */
    ISO_CHECK(bvm_decode_frame(raw, sizeof raw, &decoded) == BVM_ERR_BAD_CRC);
}

static void test_cobs_round_trip_with_zeroes(void) {
    static const uint8_t raw[6] = { 0x11, 0x00, 0x22, 0x33, 0x00, 0x44 };
    static const uint8_t expected[8] = {
        0x02, 0x11, 0x03, 0x22, 0x33, 0x02, 0x44, 0x00
    };
    uint8_t encoded[16];
    uint8_t decoded[16];
    size_t enc_len = 0;
    size_t dec_len = 0;

    ISO_CHECK(bvm_encode_wire_frame(raw, sizeof raw, encoded, sizeof encoded,
                                    &enc_len) == BVM_OK);
    ISO_CHECK_EQ_UINT(enc_len, sizeof expected);
    ISO_CHECK_MEM_EQ(encoded, expected, sizeof expected);

    ISO_CHECK(bvm_decode_wire_frame(encoded, enc_len, decoded, sizeof decoded,
                                    &dec_len) == BVM_OK);
    ISO_CHECK_EQ_UINT(dec_len, sizeof raw);
    ISO_CHECK_MEM_EQ(decoded, raw, sizeof raw);
}

static void test_cobs_canonical_full_block(void) {
    uint8_t raw[254];
    uint8_t encoded[256];
    uint8_t decoded[254];
    size_t enc_len = 0;
    size_t dec_len = 0;
    memset(raw, 0x7A, sizeof raw);

    ISO_CHECK(bvm_encode_wire_frame(raw, sizeof raw, encoded, sizeof encoded,
                                    &enc_len) == BVM_OK);
    ISO_CHECK_EQ_UINT(enc_len, 256u);
    ISO_CHECK_EQ_UINT(encoded[0], 0xFFu);
    ISO_CHECK_EQ_UINT(encoded[255], 0x00u);

    ISO_CHECK(bvm_decode_wire_frame(encoded, enc_len, decoded, sizeof decoded,
                                    &dec_len) == BVM_OK);
    ISO_CHECK_EQ_UINT(dec_len, sizeof raw);
    ISO_CHECK_MEM_EQ(decoded, raw, sizeof raw);
}

static void test_stream_frame_round_trip(void) {
    static const uint8_t payload[4] = { 0x14, 0x00, 0x00, 0x00 };
    bvm_frame_t frame;
    uint8_t raw[32];
    uint8_t wire[40];
    uint8_t decoded_raw[32];
    size_t wire_len = 0;
    bvm_frame_t decoded;

    frame.flags = BVM_FLAG_RESPONSE_REQUIRED;
    frame.message_type = BVM_MSG_PING;
    frame.request_id = 7;
    frame.payload = payload;
    frame.payload_len = sizeof payload;

    ISO_CHECK(bvm_encode_stream_frame(&frame, raw, sizeof raw, wire, sizeof wire,
                                      &wire_len) == BVM_OK);
    ISO_CHECK_EQ_UINT(wire[wire_len - 1], 0u);

    ISO_CHECK(bvm_decode_stream_frame(wire, wire_len, decoded_raw, sizeof decoded_raw,
                                      &decoded) == BVM_OK);
    ISO_CHECK_EQ_UINT(decoded.flags, frame.flags);
    ISO_CHECK_EQ_UINT(decoded.message_type, frame.message_type);
    ISO_CHECK_EQ_UINT(decoded.request_id, frame.request_id);
    ISO_CHECK_EQ_UINT(decoded.payload_len, frame.payload_len);
    ISO_CHECK_MEM_EQ(decoded.payload, payload, sizeof payload);
}

static void test_bootloader_reboot_reserved(void) {
    ISO_CHECK_EQ_UINT(BVM_MSG_BOOTLOADER_REBOOT, 0x16u);
    ISO_CHECK(bvm_message_type_is_vendor_extension(BVM_MSG_BOOTLOADER_REBOOT) == 0);
    ISO_CHECK(bvm_message_type_is_vendor_extension(0x80u) == 1);
}

static void test_rejects_reserved_frame_flags(void) {
    bvm_frame_t frame;
    uint8_t out[16];
    size_t len = 0;
    frame.flags = BVM_FLAG_COMPRESSED_PAYLOAD;
    frame.message_type = BVM_MSG_PING;
    frame.request_id = 1;
    frame.payload = NULL;
    frame.payload_len = 0;
    ISO_CHECK(bvm_encode_frame(&frame, out, sizeof out, &len) == BVM_ERR_RESERVED_FLAGS);
}

static void test_upload_and_run_payloads(void) {
    uint8_t out[32];
    size_t len = 0;
    bvm_program_begin_t begin;
    bvm_program_begin_t begin_dec;
    bvm_run_request_t run;
    bvm_run_request_t run_dec;

    begin.program_id = 1;
    begin.format = BVM_PROGRAM_FORMAT_BVM_MODULE;
    begin.total_len = 36;
    begin.program_crc32 = 0xCAFEBABEu;
    ISO_CHECK(bvm_encode_program_begin(&begin, out, sizeof out, &len) == BVM_OK);
    ISO_CHECK_EQ_UINT(len, sizeof BVM_GOLDEN_PROGRAM_BEGIN_PAYLOAD_BVM_V1);
    ISO_CHECK_MEM_EQ(out, BVM_GOLDEN_PROGRAM_BEGIN_PAYLOAD_BVM_V1,
                     sizeof BVM_GOLDEN_PROGRAM_BEGIN_PAYLOAD_BVM_V1);
    ISO_CHECK(bvm_decode_program_begin(out, len, &begin_dec) == BVM_OK);
    ISO_CHECK_EQ_UINT(begin_dec.program_id, 1u);
    ISO_CHECK_EQ_UINT(begin_dec.format, BVM_PROGRAM_FORMAT_BVM_MODULE);
    ISO_CHECK_EQ_UINT(begin_dec.total_len, 36u);
    ISO_CHECK_EQ_UINT(begin_dec.program_crc32, 0xCAFEBABEu);

    run.program_id = 1;
    run.flags = BVM_RUN_FLAG_RESET_VM_BEFORE_RUN | BVM_RUN_FLAG_BACKGROUND_RUN;
    run.instruction_budget = 1000;
    run.time_budget_ms = 0;
    ISO_CHECK(bvm_encode_run_request(&run, out, sizeof out, &len) == BVM_OK);
    ISO_CHECK_EQ_UINT(len, sizeof BVM_GOLDEN_RUN_BACKGROUND_PAYLOAD_BVM_V1);
    ISO_CHECK_MEM_EQ(out, BVM_GOLDEN_RUN_BACKGROUND_PAYLOAD_BVM_V1,
                     sizeof BVM_GOLDEN_RUN_BACKGROUND_PAYLOAD_BVM_V1);
    ISO_CHECK(bvm_decode_run_request(out, len, &run_dec) == BVM_OK);
    ISO_CHECK_EQ_UINT(run_dec.program_id, 1u);
    ISO_CHECK_EQ_UINT(run_dec.flags,
                      BVM_RUN_FLAG_RESET_VM_BEFORE_RUN | BVM_RUN_FLAG_BACKGROUND_RUN);
    ISO_CHECK_EQ_UINT(run_dec.instruction_budget, 1000u);
    ISO_CHECK_EQ_UINT(run_dec.time_budget_ms, 0u);
}

static void test_caps_report_iterates(void) {
    bvm_capability_descriptor_t caps[2];
    bvm_caps_report_header_t header;
    uint8_t out[128];
    size_t len = 0;
    bvm_caps_report_header_t decoded_header;
    bvm_decoder_t dec;
    bvm_capability_descriptor_t cap0;
    bvm_capability_descriptor_t cap1;

    caps[0].id = BVM_CAP_PROGRAM_RAM_EXEC;
    caps[0].version = 1;
    caps[0].flags = BVM_CAP_FLAG_PROTOCOL_FEATURE;
    caps[0].name = "program.ram_exec";
    caps[0].name_len = 16;
    caps[1].id = BVM_CAP_PROGRAM_STORE;
    caps[1].version = 1;
    caps[1].flags = BVM_CAP_FLAG_PROTOCOL_FEATURE;
    caps[1].name = "program.store";
    caps[1].name_len = 13;

    header.board_id = "uno-r4-minima";
    header.board_id_len = 13;
    header.runtime_id = "board-vm-rust";
    header.runtime_id_len = 13;
    header.max_program_bytes = 1024;
    header.max_stack_values = 8;
    header.max_handles = 8;
    header.supports_store_program = 1;
    header.capability_count = 2;

    ISO_CHECK(bvm_encode_caps_report(&header, caps, 2, out, sizeof out, &len) == BVM_OK);
    ISO_CHECK(bvm_decode_caps_report_header(out, len, &decoded_header, &dec) == BVM_OK);
    ISO_CHECK_EQ_UINT(decoded_header.max_program_bytes, 1024u);
    ISO_CHECK_EQ_UINT(decoded_header.max_stack_values, 8u);
    ISO_CHECK_EQ_UINT(decoded_header.max_handles, 8u);
    ISO_CHECK(decoded_header.supports_store_program == 1);
    ISO_CHECK_EQ_UINT(decoded_header.capability_count, 2u);
    ISO_CHECK(memcmp(decoded_header.board_id, "uno-r4-minima", 13) == 0);
    ISO_CHECK(memcmp(decoded_header.runtime_id, "board-vm-rust", 13) == 0);

    ISO_CHECK(bvm_decoder_read_capability_descriptor(&dec, &cap0) == BVM_OK);
    ISO_CHECK_EQ_UINT(cap0.id, BVM_CAP_PROGRAM_RAM_EXEC);
    ISO_CHECK_EQ_UINT(cap0.name_len, 16u);
    ISO_CHECK(memcmp(cap0.name, "program.ram_exec", 16) == 0);
    ISO_CHECK(bvm_decoder_read_capability_descriptor(&dec, &cap1) == BVM_OK);
    ISO_CHECK_EQ_UINT(cap1.id, BVM_CAP_PROGRAM_STORE);
    ISO_CHECK(memcmp(cap1.name, "program.store", 13) == 0);
    ISO_CHECK(bvm_decoder_finish(&dec) == BVM_OK);
}

static void test_caps_report_count_mismatch(void) {
    bvm_capability_descriptor_t caps[1];
    bvm_caps_report_header_t header;
    uint8_t out[64];
    size_t len = 0;
    caps[0].id = BVM_CAP_PROGRAM_STORE;
    caps[0].version = 1;
    caps[0].flags = 0;
    caps[0].name = "x";
    caps[0].name_len = 1;
    header.board_id = "b";
    header.board_id_len = 1;
    header.runtime_id = "r";
    header.runtime_id_len = 1;
    header.max_program_bytes = 0;
    header.max_stack_values = 0;
    header.max_handles = 0;
    header.supports_store_program = 0;
    header.capability_count = 5; /* lie: doesn't match caps_count of 1 */
    ISO_CHECK(bvm_encode_caps_report(&header, caps, 1, out, sizeof out, &len)
              == BVM_ERR_PAYLOAD_LENGTH_MISMATCH);
}

static void test_values_reject_unknown_tag(void) {
    static const uint8_t bytes[1] = { 0x99 };
    bvm_value_t v;
    ISO_CHECK(bvm_decode_value(bytes, sizeof bytes, &v) == BVM_ERR_UNSUPPORTED_VALUE);
}

static void test_value_round_trips(void) {
    /* Exercise every Value arm through encode -> decode. */
    uint8_t out[32];
    size_t len = 0;
    bvm_value_t v;
    bvm_value_t d;

    v.tag = BVM_VALUE_UNIT;
    ISO_CHECK(bvm_encode_value(&v, out, sizeof out, &len) == BVM_OK);
    ISO_CHECK(bvm_decode_value(out, len, &d) == BVM_OK);
    ISO_CHECK(d.tag == BVM_VALUE_UNIT);

    v.tag = BVM_VALUE_BOOL;
    v.as.boolean = 1;
    ISO_CHECK(bvm_encode_value(&v, out, sizeof out, &len) == BVM_OK);
    ISO_CHECK(bvm_decode_value(out, len, &d) == BVM_OK);
    ISO_CHECK(d.tag == BVM_VALUE_BOOL && d.as.boolean == 1);

    v.tag = BVM_VALUE_U32;
    v.as.u32 = 0xDEADBEEFu;
    ISO_CHECK(bvm_encode_value(&v, out, sizeof out, &len) == BVM_OK);
    ISO_CHECK(bvm_decode_value(out, len, &d) == BVM_OK);
    ISO_CHECK(d.tag == BVM_VALUE_U32 && d.as.u32 == 0xDEADBEEFu);

    v.tag = BVM_VALUE_I16;
    v.as.i16 = -1234;
    ISO_CHECK(bvm_encode_value(&v, out, sizeof out, &len) == BVM_OK);
    ISO_CHECK(bvm_decode_value(out, len, &d) == BVM_OK);
    ISO_CHECK(d.tag == BVM_VALUE_I16 && d.as.i16 == -1234);

    v.tag = BVM_VALUE_STRING;
    v.as.str.ptr = "hi\xE2\x9C\x93"; /* "hi" + U+2713 CHECK MARK, valid UTF-8 */
    v.as.str.len = 5;
    ISO_CHECK(bvm_encode_value(&v, out, sizeof out, &len) == BVM_OK);
    ISO_CHECK(bvm_decode_value(out, len, &d) == BVM_OK);
    ISO_CHECK(d.tag == BVM_VALUE_STRING && d.as.str.len == 5);
    ISO_CHECK(memcmp(d.as.str.ptr, "hi\xE2\x9C\x93", 5) == 0);
}

static void test_invalid_utf8_rejected(void) {
    /* A string payload: uleb128 len=1 then a lone continuation byte 0x80. */
    static const uint8_t bytes[3] = { 0x08, 0x01, 0x80 };
    bvm_value_t v;
    ISO_CHECK(bvm_decode_value(bytes, sizeof bytes, &v) == BVM_ERR_INVALID_UTF8);
}

static void test_error_paths(void) {
    uint8_t small[4];
    size_t len = 0;
    bvm_frame_t frame;
    bvm_ping_t ping;
    bvm_frame_t decoded;
    uint8_t tiny[4] = { 0, 0, 0, 0 };

    /* Encoding into a too-small buffer reports OutputTooSmall. */
    ping.nonce = 0x11223344u;
    frame.flags = 0;
    frame.message_type = BVM_MSG_PING;
    frame.request_id = 0;
    frame.payload = NULL;
    frame.payload_len = 0;
    ISO_CHECK(bvm_encode_frame(&frame, small, sizeof small, &len) == BVM_ERR_OUTPUT_TOO_SMALL);

    /* Frames shorter than 8 bytes are rejected before CRC. */
    ISO_CHECK(bvm_decode_frame(tiny, sizeof tiny, &decoded) == BVM_ERR_INPUT_TOO_SHORT);

    /* Wire frame without a terminator. */
    {
        static const uint8_t noterm[3] = { 0x02, 0x11, 0x22 };
        uint8_t out[8];
        ISO_CHECK(bvm_decode_wire_frame(noterm, sizeof noterm, out, sizeof out, &len)
                  == BVM_ERR_MISSING_TERMINATOR);
    }
    /* Empty wire frame. */
    {
        uint8_t out[8];
        ISO_CHECK(bvm_decode_wire_frame(NULL, 0, out, sizeof out, &len)
                  == BVM_ERR_INPUT_TOO_SHORT);
    }
    /* COBS with an embedded zero code byte is invalid. */
    {
        static const uint8_t badcobs[3] = { 0x00, 0x11, 0x00 };
        uint8_t out[8];
        ISO_CHECK(bvm_cobs_decode(badcobs, 2, out, sizeof out, &len) == BVM_ERR_INVALID_COBS);
    }
    /* Trailing bytes after a complete ping payload. */
    {
        static const uint8_t trailing[5] = { 0x44, 0x33, 0x22, 0x11, 0xFF };
        ISO_CHECK(bvm_decode_ping(trailing, sizeof trailing, &ping) == BVM_ERR_TRAILING_BYTES);
    }
}

/* Feed random and truncated buffers to every decoder; nothing may crash and
 * a decode must never read past the buffer (ASan/UBSan enforce this). */
static void test_fuzz_decoders(void) {
    int iter;
    for (iter = 0; iter < 20000; ++iter) {
        uint8_t buf[24];
        size_t n = (size_t)(rng_next() % (sizeof buf + 1));
        size_t i;
        bvm_frame_t frame;
        bvm_hello_t hello;
        bvm_value_t value;
        bvm_program_begin_t begin;
        bvm_run_request_t run;
        bvm_caps_report_header_t hdr;
        bvm_decoder_t dec;
        uint8_t scratch[32];
        size_t out_len = 0;
        for (i = 0; i < n; ++i) {
            buf[i] = (uint8_t)(rng_next() & 0xFFu);
        }
        (void)bvm_decode_frame(buf, n, &frame);
        (void)bvm_decode_hello(buf, n, &hello);
        (void)bvm_decode_value(buf, n, &value);
        (void)bvm_decode_program_begin(buf, n, &begin);
        (void)bvm_decode_run_request(buf, n, &run);
        (void)bvm_decode_caps_report_header(buf, n, &hdr, &dec);
        (void)bvm_decode_wire_frame(buf, n, scratch, sizeof scratch, &out_len);
        (void)bvm_cobs_decode(buf, n, scratch, sizeof scratch, &out_len);
    }
    ISO_CHECK(1); /* reaching here without a sanitizer trap is the assertion */
}

/* Byte-flip fuzz: mutate a valid wire frame and confirm the decoder either
 * rejects it or round-trips cleanly, never overrunning the buffer. */
static void test_fuzz_byte_flip_wire(void) {
    bvm_frame_t frame;
    uint8_t raw[32];
    uint8_t wire[40];
    size_t wire_len = 0;
    int iter;
    static const uint8_t payload[4] = { 0x14, 0x00, 0x00, 0x00 };

    frame.flags = BVM_FLAG_RESPONSE_REQUIRED;
    frame.message_type = BVM_MSG_PING;
    frame.request_id = 7;
    frame.payload = payload;
    frame.payload_len = sizeof payload;
    ISO_CHECK(bvm_encode_stream_frame(&frame, raw, sizeof raw, wire, sizeof wire,
                                      &wire_len) == BVM_OK);

    for (iter = 0; iter < 20000; ++iter) {
        uint8_t mutated[40];
        size_t pos = (size_t)(rng_next() % wire_len);
        uint8_t decoded_raw[40];
        bvm_frame_t decoded;
        memcpy(mutated, wire, wire_len);
        mutated[pos] ^= (uint8_t)(1u << (rng_next() & 7u));
        /* Result is don't-care; the point is no OOB access under sanitizers. */
        (void)bvm_decode_stream_frame(mutated, wire_len, decoded_raw, sizeof decoded_raw,
                                      &decoded);
    }
    ISO_CHECK(1);
}

int main(void) {
    test_crc_check_vector();
    test_uleb128_boundaries();
    test_hello_payload_golden();
    test_raw_frame_with_crc();
    test_hello_wire_frame_golden();
    test_rejects_bad_crc();
    test_cobs_round_trip_with_zeroes();
    test_cobs_canonical_full_block();
    test_stream_frame_round_trip();
    test_bootloader_reboot_reserved();
    test_rejects_reserved_frame_flags();
    test_upload_and_run_payloads();
    test_caps_report_iterates();
    test_caps_report_count_mismatch();
    test_values_reject_unknown_tag();
    test_value_round_trips();
    test_invalid_utf8_rejected();
    test_error_paths();
    test_fuzz_decoders();
    test_fuzz_byte_flip_wire();
    return ISO_TEST_RESULT();
}
