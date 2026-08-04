/*
 * http1.c — HTTP/1.1 request & response head parsing (implementation).
 * ===========================================================================
 *
 * A faithful C port of the Rust `http1` crate. The parse is three stages:
 *
 *   1. split_head_lines — carve the byte input into head lines (LF-terminated,
 *      trailing CR stripped), stopping at the blank line; that blank line's end
 *      is the body offset.
 *   2. start line + headers — tokenise the first line (method/target/version, or
 *      version/status/reason) and split each remaining line at its first ':'.
 *   3. body framing — decide Content-Length / chunked / until-EOF / none from the
 *      headers (and, for responses, the status code).
 *
 * http-core's head structs borrow their strings, so the final step materialises
 * owned copies (out of the input bytes) and points the head at them; the parsed
 * head owns all of that and frees it in the matching *_free.
 */
#include "http1/http1.h"

#include <stdlib.h> /* malloc, realloc, free */
#include <string.h> /* memcpy */

/* ------------------------------------------------------------------------- *
 * Small helpers
 * ------------------------------------------------------------------------- */

typedef struct {
    const char *ptr;
    size_t len;
} bspan; /* a borrowed byte range */

typedef struct {
    const char *nptr;
    size_t nlen;
    const char *vptr;
    size_t vlen;
} hspan; /* a borrowed header name/value pair */

static int is_ws(unsigned char c) {
    return c == ' ' || c == '\t' || c == '\n' || c == '\r' || c == '\f' || c == '\v';
}

/* Duplicate n bytes into a fresh NUL-terminated buffer (NULL on OOM). */
static char *dupn(const char *p, size_t n) {
    char *d = (char *)malloc(n + 1);
    if (!d) {
        return NULL;
    }
    if (n > 0) {
        memcpy(d, p, n);
    }
    d[n] = '\0';
    return d;
}

/* Trim all ASCII whitespace from both ends of the span. */
static void trim_ws(const char **p, size_t *n) {
    while (*n > 0 && is_ws((unsigned char)(*p)[0])) {
        (*p)++;
        (*n)--;
    }
    while (*n > 0 && is_ws((unsigned char)(*p)[*n - 1])) {
        (*n)--;
    }
}

/* Trim only spaces and tabs from both ends (the Rust header-value trim set). */
static void trim_sp_tab(const char **p, size_t *n) {
    while (*n > 0 && ((*p)[0] == ' ' || (*p)[0] == '\t')) {
        (*p)++;
        (*n)--;
    }
    while (*n > 0 && ((*p)[*n - 1] == ' ' || (*p)[*n - 1] == '\t')) {
        (*n)--;
    }
}

/* ASCII case-insensitive equality of a span against a NUL-terminated string. */
static int ci_eq(const char *a, size_t alen, const char *b) {
    size_t i;
    for (i = 0; i < alen; i++) {
        unsigned char x = (unsigned char)a[i];
        unsigned char y = (unsigned char)b[i];
        if (y == '\0') {
            return 0;
        }
        if (x >= 'A' && x <= 'Z') {
            x = (unsigned char)(x + 32);
        }
        if (y >= 'A' && y <= 'Z') {
            y = (unsigned char)(y + 32);
        }
        if (x != y) {
            return 0;
        }
    }
    return b[alen] == '\0';
}

/*
 * Validate a byte range as UTF-8 (the Rust `from_utf8` gate on head lines).
 * Rejects overlong forms, surrogates, and out-of-range code points.
 */
static int utf8_valid(const char *s, size_t n) {
    const unsigned char *b = (const unsigned char *)s;
    size_t i = 0;
    while (i < n) {
        unsigned char c = b[i];
        size_t need;
        unsigned long cp;
        unsigned long min_cp;
        if (c < 0x80) {
            i++;
            continue;
        } else if ((c & 0xE0) == 0xC0) {
            need = 1;
            cp = c & 0x1FU;
            min_cp = 0x80;
        } else if ((c & 0xF0) == 0xE0) {
            need = 2;
            cp = c & 0x0FU;
            min_cp = 0x800;
        } else if ((c & 0xF8) == 0xF0) {
            need = 3;
            cp = c & 0x07U;
            min_cp = 0x10000;
        } else {
            return 0;
        }
        if (i + need + 1 > n) {
            return 0;
        }
        {
            size_t k;
            for (k = 1; k <= need; k++) {
                unsigned char cc = b[i + k];
                if ((cc & 0xC0) != 0x80) {
                    return 0;
                }
                cp = (cp << 6) | (cc & 0x3FU);
            }
        }
        if (cp < min_cp || (cp >= 0xD800 && cp <= 0xDFFF) || cp > 0x10FFFF) {
            return 0;
        }
        i += need + 1;
    }
    return 1;
}

