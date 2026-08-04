// protobuf.hpp — a zero-dependency Protocol Buffers *wire-format* codec, C++17.
// ===========================================================================
//
// A faithful, header-only port of the Rust `protobuf` crate: just the wire
// format (https://protobuf.dev/programming-guides/encoding/) — enough to encode
// and decode messages byte-for-byte compatibly with Google's protobuf. No
// `.proto` compiler and no codegen: callers hand-write the few encode/decode
// calls for the messages they need.
//
// ── The wire format in one paragraph ────────────────────────────────────────
// A message is a flat sequence of (tag, value) records with no framing. Each
// tag is a varint whose low 3 bits are the wire type and whose upper bits are
// the field number:  tag = (field_number << 3) | wire_type.
//
//   0  Varint           one LEB128 varint (ints, bools, enums)
//   1  Fixed64          8 little-endian bytes
//   2  LengthDelimited  a varint length n, then n bytes (string/bytes/message)
//   5  Fixed32          4 little-endian bytes
//
// Errors are reported as C++ exceptions (`ca::protobuf::Error`); encoding never
// fails. Decoded length-delimited payloads are returned as views that borrow
// the reader's input buffer (no copy).
//
// Pure ISO C++17 — no <cmath>, no compiler extensions, no 128-bit integers.
#ifndef PROTOBUF_HPP
#define PROTOBUF_HPP

#include <cstddef>
#include <cstdint>
#include <optional>
#include <stdexcept>
#include <string>
#include <string_view>
#include <vector>

namespace ca {
namespace protobuf {

// Wire types (the low 3 bits of a field tag). Deprecated group types (3, 4)
// are not represented and are rejected on decode.
enum class WireType : std::uint8_t {
    Varint = 0,
    Fixed64 = 1,
    LengthDelimited = 2,
    Fixed32 = 5
};

// The kind of a decode error.
enum class ErrorKind {
    TruncatedVarint,   // varint past buffer end, or over 10 bytes (overflow)
    UnexpectedEof,     // a field claimed more bytes than remain
    UnknownWireType,   // a wire type this codec does not implement (3,4,6,7)
    ZeroFieldNumber    // field number 0, which protobuf forbids
};

// A decode error. Encoding cannot fail, so it is only ever thrown by Reader.
class Error : public std::runtime_error {
   public:
    explicit Error(ErrorKind kind)
        : std::runtime_error(message_for(kind)), kind_(kind) {}
    ErrorKind kind() const noexcept { return kind_; }

   private:
    static const char *message_for(ErrorKind kind) {
        switch (kind) {
            case ErrorKind::TruncatedVarint:
                return "truncated or over-long varint";
            case ErrorKind::UnexpectedEof:
                return "unexpected end of protobuf buffer";
            case ErrorKind::UnknownWireType:
                return "unknown protobuf wire type";
            case ErrorKind::ZeroFieldNumber:
                return "protobuf field number 0 is illegal";
        }
        return "unknown protobuf error";
    }
    ErrorKind kind_;
};

// ── Writer ──────────────────────────────────────────────────────────────────

// Builds a message by appending fields in call order. The output is exactly the
// concatenation of the fields written; no framing is inserted.
class Writer {
   public:
    Writer() = default;

    // Borrow the current bytes.
    const std::vector<std::uint8_t> &bytes() const { return buf_; }
    // Consume the writer and return the encoded message bytes.
    std::vector<std::uint8_t> into_bytes() { return std::move(buf_); }

    // Append a raw LEB128 varint (no tag).
    Writer &write_varint(std::uint64_t value) {
        for (;;) {
            std::uint8_t byte = static_cast<std::uint8_t>(value & 0x7f);
            value >>= 7;
            if (value == 0) {
                buf_.push_back(byte);
                break;
            }
            buf_.push_back(static_cast<std::uint8_t>(byte | 0x80));
        }
        return *this;
    }

