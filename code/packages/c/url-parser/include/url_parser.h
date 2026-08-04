/*
 * url_parser.h — a URL parser with relative-reference resolution and
 * percent-encoding, in pure ISO C17. A faithful port of the Rust `url-parser`
 * crate.
 * ===========================================================================
 *
 * Splits an absolute URL into its components and puts them back together:
 *
 *   http://alice:secret@www.example.com:8080/docs/page.html?q=hello#sec2
 *   \__/   \___________/ \_____________/ \__/ \____________/ \_____/ \__/
 *  scheme    userinfo         host       port      path      query  fragment
 *
 * Also implements RFC 1808 relative resolution (`url_resolve`) — merging a
 * relative reference against a base, including `.`/`..` removal — and
 * percent-encoding / decoding.
 *
 * Invariants (as in the crate): the scheme and host are lower-cased; the path
 * starts with '/' for authority-based URLs; query/fragment exclude their leading
 * '?'/'#'. IPv6 hosts keep their brackets ("[::1]").
 *
 * Portability: pure ISO C17 — GCC, Clang, and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors.
 */
#ifndef URL_PARSER_H
#define URL_PARSER_H

#include <stddef.h> /* size_t */

typedef enum {
    URL_OK = 0,
    URL_ERR_MISSING_SCHEME,           /* no scheme (no "://" or "scheme:") */
    URL_ERR_INVALID_SCHEME,           /* scheme is not [a-z][a-z0-9+.-]* */
    URL_ERR_INVALID_PORT,             /* port is not a valid 0..65535 */
    URL_ERR_INVALID_PERCENT_ENCODING, /* malformed %XX, or non-UTF-8 result */
    URL_ERR_EMPTY_HOST,
    URL_ERR_RELATIVE_WITHOUT_BASE,
    URL_ERR_ALLOC
} UrlStatus;

/* A parsed URL. Optional components are NULL (or has_port == 0) when absent.
 * All strings are owned; release with url_free. */
typedef struct {
    char *scheme;         /* lower-cased, always present after a successful parse */
    char *userinfo;       /* NULL if absent */
    char *host;           /* lower-cased; NULL if absent */
    int has_port;         /* 1 if an explicit port is present */
    unsigned short port;  /* meaningful only when has_port */
    char *path;
    char *query;          /* without the leading '?'; NULL if absent */
    char *fragment;       /* without the leading '#'; NULL if absent */
} Url;

/* ---- parse / resolve -------------------------------------------------- */

/* url_parse — parse an absolute URL into *out (release with url_free). On error
 * *out is zeroed. */
UrlStatus url_parse(const char *input, Url *out);

/* url_resolve — resolve `relative` against `base` (RFC 1808) into *out. */
UrlStatus url_resolve(const Url *base, const char *relative, Url *out);

/* url_free — free all components of *url. Safe on a zeroed Url. */
void url_free(Url *url);

/* ---- accessors (malloc'd strings; caller frees) ----------------------- */

/* url_effective_port — the explicit port, or the scheme default
 * (http=80, https=443, ftp=21). Returns 1 and sets *port if one applies,
 * else 0. */
int url_effective_port(const Url *url, unsigned short *port);

/* url_authority — "[userinfo@]host[:port]" (empty string if no host/userinfo).
 * NULL on allocation failure. */
char *url_authority(const Url *url);

/* url_to_string — serialize the URL back to a string. NULL on allocation
 * failure. */
char *url_to_string(const Url *url);

/* ---- percent-encoding ------------------------------------------------- */

/* url_percent_encode — encode all but the unreserved set (A-Za-z0-9-_.~/) as
 * %XX. Returns a malloc'd string (caller frees), or NULL on allocation
 * failure. */
char *url_percent_encode(const char *input);

/* url_percent_decode — decode %XX sequences, validating that the result is
 * UTF-8. On URL_OK sets *out (malloc'd, caller frees). */
UrlStatus url_percent_decode(const char *input, char **out);

#endif /* URL_PARSER_H */
