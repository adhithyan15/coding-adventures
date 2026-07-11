// resp_protocol.hpp — the RESP (REdis Serialization Protocol) v2, in pure ISO
// C++17, header-only, in namespace ca::resp. A faithful port of the Rust
// `resp-protocol` crate.
// ===========================================================================
//
// RESP is the line protocol Redis speaks. A value is one of five frame types,
// each introduced by a one-byte prefix and terminated by CRLF ("\r\n"):
//
//   +OK\r\n              simple string
//   -ERR boom\r\n        error   (message may be split "TYPE detail")
//   :-42\r\n             integer (signed 64-bit)
//   $3\r\nfoo\r\n        bulk string (length-prefixed bytes; $-1 == null)
//   *2\r\n:1\r\n:2\r\n   array (count-prefixed values; *-1 == null)
//
// A bare line with no known prefix is parsed as an "inline command": the line is
// split on ASCII whitespace and each token becomes a bulk string inside an
// array.
//
// The recursive value is `ca::resp::Value` (value semantics — copy, move, and
// compare freely). Encoding may fail only for a simple string containing CR/LF
// (returns std::nullopt). Decoding distinguishes three outcomes — a value, "need
// more bytes", and "malformed" — via DecodeResult.
//
// Portability: pure ISO C++17 — GCC, Clang, and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors. Standard library only.
#ifndef CA_RESP_PROTOCOL_HPP
#define CA_RESP_PROTOCOL_HPP

#include <cstddef>
#include <cstdint>
#include <deque>
#include <optional>
#include <string>
#include <vector>

namespace ca {
namespace resp {

enum class Type { SimpleString, Error, Integer, BulkString, Array };

// A RESP value. Only the field(s) matching `type` are meaningful; use the
// factories to build one. std::vector supports the incomplete `Value` here
// (guaranteed since C++17), which is what makes the recursive Array work.
struct Value {
    Type type = Type::Integer;
    std::string simple;               // SimpleString
    std::string error_message;        // Error (the whole message)
    long long integer = 0;            // Integer
    std::vector<std::uint8_t> bulk;   // BulkString bytes (when !bulk_null)
    bool bulk_null = false;           // BulkString: the null bulk string
    std::vector<Value> array;         // Array children (when !array_null)
    bool array_null = false;          // Array: the null array

    static Value simple_string(std::string s) {
        Value v;
        v.type = Type::SimpleString;
        v.simple = std::move(s);
        return v;
    }
    static Value make_error(std::string message) {
        Value v;
        v.type = Type::Error;
        v.error_message = std::move(message);
        return v;
    }
    static Value make_integer(long long value) {
        Value v;
        v.type = Type::Integer;
        v.integer = value;
        return v;
    }
    static Value bulk_string(std::vector<std::uint8_t> bytes) {
        Value v;
        v.type = Type::BulkString;
        v.bulk = std::move(bytes);
        return v;
    }
    static Value bulk_string(const std::string& s) {
        return bulk_string(std::vector<std::uint8_t>(s.begin(), s.end()));
    }
    static Value bulk_string_null() {
        Value v;
        v.type = Type::BulkString;
        v.bulk_null = true;
        return v;
    }
    static Value make_array(std::vector<Value> items) {
        Value v;
        v.type = Type::Array;
        v.array = std::move(items);
        return v;
    }
    static Value make_array_null() {
        Value v;
        v.type = Type::Array;
        v.array_null = true;
        return v;
    }

    // Error accessors: the message split at the first space (Rust semantics).
    std::string error_type() const {
        std::size_t p = error_message.find(' ');
        return p == std::string::npos ? error_message : error_message.substr(0, p);
    }
    std::string error_detail() const {
        std::size_t p = error_message.find(' ');
        return p == std::string::npos ? std::string() : error_message.substr(p + 1);
    }

