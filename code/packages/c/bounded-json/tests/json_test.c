/* fopen is ISO C17; Microsoft's alternative fopen_s is not. This fixture-only
 * compatibility definition does not disable compiler warnings or library checks. */
#ifdef _MSC_VER
#define _CRT_SECURE_NO_WARNINGS
#endif
#include "bounded_json.h"
#include "iso_test.h"
#include <float.h>
#include <locale.h>
#include <stdint.h>
#include <stdlib.h>

static BJsonDoc *parse(const char *text) {
    BJsonDoc *doc = NULL;
    BJsonError error;
    ISO_CHECK(bj_parse(text, strlen(text), &doc, &error) == BJ_OK);
    ISO_CHECK(error.kind == BJ_OK && error.offset == 0);
    return doc;
}
static void rejects(const char *text) {
    BJsonDoc *doc = NULL;
    BJsonError error;
    ISO_CHECK(bj_parse(text, strlen(text), &doc, &error) != BJ_OK);
    ISO_CHECK(doc == NULL && error.kind != BJ_OK && error.offset <= strlen(text));
}
static const BJsonValue *field(const BJsonValue *v, const char *wanted) {
    size_t i;
    for (i = 0; i < bj_size(v); i++) {
        const char *key; size_t n; const BJsonValue *child;
        if (bj_member(v, i, &key, &n, &child) && n == strlen(wanted) && !memcmp(key, wanted, n)) return child;
    }
    return NULL;
}
static void values_and_accessors(void) {
    BJsonDoc *doc = parse(" \t\r\n{\"a\":[null,true,false,-42,1.25,\"x\"],\"a\":2,\"a\\u0000z\":3} \r\n");
    const BJsonValue *root = bj_root(doc), *array = field(root, "a"), *child;
    const char *text; size_t n; int flag = 7; int64_t integer = 99; double number = 99;
    ISO_CHECK(bj_kind(root) == BJ_OBJECT && bj_size(root) == 3);
    ISO_CHECK(bj_kind(array) == BJ_ARRAY && bj_size(array) == 6);
    ISO_CHECK(bj_kind(bj_at(array, 0)) == BJ_NULL);
    ISO_CHECK(bj_boolean(bj_at(array, 1), &flag) && flag == 1);
    ISO_CHECK(bj_boolean(bj_at(array, 2), &flag) && flag == 0);
    ISO_CHECK(bj_integer(bj_at(array, 3), &integer) && integer == -42);
    ISO_CHECK(bj_number(bj_at(array, 3), &number) && number == -42);
    ISO_CHECK(bj_number(bj_at(array, 4), &number) && number == 1.25);
    ISO_CHECK(bj_string(bj_at(array, 5), &text, &n) && n == 1 && text[0] == 'x');
    ISO_CHECK(bj_member(root, 1, &text, &n, &child) && n == 1 && text[0] == 'a');
    ISO_CHECK(bj_integer(child, &integer) && integer == 2);
    ISO_CHECK(bj_member(root, 2, &text, &n, &child) && n == 3 && text[1] == '\0' && text[2] == 'z');
    ISO_CHECK(!bj_at(root, 0) && !bj_at(NULL, 0) && !bj_at(array, SIZE_MAX));
    ISO_CHECK(!bj_root(NULL) && bj_kind(NULL) == BJ_INVALID && bj_size(NULL) == 0);
    ISO_CHECK(bj_size(bj_at(array, 3)) == 0);
    ISO_CHECK(!bj_member(root, 3, &text, &n, &child));
    ISO_CHECK(!bj_member(array, 0, &text, &n, &child));
    ISO_CHECK(!bj_member(NULL, 0, &text, &n, &child));
    ISO_CHECK(!bj_member(root, 0, NULL, &n, &child));
    ISO_CHECK(!bj_member(root, 0, &text, NULL, &child));
    ISO_CHECK(!bj_member(root, 0, &text, &n, NULL));
    ISO_CHECK(!bj_boolean(NULL, &flag) && !bj_boolean(root, &flag) && !bj_boolean(bj_at(array, 1), NULL));
    ISO_CHECK(!bj_integer(NULL, &integer) && !bj_integer(bj_at(array, 4), &integer) && !bj_integer(bj_at(array, 3), NULL));
    ISO_CHECK(!bj_number(NULL, &number) && !bj_number(root, &number) && !bj_number(bj_at(array, 3), NULL));
    ISO_CHECK(!bj_string(NULL, &text, &n) && !bj_string(root, &text, &n));
    ISO_CHECK(!bj_string(bj_at(array, 5), NULL, &n) && !bj_string(bj_at(array, 5), &text, NULL));
    bj_doc_free(&doc); bj_doc_free(&doc); bj_doc_free(NULL);
    ISO_CHECK(!doc);
    doc = parse("[]"); ISO_CHECK(bj_size(bj_root(doc)) == 0); bj_doc_free(&doc);
    doc = parse("{}"); ISO_CHECK(bj_size(bj_root(doc)) == 0); bj_doc_free(&doc);
    {
        char buffer[] = "[\"owned\",45]";
        ISO_CHECK(bj_parse(buffer, strlen(buffer), &doc, NULL) == BJ_OK);
        memset(buffer, 'x', strlen(buffer));
        ISO_CHECK(bj_string(bj_at(bj_root(doc), 0), &text, &n) && n == 5 && !memcmp(text, "owned", 5));
        ISO_CHECK(bj_integer(bj_at(bj_root(doc), 1), &integer) && integer == 45);
        ISO_CHECK(bj_parse("0", 1, &doc, NULL) == BJ_ARGUMENT);
        ISO_CHECK(bj_kind(bj_root(doc)) == BJ_ARRAY); /* existing owner preserved */
        bj_doc_free(&doc);
    }
}
static void grammar_and_unicode(void) {
    const char *bad[] = {"", " ", "nul", "nullx", "True", "false true", "tru", "fals", "+1", ".1", "01", "-01", "-", "1.", "1e", "1E+", "1e--2", "NaN", "Infinity", "1e9999", "-1e9999", "[", "[1", "[1,]", "[1 2]", "{", "{x:0}", "{\"x\"}", "{\"x\":}", "{\"x\":0,}", "{\"x\":0 \"y\":1}", "\"", "\"x", "\"\\", "\"\\q\"", "\"\\u\"", "\"\\u0x00\"", "\"\\uD800\"", "\"\\uD800\\x0000\"", "\"\\uD800\\u0000\"", "\"\\uDC00\"", "\"\n\"", "\"\xff\"", "\"\xc0\x80\"", "\"\xe0\x80\x80\"", "\"\xed\xa0\x80\"", "\"\xf4\x90\x80\x80\"", "\"\xe2x\"", "\"\xf0\x80\x80\x80\"", "\"\xc2", "\"\xe2\x82", "\"\xf0\x90\x80"};
    size_t i, length;
    const char *text;
    BJsonDoc *doc;
    for (i = 0; i < sizeof(bad) / sizeof(bad[0]); i++) rejects(bad[i]);
    doc = parse("\"\\\"\\\\\\/\\b\\f\\n\\r\\t\\u0000\\u007f\\u00a2\\u20ac\\uD83D\\uDE00\"");
    ISO_CHECK(bj_string(bj_root(doc), &text, &length));
    ISO_CHECK(length == 19 && !memcmp(text, "\"\\/\b\f\n\r\t\0\x7f\xc2\xa2\xe2\x82\xac\xf0\x9f\x98\x80", 19));
    bj_doc_free(&doc);
    doc = parse("\"\xc2\xa2\xe2\x82\xac\xf0\x9f\x98\x80\"");
    ISO_CHECK(bj_string(bj_root(doc), &text, &length) && length == 9);
    bj_doc_free(&doc);
    doc = parse("\"\\u0041\\uabcd\""); bj_doc_free(&doc);
    {
        const char embedded[] = {'n','u','l','l','\0'};
        ISO_CHECK(bj_parse(embedded, sizeof(embedded), &doc, NULL) == BJ_SYNTAX);
    }
}
static void numbers_and_locale(void) {
    const char *numbers[] = {"0", "-0", "9223372036854775807", "-9223372036854775808", "9223372036854775808", "-9223372036854775809", "1.0", "1e0", "1E+2", "-0.25e-2", "1.7976931348623157e308", "-1.7976931348623157e308", "5e-324", "1e-9999"};
    size_t i;
    for (i = 0; i < sizeof(numbers) / sizeof(numbers[0]); i++) {
        BJsonDoc *doc = parse(numbers[i]); int64_t integer; double number;
        ISO_CHECK(bj_number(bj_root(doc), &number));
        ISO_CHECK(bj_kind(bj_root(doc)) == ((i < 4) ? BJ_INTEGER : BJ_FLOAT));
        ISO_CHECK(bj_integer(bj_root(doc), &integer) == (i < 4));
        if (i == 2) ISO_CHECK(integer == INT64_MAX);
        if (i == 3) ISO_CHECK(integer == INT64_MIN);
        if (i == 10) ISO_CHECK(number == DBL_MAX);
        if (i == 13) ISO_CHECK(number == 0);
        bj_doc_free(&doc);
    }
    /* The test changes the host locale; the library must not. Try spellings
     * available on both POSIX and Windows, without requiring optional locales. */
    {
        const char *locales[] = {"de_DE.UTF-8", "German_Germany.1252", "de-DE", "C"};
        for (i = 0; i < sizeof(locales) / sizeof(locales[0]); i++) {
            const char *selected = setlocale(LC_NUMERIC, locales[i]);
            if (selected) {
                char saved[256]; BJsonDoc *doc; double n;
                size_t selected_length = strlen(selected);
                ISO_CHECK(selected_length < sizeof(saved));
                if (selected_length >= sizeof(saved)) continue;
                memcpy(saved, selected, selected_length + 1);
                doc = parse("1234.5e-1");
                ISO_CHECK(bj_number(bj_root(doc), &n) && n == 123.45);
                ISO_CHECK(!strcmp(saved, setlocale(LC_NUMERIC, NULL)));
                bj_doc_free(&doc);
            }
        }
        ISO_CHECK(setlocale(LC_NUMERIC, "C") != NULL);
    }
}
static void limits(void) {
    BJsonOptions o = bj_options(); BJsonDoc *doc = NULL; BJsonError e;
    char nested[260]; size_t i;
    ISO_CHECK(bj_parse(NULL, 0, &doc, &e) == BJ_ARGUMENT);
    ISO_CHECK(bj_parse("0", 1, NULL, NULL) == BJ_ARGUMENT);
    o.max_bytes = 0; ISO_CHECK(bj_parse_with_options("0", 1, &o, &doc, &e) == BJ_ARGUMENT);
    o = bj_options(); o.max_bytes++; ISO_CHECK(bj_parse_with_options("0", 1, &o, &doc, NULL) == BJ_ARGUMENT);
    o = bj_options(); o.max_depth++; ISO_CHECK(bj_parse_with_options("0", 1, &o, &doc, NULL) == BJ_ARGUMENT);
    o = bj_options(); o.max_nodes = 0; ISO_CHECK(bj_parse_with_options("0", 1, &o, &doc, NULL) == BJ_ARGUMENT);
    o.max_nodes = SIZE_MAX; ISO_CHECK(bj_parse_with_options("0", 1, &o, &doc, NULL) == BJ_ARGUMENT);
    o = bj_options(); o.max_alloc_bytes = 1; ISO_CHECK(bj_parse_with_options("0", 1, &o, &doc, NULL) == BJ_ARGUMENT);
    o.max_alloc_bytes = SIZE_MAX; ISO_CHECK(bj_parse_with_options("0", 1, &o, &doc, NULL) == BJ_ARGUMENT);
    o = bj_options(); o.allocate = NULL; ISO_CHECK(bj_parse_with_options("0", 1, &o, &doc, NULL) == BJ_ARGUMENT);
    o = bj_options(); o.release = NULL; ISO_CHECK(bj_parse_with_options("0", 1, &o, &doc, NULL) == BJ_ARGUMENT);
    ISO_CHECK(bj_parse("", BJ_MAX_BYTES + 1, &doc, &e) == BJ_BYTE_LIMIT && e.offset == 0);
    o = bj_options(); o.max_bytes = 1;
    ISO_CHECK(bj_parse_with_options("12", 2, &o, &doc, &e) == BJ_BYTE_LIMIT);
    ISO_CHECK(bj_parse_with_options("0", 1, &o, &doc, &e) == BJ_OK); bj_doc_free(&doc);
    o = bj_options(); o.max_depth = 0;
    ISO_CHECK(bj_parse_with_options("0", 1, &o, &doc, &e) == BJ_OK); bj_doc_free(&doc);
    ISO_CHECK(bj_parse_with_options("[]", 2, &o, &doc, &e) == BJ_DEPTH_LIMIT);
    for (i = 0; i < 129; i++) nested[i] = '[';
    nested[129] = '0'; for (i = 130; i < 259; i++) nested[i] = ']'; nested[259] = '\0';
    ISO_CHECK(bj_parse(nested, 259, &doc, &e) == BJ_DEPTH_LIMIT && e.offset <= 259);
    nested[128] = '0'; for (i = 129; i < 257; i++) nested[i] = ']'; nested[257] = '\0';
    ISO_CHECK(bj_parse(nested, 257, &doc, &e) == BJ_OK); bj_doc_free(&doc);
    o = bj_options(); o.max_nodes = 2;
    ISO_CHECK(bj_parse_with_options("[0,1]", 5, &o, &doc, &e) == BJ_NODE_LIMIT);
    ISO_CHECK(bj_parse_with_options("[0]", 3, &o, &doc, &e) == BJ_OK); bj_doc_free(&doc);
    o = bj_options(); o.max_alloc_bytes = 256;
    ISO_CHECK(bj_parse_with_options("{\"x\":[0]}", 9, &o, &doc, &e) == BJ_MEMORY_LIMIT);
    ISO_CHECK(!doc);
}
typedef struct { size_t calls, fail_at, live; } Fault;
static void *fault_alloc(size_t n, void *context) {
    Fault *f = (Fault *)context; void *p;
    if (f->calls++ == f->fail_at) return NULL;
    p = malloc(n); if (p) f->live++;
    return p;
}
static void fault_free(void *p, void *context) {
    Fault *f = (Fault *)context;
    ISO_CHECK(p != NULL && f->live > 0); f->live--; free(p);
}
static void allocation_failures(void) {
    const char *inputs[] = {"{\"x\":[\"a\\u20ac\",1.5,9223372036854775808,true,null,{\"z\":[]}]} retry", "{\"x\":[\"a\\u20ac\",1.5,9223372036854775808,true,null,{\"z\":[]}]}", "\"hello\""};
    size_t input, fail_at;
    for (input = 0; input < sizeof(inputs) / sizeof(inputs[0]); input++) {
        for (fail_at = 0; fail_at < 100; fail_at++) {
            Fault f = {0, fail_at, 0}; BJsonOptions o = bj_options(); BJsonDoc *doc = NULL;
            BJsonStatus result;
            o.allocate = fault_alloc; o.release = fault_free; o.context = &f;
            result = bj_parse_with_options(inputs[input], strlen(inputs[input]), &o, &doc, NULL);
            if (f.calls > fail_at) ISO_CHECK(result == BJ_ALLOC && !doc);
            else ISO_CHECK(result == ((input == 0) ? BJ_SYNTAX : BJ_OK));
            bj_doc_free(&doc);
            ISO_CHECK(f.live == 0);
            if (f.calls <= fail_at) break;
        }
        ISO_CHECK(fail_at < 100);
    }
}
static void checkpoint_json_inventory(void) {
    FILE *file = fopen("../../../specs/fixtures/neural-framework-v1/checkpoints.json", "rb");
    char *text; long length; BJsonDoc *doc = NULL;
    ISO_CHECK(file != NULL); if (!file) return;
    ISO_CHECK(fseek(file, 0, SEEK_END) == 0); length = ftell(file);
    ISO_CHECK(length > 0 && length < (long)BJ_MAX_BYTES);
    if (length <= 0 || length >= (long)BJ_MAX_BYTES) { fclose(file); return; }
    rewind(file); text = (char *)malloc((size_t)length);
    ISO_CHECK(text != NULL); if (!text) { fclose(file); return; }
    ISO_CHECK(fread(text, 1, (size_t)length, file) == (size_t)length); fclose(file);
    ISO_CHECK(bj_parse(text, (size_t)length, &doc, NULL) == BJ_OK); free(text);
    ISO_CHECK(bj_size(field(bj_root(doc), "cases")) == 55);
    /* Parsing is NOT schema acceptance: even rejected checkpoint shapes are
     * valid JSON values. Only a later native dense loader decides acceptance. */
    bj_doc_free(&doc);
}
static void wide_and_byte_envelopes(void) {
    char *text = (char *)malloc(BJ_MAX_BYTES + 1);
    BJsonDoc *doc = NULL; BJsonOptions o = bj_options(); BJsonError e;
    size_t i, n = BJ_MAX_NODES;
    ISO_CHECK(text != NULL); if (!text) return;
    memset(text, 'x', BJ_MAX_BYTES); text[0] = '"'; text[BJ_MAX_BYTES - 1] = '"';
    ISO_CHECK(bj_parse(text, BJ_MAX_BYTES, &doc, &e) == BJ_OK); bj_doc_free(&doc);
    o.max_alloc_bytes = 1024;
    ISO_CHECK(bj_parse_with_options(text, BJ_MAX_BYTES, &o, &doc, &e) == BJ_MEMORY_LIMIT);
    text[0] = '[';
    for (i = 0; i < n; i++) { text[1 + 2 * i] = '0'; text[2 + 2 * i] = ','; }
    text[2 * n] = ']';
    ISO_CHECK(bj_parse(text, 2 * n + 1, &doc, &e) == BJ_NODE_LIMIT);
    /* Root plus n-1 scalars exactly fills the default node budget. */
    text[2 * n - 2] = ']';
    ISO_CHECK(bj_parse(text, 2 * n - 1, &doc, &e) == BJ_OK);
    ISO_CHECK(bj_size(bj_root(doc)) == n - 1); bj_doc_free(&doc); free(text);
}
static void deterministic_byte_probes(void) {
    uint32_t state = 19; size_t i, j;
    for (i = 0; i < 500; i++) {
        char bytes[32]; BJsonDoc *doc = NULL; BJsonError e; BJsonStatus status;
        size_t length = i % sizeof(bytes);
        for (j = 0; j < length; j++) { state = state * UINT32_C(1664525) + UINT32_C(1013904223); bytes[j] = (char)(state >> 24); }
        status = bj_parse(bytes, length, &doc, &e);
        ISO_CHECK(e.offset <= length);
        ISO_CHECK(status == BJ_OK ? bj_root(doc) != NULL : doc == NULL);
        bj_doc_free(&doc);
    }
}
int main(void) {
    values_and_accessors(); grammar_and_unicode(); numbers_and_locale();
    limits(); allocation_failures(); checkpoint_json_inventory();
    wide_and_byte_envelopes(); deterministic_byte_probes();
    return ISO_TEST_RESULT();
}
