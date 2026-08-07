/*
 * http1_test.c — tests for the HTTP/1.1 head parser.
 * ===========================================================================
 *
 * Mirrors the Rust unit tests (simple request, Content-Length, response +
 * reason, until-EOF, body-less status override, LF-only + duplicate headers,
 * invalid header / Content-Length, and the redacted summaries) and adds a
 * chunked case plus the C ownership / NULL-argument paths. Runs under ASan+UBSan
 * so any leak or out-of-bounds parse of the (untrusted) input fails the build.
 * The head vocabulary comes from the composed pure-ISO http-core package.
 */
#include "http1/http1.h"
#include "http_core.h"
#include "iso_test.h"

#include <string.h>

/* Parse a request from a C-string literal (length = strlen). */
static http1_status req(const char *s, Http1ParsedRequestHead *out) {
    return http1_parse_request_head((const unsigned char *)s, strlen(s), out);
}
static http1_status resp(const char *s, Http1ParsedResponseHead *out) {
    return http1_parse_response_head((const unsigned char *)s, strlen(s), out);
}

/* Rust: parses_simple_request. */
static void test_simple_request(void) {
    Http1ParsedRequestHead p;
    ISO_CHECK_EQ_INT(req("GET / HTTP/1.0\r\nHost: example.com\r\n\r\n", &p), HTTP1_OK);
    ISO_CHECK_STR_EQ(p.head.method, "GET");
    ISO_CHECK_STR_EQ(p.head.target, "/");
    ISO_CHECK_EQ_INT(p.head.version.major, 1);
    ISO_CHECK_EQ_INT(p.head.version.minor, 0);
    ISO_CHECK_EQ_UINT(p.head.nheaders, 1);
    ISO_CHECK_STR_EQ(p.head.headers[0].name, "Host");
    ISO_CHECK_STR_EQ(p.head.headers[0].value, "example.com");
    ISO_CHECK_EQ_INT(p.body_kind, HTTP_BODY_NONE);
    http1_parsed_request_free(&p);
}

/* Rust: parses_content_length_request. */
static void test_content_length_request(void) {
    Http1ParsedRequestHead p;
    ISO_CHECK_EQ_INT(req("POST /submit HTTP/1.1\r\nContent-Length: 5\r\n\r\nhello", &p), HTTP1_OK);
    ISO_CHECK_EQ_UINT(p.body_offset, 44);
    ISO_CHECK_EQ_INT(p.body_kind, HTTP_BODY_CONTENT_LENGTH);
    ISO_CHECK_EQ_UINT(p.body_length, 5);
    http1_parsed_request_free(&p);
}

/* Rust: parses_response_and_reason. */
static void test_response_reason(void) {
    Http1ParsedResponseHead p;
    ISO_CHECK_EQ_INT(resp("HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\nbody", &p), HTTP1_OK);
    ISO_CHECK_EQ_INT(p.head.status, 200);
    ISO_CHECK_STR_EQ(p.head.reason, "OK");
    ISO_CHECK_EQ_INT(p.body_kind, HTTP_BODY_CONTENT_LENGTH);
    ISO_CHECK_EQ_UINT(p.body_length, 4);
    http1_parsed_response_free(&p);
}

/* Rust: response_without_length_reads_until_eof. */
static void test_until_eof(void) {
    Http1ParsedResponseHead p;
    ISO_CHECK_EQ_INT(resp("HTTP/1.0 200 OK\r\nServer: Venture\r\n\r\n", &p), HTTP1_OK);
    ISO_CHECK_EQ_INT(p.body_kind, HTTP_BODY_UNTIL_EOF);
    http1_parsed_response_free(&p);
}

/* Rust: bodyless_status_codes_override_headers. */
static void test_bodyless_status(void) {
    Http1ParsedResponseHead p;
    ISO_CHECK_EQ_INT(resp("HTTP/1.1 204 No Content\r\nContent-Length: 12\r\n\r\n", &p), HTTP1_OK);
    ISO_CHECK_EQ_INT(p.body_kind, HTTP_BODY_NONE); /* 204 overrides the length */
    http1_parsed_response_free(&p);
}

/* Rust: accepts_lf_only_lines_and_duplicate_headers. */
static void test_lf_only_and_duplicates(void) {
    Http1ParsedResponseHead p;
    ISO_CHECK_EQ_INT(
        resp("\nHTTP/1.1 200 OK\nSet-Cookie: a=1\nSet-Cookie: b=2\n\npayload", &p), HTTP1_OK);
    ISO_CHECK_EQ_UINT(p.head.nheaders, 2);
    ISO_CHECK_STR_EQ(p.head.headers[0].value, "a=1");
    ISO_CHECK_STR_EQ(p.head.headers[1].value, "b=2");
    http1_parsed_response_free(&p);
}

/* Rust: rejects_invalid_headers. */
static void test_invalid_header(void) {
    Http1ParsedRequestHead p;
    ISO_CHECK_EQ_INT(req("GET / HTTP/1.1\r\nHost example.com\r\n\r\n", &p), HTTP1_ERR_INVALID_HEADER);
}

/* Rust: rejects_invalid_content_length. */
static void test_invalid_content_length(void) {
    Http1ParsedResponseHead p;
    ISO_CHECK_EQ_INT(resp("HTTP/1.1 200 OK\r\nContent-Length: nope\r\n\r\n", &p),
                     HTTP1_ERR_INVALID_CONTENT_LENGTH);
}

