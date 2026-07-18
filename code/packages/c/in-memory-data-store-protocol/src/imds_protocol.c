/*
 * imds_protocol.c — in-memory data store protocol IR (implementation).
 * ===========================================================================
 *
 * A faithful C port of the Rust `in-memory-data-store-protocol` crate. The Rust
 * is tiny — a `CommandFrame` struct with `from_parts`, and an `EngineResponse`
 * enum with a handful of constructors — but it is a *real* recursive data type
 * with owned buffers, so the port is a good exercise in doing C ownership
 * by hand: every String/Vec becomes a `malloc`'d buffer we are responsible for,
 * and the recursive `Array(Vec<EngineResponse>)` becomes a recursively-freed tree.
 *
 * The Rust never allocates fallibly (String/Vec abort on OOM), so nothing there
 * returns a Result. We are stricter: constructors that allocate report
 * IMDS_ERR_NOMEM instead of aborting, and unwind cleanly so a partially-built
 * value never leaks.
 */
#include "imds_protocol/imds_protocol.h"

#include <stdlib.h> /* malloc, free */
#include <string.h> /* memcpy, strlen */

/* ------------------------------------------------------------------------- *
 * Small helpers
 * ------------------------------------------------------------------------- */

/*
 * imds__memdup — copy `len` bytes into a fresh buffer. We over-allocate by one
 * and NUL-terminate so a caller may treat the result as a C string when the
 * payload happens to be text, but `len` remains the source of truth (payloads
 * may embed NULs). A zero-length copy still returns a valid 1-byte buffer, never
 * NULL, so "empty" is distinguishable from "allocation failed".
 */
static unsigned char *imds__memdup(const unsigned char *src, size_t len) {
    unsigned char *dst;
    /* len + 1 must not wrap, and a positive length needs real bytes to copy
     * from (a NULL source with len > 0 is a malformed arg — reject, don't UB). */
    if (len == (size_t)-1 || (len > 0 && !src)) {
        return NULL;
    }
    dst = (unsigned char *)malloc(len + 1);
    if (!dst) {
        return NULL;
    }
    if (len > 0) {
        memcpy(dst, src, len);
    }
    dst[len] = '\0';
    return dst;
}

/* imds__strdup — duplicate a NUL-terminated string (ISO C has no strdup). */
static char *imds__strdup(const char *s) {
    size_t len = strlen(s);
    char *dst = (char *)malloc(len + 1);
    if (!dst) {
        return NULL;
    }
    memcpy(dst, s, len + 1);
    return dst;
}

/*
 * imds__ascii_upper_dup — the Rust `ascii_upper`, byte-exact.
 *
 * The Rust is `bytes.iter().map(|b| b.to_ascii_uppercase() as char).collect()`.
 * The subtlety is `as char`: each *byte* (after the ASCII-uppercase shift of
 * 'a'..='z') becomes the Unicode scalar value with that number (Latin-1), and
 * `collect::<String>()` then UTF-8-encodes it. So an ASCII byte stays one byte,
 * but a byte ≥ 0x80 EXPANDS to the two-byte UTF-8 encoding of U+0080..U+00FF
 * (0xC2/0xC3 lead + continuation). We reproduce exactly that, so the resulting
 * C string equals the Rust `command` field's `.as_bytes()` for any input —
 * including non-ASCII command names, not just the common all-ASCII case.
 *
 * Worst-case output is 2·len bytes (+1 for the terminator).
 */
static char *imds__ascii_upper_dup(const unsigned char *bytes, size_t len) {
    char *dst;
    size_t i;
    size_t o = 0;
    /* Each input byte emits at most 2 output bytes, plus the terminator; guard
     * the 2*len+1 sizing against size_t wrap before allocating. */
    if (len > ((size_t)-1 - 1) / 2) {
        return NULL;
    }
    if (len > 0 && !bytes) {
        return NULL;
    }
    dst = (char *)malloc(2 * len + 1);
    if (!dst) {
        return NULL;
    }
    for (i = 0; i < len; i++) {
        unsigned char b = bytes[i];
        if (b >= 'a' && b <= 'z') {
            b = (unsigned char)(b - ('a' - 'A'));
        }
        if (b < 0x80) {
            dst[o++] = (char)b;
        } else {
            /* U+0080..U+00FF → two-byte UTF-8. */
            dst[o++] = (char)(0xC0 | (b >> 6));
            dst[o++] = (char)(0x80 | (b & 0x3F));
        }
    }
    dst[o] = '\0';
    return dst;
}

