/*
 * dns_message.c — implementation of the pure-ISO C DNS wire codec.
 * ==============================================================
 *
 * See dns_message.h. The parser borrows the caller's input and builds owned
 * structures; the serializer builds an owned byte buffer. Every heap allocation
 * is paired with a free on both the success and error paths.
 */
#include "dns_message.h"

#include <stdlib.h> /* malloc, calloc, realloc, free */
#include <string.h> /* memcpy, memset, strlen, strcmp */

#define DNS_HEADER_LEN 12
#define MAX_LABEL_LEN 63
#define MAX_ENCODED_NAME_LEN 255
#define MIN_QUESTION_WIRE_LEN 5
#define MIN_RECORD_WIRE_LEN 11
#define MAX_NAME_POINTER_HOPS 128

/* ── Error constructors ────────────────────────────────────────────────────*/

static DnsError ok_err(void) {
    DnsError e;
    e.kind = DNS_OK;
    e.message = NULL;
    e.detail = 0;
    return e;
}
static DnsError err_kind(DnsErrorKind kind) {
    DnsError e;
    e.kind = kind;
    e.message = NULL;
    e.detail = 0;
    return e;
}
static DnsError err_detail(DnsErrorKind kind, size_t detail) {
    DnsError e = err_kind(kind);
    e.detail = detail;
    return e;
}
static DnsError err_unsupported(const char *message) {
    DnsError e = err_kind(DNS_ERR_UNSUPPORTED);
    e.message = message;
    return e;
}

const char *dns_error_kind_str(DnsErrorKind kind) {
    switch (kind) {
        case DNS_OK: return "ok";
        case DNS_ERR_TRUNCATED_HEADER: return "truncated header";
        case DNS_ERR_UNEXPECTED_EOF: return "unexpected end of buffer";
        case DNS_ERR_LABEL_TOO_LONG: return "label too long";
        case DNS_ERR_NAME_TOO_LONG: return "name too long";
        case DNS_ERR_POINTER_OUT_OF_BOUNDS:
            return "compression pointer out of bounds";
        case DNS_ERR_POINTER_LOOP: return "compression pointer loop";
        case DNS_ERR_NON_ASCII_LABEL: return "non-ASCII label";
        case DNS_ERR_INVALID_SECTION_COUNT: return "invalid section count";
        case DNS_ERR_UNSUPPORTED: return "unsupported";
        case DNS_ERR_OUT_OF_MEMORY: return "out of memory";
    }
    return "unknown error";
}

/* ── Small helpers ─────────────────────────────────────────────────────────*/

static char *dup_bytes_as_str(const uint8_t *p, size_t n) {
    char *s = (char *)malloc(n + 1);
    if (s == NULL) return NULL;
    if (n > 0) memcpy(s, p, n);
    s[n] = '\0';
    return s;
}

/* A growable byte buffer for serialization. */
typedef struct {
    uint8_t *data;
    size_t len;
    size_t cap;
    int oom;
} Buf;

static void buf_init(Buf *b) {
    b->data = NULL;
    b->len = 0;
    b->cap = 0;
    b->oom = 0;
}
static void buf_free(Buf *b) {
    free(b->data);
    buf_init(b);
}
static int buf_reserve(Buf *b, size_t extra) {
    size_t need, nc;
    uint8_t *nd;
    if (b->oom) return 0;
    if (extra > (size_t)-1 - b->len) {
        b->oom = 1;
        return 0;
    }
    need = b->len + extra;
    if (need <= b->cap) return 1;
    nc = b->cap ? b->cap : 32;
    while (nc < need) {
        if (nc > ((size_t)-1) / 2) {
            nc = need;
            break;
        }
        nc *= 2;
    }
    nd = (uint8_t *)realloc(b->data, nc);
    if (nd == NULL) {
        b->oom = 1;
        return 0;
    }
    b->data = nd;
    b->cap = nc;
    return 1;
}
static void buf_push(Buf *b, uint8_t byte) {
    if (!buf_reserve(b, 1)) return;
    b->data[b->len++] = byte;
}
static void buf_extend(Buf *b, const uint8_t *p, size_t n) {
    if (!buf_reserve(b, n)) return;
    if (n > 0) memcpy(b->data + b->len, p, n);
    b->len += n;
}
static void buf_u16(Buf *b, uint16_t v) {
    buf_push(b, (uint8_t)(v >> 8));
    buf_push(b, (uint8_t)(v & 0xff));
}
static void buf_u32(Buf *b, uint32_t v) {
    buf_push(b, (uint8_t)(v >> 24));
    buf_push(b, (uint8_t)((v >> 16) & 0xff));
    buf_push(b, (uint8_t)((v >> 8) & 0xff));
    buf_push(b, (uint8_t)(v & 0xff));
}

/* ── Enum conversions ──────────────────────────────────────────────────────*/

