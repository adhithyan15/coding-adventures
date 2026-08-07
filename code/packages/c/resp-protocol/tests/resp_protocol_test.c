/* Tests for the C resp-protocol, using the iso_test.h harness. The encode and
 * decode vectors are taken from the Rust crate's own tests. */
#include "iso_test.h"

#include <stdlib.h> /* malloc, free */
#include <string.h> /* strlen, strcmp */

#include "resp_protocol.h"

/* ---- small builders --------------------------------------------------- */

static RespValue *bulk(const char *s) {
    return resp_bulk_string((const unsigned char *)s, strlen(s));
}
static RespValue *arr1(RespValue *a) {
    RespValue **it = malloc(sizeof *it);
    it[0] = a;
    return resp_array(it, 1);
}
static RespValue *arr2(RespValue *a, RespValue *b) {
    RespValue **it = malloc(2 * sizeof *it);
    it[0] = a;
    it[1] = b;
    return resp_array(it, 2);
}
static RespValue *arr4(RespValue *a, RespValue *b, RespValue *c, RespValue *d) {
    RespValue **it = malloc(4 * sizeof *it);
    it[0] = a;
    it[1] = b;
    it[2] = c;
    it[3] = d;
    return resp_array(it, 4);
}

/* Encode `v` (consumed/freed) and assert the bytes equal expected[0..exp_len]. */
static void check_encode(RespValue *v, const char *expected, size_t exp_len) {
    unsigned char *out = NULL;
    size_t len = 0;
    RespEncodeStatus st;
    ISO_CHECK(v != NULL);
    st = resp_encode(v, &out, &len);
    ISO_CHECK_EQ_INT((int)st, (int)RESP_ENCODE_OK);
    ISO_CHECK_EQ_UINT(len, exp_len);
    if (st == RESP_ENCODE_OK) {
        if (len == exp_len) {
            ISO_CHECK_MEM_EQ(out, expected, exp_len);
        }
        free(out);
    }
    resp_free(v);
}

/* Decode input[0..inlen]; assert OK, equals `expected` (freed), consumes n. */
static void check_decode(const char *input, size_t inlen, RespValue *expected,
                         size_t exp_consumed) {
    RespValue *out = NULL;
    size_t consumed = 0;
    RespDecodeStatus st =
        resp_decode((const unsigned char *)input, inlen, &out, &consumed);
    ISO_CHECK_EQ_INT((int)st, (int)RESP_DECODE_OK);
    if (st == RESP_DECODE_OK) {
        ISO_CHECK(resp_equal(out, expected));
        ISO_CHECK_EQ_UINT(consumed, exp_consumed);
        resp_free(out);
    }
    resp_free(expected);
}

static RespDecodeStatus decode_status(const char *input, size_t inlen) {
    RespValue *out = NULL;
    size_t consumed = 0;
    RespDecodeStatus st =
        resp_decode((const unsigned char *)input, inlen, &out, &consumed);
    resp_free(out); /* NULL on non-OK */
    return st;
}