    bool operator==(const Value& o) const {
        if (type != o.type) {
            return false;
        }
        switch (type) {
            case Type::SimpleString:
                return simple == o.simple;
            case Type::Error:
                return error_message == o.error_message;
            case Type::Integer:
                return integer == o.integer;
            case Type::BulkString:
                return bulk_null ? o.bulk_null : (!o.bulk_null && bulk == o.bulk);
            case Type::Array:
                return array_null ? o.array_null
                                  : (!o.array_null && array == o.array);
        }
        return false;
    }
    bool operator!=(const Value& o) const { return !(*this == o); }
};

namespace detail {

// A validating UTF-8 scan (the crate rejects non-UTF-8 in text frames).
inline bool utf8_valid(const std::uint8_t* s, std::size_t n) {
    std::size_t i = 0;
    while (i < n) {
        std::uint8_t c = s[i];
        std::size_t extra, k;
        unsigned long min_cp, cp;
        if (c < 0x80) {
            ++i;
            continue;
        } else if ((c & 0xE0) == 0xC0) {
            extra = 1;
            min_cp = 0x80;
            cp = c & 0x1Fu;
        } else if ((c & 0xF0) == 0xE0) {
            extra = 2;
            min_cp = 0x800;
            cp = c & 0x0Fu;
        } else if ((c & 0xF8) == 0xF0) {
            extra = 3;
            min_cp = 0x10000;
            cp = c & 0x07u;
        } else {
            return false;
        }
        if (extra >= n - i) {
            return false;
        }
        for (k = 1; k <= extra; ++k) {
            std::uint8_t cc = s[i + k];
            if ((cc & 0xC0) != 0x80) {
                return false;
            }
            cp = (cp << 6) | (cc & 0x3Fu);
        }
        if (cp < min_cp || cp > 0x10FFFFuL || (cp >= 0xD800uL && cp <= 0xDFFFuL)) {
            return false;
        }
        i += extra + 1;
    }
    return true;
}

// Strict signed-decimal parse matching Rust's i64/isize parse.
inline bool parse_i64(const std::uint8_t* s, std::size_t n, long long& out) {
    std::size_t i = 0;
    bool neg = false;
    unsigned long long acc = 0;
    if (n == 0) {
        return false;
    }
    if (s[0] == '+' || s[0] == '-') {
        neg = (s[0] == '-');
        i = 1;
        if (i == n) {
            return false;
        }
    }
    for (; i < n; ++i) {
        if (s[i] < '0' || s[i] > '9') {
            return false;
        }
        unsigned int d = static_cast<unsigned int>(s[i] - '0');
        if (acc > (18446744073709551615ULL - d) / 10ULL) {
            return false;
        }
        acc = acc * 10ULL + d;
    }
    if (neg) {
        if (acc > 9223372036854775808ULL) {
            return false;
        }
        out = (acc == 9223372036854775808ULL) ? (-9223372036854775807LL - 1)
                                              : -static_cast<long long>(acc);
    } else {
        if (acc > 9223372036854775807ULL) {
            return false;
        }
        out = static_cast<long long>(acc);
    }
    return true;
}

inline bool is_ascii_ws(std::uint8_t c) {
    return c == ' ' || c == '\t' || c == '\n' || c == '\r' || c == '\f';
}

inline void append_str(std::vector<std::uint8_t>& out, const std::string& s) {
    out.insert(out.end(), s.begin(), s.end());
}

// Recursive encode; returns false only on a simple string containing CR/LF.
inline bool encode_into(const Value& v, std::vector<std::uint8_t>& out) {
    switch (v.type) {
        case Type::SimpleString:
            if (v.simple.find('\r') != std::string::npos ||
                v.simple.find('\n') != std::string::npos) {
                return false;
            }
            out.push_back('+');
            append_str(out, v.simple);
            append_str(out, "\r\n");
            return true;
        case Type::Error:
            out.push_back('-');
            append_str(out, v.error_message);
            append_str(out, "\r\n");
            return true;
        case Type::Integer:
            out.push_back(':');
            append_str(out, std::to_string(v.integer));
            append_str(out, "\r\n");
            return true;
        case Type::BulkString:
            if (v.bulk_null) {
                append_str(out, "$-1\r\n");
            } else {
                out.push_back('$');
                append_str(out, std::to_string(v.bulk.size()));
                append_str(out, "\r\n");
                out.insert(out.end(), v.bulk.begin(), v.bulk.end());
                append_str(out, "\r\n");
            }
            return true;
        case Type::Array:
            if (v.array_null) {
                append_str(out, "*-1\r\n");
            } else {
                out.push_back('*');
                append_str(out, std::to_string(v.array.size()));
                append_str(out, "\r\n");
                for (const Value& item : v.array) {
                    if (!encode_into(item, out)) {
                        return false;
                    }
                }
            }
            return true;
    }
    return true;
}

// Find the first CRLF; on success set line_len (bytes before it) and consumed.
inline bool read_line(const std::uint8_t* buf, std::size_t len,
                      std::size_t& line_len, std::size_t& consumed) {
    if (len < 2) {
        return false;
    }
    for (std::size_t i = 0; i + 1 < len; ++i) {
        if (buf[i] == '\r' && buf[i + 1] == '\n') {
            line_len = i;
            consumed = i + 2;
            return true;
        }
    }
    return false;
}

}  // namespace detail

// ---- encoding ---------------------------------------------------------

// Serialize `value`; std::nullopt iff a simple string contained CR or LF.
inline std::optional<std::vector<std::uint8_t>> encode(const Value& value) {
    std::vector<std::uint8_t> out;
    if (!detail::encode_into(value, out)) {
        return std::nullopt;
    }
    return out;
}

// ---- decoding ---------------------------------------------------------

struct DecodeResult {
    enum class Status { Value, Incomplete, Error };
    Status status = Status::Incomplete;
    Value value;              // valid iff status == Value
    std::size_t consumed = 0; // valid iff status == Value
    std::string error;        // message iff status == Error