static DnsOpcode opcode_from_bits(uint8_t bits) {
    DnsOpcode o;
    if (bits == 0) {
        o.kind = DNS_OPCODE_QUERY;
        o.value = 0;
    } else {
        o.kind = DNS_OPCODE_UNKNOWN;
        o.value = bits;
    }
    return o;
}
static uint8_t opcode_to_bits(DnsOpcode o) {
    return o.kind == DNS_OPCODE_QUERY ? 0 : (uint8_t)(o.value & 0x0f);
}

static DnsResponseCode rcode_from_bits(uint8_t bits) {
    DnsResponseCode r;
    r.value = 0;
    switch (bits) {
        case 0: r.kind = DNS_RCODE_NO_ERROR; break;
        case 1: r.kind = DNS_RCODE_FORMAT_ERROR; break;
        case 2: r.kind = DNS_RCODE_SERVER_FAILURE; break;
        case 3: r.kind = DNS_RCODE_NAME_ERROR; break;
        case 4: r.kind = DNS_RCODE_NOT_IMPLEMENTED; break;
        case 5: r.kind = DNS_RCODE_REFUSED; break;
        default:
            r.kind = DNS_RCODE_UNKNOWN;
            r.value = bits;
            break;
    }
    return r;
}
static uint8_t rcode_to_bits(DnsResponseCode r) {
    switch (r.kind) {
        case DNS_RCODE_NO_ERROR: return 0;
        case DNS_RCODE_FORMAT_ERROR: return 1;
        case DNS_RCODE_SERVER_FAILURE: return 2;
        case DNS_RCODE_NAME_ERROR: return 3;
        case DNS_RCODE_NOT_IMPLEMENTED: return 4;
        case DNS_RCODE_REFUSED: return 5;
        case DNS_RCODE_UNKNOWN: return (uint8_t)(r.value & 0x0f);
    }
    return 0;
}

static DnsRecordType rtype_from_u16(uint16_t v) {
    DnsRecordType t;
    t.value = 0;
    switch (v) {
        case 1: t.kind = DNS_TYPE_A; break;
        case 2: t.kind = DNS_TYPE_NS; break;
        case 5: t.kind = DNS_TYPE_CNAME; break;
        case 6: t.kind = DNS_TYPE_SOA; break;
        case 12: t.kind = DNS_TYPE_PTR; break;
        case 15: t.kind = DNS_TYPE_MX; break;
        case 16: t.kind = DNS_TYPE_TXT; break;
        case 28: t.kind = DNS_TYPE_AAAA; break;
        case 33: t.kind = DNS_TYPE_SRV; break;
        default:
            t.kind = DNS_TYPE_UNKNOWN;
            t.value = v;
            break;
    }
    return t;
}
uint16_t dns_record_type_to_u16(DnsRecordType t) {
    switch (t.kind) {
        case DNS_TYPE_A: return 1;
        case DNS_TYPE_NS: return 2;
        case DNS_TYPE_CNAME: return 5;
        case DNS_TYPE_SOA: return 6;
        case DNS_TYPE_PTR: return 12;
        case DNS_TYPE_MX: return 15;
        case DNS_TYPE_TXT: return 16;
        case DNS_TYPE_AAAA: return 28;
        case DNS_TYPE_SRV: return 33;
        case DNS_TYPE_UNKNOWN: return t.value;
    }
    return 0;
}
DnsRecordType dns_record_type_known(DnsRecordTypeKind kind) {
    DnsRecordType t;
    t.kind = kind;
    t.value = 0;
    return t;
}
int dns_record_type_equal(DnsRecordType a, DnsRecordType b) {
    if (a.kind != b.kind) return 0;
    if (a.kind == DNS_TYPE_UNKNOWN) return a.value == b.value;
    return 1;
}

static DnsClass class_from_u16(uint16_t v) {
    DnsClass c;
    if (v == 1) {
        c.kind = DNS_CLASS_IN;
        c.value = 0;
    } else {
        c.kind = DNS_CLASS_UNKNOWN;
        c.value = v;
    }
    return c;
}
static uint16_t class_to_u16(DnsClass c) {
    return c.kind == DNS_CLASS_IN ? 1 : c.value;
}

/* ── Flags ─────────────────────────────────────────────────────────────────*/

DnsFlags dns_flags_query(void) {
    DnsFlags f;
    f.is_response = 0;
    f.opcode = opcode_from_bits(0);
    f.authoritative_answer = 0;
    f.truncated = 0;
    f.recursion_desired = 1;
    f.recursion_available = 0;
    f.response_code = rcode_from_bits(0);
    return f;
}
static DnsFlags flags_parse(uint16_t word) {
    DnsFlags f;
    f.is_response = (word & 0x8000) != 0;
    f.opcode = opcode_from_bits((uint8_t)((word >> 11) & 0x0f));
    f.authoritative_answer = (word & 0x0400) != 0;
    f.truncated = (word & 0x0200) != 0;
    f.recursion_desired = (word & 0x0100) != 0;
    f.recursion_available = (word & 0x0080) != 0;
    f.response_code = rcode_from_bits((uint8_t)(word & 0x000f));
    return f;
}
static uint16_t flags_serialize(const DnsFlags *f) {
    uint16_t word = 0;
    if (f->is_response) word |= 0x8000;
    word = (uint16_t)(word | ((uint16_t)opcode_to_bits(f->opcode) << 11));
    if (f->authoritative_answer) word |= 0x0400;
    if (f->truncated) word |= 0x0200;
    if (f->recursion_desired) word |= 0x0100;
    if (f->recursion_available) word |= 0x0080;
    word = (uint16_t)(word | (uint16_t)rcode_to_bits(f->response_code));
    return word;
}

