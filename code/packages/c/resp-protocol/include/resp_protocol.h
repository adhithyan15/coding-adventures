/*
 * resp_protocol.h — the RESP (REdis Serialization Protocol) v2, in pure ISO
 * C17. A faithful port of the Rust `resp-protocol` crate.
 * ===========================================================================
 *
 * RESP is the line protocol Redis speaks. A value is one of five frame types,
 * each introduced by a one-byte prefix and terminated by CRLF ("\r\n"):
 *
 *   +OK\r\n              simple string
 *   -ERR boom\r\n        error   (message may be split "TYPE detail")
 *   :-42\r\n             integer (signed 64-bit)
 *   $3\r\nfoo\r\n        bulk string (length-prefixed bytes; $-1 == null)
 *   *2\r\n:1\r\n:2\r\n   array (count-prefixed values; *-1 == null)
 *
 * A bare line with no known prefix is parsed as an "inline command": the line is
 * split on ASCII whitespace and each token becomes a bulk string inside an
 * array (this is how a human typing into `redis-cli` is understood).
 *
 * This port models the recursive value with a tagged union (RespValue). Values
 * are heap-allocated; free any value you own — including every value returned by
 * a decode — with resp_free (which recurses into arrays).
 *
 * Portability: pure ISO C17 — GCC, Clang, and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors. Bulk-string contents are arbitrary bytes.
 */
#ifndef RESP_PROTOCOL_H
#define RESP_PROTOCOL_H

#include <stddef.h> /* size_t */

/* The five RESP frame types. */
typedef enum {
    RESP_SIMPLE_STRING,
    RESP_ERROR,
    RESP_INTEGER,
    RESP_BULK_STRING,
    RESP_ARRAY
} RespType;

/* A RESP value (a tagged union). Construct with the resp_* constructors below;
 * release with resp_free. Read fields only after checking `type`. */
typedef struct RespValue RespValue;
struct RespValue {
    RespType type;
    union {
        /* RESP_SIMPLE_STRING: an owned NUL-terminated string. */
        char *simple;
        /* RESP_ERROR: the full message plus its "TYPE detail" split (all owned,
         * NUL-terminated; detail is "" when the message has no space). */
        struct {
            char *message;
            char *error_type;
            char *detail;
        } error;
        /* RESP_INTEGER: a signed 64-bit value. */
        long long integer;
        /* RESP_BULK_STRING: bytes of length `len`, or the null bulk string
         * (is_null == 1, data == NULL). `data` is not NUL-terminated. */
        struct {
            unsigned char *data;
            size_t len;
            int is_null;
        } bulk;
        /* RESP_ARRAY: `count` child values, or the null array (is_null == 1,
         * items == NULL). */
        struct {
            RespValue **items;
            size_t count;
            int is_null;
        } array;
    } as;
};

/* ---- constructors (heap-allocated; NULL on allocation failure) --------- */

RespValue *resp_simple_string(const char *s);
RespValue *resp_error(const char *message);
RespValue *resp_integer(long long value);
/* Non-null bulk string copying `len` bytes from `data` (data may be NULL iff
 * len == 0). */
RespValue *resp_bulk_string(const unsigned char *data, size_t len);
RespValue *resp_bulk_null(void);
/* Array taking OWNERSHIP of `items` (an array of `count` owned RespValue*, which
 * resp_free will free) — pass items == NULL with count == 0 for an empty array. */
RespValue *resp_array(RespValue **items, size_t count);
RespValue *resp_array_null(void);

/* resp_free — recursively free a value and, for arrays, its children. NULL-safe. */
void resp_free(RespValue *value);

/* resp_equal — structural equality (1 or 0), useful for tests. */
int resp_equal(const RespValue *a, const RespValue *b);

/* Error accessors (valid only for RESP_ERROR values). */
const char *resp_error_type(const RespValue *value);
const char *resp_error_detail(const RespValue *value);

/* ---- encoding --------------------------------------------------------- */

typedef enum {
    RESP_ENCODE_OK = 0,
    RESP_ENCODE_ERR_SIMPLE_NEWLINE, /* a simple string contained CR or LF */
    RESP_ENCODE_ERR_ALLOC
} RespEncodeStatus;

/* resp_encode — serialize `value` into a newly malloc'd buffer. On
 * RESP_ENCODE_OK, *out points to the bytes (caller frees) and *out_len is the
 * length; otherwise *out is set to NULL. */
RespEncodeStatus resp_encode(const RespValue *value, unsigned char **out,
                             size_t *out_len);

/* ---- decoding --------------------------------------------------------- */

typedef enum {
    RESP_DECODE_OK = 0,     /* a complete value was decoded */
    RESP_DECODE_INCOMPLETE, /* need more bytes (Rust Ok(None)) */
    RESP_DECODE_ERROR       /* malformed input (Rust Err) */
} RespDecodeStatus;

/* resp_decode — decode one frame from `buffer` (`len` bytes). On
 * RESP_DECODE_OK, *out is the decoded value (caller frees with resp_free) and
 * *consumed is the number of bytes read; otherwise *out is NULL. */
RespDecodeStatus resp_decode(const unsigned char *buffer, size_t len,
                             RespValue **out, size_t *consumed);

/* resp_decode_all — decode as many whole frames as `buffer` contains. On
 * RESP_DECODE_OK sets *out_items (a malloc'd array of *out_count owned values;
 * NULL when count is 0), and *consumed to the bytes read (stops at the first
 * incomplete frame). On RESP_DECODE_ERROR nothing is allocated. Free the values
 * with resp_free and then free(*out_items). */
RespDecodeStatus resp_decode_all(const unsigned char *buffer, size_t len,
                                 RespValue ***out_items, size_t *out_count,
                                 size_t *consumed);

/* ---- streaming decoder ------------------------------------------------ */

/* A stateful decoder that accumulates bytes across feeds and queues whole
 * decoded messages. Once a malformed frame is seen it latches an error. */
typedef struct RespDecoder RespDecoder;

RespDecoder *resp_decoder_new(void);
void resp_decoder_free(RespDecoder *d);

/* Feed more bytes; complete frames are decoded and queued. */
void resp_decoder_feed(RespDecoder *d, const unsigned char *data, size_t len);

/* 1 iff at least one decoded message is queued. */
int resp_decoder_has_message(const RespDecoder *d);

/* Pop the next queued message: returns 1 and sets *out (caller frees) on
 * success, or 0 if the decoder is in an error state or the queue is empty. */
int resp_decoder_get_message(RespDecoder *d, RespValue **out);

/* 1 iff the decoder has latched a decode error. */
int resp_decoder_has_error(const RespDecoder *d);

/* Feed `data`, then hand off every currently-queued message. On success returns
 * 1 and sets *out_items (a malloc'd array of *out_count owned values; NULL when
 * count is 0) — free the values with resp_free, then free(*out_items). Returns 0
 * if the decoder is (or becomes) in an error state, or on allocation failure. */
int resp_decoder_decode_all(RespDecoder *d, const unsigned char *data,
                            size_t len, RespValue ***out_items,
                            size_t *out_count);

#endif /* RESP_PROTOCOL_H */
