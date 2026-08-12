/* Execute every language-neutral CBR01 vector against the ISO C reference. */
#include "iso_test.h"

#include <stdlib.h>
#include <string.h>

#include "canonical_cbor.h"
#include "canonical_cbor_vectors.h"

static int hex_nibble(char c) {
    if (c >= '0' && c <= '9') return c - '0';
    if (c >= 'a' && c <= 'f') return c - 'a' + 10;
    return -1;
}

static uint8_t *hex_decode(const char *text, size_t *out_len) {
    size_t n = strlen(text);
    uint8_t *out;
    if ((n % 2) != 0) return NULL;
    *out_len = n / 2;
    out = *out_len == 0 ? NULL : (uint8_t *)malloc(*out_len);
    if (*out_len != 0 && out == NULL) return NULL;
    for (size_t i = 0; i < *out_len; i++) {
        int hi = hex_nibble(text[2 * i]);
        int lo = hex_nibble(text[2 * i + 1]);
        if (hi < 0 || lo < 0) {
            free(out);
            return NULL;
        }
        out[i] = (uint8_t)((hi << 4) | lo);
    }
    return out;
}

static const char *error_id(CborStatus status) {
    switch (status) {
        case CBOR_ERR_UNEXPECTED_EOF: return "unexpected-eof";
        case CBOR_ERR_TRAILING_BYTES: return "trailing-bytes";
        case CBOR_ERR_RESERVED: return "reserved";
        case CBOR_ERR_INDEFINITE: return "indefinite";
        case CBOR_ERR_NON_MINIMAL_INTEGER: return "non-minimal-integer";
        case CBOR_ERR_INVALID_UTF8: return "invalid-utf8";
        case CBOR_ERR_NON_CANONICAL_MAP_ORDER: return "non-canonical-map-order";
        case CBOR_ERR_UNSUPPORTED_SIMPLE: return "unsupported-simple";
        case CBOR_ERR_FLOAT_NOT_SUPPORTED: return "float-not-supported";
        case CBOR_ERR_TOO_DEEP: return "too-deep";
        case CBOR_ERR_LENGTH_TOO_LARGE: return "length-too-large";
        case CBOR_ERR_DUPLICATE_MAP_KEY: return "duplicate-map-key";
        case CBOR_ERR_ENCODE_TOO_DEEP: return "encode-too-deep";
        case CBOR_ERR_ENCODE_TOO_LARGE: return "encode-too-large";
        case CBOR_OK: return "ok";
        case CBOR_ERR_ALLOC: return "allocation-failed";
    }
    return "unknown";
}

static int parse_size(const char *text, size_t *out) {
    char *end = NULL;
    unsigned long value = strtoul(text, &end, 10);
    if (end == text || *end != '\0') return 0;
    *out = (size_t)value;
    return (unsigned long)*out == value;
}

static CborValue *nested_array(size_t depth) {
    CborValue *value = cbor_null();
    for (size_t i = 0; i < depth && value != NULL; i++) {
        CborValue *outer = cbor_array();
        if (outer == NULL || cbor_array_push(outer, value) != CBOR_OK) {
            cbor_free(outer);
            return NULL;
        }
        value = outer;
    }
    return value;
}

static CborValue *build_generated(const char *spec) {
    static const char nested_prefix[] = "nested-array:";
    static const char bytes_prefix[] = "bytes-repeat:";
    if (strncmp(spec, nested_prefix, sizeof(nested_prefix) - 1) == 0) {
        size_t depth;
        return parse_size(spec + sizeof(nested_prefix) - 1, &depth)
                   ? nested_array(depth)
                   : NULL;
    }
    if (strncmp(spec, bytes_prefix, sizeof(bytes_prefix) - 1) == 0) {
        const char *length_text = spec + sizeof(bytes_prefix) - 1;
        const char *separator = strchr(length_text, ':');
        size_t length;
        uint8_t *bytes;
        CborValue *value;
        if (separator == NULL || strlen(separator + 1) != 2) return NULL;
        {
            char buffer[32];
            size_t digits = (size_t)(separator - length_text);
            if (digits == 0 || digits >= sizeof(buffer)) return NULL;
            memcpy(buffer, length_text, digits);
            buffer[digits] = '\0';
            if (!parse_size(buffer, &length)) return NULL;
        }
        bytes = length == 0 ? NULL : (uint8_t *)malloc(length);
        if (length != 0 && bytes == NULL) return NULL;
        {
            int hi = hex_nibble(separator[1]);
            int lo = hex_nibble(separator[2]);
            if (hi < 0 || lo < 0) {
                free(bytes);
                return NULL;
            }
            if (length != 0) memset(bytes, (hi << 4) | lo, length);
        }
        value = cbor_bytes(bytes, length);
        free(bytes);
        return value;
    }
    return NULL;
}