int main(void) {
    /* ---- encoding (encoder.rs) --------------------------------------- */
    check_encode(resp_simple_string("OK"), "+OK\r\n", 5);
    check_encode(resp_error("ERR boom"), "-ERR boom\r\n", 11);
    check_encode(resp_integer(-42), ":-42\r\n", 6);
    check_encode(resp_integer(123), ":123\r\n", 6);
    check_encode(resp_bulk_null(), "$-1\r\n", 5);
    check_encode(bulk("abc"), "$3\r\nabc\r\n", 9);
    check_encode(bulk("payload"), "$7\r\npayload\r\n", 13);
    check_encode(resp_array_null(), "*-1\r\n", 5);
    check_encode(arr4(resp_simple_string("OK"), resp_integer(7),
                      resp_bulk_null(), arr1(resp_simple_string("nested"))),
                 "*4\r\n+OK\r\n:7\r\n$-1\r\n*1\r\n+nested\r\n", 31);
    check_encode(bulk(""), "$0\r\n\r\n", 6); /* empty bulk string */

    /* A simple string with an embedded newline is rejected. */
    {
        RespValue *v = resp_simple_string("bad\nnews");
        unsigned char *out = (unsigned char *)1;
        size_t len = 99;
        RespEncodeStatus st = resp_encode(v, &out, &len);
        ISO_CHECK_EQ_INT((int)st, (int)RESP_ENCODE_ERR_SIMPLE_NEWLINE);
        ISO_CHECK(out == NULL);
        resp_free(v);
    }

    /* ---- error type/detail split (types.rs) -------------------------- */
    {
        RespValue *e = resp_error("ERR boom");
        RespValue *s = resp_error("ERR");
        ISO_CHECK_STR_EQ(resp_error_type(e), "ERR");
        ISO_CHECK_STR_EQ(resp_error_detail(e), "boom");
        ISO_CHECK_STR_EQ(resp_error_type(s), "ERR");
        ISO_CHECK_STR_EQ(resp_error_detail(s), "");
        resp_free(e);
        resp_free(s);
    }

    /* ---- decoding (decoder.rs) --------------------------------------- */
    check_decode("+OK\r\n", 5, resp_simple_string("OK"), 5);
    check_decode("-ERR boom\r\n", 11, resp_error("ERR boom"), 11);
    check_decode(":-42\r\n", 6, resp_integer(-42), 6);
    check_decode("$-1\r\n", 5, resp_bulk_null(), 5);
    check_decode("$3\r\nfoo\r\n", 9, bulk("foo"), 9);
    check_decode("*-1\r\n", 5, resp_array_null(), 5);
    check_decode("*2\r\n+OK\r\n:1\r\n", 13,
                 arr2(resp_simple_string("OK"), resp_integer(1)), 13);
    check_decode("PING  PONG\r\n", 12, arr2(bulk("PING"), bulk("PONG")), 12);
    check_decode("$0\r\n\r\n", 6, bulk(""), 6); /* empty bulk */

    /* Incomplete inputs return INCOMPLETE (need more bytes). */
    ISO_CHECK_EQ_INT((int)decode_status("+", 1), (int)RESP_DECODE_INCOMPLETE);
    ISO_CHECK_EQ_INT((int)decode_status("$3\r\nfo", 6),
                     (int)RESP_DECODE_INCOMPLETE);
    ISO_CHECK_EQ_INT((int)decode_status("*2\r\n+OK\r\n", 9),
                     (int)RESP_DECODE_INCOMPLETE);

    /* Malformed inputs return ERROR. */
    {
        const char inv_simple[] = {'+', (char)0xff, '\r', '\n'};
        const char inv_error[] = {'-', (char)0xff, '\r', '\n'};
        const char inv_bulklen[] = {'$', (char)0xff, '\r', '\n'};
        const char inv_arrlen[] = {'*', (char)0xff, '\r', '\n'};
        ISO_CHECK_EQ_INT((int)decode_status(inv_simple, 4),
                         (int)RESP_DECODE_ERROR);
        ISO_CHECK_EQ_INT((int)decode_status(inv_error, 4),
                         (int)RESP_DECODE_ERROR);
        ISO_CHECK_EQ_INT((int)decode_status(":foo\r\n", 6),
                         (int)RESP_DECODE_ERROR);
        ISO_CHECK_EQ_INT((int)decode_status(inv_bulklen, 4),
                         (int)RESP_DECODE_ERROR);
        ISO_CHECK_EQ_INT((int)decode_status("$-10\r\n", 6),
                         (int)RESP_DECODE_ERROR);
        ISO_CHECK_EQ_INT((int)decode_status(inv_arrlen, 4),
                         (int)RESP_DECODE_ERROR);
        ISO_CHECK_EQ_INT((int)decode_status("*-10\r\n", 6),
                         (int)RESP_DECODE_ERROR);
    }

    /* A hostile array header must not pre-allocate for the declared count:
     * "*100000000\r\n:1\r\n" declares 1e8 items but supplies one, so decoding
     * returns INCOMPLETE promptly (growing children incrementally) rather than
     * attempting an ~800 MB allocation. */
    ISO_CHECK_EQ_INT((int)decode_status("*100000000\r\n:1\r\n", 16),
                     (int)RESP_DECODE_INCOMPLETE);

    /* ---- decode_all -------------------------------------------------- */
    {
        const char *stream = "+OK\r\n:1\r\n";
        RespValue **items = NULL;
        size_t count = 0, consumed = 0;
        RespDecodeStatus st = resp_decode_all((const unsigned char *)stream,
                                              strlen(stream), &items, &count,
                                              &consumed);
        ISO_CHECK_EQ_INT((int)st, (int)RESP_DECODE_OK);
        ISO_CHECK_EQ_UINT(count, 2u);
        ISO_CHECK_EQ_UINT(consumed, 9u);
        if (count == 2) {
            RespValue *e0 = resp_simple_string("OK");
            RespValue *e1 = resp_integer(1);
            ISO_CHECK(resp_equal(items[0], e0));
            ISO_CHECK(resp_equal(items[1], e1));
            resp_free(e0);
            resp_free(e1);
        }
        {
            size_t i;
            for (i = 0; i < count; i++) {
                resp_free(items[i]);
            }
            free(items);
        }
    }

    /* ---- streaming RespDecoder --------------------------------------- */
    {
        RespDecoder *d = resp_decoder_new();
        RespValue *msg = NULL;
        ISO_CHECK(d != NULL);
        ISO_CHECK(!resp_decoder_has_message(d));
        ISO_CHECK(!resp_decoder_get_message(d, &msg)); /* empty */

        resp_decoder_feed(d, (const unsigned char *)"+OK\r\n", 5);
        ISO_CHECK(resp_decoder_has_message(d));
        ISO_CHECK(resp_decoder_get_message(d, &msg));
        {
            RespValue *ok = resp_simple_string("OK");
            ISO_CHECK(resp_equal(msg, ok));
            resp_free(ok);
        }
        resp_free(msg);
        ISO_CHECK(!resp_decoder_has_message(d));

        /* feed ":1\r\n", then decode_all("+PONG\r\n") -> [Integer 1, PONG] */
        resp_decoder_feed(d, (const unsigned char *)":1\r\n", 4);
        {
            RespValue **items = NULL;
            size_t count = 0;
            int rc = resp_decoder_decode_all(
                d, (const unsigned char *)"+PONG\r\n", 7, &items, &count);
            ISO_CHECK(rc == 1);
            ISO_CHECK_EQ_UINT(count, 2u);
            if (count == 2) {
                RespValue *e0 = resp_integer(1);
                RespValue *e1 = resp_simple_string("PONG");
                ISO_CHECK(resp_equal(items[0], e0));
                ISO_CHECK(resp_equal(items[1], e1));
                resp_free(e0);
                resp_free(e1);
            }
            {
                size_t i;
                for (i = 0; i < count; i++) {
                    resp_free(items[i]);
                }
                free(items);
            }
        }
        resp_decoder_free(d);
    }

    /* A malformed frame latches the decoder into an error state. */
    {
        RespDecoder *d = resp_decoder_new();
        RespValue **items = NULL;
        size_t count = 0;
        RespValue *msg = NULL;
        int rc = resp_decoder_decode_all(d, (const unsigned char *)"*-10\r\n", 6,
                                         &items, &count);
        ISO_CHECK(rc == 0);
        ISO_CHECK(resp_decoder_has_error(d));
        ISO_CHECK(!resp_decoder_get_message(d, &msg));
        resp_decoder_free(d);
    }

    return ISO_TEST_RESULT();
}
