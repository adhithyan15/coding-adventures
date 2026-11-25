/*
 * test_json_tfv.c -- the strict JSON reader and the strict TFV validator
 * (VER05i section 8).  A conformance harness must REFUSE a corrupted fixture,
 * so almost everything here is a rejection.
 */
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "iso_test.h"
#include "pbt.h"
#include "pbt_hooks.h"
#include "pbt_internal.h"
#include "pbt_json.h"
#include "pbt_tfv.h"
#include "test_util.h"


/* Append formatted text at buf[pos] (capacity cap, always NUL terminated) and return the new position. */
static size_t appendf(char *buf, size_t cap, size_t pos, const char *fmt, ...)
{
    va_list ap;
    int n;
    va_start(ap, fmt);
    n = vsnprintf(buf + pos, cap - pos, fmt, ap);
    va_end(ap);
    ISO_CHECK_MSG(n >= 0 && (size_t)n < cap - pos, "test buffer too small");
    return pos + (size_t)n;
}

/* Parse a NUL-terminated text; returns 0 on success (the tree is freed). */
static int accepts(const char *text)
{
    pbt_json *doc = NULL;
    char err[120];
    int rc = pbt_json_parse(text, strlen(text), &doc, err, sizeof err);
    ISO_CHECK((rc == 0) == (doc != NULL));
    pbt_json_free(doc);
    return rc == 0;
}

static void expect_ok(const char *text)
{
    if (!accepts(text)) {
        printf("  should accept: %s\n", text);
    }
    ISO_CHECK(accepts(text));
}

static void expect_bad(const char *text)
{
    if (accepts(text)) {
        printf("  should reject: %s\n", text);
    }
    ISO_CHECK(!accepts(text));
}

static void test_json_accepts(void)
{
    pbt_json *doc = NULL;
    char err[120];
    expect_ok("null");
    expect_ok("true");
    expect_ok("  false  ");
    expect_ok("0");
    expect_ok("-0");
    expect_ok("-12.5e+3");
    expect_ok("1E5");
    expect_ok("\"\"");
    expect_ok("[]");
    expect_ok("{}");
    expect_ok("[1, 2 ,3]");
    expect_ok("{\"a\": [true, null], \"b\": {\"c\": \"d\"}}");
    expect_ok("\"\\u00e9\\ud83d\\ude00\\n\\t\\/\\\\\\\"\\b\\f\\r\"");
    expect_ok("\"caf\xc3\xa9 \xe2\x82\xac \xf0\x9f\x98\x80\"");

    /* Numbers are kept as TEXT; strings are decoded; \u0000 is legal and counted. */
    ISO_CHECK(pbt_json_parse("[-12.50e+3, \"a\\u0000b\", \"\\u00e9\"]", 33, &doc, err, sizeof err) == 0);
    ISO_CHECK(doc != NULL && doc->type == PBT_J_ARR && doc->n == 3);
    if (doc != NULL && doc->n == 3) {
        ISO_CHECK(doc->items[0].type == PBT_J_NUM);
        ISO_CHECK_STR_EQ(doc->items[0].s, "-12.50e+3");
        ISO_CHECK_EQ_UINT(doc->items[1].slen, 3);
        ISO_CHECK(memcmp(doc->items[1].s, "a\0b", 4) == 0);
        ISO_CHECK_STR_EQ(doc->items[2].s, "\xc3\xa9");
    }
    pbt_json_free(doc);

    /* Object access. */
    ISO_CHECK(pbt_json_parse("{\"x\":1,\"y\":[true]}", 18, &doc, err, sizeof err) == 0);
    ISO_CHECK(pbt_json_get(doc, "x") != NULL && pbt_json_get(doc, "x")->type == PBT_J_NUM);
    ISO_CHECK(pbt_json_get(doc, "nope") == NULL);
    ISO_CHECK(pbt_json_get(&doc->items[0], "x") == NULL); /* not an object */
    ISO_CHECK(pbt_json_get(NULL, "x") == NULL);
    pbt_json_free(doc);
    pbt_json_free(NULL); /* harmless */
}

