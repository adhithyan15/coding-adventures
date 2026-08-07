// board_vm_protocol_test.cpp — unit tests for the C++ board VM protocol codec.
//
// Mirrors the Rust crate's test suite (golden vectors, CRC check value,
// ULEB128 boundaries, frame/wire round-trips, error paths) and adds a
// truncation / byte-flip fuzz sweep over the decoders.
#include "board_vm_protocol.hpp"
#include "iso_test.h"

#include <cstring>
#include <vector>

namespace bvm = ca::board_vm_protocol;

namespace {

// Deterministic xorshift PRNG for reproducible fuzzing.
std::uint32_t g_rng = 0x1234ABCDu;
std::uint32_t rng_next() {
    std::uint32_t x = g_rng;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    g_rng = x;
    return x;
}

bvm::ByteView view(const std::vector<std::uint8_t>& v) {
    return bvm::ByteView(v.data(), v.size());
}

// Run a callable and report whether it threw the expected Error.
template <typename F>
bool throws_error(bvm::Error expected, F&& fn) {
    try {
        fn();
    } catch (const bvm::ProtocolError& e) {
        return e.code() == expected;
    } catch (...) {
        return false;
    }
    return false;
}

void test_crc_check_vector() {
    static const std::uint8_t msg[9] = {'1', '2', '3', '4', '5', '6', '7', '8', '9'};
    ISO_CHECK_EQ_UINT(bvm::crc16_ccitt_false(msg, sizeof msg), 0x29B1u);
}

void test_uleb128_boundaries() {
    static const std::uint8_t expected[9] = {0x00, 0x7F, 0x80, 0x01, 0xFF, 0x7F, 0x80, 0x80, 0x01};
    std::vector<std::uint8_t> out;
    bvm::Encoder enc(out);
    enc.write_uleb128(0);
    enc.write_uleb128(127);
    enc.write_uleb128(128);
    enc.write_uleb128(16383);
    enc.write_uleb128(16384);
    ISO_CHECK_EQ_UINT(out.size(), sizeof expected);
    ISO_CHECK_MEM_EQ(out.data(), expected, sizeof expected);
}

void test_hello_payload_golden() {
    bvm::Hello hello;
    hello.min_version = 1;
    hello.max_version = 1;
    hello.host_name = "bvm";
    hello.host_nonce = 0x1234ABCDu;

    std::vector<std::uint8_t> payload = bvm::encode_hello(hello);
    ISO_CHECK_EQ_UINT(payload.size(), bvm::GOLDEN_HELLO_PAYLOAD_BVM_V1.size());
    ISO_CHECK_MEM_EQ(payload.data(), bvm::GOLDEN_HELLO_PAYLOAD_BVM_V1.data(),
                     bvm::GOLDEN_HELLO_PAYLOAD_BVM_V1.size());

    bvm::Hello decoded = bvm::decode_hello(view(payload));
    ISO_CHECK(decoded == hello);
}

void test_raw_frame_with_crc() {
    static const std::uint8_t payload[10] = {0x01, 0x01, 0x03, 'b', 'v', 'm', 0xCD, 0xAB, 0x34, 0x12};
    bvm::Frame frame;
    frame.flags = bvm::FLAG_RESPONSE_REQUIRED;
    frame.message_type = bvm::MessageType::Hello;
    frame.request_id = 0x1234u;
    frame.payload = bvm::ByteView(payload, sizeof payload);

    std::vector<std::uint8_t> out = bvm::encode_frame(frame);
    ISO_CHECK_EQ_UINT(out.size(), bvm::GOLDEN_HELLO_RAW_FRAME_BVM_V1.size());
    ISO_CHECK_MEM_EQ(out.data(), bvm::GOLDEN_HELLO_RAW_FRAME_BVM_V1.data(),
                     bvm::GOLDEN_HELLO_RAW_FRAME_BVM_V1.size());

    bvm::Frame decoded = bvm::decode_frame(view(out));
    ISO_CHECK(decoded == frame);
}

void test_hello_wire_frame_golden() {
    bvm::Frame frame;
    frame.flags = bvm::FLAG_RESPONSE_REQUIRED;
    frame.message_type = bvm::MessageType::Hello;
    frame.request_id = 0x1234u;
    frame.payload = bvm::ByteView(bvm::GOLDEN_HELLO_PAYLOAD_BVM_V1.data(),
                                  bvm::GOLDEN_HELLO_PAYLOAD_BVM_V1.size());

    std::vector<std::uint8_t> raw;
    std::vector<std::uint8_t> wire = bvm::encode_stream_frame(frame, &raw);
    ISO_CHECK_EQ_UINT(raw.size(), bvm::GOLDEN_HELLO_RAW_FRAME_BVM_V1.size());
    ISO_CHECK_MEM_EQ(raw.data(), bvm::GOLDEN_HELLO_RAW_FRAME_BVM_V1.data(),
                     bvm::GOLDEN_HELLO_RAW_FRAME_BVM_V1.size());
    ISO_CHECK_EQ_UINT(wire.size(), bvm::GOLDEN_HELLO_WIRE_FRAME_BVM_V1.size());
    ISO_CHECK_MEM_EQ(wire.data(), bvm::GOLDEN_HELLO_WIRE_FRAME_BVM_V1.data(),
                     bvm::GOLDEN_HELLO_WIRE_FRAME_BVM_V1.size());
}

void test_rejects_bad_crc() {
    std::uint8_t raw[18] = {0x01, 0x01, 0x01, 0x34, 0x12, 0x0A, 0x01, 0x01, 0x03, 'b',
                            'v',  'm',  0xCD, 0xAB, 0x34, 0x12, 0x19, 0x49};
    raw[8] ^= 0x01u;
    ISO_CHECK(throws_error(bvm::Error::BadCrc,
                           [&] { bvm::decode_frame(bvm::ByteView(raw, sizeof raw)); }));
}

void test_cobs_round_trip_with_zeroes() {
    static const std::uint8_t raw[6] = {0x11, 0x00, 0x22, 0x33, 0x00, 0x44};
    static const std::uint8_t expected[8] = {0x02, 0x11, 0x03, 0x22, 0x33, 0x02, 0x44, 0x00};
    std::vector<std::uint8_t> encoded = bvm::encode_wire_frame(bvm::ByteView(raw, sizeof raw));
    ISO_CHECK_EQ_UINT(encoded.size(), sizeof expected);
    ISO_CHECK_MEM_EQ(encoded.data(), expected, sizeof expected);

    std::vector<std::uint8_t> decoded = bvm::decode_wire_frame(view(encoded));
    ISO_CHECK_EQ_UINT(decoded.size(), sizeof raw);
    ISO_CHECK_MEM_EQ(decoded.data(), raw, sizeof raw);
}

void test_cobs_canonical_full_block() {
    std::vector<std::uint8_t> raw(254, 0x7A);
    std::vector<std::uint8_t> encoded = bvm::encode_wire_frame(view(raw));
    ISO_CHECK_EQ_UINT(encoded.size(), 256u);
    ISO_CHECK_EQ_UINT(encoded[0], 0xFFu);
    ISO_CHECK_EQ_UINT(encoded[255], 0x00u);

    std::vector<std::uint8_t> decoded = bvm::decode_wire_frame(view(encoded));
    ISO_CHECK_EQ_UINT(decoded.size(), raw.size());
    ISO_CHECK_MEM_EQ(decoded.data(), raw.data(), raw.size());
}

void test_stream_frame_round_trip() {
    static const std::uint8_t payload[4] = {0x14, 0x00, 0x00, 0x00};
    bvm::Frame frame;
    frame.flags = bvm::FLAG_RESPONSE_REQUIRED;
    frame.message_type = bvm::MessageType::Ping;
    frame.request_id = 7;
    frame.payload = bvm::ByteView(payload, sizeof payload);

    std::vector<std::uint8_t> wire = bvm::encode_stream_frame(frame);
    ISO_CHECK_EQ_UINT(wire.back(), 0u);

    std::vector<std::uint8_t> raw_out;
    bvm::Frame decoded = bvm::decode_stream_frame(view(wire), raw_out);
    ISO_CHECK(decoded == frame);
}

void test_bootloader_reboot_reserved() {
    ISO_CHECK_EQ_UINT(static_cast<unsigned>(bvm::MessageType::BootloaderReboot), 0x16u);
    ISO_CHECK(bvm::is_vendor_extension(bvm::MessageType::BootloaderReboot) == false);
    ISO_CHECK(bvm::is_vendor_extension(static_cast<bvm::MessageType>(0x80)) == true);
}

void test_rejects_reserved_frame_flags() {
    bvm::Frame frame;
    frame.flags = bvm::FLAG_COMPRESSED_PAYLOAD;
    frame.message_type = bvm::MessageType::Ping;
    frame.request_id = 1;
    ISO_CHECK(throws_error(bvm::Error::ReservedFlags, [&] { bvm::encode_frame(frame); }));
}

void test_upload_and_run_payloads() {
    bvm::ProgramBegin begin;
    begin.program_id = 1;
    begin.format = bvm::ProgramFormat::BvmModule;
    begin.total_len = 36;
    begin.program_crc32 = 0xCAFEBABEu;
    std::vector<std::uint8_t> out = bvm::encode_program_begin(begin);
    ISO_CHECK_EQ_UINT(out.size(), bvm::GOLDEN_PROGRAM_BEGIN_PAYLOAD_BVM_V1.size());
    ISO_CHECK_MEM_EQ(out.data(), bvm::GOLDEN_PROGRAM_BEGIN_PAYLOAD_BVM_V1.data(),
                     bvm::GOLDEN_PROGRAM_BEGIN_PAYLOAD_BVM_V1.size());
    ISO_CHECK(bvm::decode_program_begin(view(out)) == begin);

    bvm::RunRequest run;
    run.program_id = 1;
    run.flags = bvm::RUN_FLAG_RESET_VM_BEFORE_RUN | bvm::RUN_FLAG_BACKGROUND_RUN;
    run.instruction_budget = 1000;
    run.time_budget_ms = 0;
    out = bvm::encode_run_request(run);
    ISO_CHECK_EQ_UINT(out.size(), bvm::GOLDEN_RUN_BACKGROUND_PAYLOAD_BVM_V1.size());
    ISO_CHECK_MEM_EQ(out.data(), bvm::GOLDEN_RUN_BACKGROUND_PAYLOAD_BVM_V1.data(),
                     bvm::GOLDEN_RUN_BACKGROUND_PAYLOAD_BVM_V1.size());
    ISO_CHECK(bvm::decode_run_request(view(out)) == run);
}

void test_caps_report_iterates() {
    std::vector<bvm::CapabilityDescriptor> caps(2);
    caps[0].id = bvm::CAP_PROGRAM_RAM_EXEC;
    caps[0].version = 1;
    caps[0].flags = bvm::CAP_FLAG_PROTOCOL_FEATURE;
    caps[0].name = "program.ram_exec";
    caps[1].id = bvm::CAP_PROGRAM_STORE;
    caps[1].version = 1;
    caps[1].flags = bvm::CAP_FLAG_PROTOCOL_FEATURE;
    caps[1].name = "program.store";

    bvm::CapsReportHeader header;
    header.board_id = "uno-r4-minima";
    header.runtime_id = "board-vm-rust";
    header.max_program_bytes = 1024;
    header.max_stack_values = 8;
    header.max_handles = 8;
    header.supports_store_program = true;
    header.capability_count = 2;

    std::vector<std::uint8_t> out = bvm::encode_caps_report(header, caps);
    std::pair<bvm::CapsReportHeader, bvm::Decoder> decoded = bvm::decode_caps_report_header(view(out));
    ISO_CHECK(decoded.first == header);
    bvm::Decoder dec = decoded.second;
    ISO_CHECK(dec.read_capability_descriptor() == caps[0]);
    ISO_CHECK(dec.read_capability_descriptor() == caps[1]);
    dec.finish(); // throws if not exactly consumed
    ISO_CHECK(true);
}

void test_caps_report_count_mismatch() {
    std::vector<bvm::CapabilityDescriptor> caps(1);
    caps[0].name = "x";
    bvm::CapsReportHeader header;
    header.board_id = "b";
    header.runtime_id = "r";
    header.capability_count = 5; // mismatch
    ISO_CHECK(throws_error(bvm::Error::PayloadLengthMismatch,
                           [&] { bvm::encode_caps_report(header, caps); }));
}

void test_values_reject_unknown_tag() {
    static const std::uint8_t bytes[1] = {0x99};
    ISO_CHECK(throws_error(bvm::Error::UnsupportedValue,
                           [&] { bvm::decode_value(bvm::ByteView(bytes, sizeof bytes)); }));
}

void test_value_round_trips() {
    const bvm::Value values[] = {
        bvm::Value::unit(),
        bvm::Value::make_bool(true),
        bvm::Value::make_u8(0xAB),
        bvm::Value::make_u16(0x1234),
        bvm::Value::make_u32(0xDEADBEEFu),
        bvm::Value::make_i16(-1234),
        bvm::Value::make_handle(0x0007),
    };
    for (const bvm::Value& v : values) {
        std::vector<std::uint8_t> out = bvm::encode_value(v);
        ISO_CHECK(bvm::decode_value(view(out)) == v);
    }

    static const std::uint8_t utf8[5] = {'h', 'i', 0xE2, 0x9C, 0x93}; // "hi" + U+2713
    bvm::Value sv = bvm::Value::make_string(
        std::string_view(reinterpret_cast<const char*>(utf8), sizeof utf8));
    std::vector<std::uint8_t> out = bvm::encode_value(sv);
    ISO_CHECK(bvm::decode_value(view(out)) == sv);

    static const std::uint8_t blob[3] = {0x00, 0x7F, 0xFF};
    bvm::Value bv = bvm::Value::make_bytes(bvm::ByteView(blob, sizeof blob));
    out = bvm::encode_value(bv);
    ISO_CHECK(bvm::decode_value(view(out)) == bv);
}

void test_invalid_utf8_rejected() {
    static const std::uint8_t bytes[3] = {0x08, 0x01, 0x80}; // string len=1, lone continuation
    ISO_CHECK(throws_error(bvm::Error::InvalidUtf8,
                           [&] { bvm::decode_value(bvm::ByteView(bytes, sizeof bytes)); }));
}

void test_error_paths() {
    static const std::uint8_t tiny[4] = {0, 0, 0, 0};
    ISO_CHECK(throws_error(bvm::Error::InputTooShort,
                           [&] { bvm::decode_frame(bvm::ByteView(tiny, sizeof tiny)); }));

    static const std::uint8_t noterm[3] = {0x02, 0x11, 0x22};
    ISO_CHECK(throws_error(bvm::Error::MissingTerminator,
                           [&] { bvm::decode_wire_frame(bvm::ByteView(noterm, sizeof noterm)); }));

    ISO_CHECK(throws_error(bvm::Error::InputTooShort,
                           [&] { bvm::decode_wire_frame(bvm::ByteView(nullptr, 0)); }));

    static const std::uint8_t badcobs[2] = {0x00, 0x11};
    ISO_CHECK(throws_error(bvm::Error::InvalidCobs,
                           [&] { bvm::cobs_decode(bvm::ByteView(badcobs, sizeof badcobs)); }));

    static const std::uint8_t trailing[5] = {0x44, 0x33, 0x22, 0x11, 0xFF};
    ISO_CHECK(throws_error(bvm::Error::TrailingBytes,
                           [&] { bvm::decode_ping(bvm::ByteView(trailing, sizeof trailing)); }));

    static const std::uint8_t badbool[2] = {0x01, 0x02}; // value tag Bool, then 0x02
    ISO_CHECK(throws_error(bvm::Error::InvalidBool,
                           [&] { bvm::decode_value(bvm::ByteView(badbool, sizeof badbool)); }));
}

// Random / truncated inputs to every decoder: nothing may crash or read OOB
// (ASan/UBSan enforce that); exceptions are expected and swallowed.
void test_fuzz_decoders() {
    for (int iter = 0; iter < 20000; ++iter) {
        std::uint8_t buf[24];
        std::size_t n = static_cast<std::size_t>(rng_next() % (sizeof buf + 1));
        for (std::size_t i = 0; i < n; ++i) {
            buf[i] = static_cast<std::uint8_t>(rng_next() & 0xFF);
        }
        bvm::ByteView v(buf, n);
        try { (void)bvm::decode_frame(v); } catch (const bvm::ProtocolError&) {}
        try { (void)bvm::decode_hello(v); } catch (const bvm::ProtocolError&) {}
        try { (void)bvm::decode_value(v); } catch (const bvm::ProtocolError&) {}
        try { (void)bvm::decode_program_begin(v); } catch (const bvm::ProtocolError&) {}
        try { (void)bvm::decode_run_request(v); } catch (const bvm::ProtocolError&) {}
        try { (void)bvm::decode_caps_report_header(v); } catch (const bvm::ProtocolError&) {}
        try { (void)bvm::decode_wire_frame(v); } catch (const bvm::ProtocolError&) {}
        try { (void)bvm::cobs_decode(v); } catch (const bvm::ProtocolError&) {}
    }
    ISO_CHECK(true);
}

// Byte-flip fuzz over a valid wire frame.
void test_fuzz_byte_flip_wire() {
    static const std::uint8_t payload[4] = {0x14, 0x00, 0x00, 0x00};
    bvm::Frame frame;
    frame.flags = bvm::FLAG_RESPONSE_REQUIRED;
    frame.message_type = bvm::MessageType::Ping;
    frame.request_id = 7;
    frame.payload = bvm::ByteView(payload, sizeof payload);
    std::vector<std::uint8_t> wire = bvm::encode_stream_frame(frame);

    for (int iter = 0; iter < 20000; ++iter) {
        std::vector<std::uint8_t> mutated = wire;
        std::size_t pos = static_cast<std::size_t>(rng_next() % mutated.size());
        mutated[pos] ^= static_cast<std::uint8_t>(1u << (rng_next() & 7u));
        std::vector<std::uint8_t> raw_out;
        try {
            (void)bvm::decode_stream_frame(view(mutated), raw_out);
        } catch (const bvm::ProtocolError&) {
        }
    }
    ISO_CHECK(true);
}

} // namespace

int main() {
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
    ISO_TEST_RESULT();
}
