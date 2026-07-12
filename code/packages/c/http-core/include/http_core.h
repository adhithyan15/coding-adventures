/*
 * http_core.h — shared HTTP message types and helpers, in pure ISO C17. A
 * faithful port of the Rust `http-core` crate.
 * ===========================================================================
 *
 * Version-specific parsers disagree about wire syntax, but they should agree
 * about the semantic shapes application code consumes. This crate provides those
 * shared shapes — headers, versions, request/response heads, body-framing hints
 * — plus the syntax-level helpers that read them: route-pattern matching,
 * request-target splitting, query-pair iteration, and Content-* parsing.
 *
 * SCOPE. A syntax-level core: query values are NOT percent-decoded, so a caller
 * can apply its own decoding policy.
 *
 * OWNERSHIP. Functions that produce variable-length results write malloc'd data
 * through an out-parameter and return a status; release each with the matching
 * `*_free`. Headers are passed in as caller-owned `HttpHeader` arrays (borrowed,
 * never freed by this library).
 *
 * DIVERGENCE FROM RUST. Rust `Result`/`Option` become status codes: parsing
 * that Rust reports as `Err(String)` is reported here as a return of -1 / 0
 * without the error text (the semantic outcome is identical).
 *
 * PORTABILITY. Pure ISO C17 — no POSIX strdup/strndup, no extensions. Builds
 * clean under GCC, Clang, and MSVC with -pedantic-errors / /permissive- and
 * warnings-as-errors.
 */
#ifndef CA_HTTP_CORE_H
#define CA_HTTP_CORE_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* A name/value pair with owned strings (a captured route param, or a query
 * pair). */
typedef struct {
    char *name;
    char *value;
} HttpPair;

/* A batch of pairs. */
typedef struct {
    HttpPair *items; /* NULL when count == 0 */
    size_t count;
} HttpPairs;

/* Release a pair batch. */
void http_pairs_free(HttpPairs *pairs);

/* ── Route patterns ─────────────────────────────────────────────────────── */

/* An opaque parsed path pattern such as "/hello/:name". */
typedef struct HttpRoutePattern HttpRoutePattern;

/* Parse a route pattern. Returns NULL on allocation failure. */
HttpRoutePattern *http_route_parse(const char *pattern);
void http_route_free(HttpRoutePattern *r);

/* Match a path, capturing (name, value) for each `:param` segment. Returns 1 on
 * a match (fills *out; release with http_pairs_free), 0 on no match (*out
 * zeroed), or -1 on allocation failure. */
int http_route_match_path(const HttpRoutePattern *r, const char *path,
                          HttpPairs *out);
/* As above, but matches only the path portion of a full request target, so a
 * query string cannot make a valid route miss. */
int http_route_match_target(const HttpRoutePattern *r, const char *target,
                            HttpPairs *out);

/* ── Request target & query ─────────────────────────────────────────────── */

/* An origin-form request target split into owned parts; `query` / `fragment`
 * are NULL when absent. */
typedef struct {
    char *path;
    char *query;
    char *fragment;
} HttpRequestTarget;

/* Split `target` into path/query/fragment. Returns 0 (fills *out; release with
 * http_request_target_free) or -1 on allocation failure. */
int http_parse_request_target(const char *target, HttpRequestTarget *out);
void http_request_target_free(HttpRequestTarget *t);

/* Collect the raw (name, value) pairs of a query string (may be NULL → empty);
 * values are NOT decoded. Returns 0 (fills *out) or -1 on allocation failure. */
int http_query_pairs(const char *query, HttpPairs *out);

/* First value for `name` in a query string. Returns 1 (*out = malloc'd copy;
 * caller frees), 0 if absent, or -1 on allocation failure. */
int http_query_value(const char *query, const char *name, char **out);

/* ── HTTP version ───────────────────────────────────────────────────────── */

typedef struct {
    unsigned short major;
    unsigned short minor;
} HttpVersion;

/* Parse a textual "HTTP/x.y" marker. Returns 0 (fills *out) or -1 if malformed. */
int http_version_parse(const char *text, HttpVersion *out);

/* Render "HTTP/x.y" into `buf`. Returns the length written (excluding the NUL),
 * or -1 if `buf` is too small. */
int http_version_to_string(HttpVersion v, char *buf, size_t buflen);

/* ── Headers ────────────────────────────────────────────────────────────── */

/* One HTTP header line (borrowed; the library never frees these). */
typedef struct {
    const char *name;
    const char *value;
} HttpHeader;

/* First header value matching `name` (ASCII case-insensitive), or NULL. The
 * returned pointer borrows from the header array. */
const char *http_find_header(const HttpHeader *headers, size_t nheaders,
                             const char *name);

/* Content-Length when present and a valid integer. Returns 1 (*out set) or 0. */
int http_parse_content_length(const HttpHeader *headers, size_t nheaders,
                              size_t *out);

/* Content-Type split into media type and optional charset. Returns 1 (*media
 * malloc'd; *charset malloc'd or NULL when absent; free each), 0 if no valid
 * Content-Type, or -1 on allocation failure. */
int http_parse_content_type(const HttpHeader *headers, size_t nheaders,
                            char **media, char **charset);

/* ── Body framing, request/response heads ───────────────────────────────── */

typedef enum {
    HTTP_BODY_NONE,
    HTTP_BODY_CONTENT_LENGTH,
    HTTP_BODY_UNTIL_EOF,
    HTTP_BODY_CHUNKED
} HttpBodyKind;

/* Caller-owned request head (fields borrow the caller's strings/array). */
typedef struct {
    const char *method;
    const char *target;
    HttpVersion version;
    const HttpHeader *headers;
    size_t nheaders;
} HttpRequestHead;

/* Delegating helpers mirroring the Rust methods. */
const char *http_request_head_header(const HttpRequestHead *req,
                                     const char *name);
int http_request_head_path(const HttpRequestHead *req, char **out); /* 0/-1 */
int http_request_head_query_value(const HttpRequestHead *req, const char *name,
                                  char **out); /* 1/0/-1 */
int http_request_head_content_length(const HttpRequestHead *req,
                                     size_t *out); /* 1/0 */
int http_request_head_content_type(const HttpRequestHead *req, char **media,
                                   char **charset); /* 1/0/-1 */

/* Caller-owned response head. */
typedef struct {
    HttpVersion version;
    unsigned short status;
    const char *reason;
    const HttpHeader *headers;
    size_t nheaders;
} HttpResponseHead;

const char *http_response_head_header(const HttpResponseHead *resp,
                                      const char *name);
int http_response_head_content_length(const HttpResponseHead *resp,
                                      size_t *out); /* 1/0 */
int http_response_head_content_type(const HttpResponseHead *resp, char **media,
                                    char **charset); /* 1/0/-1 */

#ifdef __cplusplus
}
#endif

#endif /* CA_HTTP_CORE_H */