static uint8_t *build_wire(const char *spec, size_t *out_len) {
    static const char nested_prefix[] = "wire:nested-array:";
    static const char bytes_prefix[] = "wire:bytes-repeat:";
    if (strncmp(spec, nested_prefix, sizeof(nested_prefix) - 1) == 0) {
        size_t depth;
        uint8_t *wire;
        if (!parse_size(spec + sizeof(nested_prefix) - 1, &depth)) return NULL;
        wire = (uint8_t *)malloc(depth + 1);
        if (wire == NULL) return NULL;
        memset(wire, 0x81, depth);
        wire[depth] = 0xF6;
        *out_len = depth + 1;
        return wire;
    }
    if (strncmp(spec, bytes_prefix, sizeof(bytes_prefix) - 1) == 0) {
        const char *length_text = spec + sizeof(bytes_prefix) - 1;
        const char *separator = strchr(length_text, ':');
        size_t length;
        uint8_t byte;
        uint8_t *wire;
        size_t header;
        if (separator == NULL || strlen(separator + 1) != 2) return NULL;
        {
            char buffer[32];
            size_t digits = (size_t)(separator - length_text);
            if (digits == 0 || digits >= sizeof(buffer)) return NULL;
            memcpy(buffer, length_text, digits);
            buffer[digits] = '\0';
            if (!parse_size(buffer, &length)) return NULL;
        }
        byte = (uint8_t)((hex_nibble(separator[1]) << 4) | hex_nibble(separator[2]));
        header = length <= 23 ? 1 : (length <= 0xFF ? 2 : (length <= 0xFFFF ? 3 : 5));
        wire = (uint8_t *)malloc(header + length);
        if (wire == NULL) return NULL;
        if (header == 1) wire[0] = (uint8_t)(0x40 | length);
        else if (header == 2) { wire[0] = 0x58; wire[1] = (uint8_t)length; }
        else if (header == 3) { wire[0] = 0x59; wire[1] = (uint8_t)(length >> 8); wire[2] = (uint8_t)length; }
        else { wire[0] = 0x5A; wire[1] = (uint8_t)(length >> 24); wire[2] = (uint8_t)(length >> 16); wire[3] = (uint8_t)(length >> 8); wire[4] = (uint8_t)length; }
        if (length != 0) memset(wire + header, byte, length);
        *out_len = header + length;
        return wire;
    }
    return NULL;
}

static CborValue *build_map(const char *spec) {
    size_t spec_len = strlen(spec);
    char *copy = (char *)malloc(spec_len + 1);
    CborValue *map = cbor_map();
    char *cursor;
    if (copy == NULL || map == NULL) { free(copy); cbor_free(map); return NULL; }
    memcpy(copy, spec, spec_len + 1);
    cursor = copy;
    while (*cursor != '\0') {
        char *end = strchr(cursor, ';');
        char *equal;
        CborValue *key = NULL, *value = NULL;
        uint8_t *key_bytes, *value_bytes;
        size_t key_len, value_len;
        if (end != NULL) *end = '\0';
        equal = strchr(cursor, '=');
        if (equal == NULL) goto fail;
        *equal = '\0';
        key_bytes = hex_decode(cursor, &key_len);
        value_bytes = hex_decode(equal + 1, &value_len);
        if ((key_len != 0 && key_bytes == NULL) || (value_len != 0 && value_bytes == NULL)) {
            free(key_bytes); free(value_bytes); goto fail;
        }
        if (cbor_decode(key_bytes, key_len, &key) != CBOR_OK ||
            cbor_decode(value_bytes, value_len, &value) != CBOR_OK) {
            free(key_bytes); free(value_bytes); cbor_free(key); cbor_free(value); goto fail;
        }
        free(key_bytes); free(value_bytes);
        if (cbor_map_push(map, key, value) != CBOR_OK) goto fail;
        if (end == NULL) break;
        cursor = end + 1;
    }
    free(copy);
    return map;
fail:
    free(copy);
    cbor_free(map);
    return NULL;
}

