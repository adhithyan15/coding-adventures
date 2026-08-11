// Execute every language-neutral CBR01 vector against the ISO C++ reference.
#include "iso_test.h"

#include <cstdint>
#include <string>
#include <vector>

#include "canonical_cbor.hpp"
#include "canonical_cbor_vectors.h"

namespace cc = ca::canonical_cbor;
using Bytes = std::vector<std::uint8_t>;
using cc::CborError;
using cc::CborValue;

static int nibble(char c) {
    if (c >= '0' && c <= '9') return c - '0';
    if (c >= 'a' && c <= 'f') return c - 'a' + 10;
    return -1;
}

static Bytes hex_decode(const std::string& text) {
    Bytes out;
    out.reserve(text.size() / 2);
    for (std::size_t i = 0; i < text.size(); i += 2)
        out.push_back(static_cast<std::uint8_t>((nibble(text[i]) << 4) |
                                                nibble(text[i + 1])));
    return out;
}

static const char* error_id(CborError error) {
    switch (error) {
        case CborError::UnexpectedEof: return "unexpected-eof";
        case CborError::TrailingBytes: return "trailing-bytes";
        case CborError::Reserved: return "reserved";
        case CborError::Indefinite: return "indefinite";
        case CborError::NonMinimalInteger: return "non-minimal-integer";
        case CborError::InvalidUtf8: return "invalid-utf8";
        case CborError::NonCanonicalMapOrder: return "non-canonical-map-order";
        case CborError::UnsupportedSimple: return "unsupported-simple";
        case CborError::FloatNotSupported: return "float-not-supported";
        case CborError::TooDeep: return "too-deep";
        case CborError::LengthTooLarge: return "length-too-large";
        case CborError::DuplicateMapKey: return "duplicate-map-key";
        case CborError::EncodeTooDeep: return "encode-too-deep";
        case CborError::EncodeTooLarge: return "encode-too-large";
    }
    return "unknown";
}

static std::size_t number_after(const std::string& spec, const std::string& prefix) {
    return static_cast<std::size_t>(std::stoull(spec.substr(prefix.size())));
}

static CborValue nested_array(std::size_t depth) {
    CborValue value = CborValue::null();
    for (std::size_t i = 0; i < depth; i++)
        value = CborValue::arr({std::move(value)});
    return value;
}

static CborValue build_generated(const std::string& spec) {
    const std::string nested = "nested-array:";
    const std::string repeated = "bytes-repeat:";
    if (spec.rfind(nested, 0) == 0)
        return nested_array(number_after(spec, nested));
    std::size_t separator = spec.find(':', repeated.size());
    std::size_t length = static_cast<std::size_t>(
        std::stoull(spec.substr(repeated.size(), separator - repeated.size())));
    std::uint8_t byte = static_cast<std::uint8_t>(
        (nibble(spec[separator + 1]) << 4) | nibble(spec[separator + 2]));
    return CborValue::byte_string(Bytes(length, byte));
}

static Bytes build_wire(const std::string& spec) {
    const std::string nested = "wire:nested-array:";
    const std::string repeated = "wire:bytes-repeat:";
    if (spec.rfind(nested, 0) == 0) {
        std::size_t depth = number_after(spec, nested);
        Bytes wire(depth, 0x81);
        wire.push_back(0xF6);
        return wire;
    }
    std::size_t separator = spec.find(':', repeated.size());
    std::size_t length = static_cast<std::size_t>(
        std::stoull(spec.substr(repeated.size(), separator - repeated.size())));
    std::uint8_t byte = static_cast<std::uint8_t>(
        (nibble(spec[separator + 1]) << 4) | nibble(spec[separator + 2]));
    Bytes wire;
    if (length <= 23) {
        wire.push_back(static_cast<std::uint8_t>(0x40 | length));
    } else if (length <= 0xFF) {
        wire = {0x58, static_cast<std::uint8_t>(length)};
    } else if (length <= 0xFFFF) {
        wire = {0x59, static_cast<std::uint8_t>(length >> 8),
                static_cast<std::uint8_t>(length)};
    } else {
        wire = {0x5A, static_cast<std::uint8_t>(length >> 24),
                static_cast<std::uint8_t>(length >> 16),
                static_cast<std::uint8_t>(length >> 8),
                static_cast<std::uint8_t>(length)};
    }
    wire.insert(wire.end(), length, byte);
    return wire;
}

static CborValue build_map(const std::string& spec) {
    std::vector<std::pair<CborValue, CborValue>> entries;
    std::size_t start = 0;
    while (start < spec.size()) {
        std::size_t end = spec.find(';', start);
        std::string fragment = spec.substr(start, end - start);
        std::size_t equal = fragment.find('=');
        entries.emplace_back(
            cc::decode(hex_decode(fragment.substr(0, equal))),
            cc::decode(hex_decode(fragment.substr(equal + 1))));
        if (end == std::string::npos) break;
        start = end + 1;
    }
    return CborValue::mapping(std::move(entries));
}

static std::string decode_error(const std::string& input) {
    Bytes wire;
    const std::string generated = "nested-array-wire:";
    if (input.rfind(generated, 0) == 0)
        wire = build_wire("wire:nested-array:" + input.substr(generated.size()));
    else
        wire = hex_decode(input);
    try {
        cc::decode(wire);
    } catch (const cc::CborException& error) {
        ISO_CHECK(std::string(cc::error_message(error.error())).rfind("canonical-cbor:", 0) == 0);
        return error_id(error.error());
    }
    return "ok";
}

int main() {
    for (std::size_t i = 0; i < CANONICAL_CBOR_V1_VECTOR_COUNT; i++) {
        const CanonicalCborVector& vector = CANONICAL_CBOR_V1_VECTORS[i];
        std::string operation(vector.operation);
        if (operation == "round-trip") {
            ISO_CHECK(cc::encode(cc::decode(hex_decode(vector.input))) ==
                      hex_decode(vector.expected));
        } else if (operation == "decode-error") {
            ISO_CHECK(decode_error(vector.input) == vector.expected);
        } else {
            CborValue value = operation == "encode-map"
                                  ? build_map(vector.input)
                                  : (std::string(vector.input) == "duplicate-map-key"
                                         ? build_map("6161=00;6161=01")
                                         : build_generated(vector.input));
            try {
                Bytes encoded = cc::encode(value);
                if (operation == "encode-error") {
                    ISO_CHECK(0);
                } else {
                    Bytes expected = operation == "encode-map"
                                         ? hex_decode(vector.expected)
                                         : build_wire(vector.expected);
                    ISO_CHECK(encoded == expected);
                }
            } catch (const cc::CborException& error) {
                ISO_CHECK(operation == "encode-error" &&
                          std::string(error_id(error.error())) == vector.expected);
                ISO_CHECK(std::string(cc::error_message(error.error())).rfind("canonical-cbor:", 0) == 0);
            }
        }
    }
    return ISO_TEST_RESULT();
}
