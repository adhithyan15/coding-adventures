// Tests for the C++ canonical-cbor library, using the header-only iso_test.h
// harness (pure ISO). Byte vectors and error expectations mirror the Rust
// crate's own unit tests one-for-one.
#include "iso_test.h"

#include <cstdint>
#include <string>
#include <utility>
#include <vector>

#include "canonical_cbor.hpp"

namespace cc = ca::canonical_cbor;
using cc::CborError;
using cc::CborValue;
using Bytes = std::vector<std::uint8_t>;

// True iff encode(v) equals exp.
static bool enc_eq(const CborValue& v, const Bytes& exp) {
    return cc::encode(v) == exp;
}

// Return the CborError thrown by decode(b), or -1 (as int) if it did not throw.
static int dec_err(const Bytes& b) {
    try {
        cc::decode(b);
        return -1;
    } catch (const cc::CborException& e) {
        return static_cast<int>(e.error());
    }
}

static CborValue text(const std::string& s) { return CborValue::text_string(s); }
static CborValue uint_(std::uint64_t n) { return CborValue::unsigned_val(n); }

int main() {
    // ── smallest-form unsigned ────────────────────────────────────────────
    for (std::uint64_t n = 0; n <= 23; n++)
        ISO_CHECK(enc_eq(uint_(n), Bytes{static_cast<std::uint8_t>(n)}));
    ISO_CHECK(enc_eq(uint_(24), Bytes{0x18, 24}));
    ISO_CHECK(enc_eq(uint_(255), Bytes{0x18, 255}));
    ISO_CHECK(enc_eq(uint_(256), Bytes{0x19, 0x01, 0x00}));
    ISO_CHECK(enc_eq(uint_(65536), Bytes{0x1A, 0x00, 0x01, 0x00, 0x00}));
    ISO_CHECK(enc_eq(uint_(0xFFFFFFFFFFFFFFFFull),
                     Bytes{0x1B, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF}));

    // ── decoder rejects non-minimal integer forms ─────────────────────────
    ISO_CHECK(dec_err(Bytes{0x18, 0x05}) == (int)CborError::NonMinimalInteger);
    ISO_CHECK(dec_err(Bytes{0x19, 0x00, 0xFF}) == (int)CborError::NonMinimalInteger);
    ISO_CHECK(dec_err(Bytes{0x1A, 0x00, 0x00, 0xFF, 0xFF}) ==
              (int)CborError::NonMinimalInteger);
    ISO_CHECK(dec_err(Bytes{0x1B, 0, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF}) ==
              (int)CborError::NonMinimalInteger);

    // ── negatives ─────────────────────────────────────────────────────────
    ISO_CHECK(enc_eq(CborValue::negative(0), Bytes{0x20}));   // -1
    ISO_CHECK(enc_eq(CborValue::negative(23), Bytes{0x37}));  // -24
    ISO_CHECK(enc_eq(CborValue::negative(24), Bytes{0x38, 24}));  // -25

    // ── bytes / text ──────────────────────────────────────────────────────
    ISO_CHECK(enc_eq(CborValue::byte_string({}), Bytes{0x40}));
    ISO_CHECK(enc_eq(CborValue::byte_string({1, 2, 3, 4}), Bytes{0x44, 1, 2, 3, 4}));
    ISO_CHECK(enc_eq(text("abc"), Bytes{0x63, 'a', 'b', 'c'}));
    ISO_CHECK(dec_err(Bytes{0x61, 0xFF}) == (int)CborError::InvalidUtf8);

    // ── arrays ────────────────────────────────────────────────────────────
    ISO_CHECK(enc_eq(CborValue::arr({}), Bytes{0x80}));
    ISO_CHECK(enc_eq(CborValue::arr({uint_(1), uint_(2), uint_(3)}),
                     Bytes{0x83, 0x01, 0x02, 0x03}));
    ISO_CHECK(enc_eq(CborValue::arr({uint_(3), uint_(2), uint_(1)}),
                     Bytes{0x83, 0x03, 0x02, 0x01}));

    // ── maps: canonical length-first ordering ─────────────────────────────
    {
        auto m1 = CborValue::mapping({{text("a"), uint_(1)}, {text("bb"), uint_(2)}});
        auto m2 = CborValue::mapping({{text("bb"), uint_(2)}, {text("a"), uint_(1)}});
        auto b1 = cc::encode(m1);
        ISO_CHECK(b1 == cc::encode(m2));  // order-independent
        ISO_CHECK(b1[0] == 0xA2 && b1[1] == 0x61 && b1[2] == 'a');

        // tie broken lex: "a" before "b"
        auto m = CborValue::mapping({{text("b"), uint_(2)}, {text("a"), uint_(1)}});
        ISO_CHECK((cc::encode(m) == Bytes{0xA2, 0x61, 'a', 0x01, 0x61, 'b', 0x02}));

        // decoder accepts canonical
        auto dv = cc::decode(Bytes{0xA2, 0x61, 'a', 0x01, 0x61, 'b', 0x02});
        ISO_CHECK(dv.type == CborValue::Type::Map && dv.map.size() == 2);
        ISO_CHECK(dv.map[0].first == text("a"));
        ISO_CHECK(dv.map[1].first == text("b"));

        // rejects non-canonical order and duplicate keys
        ISO_CHECK(dec_err(Bytes{0xA2, 0x61, 'b', 0x02, 0x61, 'a', 0x01}) ==
                  (int)CborError::NonCanonicalMapOrder);
        ISO_CHECK(dec_err(Bytes{0xA2, 0x61, 'a', 0x01, 0x61, 'a', 0x02}) ==
                  (int)CborError::NonCanonicalMapOrder);
    }

    // ── round-trip: encode -> decode -> re-encode is byte-identical ───────
    {
        auto meta = CborValue::mapping(
            {{text("v"), uint_(1)}, {text("draft"), CborValue::boolean_val(true)}});
        auto tags = CborValue::arr({text("urgent"), text("draft")});
        auto v = CborValue::mapping({
            {text("title"), text("hello world")},
            {text("count"), uint_(42)},
            {text("tags"), tags},
            {text("meta"), meta},
            {text("note"), CborValue::null()},
            {text("blob"), CborValue::byte_string({0xDE, 0xAD, 0xBE, 0xEF})},
        });
        auto bytes = cc::encode(v);
        auto back = cc::decode(bytes);
        ISO_CHECK(cc::encode(back) == bytes);  // re-encode is byte-identical
    }

    // ── tags ──────────────────────────────────────────────────────────────
    {
        auto v = CborValue::tag(0, text("2026-05-04"));
        auto bytes = cc::encode(v);
        ISO_CHECK(bytes[0] == 0xC0);
        ISO_CHECK(cc::decode(bytes) == v);

        auto big = CborValue::tag(1234567, uint_(0));
        ISO_CHECK(cc::decode(cc::encode(big)) == big);
    }

    // ── rejects indefinite / reserved / undefined / floats ────────────────
    ISO_CHECK(dec_err(Bytes{0x9F, 0x01, 0xFF}) == (int)CborError::Indefinite);
    ISO_CHECK(dec_err(Bytes{0xBF, 0x61, 'a', 0x01, 0xFF}) == (int)CborError::Indefinite);
    ISO_CHECK(dec_err(Bytes{0x1C}) == (int)CborError::Reserved);
    ISO_CHECK(dec_err(Bytes{0xF7}) == (int)CborError::UnsupportedSimple);
    ISO_CHECK(dec_err(Bytes{0xF9, 0x00, 0x00}) == (int)CborError::FloatNotSupported);
    ISO_CHECK(dec_err(Bytes{0xFA, 0, 0, 0, 0}) == (int)CborError::FloatNotSupported);
    ISO_CHECK(dec_err(Bytes{0xFB, 0, 0, 0, 0, 0, 0, 0, 0}) ==
              (int)CborError::FloatNotSupported);

    // ── trailing / EOF / truncation ───────────────────────────────────────
    ISO_CHECK(dec_err(Bytes{0x01, 0x00}) == (int)CborError::TrailingBytes);
    ISO_CHECK(dec_err(Bytes{}) == (int)CborError::UnexpectedEof);
    ISO_CHECK(dec_err(Bytes{0x18}) == (int)CborError::UnexpectedEof);
    ISO_CHECK(dec_err(Bytes{0x44, 0xAA, 0xBB}) == (int)CborError::LengthTooLarge);

    // ── stress: large array / map round-trips ─────────────────────────────
    {
        std::vector<CborValue> items;
        for (std::uint64_t i = 0; i < 1000; i++) items.push_back(uint_(i));
        auto arr = CborValue::arr(items);
        ISO_CHECK(cc::decode(cc::encode(arr)) == arr);

        std::vector<std::pair<CborValue, CborValue>> entries, rev;
        for (std::uint64_t i = 0; i < 100; i++)
            entries.emplace_back(uint_(i), uint_(i * 7));
        rev.assign(entries.rbegin(), entries.rend());
        auto ba = cc::encode(CborValue::mapping(entries));
        ISO_CHECK(ba == cc::encode(CborValue::mapping(rev)));  // deterministic
        auto dm = cc::decode(ba);
        ISO_CHECK(dm.type == CborValue::Type::Map && dm.map.size() == 100);
        for (std::uint64_t i = 0; i < 100; i++)  // canonical order 0..99
            ISO_CHECK(dm.map[i].first.type == CborValue::Type::Unsigned &&
                      dm.map[i].first.u == i);
    }

    // ── simple values round-trip ──────────────────────────────────────────
    for (const auto& v : {CborValue::boolean_val(false), CborValue::boolean_val(true),
                          CborValue::null()})
        ISO_CHECK(cc::decode(cc::encode(v)) == v);

    // ── DoS defences: depth cap and oversized lengths ─────────────────────
    {
        Bytes deep_arr(cc::MAX_DECODE_DEPTH + 10, 0x81);
        deep_arr.push_back(0x00);
        ISO_CHECK(dec_err(deep_arr) == (int)CborError::TooDeep);

        Bytes deep_tag(cc::MAX_DECODE_DEPTH + 10, 0xC6);
        deep_tag.push_back(0x00);
        ISO_CHECK(dec_err(deep_tag) == (int)CborError::TooDeep);

        Bytes at_limit(cc::MAX_DECODE_DEPTH, 0x81);
        at_limit.push_back(0x00);
        bool ok = true;
        try {
            cc::decode(at_limit);
        } catch (...) {
            ok = false;
        }
        ISO_CHECK(ok);  // exactly MAX_DECODE_DEPTH is accepted

        ISO_CHECK(dec_err(Bytes{0x9B, 0, 0, 0x01, 0, 0, 0, 0, 0, 0}) ==
                  (int)CborError::LengthTooLarge);
        ISO_CHECK(dec_err(Bytes{0xBB, 0, 0, 0x01, 0, 0, 0, 0, 0, 0, 0}) ==
                  (int)CborError::LengthTooLarge);
        ISO_CHECK(dec_err(Bytes{0x5B, 0, 0, 0x01, 0, 0, 0, 0, 0}) ==
                  (int)CborError::LengthTooLarge);
        ISO_CHECK(dec_err(Bytes{0x9B, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                                0xFF}) == (int)CborError::LengthTooLarge);
    }

    return ISO_TEST_RESULT();
}