/* ── Label / name validation ───────────────────────────────────────────────*/

static DnsError validate_label(const uint8_t *label, size_t len) {
    size_t i;
    if (len > MAX_LABEL_LEN) return err_detail(DNS_ERR_LABEL_TOO_LONG, len);
    for (i = 0; i < len; i++)
        if (label[i] > 0x7f) return err_kind(DNS_ERR_NON_ASCII_LABEL);
    return ok_err();
}
static DnsError validate_encoded_name_len(const DnsName *name) {
    size_t total = 1, i;
    for (i = 0; i < name->n_labels; i++) {
        size_t ll = strlen(name->labels[i]);
        if (ll > MAX_LABEL_LEN) return err_detail(DNS_ERR_LABEL_TOO_LONG, ll);
        total += 1 + ll;
    }
    if (total > MAX_ENCODED_NAME_LEN) return err_kind(DNS_ERR_NAME_TOO_LONG);
    return ok_err();
}

/* ── DnsName ───────────────────────────────────────────────────────────────*/

void dns_name_free(DnsName *name) {
    size_t i;
    if (name == NULL) return;
    for (i = 0; i < name->n_labels; i++) free(name->labels[i]);
    free(name->labels);
    name->labels = NULL;
    name->n_labels = 0;
}

int dns_name_is_root(const DnsName *name) { return name->n_labels == 0; }

/* Append an owned label string to a growable name. Takes ownership of `label`
 * (frees it on OOM). */
static int name_push_label(DnsName *name, size_t *cap, char *label) {
    if (name->n_labels == *cap) {
        size_t nc = *cap ? *cap : 4;
        char **nl;
        if (nc > ((size_t)-1) / 2 / sizeof(char *)) {
            free(label);
            return 0;
        }
        nc *= 2;
        nl = (char **)realloc(name->labels, nc * sizeof(char *));
        if (nl == NULL) {
            free(label);
            return 0;
        }
        name->labels = nl;
        *cap = nc;
    }
    name->labels[name->n_labels++] = label;
    return 1;
}

DnsError dns_name_from_ascii(const char *input, DnsName *out) {
    size_t cap = 0;
    size_t input_len = strlen(input);
    size_t trimmed_len = input_len;
    const char *p, *label_start;
    DnsError e;

    out->labels = NULL;
    out->n_labels = 0;

    /* trim trailing '.' as Rust's trim_end_matches('.') does */
    while (trimmed_len > 0 && input[trimmed_len - 1] == '.') trimmed_len--;

    if (strcmp(input, ".") == 0 || trimmed_len == 0) return ok_err(); /* root */

    label_start = input;
    for (p = input;; p++) {
        if (p == input + trimmed_len || *p == '.') {
            size_t ll = (size_t)(p - label_start);
            char *s;
            if (ll == 0) {
                dns_name_free(out);
                return err_unsupported("empty DNS label");
            }
            e = validate_label((const uint8_t *)label_start, ll);
            if (e.kind != DNS_OK) {
                dns_name_free(out);
                return e;
            }
            s = dup_bytes_as_str((const uint8_t *)label_start, ll);
            if (s == NULL) {
                dns_name_free(out);
                return err_kind(DNS_ERR_OUT_OF_MEMORY);
            }
            if (!name_push_label(out, &cap, s)) {
                dns_name_free(out);
                return err_kind(DNS_ERR_OUT_OF_MEMORY);
            }
            if (p == input + trimmed_len) break;
            label_start = p + 1;
        }
    }

    e = validate_encoded_name_len(out);
    if (e.kind != DNS_OK) {
        dns_name_free(out);
        return e;
    }
    return ok_err();
}

DnsError dns_name_clone(const DnsName *name, DnsName *out) {
    size_t i;
    out->labels = NULL;
    out->n_labels = 0;
    if (name->n_labels == 0) return ok_err();
    out->labels = (char **)calloc(name->n_labels, sizeof(char *));
    if (out->labels == NULL) return err_kind(DNS_ERR_OUT_OF_MEMORY);
    for (i = 0; i < name->n_labels; i++) {
        out->labels[i] = dup_bytes_as_str((const uint8_t *)name->labels[i],
                                          strlen(name->labels[i]));
        if (out->labels[i] == NULL) {
            out->n_labels = i;
            dns_name_free(out);
            return err_kind(DNS_ERR_OUT_OF_MEMORY);
        }
    }
    out->n_labels = name->n_labels;
    return ok_err();
}

