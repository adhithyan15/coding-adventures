/*
 * jvm_class_file.h — a small JVM class-file parser + builder, pure ISO C17.
 * ========================================================================
 *
 * A faithful port of the Rust `jvm-class-file` crate. Two jobs:
 *
 *   1. parse a deliberately small, boring subset of the JVM class-file format
 *   2. build a minimal one-method class file for tests and bootstrap tooling
 *
 * The parser is intentionally CONSERVATIVE: when the bytes ask for something it
 * does not understand — or an attacker-controlled length runs past the end of
 * the buffer — it returns JVM_ERR_FORMAT (with a message) instead of guessing.
 * Every read goes through a bounds-checked cursor, so malformed input can never
 * read out of bounds (Rust panics safely on OOB slice indexing; C must not).
 *
 * Owned strings in parsed structures are NUL-terminated. (The JVM's "modified
 * UTF-8" is stored verbatim; the small subset used by this crate is ASCII and
 * NUL-free.)
 *
 * Pure ISO C17: no <math.h>, no compiler extensions.
 */
#ifndef JVM_CLASS_FILE_H
#define JVM_CLASS_FILE_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t, uint16_t, int32_t, int64_t */

#ifdef __cplusplus
extern "C" {
#endif

/* ── Access flags & opcode constants ────────────────────────────────────────*/
#define JVM_ACC_PUBLIC ((uint16_t)0x0001)
#define JVM_ACC_STATIC ((uint16_t)0x0008)
#define JVM_ACC_SUPER ((uint16_t)0x0020)

#define JVM_OP_ACONST_NULL ((uint8_t)0x01)
#define JVM_OP_ALOAD ((uint8_t)0x19)
#define JVM_OP_ASTORE ((uint8_t)0x3A)
#define JVM_OP_DUP ((uint8_t)0x59)
#define JVM_OP_AALOAD ((uint8_t)0x32)
#define JVM_OP_AASTORE ((uint8_t)0x53)
#define JVM_OP_ANEWARRAY ((uint8_t)0xBD)
#define JVM_OP_IFNULL ((uint8_t)0xC6)
#define JVM_OP_IFNONNULL ((uint8_t)0xC7)
#define JVM_OP_GOTO ((uint8_t)0xA7)
#define JVM_OP_SWAP ((uint8_t)0x5F)

/* ── Status ─────────────────────────────────────────────────────────────────*/
typedef enum { JVM_OK = 0, JVM_ERR_FORMAT, JVM_ERR_OUT_OF_MEMORY } JvmStatus;

/* ── Constant-pool entry kinds ──────────────────────────────────────────────*/
typedef enum {
    JVM_CP_UTF8 = 1,
    JVM_CP_INTEGER = 3,
    JVM_CP_LONG = 5,
    JVM_CP_DOUBLE = 6,
    JVM_CP_CLASS = 7,
    JVM_CP_STRING = 8,
    JVM_CP_FIELDREF = 9,
    JVM_CP_METHODREF = 10,
    JVM_CP_NAME_AND_TYPE = 12
} JvmConstantKind;

/* ── Resolved constant (the "loadable" projection) ──────────────────────────*/
typedef enum {
    JVM_RC_UTF8,
    JVM_RC_INTEGER,
    JVM_RC_LONG,
    JVM_RC_DOUBLE,
    JVM_RC_STRING
} JvmResolvedKind;

typedef struct {
    JvmResolvedKind kind;
    const char *text; /* borrowed: JVM_RC_UTF8 / JVM_RC_STRING */
    int32_t integer;
    int64_t long_v;
    double double_v;
} JvmResolvedConstant;

/* A member reference (all strings borrowed from the class file). */
typedef struct {
    const char *class_name;
    const char *name;
    const char *descriptor;
} JvmMemberRef;

/* ── Opaque handles ─────────────────────────────────────────────────────────*/
typedef struct JvmClassFile JvmClassFile;
typedef struct JvmMethod JvmMethod;

/* A view of a parsed Code attribute (all pointers borrowed from the method). */
typedef struct {
    const uint8_t *code;
    size_t code_len;
    uint16_t max_stack;
    uint16_t max_locals;
    size_t nested_attribute_count;
} JvmCodeView;

/* ── Parsing ────────────────────────────────────────────────────────────────*/

/* Parse a class file. On success returns JVM_OK and stores an owned handle in
 * *out (free with jvm_class_free). On failure returns JVM_ERR_* and, if errbuf
 * is non-NULL, writes a NUL-terminated diagnostic (truncated to errlen). */
JvmStatus jvm_parse_class_file(const uint8_t *data, size_t len,
                               JvmClassFile **out, char *errbuf, size_t errlen);
void jvm_class_free(JvmClassFile *cf);

/* ── Class-file accessors ───────────────────────────────────────────────────*/
void jvm_class_version(const JvmClassFile *cf, uint16_t *major, uint16_t *minor);
uint16_t jvm_class_access_flags(const JvmClassFile *cf);
const char *jvm_class_this_name(const JvmClassFile *cf);
const char *jvm_class_super_name(const JvmClassFile *cf);

/* Constant-pool size (includes index 0 and reserved wide slots). */
size_t jvm_class_constant_pool_len(const JvmClassFile *cf);
/* Fills *kind for a present entry at index; returns 1 if present, 0 if the slot
 * is empty (index 0, a wide slot, or out of range). */
int jvm_class_constant_kind(const JvmClassFile *cf, uint16_t index,
                            JvmConstantKind *kind);

/* Resolvers (mirror the Rust JvmClassFile methods). Return JVM_ERR_FORMAT and
 * fill errbuf on a type/range mismatch. */
JvmStatus jvm_get_utf8(const JvmClassFile *cf, uint16_t index,
                       const char **out, char *errbuf, size_t errlen);
JvmStatus jvm_resolve_constant(const JvmClassFile *cf, uint16_t index,
                               JvmResolvedConstant *out, char *errbuf,
                               size_t errlen);
JvmStatus jvm_resolve_fieldref(const JvmClassFile *cf, uint16_t index,
                               JvmMemberRef *out, char *errbuf, size_t errlen);
JvmStatus jvm_resolve_methodref(const JvmClassFile *cf, uint16_t index,
                                JvmMemberRef *out, char *errbuf, size_t errlen);

/* Methods. find returns NULL if none matches (descriptor NULL = any). */
size_t jvm_class_method_count(const JvmClassFile *cf);
const JvmMethod *jvm_class_method(const JvmClassFile *cf, size_t i);
const JvmMethod *jvm_class_find_method(const JvmClassFile *cf, const char *name,
                                       const char *descriptor);
uint16_t jvm_method_access_flags(const JvmMethod *m);
const char *jvm_method_name(const JvmMethod *m);
const char *jvm_method_descriptor(const JvmMethod *m);
/* Fills *out with the first Code attribute; returns 1 if present, else 0. */
int jvm_method_code(const JvmMethod *m, JvmCodeView *out);
/* Name of the i-th nested attribute of the method's Code attribute (or NULL). */
const char *jvm_method_code_nested_name(const JvmMethod *m, size_t i);

/* ── Minimal builder ────────────────────────────────────────────────────────*/
typedef enum { JVM_MIN_INTEGER, JVM_MIN_STRING } JvmMinimalConstantKind;
typedef struct {
    JvmMinimalConstantKind kind;
    int32_t integer;   /* JVM_MIN_INTEGER */
    const char *text;  /* JVM_MIN_STRING (NUL-terminated) */
} JvmMinimalConstant;

typedef struct {
    const char *class_name;
    const char *method_name;
    const char *descriptor;
    const uint8_t *code;
    size_t code_len;
    uint16_t max_stack;
    uint16_t max_locals;
    const JvmMinimalConstant *constants;
    size_t constant_count;
    uint16_t major_version;
    uint16_t minor_version;
    uint16_t class_access_flags;
    uint16_t method_access_flags;
    const char *super_class_name; /* NULL or "" -> java/lang/Object */
} JvmBuildParams;

/* Sensible defaults matching the Rust BuildMinimalClassFileParams::default(). */
JvmBuildParams jvm_build_params_default(void);

/* Build the class bytes. On success returns JVM_OK and stores a malloc'd buffer
 * in *out_bytes (free with free) of length *out_len. */
JvmStatus jvm_build_minimal_class_file(const JvmBuildParams *params,
                                       uint8_t **out_bytes, size_t *out_len,
                                       char *errbuf, size_t errlen);

#ifdef __cplusplus
}
#endif

#endif /* JVM_CLASS_FILE_H */