static void test_json_rejects(void)
{
    /* Structure. */
    expect_bad("");
    expect_bad("   ");
    expect_bad("[");
    expect_bad("]");
    expect_bad("[1,]");
    expect_bad("[,1]");
    expect_bad("[1 2]");
    expect_bad("{");
    expect_bad("{\"a\"}");
    expect_bad("{\"a\":}");
    expect_bad("{\"a\":1,}");
    expect_bad("{a:1}");
    expect_bad("{\"a\":1 \"b\":2}");
    expect_bad("{1:2}");
    expect_bad("tru");
    expect_bad("nul");
    expect_bad("True");
    expect_bad("'a'");
    /* Trailing data. */
    expect_bad("1 2");
    expect_bad("{} x");
    expect_bad("[]]");
    expect_bad("null,");
    /* Numbers: leading zeros and every other malformed shape. */
    expect_bad("01");
    expect_bad("-01");
    expect_bad("00");
    expect_bad("+1");
    expect_bad("-");
    expect_bad(".5");
    expect_bad("1.");
    expect_bad("1.e3");
    expect_bad("1e");
    expect_bad("1e+");
    expect_bad("0x10");
    expect_bad("NaN");
    expect_bad("Infinity");
    expect_bad("[01]");
    /* Strings: raw control characters, bad escapes, truncation. */
    expect_bad("\"a\nb\"");
    expect_bad("\"a\tb\"");
    expect_bad("\"a\x01\"");
    expect_bad("\"\\x41\"");
    expect_bad("\"\\u12\"");
    expect_bad("\"\\u12g4\"");
    expect_bad("\"abc");
    expect_bad("\"abc\\");
    expect_bad("\"\\");
    /* Surrogate escapes: lone, reversed, truncated, high without low. */
    expect_bad("\"\\ud800\"");
    expect_bad("\"\\udc00\"");
    expect_bad("\"\\ud800\\u0041\"");
    expect_bad("\"\\ud800x\"");
    expect_bad("\"\\ud800\\ud800\"");
    expect_bad("\"\\udc00\\ud800\"");
    expect_bad("\"\\ud800\\");
    /* Invalid UTF-8 inside a string: lone continuation, overlong, surrogate, too large, truncated. */
    expect_bad("\"\x80\"");
    expect_bad("\"\xc0\xaf\"");
    expect_bad("\"\xc1\xbf\"");
    expect_bad("\"\xe0\x80\xaf\"");
    expect_bad("\"\xed\xa0\x80\"");
    expect_bad("\"\xf0\x80\x80\xaf\"");
    expect_bad("\"\xf4\x90\x80\x80\"");
    expect_bad("\"\xf5\x80\x80\x80\"");
    expect_bad("\"\xff\"");
    expect_bad("\"\xe2\x82\"");
    expect_bad("\"\xc3\"");
    /* Boundary code points that ARE valid. */
    expect_ok("\"\xc2\x80\"");
    expect_ok("\"\xdf\xbf\"");
    expect_ok("\"\xe0\xa0\x80\"");
    expect_ok("\"\xed\x9f\xbf\"");
    expect_ok("\"\xee\x80\x80\"");
    expect_ok("\"\xf0\x90\x80\x80\"");
    expect_ok("\"\xf4\x8f\xbf\xbf\"");
    /* Duplicate keys: plain, nested, and differing only by an escape that decodes equal. */
    expect_bad("{\"a\":1,\"a\":2}");
    expect_bad("{\"a\":1,\"b\":2,\"a\":3}");
    expect_bad("[{\"k\":{\"x\":1,\"x\":2}}]");
    expect_bad("{\"a\":1,\"\\u0061\":2}");
    expect_ok("{\"a\":1,\"A\":2,\"aa\":3,\"\":4}");
    expect_ok("{\"a\":{\"a\":1},\"b\":{\"a\":1}}"); /* the same key in DIFFERENT objects is fine */
}

