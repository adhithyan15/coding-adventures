/*
 * http_core.c — implementation of the shared HTTP helpers.
 * ===========================================================================
 *
 * Everything here is string splitting: paths on '/', targets on '#'/'?', query
 * strings on '&'/'=', Content-Type on ';'/'='. Matching and lookups borrow into
 * the input where they can (route matching splits into spans, not copies) and
 * only allocate for the owned results the caller keeps.
 */
#include "http_core.h"

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* ===========================================================================
 *  Small helpers
 * =========================================================================== */

static char *dup_n(const char *s, size_t n) {
    char *out = malloc(n + 1);
    if (!out) return NULL;
    memcpy(out, s, n);
    out[n] = '\0';
    return out;
}
static char *dup_str(const char *s) { return dup_n(s, strlen(s)); }

/* A borrowed slice of the input. */
typedef struct {
    const char *ptr;
    size_t len;
} Span;

static int is_ws(char c) {
    return c == ' ' || c == '\t' || c == '\n' || c == '\r' || c == '\f' ||
           c == '\v';
}

/* Trim leading/trailing ASCII whitespace from a span. */
static Span trim_span(const char *p, size_t len) {
    while (len > 0 && is_ws(p[0])) {
        p++;
        len--;
    }
    while (len > 0 && is_ws(p[len - 1])) len--;
    Span s = {p, len};
    return s;
}

/* Trim leading/trailing occurrences of `ch` from a span. */
static Span trim_char_span(Span s, char ch) {
    while (s.len > 0 && s.ptr[0] == ch) {
        s.ptr++;
        s.len--;
    }
    while (s.len > 0 && s.ptr[s.len - 1] == ch) s.len--;
    return s;
}

/* ASCII case-insensitive equality of two NUL-terminated strings. */
static int eq_ci(const char *a, const char *b) {
    for (;; a++, b++) {
        char ca = *a, cb = *b;
        if (ca >= 'A' && ca <= 'Z') ca = (char)(ca - 'A' + 'a');
        if (cb >= 'A' && cb <= 'Z') cb = (char)(cb - 'A' + 'a');
        if (ca != cb) return 0;
        if (ca == '\0') return 1;
    }
}

/* ASCII case-insensitive equality of a span and a NUL-terminated string. */
static int span_eq_ci(Span s, const char *cstr) {
    size_t i = 0;
    for (; i < s.len; i++) {
        char ca = s.ptr[i], cb = cstr[i];
        if (cb == '\0') return 0;
        if (ca >= 'A' && ca <= 'Z') ca = (char)(ca - 'A' + 'a');
        if (cb >= 'A' && cb <= 'Z') cb = (char)(cb - 'A' + 'a');
        if (ca != cb) return 0;
    }
    return cstr[i] == '\0';
}

/* Exact equality of a span and a NUL-terminated string. */
static int span_eq_str(Span s, const char *cstr) {
    return strlen(cstr) == s.len && memcmp(cstr, s.ptr, s.len) == 0;
}

/* Parse `len` bytes of `s` as an unsigned integer that must be all ASCII digits
 * and not exceed `limit`. Returns 0 (result in *out) or -1. */
static int parse_uint64(const char *s, size_t len, uint64_t limit,
                        uint64_t *out) {
    if (len == 0) return -1;
    uint64_t v = 0;
    for (size_t i = 0; i < len; i++) {
        char c = s[i];
        if (c < '0' || c > '9') return -1;
        if (v > (limit - (uint64_t)(c - '0')) / 10) return -1; /* would overflow limit */
        v = v * 10 + (uint64_t)(c - '0');
    }
    *out = v;
    return 0;
}

/* Split a path into non-empty '/'-delimited spans ("/" yields none). Writes a
 * malloc'd array to *out (NULL when count 0) and the count to *count_out.
 * Returns 0 or -1 on allocation failure. */
