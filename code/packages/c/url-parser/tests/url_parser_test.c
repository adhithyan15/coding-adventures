/* Tests for the C url-parser, using the iso_test.h harness. Cases are taken
 * from the Rust crate's own tests. */
#include "iso_test.h"

#include <stdlib.h> /* free, NULL */
#include <string.h> /* strcmp */

#include "url_parser.h"

/* Assert an optional string field equals `expected` (NULL means "absent"). */
static void eq_opt(const char *actual, const char *expected) {
    if (expected == NULL) {
        ISO_CHECK(actual == NULL);
    } else {
        ISO_CHECK(actual != NULL);
        if (actual) {
            ISO_CHECK_STR_EQ(actual, expected);
        }
    }
}

int main(void) {
    /* Simple http URL. */
    {
        Url u;
        ISO_CHECK_EQ_INT((int)url_parse("http://www.example.com", &u),
                         (int)URL_OK);
        ISO_CHECK_STR_EQ(u.scheme, "http");
        eq_opt(u.host, "www.example.com");
        ISO_CHECK(!u.has_port);
        ISO_CHECK_STR_EQ(u.path, "/");
        eq_opt(u.query, NULL);
        eq_opt(u.fragment, NULL);
        url_free(&u);
    }

    /* All components. */
    {
        Url u;
        url_parse("http://alice:secret@www.example.com:8080/docs/page.html"
                  "?q=hello#section2",
                  &u);
        ISO_CHECK_STR_EQ(u.scheme, "http");
        eq_opt(u.userinfo, "alice:secret");
        eq_opt(u.host, "www.example.com");
        ISO_CHECK(u.has_port && u.port == 8080);
        ISO_CHECK_STR_EQ(u.path, "/docs/page.html");
        eq_opt(u.query, "q=hello");
        eq_opt(u.fragment, "section2");
        url_free(&u);
    }

    /* Scheme + host lowercased; path case preserved. */
    {
        Url u;
        url_parse("HTTP://WWW.EXAMPLE.COM/PATH", &u);
        ISO_CHECK_STR_EQ(u.scheme, "http");
        eq_opt(u.host, "www.example.com");
        ISO_CHECK_STR_EQ(u.path, "/PATH");
        url_free(&u);
    }

    /* mailto (scheme:path form, no authority). */
    {
        Url u;
        url_parse("mailto:alice@example.com", &u);
        ISO_CHECK_STR_EQ(u.scheme, "mailto");
        eq_opt(u.host, NULL);
        ISO_CHECK_STR_EQ(u.path, "alice@example.com");
        url_free(&u);
    }

    /* effective_port: defaults and explicit. */
    {
        Url u;
        unsigned short p = 0;
        url_parse("http://example.com", &u);
        ISO_CHECK(!u.has_port);
        ISO_CHECK(url_effective_port(&u, &p) && p == 80);
        url_free(&u);
        url_parse("https://secure.example.com/login", &u);
        ISO_CHECK(url_effective_port(&u, &p) && p == 443);
        url_free(&u);
        url_parse("ftp://files.example.com/x", &u);
        ISO_CHECK(url_effective_port(&u, &p) && p == 21);
        url_free(&u);
        url_parse("http://example.com:9090", &u);
        ISO_CHECK(u.has_port && u.port == 9090);
        ISO_CHECK(url_effective_port(&u, &p) && p == 9090);
        url_free(&u);
    }

    /* authority(). */
    {
        Url u;
        char *a;
        url_parse("http://user:pass@host.com:8080/path", &u);
        a = url_authority(&u);
        ISO_CHECK(a != NULL);
        ISO_CHECK_STR_EQ(a, "user:pass@host.com:8080");
        free(a);
        url_free(&u);
        url_parse("http://host.com/path", &u);
        a = url_authority(&u);
        ISO_CHECK_STR_EQ(a, "host.com");
        free(a);
        url_free(&u);
    }

    /* Errors. */
    {
        Url u;
        ISO_CHECK_EQ_INT((int)url_parse("www.example.com", &u),
                         (int)URL_ERR_MISSING_SCHEME);
        ISO_CHECK_EQ_INT((int)url_parse("1http://x.com", &u),
                         (int)URL_ERR_INVALID_SCHEME);
        ISO_CHECK_EQ_INT((int)url_parse("http://host:99999", &u),
                         (int)URL_ERR_INVALID_PORT);
    }

    /* Percent-encoding. */
    {
        char *e = url_percent_encode("hello world");
        ISO_CHECK_STR_EQ(e, "hello%20world");
        free(e);
        e = url_percent_encode("abc-def_ghi.jkl~mno");
        ISO_CHECK_STR_EQ(e, "abc-def_ghi.jkl~mno");
        free(e);
        e = url_percent_encode("/path/to/file");
        ISO_CHECK_STR_EQ(e, "/path/to/file");
        free(e);
    }

    /* Percent-decoding. */
    {
        char *d = NULL;
        ISO_CHECK_EQ_INT((int)url_percent_decode("hello%20world", &d),
                         (int)URL_OK);
        ISO_CHECK_STR_EQ(d, "hello world");
        free(d);
        ISO_CHECK_EQ_INT((int)url_percent_decode("%E6%97%A5", &d), (int)URL_OK);
        ISO_CHECK_STR_EQ(d, "\xe6\x97\xa5"); /* 日 */
        free(d);
        ISO_CHECK_EQ_INT((int)url_percent_decode("%2", &d),
                         (int)URL_ERR_INVALID_PERCENT_ENCODING);
        ISO_CHECK_EQ_INT((int)url_percent_decode("%GG", &d),
                         (int)URL_ERR_INVALID_PERCENT_ENCODING);
    }
    {
        /* round trip */
        char *e = url_percent_encode("path with spaces & special=chars!");
        char *d = NULL;
        url_percent_decode(e, &d);
        ISO_CHECK_STR_EQ(d, "path with spaces & special=chars!");
        free(e);
        free(d);
    }

    /* Resolve: relative references. */
    {
        Url base, r;
        url_parse("http://host/a/b/c.html", &base);

        url_resolve(&base, "d.html", &r);
        ISO_CHECK_STR_EQ(r.scheme, "http");
        eq_opt(r.host, "host");
        ISO_CHECK_STR_EQ(r.path, "/a/b/d.html");
        url_free(&r);

        url_resolve(&base, "../d.html", &r);
        ISO_CHECK_STR_EQ(r.path, "/a/d.html");
        url_free(&r);

        url_resolve(&base, "../../d.html", &r);
        ISO_CHECK_STR_EQ(r.path, "/d.html");
        url_free(&r);

        url_resolve(&base, "/x/y.html", &r);
        ISO_CHECK_STR_EQ(r.path, "/x/y.html");
        eq_opt(r.host, "host");
        url_free(&r);

        url_resolve(&base, "./d", &r);
        ISO_CHECK_STR_EQ(r.path, "/a/b/d");
        url_free(&r);

        url_free(&base);
    }

    /* Resolve: scheme-relative, already-absolute, fragment-only, empty. */
    {
        Url base, r;
        url_parse("http://host/a/b", &base);

        url_resolve(&base, "//other.com/path", &r);
        ISO_CHECK_STR_EQ(r.scheme, "http");
        eq_opt(r.host, "other.com");
        ISO_CHECK_STR_EQ(r.path, "/path");
        url_free(&r);

        url_resolve(&base, "https://other.com/x", &r);
        ISO_CHECK_STR_EQ(r.scheme, "https");
        eq_opt(r.host, "other.com");
        ISO_CHECK_STR_EQ(r.path, "/x");
        url_free(&r);

        url_resolve(&base, "#sec", &r);
        ISO_CHECK_STR_EQ(r.path, "/a/b");
        eq_opt(r.fragment, "sec");
        url_free(&r);

        url_free(&base);
    }
    {
        Url base, r;
        url_parse("http://host/a/b?q=1#frag", &base);
        url_resolve(&base, "", &r);
        ISO_CHECK_STR_EQ(r.path, "/a/b");
        eq_opt(r.query, "q=1");
        eq_opt(r.fragment, NULL); /* fragment stripped */
        url_free(&r);
        url_free(&base);
    }

    /* to_url_string round trip. */
    {
        Url u;
        char *s;
        url_parse("http://a:b@host.com:8080/p/q?x=1#f", &u);
        s = url_to_string(&u);
        ISO_CHECK(s != NULL);
        ISO_CHECK_STR_EQ(s, "http://a:b@host.com:8080/p/q?x=1#f");
        free(s);
        url_free(&u);
    }

    return ISO_TEST_RESULT();
}
