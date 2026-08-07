/*
 * url_parser.c — implementation of the URL parser (see url_parser.h). A
 * faithful port of the Rust `url-parser` crate: the same single-pass component
 * split, RFC 1808 relative resolution, and percent coding.
 */
#include "url_parser.h"

#include <stdint.h> /* SIZE_MAX */
#include <stdio.h>  /* snprintf */
#include <stdlib.h> /* malloc, realloc, free */
#include <string.h> /* memcpy, memchr, strlen, strcmp */

/* ===================================================================== *
 *  Slice helpers (operate on (ptr, len), no allocation)
 * ===================================================================== */

static int is_space(unsigned char c) {
    return c == ' ' || c == '\t' || c == '\n' || c == '\r' || c == '\f' ||
           c == '\v';
}
static int is_lower(unsigned char c) { return c >= 'a' && c <= 'z'; }
static int is_digit(unsigned char c) { return c >= '0' && c <= '9'; }
static int is_alpha(unsigned char c) {
    return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z');
}
static int is_alnum(unsigned char c) { return is_alpha(c) || is_digit(c); }

/* Trim ASCII whitespace: set *out_s / *out_n to the trimmed span of `input`. */
static void trim_span(const char *input, const char **out_s, size_t *out_n) {
    size_t len = strlen(input), start = 0, end = len;
    while (start < end && is_space((unsigned char)input[start])) {
        start++;
    }
    while (end > start && is_space((unsigned char)input[end - 1])) {
        end--;
    }
    *out_s = input + start;
    *out_n = end - start;
}

static int find_char(const char *s, size_t n, char c, size_t *pos) {
    const char *p = (const char *)memchr(s, c, n);
    if (!p) {
        return 0;
    }
    *pos = (size_t)(p - s);
    return 1;
}
static int rfind_char(const char *s, size_t n, char c, size_t *pos) {
    size_t i;
    for (i = n; i > 0; i--) {
        if (s[i - 1] == c) {
            *pos = i - 1;
            return 1;
        }
    }
    return 0;
}
static int find_str3(const char *s, size_t n, char a, char b, char c,
                     size_t *pos) {
    size_t i;
    if (n < 3) {
        return 0;
    }
    for (i = 0; i + 2 < n; i++) {
        if (s[i] == a && s[i + 1] == b && s[i + 2] == c) {
            *pos = i;
            return 1;
        }
    }
    return 0;
}
static int slice_has_char(const char *s, size_t n, char c) {
    return memchr(s, c, n) != NULL;
}
static int all_digits(const char *s, size_t n) {
    size_t i;
    if (n == 0) {
        return 0;
    }
    for (i = 0; i < n; i++) {
        if (!is_digit((unsigned char)s[i])) {
            return 0;
        }
    }
    return 1;
}

/* ---- owned-string builders -------------------------------------------- */

static char *dupn(const char *s, size_t n) {
    char *p = malloc(n + 1); /* n came from a strlen-bounded span, so n < MAX */
    if (!p) {
        return NULL;
    }
    if (n) {
        memcpy(p, s, n);
    }
    p[n] = '\0';
    return p;
}
static char *dupz(const char *s) { return dupn(s, strlen(s)); }
static char *lower_dupn(const char *s, size_t n) {
    char *p = malloc(n + 1);
    size_t i;
    if (!p) {
        return NULL;
    }
    for (i = 0; i < n; i++) {
        unsigned char c = (unsigned char)s[i];
        p[i] = (c >= 'A' && c <= 'Z') ? (char)(c - 'A' + 'a') : (char)c;
    }
    p[n] = '\0';
    return p;
}

/* Growable byte/char buffer for assembling strings. */
typedef struct {
    char *data;
    size_t len, cap;
    int ok;
} Buf;

