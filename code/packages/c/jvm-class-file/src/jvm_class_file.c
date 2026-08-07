/*
 * jvm_class_file.c — a small JVM class-file parser + builder, pure ISO C17.
 * ========================================================================
 *
 * See jvm_class_file.h for the design. The parser reads through a
 * bounds-checked cursor (`Reader`) that carries a sticky status: once a read
 * runs past the end of the buffer (or a lookup fails), every later read is a
 * no-op and the first diagnostic is preserved. Callers check `reader.status`
 * after a sequence of reads rather than after every byte.
 */
#include "jvm_class_file.h"

#include <stdarg.h> /* va_list */
#include <stdio.h>  /* vsnprintf */
#include <stdlib.h> /* malloc, calloc, realloc, free */
#include <string.h> /* memcpy, strcmp, strlen */

/* ── Small helpers ──────────────────────────────────────────────────────────*/

static void set_errmsg(char *errbuf, size_t errlen, const char *fmt, ...) {
    va_list ap;
    if (!errbuf || errlen == 0) {
        return;
    }
    va_start(ap, fmt);
    vsnprintf(errbuf, errlen, fmt, ap);
    va_end(ap);
}

/* Copy `len` bytes into a fresh NUL-terminated string. NULL on OOM. */
static char *dup_bytes_str(const uint8_t *p, size_t len) {
    char *s = (char *)malloc(len + 1);
    if (!s) {
        return NULL;
    }
    if (len) {
        memcpy(s, p, len);
    }
    s[len] = '\0';
    return s;
}
static char *dup_cstr(const char *p) {
    return dup_bytes_str((const uint8_t *)p, strlen(p));
}

/* Grow guard: cap the doubling loop so cap*elem cannot overflow size_t. */
static int grow(void **data, size_t *cap, size_t need, size_t elem) {
    size_t nc;
    void *nd;
    if (need <= *cap) {
        return 1;
    }
    nc = *cap ? *cap : 4;
    while (nc < need) {
        if (nc > (size_t)-1 / 2) {
            nc = need;
            break;
        }
        nc *= 2;
    }
    if (nc > (size_t)-1 / elem) {
        return 0;
    }
    nd = realloc(*data, nc * elem);
    if (!nd) {
        return 0;
    }
    *data = nd;
    *cap = nc;
    return 1;
}

/* ── Data model ─────────────────────────────────────────────────────────────*/

typedef struct {
    int present;
    JvmConstantKind kind;
    char *utf8;
    int32_t integer;
    int64_t long_v;
    double double_v;
    uint16_t a;
    uint16_t b;
} CPEntry;

typedef struct {
    char *name;
    uint8_t *info;
    size_t info_len;
} RawAttr;

typedef struct {
    uint16_t max_stack;
    uint16_t max_locals;
    uint8_t *code;
    size_t code_len;
    RawAttr *nested;
    size_t nested_len;
} CodeAttr;

typedef struct {
    int is_code;
    CodeAttr code;
    RawAttr raw;
} MethodAttr;

struct JvmMethod {
    uint16_t access_flags;
    char *name;
    char *descriptor;
    MethodAttr *attrs;
    size_t attrs_len;
};

typedef struct {
    uint16_t access_flags;
    char *name;
    char *descriptor;
} FieldInfo;

struct JvmClassFile {
    uint16_t major, minor, access_flags;
    char *this_class_name;
    char *super_class_name;
    CPEntry *pool;
    size_t pool_len;
    FieldInfo *fields;
    size_t fields_len;
    JvmMethod *methods;
    size_t methods_len;
};

/* ── Destructors ────────────────────────────────────────────────────────────*/

static void free_raw(RawAttr *r) {
    free(r->name);
    free(r->info);
    r->name = NULL;
    r->info = NULL;
}

static void free_method_attr(MethodAttr *a) {
    if (a->is_code) {
        size_t i;
        free(a->code.code);
        for (i = 0; i < a->code.nested_len; i++) {
            free_raw(&a->code.nested[i]);
        }
        free(a->code.nested);
    } else {
        free_raw(&a->raw);
    }
}

static void free_method(JvmMethod *m) {
    size_t i;
    free(m->name);
    free(m->descriptor);
    for (i = 0; i < m->attrs_len; i++) {
        free_method_attr(&m->attrs[i]);
    }
    free(m->attrs);
}

void jvm_class_free(JvmClassFile *cf) {
    size_t i;
    if (!cf) {
        return;
    }
    free(cf->this_class_name);
    free(cf->super_class_name);
    for (i = 0; i < cf->pool_len; i++) {
        free(cf->pool[i].utf8);
    }
    free(cf->pool);
    for (i = 0; i < cf->fields_len; i++) {
        free(cf->fields[i].name);
        free(cf->fields[i].descriptor);
    }
    free(cf->fields);
    for (i = 0; i < cf->methods_len; i++) {
        free_method(&cf->methods[i]);
    }
    free(cf->methods);
    free(cf);
}

/* ── Bounds-checked reader ──────────────────────────────────────────────────*/

typedef struct {
    const uint8_t *data;
    size_t len;
    size_t off;
    JvmStatus status;
    char *errbuf;
    size_t errlen;
} Reader;