static void test_json_limits(void)
{
    char *buf;
    size_t i;
    size_t n;
    pbt_json *doc = NULL;
    char err[120];

    /* Depth: 64 nested arrays are accepted, 65 are not (and neither overflows the C stack). */
    buf = (char *)malloc(300);
    for (i = 0; i < 64; i++) {
        buf[i] = '[';
        buf[64 + i] = ']';
    }
    buf[128] = '\0';
    ISO_CHECK(pbt_json_parse(buf, 128, &doc, err, sizeof err) == 0);
    pbt_json_free(doc);
    for (i = 0; i < 65; i++) {
        buf[i] = '[';
        buf[65 + i] = ']';
    }
    buf[130] = '\0';
    ISO_CHECK(pbt_json_parse(buf, 130, &doc, err, sizeof err) != 0);
    ISO_CHECK(doc == NULL);
    ISO_CHECK(strstr(err, "deep") != NULL);
    free(buf);

    /* Depth bombs far past the limit are refused quickly, not recursed into. */
    n = 1000000;
    buf = (char *)malloc(n + 1);
    memset(buf, '[', n);
    buf[n] = '\0';
    ISO_CHECK(pbt_json_parse(buf, n, &doc, err, sizeof err) != 0);
    free(buf);
    buf = (char *)malloc(n * 6 + 1);
    for (i = 0; i < n; i++) {
        memcpy(buf + i * 6, "{\"a\":", 5);
    }
    buf[n * 6] = '\0';
    ISO_CHECK(pbt_json_parse(buf, n * 5, &doc, err, sizeof err) != 0);
    free(buf);

    /* Size: a document over 8 MiB is refused before parsing. */
    n = PBT_JSON_MAX_BYTES + 1;
    buf = (char *)malloc(n + 1);
    memset(buf, ' ', n);
    buf[n] = '\0';
    ISO_CHECK(pbt_json_parse(buf, n, &doc, err, sizeof err) != 0);
    ISO_CHECK(doc == NULL);
    ISO_CHECK(strstr(err, "exceeds") != NULL);
    free(buf);

    /* Node count: 200,000 values are accepted, 200,001 are not. */
    n = PBT_JSON_MAX_NODES;
    buf = (char *)malloc(2 * n + 4);
    buf[0] = '[';
    for (i = 0; i + 1 < n; i++) {
        buf[1 + 2 * i] = '0';
        buf[2 + 2 * i] = ',';
    }
    buf[1 + 2 * (n - 2) + 2] = '\0';
    /* values: the array itself plus (n - 1) elements = n nodes. */
    {
        size_t len = 1 + 2 * (n - 2) + 1; /* "[" + (n-2) "0," pairs + last element placeholder */
        buf[len - 1] = '0';
        buf[len] = ']';
        buf[len + 1] = '\0';
        len += 1;
        /* count: the array plus n - 2 + 1 = n - 1 elements = n nodes. */
        ISO_CHECK(pbt_json_parse(buf, len, &doc, err, sizeof err) == 0);
        pbt_json_free(doc);
        doc = NULL;
        /* One more element pushes it over. */
        buf[len - 1] = ',';
        buf[len] = '0';
        buf[len + 1] = ']';
        buf[len + 2] = '\0';
        ISO_CHECK(pbt_json_parse(buf, len + 2, &doc, err, sizeof err) != 0);
        ISO_CHECK(doc == NULL);
        ISO_CHECK(strstr(err, "too many") != NULL);
    }
    free(buf);

    /* An object with 100,000 distinct keys parses quickly (the duplicate check sorts); with a late duplicate it fails. */
    n = 100000;
    buf = (char *)malloc(n * 16 + 16);
    {
        size_t cap = n * 16 + 16;
        size_t pos = 0;
        pos = appendf(buf, cap, pos, "{");
        for (i = 0; i < n; i++) {
            pos = appendf(buf, cap, pos, "%s\"k%lu\":0", i == 0 ? "" : ",", (unsigned long)i);
        }
        pos = appendf(buf, cap, pos, "}");
        ISO_CHECK(pbt_json_parse(buf, pos, &doc, err, sizeof err) == 0);
        pbt_json_free(doc);
        doc = NULL;
        pos--; /* drop the closing brace */
        pos = appendf(buf, cap, pos, ",\"k77\":1}");
        ISO_CHECK(pbt_json_parse(buf, pos, &doc, err, sizeof err) != 0);
        ISO_CHECK(strstr(err, "duplicate") != NULL);
    }
    free(buf);

    /* Error messages are short and name a position. */
    ISO_CHECK(pbt_json_parse("[1,", 3, &doc, err, sizeof err) != 0);
    ISO_CHECK(strstr(err, "byte") != NULL);
    /* No error buffer is fine. */
    ISO_CHECK(pbt_json_parse("[1,", 3, &doc, NULL, 0) != 0);
}