static void buf_init(Buf *b) {
    b->data = NULL;
    b->len = 0;
    b->cap = 0;
    b->ok = 1;
}
static int buf_reserve(Buf *b, size_t extra) {
    size_t need, nc;
    if (!b->ok) {
        return 0;
    }
    if (extra > SIZE_MAX - b->len) {
        b->ok = 0;
        return 0;
    }
    need = b->len + extra;
    if (need <= b->cap) {
        return 1;
    }
    nc = b->cap ? b->cap : 16;
    while (nc < need) {
        if (nc > SIZE_MAX / 2) {
            nc = need;
            break;
        }
        nc *= 2;
    }
    {
        char *nd = realloc(b->data, nc);
        if (!nd) {
            b->ok = 0;
            return 0;
        }
        b->data = nd;
        b->cap = nc;
    }
    return 1;
}
static void buf_push(Buf *b, char c) {
    if (buf_reserve(b, 1)) {
        b->data[b->len++] = c;
    }
}
static void buf_append(Buf *b, const char *s, size_t n) {
    if (n && buf_reserve(b, n)) {
        memcpy(b->data + b->len, s, n);
        b->len += n;
    }
}
static void buf_append_z(Buf *b, const char *s) {
    if (s) {
        buf_append(b, s, strlen(s));
    }
}
/* Finish the buffer into a malloc'd NUL-terminated string (NULL on failure). */
static char *buf_finish(Buf *b) {
    if (!b->ok || !buf_reserve(b, 1)) {
        free(b->data);
        return NULL;
    }
    b->data[b->len] = '\0';
    return b->data; /* ownership transferred */
}

/* ===================================================================== *
 *  Small validators / converters
 * ===================================================================== */

/* Valid scheme is [a-z][a-z0-9+.-]* (the input is already lower-cased). */
static int scheme_valid(const char *s) {
    size_t i, n = strlen(s);
    if (n == 0 || !is_lower((unsigned char)s[0])) {
        return 0;
    }
    for (i = 1; i < n; i++) {
        unsigned char c = (unsigned char)s[i];
        if (!is_lower(c) && !is_digit(c) && c != '+' && c != '-' && c != '.') {
            return 0;
        }
    }
    return 1;
}

/* Does the (s,n) span look like a scheme: first alpha, rest [alnum+.-]? */
static int scheme_like(const char *s, size_t n) {
    size_t i;
    if (n == 0 || !is_alpha((unsigned char)s[0])) {
        return 0;
    }
    for (i = 0; i < n; i++) {
        unsigned char c = (unsigned char)s[i];
        if (!is_alnum(c) && c != '+' && c != '-' && c != '.') {
            return 0;
        }
    }
    return 1;
}

static UrlStatus parse_port(const char *s, size_t n, unsigned short *out) {
    unsigned long acc = 0;
    size_t i;
    if (n == 0) {
        return URL_ERR_INVALID_PORT;
    }
    for (i = 0; i < n; i++) {
        if (!is_digit((unsigned char)s[i])) {
            return URL_ERR_INVALID_PORT;
        }
        acc = acc * 10 + (unsigned long)(s[i] - '0');
        if (acc > 65535ul) {
            return URL_ERR_INVALID_PORT;
        }
    }
    *out = (unsigned short)acc;
    return URL_OK;
}

static int default_port(const char *scheme, unsigned short *out) {
    if (strcmp(scheme, "http") == 0) {
        *out = 80;
        return 1;
    }
    if (strcmp(scheme, "https") == 0) {
        *out = 443;
        return 1;
    }
    if (strcmp(scheme, "ftp") == 0) {
        *out = 21;
        return 1;
    }
    return 0;
}

/* A validating UTF-8 scan (percent_decode must yield valid UTF-8). */
static int utf8_valid(const unsigned char *s, size_t n) {
    size_t i = 0;
    while (i < n) {
        unsigned char c = s[i];
        size_t extra, k;
        unsigned long min_cp, cp;
        if (c < 0x80) {
            i++;
            continue;
        } else if ((c & 0xE0) == 0xC0) {
            extra = 1;
            min_cp = 0x80;
            cp = c & 0x1Fu;
        } else if ((c & 0xF0) == 0xE0) {
            extra = 2;
            min_cp = 0x800;
            cp = c & 0x0Fu;
        } else if ((c & 0xF8) == 0xF0) {
            extra = 3;
            min_cp = 0x10000;
            cp = c & 0x07u;
        } else {
            return 0;
        }
        if (extra >= n - i) {
            return 0;
        }
        for (k = 1; k <= extra; k++) {
            unsigned char cc = s[i + k];
            if ((cc & 0xC0) != 0x80) {
                return 0;
            }
            cp = (cp << 6) | (cc & 0x3Fu);
        }
        if (cp < min_cp || cp > 0x10FFFFuL || (cp >= 0xD800uL && cp <= 0xDFFFuL)) {
            return 0;
        }
        i += extra + 1;
    }
    return 1;
}

/* ===================================================================== *
 *  Parse
 * ===================================================================== */

void url_free(Url *url) {
    if (!url) {
        return;
    }
    free(url->scheme);
    free(url->userinfo);
    free(url->host);
    free(url->path);
    free(url->query);
    free(url->fragment);
    url->scheme = url->userinfo = url->host = NULL;
    url->path = url->query = url->fragment = NULL;
    url->has_port = 0;
    url->port = 0;
}