static size_t rd_remaining(const Reader *r) { return r->len - r->off; }

static const uint8_t *rd_read(Reader *r, size_t n) {
    const uint8_t *p;
    if (r->status != JVM_OK) {
        return NULL;
    }
    if (n > rd_remaining(r)) {
        r->status = JVM_ERR_FORMAT;
        set_errmsg(r->errbuf, r->errlen,
                   "Unexpected end of class file: need %zu bytes, have %zu", n,
                   rd_remaining(r));
        return NULL;
    }
    p = r->data + r->off;
    r->off += n;
    return p;
}

static uint8_t rd_u1(Reader *r) {
    const uint8_t *p = rd_read(r, 1);
    return p ? p[0] : 0;
}
static uint16_t rd_u2(Reader *r) {
    const uint8_t *p = rd_read(r, 2);
    if (!p) {
        return 0;
    }
    return (uint16_t)(((uint16_t)p[0] << 8) | p[1]);
}
static uint32_t rd_u4(Reader *r) {
    const uint8_t *p = rd_read(r, 4);
    if (!p) {
        return 0;
    }
    return ((uint32_t)p[0] << 24) | ((uint32_t)p[1] << 16) |
           ((uint32_t)p[2] << 8) | (uint32_t)p[3];
}
static int32_t rd_i4(Reader *r) { return (int32_t)rd_u4(r); }
static int64_t rd_i8(Reader *r) {
    uint64_t hi = rd_u4(r);
    uint64_t lo = rd_u4(r);
    return (int64_t)((hi << 32) | lo);
}
static double rd_f8(Reader *r) {
    int64_t bits = rd_i8(r);
    double d;
    memcpy(&d, &bits, sizeof d);
    return d;
}

/* ── Pool lookup during parse ───────────────────────────────────────────────*/

/* Borrowed UTF-8 string at `index`, or set status + NULL. */
static const char *pool_utf8(Reader *r, const CPEntry *pool, size_t pool_len,
                             uint16_t index) {
    if (r->status != JVM_OK) {
        return NULL;
    }
    if ((size_t)index >= pool_len || !pool[index].present) {
        r->status = JVM_ERR_FORMAT;
        set_errmsg(r->errbuf, r->errlen,
                   "Constant pool entry %u is out of range", (unsigned)index);
        return NULL;
    }
    if (pool[index].kind != JVM_CP_UTF8) {
        r->status = JVM_ERR_FORMAT;
        set_errmsg(r->errbuf, r->errlen,
                   "Constant pool entry %u is not a UTF-8 string",
                   (unsigned)index);
        return NULL;
    }
    return pool[index].utf8;
}

/* ── Attribute parsing (recursive for the Code attribute) ───────────────────*/

/* Parse one attribute into *out. Returns status; on error, *out is left in a
 * freeable (zeroed or partially built + owned) state and the caller frees it. */
static JvmStatus parse_attribute(Reader *r, const CPEntry *pool,
                                 size_t pool_len, int allow_code,
                                 MethodAttr *out) {
    const char *name;
    uint32_t attr_len;
    const uint8_t *body;

    memset(out, 0, sizeof *out);
    name = pool_utf8(r, pool, pool_len, rd_u2(r));
    attr_len = rd_u4(r);
    if (r->status != JVM_OK) {
        return r->status;
    }

    if (name && strcmp(name, "Code") == 0 && allow_code) {
        Reader nested;
        uint32_t code_len;
        const uint8_t *code_bytes;
        uint16_t exc_count, nested_count, i;

        body = rd_read(r, attr_len);
        if (r->status != JVM_OK) {
            return r->status;
        }
        nested.data = body;
        nested.len = attr_len;
        nested.off = 0;
        nested.status = JVM_OK;
        nested.errbuf = r->errbuf;
        nested.errlen = r->errlen;

        out->is_code = 1;
        out->code.max_stack = rd_u2(&nested);
        out->code.max_locals = rd_u2(&nested);
        code_len = rd_u4(&nested);
        code_bytes = rd_read(&nested, code_len);
        if (nested.status != JVM_OK) {
            return (r->status = nested.status);
        }
        if (code_len) {
            out->code.code = (uint8_t *)malloc(code_len);
            if (!out->code.code) {
                return (r->status = JVM_ERR_OUT_OF_MEMORY);
            }
            memcpy(out->code.code, code_bytes, code_len);
        }
        out->code.code_len = code_len;

        exc_count = rd_u2(&nested);
        for (i = 0; i < exc_count; i++) {
            rd_read(&nested, 8);
        }
        if (nested.status != JVM_OK) {
            return (r->status = nested.status);
        }

        nested_count = rd_u2(&nested);
        for (i = 0; i < nested_count; i++) {
            MethodAttr inner;
            size_t cap = out->code.nested_len;
            JvmStatus st = parse_attribute(&nested, pool, pool_len, 0, &inner);
            if (st != JVM_OK) {
                free_method_attr(&inner);
                return (r->status = st);
            }
            if (inner.is_code) {
                free_method_attr(&inner);
                r->status = JVM_ERR_FORMAT;
                set_errmsg(r->errbuf, r->errlen,
                           "nested Code attributes are not supported");
                return r->status;
            }
            if (!grow((void **)&out->code.nested, &cap,
                      out->code.nested_len + 1, sizeof(RawAttr))) {
                free_method_attr(&inner);
                return (r->status = JVM_ERR_OUT_OF_MEMORY);
            }
            out->code.nested[out->code.nested_len++] = inner.raw;
        }
        if (nested.status != JVM_OK) {
            return (r->status = nested.status);
        }
        if (rd_remaining(&nested) != 0) {
            r->status = JVM_ERR_FORMAT;
            set_errmsg(r->errbuf, r->errlen,
                       "trailing bytes inside Code attribute");
            return r->status;
        }
        return JVM_OK;
    }

    /* Raw attribute. */
    body = rd_read(r, attr_len);
    if (r->status != JVM_OK) {
        return r->status;
    }
    out->is_code = 0;
    out->raw.name = dup_cstr(name ? name : "");
    if (!out->raw.name) {
        return (r->status = JVM_ERR_OUT_OF_MEMORY);
    }
    if (attr_len) {
        out->raw.info = (uint8_t *)malloc(attr_len);
        if (!out->raw.info) {
            return (r->status = JVM_ERR_OUT_OF_MEMORY);
        }
        memcpy(out->raw.info, body, attr_len);
    }
    out->raw.info_len = attr_len;
    return JVM_OK;
}