static int split_segments(const char *path, Span **out, size_t *count_out) {
    *out = NULL;
    *count_out = 0;
    if (strcmp(path, "/") == 0) return 0;

    Span *arr = NULL;
    size_t n = 0, cap = 0;
    const char *start = path;
    for (const char *p = path;; p++) {
        if (*p == '/' || *p == '\0') {
            if (p > start) {
                if (n == cap) {
                    size_t ncap = cap ? cap * 2 : 8;
                    if (cap > ((size_t)-1) / 2 / sizeof *arr) {
                        free(arr);
                        return -1;
                    }
                    Span *na = realloc(arr, ncap * sizeof *arr);
                    if (!na) {
                        free(arr);
                        return -1;
                    }
                    arr = na;
                    cap = ncap;
                }
                arr[n].ptr = start;
                arr[n].len = (size_t)(p - start);
                n++;
            }
            if (*p == '\0') break;
            start = p + 1;
        }
    }
    *out = arr;
    *count_out = n;
    return 0;
}

/* Append an owned (name,value) pair to a growable HttpPair array. Consumes
 * `name` and `value` (frees them on failure). Returns 0 or -1. */
static int pairs_push(HttpPair **arr, size_t *n, size_t *cap, char *name,
                      char *value) {
    if (!name || !value) {
        free(name);
        free(value);
        return -1;
    }
    if (*n == *cap) {
        size_t ncap = *cap ? *cap * 2 : 4;
        if (*cap > ((size_t)-1) / 2 / sizeof(HttpPair)) {
            free(name);
            free(value);
            return -1;
        }
        HttpPair *na = realloc(*arr, ncap * sizeof(HttpPair));
        if (!na) {
            free(name);
            free(value);
            return -1;
        }
        *arr = na;
        *cap = ncap;
    }
    (*arr)[*n].name = name;
    (*arr)[*n].value = value;
    (*n)++;
    return 0;
}

static void pairs_free_raw(HttpPair *arr, size_t n) {
    for (size_t i = 0; i < n; i++) {
        free(arr[i].name);
        free(arr[i].value);
    }
    free(arr);
}

void http_pairs_free(HttpPairs *pairs) {
    if (!pairs) return;
    pairs_free_raw(pairs->items, pairs->count);
    pairs->items = NULL;
    pairs->count = 0;
}

/* ===========================================================================
 *  Route patterns
 * =========================================================================== */

typedef struct {
    int is_param;
    char *text; /* literal text, or parameter name */
} RSeg;

struct HttpRoutePattern {
    RSeg *segs;
    size_t n;
};

HttpRoutePattern *http_route_parse(const char *pattern) {
    Span *spans = NULL;
    size_t n = 0;
    if (split_segments(pattern, &spans, &n) != 0) return NULL;

    RSeg *segs = NULL;
    if (n > 0) {
        segs = calloc(n, sizeof *segs);
        if (!segs) {
            free(spans);
            return NULL;
        }
    }
    for (size_t i = 0; i < n; i++) {
        int param = spans[i].len > 0 && spans[i].ptr[0] == ':';
        const char *tp = param ? spans[i].ptr + 1 : spans[i].ptr;
        size_t tl = param ? spans[i].len - 1 : spans[i].len;
        char *t = dup_n(tp, tl);
        if (!t) {
            for (size_t j = 0; j < i; j++) free(segs[j].text);
            free(segs);
            free(spans);
            return NULL;
        }
        segs[i].is_param = param;
        segs[i].text = t;
    }
    free(spans);

    HttpRoutePattern *r = malloc(sizeof *r);
    if (!r) {
        for (size_t i = 0; i < n; i++) free(segs[i].text);
        free(segs);
        return NULL;
    }
    r->segs = segs;
    r->n = n;
    return r;
}

void http_route_free(HttpRoutePattern *r) {
    if (!r) return;
    for (size_t i = 0; i < r->n; i++) free(r->segs[i].text);
    free(r->segs);
    free(r);
}

int http_route_match_path(const HttpRoutePattern *r, const char *path,
                          HttpPairs *out) {
    out->items = NULL;
    out->count = 0;

    Span *spans = NULL;
    size_t n = 0;
    if (split_segments(path, &spans, &n) != 0) return -1;
    if (n != r->n) {
        free(spans);
        return 0; /* segment-count mismatch → no match */
    }

    HttpPair *items = NULL;
    size_t pc = 0, cap = 0;
    for (size_t i = 0; i < n; i++) {
        if (!r->segs[i].is_param) {
            if (!span_eq_str(spans[i], r->segs[i].text)) {
                pairs_free_raw(items, pc);
                free(spans);
                return 0; /* literal mismatch → no match */
            }
        } else {
            char *name = dup_str(r->segs[i].text);
            char *value = dup_n(spans[i].ptr, spans[i].len);
            if (pairs_push(&items, &pc, &cap, name, value) != 0) {
                pairs_free_raw(items, pc);
                free(spans);
                return -1;
            }
        }
    }
    free(spans);
    out->items = items;
    out->count = pc;
    return 1;
}