/*
 * imds__args_dup — deep-copy `nargs` args into a fresh array. On any failure,
 * unwinds everything allocated so far and returns NULL. `nargs == 0` yields a
 * NULL array (an empty arg list needs no storage), which the free path treats
 * as a no-op.
 */
static imds_arg *imds__args_dup(const imds_arg *args, size_t nargs) {
    imds_arg *copy;
    size_t i;
    if (nargs == 0) {
        return NULL;
    }
    copy = (imds_arg *)calloc(nargs, sizeof(*copy));
    if (!copy) {
        return NULL;
    }
    for (i = 0; i < nargs; i++) {
        copy[i].bytes = imds__memdup(args[i].bytes, args[i].len);
        if (!copy[i].bytes) {
            /* Unwind the prefix we already copied. */
            size_t j;
            for (j = 0; j < i; j++) {
                free(copy[j].bytes);
            }
            free(copy);
            return NULL;
        }
        copy[i].len = args[i].len;
    }
    return copy;
}

/* ------------------------------------------------------------------------- *
 * CommandFrame
 * ------------------------------------------------------------------------- */

imds_status imds_command_frame_new(const char *command, const imds_arg *args,
                                   size_t nargs, imds_command_frame *out) {
    char *cmd_copy;
    imds_arg *args_copy;
    if (!out || !command || (!args && nargs > 0)) {
        return IMDS_ERR_INVALID;
    }
    cmd_copy = imds__strdup(command);
    if (!cmd_copy) {
        return IMDS_ERR_NOMEM;
    }
    args_copy = imds__args_dup(args, nargs);
    if (nargs > 0 && !args_copy) {
        free(cmd_copy);
        return IMDS_ERR_NOMEM;
    }
    out->command = cmd_copy;
    out->args = args_copy;
    out->nargs = nargs;
    return IMDS_OK;
}

imds_status imds_command_frame_from_parts(const imds_arg *parts, size_t nparts,
                                          imds_command_frame *out) {
    char *cmd_copy;
    imds_arg *args_copy;
    size_t nargs;
    if (!out) {
        return IMDS_ERR_INVALID;
    }
    /* Rust: parts.split_first()? — None (here IMDS_NONE) on an empty list. */
    if (nparts == 0) {
        return IMDS_NONE;
    }
    if (!parts) {
        return IMDS_ERR_INVALID;
    }
    /* First part → uppercased command; the remainder → args. */
    cmd_copy = imds__ascii_upper_dup(parts[0].bytes, parts[0].len);
    if (!cmd_copy) {
        return IMDS_ERR_NOMEM;
    }
    nargs = nparts - 1;
    args_copy = imds__args_dup(parts + 1, nargs);
    if (nargs > 0 && !args_copy) {
        free(cmd_copy);
        return IMDS_ERR_NOMEM;
    }
    out->command = cmd_copy;
    out->args = args_copy;
    out->nargs = nargs;
    return IMDS_OK;
}

void imds_command_frame_free(imds_command_frame *frame) {
    size_t i;
    if (!frame) {
        return;
    }
    free(frame->command);
    for (i = 0; i < frame->nargs; i++) {
        free(frame->args[i].bytes);
    }
    free(frame->args);
    frame->command = NULL;
    frame->args = NULL;
    frame->nargs = 0;
}

/* ------------------------------------------------------------------------- *
 * EngineResponse
 * ------------------------------------------------------------------------- */

imds_status imds_resp_simple_string(const char *s, imds_engine_response *out) {
    char *copy;
    if (!out || !s) {
        return IMDS_ERR_INVALID;
    }
    copy = imds__strdup(s);
    if (!copy) {
        return IMDS_ERR_NOMEM;
    }
    out->kind = IMDS_RESP_SIMPLE_STRING;
    out->as.str = copy;
    return IMDS_OK;
}