static JvmStatus parse_method(Reader *r, const CPEntry *pool, size_t pool_len,
                              JvmMethod *out) {
    const char *name, *descriptor;
    uint16_t attributes_count, i;
    size_t cap = 0;

    memset(out, 0, sizeof *out);
    out->access_flags = rd_u2(r);
    name = pool_utf8(r, pool, pool_len, rd_u2(r));
    descriptor = pool_utf8(r, pool, pool_len, rd_u2(r));
    attributes_count = rd_u2(r);
    if (r->status != JVM_OK) {
        return r->status;
    }
    out->name = dup_cstr(name);
    out->descriptor = dup_cstr(descriptor);
    if (!out->name || !out->descriptor) {
        return (r->status = JVM_ERR_OUT_OF_MEMORY);
    }
    for (i = 0; i < attributes_count; i++) {
        MethodAttr attr;
        JvmStatus st = parse_attribute(r, pool, pool_len, 1, &attr);
        if (st != JVM_OK) {
            free_method_attr(&attr);
            return st;
        }
        if (!grow((void **)&out->attrs, &cap, out->attrs_len + 1,
                  sizeof(MethodAttr))) {
            free_method_attr(&attr);
            return (r->status = JVM_ERR_OUT_OF_MEMORY);
        }
        out->attrs[out->attrs_len++] = attr;
    }
    return JVM_OK;
}

/* ── Public: parse ──────────────────────────────────────────────────────────*/

/* Resolve a Class-entry index to its name string (borrowed). */
static const char *resolve_class_name_internal(const JvmClassFile *cf,
                                                uint16_t index, JvmStatus *st,
                                                char *errbuf, size_t errlen);