int http_route_match_target(const HttpRoutePattern *r, const char *target,
                            HttpPairs *out) {
    HttpRequestTarget t;
    if (http_parse_request_target(target, &t) != 0) return -1;
    int rc = http_route_match_path(r, t.path, out);
    http_request_target_free(&t);
    return rc;
}

/* ===========================================================================
 *  Request target & query
 * =========================================================================== */

int http_parse_request_target(const char *target, HttpRequestTarget *out) {
    out->path = NULL;
    out->query = NULL;
    out->fragment = NULL;

    size_t tlen = strlen(target);
    const char *hash = memchr(target, '#', tlen);
    size_t before_len = hash ? (size_t)(hash - target) : tlen;

    char *fragment = NULL;
    if (hash) {
        fragment = dup_str(hash + 1);
        if (!fragment) return -1;
    }

    const char *q = memchr(target, '?', before_len);
    size_t path_len = q ? (size_t)(q - target) : before_len;

    char *path = path_len == 0 ? dup_str("/") : dup_n(target, path_len);
    if (!path) {
        free(fragment);
        return -1;
    }

    char *query = NULL;
    if (q) {
        size_t qlen = before_len - (path_len + 1); /* between '?' and fragment */
        query = dup_n(q + 1, qlen);
        if (!query) {
            free(path);
            free(fragment);
            return -1;
        }
    }

    out->path = path;
    out->query = query;
    out->fragment = fragment;
    return 0;
}

void http_request_target_free(HttpRequestTarget *t) {
    if (!t) return;
    free(t->path);
    free(t->query);
    free(t->fragment);
    t->path = NULL;
    t->query = NULL;
    t->fragment = NULL;
}

int http_query_pairs(const char *query, HttpPairs *out) {
    out->items = NULL;
    out->count = 0;
    if (!query || query[0] == '\0') return 0;

    HttpPair *items = NULL;
    size_t n = 0, cap = 0;
    const char *p = query;
    while (*p) {
        const char *amp = strchr(p, '&');
        const char *end = amp ? amp : p + strlen(p);
        if (end > p) { /* skip empty pieces */
            const char *eq = memchr(p, '=', (size_t)(end - p));
            char *name, *value;
            if (eq) {
                name = dup_n(p, (size_t)(eq - p));
                value = dup_n(eq + 1, (size_t)(end - (eq + 1)));
            } else {
                name = dup_n(p, (size_t)(end - p));
                value = dup_str("");
            }
            if (pairs_push(&items, &n, &cap, name, value) != 0) {
                pairs_free_raw(items, n);
                return -1;
            }
        }
        if (!amp) break;
        p = amp + 1;
    }
    out->items = items;
    out->count = n;
    return 0;
}

int http_query_value(const char *query, const char *name, char **out) {
    HttpPairs p;
    if (http_query_pairs(query, &p) != 0) return -1;
    int rc = 0;
    for (size_t i = 0; i < p.count; i++) {
        if (strcmp(p.items[i].name, name) == 0) {
            char *v = dup_str(p.items[i].value);
            if (!v) {
                http_pairs_free(&p);
                return -1;
            }
            *out = v;
            rc = 1;
            break;
        }
    }
    http_pairs_free(&p);
    return rc;
}

/* ===========================================================================
 *  HTTP version
 * =========================================================================== */

int http_version_parse(const char *text, HttpVersion *out) {
    static const char PREFIX[] = "HTTP/";
    size_t pl = sizeof PREFIX - 1;
    if (strncmp(text, PREFIX, pl) != 0) return -1;
    const char *rest = text + pl;
    const char *dot = strchr(rest, '.');
    if (!dot) return -1;
    uint64_t maj, min;
    if (parse_uint64(rest, (size_t)(dot - rest), 0xFFFF, &maj) != 0) return -1;
    if (parse_uint64(dot + 1, strlen(dot + 1), 0xFFFF, &min) != 0) return -1;
    out->major = (unsigned short)maj;
    out->minor = (unsigned short)min;
    return 0;
}

