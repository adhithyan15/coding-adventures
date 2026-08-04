// Tests for the C++ resp-protocol, using the iso_test.h harness. The encode and
// decode vectors are taken from the Rust crate's own tests.
#include "iso_test.h"

#include <cstdint>
#include <string>
#include <vector>

#include "resp_protocol.hpp"

namespace resp = ca::resp;
using resp::Value;

static std::vector<std::uint8_t> bytes(const char* s) {
    return std::vector<std::uint8_t>(s, s + std::string(s).size());
}

// Assert encode(v) succeeds and equals `expected`.
static void check_encode(const Value& v, const char* expected) {
    auto out = resp::encode(v);
    ISO_CHECK(out.has_value());
    if (out) {
        ISO_CHECK(*out == bytes(expected));
    }
}

int main() {
    // ---- encoding (encoder.rs) ---------------------------------------
    check_encode(Value::simple_string("OK"), "+OK\r\n");
    check_encode(Value::make_error("ERR boom"), "-ERR boom\r\n");
    check_encode(Value::make_integer(-42), ":-42\r\n");
    check_encode(Value::make_integer(123), ":123\r\n");
    check_encode(Value::bulk_string_null(), "$-1\r\n");
    check_encode(Value::bulk_string("abc"), "$3\r\nabc\r\n");
    check_encode(Value::bulk_string("payload"), "$7\r\npayload\r\n");
    check_encode(Value::bulk_string(""), "$0\r\n\r\n");
    check_encode(Value::make_array_null(), "*-1\r\n");
    check_encode(
        Value::make_array({Value::simple_string("OK"), Value::make_integer(7),
                           Value::bulk_string_null(),
                           Value::make_array({Value::simple_string("nested")})}),
        "*4\r\n+OK\r\n:7\r\n$-1\r\n*1\r\n+nested\r\n");

    // A simple string with an embedded newline is rejected.
    ISO_CHECK(!resp::encode(Value::simple_string("bad\nnews")).has_value());

    // ---- error type/detail split (types.rs) --------------------------
    {
        Value e = Value::make_error("ERR boom");
        Value s = Value::make_error("ERR");
        ISO_CHECK(e.error_type() == "ERR");
        ISO_CHECK(e.error_detail() == "boom");
        ISO_CHECK(s.error_type() == "ERR");
        ISO_CHECK(s.error_detail() == "");
    }

    // ---- decoding (decoder.rs) ---------------------------------------
    {
        auto expect_value = [](const char* input, const Value& expected,
                               std::size_t consumed) {
            auto in = bytes(input);
            resp::DecodeResult r = resp::decode(in);
            ISO_CHECK(r.is_value());
            if (r.is_value()) {
                ISO_CHECK(r.value == expected);
                ISO_CHECK_EQ_UINT(r.consumed, consumed);
            }
        };
        expect_value("+OK\r\n", Value::simple_string("OK"), 5);
        expect_value("-ERR boom\r\n", Value::make_error("ERR boom"), 11);
        expect_value(":-42\r\n", Value::make_integer(-42), 6);
        expect_value("$-1\r\n", Value::bulk_string_null(), 5);
        expect_value("$3\r\nfoo\r\n", Value::bulk_string("foo"), 9);
        expect_value("*-1\r\n", Value::make_array_null(), 5);
        expect_value(
            "*2\r\n+OK\r\n:1\r\n",
            Value::make_array({Value::simple_string("OK"), Value::make_integer(1)}),
            13);
        expect_value(
            "PING  PONG\r\n",
            Value::make_array({Value::bulk_string("PING"), Value::bulk_string("PONG")}),
            12);
        expect_value("$0\r\n\r\n", Value::bulk_string(""), 6);
    }

    // Incomplete inputs.
    ISO_CHECK(resp::decode(bytes("+")).is_incomplete());
    ISO_CHECK(resp::decode(bytes("$3\r\nfo")).is_incomplete());
    ISO_CHECK(resp::decode(bytes("*2\r\n+OK\r\n")).is_incomplete());

    // Malformed inputs.
    {
        std::vector<std::uint8_t> inv_simple = {'+', 0xff, '\r', '\n'};
        std::vector<std::uint8_t> inv_error = {'-', 0xff, '\r', '\n'};
        std::vector<std::uint8_t> inv_bulklen = {'$', 0xff, '\r', '\n'};
        std::vector<std::uint8_t> inv_arrlen = {'*', 0xff, '\r', '\n'};
        ISO_CHECK(resp::decode(inv_simple).is_error());
        ISO_CHECK(resp::decode(inv_error).is_error());
        ISO_CHECK(resp::decode(bytes(":foo\r\n")).is_error());
        ISO_CHECK(resp::decode(inv_bulklen).is_error());
        ISO_CHECK(resp::decode(bytes("$-10\r\n")).is_error());
        ISO_CHECK(resp::decode(inv_arrlen).is_error());
        ISO_CHECK(resp::decode(bytes("*-10\r\n")).is_error());
    }

    // A hostile array header must not pre-allocate for the declared count:
    // "*100000000\r\n:1\r\n" declares 1e8 items but supplies one, so decoding
    // returns Incomplete promptly rather than reserving ~1e8 elements.
    ISO_CHECK(resp::decode(bytes("*100000000\r\n:1\r\n")).is_incomplete());

    // ---- decode_all --------------------------------------------------
    {
        resp::DecodeAllResult r = resp::decode_all(bytes("+OK\r\n:1\r\n"));
        ISO_CHECK(r.ok);
        ISO_CHECK_EQ_UINT(r.values.size(), 2u);
        ISO_CHECK_EQ_UINT(r.consumed, 9u);
        if (r.values.size() == 2) {
            ISO_CHECK(r.values[0] == Value::simple_string("OK"));
            ISO_CHECK(r.values[1] == Value::make_integer(1));
        }
    }

    // ---- streaming Decoder -------------------------------------------
    {
        resp::Decoder d;
        ISO_CHECK(!d.has_message());
        ISO_CHECK(!d.get_message().has_value());

        d.feed(std::string("+OK\r\n"));
        ISO_CHECK(d.has_message());
        auto msg = d.get_message();
        ISO_CHECK(msg.has_value() && *msg == Value::simple_string("OK"));
        ISO_CHECK(!d.has_message());

        d.feed(std::string(":1\r\n"));
        resp::DecodeAllResult r = d.decode_all(std::string("+PONG\r\n"));
        ISO_CHECK(r.ok);
        ISO_CHECK_EQ_UINT(r.values.size(), 2u);
        if (r.values.size() == 2) {
            ISO_CHECK(r.values[0] == Value::make_integer(1));
            ISO_CHECK(r.values[1] == Value::simple_string("PONG"));
        }
    }

    // A malformed frame latches the decoder into an error state.
    {
        resp::Decoder d;
        resp::DecodeAllResult r = d.decode_all(std::string("*-10\r\n"));
        ISO_CHECK(!r.ok);
        ISO_CHECK(d.has_error());
        ISO_CHECK(!d.get_message().has_value());
    }

    return ISO_TEST_RESULT();
}