UrlStatus url_parse(const char *input, Url *out) {
    const char *s;
    size_t n;
    char *scheme = NULL, *userinfo = NULL, *host = NULL, *path = NULL,
         *query = NULL, *fragment = NULL;
    int has_port = 0;
    unsigned short port = 0;
    UrlStatus st = URL_OK;
    size_t pos;

    memset(out, 0, sizeof *out);
    trim_span(input, &s, &n);

    if (find_str3(s, n, ':', '/', '/', &pos)) {
        /* Authority-based URL: scheme "://" ... */
        scheme = lower_dupn(s, pos);
        if (!scheme) {
            st = URL_ERR_ALLOC;
            goto fail;
        }
        if (!scheme_valid(scheme)) {
            st = URL_ERR_INVALID_SCHEME;
            goto fail;
        }
        s += pos + 3;
        n -= pos + 3;
    } else if (find_char(s, n, ':', &pos) && pos > 0 &&
               !slice_has_char(s, pos, '/')) {
        /* "scheme:path" form (e.g. mailto:) — no authority. */
        const char *p = s + pos + 1;
        size_t pn = n - pos - 1;
        size_t cut;
        scheme = lower_dupn(s, pos);
        if (!scheme) {
            st = URL_ERR_ALLOC;
            goto fail;
        }
        if (!scheme_valid(scheme)) {
            st = URL_ERR_INVALID_SCHEME;
            goto fail;
        }
        if (find_char(p, pn, '#', &cut)) {
            fragment = dupn(p + cut + 1, pn - cut - 1);
            if (!fragment) {
                st = URL_ERR_ALLOC;
                goto fail;
            }
            pn = cut;
        }
        if (find_char(p, pn, '?', &cut)) {
            query = dupn(p + cut + 1, pn - cut - 1);
            if (!query) {
                st = URL_ERR_ALLOC;
                goto fail;
            }
            pn = cut;
        }
        path = dupn(p, pn);
        if (!path) {
            st = URL_ERR_ALLOC;
            goto fail;
        }
        out->scheme = scheme;
        out->path = path;
        out->query = query;
        out->fragment = fragment;
        return URL_OK;
    } else {
        st = URL_ERR_MISSING_SCHEME;
        goto fail;
    }

    /* Fragment, then query. */
    if (find_char(s, n, '#', &pos)) {
        fragment = dupn(s + pos + 1, n - pos - 1);
        if (!fragment) {
            st = URL_ERR_ALLOC;
            goto fail;
        }
        n = pos;
    }
    if (find_char(s, n, '?', &pos)) {
        query = dupn(s + pos + 1, n - pos - 1);
        if (!query) {
            st = URL_ERR_ALLOC;
            goto fail;
        }
        n = pos;
    }

    /* Authority vs path. */
    {
        const char *auth;
        size_t authn;
        const char *hostport;
        size_t hostportn;
        const char *hp;
        size_t hn;
        if (find_char(s, n, '/', &pos)) {
            auth = s;
            authn = pos;
            path = dupn(s + pos, n - pos);
        } else {
            auth = s;
            authn = n;
            path = dupn("/", 1);
        }
        if (!path) {
            st = URL_ERR_ALLOC;
            goto fail;
        }

        /* userinfo. */
        if (rfind_char(auth, authn, '@', &pos)) {
            userinfo = dupn(auth, pos);
            if (!userinfo) {
                st = URL_ERR_ALLOC;
                goto fail;
            }
            hostport = auth + pos + 1;
            hostportn = authn - pos - 1;
        } else {
            hostport = auth;
            hostportn = authn;
        }

        /* host + port. */
        hp = hostport;
        hn = hostportn;
        if (hostportn > 0 && hostport[0] == '[') {
            size_t bracket;
            if (find_char(hostport, hostportn, ']', &bracket)) {
                const char *after = hostport + bracket + 1;
                size_t aftern = hostportn - bracket - 1;
                hp = hostport;
                hn = bracket + 1;
                if (aftern > 0 && after[0] == ':') {
                    st = parse_port(after + 1, aftern - 1, &port);
                    if (st != URL_OK) {
                        goto fail;
                    }
                    has_port = 1;
                }
            }
        } else {
            size_t colon;
            if (rfind_char(hostport, hostportn, ':', &colon)) {
                const char *mp = hostport + colon + 1;
                size_t mpn = hostportn - colon - 1;
                if (all_digits(mp, mpn)) {
                    hp = hostport;
                    hn = colon;
                    st = parse_port(mp, mpn, &port);
                    if (st != URL_OK) {
                        goto fail;
                    }
                    has_port = 1;
                }
            }
        }
        if (hn > 0) {
            host = lower_dupn(hp, hn);
            if (!host) {
                st = URL_ERR_ALLOC;
                goto fail;
            }
        }
    }

    out->scheme = scheme;
    out->userinfo = userinfo;
    out->host = host;
    out->has_port = has_port;
    out->port = port;
    out->path = path;
    out->query = query;
    out->fragment = fragment;
    return URL_OK;

fail:
    free(scheme);
    free(userinfo);
    free(host);
    free(path);
    free(query);
    free(fragment);
    memset(out, 0, sizeof *out);
    return st;
}

