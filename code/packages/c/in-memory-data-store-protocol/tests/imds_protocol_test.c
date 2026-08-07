/*
 * imds_protocol_test.c — tests for the in-memory data store protocol IR.
 * ===========================================================================
 *
 * Mirrors the behaviour of the Rust crate and pins down the C-specific ownership
 * contract: everything a constructor allocates is independent of its inputs, and
 * every value (including a nested array tree) frees cleanly. Runs under
 * ASan+UBSan so a double-free, leak, or out-of-bounds read fails the build.
 */
#include "imds_protocol/imds_protocol.h"
#include "iso_test.h"

#include <stdlib.h>
#include <string.h>

/* Convenience: an imds_arg over a C string literal's bytes (excluding the NUL). */
static imds_arg arg_str(const char *s) {
    imds_arg a;
    a.bytes = (unsigned char *)s;
    a.len = strlen(s);
    return a;
}

/* ------------------------------------------------------------------------- *
 * CommandFrame
 * ------------------------------------------------------------------------- */

static void test_command_frame_new(void) {
    imds_arg args[2];
    imds_command_frame f;
    args[0] = arg_str("key");
    args[1] = arg_str("value");

    ISO_CHECK_EQ_INT(imds_command_frame_new("SET", args, 2, &f), IMDS_OK);
    ISO_CHECK_STR_EQ(f.command, "SET");
    ISO_CHECK_EQ_UINT(f.nargs, 2);
    ISO_CHECK_EQ_UINT(f.args[0].len, 3);
    ISO_CHECK_MEM_EQ(f.args[0].bytes, "key", 3);
    ISO_CHECK_MEM_EQ(f.args[1].bytes, "value", 5);
    imds_command_frame_free(&f);

    /* new with zero args → NULL args, nargs 0, still a valid command. */
    ISO_CHECK_EQ_INT(imds_command_frame_new("PING", NULL, 0, &f), IMDS_OK);
    ISO_CHECK_STR_EQ(f.command, "PING");
    ISO_CHECK_EQ_UINT(f.nargs, 0);
    ISO_CHECK(f.args == NULL);
    imds_command_frame_free(&f);
}

static void test_command_frame_new_copies_inputs(void) {
    char cmd[8];
    unsigned char blob[4];
    imds_arg a;
    imds_command_frame f;
    memcpy(cmd, "GET", 4);
    blob[0] = 'a'; blob[1] = 'b'; blob[2] = 'c'; blob[3] = 'd';
    a.bytes = blob;
    a.len = 4;

    ISO_CHECK_EQ_INT(imds_command_frame_new(cmd, &a, 1, &f), IMDS_OK);
    /* Mutate the sources — the frame must be independent. */
    memcpy(cmd, "XXX", 3);
    blob[0] = 'z';
    ISO_CHECK_STR_EQ(f.command, "GET");
    ISO_CHECK_MEM_EQ(f.args[0].bytes, "abcd", 4);
    imds_command_frame_free(&f);
}

static void test_from_parts_empty_is_none(void) {
    imds_command_frame f;
    /* split_first() on an empty list → None → IMDS_NONE, *out untouched. */
    ISO_CHECK_EQ_INT(imds_command_frame_from_parts(NULL, 0, &f), IMDS_NONE);
}

static void test_from_parts_uppercases_command(void) {
    imds_arg parts[3];
    imds_command_frame f;
    parts[0] = arg_str("set");   /* lowercased on the wire */
    parts[1] = arg_str("Key");
    parts[2] = arg_str("Val");

    ISO_CHECK_EQ_INT(imds_command_frame_from_parts(parts, 3, &f), IMDS_OK);
    ISO_CHECK_STR_EQ(f.command, "SET"); /* command uppercased… */
    ISO_CHECK_EQ_UINT(f.nargs, 2);
    ISO_CHECK_MEM_EQ(f.args[0].bytes, "Key", 3); /* …args left verbatim */
    ISO_CHECK_MEM_EQ(f.args[1].bytes, "Val", 3);
    imds_command_frame_free(&f);
}

