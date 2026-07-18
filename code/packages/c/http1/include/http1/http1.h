/*
 * http1/http1.h — HTTP/1.1 request & response head parsing.
 * ===========================================================================
 *
 * The C port of the Rust `http1` crate, a bucket-A port of the CCPP02 campaign:
 * a pure-ISO crate that needs no OS, so it rides the `iso-harness` (links
 * nothing, strict-conformance flags on).
 *
 * HTTP/1 is text-framed: a start line, then header lines, then a blank line, then
 * the body. This crate parses exactly that head boundary — it turns the bytes up
 * to (and including) the blank line into a structured head, and tells the caller
 * where the body starts and how to frame it (Content-Length / chunked / until
 * EOF / none).
 *
 * COMPOSES `c/http-core` for the shared head vocabulary (`HttpVersion`,
 * `HttpHeader`, `HttpRequestHead`, `HttpResponseHead`, `HttpBodyKind`). This
 * package is itself pure-ISO — it compiles that package's source in rather than
 * linking anything.
 *
 * OWNERSHIP. http-core's head structs BORROW their strings (the http-core library
 * never frees them). Since this parser materialises those strings out of the
 * input, each parsed head OWNS the backing storage and points its `head` fields
 * into it. Release a parsed head with the matching `*_free` — do NOT free the
 * `head`'s fields yourself, and do not use them after the free.
 *
 * BODY LENGTH. Rust's `BodyKind::ContentLength(usize)` carries the length inline;
 * http-core's `HttpBodyKind` is a tag only. So a parsed head carries the length
 * separately in `body_length`, meaningful exactly when
 * `body_kind == HTTP_BODY_CONTENT_LENGTH`.
 */
#ifndef HTTP1_HTTP1_H
#define HTTP1_HTTP1_H

#include <stddef.h> /* size_t */

#include "http_core.h" /* HttpVersion, HttpHeader, HttpRequestHead, HttpResponseHead, HttpBodyKind */

#ifdef __cplusplus
extern "C" {
#endif

/*
 * Every result a parse can produce. The Rust `Http1ParseError` variants map to
 * the first block; the last two are C-specific (allocation / NULL argument).
 */
typedef enum {
    HTTP1_OK = 0,
    HTTP1_ERR_INCOMPLETE_HEAD,       /* ran out of bytes before the blank line */
    HTTP1_ERR_INVALID_HEAD_ENCODING, /* a head line is not valid UTF-8 */
    HTTP1_ERR_INVALID_START_LINE,    /* malformed request/response start line */
    HTTP1_ERR_INVALID_HEADER,        /* a header line has no ':' or an empty name */
    HTTP1_ERR_INVALID_VERSION,       /* the "HTTP/x.y" token did not parse */
    HTTP1_ERR_INVALID_STATUS,        /* the status code did not parse as a u16 */
    HTTP1_ERR_INVALID_CONTENT_LENGTH,/* Content-Length is not a valid integer */
    HTTP1_ERR_NOMEM,
    HTTP1_ERR_INVALID                /* NULL argument */
} http1_status;

/*
 * A parsed request head. `head` (borrowing into this struct's owned storage),
 * the body byte offset, and the body framing. Release with
 * http1_parsed_request_free. The trailing fields are owned storage — do not
 * touch them directly.
 */
typedef struct {
    HttpRequestHead head;
    size_t body_offset;
    HttpBodyKind body_kind;
    size_t body_length; /* meaningful iff body_kind == HTTP_BODY_CONTENT_LENGTH */

    /* owned storage (internal): */
    char *owned_method;
    char *owned_target;
    HttpHeader *owned_headers; /* each .name/.value is owned */
    size_t owned_nheaders;
} Http1ParsedRequestHead;

/* A parsed response head; same ownership contract as the request head. */
typedef struct {
    HttpResponseHead head;
    size_t body_offset;
    HttpBodyKind body_kind;
    size_t body_length;

    /* owned storage (internal): */
    char *owned_reason;
    HttpHeader *owned_headers;
    size_t owned_nheaders;
} Http1ParsedResponseHead;

/*
 * Redacted summary of a parsed request head — keeps the method but only the
 * target's *length* (a target can carry credentials/tokens in local traffic).
 * `method` BORROWS from the parsed head, so keep the head alive while using it.
 */
typedef struct {
    const char *method;
    size_t target_len;
    HttpVersion version;
    size_t header_count;
    size_t body_offset;
    HttpBodyKind body_kind;
    size_t body_length;
} Http1RequestHeadSummary;

/* Redacted summary of a parsed response head — keeps only the reason's length. */
typedef struct {
    HttpVersion version;
    unsigned short status;
    size_t reason_len;
    size_t header_count;
    size_t body_offset;
    HttpBodyKind body_kind;
    size_t body_length;
} Http1ResponseHeadSummary;

/*
 * Parse an HTTP/1 request/response head from `input` (of `len` bytes). On success
 * fills *out (release with the matching *_free). The various HTTP1_ERR_* codes
 * report a malformed or incomplete head; HTTP1_ERR_INVALID on a NULL argument
 * (input may be NULL only when len == 0); HTTP1_ERR_NOMEM on allocation failure.
 */
http1_status http1_parse_request_head(const unsigned char *input, size_t len,
                                      Http1ParsedRequestHead *out);
http1_status http1_parse_response_head(const unsigned char *input, size_t len,
                                       Http1ParsedResponseHead *out);

/* Release a parsed head (safe on a zeroed struct). */
void http1_parsed_request_free(Http1ParsedRequestHead *p);
void http1_parsed_response_free(Http1ParsedResponseHead *p);

/* Redacted summaries (the `method` in the request summary borrows from *p). */
Http1RequestHeadSummary http1_request_summary(const Http1ParsedRequestHead *p);
Http1ResponseHeadSummary http1_response_summary(const Http1ParsedResponseHead *p);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* HTTP1_HTTP1_H */
