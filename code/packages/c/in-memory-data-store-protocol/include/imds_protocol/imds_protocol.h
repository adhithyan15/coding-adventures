/*
 * imds_protocol/imds_protocol.h — in-memory data store protocol IR.
 * ===========================================================================
 *
 * The C port of the Rust `in-memory-data-store-protocol` crate, and the first
 * **bucket-A** port of the CCPP02 campaign: a pure-ISO crate that needs no OS at
 * all, so it rides the `iso-harness` (which links nothing and compiles with
 * `-pedantic-errors`) rather than os-platform.
 *
 * It is the little intermediate representation a Redis-style ("RESP") server uses
 * between the wire and its engine:
 *
 *   - a COMMAND FRAME — an uppercased command name plus a vector of raw byte args,
 *     the shape a decoded request takes on its way into the engine; and
 *   - an ENGINE RESPONSE — the RESP reply tree the engine hands back
 *     (simple string, error, integer, bulk string, or a (possibly-null) array of
 *     nested responses).
 *
 * OWNERSHIP. Both types own heap data (the command string / arg bytes; the
 * response's string / bytes / nested items). Every value you construct or receive
 * through an out-parameter must be released — imds_command_frame_free for a frame,
 * imds_engine_response_free for a response (which frees a whole nested array
 * tree). Both are safe on a zeroed value.
 */
#ifndef IMDS_PROTOCOL_IMDS_PROTOCOL_H
#define IMDS_PROTOCOL_IMDS_PROTOCOL_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* int64_t */

#ifdef __cplusplus
extern "C" {
#endif

/* Every result the protocol IR can produce. */
typedef enum {
    IMDS_OK = 0,
    IMDS_NONE,         /* from_parts on an empty part list (Rust's Option::None) */
    IMDS_ERR_INVALID,  /* NULL out-parameter, etc. */
    IMDS_ERR_NOMEM     /* allocation failure */
} imds_status;

/* One raw argument: a length-counted byte string (may contain NULs / zero len). */
typedef struct {
    unsigned char *bytes;
    size_t len;
} imds_arg;

/*
 * A decoded command: an (uppercased) command name and its argument list. Owns
 * `command` and every arg's bytes; release with imds_command_frame_free.
 */
typedef struct {
    char *command;
    imds_arg *args;
    size_t nargs;
} imds_command_frame;

/*
 * imds_command_frame_new — build a frame from a command name and args. The name
 * and every arg's bytes are copied. IMDS_ERR_INVALID (out/command NULL), IMDS_ERR_NOMEM.
 */
imds_status imds_command_frame_new(const char *command, const imds_arg *args,
                                   size_t nargs, imds_command_frame *out);

/*
 * imds_command_frame_from_parts — build a frame from decoded wire parts: the first
 * part becomes the command (ASCII-uppercased), the rest become args. All bytes are
 * copied. Returns IMDS_NONE (leaving *out untouched) when nparts == 0, mirroring
 * the Rust Option::None. IMDS_ERR_INVALID / IMDS_ERR_NOMEM.
 */
imds_status imds_command_frame_from_parts(const imds_arg *parts, size_t nparts,
                                          imds_command_frame *out);

/* Release a frame (safe on a zeroed struct). */
void imds_command_frame_free(imds_command_frame *frame);

/* The RESP reply variants. */
typedef enum {
    IMDS_RESP_SIMPLE_STRING,
    IMDS_RESP_ERROR,
    IMDS_RESP_INTEGER,
    IMDS_RESP_BULK_STRING, /* is_null distinguishes $-1 from a (possibly empty) blob */
    IMDS_RESP_ARRAY        /* is_null distinguishes *-1 from a (possibly empty) array */
} imds_resp_kind;

/*
 * An engine response. A tagged union; IMDS_RESP_ARRAY makes it a tree. Owns its
 * heap payload; release with imds_engine_response_free.
 */
typedef struct imds_engine_response imds_engine_response;
struct imds_engine_response {
    imds_resp_kind kind;
    union {
        char *str;      /* SIMPLE_STRING / ERROR */
        int64_t integer; /* INTEGER */
        struct {
            int is_null;
            unsigned char *bytes;
            size_t len;
        } bulk;         /* BULK_STRING */
        struct {
            int is_null;
            imds_engine_response *items;
            size_t n;
        } array;        /* ARRAY */
    } as;
};

/* Constructors. Each fills *out (owning any heap it allocates). Non-array
 * constructors that allocate return IMDS_ERR_NOMEM on failure; the trivial ones
 * cannot fail. All take a non-NULL out (IMDS_ERR_INVALID otherwise). */
imds_status imds_resp_simple_string(const char *s, imds_engine_response *out);
imds_status imds_resp_error(const char *e, imds_engine_response *out);
imds_status imds_resp_integer(int64_t v, imds_engine_response *out);
/* BULK_STRING carrying bytes (copied; len may be 0). */
imds_status imds_resp_bulk_string(const unsigned char *bytes, size_t len,
                                  imds_engine_response *out);
/* BULK_STRING null ($-1). */
imds_status imds_resp_bulk_null(imds_engine_response *out);
/*
 * ARRAY taking ownership of `items` (an array of `n` responses, previously
 * constructed). `items` MUST be a single heap block from malloc/calloc — the
 * free path calls `free(items)` after freeing each element, so a stack array,
 * a sub-object, or an aliased pointer would corrupt the heap. On success the
 * array adopts the block (do not free it yourself, and do not reuse the elements);
 * on IMDS_ERR_INVALID ownership is NOT transferred and the caller still owns
 * `items`. Pass items=NULL/n=0 for an empty array.
 */
imds_status imds_resp_array(imds_engine_response *items, size_t n,
                            imds_engine_response *out);
/* ARRAY null (*-1). */
imds_status imds_resp_array_null(imds_engine_response *out);

/* Convenience responses mirroring the Rust helpers. */
imds_status imds_resp_ok(imds_engine_response *out);   /* +OK */
imds_status imds_resp_null(imds_engine_response *out); /* $-1 (bulk null) */
imds_status imds_resp_zero(imds_engine_response *out); /* :0 */
imds_status imds_resp_one(imds_engine_response *out);  /* :1 */

/* Release a response, recursively freeing a nested array tree (safe on zeroed). */
void imds_engine_response_free(imds_engine_response *resp);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* IMDS_PROTOCOL_IMDS_PROTOCOL_H */
