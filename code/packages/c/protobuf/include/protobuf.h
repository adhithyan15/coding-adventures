/*
 * protobuf.h — a zero-dependency Protocol Buffers *wire-format* codec, ISO C17.
 * ===========================================================================
 *
 * A faithful port of the Rust `protobuf` crate: just the wire format
 * (https://protobuf.dev/programming-guides/encoding/) — enough to encode and
 * decode messages byte-for-byte compatibly with Google's protobuf. There is no
 * `.proto` compiler and no codegen: callers hand-write the few encode/decode
 * calls for the specific messages they need.
 *
 * ── The wire format in one paragraph ──────────────────────────────────────
 * A message is a flat sequence of `(tag, value)` records with no framing. Each
 * tag is a varint whose low 3 bits are the wire type and whose upper bits are
 * the field number:  `tag = (field_number << 3) | wire_type`.
 *
 *   wire type | name             | payload
 *   ----------|------------------|-----------------------------------------
 *   0         | Varint           | one LEB128 varint (ints, bools, enums)
 *   1         | Fixed64          | 8 little-endian bytes
 *   2         | LengthDelimited  | a varint length n, then n bytes
 *   5         | Fixed32          | 4 little-endian bytes
 *
 * ── Varints (LEB128, unsigned) ────────────────────────────────────────────
 * Seven bits per byte, little-endian, top bit = "more bytes follow". 300 →
 * [0xAC, 0x02]. A u64 needs at most 10 bytes; an 11th means overflow.
 *
 * Pure ISO C17: no <math.h>, no compiler extensions, no 128-bit integers.
 */
#ifndef PROTOBUF_H
#define PROTOBUF_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t, uint32_t, uint64_t */

#ifdef __cplusplus
extern "C" {
#endif

/* ── Wire types (the low 3 bits of a field tag) ────────────────────────────*/
typedef enum {
    PB_WIRE_VARINT = 0,
    PB_WIRE_FIXED64 = 1,
    PB_WIRE_LENGTH_DELIMITED = 2,
    PB_WIRE_FIXED32 = 5
} PbWireType;

/* ── Decode errors (encoding cannot fail) ──────────────────────────────────*/
typedef enum {
    PB_OK = 0,
    /* A varint ran past the buffer end, or exceeded 10 bytes (u64 overflow). */
    PB_ERR_TRUNCATED_VARINT,
    /* A length-delimited / fixed field claimed more bytes than remain. */
    PB_ERR_UNEXPECTED_EOF,
    /* The tag carried a wire type this codec does not implement (3, 4, 6, 7). */
    PB_ERR_UNKNOWN_WIRE_TYPE,
    /* A field number of zero, which protobuf forbids. */
    PB_ERR_ZERO_FIELD_NUMBER
} PbError;

/* Human-readable message for an error code (mirrors Rust's Display). */
const char *pb_error_message(PbError err);

/* ── Writer ────────────────────────────────────────────────────────────────*/

/* Builds a message by appending fields in call order. The output is exactly the
 * concatenation of the fields written. If any append hits an allocation
 * failure, `oom` latches to 1 and further appends are no-ops (check it before
 * trusting the buffer). Pair pb_writer_init with pb_writer_free. */
typedef struct {
    uint8_t *buf;
    size_t len;
    size_t cap;
    int oom;
} PbWriter;

void pb_writer_init(PbWriter *w);
void pb_writer_free(PbWriter *w);

/* Borrow the current bytes (valid until the next append or free). */
const uint8_t *pb_writer_bytes(const PbWriter *w);
size_t pb_writer_len(const PbWriter *w);

/* Transfer ownership of the buffer to the caller (like Rust's `into_bytes`);
 * `*out_len` receives the length, the writer is reset to empty, and the caller
 * must free() the returned pointer. Returns NULL if the message is empty or the
 * writer is in its OOM state. */
uint8_t *pb_writer_take(PbWriter *w, size_t *out_len);

/* Append a raw LEB128 varint (no tag). */
void pb_write_varint(PbWriter *w, uint64_t value);
/* A varint-typed field (int32/64, uint32/64, bool, enum). */
void pb_varint(PbWriter *w, uint32_t field, uint64_t value);
/* A length-delimited field carrying arbitrary bytes. */
void pb_bytes(PbWriter *w, uint32_t field, const uint8_t *value, size_t len);
/* A length-delimited string field (NUL-terminated; the NUL is not written). */
void pb_string(PbWriter *w, uint32_t field, const char *value);
/* A length-delimited field carrying an already-encoded embedded message. */
void pb_message(PbWriter *w, uint32_t field, const uint8_t *encoded, size_t len);
/* A fixed32 / sfixed32 / float field (4 little-endian bytes). */
void pb_fixed32(PbWriter *w, uint32_t field, uint32_t value);
/* A fixed64 / sfixed64 / double field (8 little-endian bytes). */
void pb_fixed64(PbWriter *w, uint32_t field, uint64_t value);

/* ── Reader ────────────────────────────────────────────────────────────────*/

/* A decoded field value, tagged by wire type. `bytes` borrows the reader's
 * input buffer (no copy) and is valid as long as that buffer lives. */
typedef struct {
    PbWireType kind;
    uint64_t varint;      /* PB_WIRE_VARINT */
    uint64_t fixed64;     /* PB_WIRE_FIXED64 */
    uint32_t fixed32;     /* PB_WIRE_FIXED32 */
    const uint8_t *bytes; /* PB_WIRE_LENGTH_DELIMITED */
    size_t bytes_len;
} PbValue;

/* One decoded field: its (1-based) number and its value. */
typedef struct {
    uint32_t number;
    PbValue value;
} PbField;

/* If `v` is a varint, store it in `*out` and return 1; else return 0. */
int pb_value_as_varint(const PbValue *v, uint64_t *out);
/* If `v` is length-delimited, store its slice in `*out`/`*out_len`, return 1. */
int pb_value_as_bytes(const PbValue *v, const uint8_t **out, size_t *out_len);

/* A cursor over an encoded message. */
typedef struct {
    const uint8_t *data;
    size_t len;
    size_t pos;
} PbReader;

void pb_reader_init(PbReader *r, const uint8_t *data, size_t len);
/* Whether every field has been consumed. */
int pb_reader_is_empty(const PbReader *r);

/* Read the next field. On success returns PB_OK and sets `*has_field` to 1 (a
 * field was read into `*out`) or 0 (clean end of message). On malformed input
 * returns the error code and leaves `*has_field` 0. Unknown field numbers are
 * yielded too, so callers can skip them (forward compatibility). */
PbError pb_reader_next_field(PbReader *r, PbField *out, int *has_field);

#ifdef __cplusplus
}
#endif

#endif /* PROTOBUF_H */