static void test_json_clone(void)
{
    pbt_json *doc = NULL;
    pbt_json *copy;
    char err[100];
    const char *text = "{\"a\":[1,{\"b\":\"x\"},null,true],\"c\":\"\"}";
    ISO_CHECK(pbt_json_parse(text, strlen(text), &doc, err, sizeof err) == 0);
    copy = pbt_json_clone(doc);
    ISO_CHECK(copy != NULL);
    if (copy != NULL) {
        pbt_json *a = pbt_json_get(copy, "a");
        ISO_CHECK(a != NULL && a->n == 4);
        ISO_CHECK_STR_EQ(pbt_json_get(&a->items[1], "b")->s, "x");
        ISO_CHECK(pbt_json_get(copy, "a") != pbt_json_get(doc, "a")); /* deep: distinct storage */
        tu_set_str(pbt_json_get(&a->items[1], "b"), "changed");
        ISO_CHECK_STR_EQ(pbt_json_get(&pbt_json_get(doc, "a")->items[1], "b")->s, "x");
    }
    pbt_json_free(copy);
    pbt_json_free(doc);
}

static void test_json_oom(void)
{
    const char *text = "{\"a\":[1,{\"b\":\"x\"},null,true],\"c\":\"\",\"d\":[[],{}]}";
    long n;
    int fired_any = 0;
    for (n = 1; n < 400; n++) {
        pbt_json *doc = NULL;
        pbt_json *copy;
        char err[100];
        long baseline = pbt_hooks.live_allocs;
        long before = pbt_hooks.injected;
        int rc;
        pbt_hooks.fail_after = n;
        rc = pbt_json_parse(text, strlen(text), &doc, err, sizeof err);
        pbt_hooks.fail_after = 0;
        if (pbt_hooks.injected != before) {
            fired_any = 1;
            ISO_CHECK(rc != 0); /* a lost allocation is never a successful parse */
            ISO_CHECK(doc == NULL);
        } else {
            ISO_CHECK(rc == 0);
        }
        pbt_json_free(doc);
        if (rc == 0) {
            /* The clone path too. */
            char err2[100];
            ISO_CHECK(pbt_json_parse(text, strlen(text), &doc, err2, sizeof err2) == 0);
            pbt_hooks.fail_after = n;
            copy = pbt_json_clone(doc);
            pbt_hooks.fail_after = 0;
            pbt_json_free(copy);
            pbt_json_free(doc);
        }
        ISO_CHECK_EQ_INT(pbt_hooks.live_allocs, baseline);
        if (pbt_hooks.injected == before) {
            break; /* n exceeded the allocations of one parse: the sweep is complete */
        }
    }
    ISO_CHECK(fired_any);
}

/* ============================ TFV ============================ */

static int tfv_ok(const char *text)
{
    char err[160];
    return pbt_tfv_validate(text, strlen(text), err, sizeof err) == 0;
}

static void tfv_good(const char *text)
{
    if (!tfv_ok(text)) {
        char err[160];
        (void)pbt_tfv_validate(text, strlen(text), err, sizeof err);
        printf("  should accept: %s -> %s\n", text, err);
    }
    ISO_CHECK(tfv_ok(text));
}

static void tfv_bad(const char *text)
{
    if (tfv_ok(text)) {
        printf("  should reject: %s\n", text);
    }
    ISO_CHECK(!tfv_ok(text));
}

static void test_tfv_accepts(void)
{
    tfv_good("tfv 1");
    tfv_good("tfv 1\n");
    tfv_good("tfv 1\n\n# a comment\n\nrec R1 Kop S706274\n");
    tfv_good("tfv 1\nr R0\n");
    tfv_good("tfv 1\nr R3 Ka N Kb T Kc F\n");
    tfv_good("tfv 1\nr R1 Ki I0\n");
    tfv_good("tfv 1\nr R1 Ki I-12\n");
    tfv_good("tfv 1\nr R1 Ki I123456789012345678901234567890\n");
    tfv_good("tfv 1\nr R1 Kf D3ff0000000000000\n");
    tfv_good("tfv 1\nr R1 Kb X00ff\n");
    tfv_good("tfv 1\nr R1 Kb X\n");
    tfv_good("tfv 1\nr R1 Ks S\n");
    tfv_good("tfv 1\nr R1 Ks Sc3a9\n"); /* e-acute */
    tfv_good("tfv 1\nr R1 Kl L2 I1 L1 N\n");
    tfv_good("tfv 1\nr R1 Km M2 I1 T I2 F\n");
    tfv_good("tfv 1\nr R1 Km M0\n");
    tfv_good("tfv 1\na.b_c/d-e R1 Kx N\nA R1 Kx N\n");
    /* -0 and +0 stay distinct map keys. */
    tfv_good("tfv 1\nr R1 Km M2 D0000000000000000 I1 D8000000000000000 I2\n");
    /* Maps are order independent only for their own keys: these two keys are different lists. */
    tfv_good("tfv 1\nr R1 Km M2 L2 I1 I2 N L2 I2 I1 N\n");
    /* A map-valued key: entries compare as sets. */
    tfv_good("tfv 1\nr R1 Km M2 M1 I1 I2 T M1 I1 I3 T\n");
}