int dns_name_equal(const DnsName *a, const DnsName *b) {
    size_t i;
    if (a->n_labels != b->n_labels) return 0;
    for (i = 0; i < a->n_labels; i++)
        if (strcmp(a->labels[i], b->labels[i]) != 0) return 0;
    return 1;
}

char *dns_name_to_string(const DnsName *name) {
    size_t total = 0, i, pos = 0;
    char *out;
    if (name->n_labels == 0) return dup_bytes_as_str((const uint8_t *)".", 1);
    for (i = 0; i < name->n_labels; i++) total += strlen(name->labels[i]) + 1;
    out = (char *)malloc(total); /* (n-1) dots + labels + NUL == total */
    if (out == NULL) return NULL;
    for (i = 0; i < name->n_labels; i++) {
        size_t ll = strlen(name->labels[i]);
        if (i > 0) out[pos++] = '.';
        memcpy(out + pos, name->labels[i], ll);
        pos += ll;
    }
    out[pos] = '\0';
    return out;
}

/* ── DnsRecordData / DnsResourceRecord free ────────────────────────────────*/

static void rdata_free(DnsRecordData *d) {
    switch (d->kind) {
        case DNS_RDATA_CNAME:
        case DNS_RDATA_PTR: dns_name_free(&d->name); break;
        case DNS_RDATA_SRV: dns_name_free(&d->srv.target); break;
        case DNS_RDATA_RAW:
            free(d->raw);
            d->raw = NULL;
            d->raw_len = 0;
            break;
        default: break;
    }
}
static void record_free(DnsResourceRecord *r) {
    dns_name_free(&r->name);
    rdata_free(&r->data);
}

void dns_message_free(DnsMessage *m) {
    size_t i;
    if (m == NULL) return;
    for (i = 0; i < m->n_questions; i++) dns_name_free(&m->questions[i].name);
    free(m->questions);
    for (i = 0; i < m->n_answers; i++) record_free(&m->answers[i]);
    free(m->answers);
    for (i = 0; i < m->n_authorities; i++) record_free(&m->authorities[i]);
    free(m->authorities);
    for (i = 0; i < m->n_additionals; i++) record_free(&m->additionals[i]);
    free(m->additionals);
    memset(m, 0, sizeof *m);
}

/* ── Reading primitives ────────────────────────────────────────────────────*/

static DnsError read_u16(const uint8_t *input, size_t len, size_t *cursor,
                         uint16_t *out) {
    if (len - *cursor < 2) return err_kind(DNS_ERR_UNEXPECTED_EOF);
    *out = (uint16_t)(((uint16_t)input[*cursor] << 8) | input[*cursor + 1]);
    *cursor += 2;
    return ok_err();
}
static DnsError read_u32(const uint8_t *input, size_t len, size_t *cursor,
                         uint32_t *out) {
    if (len - *cursor < 4) return err_kind(DNS_ERR_UNEXPECTED_EOF);
    *out = ((uint32_t)input[*cursor] << 24) |
           ((uint32_t)input[*cursor + 1] << 16) |
           ((uint32_t)input[*cursor + 2] << 8) | (uint32_t)input[*cursor + 3];
    *cursor += 4;
    return ok_err();
}

/* Read a (possibly compressed) name at *cursor, advancing it past the name in
 * the record stream (not into a pointed-to region).
 *
 * Loop safety without a visited-set: every iteration makes progress toward one
 * of two hard caps — a label read advances `offset` and adds to `encoded_len`
 * (capped at MAX_ENCODED_NAME_LEN == 255), and a pointer increments
 * `pointer_hops` (capped at MAX_NAME_POINTER_HOPS == 128). So a malicious
 * pointer chain (even a self-pointer) terminates in a bounded, constant number
 * of steps with DNS_ERR_POINTER_LOOP / DNS_ERR_NAME_TOO_LONG. The Rust original
 * also keeps a per-name HashSet of visited offsets; we omit it because the two
 * caps already guarantee termination, and a byte array sized to the whole
 * message would make parsing a many-name message O(message_len^2). */
