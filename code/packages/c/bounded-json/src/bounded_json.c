/* The parser owns an arena of individually bounded allocations. Partial trees
 * need no recursive error cleanup: release walks the arena, not input shape.
 * Nodes use linked members, avoiding realloc/overflow and quadratic copying. */
#include "bounded_json.h"
#include <float.h>
#include <limits.h>
#include <locale.h>
#include <stdlib.h>
#include <string.h>

typedef union Block Block;
/* Payloads contain only pointers, size_t, int64_t and double (or bytes).
 * Align for those exact types. MSVC's C17 headers do not expose max_align_t. */
union Block {
    struct { Block *next; } link;
    void *pointer_alignment;
    size_t size_alignment;
    int64_t integer_alignment;
    double number_alignment;
};
typedef struct { char *data; size_t length; } Text;
struct BJsonValue {
    BJsonKind kind;
    struct BJsonValue *next, *first, *last;
    size_t count;
    Text text, key;
    int boolean;
    int64_t integer;
    double number;
};
struct BJsonDoc {
    BJsonOptions options;
    Block *blocks;
    BJsonValue *root;
    size_t allocated, nodes;
};
typedef struct {
    const unsigned char *input;
    size_t length, position;
    BJsonDoc *doc;
    BJsonError error;
} Parser;

static void *system_alloc(size_t n, void *context) { (void)context; return malloc(n); }
static void system_free(void *p, void *context) { (void)context; free(p); }
BJsonOptions bj_options(void) {
    BJsonOptions o = {BJ_MAX_BYTES, BJ_MAX_DEPTH, BJ_MAX_NODES, BJ_MAX_ALLOC,
                      system_alloc, system_free, NULL};
    return o;
}
static void fail(Parser *p, BJsonStatus kind) {
    if (p->error.kind == BJ_OK) {
        p->error.kind = kind;
        p->error.offset = p->position;
    }
}
static void *owned(Parser *p, size_t bytes) {
    BJsonDoc *d = p->doc;
    Block *b;
    /* Subtraction before addition protects both the payload/header sum and
     * the cumulative allocation budget, even on 32-bit size_t. */
    if (bytes > d->options.max_alloc_bytes - d->allocated ||
        sizeof(Block) > d->options.max_alloc_bytes - d->allocated - bytes) {
        fail(p, BJ_MEMORY_LIMIT); return NULL;
    }
    b = (Block *)d->options.allocate(sizeof(Block) + bytes, d->options.context);
    if (!b) { fail(p, BJ_ALLOC); return NULL; }
    b->link.next = d->blocks;
    d->blocks = b;
    d->allocated += sizeof(Block) + bytes;
    memset(b + 1, 0, bytes);
    return b + 1;
}
void bj_doc_free(BJsonDoc **document) {
    if (document && *document) {
        BJsonDoc *d = *document;
        Block *b = d->blocks;
        BJsonOptions o = d->options;
        while (b) { Block *next = b->link.next; o.release(b, o.context); b = next; }
        o.release(d, o.context);
        *document = NULL;
    }
}
static BJsonValue *node(Parser *p) {
    BJsonValue *v;
    if (p->doc->nodes == p->doc->options.max_nodes) {
        fail(p, BJ_NODE_LIMIT); return NULL;
    }
    v = (BJsonValue *)owned(p, sizeof(*v));
    if (v) p->doc->nodes++;
    return v;
}
static void whitespace(Parser *p) {
    while (p->position < p->length) {
        unsigned char c = p->input[p->position];
        if (c != ' ' && c != '\t' && c != '\r' && c != '\n') break;
        p->position++;
    }
}
static int take(Parser *p, unsigned char c) {
    if (p->position < p->length && p->input[p->position] == c) {
        p->position++; return 1;
    }
    return 0;
}
static int hex4(Parser *p, uint32_t *out) {
    uint32_t value = 0;
    size_t i;
    for (i = 0; i < 4; i++) {
        unsigned char c;
        unsigned digit;
        if (p->position == p->length) { fail(p, BJ_SYNTAX); return 0; }
        c = p->input[p->position++];
        if (c >= '0' && c <= '9') digit = (unsigned)(c - '0');
        else if (c >= 'a' && c <= 'f') digit = (unsigned)(c - 'a') + 10u;
        else if (c >= 'A' && c <= 'F') digit = (unsigned)(c - 'A') + 10u;
        else { fail(p, BJ_SYNTAX); return 0; }
        value = value * 16u + digit;
    }
    *out = value; return 1;
}
static void emit(char *out, size_t *n, unsigned char c) {
    if (out) out[*n] = (char)c;
    (*n)++;
}
static void encode(char *out, size_t *n, uint32_t c) {
    if (c < 0x80u) emit(out, n, (unsigned char)c);
    else if (c < 0x800u) {
        emit(out, n, (unsigned char)(0xc0u | (c >> 6)));
        emit(out, n, (unsigned char)(0x80u | (c & 63u)));
    } else if (c < 0x10000u) {
        emit(out, n, (unsigned char)(0xe0u | (c >> 12)));
        emit(out, n, (unsigned char)(0x80u | ((c >> 6) & 63u)));
        emit(out, n, (unsigned char)(0x80u | (c & 63u)));
    } else {
        emit(out, n, (unsigned char)(0xf0u | (c >> 18)));
        emit(out, n, (unsigned char)(0x80u | ((c >> 12) & 63u)));
        emit(out, n, (unsigned char)(0x80u | ((c >> 6) & 63u)));
        emit(out, n, (unsigned char)(0x80u | (c & 63u)));
    }
}
/* First pass validates/counts; second pass fills an exact owned buffer. UTF-8
 * bytes are validated as code points before copying, never accepted by prefix. */