/* ===================================================================== *
 *  Accessors
 * ===================================================================== */

int url_effective_port(const Url *url, unsigned short *port) {
    if (url->has_port) {
        *port = url->port;
        return 1;
    }
    return default_port(url->scheme, port);
}

char *url_authority(const Url *url) {
    Buf b;
    buf_init(&b);
    if (url->userinfo) {
        buf_append_z(&b, url->userinfo);
        buf_push(&b, '@');
    }
    if (url->host) {
        buf_append_z(&b, url->host);
    }
    if (url->has_port) {
        char tmp[8];
        buf_push(&b, ':');
        snprintf(tmp, sizeof tmp, "%u", (unsigned)url->port);
        buf_append_z(&b, tmp);
    }
    return buf_finish(&b);
}

char *url_to_string(const Url *url) {
    Buf b;
    char *auth = NULL;
    buf_init(&b);
    buf_append_z(&b, url->scheme);
    if (url->host) {
        auth = url_authority(url);
        if (!auth) {
            free(b.data);
            return NULL;
        }
        buf_append_z(&b, "://");
        buf_append_z(&b, auth);
        free(auth);
    } else {
        buf_push(&b, ':');
    }
    buf_append_z(&b, url->path);
    if (url->query) {
        buf_push(&b, '?');
        buf_append_z(&b, url->query);
    }
    if (url->fragment) {
        buf_push(&b, '#');
        buf_append_z(&b, url->fragment);
    }
    return buf_finish(&b);
}

/* ===================================================================== *
 *  Percent coding
 * ===================================================================== */

static int is_unreserved(unsigned char c) {
    return is_alnum(c) || c == '-' || c == '_' || c == '.' || c == '~' ||
           c == '/';
}

char *url_percent_encode(const char *input) {
    static const char HEX[] = "0123456789ABCDEF";
    Buf b;
    size_t i, len = strlen(input);
    buf_init(&b);
    for (i = 0; i < len; i++) {
        unsigned char c = (unsigned char)input[i];
        if (is_unreserved(c)) {
            buf_push(&b, (char)c);
        } else {
            buf_push(&b, '%');
            buf_push(&b, HEX[c >> 4]);
            buf_push(&b, HEX[c & 0x0F]);
        }
    }
    return buf_finish(&b);
}

static int hex_digit(unsigned char c, int *out) {
    if (c >= '0' && c <= '9') {
        *out = c - '0';
    } else if (c >= 'a' && c <= 'f') {
        *out = c - 'a' + 10;
    } else if (c >= 'A' && c <= 'F') {
        *out = c - 'A' + 10;
    } else {
        return 0;
    }
    return 1;
}

UrlStatus url_percent_decode(const char *input, char **out) {
    Buf b;
    size_t i, len = strlen(input);
    char *result;
    *out = NULL;
    buf_init(&b);
    i = 0;
    while (i < len) {
        if (input[i] == '%') {
            int hi, lo;
            if (i + 2 >= len) {
                free(b.data);
                return URL_ERR_INVALID_PERCENT_ENCODING;
            }
            if (!hex_digit((unsigned char)input[i + 1], &hi) ||
                !hex_digit((unsigned char)input[i + 2], &lo)) {
                free(b.data);
                return URL_ERR_INVALID_PERCENT_ENCODING;
            }
            buf_push(&b, (char)((hi << 4) | lo));
            i += 3;
        } else {
            buf_push(&b, input[i]);
            i++;
        }
    }
    if (!b.ok) {
        free(b.data);
        return URL_ERR_ALLOC;
    }
    if (!utf8_valid((const unsigned char *)b.data, b.len)) {
        free(b.data);
        return URL_ERR_INVALID_PERCENT_ENCODING;
    }
    result = buf_finish(&b);
    if (!result) {
        return URL_ERR_ALLOC;
    }
    *out = result;
    return URL_OK;
}