static DnsError read_name(const uint8_t *input, size_t len, size_t *cursor,
                          DnsName *out) {
    size_t cap = 0;
    size_t offset = *cursor;
    int have_consumed = 0;
    size_t consumed_cursor = 0;
    size_t pointer_hops = 0;
    size_t encoded_len = 1;
    DnsError e = ok_err();

    out->labels = NULL;
    out->n_labels = 0;

    for (;;) {
        uint8_t l;
        unsigned top;
        if (offset >= len) {
            e = err_kind(DNS_ERR_UNEXPECTED_EOF);
            goto fail;
        }
        l = input[offset];
        top = l & 0xc0u;
        if (top == 0x00u) {
            offset += 1;
            if (l == 0) {
                *cursor = have_consumed ? consumed_cursor : offset;
                e = validate_encoded_name_len(out);
                if (e.kind != DNS_OK) goto fail;
                return ok_err();
            }
            {
                size_t label_len = l;
                const uint8_t *lb;
                char *s;
                if (label_len > MAX_LABEL_LEN) {
                    e = err_detail(DNS_ERR_LABEL_TOO_LONG, label_len);
                    goto fail;
                }
                if (len - offset < label_len) {
                    e = err_kind(DNS_ERR_UNEXPECTED_EOF);
                    goto fail;
                }
                lb = input + offset;
                e = validate_label(lb, label_len);
                if (e.kind != DNS_OK) goto fail;
                encoded_len += 1 + label_len;
                if (encoded_len > MAX_ENCODED_NAME_LEN) {
                    e = err_kind(DNS_ERR_NAME_TOO_LONG);
                    goto fail;
                }
                s = dup_bytes_as_str(lb, label_len);
                if (s == NULL) {
                    e = err_kind(DNS_ERR_OUT_OF_MEMORY);
                    goto fail;
                }
                if (!name_push_label(out, &cap, s)) {
                    e = err_kind(DNS_ERR_OUT_OF_MEMORY);
                    goto fail;
                }
                offset += label_len;
            }
        } else if (top == 0xc0u) {
            size_t pointer;
            if (len - offset < 2) {
                e = err_kind(DNS_ERR_UNEXPECTED_EOF);
                goto fail;
            }
            if (!have_consumed) {
                have_consumed = 1;
                consumed_cursor = offset + 2;
            }
            pointer_hops += 1;
            if (pointer_hops > MAX_NAME_POINTER_HOPS) {
                e = err_kind(DNS_ERR_POINTER_LOOP);
                goto fail;
            }
            pointer = (((size_t)l & 0x3f) << 8) | (size_t)input[offset + 1];
            if (pointer >= len) {
                e = err_detail(DNS_ERR_POINTER_OUT_OF_BOUNDS, pointer);
                goto fail;
            }
            offset = pointer;
        } else {
            e = err_unsupported("reserved DNS label prefix");
            goto fail;
        }
    }

fail:
    dns_name_free(out);
    return e;
}

/* ── Section parsing ───────────────────────────────────────────────────────*/

/* Reserve only what the remaining bytes could hold (counts are attacker
 * controlled), avoiding a huge preallocation from a bogus count. */
static size_t section_capacity(size_t len, size_t cursor, uint16_t count,
                               size_t min_entry_len) {
    size_t possible = (len - cursor) / min_entry_len;
    size_t c = count;
    return c < possible ? c : possible;
}

static DnsError parse_one_question(const uint8_t *input, size_t len,
                                   size_t *cursor, DnsQuestion *out) {
    uint16_t t, cl;
    DnsError e;
    memset(out, 0, sizeof *out);
    e = read_name(input, len, cursor, &out->name);
    if (e.kind != DNS_OK) return e;
    if ((e = read_u16(input, len, cursor, &t)).kind != DNS_OK) {
        dns_name_free(&out->name);
        return e;
    }
    if ((e = read_u16(input, len, cursor, &cl)).kind != DNS_OK) {
        dns_name_free(&out->name);
        return e;
    }
    out->qtype = rtype_from_u16(t);
    out->qclass = class_from_u16(cl);
    return ok_err();
}

static DnsError parse_questions(const uint8_t *input, size_t len,
                                size_t *cursor, uint16_t count,
                                DnsQuestion **out, size_t *out_n) {
    size_t capacity =
        section_capacity(len, *cursor, count, MIN_QUESTION_WIRE_LEN);
    DnsQuestion *arr = NULL;
    size_t n = 0;
    uint16_t i;
    *out = NULL;
    *out_n = 0;
    if (capacity > 0) {
        arr = (DnsQuestion *)calloc(capacity, sizeof(DnsQuestion));
        if (arr == NULL) return err_kind(DNS_ERR_OUT_OF_MEMORY);
    }
    for (i = 0; i < count; i++) {
        DnsQuestion q;
        DnsError e = parse_one_question(input, len, cursor, &q);
        if (e.kind != DNS_OK) {
            while (n > 0) dns_name_free(&arr[--n].name);
            free(arr);
            return e;
        }
        if (n == capacity) {
            size_t nc = capacity ? capacity * 2 : 1;
            DnsQuestion *na =
                (DnsQuestion *)realloc(arr, nc * sizeof(DnsQuestion));
            if (na == NULL) {
                dns_name_free(&q.name);
                while (n > 0) dns_name_free(&arr[--n].name);
                free(arr);
                return err_kind(DNS_ERR_OUT_OF_MEMORY);
            }
            arr = na;
            capacity = nc;
        }
        arr[n++] = q;
    }
    *out = arr;
    *out_n = n;
    return ok_err();
}