/* Rust: request_summary_omits_target_text_and_headers. */
static void test_request_summary(void) {
    Http1ParsedRequestHead p;
    Http1RequestHeadSummary s;
    ISO_CHECK_EQ_INT(
        req("POST /pair?token=secret-token HTTP/1.1\r\nHost: bridge.local\r\nContent-Length: 4\r\n\r\nbody",
            &p),
        HTTP1_OK);
    s = http1_request_summary(&p);
    ISO_CHECK_STR_EQ(s.method, "POST");
    ISO_CHECK_EQ_UINT(s.target_len, strlen("/pair?token=secret-token"));
    ISO_CHECK_EQ_INT(s.version.major, 1);
    ISO_CHECK_EQ_INT(s.version.minor, 1);
    ISO_CHECK_EQ_UINT(s.header_count, 2);
    ISO_CHECK_EQ_INT(s.body_kind, HTTP_BODY_CONTENT_LENGTH);
    ISO_CHECK_EQ_UINT(s.body_length, 4);
    http1_parsed_request_free(&p);
}

/* Rust: response_summary_omits_reason_headers_and_body. */
static void test_response_summary(void) {
    Http1ParsedResponseHead p;
    Http1ResponseHeadSummary s;
    ISO_CHECK_EQ_INT(
        resp("HTTP/1.1 503 Secret Backend Down\r\nRetry-After: 10\r\nContent-Length: 11\r\n\r\nhidden-body",
             &p),
        HTTP1_OK);
    /* The multi-word reason is joined with single spaces. */
    ISO_CHECK_STR_EQ(p.head.reason, "Secret Backend Down");
    s = http1_response_summary(&p);
    ISO_CHECK_EQ_INT(s.version.minor, 1);
    ISO_CHECK_EQ_INT(s.status, 503);
    ISO_CHECK_EQ_UINT(s.reason_len, strlen("Secret Backend Down"));
    ISO_CHECK_EQ_UINT(s.header_count, 2);
    ISO_CHECK_EQ_INT(s.body_kind, HTTP_BODY_CONTENT_LENGTH);
    ISO_CHECK_EQ_UINT(s.body_length, 11);
    http1_parsed_response_free(&p);
}

/* Transfer-Encoding: chunked → chunked framing (and it beats Content-Length). */
static void test_chunked(void) {
    Http1ParsedRequestHead p;
    ISO_CHECK_EQ_INT(
        req("POST /x HTTP/1.1\r\nTransfer-Encoding: gzip, chunked\r\nContent-Length: 5\r\n\r\n", &p),
        HTTP1_OK);
    ISO_CHECK_EQ_INT(p.body_kind, HTTP_BODY_CHUNKED);
    http1_parsed_request_free(&p);
}

/* Incomplete head (no blank line) is reported, not crashed. */
static void test_incomplete(void) {
    Http1ParsedRequestHead p;
    ISO_CHECK_EQ_INT(req("GET / HTTP/1.1\r\nHost: x\r\n", &p), HTTP1_ERR_INCOMPLETE_HEAD);
    ISO_CHECK_EQ_INT(req("GET / HTTP/1.1", &p), HTTP1_ERR_INCOMPLETE_HEAD);
}

/* Bad version and bad start line. */
static void test_bad_start_line(void) {
    Http1ParsedRequestHead p;
    ISO_CHECK_EQ_INT(req("GET / NOTHTTP\r\n\r\n", &p), HTTP1_ERR_INVALID_VERSION);
    ISO_CHECK_EQ_INT(req("GET /\r\n\r\n", &p), HTTP1_ERR_INVALID_START_LINE); /* 2 tokens */
    ISO_CHECK_EQ_INT(req("GET / HTTP/1.1 extra\r\n\r\n", &p), HTTP1_ERR_INVALID_START_LINE);
    {
        Http1ParsedResponseHead r;
        ISO_CHECK_EQ_INT(resp("HTTP/1.1\r\n\r\n", &r), HTTP1_ERR_INVALID_START_LINE); /* 1 token */
        ISO_CHECK_EQ_INT(resp("HTTP/1.1 999999 X\r\n\r\n", &r), HTTP1_ERR_INVALID_STATUS);
    }
}

static void test_invalid_params(void) {
    Http1ParsedRequestHead p;
    Http1ParsedResponseHead r;
    ISO_CHECK_EQ_INT(http1_parse_request_head((const unsigned char *)"x", 1, NULL),
                     HTTP1_ERR_INVALID);
    ISO_CHECK_EQ_INT(http1_parse_response_head((const unsigned char *)"x", 1, NULL),
                     HTTP1_ERR_INVALID);
    ISO_CHECK_EQ_INT(http1_parse_request_head(NULL, 5, &p), HTTP1_ERR_INVALID);
    ISO_CHECK_EQ_INT(http1_parse_response_head(NULL, 5, &r), HTTP1_ERR_INVALID);
    /* NULL input with len 0 → an empty head is incomplete, not a crash. */
    ISO_CHECK_EQ_INT(http1_parse_request_head(NULL, 0, &p), HTTP1_ERR_INCOMPLETE_HEAD);
    /* free is safe on a zeroed struct. */
    {
        Http1ParsedRequestHead z;
        Http1ParsedResponseHead zr;
        memset(&z, 0, sizeof(z));
        memset(&zr, 0, sizeof(zr));
        http1_parsed_request_free(&z);
        http1_parsed_response_free(&zr);
        http1_parsed_request_free(NULL);
        http1_parsed_response_free(NULL);
    }
    ISO_CHECK(1);
}

int main(void) {
    test_simple_request();
    test_content_length_request();
    test_response_reason();
    test_until_eof();
    test_bodyless_status();
    test_lf_only_and_duplicates();
    test_invalid_header();
    test_invalid_content_length();
    test_request_summary();
    test_response_summary();
    test_chunked();
    test_incomplete();
    test_bad_start_line();
    test_invalid_params();
    return ISO_TEST_RESULT();
}