static void test_from_parts_single_part_no_args(void) {
    imds_arg parts[1];
    imds_command_frame f;
    parts[0] = arg_str("ping");
    ISO_CHECK_EQ_INT(imds_command_frame_from_parts(parts, 1, &f), IMDS_OK);
    ISO_CHECK_STR_EQ(f.command, "PING");
    ISO_CHECK_EQ_UINT(f.nargs, 0);
    ISO_CHECK(f.args == NULL);
    imds_command_frame_free(&f);
}

/*
 * The Rust `ascii_upper` is `byte.to_ascii_uppercase() as char` collected into a
 * String: only 'a'..='z' shift; a byte >= 0x80 becomes U+0080..U+00FF and is
 * UTF-8-encoded to TWO bytes. Pin exactly that so the port is byte-faithful.
 */
static void test_from_parts_ascii_upper_semantics(void) {
    unsigned char raw[4];
    imds_arg parts[1];
    imds_command_frame f;
    raw[0] = 'a';   /* -> 'A'  */
    raw[1] = 'Z';   /* -> 'Z'  (already upper) */
    raw[2] = '1';   /* -> '1'  (non-alpha untouched) */
    raw[3] = 0xE9;  /* 'é' in Latin-1 -> U+00E9 -> UTF-8 0xC3 0xA9 */
    parts[0].bytes = raw;
    parts[0].len = 4;

    ISO_CHECK_EQ_INT(imds_command_frame_from_parts(parts, 1, &f), IMDS_OK);
    /* "AZ1" + 0xC3 0xA9 = 5 bytes + NUL. strlen stops at no interior NUL here. */
    ISO_CHECK_EQ_UINT(strlen(f.command), 5);
    ISO_CHECK_MEM_EQ(f.command, "AZ1\xC3\xA9", 5);
    imds_command_frame_free(&f);
}

static void test_command_frame_free_safe_on_zeroed(void) {
    imds_command_frame f;
    memset(&f, 0, sizeof(f));
    imds_command_frame_free(&f); /* no-op, must not crash */
    imds_command_frame_free(NULL);
    ISO_CHECK(1);
}

static void test_command_frame_invalid_params(void) {
    imds_command_frame f;
    imds_arg parts[1];
    parts[0] = arg_str("x");
    ISO_CHECK_EQ_INT(imds_command_frame_new("X", NULL, 0, NULL), IMDS_ERR_INVALID);
    ISO_CHECK_EQ_INT(imds_command_frame_new(NULL, NULL, 0, &f), IMDS_ERR_INVALID);
    /* args NULL but nargs>0 is a caller bug → invalid, not a NULL-deref. */
    ISO_CHECK_EQ_INT(imds_command_frame_new("X", NULL, 3, &f), IMDS_ERR_INVALID);
    ISO_CHECK_EQ_INT(imds_command_frame_from_parts(parts, 1, NULL), IMDS_ERR_INVALID);
    /* nparts>0 but parts NULL is a caller bug → invalid, not a crash. */
    ISO_CHECK_EQ_INT(imds_command_frame_from_parts(NULL, 2, &f), IMDS_ERR_INVALID);
}

/* ------------------------------------------------------------------------- *
 * EngineResponse
 * ------------------------------------------------------------------------- */

static void test_response_scalars(void) {
    imds_engine_response r;

    ISO_CHECK_EQ_INT(imds_resp_simple_string("hello", &r), IMDS_OK);
    ISO_CHECK_EQ_INT(r.kind, IMDS_RESP_SIMPLE_STRING);
    ISO_CHECK_STR_EQ(r.as.str, "hello");
    imds_engine_response_free(&r);

    ISO_CHECK_EQ_INT(imds_resp_error("boom", &r), IMDS_OK);
    ISO_CHECK_EQ_INT(r.kind, IMDS_RESP_ERROR);
    ISO_CHECK_STR_EQ(r.as.str, "boom");
    imds_engine_response_free(&r);

    ISO_CHECK_EQ_INT(imds_resp_integer(-42, &r), IMDS_OK);
    ISO_CHECK_EQ_INT(r.kind, IMDS_RESP_INTEGER);
    ISO_CHECK(r.as.integer == -42);
    imds_engine_response_free(&r); /* nothing owned; must stay safe */
}