int http_version_to_string(HttpVersion v, char *buf, size_t buflen) {
    int need = snprintf(buf, buflen, "HTTP/%u.%u", (unsigned)v.major,
                        (unsigned)v.minor);
    if (need < 0 || (size_t)need >= buflen) return -1;
    return need;
}

/* ===========================================================================
 *  Headers
 * =========================================================================== */

const char *http_find_header(const HttpHeader *headers, size_t nheaders,
                             const char *name) {
    for (size_t i = 0; i < nheaders; i++) {
        if (eq_ci(headers[i].name, name)) return headers[i].value;
    }
    return NULL;
}

int http_parse_content_length(const HttpHeader *headers, size_t nheaders,
                              size_t *out) {
    const char *v = http_find_header(headers, nheaders, "Content-Length");
    if (!v) return 0;
    uint64_t val;
    if (parse_uint64(v, strlen(v), (uint64_t)(size_t)-1, &val) != 0) {
        return 0; /* invalid → treated as absent (Rust `.ok()` → None) */
    }
    *out = (size_t)val;
    return 1;
}

int http_parse_content_type(const HttpHeader *headers, size_t nheaders,
                            char **media, char **charset) {
    *media = NULL;
    *charset = NULL;
    const char *v = http_find_header(headers, nheaders, "Content-Type");
    if (!v) return 0;

    char *media_type = NULL;
    char *cs = NULL;
    const char *p = v;
    int first = 1;
    for (;;) {
        const char *semi = strchr(p, ';');
        const char *end = semi ? semi : p + strlen(p);
        Span piece = trim_span(p, (size_t)(end - p));
        if (first) {
            if (piece.len == 0) return 0; /* empty media type → None */
            media_type = dup_n(piece.ptr, piece.len);
            if (!media_type) return -1;
            first = 0;
        } else {
            const char *eq = memchr(piece.ptr, '=', piece.len);
            if (eq) {
                Span nm = trim_span(piece.ptr, (size_t)(eq - piece.ptr));
                if (span_eq_ci(nm, "charset")) {
                    Span val = trim_span(
                        eq + 1, (size_t)(piece.ptr + piece.len - (eq + 1)));
                    val = trim_char_span(val, '"');
                    cs = dup_n(val.ptr, val.len);
                    if (!cs) {
                        free(media_type);
                        return -1;
                    }
                    break; /* first charset wins (Rust find_map) */
                }
            }
        }
        if (!semi) break;
        p = semi + 1;
    }
    *media = media_type;
    *charset = cs; /* NULL when no charset parameter */
    return 1;
}

/* ===========================================================================
 *  Request / response heads (delegating helpers)
 * =========================================================================== */

const char *http_request_head_header(const HttpRequestHead *req,
                                     const char *name) {
    return http_find_header(req->headers, req->nheaders, name);
}

int http_request_head_path(const HttpRequestHead *req, char **out) {
    HttpRequestTarget t;
    if (http_parse_request_target(req->target, &t) != 0) return -1;
    char *p = dup_str(t.path);
    http_request_target_free(&t);
    if (!p) return -1;
    *out = p;
    return 0;
}

int http_request_head_query_value(const HttpRequestHead *req, const char *name,
                                  char **out) {
    HttpRequestTarget t;
    if (http_parse_request_target(req->target, &t) != 0) return -1;
    int rc = http_query_value(t.query, name, out);
    http_request_target_free(&t);
    return rc;
}

int http_request_head_content_length(const HttpRequestHead *req, size_t *out) {
    return http_parse_content_length(req->headers, req->nheaders, out);
}

int http_request_head_content_type(const HttpRequestHead *req, char **media,
                                   char **charset) {
    return http_parse_content_type(req->headers, req->nheaders, media, charset);
}

const char *http_response_head_header(const HttpResponseHead *resp,
                                      const char *name) {
    return http_find_header(resp->headers, resp->nheaders, name);
}

int http_response_head_content_length(const HttpResponseHead *resp,
                                      size_t *out) {
    return http_parse_content_length(resp->headers, resp->nheaders, out);
}

int http_response_head_content_type(const HttpResponseHead *resp, char **media,
                                    char **charset) {
    return http_parse_content_type(resp->headers, resp->nheaders, media,
                                   charset);
}