static DnsError read_single_rdata_name(const uint8_t *input, size_t len,
                                       size_t rdata_start, size_t rdata_end,
                                       const char *trailing_error,
                                       DnsName *out) {
    size_t dc = rdata_start;
    DnsError e = read_name(input, len, &dc, out);
    if (e.kind != DNS_OK) return e;
    if (dc > rdata_end) {
        dns_name_free(out);
        return err_kind(DNS_ERR_UNEXPECTED_EOF);
    }
    if (dc != rdata_end) {
        dns_name_free(out);
        return err_unsupported(trailing_error);
    }
    return ok_err();
}

static DnsError parse_one_record(const uint8_t *input, size_t len,
                                 size_t *cursor, DnsResourceRecord *out) {
    uint16_t t, cl, rdlen16;
    uint32_t ttl;
    size_t rdlength, rdata_start, rdata_end;
    DnsError e;

    memset(out, 0, sizeof *out);
    e = read_name(input, len, cursor, &out->name);
    if (e.kind != DNS_OK) return e;
    if ((e = read_u16(input, len, cursor, &t)).kind != DNS_OK) goto fail_name;
    if ((e = read_u16(input, len, cursor, &cl)).kind != DNS_OK) goto fail_name;
    if ((e = read_u32(input, len, cursor, &ttl)).kind != DNS_OK) goto fail_name;
    if ((e = read_u16(input, len, cursor, &rdlen16)).kind != DNS_OK)
        goto fail_name;
    rdlength = rdlen16;
    if (len - *cursor < rdlength) {
        e = err_kind(DNS_ERR_UNEXPECTED_EOF);
        goto fail_name;
    }
    rdata_start = *cursor;
    rdata_end = rdata_start + rdlength;
    out->rrtype = rtype_from_u16(t);
    out->class_ = class_from_u16(cl);
    out->ttl = ttl;

    switch (out->rrtype.kind) {
        case DNS_TYPE_A:
            if (rdlength != 4) {
                e = err_unsupported("A record data must be 4 bytes");
                goto fail_name;
            }
            out->data.kind = DNS_RDATA_A;
            memcpy(out->data.a, input + rdata_start, 4);
            break;
        case DNS_TYPE_AAAA:
            if (rdlength != 16) {
                e = err_unsupported("AAAA record data must be 16 bytes");
                goto fail_name;
            }
            out->data.kind = DNS_RDATA_AAAA;
            memcpy(out->data.aaaa, input + rdata_start, 16);
            break;
        case DNS_TYPE_CNAME:
            out->data.kind = DNS_RDATA_CNAME;
            e = read_single_rdata_name(
                input, len, rdata_start, rdata_end,
                "CNAME record data must contain exactly one DNS name",
                &out->data.name);
            if (e.kind != DNS_OK) goto fail_name;
            break;
        case DNS_TYPE_PTR:
            out->data.kind = DNS_RDATA_PTR;
            e = read_single_rdata_name(
                input, len, rdata_start, rdata_end,
                "PTR record data must contain exactly one DNS name",
                &out->data.name);
            if (e.kind != DNS_OK) goto fail_name;
            break;
        case DNS_TYPE_SRV: {
            size_t dc = rdata_start;
            uint16_t pr, we, po;
            if (rdlength < 7) {
                e = err_unsupported(
                    "SRV record data must contain priority, weight, port, and "
                    "target");
                goto fail_name;
            }
            out->data.kind = DNS_RDATA_SRV;
            if ((e = read_u16(input, len, &dc, &pr)).kind != DNS_OK)
                goto fail_name;
            if ((e = read_u16(input, len, &dc, &we)).kind != DNS_OK)
                goto fail_name;
            if ((e = read_u16(input, len, &dc, &po)).kind != DNS_OK)
                goto fail_name;
            out->data.srv.priority = pr;
            out->data.srv.weight = we;
            out->data.srv.port = po;
            e = read_name(input, len, &dc, &out->data.srv.target);
            if (e.kind != DNS_OK) goto fail_name;
            if (dc > rdata_end) {
                dns_name_free(&out->data.srv.target);
                e = err_kind(DNS_ERR_UNEXPECTED_EOF);
                goto fail_name;
            }
            if (dc != rdata_end) {
                dns_name_free(&out->data.srv.target);
                e = err_unsupported(
                    "SRV record data must contain exactly priority, weight, "
                    "port, and target");
                goto fail_name;
            }
            break;
        }
        default:
            out->data.kind = DNS_RDATA_RAW;
            out->data.raw_len = rdlength;
            if (rdlength > 0) {
                out->data.raw = (uint8_t *)malloc(rdlength);
                if (out->data.raw == NULL) {
                    e = err_kind(DNS_ERR_OUT_OF_MEMORY);
                    goto fail_name;
                }
                memcpy(out->data.raw, input + rdata_start, rdlength);
            } else {
                out->data.raw = NULL;
            }
            break;
    }

    *cursor = rdata_end;
    return ok_err();

fail_name:
    dns_name_free(&out->name);
    return e;
}