static void test_response_bulk(void) {
    imds_engine_response r;
    unsigned char payload[3];
    payload[0] = 0x00; payload[1] = 0xFF; payload[2] = 0x41; /* embedded NUL */

    ISO_CHECK_EQ_INT(imds_resp_bulk_string(payload, 3, &r), IMDS_OK);
    ISO_CHECK_EQ_INT(r.kind, IMDS_RESP_BULK_STRING);
    ISO_CHECK_EQ_INT(r.as.bulk.is_null, 0);
    ISO_CHECK_EQ_UINT(r.as.bulk.len, 3);
    ISO_CHECK_MEM_EQ(r.as.bulk.bytes, payload, 3);
    imds_engine_response_free(&r);

    /* Zero-length bulk is a real (non-null) empty blob. */
    ISO_CHECK_EQ_INT(imds_resp_bulk_string(NULL, 0, &r), IMDS_OK);
    ISO_CHECK_EQ_INT(r.as.bulk.is_null, 0);
    ISO_CHECK_EQ_UINT(r.as.bulk.len, 0);
    ISO_CHECK(r.as.bulk.bytes != NULL);
    imds_engine_response_free(&r);

    ISO_CHECK_EQ_INT(imds_resp_bulk_null(&r), IMDS_OK);
    ISO_CHECK_EQ_INT(r.kind, IMDS_RESP_BULK_STRING);
    ISO_CHECK_EQ_INT(r.as.bulk.is_null, 1);
    ISO_CHECK(r.as.bulk.bytes == NULL);
    imds_engine_response_free(&r);
}

static void test_response_convenience(void) {
    imds_engine_response r;

    ISO_CHECK_EQ_INT(imds_resp_ok(&r), IMDS_OK);
    ISO_CHECK_EQ_INT(r.kind, IMDS_RESP_SIMPLE_STRING);
    ISO_CHECK_STR_EQ(r.as.str, "OK");
    imds_engine_response_free(&r);

    /* null() is a BULK null, not an ARRAY null (matches the Rust). */
    ISO_CHECK_EQ_INT(imds_resp_null(&r), IMDS_OK);
    ISO_CHECK_EQ_INT(r.kind, IMDS_RESP_BULK_STRING);
    ISO_CHECK_EQ_INT(r.as.bulk.is_null, 1);
    imds_engine_response_free(&r);

    ISO_CHECK_EQ_INT(imds_resp_zero(&r), IMDS_OK);
    ISO_CHECK(r.as.integer == 0);
    imds_engine_response_free(&r);

    ISO_CHECK_EQ_INT(imds_resp_one(&r), IMDS_OK);
    ISO_CHECK(r.as.integer == 1);
    imds_engine_response_free(&r);
}

static void test_response_array_null_and_empty(void) {
    imds_engine_response r;

    ISO_CHECK_EQ_INT(imds_resp_array_null(&r), IMDS_OK);
    ISO_CHECK_EQ_INT(r.kind, IMDS_RESP_ARRAY);
    ISO_CHECK_EQ_INT(r.as.array.is_null, 1);
    ISO_CHECK(r.as.array.items == NULL);
    imds_engine_response_free(&r);

    /* Empty (non-null) array: items NULL, n 0, is_null 0. */
    ISO_CHECK_EQ_INT(imds_resp_array(NULL, 0, &r), IMDS_OK);
    ISO_CHECK_EQ_INT(r.as.array.is_null, 0);
    ISO_CHECK_EQ_UINT(r.as.array.n, 0);
    imds_engine_response_free(&r);
}