/* ===================================================================== *
 *  Resolve
 * ===================================================================== */

/* remove_dot_segments (RFC 3986 §5.2.4). Returns a malloc'd string or NULL. */
static char *remove_dot_segments(const char *path) {
    size_t len = strlen(path), i, seg_start = 0, scount = 0, scap = 0, total = 0;
    struct Span {
        const char *p;
        size_t n;
    } *stack = NULL;
    int leading_slash = (len > 0 && path[0] == '/');
    char *result;
    Buf b;

    for (i = 0; i <= len; i++) {
        if (i == len || path[i] == '/') {
            const char *seg = path + seg_start;
            size_t segn = i - seg_start;
            if (segn == 1 && seg[0] == '.') {
                /* skip */
            } else if (segn == 2 && seg[0] == '.' && seg[1] == '.') {
                if (scount > 0) {
                    scount--;
                }
            } else {
                if (scount == scap) {
                    size_t ncap = scap ? scap * 2 : 8;
                    struct Span *ns;
                    if (scap > (SIZE_MAX / sizeof(struct Span)) / 2) {
                        free(stack);
                        return NULL;
                    }
                    ns = realloc(stack, ncap * sizeof *ns);
                    if (!ns) {
                        free(stack);
                        return NULL;
                    }
                    stack = ns;
                    scap = ncap;
                }
                stack[scount].p = seg;
                stack[scount].n = segn;
                scount++;
            }
            seg_start = i + 1;
        }
    }

    (void)total;
    buf_init(&b);
    for (i = 0; i < scount; i++) {
        if (i > 0) {
            buf_push(&b, '/');
        }
        buf_append(&b, stack[i].p, stack[i].n);
    }
    free(stack);
    result = buf_finish(&b);
    if (!result) {
        return NULL;
    }
    /* Restore a leading '/' if the input had one and the join dropped it. */
    if (leading_slash && result[0] != '/') {
        char *fixed = malloc(strlen(result) + 2);
        if (!fixed) {
            free(result);
            return NULL;
        }
        fixed[0] = '/';
        strcpy(fixed + 1, result);
        free(result);
        result = fixed;
    }
    return result;
}

/* merge_paths: base up to its last '/', then the relative path. */
static char *merge_paths(const char *base, const char *rel) {
    size_t blen = strlen(base), rlen = strlen(rel), pos;
    char *r;
    if (rfind_char(base, blen, '/', &pos)) {
        size_t prefix = pos + 1;
        r = malloc(prefix + rlen + 1);
        if (!r) {
            return NULL;
        }
        memcpy(r, base, prefix);
        memcpy(r + prefix, rel, rlen);
        r[prefix + rlen] = '\0';
    } else {
        r = malloc(1 + rlen + 1);
        if (!r) {
            return NULL;
        }
        r[0] = '/';
        memcpy(r + 1, rel, rlen);
        r[1 + rlen] = '\0';
    }
    return r;
}

/* Deep-copy a Url. */
static UrlStatus url_clone(const Url *src, Url *dst) {
    memset(dst, 0, sizeof *dst);
    dst->has_port = src->has_port;
    dst->port = src->port;
    if (src->scheme && !(dst->scheme = dupz(src->scheme))) {
        goto oom;
    }
    if (src->userinfo && !(dst->userinfo = dupz(src->userinfo))) {
        goto oom;
    }
    if (src->host && !(dst->host = dupz(src->host))) {
        goto oom;
    }
    if (src->path && !(dst->path = dupz(src->path))) {
        goto oom;
    }
    if (src->query && !(dst->query = dupz(src->query))) {
        goto oom;
    }
    if (src->fragment && !(dst->fragment = dupz(src->fragment))) {
        goto oom;
    }
    return URL_OK;
oom:
    url_free(dst);
    return URL_ERR_ALLOC;
}

/* Replace *field (owned) with a copy of (s,n); free the old value. */
static int set_field(char **field, const char *s, size_t n) {
    char *nv = dupn(s, n);
    if (!nv) {
        return 0;
    }
    free(*field);
    *field = nv;
    return 1;
}