static DnsError parse_records(const uint8_t *input, size_t len, size_t *cursor,
                              uint16_t count, DnsResourceRecord **out,
                              size_t *out_n) {
    size_t capacity =
        section_capacity(len, *cursor, count, MIN_RECORD_WIRE_LEN);
    DnsResourceRecord *arr = NULL;
    size_t n = 0;
    uint16_t i;
    *out = NULL;
    *out_n = 0;
    if (capacity > 0) {
        arr = (DnsResourceRecord *)calloc(capacity, sizeof(DnsResourceRecord));
        if (arr == NULL) return err_kind(DNS_ERR_OUT_OF_MEMORY);
    }
    for (i = 0; i < count; i++) {
        DnsResourceRecord r;
        DnsError e = parse_one_record(input, len, cursor, &r);
        if (e.kind != DNS_OK) {
            while (n > 0) record_free(&arr[--n]);
            free(arr);
            return e;
        }
        if (n == capacity) {
            size_t nc = capacity ? capacity * 2 : 1;
            DnsResourceRecord *na = (DnsResourceRecord *)realloc(
                arr, nc * sizeof(DnsResourceRecord));
            if (na == NULL) {
                record_free(&r);
                while (n > 0) record_free(&arr[--n]);
                free(arr);
                return err_kind(DNS_ERR_OUT_OF_MEMORY);
            }
            arr = na;
            capacity = nc;
        }
        arr[n++] = r;
    }
    *out = arr;
    *out_n = n;
    return ok_err();
}

DnsError dns_parse_message(const uint8_t *input, size_t len, DnsMessage *out) {
    size_t cursor = 0;
    DnsError e;
    uint16_t id, flagword;

    memset(out, 0, sizeof *out);
    if (len < DNS_HEADER_LEN) return err_kind(DNS_ERR_TRUNCATED_HEADER);

    if ((e = read_u16(input, len, &cursor, &id)).kind != DNS_OK) return e;
    if ((e = read_u16(input, len, &cursor, &flagword)).kind != DNS_OK) return e;
    out->header.id = id;
    out->header.flags = flags_parse(flagword);
    if ((e = read_u16(input, len, &cursor, &out->header.question_count)).kind !=
        DNS_OK)
        return e;
    if ((e = read_u16(input, len, &cursor, &out->header.answer_count)).kind !=
        DNS_OK)
        return e;
    if ((e = read_u16(input, len, &cursor, &out->header.authority_count))
            .kind != DNS_OK)
        return e;
    if ((e = read_u16(input, len, &cursor, &out->header.additional_count))
            .kind != DNS_OK)
        return e;

    e = parse_questions(input, len, &cursor, out->header.question_count,
                        &out->questions, &out->n_questions);
    if (e.kind != DNS_OK) goto fail;
    e = parse_records(input, len, &cursor, out->header.answer_count,
                      &out->answers, &out->n_answers);
    if (e.kind != DNS_OK) goto fail;
    e = parse_records(input, len, &cursor, out->header.authority_count,
                      &out->authorities, &out->n_authorities);
    if (e.kind != DNS_OK) goto fail;
    e = parse_records(input, len, &cursor, out->header.additional_count,
                      &out->additionals, &out->n_additionals);
    if (e.kind != DNS_OK) goto fail;

    return ok_err();

fail:
    dns_message_free(out);
    return e;
}

/* ── Serialization ─────────────────────────────────────────────────────────*/

static DnsError write_name(Buf *b, const DnsName *name) {
    size_t i;
    DnsError e = validate_encoded_name_len(name);
    if (e.kind != DNS_OK) return e;
    for (i = 0; i < name->n_labels; i++) {
        size_t ll = strlen(name->labels[i]);
        e = validate_label((const uint8_t *)name->labels[i], ll);
        if (e.kind != DNS_OK) return e;
        buf_push(b, (uint8_t)ll);
        buf_extend(b, (const uint8_t *)name->labels[i], ll);
    }
    buf_push(b, 0);
    return ok_err();
}

static DnsError write_record(Buf *out, const DnsResourceRecord *r) {
    Buf data;
    DnsError e;
    buf_init(&data);

    e = write_name(out, &r->name);
    if (e.kind != DNS_OK) return e;
    buf_u16(out, dns_record_type_to_u16(r->rrtype));
    buf_u16(out, class_to_u16(r->class_));
    buf_u32(out, r->ttl);

    switch (r->data.kind) {
        case DNS_RDATA_A: buf_extend(&data, r->data.a, 4); break;
        case DNS_RDATA_AAAA: buf_extend(&data, r->data.aaaa, 16); break;
        case DNS_RDATA_CNAME:
        case DNS_RDATA_PTR:
            e = write_name(&data, &r->data.name);
            if (e.kind != DNS_OK) {
                buf_free(&data);
                return e;
            }
            break;
        case DNS_RDATA_SRV:
            buf_u16(&data, r->data.srv.priority);
            buf_u16(&data, r->data.srv.weight);
            buf_u16(&data, r->data.srv.port);
            e = write_name(&data, &r->data.srv.target);
            if (e.kind != DNS_OK) {
                buf_free(&data);
                return e;
            }
            break;
        case DNS_RDATA_RAW:
            buf_extend(&data, r->data.raw, r->data.raw_len);
            break;
    }

    if (data.oom) {
        buf_free(&data);
        return err_kind(DNS_ERR_OUT_OF_MEMORY);
    }
    if (data.len > 0xffff) {
        buf_free(&data);
        return err_unsupported("record data too large");
    }
    buf_u16(out, (uint16_t)data.len);
    buf_extend(out, data.data, data.len);
    buf_free(&data);
    return ok_err();
}