/* ------------------------------------------------------------------------- *
 * A tiny growable pointer array (used for lines and tokens).
 * ------------------------------------------------------------------------- */

/* Ensure room for one more element; returns the (possibly moved) base or NULL. */
static void *grow_one(void *base, size_t used, size_t *cap, size_t elem) {
    size_t new_cap;
    void *grown;
    if (used < *cap) {
        return base;
    }
    new_cap = (*cap == 0) ? 8 : *cap * 2;
    if (*cap > ((size_t)-1) / elem / 2) {
        return NULL;
    }
    grown = realloc(base, new_cap * elem);
    if (!grown) {
        return NULL;
    }
    *cap = new_cap;
    return grown;
}

/* ------------------------------------------------------------------------- *
 * Stage 1 — split the head into lines
 * ------------------------------------------------------------------------- */

/*
 * Carve `input` into head lines. Leading blank lines are skipped; each line runs
 * to the next '\n' (a trailing '\r' is dropped); the first empty line ends the
 * head and its end offset becomes *body_offset. Returns a malloc'd `*lines`
 * array of `*nlines` borrowed spans (caller frees the array, not the spans).
 */
static http1_status split_head_lines(const unsigned char *input, size_t len,
                                     bspan **lines_out, size_t *nlines_out,
                                     size_t *body_offset) {
    bspan *lines = NULL;
    size_t n = 0, cap = 0;
    size_t index = 0;

    /* Skip leading blank lines (CRLF or bare LF). */
    while (index < len) {
        if (index + 1 < len && input[index] == '\r' && input[index + 1] == '\n') {
            index += 2;
        } else if (input[index] == '\n') {
            index += 1;
        } else {
            break;
        }
    }

    for (;;) {
        size_t line_start, line_end;
        if (index >= len) {
            free(lines);
            return HTTP1_ERR_INCOMPLETE_HEAD;
        }
        line_start = index;
        while (index < len && input[index] != '\n') {
            index++;
        }
        if (index >= len) {
            free(lines); /* no line terminator → head not finished */
            return HTTP1_ERR_INCOMPLETE_HEAD;
        }
        line_end = (index > line_start && input[index - 1] == '\r') ? index - 1 : index;
        index++; /* past the '\n' */

        if (line_end == line_start) {
            /* Blank line: the head ends; the body starts here. */
            *lines_out = lines;
            *nlines_out = n;
            *body_offset = index;
            return HTTP1_OK;
        }
        {
            bspan *grown = (bspan *)grow_one(lines, n, &cap, sizeof(*lines));
            if (!grown) {
                free(lines);
                return HTTP1_ERR_NOMEM;
            }
            lines = grown;
            lines[n].ptr = (const char *)input + line_start;
            lines[n].len = line_end - line_start;
            n++;
        }
    }
}

/* ------------------------------------------------------------------------- *
 * Stage 2 — tokens, headers, version, status
 * ------------------------------------------------------------------------- */

/* Collect whitespace-delimited tokens of a line into a malloc'd span array. */
static http1_status tokenize_ws(const char *p, size_t len, bspan **out, size_t *nout) {
    bspan *toks = NULL;
    size_t n = 0, cap = 0;
    size_t i = 0;
    while (i < len) {
        size_t start;
        while (i < len && is_ws((unsigned char)p[i])) {
            i++;
        }
        if (i >= len) {
            break;
        }
        start = i;
        while (i < len && !is_ws((unsigned char)p[i])) {
            i++;
        }
        {
            bspan *grown = (bspan *)grow_one(toks, n, &cap, sizeof(*toks));
            if (!grown) {
                free(toks);
                return HTTP1_ERR_NOMEM;
            }
            toks = grown;
            toks[n].ptr = p + start;
            toks[n].len = i - start;
            n++;
        }
    }
    *out = toks;
    *nout = n;
    return HTTP1_OK;
}