JvmStatus jvm_parse_class_file(const uint8_t *data, size_t len,
                               JvmClassFile **out, char *errbuf, size_t errlen) {
    Reader r;
    JvmClassFile *cf;
    uint32_t magic;
    size_t pool_count, index;
    uint16_t this_class_index, super_class_index, interfaces_count;
    uint16_t fields_count, methods_count, class_attr_count, i;
    JvmStatus st;
    const char *tn, *sn;

    *out = NULL;
    r.data = data;
    r.len = len;
    r.off = 0;
    r.status = JVM_OK;
    r.errbuf = errbuf;
    r.errlen = errlen;

    magic = rd_u4(&r);
    if (r.status != JVM_OK) {
        return r.status;
    }
    if (magic != 0xCAFEBABEu) {
        set_errmsg(errbuf, errlen,
                   "Invalid class-file magic: expected 0xCAFEBABE, got 0x%08X",
                   magic);
        return JVM_ERR_FORMAT;
    }

    cf = (JvmClassFile *)calloc(1, sizeof(JvmClassFile));
    if (!cf) {
        return JVM_ERR_OUT_OF_MEMORY;
    }

    cf->minor = rd_u2(&r);
    cf->major = rd_u2(&r);
    pool_count = rd_u2(&r);
    if (r.status != JVM_OK) {
        jvm_class_free(cf);
        return r.status;
    }
    cf->pool = (CPEntry *)calloc(pool_count ? pool_count : 1, sizeof(CPEntry));
    if (!cf->pool) {
        jvm_class_free(cf);
        return JVM_ERR_OUT_OF_MEMORY;
    }
    cf->pool_len = pool_count;

    index = 1;
    while (index < pool_count) {
        uint8_t tag = rd_u1(&r);
        CPEntry *e;
        if (r.status != JVM_OK) {
            jvm_class_free(cf);
            return r.status;
        }
        e = &cf->pool[index];
        switch (tag) {
            case JVM_CP_UTF8: {
                uint16_t slen = rd_u2(&r);
                const uint8_t *bytes = rd_read(&r, slen);
                if (r.status != JVM_OK) {
                    jvm_class_free(cf);
                    return r.status;
                }
                e->utf8 = dup_bytes_str(bytes, slen);
                if (!e->utf8) {
                    jvm_class_free(cf);
                    return JVM_ERR_OUT_OF_MEMORY;
                }
                e->kind = JVM_CP_UTF8;
                e->present = 1;
                break;
            }
            case JVM_CP_INTEGER:
                e->integer = rd_i4(&r);
                e->kind = JVM_CP_INTEGER;
                e->present = 1;
                break;
            case JVM_CP_LONG:
                e->long_v = rd_i8(&r);
                e->kind = JVM_CP_LONG;
                e->present = 1;
                index += 2;
                continue;
            case JVM_CP_DOUBLE:
                e->double_v = rd_f8(&r);
                e->kind = JVM_CP_DOUBLE;
                e->present = 1;
                index += 2;
                continue;
            case JVM_CP_CLASS:
                e->a = rd_u2(&r);
                e->kind = JVM_CP_CLASS;
                e->present = 1;
                break;
            case JVM_CP_STRING:
                e->a = rd_u2(&r);
                e->kind = JVM_CP_STRING;
                e->present = 1;
                break;
            case JVM_CP_FIELDREF:
                e->a = rd_u2(&r);
                e->b = rd_u2(&r);
                e->kind = JVM_CP_FIELDREF;
                e->present = 1;
                break;
            case JVM_CP_METHODREF:
                e->a = rd_u2(&r);
                e->b = rd_u2(&r);
                e->kind = JVM_CP_METHODREF;
                e->present = 1;
                break;
            case JVM_CP_NAME_AND_TYPE:
                e->a = rd_u2(&r);
                e->b = rd_u2(&r);
                e->kind = JVM_CP_NAME_AND_TYPE;
                e->present = 1;
                break;
            default:
                set_errmsg(errbuf, errlen, "Unsupported constant-pool tag: %u",
                           (unsigned)tag);
                jvm_class_free(cf);
                return JVM_ERR_FORMAT;
        }
        if (r.status != JVM_OK) {
            jvm_class_free(cf);
            return r.status;
        }
        index += 1;
    }

    cf->access_flags = rd_u2(&r);
    this_class_index = rd_u2(&r);
    super_class_index = rd_u2(&r);
    interfaces_count = rd_u2(&r);
    for (i = 0; i < interfaces_count; i++) {
        rd_u2(&r);
    }
    if (r.status != JVM_OK) {
        jvm_class_free(cf);
        return r.status;
    }

    fields_count = rd_u2(&r);
    if (r.status != JVM_OK) {
        jvm_class_free(cf);
        return r.status;
    }
    cf->fields = (FieldInfo *)calloc(fields_count ? fields_count : 1,
                                     sizeof(FieldInfo));
    if (!cf->fields) {
        jvm_class_free(cf);
        return JVM_ERR_OUT_OF_MEMORY;
    }
    for (i = 0; i < fields_count; i++) {
        FieldInfo *f = &cf->fields[cf->fields_len];
        const char *fn, *fd;
        uint16_t attrs, k;
        f->access_flags = rd_u2(&r);
        fn = pool_utf8(&r, cf->pool, cf->pool_len, rd_u2(&r));
        fd = pool_utf8(&r, cf->pool, cf->pool_len, rd_u2(&r));
        attrs = rd_u2(&r);
        if (r.status != JVM_OK) {
            jvm_class_free(cf);
            return r.status;
        }
        f->name = dup_cstr(fn);
        f->descriptor = dup_cstr(fd);
        if (!f->name || !f->descriptor) {
            free(f->name);
            free(f->descriptor);
            f->name = f->descriptor = NULL;
            jvm_class_free(cf);
            return JVM_ERR_OUT_OF_MEMORY;
        }
        cf->fields_len++;
        for (k = 0; k < attrs; k++) {
            MethodAttr skip;
            st = parse_attribute(&r, cf->pool, cf->pool_len, 0, &skip);
            free_method_attr(&skip);
            if (st != JVM_OK) {
                jvm_class_free(cf);
                return st;
            }
        }
    }

    methods_count = rd_u2(&r);
    if (r.status != JVM_OK) {
        jvm_class_free(cf);
        return r.status;
    }
    cf->methods = (JvmMethod *)calloc(methods_count ? methods_count : 1,
                                      sizeof(JvmMethod));
    if (!cf->methods) {
        jvm_class_free(cf);
        return JVM_ERR_OUT_OF_MEMORY;
    }
    for (i = 0; i < methods_count; i++) {
        st = parse_method(&r, cf->pool, cf->pool_len,
                          &cf->methods[cf->methods_len]);
        if (st != JVM_OK) {
            free_method(&cf->methods[cf->methods_len]);
            jvm_class_free(cf);
            return st;
        }
        cf->methods_len++;
    }

    class_attr_count = rd_u2(&r);
    if (r.status != JVM_OK) {
        jvm_class_free(cf);
        return r.status;
    }
    for (i = 0; i < class_attr_count; i++) {
        MethodAttr skip;
        st = parse_attribute(&r, cf->pool, cf->pool_len, 0, &skip);
        free_method_attr(&skip);
        if (st != JVM_OK) {
            jvm_class_free(cf);
            return st;
        }
    }

    if (rd_remaining(&r) != 0) {
        set_errmsg(errbuf, errlen, "Trailing bytes after class-file parse: %zu",
                   rd_remaining(&r));
        jvm_class_free(cf);
        return JVM_ERR_FORMAT;
    }

    /* Resolve this/super class names last (needs the full pool). */
    st = JVM_OK;
    tn = resolve_class_name_internal(cf, this_class_index, &st, errbuf, errlen);
    if (st != JVM_OK) {
        jvm_class_free(cf);
        return st;
    }
    sn = resolve_class_name_internal(cf, super_class_index, &st, errbuf, errlen);
    if (st != JVM_OK) {
        jvm_class_free(cf);
        return st;
    }
    cf->this_class_name = dup_cstr(tn);
    cf->super_class_name = dup_cstr(sn);
    if (!cf->this_class_name || !cf->super_class_name) {
        jvm_class_free(cf);
        return JVM_ERR_OUT_OF_MEMORY;
    }

    *out = cf;
    return JVM_OK;
}