static int string_pass(Parser *p, char *out, size_t *length) {
    size_t n = 0;
    if (!take(p, '"')) { fail(p, BJ_SYNTAX); return 0; }
    while (p->position < p->length) {
        unsigned char c = p->input[p->position++];
        if (c == '"') { *length = n; return 1; }
        if (c < 0x20u) { fail(p, BJ_SYNTAX); return 0; }
        if (c == '\\') {
            uint32_t value;
            if (p->position == p->length) { fail(p, BJ_SYNTAX); return 0; }
            c = p->input[p->position++];
            if (c == '"' || c == '\\' || c == '/') emit(out, &n, c);
            else if (c == 'b') emit(out, &n, '\b');
            else if (c == 'f') emit(out, &n, '\f');
            else if (c == 'n') emit(out, &n, '\n');
            else if (c == 'r') emit(out, &n, '\r');
            else if (c == 't') emit(out, &n, '\t');
            else if (c == 'u') {
                if (!hex4(p, &value)) return 0;
                if (value >= 0xd800u && value <= 0xdbffu) {
                    uint32_t low;
                    if (!take(p, '\\') || !take(p, 'u')) { fail(p, BJ_UTF8); return 0; }
                    if (!hex4(p, &low)) return 0;
                    if (low < 0xdc00u || low > 0xdfffu) { fail(p, BJ_UTF8); return 0; }
                    value = 0x10000u + ((value - 0xd800u) << 10) + low - 0xdc00u;
                } else if (value >= 0xdc00u && value <= 0xdfffu) {
                    fail(p, BJ_UTF8); return 0;
                }
                encode(out, &n, value);
            } else { fail(p, BJ_SYNTAX); return 0; }
        } else if (c < 0x80u) emit(out, &n, c);
        else {
            unsigned count, i;
            uint32_t value, minimum;
            if (c >= 0xc2u && c <= 0xdfu) { count = 1; value = c & 31u; minimum = 0x80u; }
            else if (c >= 0xe0u && c <= 0xefu) { count = 2; value = c & 15u; minimum = 0x800u; }
            else if (c >= 0xf0u && c <= 0xf4u) { count = 3; value = c & 7u; minimum = 0x10000u; }
            else { fail(p, BJ_UTF8); return 0; }
            for (i = 0; i < count; i++) {
                if (p->position == p->length) { fail(p, BJ_UTF8); return 0; }
                c = p->input[p->position++];
                if ((c & 0xc0u) != 0x80u) { fail(p, BJ_UTF8); return 0; }
                value = (value << 6) | (c & 63u);
            }
            if (value < minimum || value > 0x10ffffu ||
                (value >= 0xd800u && value <= 0xdfffu)) { fail(p, BJ_UTF8); return 0; }
            encode(out, &n, value);
        }
    }
    fail(p, BJ_SYNTAX); return 0;
}
static int parse_string(Parser *p, Text *text) {
    size_t start = p->position, length;
    if (!string_pass(p, NULL, &length)) return 0;
    text->data = (char *)owned(p, length + 1);
    if (!text->data) return 0;
    p->position = start;
    if (!string_pass(p, text->data, &text->length)) return 0;
    text->data[length] = '\0';
    return 1;
}
static int digit(unsigned char c) { return c >= '0' && c <= '9'; }
static int parse_number(Parser *p, BJsonValue *v) {
    size_t start = p->position, end, i, point = 0, decimal_length;
    int negative = take(p, '-'), integral = 1, fits = 1;
    uint64_t exact = 0, limit = (uint64_t)INT64_MAX + (negative ? 1u : 0u);
    const char *decimal;
    char *token, *tail;
    if (take(p, '0')) { /* A following digit is rejected by the outer grammar. */ }
    else {
        if (p->position == p->length || !digit(p->input[p->position])) { fail(p, BJ_SYNTAX); return 0; }
        while (p->position < p->length && digit(p->input[p->position])) p->position++;
    }
    if (take(p, '.')) {
        integral = 0; point = p->position - 1;
        if (p->position == p->length || !digit(p->input[p->position])) { fail(p, BJ_SYNTAX); return 0; }
        while (p->position < p->length && digit(p->input[p->position])) p->position++;
    }
    if (take(p, 'e') || take(p, 'E')) {
        integral = 0;
        if (!take(p, '+')) (void)take(p, '-');
        if (p->position == p->length || !digit(p->input[p->position])) { fail(p, BJ_SYNTAX); return 0; }
        while (p->position < p->length && digit(p->input[p->position])) p->position++;
    }
    end = p->position;
    if (integral) {
        for (i = start + (negative ? 1u : 0u); i < end; i++) {
            unsigned d = (unsigned)(p->input[i] - '0');
            if (exact > (limit - d) / 10u) { fits = 0; break; }
            exact = exact * 10u + d;
        }
        if (fits) {
            v->kind = BJ_INTEGER;
            v->integer = negative ? (exact == (uint64_t)INT64_MAX + 1u ? INT64_MIN : -(int64_t)exact) : (int64_t)exact;
            v->number = (double)v->integer;
            return 1;
        }
    }
    /* strtod obeys LC_NUMERIC. Translate the already-validated JSON decimal
     * point into the current locale spelling; never call setlocale here.
     * This also preserves the C runtime's correctly rounded double conversion.
     * Concurrent global setlocale by the application requires external locking. */
    decimal = localeconv()->decimal_point;
    decimal_length = strlen(decimal);
    if (!decimal_length || decimal_length > 16) { fail(p, BJ_NUMBER); return 0; }
    token = (char *)owned(p, end - start + decimal_length + 1);
    if (!token) return 0;
    if (point) {
        size_t prefix = point - start;
        memcpy(token, p->input + start, prefix);
        memcpy(token + prefix, decimal, decimal_length);
        memcpy(token + prefix + decimal_length, p->input + point + 1, end - point - 1);
        token[end - start + decimal_length - 1] = '\0';
    } else {
        memcpy(token, p->input + start, end - start); token[end - start] = '\0';
    }
    v->number = strtod(token, &tail);
    if (*tail || v->number != v->number || v->number > DBL_MAX || v->number < -DBL_MAX) {
        fail(p, BJ_NUMBER); return 0;
    }
    v->kind = BJ_FLOAT; return 1;
}
static BJsonValue *value(Parser *p, size_t depth) {
    BJsonValue *v;
    unsigned char c;
    whitespace(p);
    if (p->position == p->length) { fail(p, BJ_SYNTAX); return NULL; }
    c = p->input[p->position];
    v = node(p);
    if (!v) return NULL;
    if (c == '"') {
        v->kind = BJ_STRING;
        return parse_string(p, &v->text) ? v : NULL;
    }
    if (c == '-' || digit(c)) return parse_number(p, v) ? v : NULL;
    if (c == '[' || c == '{') {
        unsigned char close = (unsigned char)((c == '[') ? ']' : '}');
        if (depth == p->doc->options.max_depth) { fail(p, BJ_DEPTH_LIMIT); return NULL; }
        v->kind = (c == '[') ? BJ_ARRAY : BJ_OBJECT;
        p->position++; whitespace(p);
        if (take(p, close)) return v;
        for (;;) {
            Text key = {NULL, 0};
            BJsonValue *child;
            if (v->kind == BJ_OBJECT) {
                if (!parse_string(p, &key)) return NULL;
                whitespace(p);
                if (!take(p, ':')) { fail(p, BJ_SYNTAX); return NULL; }
            }
            child = value(p, depth + 1);
            if (!child) return NULL;
            child->key = key;
            if (v->last) v->last->next = child; else v->first = child;
            v->last = child; v->count++;
            whitespace(p);
            if (take(p, close)) return v;
            if (!take(p, ',')) { fail(p, BJ_SYNTAX); return NULL; }
            whitespace(p);
        }
    }
    {
        const char *literal;
        size_t n;
        if (c == 't') { literal = "true"; n = 4; v->kind = BJ_BOOL; v->boolean = 1; }
        else if (c == 'f') { literal = "false"; n = 5; v->kind = BJ_BOOL; }
        else if (c == 'n') { literal = "null"; n = 4; v->kind = BJ_NULL; }
        else { fail(p, BJ_SYNTAX); return NULL; }
        if (n > p->length - p->position || memcmp(p->input + p->position, literal, n)) { fail(p, BJ_SYNTAX); return NULL; }
        p->position += n; return v;
    }
}
BJsonStatus bj_parse_with_options(const char *input, size_t length,
    const BJsonOptions *options, BJsonDoc **out, BJsonError *error) {
    BJsonOptions o = options ? *options : bj_options();
    Parser p;
    BJsonDoc *d;
    BJsonStatus status = BJ_OK;
    if (error) { error->kind = BJ_OK; error->offset = 0; }
    if (!input || !out || *out || !o.allocate || !o.release ||
        !o.max_bytes || o.max_bytes > BJ_MAX_BYTES || o.max_depth > BJ_MAX_DEPTH ||
        !o.max_nodes || o.max_nodes > BJ_MAX_NODES ||
        o.max_alloc_bytes < sizeof(BJsonDoc) || o.max_alloc_bytes > BJ_MAX_ALLOC) status = BJ_ARGUMENT;
    else if (length > o.max_bytes) status = BJ_BYTE_LIMIT;
    if (status != BJ_OK) { if (error) error->kind = status; return status; }
    d = (BJsonDoc *)o.allocate(sizeof(*d), o.context);
    if (!d) { if (error) error->kind = BJ_ALLOC; return BJ_ALLOC; }
    memset(d, 0, sizeof(*d));
    d->options = o; d->allocated = sizeof(*d);
    memset(&p, 0, sizeof(p));
    p.input = (const unsigned char *)input; p.length = length; p.doc = d;
    d->root = value(&p, 0);
    whitespace(&p);
    if (p.error.kind == BJ_OK && p.position != length) fail(&p, BJ_SYNTAX);
    if (p.error.kind != BJ_OK) {
        status = p.error.kind;
        if (error) *error = p.error;
        bj_doc_free(&d);
    } else *out = d;
    return status;
}
BJsonStatus bj_parse(const char *input, size_t length, BJsonDoc **out, BJsonError *error) {
    return bj_parse_with_options(input, length, NULL, out, error);
}
const BJsonValue *bj_root(const BJsonDoc *d) { return d ? d->root : NULL; }
BJsonKind bj_kind(const BJsonValue *v) { return v ? v->kind : BJ_INVALID; }
int bj_boolean(const BJsonValue *v, int *out) {
    if (!v || !out || v->kind != BJ_BOOL) return 0;
    *out = v->boolean; return 1;
}
int bj_integer(const BJsonValue *v, int64_t *out) {
    if (!v || !out || v->kind != BJ_INTEGER) return 0;
    *out = v->integer; return 1;
}
int bj_number(const BJsonValue *v, double *out) {
    if (!v || !out || (v->kind != BJ_INTEGER && v->kind != BJ_FLOAT)) return 0;
    *out = v->number; return 1;
}
int bj_string(const BJsonValue *v, const char **out, size_t *length) {
    if (!v || !out || !length || v->kind != BJ_STRING) return 0;
    *out = v->text.data; *length = v->text.length; return 1;
}
size_t bj_size(const BJsonValue *v) {
    return v && (v->kind == BJ_ARRAY || v->kind == BJ_OBJECT) ? v->count : 0;
}
static const BJsonValue *at(const BJsonValue *v, size_t index) {
    const BJsonValue *child = v->first;
    while (child && index) { child = child->next; index--; }
    return child;
}
const BJsonValue *bj_at(const BJsonValue *v, size_t index) {
    return v && v->kind == BJ_ARRAY ? at(v, index) : NULL;
}
int bj_member(const BJsonValue *v, size_t index, const char **name,
              size_t *length, const BJsonValue **out) {
    const BJsonValue *child;
    if (!v || v->kind != BJ_OBJECT || !name || !length || !out) return 0;
    child = at(v, index);
    if (!child) return 0;
    *name = child->key.data; *length = child->key.length; *out = child;
    return 1;
}