/* Parse header lines into borrowed name/value spans. */
static http1_status parse_headers(const bspan *lines, size_t nlines,
                                  hspan **out, size_t *nout) {
    hspan *hs = NULL;
    size_t n = 0, cap = 0;
    size_t li;
    for (li = 0; li < nlines; li++) {
        const char *text = lines[li].ptr;
        size_t tlen = lines[li].len;
        size_t colon;
        const char *nptr;
        size_t nlen;
        const char *vptr;
        size_t vlen;
        hspan *grown;
        if (!utf8_valid(text, tlen)) {
            free(hs);
            return HTTP1_ERR_INVALID_HEAD_ENCODING;
        }
        /* split_once(':') — first colon. */
        colon = 0;
        while (colon < tlen && text[colon] != ':') {
            colon++;
        }
        if (colon == tlen) {
            free(hs);
            return HTTP1_ERR_INVALID_HEADER; /* no ':' */
        }
        nptr = text;
        nlen = colon;
        trim_ws(&nptr, &nlen); /* name: trim all whitespace */
        if (nlen == 0) {
            free(hs);
            return HTTP1_ERR_INVALID_HEADER; /* empty name */
        }
        vptr = text + colon + 1;
        vlen = tlen - colon - 1;
        trim_sp_tab(&vptr, &vlen); /* value: trim spaces/tabs only */

        grown = (hspan *)grow_one(hs, n, &cap, sizeof(*hs));
        if (!grown) {
            free(hs);
            return HTTP1_ERR_NOMEM;
        }
        hs = grown;
        hs[n].nptr = nptr;
        hs[n].nlen = nlen;
        hs[n].vptr = vptr;
        hs[n].vlen = vlen;
        n++;
    }
    *out = hs;
    *nout = n;
    return HTTP1_OK;
}

/* Parse a decimal token as an unsigned value with an inclusive max. Returns 1 on
 * success (*out set), 0 on empty / non-digit / overflow. */
static int parse_uint(const char *p, size_t len, unsigned long max, unsigned long *out) {
    unsigned long v = 0;
    size_t i;
    if (len == 0) {
        return 0;
    }
    for (i = 0; i < len; i++) {
        unsigned d;
        if (p[i] < '0' || p[i] > '9') {
            return 0;
        }
        d = (unsigned)(p[i] - '0');
        /* Guard d > max before the (max - d) subtraction as well, so the helper
         * stays correct even if reused with a small max. */
        if (d > max || v > (max - d) / 10) {
            return 0;
        }
        v = v * 10 + d;
    }
    *out = v;
    return 1;
}

/* Same but into a size_t (for Content-Length). */
static int parse_size(const char *p, size_t len, size_t *out) {
    unsigned long v = 0;
    size_t i;
    if (len == 0) {
        return 0;
    }
    for (i = 0; i < len; i++) {
        unsigned d;
        if (p[i] < '0' || p[i] > '9') {
            return 0;
        }
        d = (unsigned)(p[i] - '0');
        if (v > (((size_t)-1) - d) / 10) {
            return 0;
        }
        v = v * 10 + d;
    }
    *out = v;
    return 1;
}

/* ------------------------------------------------------------------------- *
 * Stage 3 — body framing
 * ------------------------------------------------------------------------- */

static int has_chunked_te(const hspan *hs, size_t n) {
    size_t i;
    for (i = 0; i < n; i++) {
        if (ci_eq(hs[i].nptr, hs[i].nlen, "Transfer-Encoding")) {
            /* split the value on ',' and look for a trimmed "chunked" piece. */
            const char *v = hs[i].vptr;
            size_t vlen = hs[i].vlen;
            size_t start = 0, k = 0;
            for (;; k++) {
                if (k == vlen || v[k] == ',') {
                    const char *piece = v + start;
                    size_t plen = k - start;
                    trim_ws(&piece, &plen);
                    if (ci_eq(piece, plen, "chunked")) {
                        return 1;
                    }
                    if (k == vlen) {
                        break;
                    }
                    start = k + 1;
                }
            }
        }
    }
    return 0;
}