/* ── Accessors ──────────────────────────────────────────────────────────────*/

void jvm_class_version(const JvmClassFile *cf, uint16_t *major,
                       uint16_t *minor) {
    if (major) {
        *major = cf->major;
    }
    if (minor) {
        *minor = cf->minor;
    }
}
uint16_t jvm_class_access_flags(const JvmClassFile *cf) {
    return cf->access_flags;
}
const char *jvm_class_this_name(const JvmClassFile *cf) {
    return cf->this_class_name;
}
const char *jvm_class_super_name(const JvmClassFile *cf) {
    return cf->super_class_name;
}
size_t jvm_class_constant_pool_len(const JvmClassFile *cf) {
    return cf->pool_len;
}
int jvm_class_constant_kind(const JvmClassFile *cf, uint16_t index,
                            JvmConstantKind *kind) {
    if ((size_t)index >= cf->pool_len || !cf->pool[index].present) {
        return 0;
    }
    if (kind) {
        *kind = cf->pool[index].kind;
    }
    return 1;
}

/* Borrowed entry lookup with range/wide-slot checks. */
static const CPEntry *cp_entry(const JvmClassFile *cf, uint16_t index,
                               JvmStatus *st, char *errbuf, size_t errlen) {
    if (index == 0 || (size_t)index >= cf->pool_len) {
        *st = JVM_ERR_FORMAT;
        set_errmsg(errbuf, errlen, "Constant pool index %u is out of range",
                   (unsigned)index);
        return NULL;
    }
    if (!cf->pool[index].present) {
        *st = JVM_ERR_FORMAT;
        set_errmsg(errbuf, errlen,
                   "Constant pool index %u points at a reserved wide slot",
                   (unsigned)index);
        return NULL;
    }
    return &cf->pool[index];
}

JvmStatus jvm_get_utf8(const JvmClassFile *cf, uint16_t index,
                       const char **out, char *errbuf, size_t errlen) {
    JvmStatus st = JVM_OK;
    const CPEntry *e = cp_entry(cf, index, &st, errbuf, errlen);
    if (!e) {
        return st;
    }
    if (e->kind != JVM_CP_UTF8) {
        set_errmsg(errbuf, errlen,
                   "Constant pool entry %u is not a UTF-8 string",
                   (unsigned)index);
        return JVM_ERR_FORMAT;
    }
    *out = e->utf8;
    return JVM_OK;
}

static const char *resolve_class_name_internal(const JvmClassFile *cf,
                                               uint16_t index, JvmStatus *st,
                                               char *errbuf, size_t errlen) {
    const CPEntry *e = cp_entry(cf, index, st, errbuf, errlen);
    const char *out = NULL;
    if (!e) {
        return NULL;
    }
    if (e->kind != JVM_CP_CLASS) {
        *st = JVM_ERR_FORMAT;
        set_errmsg(errbuf, errlen, "Constant pool entry %u is not a Class entry",
                   (unsigned)index);
        return NULL;
    }
    *st = jvm_get_utf8(cf, e->a, &out, errbuf, errlen);
    return (*st == JVM_OK) ? out : NULL;
}

/* Resolve a NameAndType index to (name, descriptor) borrowed strings. */
static JvmStatus resolve_name_and_type(const JvmClassFile *cf, uint16_t index,
                                       const char **name, const char **desc,
                                       char *errbuf, size_t errlen) {
    JvmStatus st = JVM_OK;
    const CPEntry *e = cp_entry(cf, index, &st, errbuf, errlen);
    if (!e) {
        return st;
    }
    if (e->kind != JVM_CP_NAME_AND_TYPE) {
        set_errmsg(errbuf, errlen,
                   "Constant pool entry %u is not a NameAndType entry",
                   (unsigned)index);
        return JVM_ERR_FORMAT;
    }
    st = jvm_get_utf8(cf, e->a, name, errbuf, errlen);
    if (st != JVM_OK) {
        return st;
    }
    return jvm_get_utf8(cf, e->b, desc, errbuf, errlen);
}