static int bytes_equal(const uint8_t *a, size_t a_len, const uint8_t *b, size_t b_len) {
    return a_len == b_len && (a_len == 0 || memcmp(a, b, a_len) == 0);
}

int main(void) {
    for (size_t i = 0; i < CANONICAL_CBOR_V1_VECTOR_COUNT; i++) {
        const CanonicalCborVector *vector = &CANONICAL_CBOR_V1_VECTORS[i];
        if (strcmp(vector->operation, "round-trip") == 0) {
            size_t input_len, expected_len, encoded_len = 0;
            uint8_t *input = hex_decode(vector->input, &input_len);
            uint8_t *expected = hex_decode(vector->expected, &expected_len);
            uint8_t *encoded = NULL;
            CborValue *value = NULL;
            ISO_CHECK(cbor_decode(input, input_len, &value) == CBOR_OK);
            ISO_CHECK(cbor_encode(value, &encoded, &encoded_len) == CBOR_OK);
            ISO_CHECK(bytes_equal(encoded, encoded_len, expected, expected_len));
            free(input); free(expected); free(encoded); cbor_free(value);
        } else if (strcmp(vector->operation, "decode-error") == 0) {
            size_t input_len;
            uint8_t *input;
            CborValue *value = NULL;
            CborStatus status;
            if (strncmp(vector->input, "nested-array-wire:", 18) == 0) {
                static const char prefix[] = "wire:nested-array:";
                char spec[64];
                size_t suffix_len = strlen(vector->input + 18);
                ISO_CHECK(sizeof(prefix) - 1 + suffix_len + 1 <= sizeof(spec));
                memcpy(spec, prefix, sizeof(prefix) - 1);
                memcpy(spec + sizeof(prefix) - 1, vector->input + 18, suffix_len + 1);
                input = build_wire(spec, &input_len);
            } else {
                input = hex_decode(vector->input, &input_len);
            }
            status = cbor_decode(input, input_len, &value);
            ISO_CHECK(strcmp(error_id(status), vector->expected) == 0);
            ISO_CHECK(strncmp(cbor_status_message(status), "canonical-cbor:", 15) == 0);
            free(input); cbor_free(value);
        } else {
            CborValue *value = strcmp(vector->operation, "encode-map") == 0
                                   ? build_map(vector->input)
                                   : (strcmp(vector->input, "duplicate-map-key") == 0
                                          ? build_map("6161=00;6161=01")
                                          : build_generated(vector->input));
            uint8_t *encoded = NULL;
            size_t encoded_len = 0;
            CborStatus status = cbor_encode(value, &encoded, &encoded_len);
            if (strcmp(vector->operation, "encode-error") == 0) {
                ISO_CHECK(strcmp(error_id(status), vector->expected) == 0);
                ISO_CHECK(strncmp(cbor_status_message(status), "canonical-cbor:", 15) == 0);
            } else {
                size_t expected_len;
                uint8_t *expected = strcmp(vector->operation, "encode-map") == 0
                                        ? hex_decode(vector->expected, &expected_len)
                                        : build_wire(vector->expected, &expected_len);
                ISO_CHECK(status == CBOR_OK);
                ISO_CHECK(bytes_equal(encoded, encoded_len, expected, expected_len));
                free(expected);
            }
            free(encoded); cbor_free(value);
        }
    }
    return ISO_TEST_RESULT();
}