/* Content-Length: returns 1 with *out set / *present=1 when present & valid,
 * 1 with *present=0 when absent, 0 (invalid) when present but not an integer. */
static int declared_content_length(const hspan *hs, size_t n, int *present, size_t *out) {
    size_t i;
    for (i = 0; i < n; i++) {
        if (ci_eq(hs[i].nptr, hs[i].nlen, "Content-Length")) {
            *present = 1;
            return parse_size(hs[i].vptr, hs[i].vlen, out);
        }
    }
    *present = 0;
    return 1;
}

/* ------------------------------------------------------------------------- *
 * Materialisation — copy borrowed spans into owned storage
 * ------------------------------------------------------------------------- */

/* Copy the header spans into an owned HttpHeader array. NULL on OOM. */
static HttpHeader *own_headers(const hspan *hs, size_t n) {
    HttpHeader *arr;
    size_t i;
    if (n == 0) {
        return NULL;
    }
    arr = (HttpHeader *)calloc(n, sizeof(*arr));
    if (!arr) {
        return NULL;
    }
    for (i = 0; i < n; i++) {
        char *name = dupn(hs[i].nptr, hs[i].nlen);
        char *value = dupn(hs[i].vptr, hs[i].vlen);
        if (!name || !value) {
            size_t j;
            free(name);
            free(value);
            for (j = 0; j < i; j++) {
                free((char *)arr[j].name);
                free((char *)arr[j].value);
            }
            free(arr);
            return NULL;
        }
        arr[i].name = name;
        arr[i].value = value;
    }
    return arr;
}

static void free_owned_headers(HttpHeader *arr, size_t n) {
    size_t i;
    for (i = 0; i < n; i++) {
        free((char *)arr[i].name);
        free((char *)arr[i].value);
    }
    free(arr);
}

/* ------------------------------------------------------------------------- *
 * Public API — request
 * ------------------------------------------------------------------------- */

http1_status http1_parse_request_head(const unsigned char *input, size_t len,
                                      Http1ParsedRequestHead *out) {
    bspan *lines = NULL;
    size_t nlines = 0, body_offset = 0;
    bspan *toks = NULL;
    size_t ntoks = 0;
    hspan *hs = NULL;
    size_t nhs = 0;
    http1_status st;
    HttpVersion version;
    char *vtmp;
    int present = 0;
    size_t clen = 0;
    HttpBodyKind body_kind;
    size_t body_length = 0;

    if (!out || (!input && len > 0)) {
        return HTTP1_ERR_INVALID;
    }

    st = split_head_lines(input, len, &lines, &nlines, &body_offset);
    if (st != HTTP1_OK) {
        return st;
    }
    if (nlines == 0) {
        free(lines);
        return HTTP1_ERR_INVALID_START_LINE; /* split_first on empty */
    }
    if (!utf8_valid(lines[0].ptr, lines[0].len)) {
        free(lines);
        return HTTP1_ERR_INVALID_HEAD_ENCODING;
    }

    /* Start line: exactly method / target / version. */
    st = tokenize_ws(lines[0].ptr, lines[0].len, &toks, &ntoks);
    if (st != HTTP1_OK) {
        free(lines);
        return st;
    }
    if (ntoks != 3) {
        free(lines);
        free(toks);
        return HTTP1_ERR_INVALID_START_LINE;
    }
    vtmp = dupn(toks[2].ptr, toks[2].len);
    if (!vtmp) {
        free(lines);
        free(toks);
        return HTTP1_ERR_NOMEM;
    }
    if (http_version_parse(vtmp, &version) != 0) {
        free(vtmp);
        free(lines);
        free(toks);
        return HTTP1_ERR_INVALID_VERSION;
    }
    free(vtmp);

    /* Headers + body framing. */
    st = parse_headers(lines + 1, nlines - 1, &hs, &nhs);
    if (st != HTTP1_OK) {
        free(lines);
        free(toks);
        return st;
    }
    if (!declared_content_length(hs, nhs, &present, &clen)) {
        free(lines);
        free(toks);
        free(hs);
        return HTTP1_ERR_INVALID_CONTENT_LENGTH;
    }
    if (has_chunked_te(hs, nhs)) {
        body_kind = HTTP_BODY_CHUNKED;
    } else if (present && clen > 0) {
        body_kind = HTTP_BODY_CONTENT_LENGTH;
        body_length = clen;
    } else {
        body_kind = HTTP_BODY_NONE;
    }

    /* Materialise owned storage. */
    out->owned_method = dupn(toks[0].ptr, toks[0].len);
    out->owned_target = dupn(toks[1].ptr, toks[1].len);
    out->owned_headers = own_headers(hs, nhs);
    out->owned_nheaders = nhs;
    if (!out->owned_method || !out->owned_target || (nhs > 0 && !out->owned_headers)) {
        free(out->owned_method);
        free(out->owned_target);
        if (out->owned_headers) {
            free_owned_headers(out->owned_headers, nhs);
        }
        free(lines);
        free(toks);
        free(hs);
        return HTTP1_ERR_NOMEM;
    }

    out->head.method = out->owned_method;
    out->head.target = out->owned_target;
    out->head.version = version;
    out->head.headers = out->owned_headers;
    out->head.nheaders = nhs;
    out->body_offset = body_offset;
    out->body_kind = body_kind;
    out->body_length = body_length;

    free(lines);
    free(toks);
    free(hs);
    return HTTP1_OK;
}