JvmStatus jvm_resolve_constant(const JvmClassFile *cf, uint16_t index,
                               JvmResolvedConstant *out, char *errbuf,
                               size_t errlen) {
    JvmStatus st = JVM_OK;
    const CPEntry *e = cp_entry(cf, index, &st, errbuf, errlen);
    if (!e) {
        return st;
    }
    out->text = NULL;
    out->integer = 0;
    out->long_v = 0;
    out->double_v = 0.0;
    switch (e->kind) {
        case JVM_CP_UTF8:
            out->kind = JVM_RC_UTF8;
            out->text = e->utf8;
            return JVM_OK;
        case JVM_CP_INTEGER:
            out->kind = JVM_RC_INTEGER;
            out->integer = e->integer;
            return JVM_OK;
        case JVM_CP_LONG:
            out->kind = JVM_RC_LONG;
            out->long_v = e->long_v;
            return JVM_OK;
        case JVM_CP_DOUBLE:
            out->kind = JVM_RC_DOUBLE;
            out->double_v = e->double_v;
            return JVM_OK;
        case JVM_CP_STRING:
            out->kind = JVM_RC_STRING;
            return jvm_get_utf8(cf, e->a, &out->text, errbuf, errlen);
        default:
            set_errmsg(errbuf, errlen,
                       "Constant pool entry %u is not a loadable constant",
                       (unsigned)index);
            return JVM_ERR_FORMAT;
    }
}

JvmStatus jvm_resolve_fieldref(const JvmClassFile *cf, uint16_t index,
                               JvmMemberRef *out, char *errbuf, size_t errlen) {
    JvmStatus st = JVM_OK;
    const CPEntry *e = cp_entry(cf, index, &st, errbuf, errlen);
    if (!e) {
        return st;
    }
    if (e->kind != JVM_CP_FIELDREF) {
        set_errmsg(errbuf, errlen,
                   "Constant pool entry %u is not a Fieldref entry",
                   (unsigned)index);
        return JVM_ERR_FORMAT;
    }
    out->class_name = resolve_class_name_internal(cf, e->a, &st, errbuf, errlen);
    if (st != JVM_OK) {
        return st;
    }
    return resolve_name_and_type(cf, e->b, &out->name, &out->descriptor, errbuf,
                                 errlen);
}

JvmStatus jvm_resolve_methodref(const JvmClassFile *cf, uint16_t index,
                                JvmMemberRef *out, char *errbuf, size_t errlen) {
    JvmStatus st = JVM_OK;
    const CPEntry *e = cp_entry(cf, index, &st, errbuf, errlen);
    if (!e) {
        return st;
    }
    if (e->kind != JVM_CP_METHODREF) {
        set_errmsg(errbuf, errlen,
                   "Constant pool entry %u is not a Methodref entry",
                   (unsigned)index);
        return JVM_ERR_FORMAT;
    }
    out->class_name = resolve_class_name_internal(cf, e->a, &st, errbuf, errlen);
    if (st != JVM_OK) {
        return st;
    }
    return resolve_name_and_type(cf, e->b, &out->name, &out->descriptor, errbuf,
                                 errlen);
}

/* ── Method accessors ───────────────────────────────────────────────────────*/
size_t jvm_class_method_count(const JvmClassFile *cf) { return cf->methods_len; }
const JvmMethod *jvm_class_method(const JvmClassFile *cf, size_t i) {
    return i < cf->methods_len ? &cf->methods[i] : NULL;
}
const JvmMethod *jvm_class_find_method(const JvmClassFile *cf, const char *name,
                                       const char *descriptor) {
    size_t i;
    for (i = 0; i < cf->methods_len; i++) {
        const JvmMethod *m = &cf->methods[i];
        if (strcmp(m->name, name) == 0 &&
            (descriptor == NULL || strcmp(descriptor, m->descriptor) == 0)) {
            return m;
        }
    }
    return NULL;
}
uint16_t jvm_method_access_flags(const JvmMethod *m) { return m->access_flags; }
const char *jvm_method_name(const JvmMethod *m) { return m->name; }
const char *jvm_method_descriptor(const JvmMethod *m) { return m->descriptor; }

static const MethodAttr *method_code(const JvmMethod *m) {
    size_t i;
    for (i = 0; i < m->attrs_len; i++) {
        if (m->attrs[i].is_code) {
            return &m->attrs[i];
        }
    }
    return NULL;
}

int jvm_method_code(const JvmMethod *m, JvmCodeView *out) {
    const MethodAttr *a = method_code(m);
    if (!a) {
        return 0;
    }
    if (out) {
        out->code = a->code.code;
        out->code_len = a->code.code_len;
        out->max_stack = a->code.max_stack;
        out->max_locals = a->code.max_locals;
        out->nested_attribute_count = a->code.nested_len;
    }
    return 1;
}

const char *jvm_method_code_nested_name(const JvmMethod *m, size_t i) {
    const MethodAttr *a = method_code(m);
    if (!a || i >= a->code.nested_len) {
        return NULL;
    }
    return a->code.nested[i].name;
}

/* ── Minimal builder ────────────────────────────────────────────────────────*/

typedef struct {
    uint8_t *data;
    size_t len, cap;
    int oom;
} ByteBuf;