static void test_tfv_corpus_reject_texts(void)
{
    tfv_bad("bad header\n");
    tfv_bad("tfv 1\nr R1 Kop S706274 Kdata XFF\n");
}

static void test_tfv_rejects(void)
{
    /* Header. */
    tfv_bad("");
    tfv_bad("\n");
    tfv_bad("tfv 2\n");
    tfv_bad("tfv 1 \n");
    tfv_bad("TFV 1\n");
    tfv_bad("tfv 1\r\n");
    tfv_bad(" tfv 1\n");
    tfv_bad("tfv 10\n");
    tfv_bad("tfv\n");
    /* Spacing. */
    tfv_bad("tfv 1\nr  R1 Ka N\n");
    tfv_bad("tfv 1\nr R1 Ka N \n");
    tfv_bad("tfv 1\n r R1 Ka N\n");
    tfv_bad("tfv 1\nr R1 Ka N\r\n");
    tfv_bad("tfv 1\nr\tR1 Ka N\n");
    /* Record shape. */
    tfv_bad("tfv 1\nr\n");
    tfv_bad("tfv 1\nr N\n");
    tfv_bad("tfv 1\nr R\n");
    tfv_bad("tfv 1\nr R01 Ka N\n");
    tfv_bad("tfv 1\nr R-1\n");
    tfv_bad("tfv 1\nr Rx\n");
    tfv_bad("tfv 1\nr R1000000000 Ka N\n");
    tfv_bad("tfv 1\nr R2 Ka N\n");        /* count disagrees: ends early */
    tfv_bad("tfv 1\nr R1 Ka N Kb N\n");   /* trailing tokens */
    tfv_bad("tfv 1\nr R1 a N\n");         /* key without K */
    tfv_bad("tfv 1\nr R1 KA N\n");        /* uppercase field */
    tfv_bad("tfv 1\nr R1 K1a N\n");       /* field starts with a digit */
    tfv_bad("tfv 1\nr R1 K N\n");
    tfv_bad("tfv 1\nr R1 Ka\n");          /* missing value */
    tfv_bad("tfv 1\nb@d R1 Ka N\n");      /* bad id byte */
    /* Duplicates. */
    tfv_bad("tfv 1\nr R2 Ka N Ka T\n");
    tfv_bad("tfv 1\nr R1 Ka N\nr R1 Kb N\n");
    tfv_bad("tfv 1\nr R1 Km M2 I1 T I1 F\n");
    tfv_bad("tfv 1\nr R1 Km M2 N T N F\n");
    tfv_bad("tfv 1\nr R1 Km M2 L1 I1 T L1 I1 F\n");
    tfv_bad("tfv 1\nr R1 Km M2 M1 I1 I2 T M1 I1 I2 F\n");
    /* Every NaN equals every NaN. */
    tfv_bad("tfv 1\nr R1 Km M2 D7ff8000000000000 I1 D7ff8000000000001 I2\n");
    tfv_bad("tfv 1\nr R1 Km M2 DfffffffffffffFFF I1 D7ff0000000000001 I2\n"); /* also uppercase: bad anyway */
    /* Scalars. */
    tfv_bad("tfv 1\nr R1 Ka Nx\n");
    tfv_bad("tfv 1\nr R1 Ka T1\n");
    tfv_bad("tfv 1\nr R1 Ka I\n");
    tfv_bad("tfv 1\nr R1 Ka I-\n");
    tfv_bad("tfv 1\nr R1 Ka I-0\n");
    tfv_bad("tfv 1\nr R1 Ka I007\n");
    tfv_bad("tfv 1\nr R1 Ka I00\n");
    tfv_bad("tfv 1\nr R1 Ka I1a\n");
    tfv_bad("tfv 1\nr R1 Ka I+1\n");
    tfv_bad("tfv 1\nr R1 Ka D3ff000000000000\n");     /* 15 digits */
    tfv_bad("tfv 1\nr R1 Ka D3ff00000000000000\n");   /* 17 digits */
    tfv_bad("tfv 1\nr R1 Ka D3FF0000000000000\n");    /* uppercase */
    tfv_bad("tfv 1\nr R1 Ka D3ff000000000000g\n");
    tfv_bad("tfv 1\nr R1 Ka XFF\n");
    tfv_bad("tfv 1\nr R1 Ka Xf\n");                   /* odd length */
    tfv_bad("tfv 1\nr R1 Ka Xgg\n");
    tfv_bad("tfv 1\nr R1 Ka Sff\n");                  /* not UTF-8 */
    tfv_bad("tfv 1\nr R1 Ka Sc0af\n");                /* overlong */
    tfv_bad("tfv 1\nr R1 Ka Sc3\n");                  /* truncated */
    tfv_bad("tfv 1\nr R1 Ka Seda080\n");             /* surrogate */
    tfv_bad("tfv 1\nr R1 Ka Q1\n");                   /* unknown tag */
    tfv_bad("tfv 1\nr R1 Ka n\n");
    /* Containers. */
    tfv_bad("tfv 1\nr R1 Ka L\n");
    tfv_bad("tfv 1\nr R1 Ka L01 N\n");
    tfv_bad("tfv 1\nr R1 Ka L2 N\n");
    tfv_bad("tfv 1\nr R1 Ka L1 N N\n");
    tfv_bad("tfv 1\nr R1 Ka M1 N\n");
    tfv_bad("tfv 1\nr R1 Ka Lx\n");
    tfv_bad("tfv 1\nr R1 Ka L1000000000\n");
    /* Comments only skip WHOLE lines starting with '#'. */
    tfv_bad("tfv 1\nr R1 Ka N # trailing comment\n");
    tfv_bad("tfv 1\n #indented comment\n");
    /* Invalid UTF-8 anywhere in the document. */
    tfv_bad("tfv 1\n# \xff\n");
    tfv_bad("tfv 1\n# \xc3\n");
}