/* ------------------------------------------------------------------------- *
 * Public API — response
 * ------------------------------------------------------------------------- */

/* Build the reason string by joining tokens[2..] with single spaces (owned). */
static char *join_reason(const bspan *toks, size_t ntoks) {
    size_t total = 0;
    size_t i;
    char *out;
    size_t o = 0;
    if (ntoks <= 2) {
        return dupn("", 0); /* empty reason */
    }
    for (i = 2; i < ntoks; i++) {
        total += toks[i].len;
        if (i > 2) {
            total += 1; /* separating space */
        }
    }
    out = (char *)malloc(total + 1);
    if (!out) {
        return NULL;
    }
    for (i = 2; i < ntoks; i++) {
        if (i > 2) {
            out[o++] = ' ';
        }
        if (toks[i].len > 0) {
            memcpy(out + o, toks[i].ptr, toks[i].len);
            o += toks[i].len;
        }
    }
    out[o] = '\0';
    return out;
}

http1_status http1_parse_response_head(const unsigned char *input, size_t len,
                                       Http1ParsedResponseHead *out) {
    bspan *lines = NULL;
    size_t nlines = 0, body_offset = 0;
    bspan *toks = NULL;
    size_t ntoks = 0;
    hspan *hs = NULL;
    size_t nhs = 0;
    http1_status st;
    HttpVersion version;
    char *vtmp;
    unsigned long status_ul = 0;
    unsigned short status;
    char *reason;
    int present = 0;
    size_t clen = 0;
    HttpBodyKind body_kind;
    size_t body_length = 0;

    if (!out || (!input && len > 0)) {
        return HTTP1_ERR_INVALID;
    }

    st = split_head_lines(input, len, &lines, &nlines, &body_offset);
    if (st != HTTP1_OK) {
        return st;
    }
    if (nlines == 0) {
        free(lines);
        return HTTP1_ERR_INVALID_START_LINE;
    }
    if (!utf8_valid(lines[0].ptr, lines[0].len)) {
        free(lines);
        return HTTP1_ERR_INVALID_HEAD_ENCODING;
    }

    /* Status line: version, status, then an optional reason (>= 2 tokens). */
    st = tokenize_ws(lines[0].ptr, lines[0].len, &toks, &ntoks);
    if (st != HTTP1_OK) {
        free(lines);
        return st;
    }
    if (ntoks < 2) {
        free(lines);
        free(toks);
        return HTTP1_ERR_INVALID_START_LINE;
    }
    vtmp = dupn(toks[0].ptr, toks[0].len);
    if (!vtmp) {
        free(lines);
        free(toks);
        return HTTP1_ERR_NOMEM;
    }
    if (http_version_parse(vtmp, &version) != 0) {
        free(vtmp);
        free(lines);
        free(toks);
        return HTTP1_ERR_INVALID_VERSION;
    }
    free(vtmp);
    if (!parse_uint(toks[1].ptr, toks[1].len, 65535UL, &status_ul)) {
        free(lines);
        free(toks);
        return HTTP1_ERR_INVALID_STATUS;
    }
    status = (unsigned short)status_ul;

    st = parse_headers(lines + 1, nlines - 1, &hs, &nhs);
    if (st != HTTP1_OK) {
        free(lines);
        free(toks);
        return st;
    }

    /* Body framing (status overrides come first). */
    if ((status >= 100 && status < 200) || status == 204 || status == 304) {
        body_kind = HTTP_BODY_NONE;
    } else if (has_chunked_te(hs, nhs)) {
        body_kind = HTTP_BODY_CHUNKED;
    } else {
        if (!declared_content_length(hs, nhs, &present, &clen)) {
            free(lines);
            free(toks);
            free(hs);
            return HTTP1_ERR_INVALID_CONTENT_LENGTH;
        }
        if (present) {
            if (clen == 0) {
                body_kind = HTTP_BODY_NONE;
            } else {
                body_kind = HTTP_BODY_CONTENT_LENGTH;
                body_length = clen;
            }
        } else {
            body_kind = HTTP_BODY_UNTIL_EOF;
        }
    }

    reason = join_reason(toks, ntoks);
    out->owned_reason = reason;
    out->owned_headers = own_headers(hs, nhs);
    out->owned_nheaders = nhs;
    if (!reason || (nhs > 0 && !out->owned_headers)) {
        free(reason);
        if (out->owned_headers) {
            free_owned_headers(out->owned_headers, nhs);
        }
        free(lines);
        free(toks);
        free(hs);
        return HTTP1_ERR_NOMEM;
    }

    out->head.version = version;
    out->head.status = status;
    out->head.reason = out->owned_reason;
    out->head.headers = out->owned_headers;
    out->head.nheaders = nhs;
    out->body_offset = body_offset;
    out->body_kind = body_kind;
    out->body_length = body_length;

    free(lines);
    free(toks);
    free(hs);
    return HTTP1_OK;
}