    static DecodeResult make_value(Value v, std::size_t n) {
        DecodeResult r;
        r.status = Status::Value;
        r.value = std::move(v);
        r.consumed = n;
        return r;
    }
    static DecodeResult incomplete() {
        DecodeResult r;
        r.status = Status::Incomplete;
        return r;
    }
    static DecodeResult make_error(std::string msg) {
        DecodeResult r;
        r.status = Status::Error;
        r.error = std::move(msg);
        return r;
    }
    bool is_value() const { return status == Status::Value; }
    bool is_incomplete() const { return status == Status::Incomplete; }
    bool is_error() const { return status == Status::Error; }
};

namespace detail {

inline DecodeResult decode_one(const std::uint8_t* buf, std::size_t len);

inline DecodeResult decode_bulk(const std::uint8_t* buf, std::size_t len) {
    std::size_t line_len, cons;
    long long length;
    if (!read_line(buf + 1, len - 1, line_len, cons)) {
        return DecodeResult::incomplete();
    }
    if (!utf8_valid(buf + 1, line_len) || !parse_i64(buf + 1, line_len, length)) {
        return DecodeResult::make_error("invalid RESP bulk string length");
    }
    if (length == -1) {
        return DecodeResult::make_value(Value::bulk_string_null(), cons + 1);
    }
    if (length < -1) {
        return DecodeResult::make_error("bulk string length cannot be negative");
    }
    std::size_t blen = static_cast<std::size_t>(length);
    std::size_t body_start = 1 + cons;
    if (blen > SIZE_MAX - body_start) {
        return DecodeResult::incomplete();
    }
    std::size_t body_end = body_start + blen;
    if (body_end > SIZE_MAX - 2) {
        return DecodeResult::incomplete();
    }
    std::size_t tail_end = body_end + 2;
    if (len < tail_end) {
        return DecodeResult::incomplete();
    }
    if (!(buf[body_end] == '\r' && buf[body_end + 1] == '\n')) {
        return DecodeResult::make_error("missing trailing CRLF after bulk body");
    }
    return DecodeResult::make_value(
        Value::bulk_string(
            std::vector<std::uint8_t>(buf + body_start, buf + body_end)),
        tail_end);
}

inline DecodeResult decode_array(const std::uint8_t* buf, std::size_t len) {
    std::size_t line_len, cons;
    long long count;
    if (!read_line(buf + 1, len - 1, line_len, cons)) {
        return DecodeResult::incomplete();
    }
    if (!utf8_valid(buf + 1, line_len) || !parse_i64(buf + 1, line_len, count)) {
        return DecodeResult::make_error("invalid RESP array length");
    }
    if (count == -1) {
        return DecodeResult::make_value(Value::make_array_null(), cons + 1);
    }
    if (count < -1) {
        return DecodeResult::make_error("array length cannot be negative");
    }
    std::size_t n = static_cast<std::size_t>(count);
    std::size_t offset = cons + 1;
    std::vector<Value> values;
    for (std::size_t i = 0; i < n; ++i) {
        DecodeResult r = decode_one(buf + offset, len - offset);
        if (r.is_value()) {
            values.push_back(std::move(r.value));
            offset += r.consumed;
        } else {
            return r; // Incomplete or Error, propagated
        }
    }
    return DecodeResult::make_value(Value::make_array(std::move(values)), offset);
}

inline DecodeResult decode_inline(const std::uint8_t* buf, std::size_t len) {
    std::size_t line_len, cons, i = 0;
    if (!read_line(buf, len, line_len, cons)) {
        return DecodeResult::incomplete();
    }
    std::vector<Value> tokens;
    while (i < line_len) {
        while (i < line_len && is_ascii_ws(buf[i])) {
            ++i;
        }
        if (i >= line_len) {
            break;
        }
        std::size_t start = i;
        while (i < line_len && !is_ascii_ws(buf[i])) {
            ++i;
        }
        tokens.push_back(Value::bulk_string(
            std::vector<std::uint8_t>(buf + start, buf + i)));
    }
    return DecodeResult::make_value(Value::make_array(std::move(tokens)), cons);
}

inline DecodeResult decode_one(const std::uint8_t* buf, std::size_t len) {
    if (len == 0) {
        return DecodeResult::incomplete();
    }
    std::uint8_t prefix = buf[0];
    if (prefix == '+' || prefix == '-' || prefix == ':') {
        std::size_t line_len, cons;
        if (!read_line(buf + 1, len - 1, line_len, cons)) {
            return DecodeResult::incomplete();
        }
        if (!utf8_valid(buf + 1, line_len)) {
            return DecodeResult::make_error("invalid UTF-8 in RESP line");
        }
        std::string s(reinterpret_cast<const char*>(buf + 1), line_len);
        if (prefix == ':') {
            long long iv;
            if (!parse_i64(buf + 1, line_len, iv)) {
                return DecodeResult::make_error("invalid RESP integer");
            }
            return DecodeResult::make_value(Value::make_integer(iv), cons + 1);
        }
        Value v = (prefix == '+') ? Value::simple_string(std::move(s))
                                  : Value::make_error(std::move(s));
        return DecodeResult::make_value(std::move(v), cons + 1);
    }
    if (prefix == '$') {
        return decode_bulk(buf, len);
    }
    if (prefix == '*') {
        return decode_array(buf, len);
    }
    return decode_inline(buf, len);
}

}  // namespace detail

// Decode one frame from `buf` (`len` bytes).
inline DecodeResult decode(const std::uint8_t* buf, std::size_t len) {
    return detail::decode_one(buf, len);
}
inline DecodeResult decode(const std::vector<std::uint8_t>& buf) {
    return detail::decode_one(buf.data(), buf.size());
}

struct DecodeAllResult {
    bool ok = true;
    std::vector<Value> values;
    std::size_t consumed = 0;
    std::string error;
};

// Decode as many whole frames as `buf` contains, stopping at the first
// incomplete frame; ok == false with `error` set on a malformed frame.
inline DecodeAllResult decode_all(const std::uint8_t* buf, std::size_t len) {
    DecodeAllResult result;
    std::size_t offset = 0;
    while (offset < len) {
        DecodeResult r = detail::decode_one(buf + offset, len - offset);
        if (r.is_value()) {
            result.values.push_back(std::move(r.value));
            offset += r.consumed;
        } else if (r.is_incomplete()) {
            break;
        } else {
            result.ok = false;
            result.error = std::move(r.error);
            result.values.clear();
            return result;
        }
    }
    result.consumed = offset;
    return result;
}
inline DecodeAllResult decode_all(const std::vector<std::uint8_t>& buf) {
    return decode_all(buf.data(), buf.size());
}

// ---- streaming decoder ------------------------------------------------

// Accumulates bytes across feeds and queues whole decoded messages; latches an
// error once a malformed frame is seen.
class Decoder {
public:
    void feed(const std::uint8_t* data, std::size_t len) {
        buffer_.insert(buffer_.end(), data, data + len);
        drain();
    }
    void feed(const std::vector<std::uint8_t>& data) {
        feed(data.data(), data.size());
    }
    void feed(const std::string& data) {
        feed(reinterpret_cast<const std::uint8_t*>(data.data()), data.size());
    }

