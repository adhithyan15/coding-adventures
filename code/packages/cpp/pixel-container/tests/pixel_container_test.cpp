// Tests for the C++ pixel-container, using the header-only iso_test.h harness
// (pure ISO). Vectors mirror the Rust crate's own unit tests.
#include "iso_test.h"

#include <array>
#include <cstdint>
#include <stdexcept>
#include <string>
#include <vector>

#include "pixel_container.hpp"

using ca::ImageCodec;
using ca::PixelContainer;
using Bytes = std::vector<std::uint8_t>;
using Rgba = std::array<std::uint8_t, 4>;

// A minimal stub codec mirroring the Rust test: header [w, h, 0, 0] then raw
// RGBA bytes.
class StubCodec : public ImageCodec {
public:
    std::string mime_type() const override { return "image/x-stub"; }
    Bytes encode(const PixelContainer& c) const override {
        Bytes out = {(std::uint8_t)c.width, (std::uint8_t)c.height, 0, 0};
        out.insert(out.end(), c.data.begin(), c.data.end());
        return out;
    }
    PixelContainer decode(const Bytes& bytes) const override {
        if (bytes.size() < 4) {
            throw std::invalid_argument("stub: too short");
        }
        Bytes data(bytes.begin() + 4, bytes.end());
        return PixelContainer::from_data(bytes[0], bytes[1], std::move(data));
    }
};

int main() {
    // ── new: size + all zeros ────────────────────────────────────────────
    {
        PixelContainer p(10, 20);
        ISO_CHECK_EQ_UINT(p.width, 10u);
        ISO_CHECK_EQ_UINT(p.height, 20u);
        ISO_CHECK_EQ_UINT(p.byte_count(), 10u * 20u * 4u);
        ISO_CHECK_EQ_UINT(p.pixel_count(), 200u);
        bool all_zero = true;
        for (std::uint8_t b : p.data) {
            if (b != 0) all_zero = false;
        }
        ISO_CHECK(all_zero);
    }

    // ── zero dimensions ──────────────────────────────────────────────────
    {
        PixelContainer p(0, 0);
        ISO_CHECK_EQ_UINT(p.byte_count(), 0u);
    }

    // ── from_data + wrong-length throws ──────────────────────────────────
    {
        PixelContainer p = PixelContainer::from_data(1, 1, {255, 128, 64, 32});
        ISO_CHECK((p.pixel_at(0, 0) == Rgba{255, 128, 64, 32}));
        bool threw = false;
        try {
            (void)PixelContainer::from_data(1, 1, {1, 2, 3});
        } catch (const std::invalid_argument&) {
            threw = true;
        }
        ISO_CHECK(threw);
    }

    // ── set/get + offset ─────────────────────────────────────────────────
    {
        PixelContainer p(5, 5);
        p.set_pixel(3, 2, 1, 2, 3, 4);  // offset = (2*5+3)*4 = 52
        ISO_CHECK((p.pixel_at(3, 2) == Rgba{1, 2, 3, 4}));
        ISO_CHECK(p.data[52] == 1 && p.data[55] == 4);
    }

    // ── out of bounds ────────────────────────────────────────────────────
    {
        PixelContainer p(4, 4);
        ISO_CHECK((p.pixel_at(4, 0) == Rgba{0, 0, 0, 0}));
        ISO_CHECK((p.pixel_at(100, 100) == Rgba{0, 0, 0, 0}));
        p.set_pixel(10, 10, 255, 255, 255, 255);  // no-op
        bool untouched = true;
        for (std::uint8_t b : p.data) {
            if (b != 0) untouched = false;
        }
        ISO_CHECK(untouched);
    }

    // ── fill ─────────────────────────────────────────────────────────────
    {
        PixelContainer p(3, 3);
        p.fill(255, 128, 0, 255);
        bool ok = true;
        for (std::uint32_t y = 0; y < 3; ++y) {
            for (std::uint32_t x = 0; x < 3; ++x) {
                if (p.pixel_at(x, y) != Rgba{255, 128, 0, 255}) ok = false;
            }
        }
        ISO_CHECK(ok);
    }

    // ── clone independence + equality ────────────────────────────────────
    {
        PixelContainer orig(2, 2);
        orig.set_pixel(0, 0, 1, 2, 3, 4);
        PixelContainer copy = orig;  // deep copy
        copy.set_pixel(0, 0, 99, 99, 99, 99);
        ISO_CHECK((orig.pixel_at(0, 0) == Rgba{1, 2, 3, 4}));
        ISO_CHECK(orig != copy);

        PixelContainer a = PixelContainer::from_data(1, 1, {1, 2, 3, 4});
        PixelContainer b = PixelContainer::from_data(1, 1, {1, 2, 3, 4});
        PixelContainer c = PixelContainer::from_data(1, 1, {1, 2, 3, 5});
        ISO_CHECK(a == b);
        ISO_CHECK(a != c);
    }

    // ── ImageCodec stub round trip ───────────────────────────────────────
    {
        StubCodec codec;
        ISO_CHECK(codec.mime_type() == "image/x-stub");
        PixelContainer orig(2, 1);
        orig.set_pixel(0, 0, 10, 20, 30, 40);
        orig.set_pixel(1, 0, 50, 60, 70, 80);
        Bytes encoded = codec.encode(orig);
        PixelContainer decoded = codec.decode(encoded);
        ISO_CHECK(decoded == orig);
        bool threw = false;
        try {
            (void)codec.decode(Bytes{});
        } catch (const std::invalid_argument&) {
            threw = true;
        }
        ISO_CHECK(threw);
    }

    // ── dimension overflow throws ────────────────────────────────────────
    {
        bool threw = false;
        try {
            PixelContainer p(0xFFFFFFFFu, 0xFFFFFFFFu);
            (void)p;
        } catch (const std::length_error&) {
            threw = true;
        }
        ISO_CHECK(threw);
    }

    return ISO_TEST_RESULT();
}