    // A varint-typed field (int32/64, uint32/64, bool, enum).
    Writer &varint(std::uint32_t field, std::uint64_t value) {
        write_tag(field, WireType::Varint);
        return write_varint(value);
    }
    // A length-delimited field carrying arbitrary bytes.
    Writer &bytes(std::uint32_t field, const std::uint8_t *value,
                  std::size_t len) {
        write_tag(field, WireType::LengthDelimited);
        write_varint(static_cast<std::uint64_t>(len));
        buf_.insert(buf_.end(), value, value + len);
        return *this;
    }
    Writer &bytes(std::uint32_t field, const std::vector<std::uint8_t> &value) {
        return bytes(field, value.data(), value.size());
    }
    // A length-delimited string field (UTF-8 bytes of the view).
    Writer &string(std::uint32_t field, std::string_view value) {
        return bytes(field, reinterpret_cast<const std::uint8_t *>(value.data()),
                     value.size());
    }
    // A length-delimited field carrying an already-encoded embedded message.
    Writer &message(std::uint32_t field,
                    const std::vector<std::uint8_t> &encoded) {
        return bytes(field, encoded.data(), encoded.size());
    }
    Writer &message(std::uint32_t field, const std::uint8_t *encoded,
                    std::size_t len) {
        return bytes(field, encoded, len);
    }
    // A fixed32 / sfixed32 / float field (4 little-endian bytes).
    Writer &fixed32(std::uint32_t field, std::uint32_t value) {
        write_tag(field, WireType::Fixed32);
        for (int i = 0; i < 4; ++i)
            buf_.push_back(static_cast<std::uint8_t>((value >> (i * 8)) & 0xff));
        return *this;
    }
    // A fixed64 / sfixed64 / double field (8 little-endian bytes).
    Writer &fixed64(std::uint32_t field, std::uint64_t value) {
        write_tag(field, WireType::Fixed64);
        for (int i = 0; i < 8; ++i)
            buf_.push_back(static_cast<std::uint8_t>((value >> (i * 8)) & 0xff));
        return *this;
    }

   private:
    void write_tag(std::uint32_t field, WireType wire) {
        write_varint((static_cast<std::uint64_t>(field) << 3) |
                     static_cast<std::uint64_t>(wire));
    }
    std::vector<std::uint8_t> buf_;
};

// ── Reader ──────────────────────────────────────────────────────────────────

// A non-owning view of bytes borrowed from the reader's input buffer. (We roll
// our own rather than use std::basic_string_view<std::uint8_t>, whose
// char_traits<unsigned char> is non-standard and rejected under -Werror.)
struct ByteView {
    const std::uint8_t *ptr = nullptr;
    std::size_t len = 0;

    ByteView() = default;
    ByteView(const std::uint8_t *p, std::size_t n) : ptr(p), len(n) {}

    const std::uint8_t *data() const { return ptr; }
    std::size_t size() const { return len; }
    bool empty() const { return len == 0; }
    std::uint8_t operator[](std::size_t i) const { return ptr[i]; }

    // Content equality (matches Rust's `PartialEq` on the borrowed slice).
    bool operator==(const ByteView &o) const {
        if (len != o.len) return false;
        for (std::size_t i = 0; i < len; ++i)
            if (ptr[i] != o.ptr[i]) return false;
        return true;
    }
    bool operator!=(const ByteView &o) const { return !(*this == o); }
};

struct Value {
    WireType kind{};
    std::uint64_t varint = 0;   // Varint
    std::uint64_t fixed64 = 0;  // Fixed64
    std::uint32_t fixed32 = 0;  // Fixed32
    ByteView bytes{};           // LengthDelimited (borrows input)

    // The varint payload, or nullopt for other wire types.
    std::optional<std::uint64_t> as_varint() const {
        if (kind == WireType::Varint) return varint;
        return std::nullopt;
    }
    // The length-delimited payload, or nullopt for other wire types.
    std::optional<ByteView> as_bytes() const {
        if (kind == WireType::LengthDelimited) return bytes;
        return std::nullopt;
    }