static void bb_append(ByteBuf *b, const uint8_t *p, size_t n) {
    if (b->oom) {
        return;
    }
    if (!grow((void **)&b->data, &b->cap, b->len + n, 1)) {
        b->oom = 1;
        return;
    }
    if (n) {
        memcpy(b->data + b->len, p, n);
    }
    b->len += n;
}
static void bb_u1(ByteBuf *b, uint8_t v) { bb_append(b, &v, 1); }
static void bb_u2(ByteBuf *b, uint16_t v) {
    uint8_t t[2];
    t[0] = (uint8_t)(v >> 8);
    t[1] = (uint8_t)(v & 0xFF);
    bb_append(b, t, 2);
}
static void bb_u4(ByteBuf *b, uint32_t v) {
    uint8_t t[4];
    t[0] = (uint8_t)(v >> 24);
    t[1] = (uint8_t)((v >> 16) & 0xFF);
    t[2] = (uint8_t)((v >> 8) & 0xFF);
    t[3] = (uint8_t)(v & 0xFF);
    bb_append(b, t, 4);
}

/* A constant-pool builder: dedup by string key, emit in insertion order. */
typedef struct {
    char *key;
    ByteBuf payload;
} PoolEnt;
typedef struct {
    PoolEnt *ents;
    size_t len, cap;
    int oom;
} PoolBuilder;

static void pool_builder_free(PoolBuilder *p) {
    size_t i;
    for (i = 0; i < p->len; i++) {
        free(p->ents[i].key);
        free(p->ents[i].payload.data);
    }
    free(p->ents);
    p->ents = NULL;
    p->len = p->cap = 0;
}

/* Insert `payload` under `key` (deduped). Returns the 1-based index, or 0 on
 * error (OOM latched in p->oom). Takes ownership of payload->data. */
static uint16_t pool_add(PoolBuilder *p, const char *key, ByteBuf *payload) {
    size_t i;
    if (p->oom || payload->oom) {
        p->oom = 1;
        free(payload->data);
        return 0;
    }
    for (i = 0; i < p->len; i++) {
        if (strcmp(p->ents[i].key, key) == 0) {
            free(payload->data);
            return (uint16_t)(i + 1);
        }
    }
    /* The emitted constant_pool_count is entries + 1, so entries must stay
     * <= 0xFFFE for the count to fit in a u16 (matches Rust's try_from). */
    if (p->len >= 0xFFFE) {
        p->oom = 1; /* pool exceeds u16 count — reuse the OOM latch as "fail" */
        free(payload->data);
        return 0;
    }
    if (!grow((void **)&p->ents, &p->cap, p->len + 1, sizeof(PoolEnt))) {
        p->oom = 1;
        free(payload->data);
        return 0;
    }
    p->ents[p->len].key = dup_cstr(key);
    if (!p->ents[p->len].key) {
        p->oom = 1;
        free(payload->data);
        return 0;
    }
    p->ents[p->len].payload = *payload; /* transfer ownership */
    p->len++;
    return (uint16_t)p->len;
}

static uint16_t pool_utf8_build(PoolBuilder *p, const char *value) {
    ByteBuf payload = {0};
    char *key;
    uint16_t idx;
    size_t vlen = strlen(value);
    if (vlen > 0xFFFF) {
        p->oom = 1;
        return 0;
    }
    bb_u1(&payload, JVM_CP_UTF8);
    bb_u2(&payload, (uint16_t)vlen);
    bb_append(&payload, (const uint8_t *)value, vlen);
    key = (char *)malloc(strlen("Utf8:") + vlen + 1);
    if (!key) {
        p->oom = 1;
        free(payload.data);
        return 0;
    }
    memcpy(key, "Utf8:", 5);
    memcpy(key + 5, value, vlen + 1);
    idx = pool_add(p, key, &payload);
    free(key);
    return idx;
}

static uint16_t pool_class_ref(PoolBuilder *p, const char *value) {
    ByteBuf payload = {0};
    char *key;
    uint16_t name_index, idx;
    size_t vlen = strlen(value);
    name_index = pool_utf8_build(p, value);
    if (p->oom) {
        return 0;
    }
    bb_u1(&payload, JVM_CP_CLASS);
    bb_u2(&payload, name_index);
    key = (char *)malloc(strlen("Class:") + vlen + 1);
    if (!key) {
        p->oom = 1;
        free(payload.data);
        return 0;
    }
    memcpy(key, "Class:", 6);
    memcpy(key + 6, value, vlen + 1);
    idx = pool_add(p, key, &payload);
    free(key);
    return idx;
}

static uint16_t pool_string(PoolBuilder *p, const char *value) {
    ByteBuf payload = {0};
    char *key;
    uint16_t string_index, idx;
    size_t vlen = strlen(value);
    string_index = pool_utf8_build(p, value);
    if (p->oom) {
        return 0;
    }
    bb_u1(&payload, JVM_CP_STRING);
    bb_u2(&payload, string_index);
    key = (char *)malloc(strlen("String:") + vlen + 1);
    if (!key) {
        p->oom = 1;
        free(payload.data);
        return 0;
    }
    memcpy(key, "String:", 7);
    memcpy(key + 7, value, vlen + 1);
    idx = pool_add(p, key, &payload);
    free(key);
    return idx;
}

