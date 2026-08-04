/*
 * Tests for jvm-class-file, using the header-only iso_test.h harness (pure ISO).
 * Vectors mirror the Rust crate's unit tests.
 */
#include "iso_test.h"

#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include "jvm_class_file.h"

/* A tiny growable byte buffer for hand-assembling synthetic class files. */
typedef struct {
    uint8_t data[512];
    size_t len;
} Buf;
static void bu1(Buf *b, uint8_t v) { b->data[b->len++] = v; }
static void bu2(Buf *b, uint16_t v) {
    bu1(b, (uint8_t)(v >> 8));
    bu1(b, (uint8_t)(v & 0xFF));
}
static void bu4(Buf *b, uint32_t v) {
    bu1(b, (uint8_t)(v >> 24));
    bu1(b, (uint8_t)((v >> 16) & 0xFF));
    bu1(b, (uint8_t)((v >> 8) & 0xFF));
    bu1(b, (uint8_t)(v & 0xFF));
}
static void butf8(Buf *b, const char *s) {
    size_t n = strlen(s), i;
    bu1(b, 1);
    bu2(b, (uint16_t)n);
    for (i = 0; i < n; i++) {
        bu1(b, (uint8_t)s[i]);
    }
}
static void bbytes(Buf *b, const uint8_t *p, size_t n) {
    size_t i;
    for (i = 0; i < n; i++) {
        bu1(b, p[i]);
    }
}