    bool operator==(const Value &o) const {
        if (kind != o.kind) return false;
        switch (kind) {
            case WireType::Varint: return varint == o.varint;
            case WireType::Fixed64: return fixed64 == o.fixed64;
            case WireType::Fixed32: return fixed32 == o.fixed32;
            case WireType::LengthDelimited: return bytes == o.bytes;
        }
        return false;
    }
    bool operator!=(const Value &o) const { return !(*this == o); }
};

// One decoded field: its (1-based) number and its value.
struct Field {
    std::uint32_t number = 0;
    Value value{};
    bool operator==(const Field &o) const {
        return number == o.number && value == o.value;
    }
    bool operator!=(const Field &o) const { return !(*this == o); }
};

// A cursor over an encoded message. Iterate with next_field(); unknown field
// numbers are yielded too so callers can skip them (forward compatibility).
class Reader {
   public:
    Reader(const std::uint8_t *data, std::size_t len) : data_(data), len_(len) {}
    explicit Reader(const std::vector<std::uint8_t> &v)
        : data_(v.data()), len_(v.size()) {}

    // Whether every field has been consumed.
    bool is_empty() const { return pos_ >= len_; }

    // Read the next field, or nullopt at end of message. Throws `Error` on
    // malformed input.
    std::optional<Field> next_field() {
        if (is_empty()) return std::nullopt;
        std::uint64_t tag = read_varint();
        std::uint32_t number = static_cast<std::uint32_t>(tag >> 3);
        if (number == 0) throw Error(ErrorKind::ZeroFieldNumber);

        Field f;
        f.number = number;
        switch (tag & 0x7) {
            case 0:
                f.value.kind = WireType::Varint;
                f.value.varint = read_varint();
                break;
            case 1: {
                const std::uint8_t *b = read_slice(8);
                f.value.kind = WireType::Fixed64;
                f.value.fixed64 = load_u64_le(b);
                break;
            }
            case 2: {
                std::uint64_t vlen = read_varint();
                if (vlen > static_cast<std::uint64_t>(len_ - pos_))
                    throw Error(ErrorKind::UnexpectedEof);
                const std::uint8_t *b = read_slice(static_cast<std::size_t>(vlen));
                f.value.kind = WireType::LengthDelimited;
                f.value.bytes = ByteView(b, static_cast<std::size_t>(vlen));
                break;
            }
            case 5: {
                const std::uint8_t *b = read_slice(4);
                f.value.kind = WireType::Fixed32;
                f.value.fixed32 = load_u32_le(b);
                break;
            }
            default: throw Error(ErrorKind::UnknownWireType);
        }
        return f;
    }

   private:
    std::uint64_t read_varint() {
        std::uint64_t result = 0;
        // A u64 needs at most ceil(64/7) = 10 varint bytes; more means overflow.
        for (int shift = 0; shift < 64; shift += 7) {
            if (pos_ >= len_) throw Error(ErrorKind::TruncatedVarint);
            std::uint8_t byte = data_[pos_++];
            result |= static_cast<std::uint64_t>(byte & 0x7f) << shift;
            if ((byte & 0x80) == 0) return result;
        }
        throw Error(ErrorKind::TruncatedVarint);
    }
    const std::uint8_t *read_slice(std::size_t len) {
        if (len > len_ - pos_) throw Error(ErrorKind::UnexpectedEof);
        const std::uint8_t *p = data_ + pos_;
        pos_ += len;
        return p;
    }
    static std::uint64_t load_u64_le(const std::uint8_t *p) {
        std::uint64_t r = 0;
        for (int i = 0; i < 8; ++i)
            r |= static_cast<std::uint64_t>(p[i]) << (i * 8);
        return r;
    }
    static std::uint32_t load_u32_le(const std::uint8_t *p) {
        return static_cast<std::uint32_t>(p[0]) |
               (static_cast<std::uint32_t>(p[1]) << 8) |
               (static_cast<std::uint32_t>(p[2]) << 16) |
               (static_cast<std::uint32_t>(p[3]) << 24);
    }

    const std::uint8_t *data_;
    std::size_t len_;
    std::size_t pos_ = 0;
};

}  // namespace protobuf
}  // namespace ca

#endif  // PROTOBUF_HPP