static uint16_t pool_integer(PoolBuilder *p, int32_t value) {
    ByteBuf payload = {0};
    char key[32];
    uint16_t idx;
    bb_u1(&payload, JVM_CP_INTEGER);
    bb_u4(&payload, (uint32_t)value);
    /* key size is bounded by the int range; no truncation risk. */
    snprintf(key, sizeof key, "Integer:%ld", (long)value);
    idx = pool_add(p, key, &payload);
    return idx;
}

JvmBuildParams jvm_build_params_default(void) {
    JvmBuildParams p;
    p.class_name = "";
    p.method_name = "";
    p.descriptor = "";
    p.code = NULL;
    p.code_len = 0;
    p.max_stack = 0;
    p.max_locals = 0;
    p.constants = NULL;
    p.constant_count = 0;
    p.major_version = 61;
    p.minor_version = 0;
    p.class_access_flags = JVM_ACC_PUBLIC | JVM_ACC_SUPER;
    p.method_access_flags = JVM_ACC_PUBLIC | JVM_ACC_STATIC;
    p.super_class_name = "java/lang/Object";
    return p;
}

JvmStatus jvm_build_minimal_class_file(const JvmBuildParams *params,
                                       uint8_t **out_bytes, size_t *out_len,
                                       char *errbuf, size_t errlen) {
    PoolBuilder pool = {0};
    ByteBuf code_body = {0}, code_attr = {0}, method_info = {0}, out = {0};
    uint16_t this_index, super_index, method_name_index, descriptor_index,
        code_name_index;
    const char *super_name;
    size_t i;
    JvmStatus rc = JVM_ERR_OUT_OF_MEMORY;

    *out_bytes = NULL;
    *out_len = 0;

    if (!params->class_name || params->class_name[0] == '\0') {
        set_errmsg(errbuf, errlen, "class name must not be empty");
        return JVM_ERR_FORMAT;
    }
    if (!params->method_name || params->method_name[0] == '\0') {
        set_errmsg(errbuf, errlen, "method name must not be empty");
        return JVM_ERR_FORMAT;
    }
    if (!params->descriptor || params->descriptor[0] == '\0') {
        set_errmsg(errbuf, errlen, "descriptor must not be empty");
        return JVM_ERR_FORMAT;
    }

    super_name = (params->super_class_name && params->super_class_name[0])
                     ? params->super_class_name
                     : "java/lang/Object";

    this_index = pool_class_ref(&pool, params->class_name);
    super_index = pool_class_ref(&pool, super_name);
    method_name_index = pool_utf8_build(&pool, params->method_name);
    descriptor_index = pool_utf8_build(&pool, params->descriptor);
    code_name_index = pool_utf8_build(&pool, "Code");
    for (i = 0; i < params->constant_count; i++) {
        if (params->constants[i].kind == JVM_MIN_INTEGER) {
            pool_integer(&pool, params->constants[i].integer);
        } else {
            pool_string(&pool, params->constants[i].text
                                   ? params->constants[i].text
                                   : "");
        }
    }
    if (pool.oom) {
        goto done;
    }

    bb_u2(&code_body, params->max_stack);
    bb_u2(&code_body, params->max_locals);
    if (params->code_len > 0xFFFFFFFFu) {
        set_errmsg(errbuf, errlen, "method code exceeds 4 GiB");
        rc = JVM_ERR_FORMAT;
        goto done;
    }
    bb_u4(&code_body, (uint32_t)params->code_len);
    bb_append(&code_body, params->code, params->code_len);
    bb_u2(&code_body, 0);
    bb_u2(&code_body, 0);

    bb_u2(&code_attr, code_name_index);
    bb_u4(&code_attr, (uint32_t)code_body.len);
    bb_append(&code_attr, code_body.data, code_body.len);

    bb_u2(&method_info, params->method_access_flags);
    bb_u2(&method_info, method_name_index);
    bb_u2(&method_info, descriptor_index);
    bb_u2(&method_info, 1);
    bb_append(&method_info, code_attr.data, code_attr.len);

    bb_u4(&out, 0xCAFEBABEu);
    bb_u2(&out, params->minor_version);
    bb_u2(&out, params->major_version);
    bb_u2(&out, (uint16_t)(pool.len + 1));
    for (i = 0; i < pool.len; i++) {
        bb_append(&out, pool.ents[i].payload.data, pool.ents[i].payload.len);
    }
    bb_u2(&out, params->class_access_flags);
    bb_u2(&out, this_index);
    bb_u2(&out, super_index);
    bb_u2(&out, 0);
    bb_u2(&out, 0);
    bb_u2(&out, 1);
    bb_append(&out, method_info.data, method_info.len);
    bb_u2(&out, 0);

    if (code_body.oom || code_attr.oom || method_info.oom || out.oom) {
        goto done;
    }

    *out_bytes = out.data;
    *out_len = out.len;
    out.data = NULL; /* transfer ownership */
    rc = JVM_OK;

done:
    free(code_body.data);
    free(code_attr.data);
    free(method_info.data);
    free(out.data);
    pool_builder_free(&pool);
    if (rc == JVM_ERR_OUT_OF_MEMORY) {
        set_errmsg(errbuf, errlen, "out of memory building class file");
    }
    return rc;
}