static DnsError write_records(Buf *out, const DnsResourceRecord *arr,
                              size_t n) {
    size_t i;
    for (i = 0; i < n; i++) {
        DnsError e = write_record(out, arr + i);
        if (e.kind != DNS_OK) return e;
    }
    return ok_err();
}

DnsError dns_serialize_message(const DnsMessage *m, uint8_t **out_bytes,
                               size_t *out_len) {
    Buf out;
    DnsError e;
    size_t i;

    *out_bytes = NULL;
    *out_len = 0;

    if (m->n_questions > 0xffff || m->n_answers > 0xffff ||
        m->n_authorities > 0xffff || m->n_additionals > 0xffff)
        return err_kind(DNS_ERR_INVALID_SECTION_COUNT);

    buf_init(&out);
    buf_u16(&out, m->header.id);
    buf_u16(&out, flags_serialize(&m->header.flags));
    buf_u16(&out, (uint16_t)m->n_questions);
    buf_u16(&out, (uint16_t)m->n_answers);
    buf_u16(&out, (uint16_t)m->n_authorities);
    buf_u16(&out, (uint16_t)m->n_additionals);

    for (i = 0; i < m->n_questions; i++) {
        e = write_name(&out, &m->questions[i].name);
        if (e.kind != DNS_OK) {
            buf_free(&out);
            return e;
        }
        buf_u16(&out, dns_record_type_to_u16(m->questions[i].qtype));
        buf_u16(&out, class_to_u16(m->questions[i].qclass));
    }

    if ((e = write_records(&out, m->answers, m->n_answers)).kind != DNS_OK ||
        (e = write_records(&out, m->authorities, m->n_authorities)).kind !=
            DNS_OK ||
        (e = write_records(&out, m->additionals, m->n_additionals)).kind !=
            DNS_OK) {
        buf_free(&out);
        return e;
    }

    if (out.oom) {
        buf_free(&out);
        return err_kind(DNS_ERR_OUT_OF_MEMORY);
    }
    *out_bytes = out.data;
    *out_len = out.len;
    return ok_err();
}

/* ── build_query ───────────────────────────────────────────────────────────*/

DnsError dns_build_query(uint16_t id, DnsName name, DnsRecordType qtype,
                         DnsMessage *out) {
    memset(out, 0, sizeof *out);
    out->questions = (DnsQuestion *)calloc(1, sizeof(DnsQuestion));
    if (out->questions == NULL) {
        dns_name_free(&name);
        return err_kind(DNS_ERR_OUT_OF_MEMORY);
    }
    out->header.id = id;
    out->header.flags = dns_flags_query();
    out->header.question_count = 1;
    out->questions[0].name = name; /* takes ownership */
    out->questions[0].qtype = qtype;
    out->questions[0].qclass = class_from_u16(1); /* IN */
    out->n_questions = 1;
    return ok_err();
}

/* ── Accessors ─────────────────────────────────────────────────────────────*/

int dns_message_is_success(const DnsMessage *m) {
    return m->header.flags.is_response &&
           m->header.flags.response_code.kind == DNS_RCODE_NO_ERROR;
}

const DnsResourceRecord *dns_message_first_answer_of_type(const DnsMessage *m,
                                                          DnsRecordType qtype) {
    size_t i;
    for (i = 0; i < m->n_answers; i++)
        if (dns_record_type_equal(m->answers[i].rrtype, qtype))
            return &m->answers[i];
    return NULL;
}

size_t dns_message_ipv4_answers(const DnsMessage *m, uint8_t (*out)[4],
                                size_t cap) {
    size_t i, n = 0;
    for (i = 0; i < m->n_answers; i++) {
        if (m->answers[i].data.kind == DNS_RDATA_A) {
            if (out != NULL && n < cap) memcpy(out[n], m->answers[i].data.a, 4);
            n++;
        }
    }
    return n;
}

size_t dns_message_ipv6_answers(const DnsMessage *m, uint8_t (*out)[16],
                                size_t cap) {
    size_t i, n = 0;
    for (i = 0; i < m->n_answers; i++) {
        if (m->answers[i].data.kind == DNS_RDATA_AAAA) {
            if (out != NULL && n < cap)
                memcpy(out[n], m->answers[i].data.aaaa, 16);
            n++;
        }
    }
    return n;
}