static void test_tfv_limits(void)
{
    char *buf;
    size_t i;
    size_t n;
    char err[160];

    /* Depth: 64 nested lists are accepted, 65 are not. */
    buf = (char *)malloc(4000);
    n = appendf(buf, 4000, 0, "tfv 1\nr R1 Ka");
    for (i = 0; i < 64; i++) {
        n = appendf(buf, 4000, n, " L1");
    }
    n = appendf(buf, 4000, n, " N\n");
    ISO_CHECK(pbt_tfv_validate(buf, n, err, sizeof err) == 0);
    n = appendf(buf, 4000, 0, "tfv 1\nr R1 Ka");
    for (i = 0; i < 65; i++) {
        n = appendf(buf, 4000, n, " L1");
    }
    n = appendf(buf, 4000, n, " N\n");
    ISO_CHECK(pbt_tfv_validate(buf, n, err, sizeof err) != 0);
    ISO_CHECK(strstr(err, "deep") != NULL);
    free(buf);

    /* Integer digits: 1000 accepted, 1001 refused. */
    buf = (char *)malloc(1200);
    n = appendf(buf, 1200, 0, "tfv 1\nr R1 Ka I");
    memset(buf + n, '9', 1000);
    n += 1000;
    buf[n++] = '\n';
    ISO_CHECK(pbt_tfv_validate(buf, n, err, sizeof err) == 0);
    buf[n - 1] = '9';
    buf[n++] = '\n';
    ISO_CHECK(pbt_tfv_validate(buf, n, err, sizeof err) != 0);
    free(buf);

    /* Size: more than 1 MiB is refused up front. */
    n = PBT_TFV_MAX_BYTES + 1;
    buf = (char *)malloc(n + 1);
    memset(buf, '#', n);
    memcpy(buf, "tfv 1\n", 6);
    ISO_CHECK(pbt_tfv_validate(buf, n, err, sizeof err) != 0);
    ISO_CHECK(strstr(err, "too large") != NULL);
    free(buf);

    /* Token budget: a list of 200,000 "N" tokens is over the cap. */
    n = 200100 * 2 + 64;
    buf = (char *)malloc(n);
    {
        size_t pos = appendf(buf, n, 0, "tfv 1\nr R1 Ka L200000");
        for (i = 0; i < 200000; i++) {
            buf[pos++] = ' ';
            buf[pos++] = 'N';
        }
        buf[pos++] = '\n';
        ISO_CHECK(pbt_tfv_validate(buf, pos, err, sizeof err) != 0);
        ISO_CHECK(strstr(err, "too many") != NULL);
    }
    free(buf);

    /* A hostile count is only a loop bound: the tokens run out first, with no big allocation. */
    ISO_CHECK(!tfv_ok("tfv 1\nr R1 Ka L999999999 N N\n"));

    /* A map with many distinct keys is validated by sorting, not by pairwise comparison. */
    n = 50000 * 12 + 64;
    buf = (char *)malloc(n);
    {
        size_t pos = appendf(buf, n, 0, "tfv 1\nr R1 Km M50000");
        for (i = 0; i < 50000; i++) {
            pos = appendf(buf, n, pos, " I%lu N", (unsigned long)i);
        }
        buf[pos++] = '\n';
        ISO_CHECK(pbt_tfv_validate(buf, pos, err, sizeof err) == 0);
    }
    free(buf);

    /* No error buffer is fine. */
    ISO_CHECK(pbt_tfv_validate("bad", 3, NULL, 0) != 0);
}