/* ------------------------------------------------------------------------- *
 * Free + summaries
 * ------------------------------------------------------------------------- */

void http1_parsed_request_free(Http1ParsedRequestHead *p) {
    if (!p) {
        return;
    }
    free(p->owned_method);
    free(p->owned_target);
    if (p->owned_headers) {
        free_owned_headers(p->owned_headers, p->owned_nheaders);
    }
    p->owned_method = NULL;
    p->owned_target = NULL;
    p->owned_headers = NULL;
    p->owned_nheaders = 0;
}

void http1_parsed_response_free(Http1ParsedResponseHead *p) {
    if (!p) {
        return;
    }
    free(p->owned_reason);
    if (p->owned_headers) {
        free_owned_headers(p->owned_headers, p->owned_nheaders);
    }
    p->owned_reason = NULL;
    p->owned_headers = NULL;
    p->owned_nheaders = 0;
}

/* strlen without <string.h>'s name clash risk — small and local. */
static size_t clen_of(const char *s) {
    size_t n = 0;
    while (s[n]) {
        n++;
    }
    return n;
}

Http1RequestHeadSummary http1_request_summary(const Http1ParsedRequestHead *p) {
    Http1RequestHeadSummary s;
    s.method = p->head.method;
    s.target_len = clen_of(p->head.target);
    s.version = p->head.version;
    s.header_count = p->head.nheaders;
    s.body_offset = p->body_offset;
    s.body_kind = p->body_kind;
    s.body_length = p->body_length;
    return s;
}

Http1ResponseHeadSummary http1_response_summary(const Http1ParsedResponseHead *p) {
    Http1ResponseHeadSummary s;
    s.version = p->head.version;
    s.status = p->head.status;
    s.reason_len = clen_of(p->head.reason);
    s.header_count = p->head.nheaders;
    s.body_offset = p->body_offset;
    s.body_kind = p->body_kind;
    s.body_length = p->body_length;
    return s;
}
