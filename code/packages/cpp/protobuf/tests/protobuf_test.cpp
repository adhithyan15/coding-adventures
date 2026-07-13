// Tests for protobuf, using the header-only iso_test.h harness (pure ISO).
// Vectors mirror the Rust crate's own unit tests, including the canonical
// examples from the protobuf encoding docs.
#include "iso_test.h"

#include <cstdint>
#include <optional>
#include <string_view>
#include <vector>

#include "protobuf.hpp"

namespace pb = ca::protobuf;

// A ByteView over a string literal's bytes (excluding the terminating NUL).
static pb::ByteView bv(const char *s, std::size_t n) {
    return pb::ByteView(reinterpret_cast<const std::uint8_t *>(s), n);
}

int main() {
    using pb::Field;
    using pb::Reader;
    using pb::Value;
    using pb::WireType;
    using pb::Writer;

    // ── varint round-trip at boundaries ──────────────────────────────────────
    {
        const std::uint64_t vals[] = {0u,
                                      1u,
                                      127u,
                                      128u,
                                      300u,
                                      16383u,
                                      16384u,
                                      0xFFFFFFFFull,
                                      0xFFFFFFFFFFFFFFFFull};
        for (std::uint64_t v : vals) {
            Writer w;
            w.varint(1, v);
            auto bytes = w.into_bytes();
            Reader r(bytes);
            auto f = r.next_field();
            ISO_CHECK(f.has_value());
            ISO_CHECK(f->value.as_varint() == std::optional<std::uint64_t>(v));
            ISO_CHECK(r.is_empty());
        }
    }

    // ── varint 300 matches the spec bytes ────────────────────────────────────
    {
        Writer w;
        w.write_varint(300);
        auto bytes = w.into_bytes();
        std::vector<std::uint8_t> expect = {0xac, 0x02};
        ISO_CHECK(bytes == expect);
    }

    // ── field 1, varint 150 → tag 0x08, then 0x96 0x01 ───────────────────────
    {
        Writer w;
        w.varint(1, 150);
        std::vector<std::uint8_t> expect = {0x08, 0x96, 0x01};
        ISO_CHECK(w.into_bytes() == expect);
    }

    // ── all wire types round-trip ────────────────────────────────────────────
    {
        Writer w;
        std::vector<std::uint8_t> payload = {0xde, 0xad, 0xbe, 0xef};
        w.varint(1, 150)
            .string(2, "testing")
            .bytes(3, payload)
            .fixed32(4, 0x12345678u)
            .fixed64(5, 0x0102030405060708ull);
        auto encoded = w.into_bytes();

        Reader r(encoded);
        {
            auto f = r.next_field();
            ISO_CHECK(f.has_value() &&
                      *f == (Field{1, Value{WireType::Varint, 150}}));
        }
        {
            auto f = r.next_field();
            ISO_CHECK(f.has_value() && f->number == 2 &&
                      f->value.as_bytes() ==
                          std::optional<pb::ByteView>(bv("testing", 7)));
        }
        {
            auto f = r.next_field();
            ISO_CHECK(f.has_value() && f->number == 3 &&
                      f->value.as_bytes().has_value() &&
                      f->value.bytes.size() == 4 &&
                      f->value.bytes[0] == 0xde && f->value.bytes[3] == 0xef);
        }
        {
            auto f = r.next_field();
            ISO_CHECK(f.has_value() && f->number == 4 &&
                      f->value.kind == WireType::Fixed32 &&
                      f->value.fixed32 == 0x12345678u);
        }
        {
            auto f = r.next_field();
            ISO_CHECK(f.has_value() && f->number == 5 &&
                      f->value.kind == WireType::Fixed64 &&
                      f->value.fixed64 == 0x0102030405060708ull);
        }
        ISO_CHECK(!r.next_field().has_value());  // clean end
    }

    // ── reader skips unknown fields ──────────────────────────────────────────
    {
        Writer w;
        w.varint(1, 11).varint(7, 999).string(2, "keep");
        auto encoded = w.into_bytes();

        Reader r(encoded);
        std::vector<std::uint32_t> kept;
        while (auto f = r.next_field()) {
            if (f->number == 1 || f->number == 2) kept.push_back(f->number);
        }
        std::vector<std::uint32_t> expect = {1, 2};
        ISO_CHECK(kept == expect);
    }

    // ── nested message round-trip ────────────────────────────────────────────
    {
        std::vector<std::uint8_t> inner = [] {
            Writer w;
            w.string(1, "inner");
            return w.into_bytes();
        }();
        Writer outer;
        outer.message(1, inner).varint(2, 5);
        auto encoded = outer.into_bytes();

        Reader r(encoded);
        auto f = r.next_field();
        ISO_CHECK(f.has_value() && f->number == 1);
        auto inner_bytes = f->value.as_bytes();
        ISO_CHECK(inner_bytes.has_value());
        Reader inner_r(inner_bytes->data(), inner_bytes->size());
        auto g = inner_r.next_field();
        ISO_CHECK(g.has_value() &&
                  g->value.as_bytes() ==
                      std::optional<pb::ByteView>(bv("inner", 5)));
    }

    // ── error cases (Reader throws pb::Error) ────────────────────────────────
    {
        std::vector<std::uint8_t> truncated = {0x80};  // continuation, no next
        Reader r(truncated);
        bool threw = false;
        try {
            r.next_field();
        } catch (const pb::Error &e) {
            threw = (e.kind() == pb::ErrorKind::TruncatedVarint);
        }
        ISO_CHECK(threw);
    }
    {
        std::vector<std::uint8_t> overlong = {0x0a, 0x64};  // claims 100 bytes
        Reader r(overlong);
        bool threw = false;
        try {
            r.next_field();
        } catch (const pb::Error &e) {
            threw = (e.kind() == pb::ErrorKind::UnexpectedEof);
        }
        ISO_CHECK(threw);
    }
    {
        std::vector<std::uint8_t> zero_field = {0x00};  // field 0, varint
        Reader r(zero_field);
        bool threw = false;
        try {
            r.next_field();
        } catch (const pb::Error &e) {
            threw = (e.kind() == pb::ErrorKind::ZeroFieldNumber);
        }
        ISO_CHECK(threw);
    }
    {
        std::vector<std::uint8_t> group = {0x0b};  // field 1, wire type 3
        Reader r(group);
        bool threw = false;
        try {
            r.next_field();
        } catch (const pb::Error &e) {
            threw = (e.kind() == pb::ErrorKind::UnknownWireType);
        }
        ISO_CHECK(threw);
    }

    // ── as_varint / as_bytes on the wrong type return nullopt ────────────────
    {
        Value v{WireType::Varint, 42};
        ISO_CHECK(v.as_varint() == std::optional<std::uint64_t>(42));
        ISO_CHECK(!v.as_bytes().has_value());
        Value b{WireType::LengthDelimited};
        b.bytes = bv("x", 1);
        ISO_CHECK(!b.as_varint().has_value());
        ISO_CHECK(b.as_bytes().has_value());
    }

    return ISO_TEST_RESULT();
}
