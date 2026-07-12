// pixel_container.hpp — a flat, row-major RGBA8 pixel buffer and an image-codec
// interface, in pure ISO C++17, header-only, in namespace ca. A faithful port of
// the Rust `pixel-container` crate.
// ===========================================================================
//
// PixelContainer is the universal interchange type between renderers and image
// codecs: 4 bytes per pixel in RGBA order, row-major from the top-left.
//
//   offset = (y * width + x) * 4
//   data[offset + 0] = R,  +1 = G,  +2 = B,  +3 = A
//
// ImageCodec is the abstract base every codec (BMP, PPM, QOI, PNG, ...)
// implements to encode/decode a PixelContainer — no rendering types in scope.
//
// `from_data` throws std::invalid_argument on a length mismatch (the Rust crate
// panics); `PixelContainer` has value semantics (deep copy on copy).
//
// Portability: pure ISO C++17 — GCC, Clang, and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors. Standard library only.
#ifndef CA_PIXEL_CONTAINER_HPP
#define CA_PIXEL_CONTAINER_HPP

#include <array>
#include <cstddef>
#include <cstdint>
#include <stdexcept>
#include <string>
#include <vector>

namespace ca {

class PixelContainer {
public:
    std::uint32_t width;
    std::uint32_t height;
    std::vector<std::uint8_t> data;  // width*height*4 bytes, row-major RGBA

    // A blank (all-zero, fully transparent) buffer of the given size.
    PixelContainer(std::uint32_t w, std::uint32_t h)
        : width(w), height(h), data(byte_size(w, h), 0) {}

    // A buffer from an existing RGBA8 pixel buffer. Throws std::invalid_argument
    // if `pixels.size() != width*height*4`.
    static PixelContainer from_data(std::uint32_t w, std::uint32_t h,
                                    std::vector<std::uint8_t> pixels) {
        std::size_t expected = byte_size(w, h);
        if (pixels.size() != expected) {
            throw std::invalid_argument(
                "PixelContainer::from_data: data length != width*height*4");
        }
        return PixelContainer(w, h, std::move(pixels));
    }

    // Read the RGBA components at (x, y); {0,0,0,0} if out of bounds.
    std::array<std::uint8_t, 4> pixel_at(std::uint32_t x,
                                         std::uint32_t y) const {
        if (x >= width || y >= height) {
            return {0, 0, 0, 0};
        }
        std::size_t i = (static_cast<std::size_t>(y) * width + x) * 4;
        return {data[i], data[i + 1], data[i + 2], data[i + 3]};
    }

    // Write the RGBA components at (x, y); no-op if out of bounds.
    void set_pixel(std::uint32_t x, std::uint32_t y, std::uint8_t r,
                   std::uint8_t g, std::uint8_t b, std::uint8_t a) {
        if (x >= width || y >= height) {
            return;
        }
        std::size_t i = (static_cast<std::size_t>(y) * width + x) * 4;
        data[i] = r;
        data[i + 1] = g;
        data[i + 2] = b;
        data[i + 3] = a;
    }

    // Fill the whole buffer with one RGBA colour.
    void fill(std::uint8_t r, std::uint8_t g, std::uint8_t b, std::uint8_t a) {
        for (std::size_t i = 0; i + 4 <= data.size(); i += 4) {
            data[i] = r;
            data[i + 1] = g;
            data[i + 2] = b;
            data[i + 3] = a;
        }
    }

    std::size_t pixel_count() const {
        return static_cast<std::size_t>(width) * height;
    }
    std::size_t byte_count() const { return data.size(); }

    bool operator==(const PixelContainer& o) const {
        return width == o.width && height == o.height && data == o.data;
    }
    bool operator!=(const PixelContainer& o) const { return !(*this == o); }

private:
    PixelContainer(std::uint32_t w, std::uint32_t h,
                   std::vector<std::uint8_t> d)
        : width(w), height(h), data(std::move(d)) {}

    // width*height*4 as size_t, throwing on overflow.
    static std::size_t byte_size(std::uint32_t w, std::uint32_t h) {
        std::size_t ww = w, hh = h;
        if (ww != 0 && hh > (static_cast<std::size_t>(-1)) / ww) {
            throw std::length_error("PixelContainer dimensions overflow size_t");
        }
        std::size_t pixels = ww * hh;
        if (pixels > (static_cast<std::size_t>(-1)) / 4) {
            throw std::length_error("PixelContainer dimensions overflow size_t");
        }
        return pixels * 4;
    }
};

// A codec's encode/decode interface over PixelContainer. A codec implements
// this to participate in the pipeline; it never sees any rendering type.
class ImageCodec {
public:
    virtual ~ImageCodec() = default;
    // The IANA MIME type for this format, e.g. "image/png".
    virtual std::string mime_type() const = 0;
    // Encode a pixel buffer into the bytes of this format.
    virtual std::vector<std::uint8_t> encode(const PixelContainer& c) const = 0;
    // Decode bytes into a pixel buffer, throwing on invalid input.
    virtual PixelContainer decode(const std::vector<std::uint8_t>& bytes) const = 0;
};

}  // namespace ca

#endif  // CA_PIXEL_CONTAINER_HPP
