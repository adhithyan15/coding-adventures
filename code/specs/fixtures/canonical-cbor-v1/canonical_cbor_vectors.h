#ifndef CA_CANONICAL_CBOR_V1_VECTORS_H
#define CA_CANONICAL_CBOR_V1_VECTORS_H

#include <stddef.h>

typedef struct {
    const char *id;
    const char *operation;
    const char *input;
    const char *expected;
} CanonicalCborVector;

static const CanonicalCborVector CANONICAL_CBOR_V1_VECTORS[] = {
    {"cbor-v1-unsigned-zero", "round-trip", "00", "00"},
    {"cbor-v1-unsigned-23", "round-trip", "17", "17"},
    {"cbor-v1-unsigned-24", "round-trip", "1818", "1818"},
    {"cbor-v1-unsigned-255", "round-trip", "18ff", "18ff"},
    {"cbor-v1-unsigned-256", "round-trip", "190100", "190100"},
    {"cbor-v1-unsigned-65535", "round-trip", "19ffff", "19ffff"},
    {"cbor-v1-unsigned-65536", "round-trip", "1a00010000", "1a00010000"},
    {"cbor-v1-unsigned-u32max", "round-trip", "1affffffff", "1affffffff"},
    {"cbor-v1-unsigned-u32plus1", "round-trip", "1b0000000100000000", "1b0000000100000000"},
    {"cbor-v1-unsigned-u64max", "round-trip", "1bffffffffffffffff", "1bffffffffffffffff"},
    {"cbor-v1-negative-one", "round-trip", "20", "20"},
    {"cbor-v1-negative-24", "round-trip", "37", "37"},
    {"cbor-v1-negative-25", "round-trip", "3818", "3818"},
    {"cbor-v1-bytes-empty", "round-trip", "40", "40"},
    {"cbor-v1-bytes-three", "round-trip", "430001ff", "430001ff"},
    {"cbor-v1-text-empty", "round-trip", "60", "60"},
    {"cbor-v1-text-utf8", "round-trip", "63e29883", "63e29883"},
    {"cbor-v1-array", "round-trip", "8300f5f6", "8300f5f6"},
    {"cbor-v1-map-wire", "round-trip", "a300f460f56161f6", "a300f460f56161f6"},
    {"cbor-v1-tag-zero", "round-trip", "c0f6", "c0f6"},
    {"cbor-v1-tag-24", "round-trip", "d81800", "d81800"},
    {"cbor-v1-simple-values", "round-trip", "83f4f5f6", "83f4f5f6"},
    {"cbor-v1-map-input-order", "encode-map", "6162=f6;1818=f5;60=f4;00=01;6161=02", "a5000160f41818f56161026162f6"},
    {"cbor-v1-depth-128", "generated-round-trip", "nested-array:128", "wire:nested-array:128"},
    {"cbor-v1-size-limit", "generated-round-trip", "bytes-repeat:1048571:00", "wire:bytes-repeat:1048571:00"},
    {"cbor-v1-empty-input", "decode-error", "", "unexpected-eof"},
    {"cbor-v1-truncated-argument", "decode-error", "1a0001", "unexpected-eof"},
    {"cbor-v1-trailing", "decode-error", "0000", "trailing-bytes"},
    {"cbor-v1-reserved-28", "decode-error", "1c", "reserved"},
    {"cbor-v1-reserved-30", "decode-error", "1e", "reserved"},
    {"cbor-v1-indefinite-bytes", "decode-error", "5f", "indefinite"},
    {"cbor-v1-indefinite-text", "decode-error", "7f", "indefinite"},
    {"cbor-v1-indefinite-array", "decode-error", "9f", "indefinite"},
    {"cbor-v1-indefinite-map", "decode-error", "bf", "indefinite"},
    {"cbor-v1-break", "decode-error", "ff", "indefinite"},
    {"cbor-v1-nonminimal-inline", "decode-error", "1800", "non-minimal-integer"},
    {"cbor-v1-nonminimal-u8", "decode-error", "1900ff", "non-minimal-integer"},
    {"cbor-v1-nonminimal-u16", "decode-error", "1a0000ffff", "non-minimal-integer"},
    {"cbor-v1-nonminimal-u32", "decode-error", "1b00000000ffffffff", "non-minimal-integer"},
    {"cbor-v1-nonminimal-length", "decode-error", "5800", "non-minimal-integer"},
    {"cbor-v1-nonminimal-tag", "decode-error", "d800f6", "non-minimal-integer"},
    {"cbor-v1-invalid-utf8", "decode-error", "61ff", "invalid-utf8"},
    {"cbor-v1-invalid-utf8-overlong", "decode-error", "62c080", "invalid-utf8"},
    {"cbor-v1-map-out-of-order", "decode-error", "a2616200616101", "non-canonical-map-order"},
    {"cbor-v1-map-duplicate-wire", "decode-error", "a2616100616101", "non-canonical-map-order"},
    {"cbor-v1-unsupported-simple", "decode-error", "f0", "unsupported-simple"},
    {"cbor-v1-undefined", "decode-error", "f7", "unsupported-simple"},
    {"cbor-v1-float16", "decode-error", "f90000", "float-not-supported"},
    {"cbor-v1-float32", "decode-error", "fa00000000", "float-not-supported"},
    {"cbor-v1-float64", "decode-error", "fb0000000000000000", "float-not-supported"},
    {"cbor-v1-hostile-bytes-length", "decode-error", "5bffffffffffffffff", "length-too-large"},
    {"cbor-v1-depth-129-decode", "decode-error", "nested-array-wire:129", "too-deep"},
    {"cbor-v1-duplicate-encode", "encode-error", "duplicate-map-key", "duplicate-map-key"},
    {"cbor-v1-depth-129-encode", "encode-error", "nested-array:129", "encode-too-deep"},
    {"cbor-v1-size-limit-plus-one", "encode-error", "bytes-repeat:1048572:00", "encode-too-large"},
};

#define CANONICAL_CBOR_V1_VECTOR_COUNT \
    (sizeof(CANONICAL_CBOR_V1_VECTORS) / sizeof(CANONICAL_CBOR_V1_VECTORS[0]))

#endif