UrlStatus url_resolve(const Url *base, const char *relative, Url *out) {
    const char *r;
    size_t rn;
    UrlStatus st;
    memset(out, 0, sizeof *out);
    trim_span(relative, &r, &rn);

    /* Empty → base without fragment. */
    if (rn == 0) {
        st = url_clone(base, out);
        if (st != URL_OK) {
            return st;
        }
        free(out->fragment);
        out->fragment = NULL;
        return URL_OK;
    }

    /* Fragment-only. */
    if (r[0] == '#') {
        st = url_clone(base, out);
        if (st != URL_OK) {
            return st;
        }
        if (!set_field(&out->fragment, r + 1, rn - 1)) {
            url_free(out);
            return URL_ERR_ALLOC;
        }
        return URL_OK;
    }

    /* Already absolute (has a scheme). */
    {
        size_t colon;
        size_t dummy;
        int has_cc_slash = find_str3(r, rn, ':', '/', '/', &dummy);
        int has_colon = find_char(r, rn, ':', &colon);
        if (has_cc_slash || (has_colon && !(rn > 0 && r[0] == '/'))) {
            if (has_colon && colon > 0 && scheme_like(r, colon)) {
                char *tmp = dupn(r, rn);
                if (!tmp) {
                    return URL_ERR_ALLOC;
                }
                st = url_parse(tmp, out);
                free(tmp);
                return st;
            }
        }
    }

    /* Scheme-relative "//host/path". */
    if (rn >= 2 && r[0] == '/' && r[1] == '/') {
        Buf b;
        char *full;
        buf_init(&b);
        buf_append_z(&b, base->scheme);
        buf_push(&b, ':');
        buf_append(&b, r, rn);
        full = buf_finish(&b);
        if (!full) {
            return URL_ERR_ALLOC;
        }
        st = url_parse(full, out);
        free(full);
        return st;
    }

    /* Absolute path "/path". */
    if (r[0] == '/') {
        const char *p = r;
        size_t pn = rn;
        char *frag = NULL, *qry = NULL, *newpath;
        size_t cut;
        if (find_char(p, pn, '#', &cut)) {
            frag = dupn(p + cut + 1, pn - cut - 1);
            if (!frag) {
                return URL_ERR_ALLOC;
            }
            pn = cut;
        }
        if (find_char(p, pn, '?', &cut)) {
            qry = dupn(p + cut + 1, pn - cut - 1);
            if (!qry) {
                free(frag);
                return URL_ERR_ALLOC;
            }
            pn = cut;
        }
        {
            char *seg = dupn(p, pn);
            if (!seg) {
                free(frag);
                free(qry);
                return URL_ERR_ALLOC;
            }
            newpath = remove_dot_segments(seg);
            free(seg);
        }
        if (!newpath) {
            free(frag);
            free(qry);
            return URL_ERR_ALLOC;
        }
        st = url_clone(base, out);
        if (st != URL_OK) {
            free(frag);
            free(qry);
            free(newpath);
            return st;
        }
        free(out->path);
        out->path = newpath;
        free(out->query);
        out->query = qry;
        free(out->fragment);
        out->fragment = frag;
        return URL_OK;
    }

    /* Relative path — merge with the base path. */
    {
        const char *p = r;
        size_t pn = rn;
        char *frag = NULL, *qry = NULL, *relpath, *merged, *newpath;
        size_t cut;
        if (find_char(p, pn, '#', &cut)) {
            frag = dupn(p + cut + 1, pn - cut - 1);
            if (!frag) {
                return URL_ERR_ALLOC;
            }
            pn = cut;
        }
        if (find_char(p, pn, '?', &cut)) {
            qry = dupn(p + cut + 1, pn - cut - 1);
            if (!qry) {
                free(frag);
                return URL_ERR_ALLOC;
            }
            pn = cut;
        }
        relpath = dupn(p, pn);
        if (!relpath) {
            free(frag);
            free(qry);
            return URL_ERR_ALLOC;
        }
        merged = merge_paths(base->path ? base->path : "", relpath);
        free(relpath);
        if (!merged) {
            free(frag);
            free(qry);
            return URL_ERR_ALLOC;
        }
        newpath = remove_dot_segments(merged);
        free(merged);
        if (!newpath) {
            free(frag);
            free(qry);
            return URL_ERR_ALLOC;
        }
        st = url_clone(base, out);
        if (st != URL_OK) {
            free(frag);
            free(qry);
            free(newpath);
            return st;
        }
        free(out->path);
        out->path = newpath;
        free(out->query);
        out->query = qry;
        free(out->fragment);
        out->fragment = frag;
        return URL_OK;
    }
}