/* Sweep fail_after over EVERY allocation of one TFV input: never valid after a lost allocation, never a leak, never a double free (ASan). */
static long tfv_oom_sweep(const char *text, int expect_valid)
{
    long n;
    long runs = 0;
    int fired_any = 0;
    ISO_CHECK(tfv_ok(text) == expect_valid);
    for (n = 1; n < 2000; n++) {
        long baseline = pbt_hooks.live_allocs;
        long before = pbt_hooks.injected;
        char err[160];
        int rc;
        pbt_hooks.fail_after = n;
        rc = pbt_tfv_validate(text, strlen(text), err, sizeof err);
        pbt_hooks.fail_after = 0;
        runs++;
        if (pbt_hooks.injected != before) {
            fired_any = 1;
            ISO_CHECK(rc != 0); /* a lost allocation is never "valid" */
        } else {
            ISO_CHECK((rc == 0) == expect_valid);
        }
        ISO_CHECK_EQ_INT(pbt_hooks.live_allocs, baseline);
        if (pbt_hooks.injected == before) {
            break;
        }
    }
    ISO_CHECK(fired_any);
    return runs;
}

static void test_tfv_oom(void)
{
    static const char *const valid[] = {
        "tfv 1\nr R2 Km M2 L1 I1 T M1 I2 L1 Sc3a9 F Kl L2 N N\nq R1 Kz D7ff8000000000000\n",
        /* the regression input: a list as a map KEY, then a map as a map key, nested deeper */
        "tfv 1\nr R1 Ka M1 L0 N\n",
        "tfv 1\nr R1 Ka M1 L2 N L1 I1 M1 I1 I2\n",
        "tfv 1\nr R1 Ka M2 M1 L1 N T L1 M1 N N M1 L2 I1 I2 N F\n",
        "tfv 1\nr R1 Ka M3 L1 I1 L0 L1 I2 L0 L0 L0\n",
    };
    static const char *const invalid[] = {
        "tfv 1\nr R1 Ka M2 L1 I1 T L1 I1 F\n",          /* duplicate list key */
        "tfv 1\nr R1 Ka M2 M1 L0 N T M1 L0 N F\n",      /* duplicate map key */
    };
    size_t i;
    long runs = 0;
    for (i = 0; i < sizeof valid / sizeof valid[0]; i++) {
        runs += tfv_oom_sweep(valid[i], 1);
    }
    for (i = 0; i < sizeof invalid / sizeof invalid[0]; i++) {
        runs += tfv_oom_sweep(invalid[i], 0);
    }
    printf("  %ld TFV injected-failure runs\n", runs);
}

int main(void)
{
    pbt_hooks_reset();
    test_json_accepts();
    test_json_rejects();
    test_json_limits();
    test_json_clone();
    test_json_oom();
    test_tfv_accepts();
    test_tfv_corpus_reject_texts();
    test_tfv_rejects();
    test_tfv_limits();
    test_tfv_oom();
    ISO_CHECK_EQ_INT(pbt_hooks.live_allocs, 0);
    return ISO_TEST_RESULT();
}