imds_status imds_resp_error(const char *e, imds_engine_response *out) {
    char *copy;
    if (!out || !e) {
        return IMDS_ERR_INVALID;
    }
    copy = imds__strdup(e);
    if (!copy) {
        return IMDS_ERR_NOMEM;
    }
    out->kind = IMDS_RESP_ERROR;
    out->as.str = copy;
    return IMDS_OK;
}

imds_status imds_resp_integer(int64_t v, imds_engine_response *out) {
    if (!out) {
        return IMDS_ERR_INVALID;
    }
    out->kind = IMDS_RESP_INTEGER;
    out->as.integer = v;
    return IMDS_OK;
}

imds_status imds_resp_bulk_string(const unsigned char *bytes, size_t len,
                                  imds_engine_response *out) {
    unsigned char *copy;
    if (!out || (!bytes && len > 0)) {
        return IMDS_ERR_INVALID;
    }
    copy = imds__memdup(bytes, len);
    if (!copy) {
        return IMDS_ERR_NOMEM;
    }
    out->kind = IMDS_RESP_BULK_STRING;
    out->as.bulk.is_null = 0;
    out->as.bulk.bytes = copy;
    out->as.bulk.len = len;
    return IMDS_OK;
}

imds_status imds_resp_bulk_null(imds_engine_response *out) {
    if (!out) {
        return IMDS_ERR_INVALID;
    }
    out->kind = IMDS_RESP_BULK_STRING;
    out->as.bulk.is_null = 1;
    out->as.bulk.bytes = NULL;
    out->as.bulk.len = 0;
    return IMDS_OK;
}

imds_status imds_resp_array(imds_engine_response *items, size_t n,
                            imds_engine_response *out) {
    if (!out || (!items && n > 0)) {
        return IMDS_ERR_INVALID;
    }
    /* We take ownership of the caller's `items` buffer as-is — no copy — so an
     * array is assembled by building its children then handing them over. */
    out->kind = IMDS_RESP_ARRAY;
    out->as.array.is_null = 0;
    out->as.array.items = items;
    out->as.array.n = n;
    return IMDS_OK;
}

imds_status imds_resp_array_null(imds_engine_response *out) {
    if (!out) {
        return IMDS_ERR_INVALID;
    }
    out->kind = IMDS_RESP_ARRAY;
    out->as.array.is_null = 1;
    out->as.array.items = NULL;
    out->as.array.n = 0;
    return IMDS_OK;
}

/* +OK — the ubiquitous success reply. */
imds_status imds_resp_ok(imds_engine_response *out) {
    return imds_resp_simple_string("OK", out);
}

/* Rust `null()` is BulkString(None), not Array(None). */
imds_status imds_resp_null(imds_engine_response *out) {
    return imds_resp_bulk_null(out);
}

imds_status imds_resp_zero(imds_engine_response *out) {
    return imds_resp_integer(0, out);
}

imds_status imds_resp_one(imds_engine_response *out) {
    return imds_resp_integer(1, out);
}

void imds_engine_response_free(imds_engine_response *resp) {
    if (!resp) {
        return;
    }
    switch (resp->kind) {
    case IMDS_RESP_SIMPLE_STRING:
    case IMDS_RESP_ERROR:
        free(resp->as.str);
        resp->as.str = NULL;
        break;
    case IMDS_RESP_BULK_STRING:
        free(resp->as.bulk.bytes);
        resp->as.bulk.bytes = NULL;
        break;
    case IMDS_RESP_ARRAY: {
        size_t i;
        for (i = 0; i < resp->as.array.n; i++) {
            imds_engine_response_free(&resp->as.array.items[i]);
        }
        free(resp->as.array.items);
        resp->as.array.items = NULL;
        resp->as.array.n = 0;
        break;
    }
    case IMDS_RESP_INTEGER:
        /* nothing owned */
        break;
    }
}