    bool has_message() const { return !queue_.empty(); }
    bool has_error() const { return error_.has_value(); }

    // Pop the next message; nullopt if in an error state or the queue is empty.
    std::optional<Value> get_message() {
        if (error_ || queue_.empty()) {
            return std::nullopt;
        }
        Value v = std::move(queue_.front());
        queue_.pop_front();
        return v;
    }

    // Feed, then hand off every currently-queued message; ok == false if the
    // decoder is (or becomes) in an error state.
    DecodeAllResult decode_all(const std::uint8_t* data, std::size_t len) {
        feed(data, len);
        DecodeAllResult result;
        if (error_) {
            result.ok = false;
            result.error = *error_;
            return result;
        }
        while (!queue_.empty()) {
            result.values.push_back(std::move(queue_.front()));
            queue_.pop_front();
        }
        return result;
    }
    DecodeAllResult decode_all(const std::string& data) {
        return decode_all(reinterpret_cast<const std::uint8_t*>(data.data()),
                          data.size());
    }

private:
    void drain() {
        if (error_) {
            return;
        }
        for (;;) {
            DecodeResult r = detail::decode_one(buffer_.data(), buffer_.size());
            if (r.is_value()) {
                queue_.push_back(std::move(r.value));
                buffer_.erase(buffer_.begin(),
                              buffer_.begin() +
                                  static_cast<std::ptrdiff_t>(r.consumed));
            } else if (r.is_incomplete()) {
                break;
            } else {
                error_ = r.error;
                break;
            }
        }
    }

    std::vector<std::uint8_t> buffer_;
    std::deque<Value> queue_;
    std::optional<std::string> error_;
};

}  // namespace resp
}  // namespace ca

#endif  // CA_RESP_PROTOCOL_HPP
