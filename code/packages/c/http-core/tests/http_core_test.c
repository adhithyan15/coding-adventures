/*
 * Tests for the C http-core helpers, using the header-only iso_test.h harness
 * (pure ISO). Vectors mirror the Rust crate's own unit tests — version parsing,
 * case-insensitive header lookup, Content-Length/Content-Type parsing, request
 * target splitting (query left undecoded), query pairs/values, and route-pattern
 * matching by path.
 */
#include "iso_test.h"

#include <stdlib.h>
#include <string.h>

#include "http_core.h"

/* Assert pair `i` in a batch equals (name, value). */
static void chk_pair(const HttpPairs *p, size_t i, const char *name,
                     const char *value) {
    ISO_CHECK(i < p->count);
    if (i < p->count) {
        ISO_CHECK_STR_EQ(p->items[i].name, name);
        ISO_CHECK_STR_EQ(p->items[i].value, value);
    }
}

int main(void) {
    /* ── HTTP version parse / display ───────────────────────────────────── */
    {
        HttpVersion v;
        ISO_CHECK(http_version_parse("HTTP/1.1", &v) == 0);
        ISO_CHECK_EQ_UINT(v.major, 1u);
        ISO_CHECK_EQ_UINT(v.minor, 1u);
        char buf[32];
        int n = http_version_to_string(v, buf, sizeof buf);
        ISO_CHECK_EQ_INT(n, 8);
        ISO_CHECK_STR_EQ(buf, "HTTP/1.1");
        ISO_CHECK(http_version_parse("1.1", &v) == -1);
        ISO_CHECK(http_version_parse("HTTP/x.1", &v) == -1);
        ISO_CHECK(http_version_parse("HTTP/1", &v) == -1);
    }

    /* ── case-insensitive header lookup ─────────────────────────────────── */
    {
        HttpHeader headers[] = {{"Content-Type", "text/plain"}};
        const char *v = http_find_header(headers, 1, "content-type");
        ISO_CHECK(v != NULL);
        if (v) ISO_CHECK_STR_EQ(v, "text/plain");
        ISO_CHECK(http_find_header(headers, 1, "missing") == NULL);
    }

    /* ── content-length / content-type helpers ──────────────────────────── */
    {
        HttpHeader headers[] = {{"Content-Length", "42"},
                                {"Content-Type", "text/html; charset=utf-8"}};
        size_t len = 0;
        ISO_CHECK(http_parse_content_length(headers, 2, &len) == 1);
        ISO_CHECK_EQ_UINT(len, 42u);

        char *media = NULL, *charset = NULL;
        ISO_CHECK(http_parse_content_type(headers, 2, &media, &charset) == 1);
        ISO_CHECK_STR_EQ(media, "text/html");
        ISO_CHECK(charset != NULL);
        if (charset) ISO_CHECK_STR_EQ(charset, "utf-8");
        free(media);
        free(charset);
    }
    {
        /* An oversized Content-Length must be rejected, not silently wrapped. */
        HttpHeader headers[] = {{"Content-Length", "18446744073709551616"}};
        size_t len = 0;
        ISO_CHECK(http_parse_content_length(headers, 1, &len) == 0);
    }
    {
        /* A media type with no charset parameter. */
        HttpHeader headers[] = {{"Content-Type", "application/json"}};
        char *media = NULL, *charset = NULL;
        ISO_CHECK(http_parse_content_type(headers, 1, &media, &charset) == 1);
        ISO_CHECK_STR_EQ(media, "application/json");
        ISO_CHECK(charset == NULL);
        free(media);
    }

    /* ── request-target parsing (query not decoded) ─────────────────────── */
    {
        HttpRequestTarget t;
        ISO_CHECK(http_parse_request_target(
                      "/clip/v2/resource/light?id=abc%20123&limit=10#ignored",
                      &t) == 0);
        ISO_CHECK_STR_EQ(t.path, "/clip/v2/resource/light");
        ISO_CHECK(t.query != NULL);
        if (t.query) ISO_CHECK_STR_EQ(t.query, "id=abc%20123&limit=10");
        ISO_CHECK(t.fragment != NULL);
        if (t.fragment) ISO_CHECK_STR_EQ(t.fragment, "ignored");

        HttpPairs pairs;
        ISO_CHECK(http_query_pairs(t.query, &pairs) == 0);
        ISO_CHECK_EQ_UINT(pairs.count, 2u);
        chk_pair(&pairs, 0, "id", "abc%20123");
        chk_pair(&pairs, 1, "limit", "10");
        http_pairs_free(&pairs);

        char *val = NULL;
        ISO_CHECK(http_query_value(t.query, "limit", &val) == 1);
        ISO_CHECK_STR_EQ(val, "10");
        free(val);
        ISO_CHECK(http_query_value(t.query, "missing", &val) == 0);
        http_request_target_free(&t);
    }

    /* ── request head path/query helpers ────────────────────────────────── */
    {
        HttpVersion v11 = {1, 1};
        HttpRequestHead req = {"GET", "/api/devices?room=kitchen&verbose", v11,
                               NULL, 0};
        char *path = NULL;
        ISO_CHECK(http_request_head_path(&req, &path) == 0);
        ISO_CHECK_STR_EQ(path, "/api/devices");
        free(path);

        char *room = NULL;
        ISO_CHECK(http_request_head_query_value(&req, "room", &room) == 1);
        ISO_CHECK_STR_EQ(room, "kitchen");
        free(room);

        char *verbose = NULL;
        ISO_CHECK(http_request_head_query_value(&req, "verbose", &verbose) == 1);
        ISO_CHECK_STR_EQ(verbose, ""); /* present, empty value */
        free(verbose);
    }

    /* ── route matching by path only (query ignored) ────────────────────── */
    {
        HttpRoutePattern *pat =
            http_route_parse("/clip/v2/resource/:kind/:id");
        ISO_CHECK(pat != NULL);
        HttpPairs m;
        ISO_CHECK(http_route_match_target(
                      pat, "/clip/v2/resource/light/abc?limit=10", &m) == 1);
        ISO_CHECK_EQ_UINT(m.count, 2u);
        chk_pair(&m, 0, "kind", "light");
        chk_pair(&m, 1, "id", "abc");
        http_pairs_free(&m);

        ISO_CHECK(http_route_match_target(pat, "/clip/v2/resource/light", &m) ==
                  0);
        http_route_free(pat);
    }

    /* ── heads delegate to the content helpers ──────────────────────────── */
    {
        HttpVersion v11 = {1, 1};
        HttpHeader rh[] = {{"Content-Length", "5"}};
        HttpRequestHead req = {"POST", "/submit", v11, rh, 1};
        size_t len = 0;
        ISO_CHECK(http_request_head_content_length(&req, &len) == 1);
        ISO_CHECK_EQ_UINT(len, 5u);

        HttpVersion v10 = {1, 0};
        HttpHeader sh[] = {{"Content-Type", "application/json"}};
        HttpResponseHead resp = {v10, 200, "OK", sh, 1};
        char *media = NULL, *charset = NULL;
        ISO_CHECK(http_response_head_content_type(&resp, &media, &charset) == 1);
        ISO_CHECK_STR_EQ(media, "application/json");
        ISO_CHECK(charset == NULL);
        free(media);
    }

    /* ── named-parameter matching ───────────────────────────────────────── */
    {
        HttpRoutePattern *pat = http_route_parse("/hello/:name");
        HttpPairs m;
        ISO_CHECK(http_route_match_path(pat, "/hello/Adhithya", &m) == 1);
        ISO_CHECK_EQ_UINT(m.count, 1u);
        chk_pair(&m, 0, "name", "Adhithya");
        http_pairs_free(&m);
        ISO_CHECK(http_route_match_path(pat, "/hello", &m) == 0);
        ISO_CHECK(http_route_match_path(pat, "/goodbye/Adhithya", &m) == 0);
        http_route_free(pat);
    }

    /* ── root-path handling ─────────────────────────────────────────────── */
    {
        HttpRoutePattern *pat = http_route_parse("/");
        HttpPairs m;
        ISO_CHECK(http_route_match_path(pat, "/", &m) == 1);
        ISO_CHECK_EQ_UINT(m.count, 0u); /* matches with no captures */
        http_pairs_free(&m);
        ISO_CHECK(http_route_match_path(pat, "/extra", &m) == 0);
        http_route_free(pat);
    }

    return ISO_TEST_RESULT();
}