int main(void) {
    char err[128];

    /* ── builds and parses a minimal class file ────────────────────────────*/
    {
        JvmBuildParams params = jvm_build_params_default();
        JvmMinimalConstant consts[2];
        static const uint8_t code[] = {0xB1};
        uint8_t *bytes = NULL;
        size_t blen = 0;
        JvmClassFile *cf = NULL;
        uint16_t major = 0, minor = 0;
        const JvmMethod *m;
        JvmCodeView cv;
        size_t i;
        int has_int7 = 0, has_hello = 0;

        consts[0].kind = JVM_MIN_INTEGER;
        consts[0].integer = 7;
        consts[1].kind = JVM_MIN_STRING;
        consts[1].text = "hello";
        params.class_name = "demo/Example";
        params.method_name = "main";
        params.descriptor = "([Ljava/lang/String;)V";
        params.code = code;
        params.code_len = sizeof code;
        params.max_stack = 0;
        params.max_locals = 1;
        params.constants = consts;
        params.constant_count = 2;

        ISO_CHECK_EQ_INT(
            jvm_build_minimal_class_file(&params, &bytes, &blen, err, sizeof err),
            JVM_OK);
        ISO_CHECK(bytes != NULL && blen > 0);

        ISO_CHECK_EQ_INT(
            jvm_parse_class_file(bytes, blen, &cf, err, sizeof err), JVM_OK);
        ISO_CHECK_STR_EQ(jvm_class_this_name(cf), "demo/Example");
        ISO_CHECK_STR_EQ(jvm_class_super_name(cf), "java/lang/Object");
        jvm_class_version(cf, &major, &minor);
        ISO_CHECK_EQ_INT(major, 61);

        m = jvm_class_find_method(cf, "main", "([Ljava/lang/String;)V");
        ISO_CHECK(m != NULL);
        ISO_CHECK(jvm_method_code(m, &cv));
        ISO_CHECK_EQ_UINT(cv.code_len, 1u);
        ISO_CHECK_EQ_UINT(cv.code[0], 0xB1u);

        for (i = 0; i < jvm_class_constant_pool_len(cf); i++) {
            JvmConstantKind kind;
            JvmResolvedConstant rc;
            if (!jvm_class_constant_kind(cf, (uint16_t)i, &kind)) {
                continue;
            }
            if (kind != JVM_CP_INTEGER && kind != JVM_CP_STRING &&
                kind != JVM_CP_UTF8) {
                continue;
            }
            if (jvm_resolve_constant(cf, (uint16_t)i, &rc, err, sizeof err) !=
                JVM_OK) {
                continue;
            }
            if (rc.kind == JVM_RC_INTEGER && rc.integer == 7) {
                has_int7 = 1;
            }
            if (rc.kind == JVM_RC_STRING && strcmp(rc.text, "hello") == 0) {
                has_hello = 1;
            }
        }
        ISO_CHECK(has_int7);
        ISO_CHECK(has_hello);

        jvm_class_free(cf);
        free(bytes);
    }

    /* ── rejects invalid magic ─────────────────────────────────────────────*/
    {
        static const uint8_t bad[] = {0, 1, 2, 3};
        JvmClassFile *cf = NULL;
        JvmStatus st = jvm_parse_class_file(bad, sizeof bad, &cf, err, sizeof err);
        ISO_CHECK_EQ_INT(st, JVM_ERR_FORMAT);
        ISO_CHECK(strstr(err, "Invalid class-file magic") != NULL);
        ISO_CHECK(cf == NULL);
    }

    /* ── resolves fieldrefs and methodrefs ─────────────────────────────────*/
    {
        Buf pool, code_body, code_attr, method, cls;
        JvmClassFile *cf = NULL;
        uint16_t fieldref_index = 0, methodref_index = 0;
        JvmMemberRef fr, mr;
        size_t i;

        pool.len = 0;
        /* 1 Utf8 "demo/Refs"  2 Class->1  3 Utf8 "java/lang/Object" 4 Class->3
         * 5 Utf8 "VALUE" 6 Utf8 "I" 7 NameAndType(5,6) 8 Fieldref(2,7)
         * 9 Utf8 "helper" 10 Utf8 "()I" 11 NameAndType(9,10) 12 Methodref(2,11)
         * 13 Utf8 "main" 14 Utf8 "()V" 15 Utf8 "Code" */
        butf8(&pool, "demo/Refs");
        bu1(&pool, 7);
        bu2(&pool, 1);
        butf8(&pool, "java/lang/Object");
        bu1(&pool, 7);
        bu2(&pool, 3);
        butf8(&pool, "VALUE");
        butf8(&pool, "I");
        bu1(&pool, 12);
        bu2(&pool, 5);
        bu2(&pool, 6);
        bu1(&pool, 9);
        bu2(&pool, 2);
        bu2(&pool, 7);
        butf8(&pool, "helper");
        butf8(&pool, "()I");
        bu1(&pool, 12);
        bu2(&pool, 9);
        bu2(&pool, 10);
        bu1(&pool, 10);
        bu2(&pool, 2);
        bu2(&pool, 11);
        butf8(&pool, "main");
        butf8(&pool, "()V");
        butf8(&pool, "Code");

        code_body.len = 0;
        bu2(&code_body, 0);
        bu2(&code_body, 1);
        bu4(&code_body, 1);
        bu1(&code_body, 0xB1);
        bu2(&code_body, 0);
        bu2(&code_body, 0);
        code_attr.len = 0;
        bu2(&code_attr, 15);
        bu4(&code_attr, (uint32_t)code_body.len);
        bbytes(&code_attr, code_body.data, code_body.len);
        method.len = 0;
        bu2(&method, JVM_ACC_PUBLIC | JVM_ACC_STATIC);
        bu2(&method, 13);
        bu2(&method, 14);
        bu2(&method, 1);
        bbytes(&method, code_attr.data, code_attr.len);

        cls.len = 0;
        bu4(&cls, 0xCAFEBABE);
        bu2(&cls, 0);
        bu2(&cls, 61);
        bu2(&cls, 16);
        bbytes(&cls, pool.data, pool.len);
        bu2(&cls, JVM_ACC_PUBLIC | JVM_ACC_SUPER);
        bu2(&cls, 2);
        bu2(&cls, 4);
        bu2(&cls, 0);
        bu2(&cls, 0);
        bu2(&cls, 1);
        bbytes(&cls, method.data, method.len);
        bu2(&cls, 0);

        ISO_CHECK_EQ_INT(
            jvm_parse_class_file(cls.data, cls.len, &cf, err, sizeof err),
            JVM_OK);
        for (i = 0; i < jvm_class_constant_pool_len(cf); i++) {
            JvmConstantKind kind;
            if (!jvm_class_constant_kind(cf, (uint16_t)i, &kind)) {
                continue;
            }
            if (kind == JVM_CP_FIELDREF) {
                fieldref_index = (uint16_t)i;
            }
            if (kind == JVM_CP_METHODREF) {
                methodref_index = (uint16_t)i;
            }
        }
        ISO_CHECK(fieldref_index != 0 && methodref_index != 0);
        ISO_CHECK_EQ_INT(
            jvm_resolve_fieldref(cf, fieldref_index, &fr, err, sizeof err),
            JVM_OK);
        ISO_CHECK_STR_EQ(fr.class_name, "demo/Refs");
        ISO_CHECK_STR_EQ(fr.name, "VALUE");
        ISO_CHECK_STR_EQ(fr.descriptor, "I");
        ISO_CHECK_EQ_INT(
            jvm_resolve_methodref(cf, methodref_index, &mr, err, sizeof err),
            JVM_OK);
        ISO_CHECK_STR_EQ(mr.class_name, "demo/Refs");
        ISO_CHECK_STR_EQ(mr.name, "helper");
        ISO_CHECK_STR_EQ(mr.descriptor, "()I");
        jvm_class_free(cf);
    }

    /* ── a Code attribute may hold a nested (Raw) attribute named "Code" ────*/
    {
        Buf pool, nested_body, outer_body, code_attr, method, cls;
        JvmClassFile *cf = NULL;
        const JvmMethod *m;
        JvmCodeView cv;

        pool.len = 0;
        /* 1 Utf8 "demo/Nested" 2 Class->1 3 Utf8 "java/lang/Object" 4 Class->3
         * 5 Utf8 "main" 6 Utf8 "()V" 7 Utf8 "Code" */
        butf8(&pool, "demo/Nested");
        bu1(&pool, 7);
        bu2(&pool, 1);
        butf8(&pool, "java/lang/Object");
        bu1(&pool, 7);
        bu2(&pool, 3);
        butf8(&pool, "main");
        butf8(&pool, "()V");
        butf8(&pool, "Code");

        nested_body.len = 0;
        bu2(&nested_body, 0);
        bu2(&nested_body, 0);
        bu4(&nested_body, 1);
        bu1(&nested_body, 0xB1);
        bu2(&nested_body, 0);
        bu2(&nested_body, 0);

        outer_body.len = 0;
        bu2(&outer_body, 0);
        bu2(&outer_body, 1);
        bu4(&outer_body, 1);
        bu1(&outer_body, 0xB1);
        bu2(&outer_body, 0);
        bu2(&outer_body, 1); /* 1 nested attribute */
        bu2(&outer_body, 7); /* "Code" */
        bu4(&outer_body, (uint32_t)nested_body.len);
        bbytes(&outer_body, nested_body.data, nested_body.len);

        code_attr.len = 0;
        bu2(&code_attr, 7);
        bu4(&code_attr, (uint32_t)outer_body.len);
        bbytes(&code_attr, outer_body.data, outer_body.len);

        method.len = 0;
        bu2(&method, JVM_ACC_PUBLIC | JVM_ACC_STATIC);
        bu2(&method, 5);
        bu2(&method, 6);
        bu2(&method, 1);
        bbytes(&method, code_attr.data, code_attr.len);

        cls.len = 0;
        bu4(&cls, 0xCAFEBABE);
        bu2(&cls, 0);
        bu2(&cls, 61);
        bu2(&cls, 8);
        bbytes(&cls, pool.data, pool.len);
        bu2(&cls, JVM_ACC_PUBLIC | JVM_ACC_SUPER);
        bu2(&cls, 2);
        bu2(&cls, 4);
        bu2(&cls, 0);
        bu2(&cls, 0);
        bu2(&cls, 1);
        bbytes(&cls, method.data, method.len);
        bu2(&cls, 0);

        ISO_CHECK_EQ_INT(
            jvm_parse_class_file(cls.data, cls.len, &cf, err, sizeof err),
            JVM_OK);
        m = jvm_class_find_method(cf, "main", "()V");
        ISO_CHECK(m != NULL);
        ISO_CHECK(jvm_method_code(m, &cv));
        ISO_CHECK_EQ_UINT(cv.nested_attribute_count, 1u);
        ISO_CHECK_STR_EQ(jvm_method_code_nested_name(m, 0), "Code");
        jvm_class_free(cf);
    }

    /* ── a truncated file is rejected, not read out of bounds ──────────────*/
    {
        static const uint8_t magic_only[] = {0xCA, 0xFE, 0xBA, 0xBE};
        JvmClassFile *cf = NULL;
        JvmStatus st = jvm_parse_class_file(magic_only, sizeof magic_only, &cf,
                                            err, sizeof err);
        ISO_CHECK_EQ_INT(st, JVM_ERR_FORMAT);
        ISO_CHECK(cf == NULL);
    }

    return ISO_TEST_RESULT();
}