/*
 * A nested tree: Array[ Integer(7), SimpleString("id"), Array[ BulkString("x") ] ].
 * Freeing the outer response must recursively free every child — verified under
 * ASan (a missed child leaks; a double-freed child aborts).
 */
static void test_response_nested_array_tree(void) {
    imds_engine_response *outer_items;
    imds_engine_response *inner_items;
    imds_engine_response root;

    inner_items = (imds_engine_response *)malloc(sizeof(*inner_items));
    ISO_CHECK(inner_items != NULL);
    ISO_CHECK_EQ_INT(imds_resp_bulk_string((const unsigned char *)"x", 1,
                                           &inner_items[0]), IMDS_OK);

    outer_items = (imds_engine_response *)malloc(3 * sizeof(*outer_items));
    ISO_CHECK(outer_items != NULL);
    ISO_CHECK_EQ_INT(imds_resp_integer(7, &outer_items[0]), IMDS_OK);
    ISO_CHECK_EQ_INT(imds_resp_simple_string("id", &outer_items[1]), IMDS_OK);
    ISO_CHECK_EQ_INT(imds_resp_array(inner_items, 1, &outer_items[2]), IMDS_OK);

    ISO_CHECK_EQ_INT(imds_resp_array(outer_items, 3, &root), IMDS_OK);
    ISO_CHECK_EQ_UINT(root.as.array.n, 3);
    ISO_CHECK_EQ_INT(root.as.array.items[2].kind, IMDS_RESP_ARRAY);
    ISO_CHECK_MEM_EQ(root.as.array.items[2].as.array.items[0].as.bulk.bytes, "x", 1);

    imds_engine_response_free(&root); /* frees the whole tree */
}

static void test_response_free_safe_on_zeroed(void) {
    imds_engine_response r;
    memset(&r, 0, sizeof(r)); /* kind 0 == SIMPLE_STRING, str NULL */
    imds_engine_response_free(&r);
    imds_engine_response_free(NULL);
    ISO_CHECK(1);
}

static void test_response_invalid_params(void) {
    imds_engine_response r;
    ISO_CHECK_EQ_INT(imds_resp_simple_string("x", NULL), IMDS_ERR_INVALID);
    ISO_CHECK_EQ_INT(imds_resp_simple_string(NULL, &r), IMDS_ERR_INVALID);
    ISO_CHECK_EQ_INT(imds_resp_error(NULL, &r), IMDS_ERR_INVALID);
    ISO_CHECK_EQ_INT(imds_resp_integer(0, NULL), IMDS_ERR_INVALID);
    ISO_CHECK_EQ_INT(imds_resp_bulk_null(NULL), IMDS_ERR_INVALID);
    ISO_CHECK_EQ_INT(imds_resp_array_null(NULL), IMDS_ERR_INVALID);
    /* bytes NULL but len>0 is a caller bug. */
    ISO_CHECK_EQ_INT(imds_resp_bulk_string(NULL, 4, &r), IMDS_ERR_INVALID);
    /* items NULL but n>0 is a caller bug. */
    ISO_CHECK_EQ_INT(imds_resp_array(NULL, 2, &r), IMDS_ERR_INVALID);
}

int main(void) {
    test_command_frame_new();
    test_command_frame_new_copies_inputs();
    test_from_parts_empty_is_none();
    test_from_parts_uppercases_command();
    test_from_parts_single_part_no_args();
    test_from_parts_ascii_upper_semantics();
    test_command_frame_free_safe_on_zeroed();
    test_command_frame_invalid_params();
    test_response_scalars();
    test_response_bulk();
    test_response_convenience();
    test_response_array_null_and_empty();
    test_response_nested_array_tree();
    test_response_free_safe_on_zeroed();
    test_response_invalid_params();
    return ISO_TEST_RESULT();
}
