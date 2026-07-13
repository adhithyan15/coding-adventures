/*
 * format_doc.c — implementation of the pure-ISO C document algebra.
 * ================================================================
 *
 * The `FdDoc` tree is immutable and caller-owned; `fd_layout_doc` walks it with
 * an explicit command stack (indent level + flat/broken mode + active
 * annotations), deciding each group via a `fits` look-ahead that borrows the
 * parent stack rather than cloning it. Active annotations are threaded through
 * the interpreter as an immutable cons-list in a per-run arena, so commands
 * stay trivially copyable and only emitted spans pay for a materialised copy.
 */
#include "format_doc.h"

#include <stdlib.h> /* malloc, realloc, free, calloc */
#include <string.h> /* memcpy, memchr, strlen, strcmp */

/* ── Annotations ──────────────────────────────────────────────────────────*/

static char *str_dup(const char *s) {
    size_t n = strlen(s) + 1;
    char *p = (char *)malloc(n);
    if (p != NULL) memcpy(p, s, n);
    return p;
}

FdAnnotation fd_ann_str(const char *s) {
    FdAnnotation a = {FD_ANN_STR, NULL, 0, 0};
    a.str = str_dup(s);
    return a;
}
FdAnnotation fd_ann_int(int64_t v) {
    FdAnnotation a = {FD_ANN_INT, NULL, v, 0};
    return a;
}
FdAnnotation fd_ann_bool(int v) {
    FdAnnotation a = {FD_ANN_BOOL, NULL, 0, v ? 1 : 0};
    return a;
}
FdAnnotation fd_ann_null(void) {
    FdAnnotation a = {FD_ANN_NULL, NULL, 0, 0};
    return a;
}
void fd_ann_free(FdAnnotation *a) {
    if (a != NULL && a->kind == FD_ANN_STR) {
        free(a->str);
        a->str = NULL;
    }
}
int fd_ann_equal(const FdAnnotation *a, const FdAnnotation *b) {
    if (a->kind != b->kind) return 0;
    switch (a->kind) {
        case FD_ANN_STR:
            return strcmp(a->str ? a->str : "", b->str ? b->str : "") == 0;
        case FD_ANN_INT:
            return a->i == b->i;
        case FD_ANN_BOOL:
            return a->b == b->b;
        case FD_ANN_NULL:
            return 1;
    }
    return 0;
}
/* Deep-copy an annotation (dups an owned string). Returns 0 on OOM. */
static int ann_copy(const FdAnnotation *src, FdAnnotation *dst) {
    *dst = *src;
    if (src->kind == FD_ANN_STR) {
        dst->str = src->str ? str_dup(src->str) : NULL;
        if (src->str && dst->str == NULL) return 0;
    }
    return 1;
}

/* ── Document nodes ───────────────────────────────────────────────────────*/

typedef enum {
    FD_NIL,
    FD_TEXT,
    FD_CONCAT,
    FD_GROUP,
    FD_INDENT,
    FD_LINE,
    FD_IFBREAK,
    FD_ANNOTATE
} FdDocKind;

typedef enum { FD_LINE_SOFT, FD_LINE_NORMAL, FD_LINE_HARD } FdLineMode;

struct FdDoc {
    FdDocKind kind;
    union {
        char *text;
        struct {
            FdDoc **items;
            size_t len;
        } concat;
        FdDoc *child; /* group */
        struct {
            size_t levels;
            FdDoc *content;
        } indent;
        FdLineMode line_mode;
        struct {
            FdDoc *broken;
            FdDoc *flat;
        } ifbreak;
        struct {
            FdAnnotation ann;
            FdDoc *content;
        } annotate;
    } as;
};

static FdDoc *alloc_doc(FdDocKind kind) {
    FdDoc *d = (FdDoc *)calloc(1, sizeof(FdDoc));
    if (d != NULL) d->kind = kind;
    return d;
}

void fd_free(FdDoc *d) {
    if (d == NULL) return;
    switch (d->kind) {
        case FD_TEXT:
            free(d->as.text);
            break;
        case FD_CONCAT:
            for (size_t i = 0; i < d->as.concat.len; i++)
                fd_free(d->as.concat.items[i]);
            free(d->as.concat.items);
            break;
        case FD_GROUP:
            fd_free(d->as.child);
            break;
        case FD_INDENT:
            fd_free(d->as.indent.content);
            break;
        case FD_IFBREAK:
            fd_free(d->as.ifbreak.broken);
            fd_free(d->as.ifbreak.flat);
            break;
        case FD_ANNOTATE:
            fd_ann_free(&d->as.annotate.ann);
            fd_free(d->as.annotate.content);
            break;
        default:
            break; /* NIL, LINE own nothing */
    }
    free(d);
}

FdDoc *fd_nil(void) { return alloc_doc(FD_NIL); }

static FdDoc *make_line(FdLineMode m) {
    FdDoc *d = alloc_doc(FD_LINE);
    if (d != NULL) d->as.line_mode = m;
    return d;
}
FdDoc *fd_line(void) { return make_line(FD_LINE_NORMAL); }
FdDoc *fd_softline(void) { return make_line(FD_LINE_SOFT); }
FdDoc *fd_hardline(void) { return make_line(FD_LINE_HARD); }

static FdDoc *make_text(const char *s, size_t len) {
    FdDoc *d = alloc_doc(FD_TEXT);
    if (d == NULL) return NULL;
    d->as.text = (char *)malloc(len + 1);
    if (d->as.text == NULL) {
        free(d);
        return NULL;
    }
    memcpy(d->as.text, s, len);
    d->as.text[len] = '\0';
    return d;
}

/* Grow a doc pointer array by one; returns 0 on OOM. */
static int docvec_push(FdDoc ***arr, size_t *len, size_t *cap, FdDoc *v) {
    if (*len == *cap) {
        size_t nc = *cap ? *cap : 8;
        if (nc > ((size_t)-1) / 2 / sizeof(FdDoc *)) return 0;
        nc *= 2;
        FdDoc **p = (FdDoc **)realloc(*arr, nc * sizeof(FdDoc *));
        if (p == NULL) return 0;
        *arr = p;
        *cap = nc;
    }
    (*arr)[(*len)++] = v;
    return 1;
}

FdDoc *fd_text(const char *value) {
    size_t len = strlen(value);
    if (len == 0) return fd_nil();
    if (memchr(value, '\n', len) == NULL && memchr(value, '\r', len) == NULL)
        return make_text(value, len);

    /* Normalise CRLF / CR to LF into a scratch buffer, then split on LF. */
    char *norm = (char *)malloc(len + 1);
    if (norm == NULL) return NULL;
    size_t nn = 0;
    for (size_t i = 0; i < len; i++) {
        if (value[i] == '\r') {
            norm[nn++] = '\n';
            if (i + 1 < len && value[i + 1] == '\n') i++; /* CRLF -> one LF */
        } else {
            norm[nn++] = value[i];
        }
    }
    norm[nn] = '\0';

    FdDoc **parts = NULL;
    size_t plen = 0, pcap = 0;
    int ok = 1;
    size_t start = 0;
    int first = 1;
    for (size_t i = 0; i <= nn && ok; i++) {
        if (i == nn || norm[i] == '\n') {
            if (!first) {
                FdDoc *hl = fd_hardline();
                if (hl == NULL || !docvec_push(&parts, &plen, &pcap, hl)) {
                    fd_free(hl);
                    ok = 0;
                    break;
                }
            }
            first = 0;
            size_t piece_len = i - start;
            if (piece_len > 0) {
                FdDoc *t = make_text(norm + start, piece_len);
                if (t == NULL || !docvec_push(&parts, &plen, &pcap, t)) {
                    fd_free(t);
                    ok = 0;
                    break;
                }
            }
            start = i + 1;
        }
    }
    free(norm);
    if (!ok) {
        for (size_t i = 0; i < plen; i++) fd_free(parts[i]);
        free(parts);
        return NULL;
    }
    FdDoc *result = fd_concat(parts, plen);
    free(parts);
    return result;
}

FdDoc *fd_concat(FdDoc **parts, size_t n) {
    FdDoc **flat = NULL;
    size_t flen = 0, fcap = 0;
    int ok = 1;
    size_t i = 0;

    for (; i < n && ok; i++) {
        FdDoc *part = parts[i];
        if (part == NULL) continue;
        if (part->kind == FD_NIL) {
            fd_free(part);
        } else if (part->kind == FD_CONCAT) {
            /* Splice the inner concat's non-nil children into `flat`, then free
             * the (now-empty) concat shell. */
            size_t j = 0;
            for (; j < part->as.concat.len && ok; j++) {
                FdDoc *nested = part->as.concat.items[j];
                if (nested == NULL) continue;
                if (nested->kind == FD_NIL) {
                    fd_free(nested);
                } else if (!docvec_push(&flat, &flen, &fcap, nested)) {
                    fd_free(nested);
                    ok = 0;
                }
            }
            /* On OOM mid-splice, free the children we never moved into `flat`
             * (items[0..j) are owned by `flat`; items[j] was already freed). */
            for (size_t k = j; k < part->as.concat.len; k++)
                fd_free(part->as.concat.items[k]);
            free(part->as.concat.items);
            free(part);
        } else if (!docvec_push(&flat, &flen, &fcap, part)) {
            fd_free(part);
            ok = 0;
        }
    }

    if (!ok) {
        /* Honour the consume-all contract: free inputs never processed
         * (parts[i..n)). The loop's post-increment leaves `i` one past the
         * failing top-level part, which was already freed above. */
        for (size_t k = i; k < n; k++) fd_free(parts[k]);
        for (size_t k = 0; k < flen; k++) fd_free(flat[k]);
        free(flat);
        return NULL;
    }
    if (flen == 0) {
        free(flat);
        return fd_nil();
    }
    if (flen == 1) {
        FdDoc *only = flat[0];
        free(flat);
        return only;
    }
    FdDoc *d = alloc_doc(FD_CONCAT);
    if (d == NULL) {
        for (size_t i = 0; i < flen; i++) fd_free(flat[i]);
        free(flat);
        return NULL;
    }
    d->as.concat.items = flat;
    d->as.concat.len = flen;
    return d;
}

FdDoc *fd_group(FdDoc *content) {
    if (content == NULL) return NULL;
    FdDoc *d = alloc_doc(FD_GROUP);
    if (d == NULL) { fd_free(content); return NULL; }
    d->as.child = content;
    return d;
}

FdDoc *fd_indent(FdDoc *content, size_t levels) {
    if (content == NULL) return NULL;
    if (levels == 0) return content;
    FdDoc *d = alloc_doc(FD_INDENT);
    if (d == NULL) { fd_free(content); return NULL; }
    d->as.indent.levels = levels;
    d->as.indent.content = content;
    return d;
}

FdDoc *fd_if_break(FdDoc *broken, FdDoc *flat) {
    if (broken == NULL || flat == NULL) {
        fd_free(broken);
        fd_free(flat);
        return NULL;
    }
    FdDoc *d = alloc_doc(FD_IFBREAK);
    if (d == NULL) { fd_free(broken); fd_free(flat); return NULL; }
    d->as.ifbreak.broken = broken;
    d->as.ifbreak.flat = flat;
    return d;
}

FdDoc *fd_annotate(FdAnnotation annotation, FdDoc *content) {
    if (content == NULL) { fd_ann_free(&annotation); return NULL; }
    FdDoc *d = alloc_doc(FD_ANNOTATE);
    if (d == NULL) { fd_ann_free(&annotation); fd_free(content); return NULL; }
    d->as.annotate.ann = annotation;
    d->as.annotate.content = content;
    return d;
}

/* Deep-copy a doc tree (for fd_join's separator). Returns NULL on OOM. */
static FdDoc *fd_clone(const FdDoc *s) {
    if (s == NULL) return NULL;
    switch (s->kind) {
        case FD_NIL:
            return fd_nil();
        case FD_TEXT:
            return make_text(s->as.text, strlen(s->as.text));
        case FD_LINE:
            return make_line(s->as.line_mode);
        case FD_CONCAT: {
            FdDoc *d = alloc_doc(FD_CONCAT);
            if (d == NULL) return NULL;
            if (s->as.concat.len > 0) {
                d->as.concat.items =
                    (FdDoc **)calloc(s->as.concat.len, sizeof(FdDoc *));
                if (d->as.concat.items == NULL) { free(d); return NULL; }
            }
            d->as.concat.len = s->as.concat.len;
            for (size_t i = 0; i < s->as.concat.len; i++) {
                d->as.concat.items[i] = fd_clone(s->as.concat.items[i]);
                if (d->as.concat.items[i] == NULL) { fd_free(d); return NULL; }
            }
            return d;
        }
        case FD_GROUP:
            return fd_group(fd_clone(s->as.child));
        case FD_INDENT: {
            FdDoc *c = fd_clone(s->as.indent.content);
            if (c == NULL) return NULL;
            FdDoc *d = alloc_doc(FD_INDENT);
            if (d == NULL) { fd_free(c); return NULL; }
            d->as.indent.levels = s->as.indent.levels;
            d->as.indent.content = c;
            return d;
        }
        case FD_IFBREAK: {
            FdDoc *b = fd_clone(s->as.ifbreak.broken);
            FdDoc *f = fd_clone(s->as.ifbreak.flat);
            if (b == NULL || f == NULL) { fd_free(b); fd_free(f); return NULL; }
            FdDoc *d = alloc_doc(FD_IFBREAK);
            if (d == NULL) { fd_free(b); fd_free(f); return NULL; }
            d->as.ifbreak.broken = b;
            d->as.ifbreak.flat = f;
            return d;
        }
        case FD_ANNOTATE: {
            FdAnnotation a;
            if (!ann_copy(&s->as.annotate.ann, &a)) return NULL;
            FdDoc *c = fd_clone(s->as.annotate.content);
            if (c == NULL) { fd_ann_free(&a); return NULL; }
            FdDoc *d = alloc_doc(FD_ANNOTATE);
            if (d == NULL) { fd_ann_free(&a); fd_free(c); return NULL; }
            d->as.annotate.ann = a;
            d->as.annotate.content = c;
            return d;
        }
    }
    return NULL;
}

FdDoc *fd_join(FdDoc *separator, FdDoc **parts, size_t n) {
    if (n == 0) {
        fd_free(separator);
        return fd_nil();
    }
    FdDoc **out = NULL;
    size_t olen = 0, ocap = 0;
    int ok = 1;
    size_t i = 0;
    for (; i < n && ok; i++) {
        if (i > 0) {
            FdDoc *sep = fd_clone(separator);
            if (sep == NULL || !docvec_push(&out, &olen, &ocap, sep)) {
                fd_free(sep);
                ok = 0;
                break;
            }
        }
        /* Leave parts[i] unconsumed on failure so the tail cleanup frees it
         * exactly once (avoids a double free). */
        if (!docvec_push(&out, &olen, &ocap, parts[i])) {
            ok = 0;
            break;
        }
    }
    fd_free(separator);
    if (!ok) {
        /* Honour the consume-all contract: free inputs never moved into `out`
         * (parts[i..n)); `out` owns the earlier parts plus separator clones. */
        for (size_t k = i; k < n; k++) fd_free(parts[k]);
        for (size_t k = 0; k < olen; k++) fd_free(out[k]);
        free(out);
        return NULL;
    }
    FdDoc *result = fd_concat(out, olen);
    free(out);
    return result;
}

/* ── Layout ───────────────────────────────────────────────────────────────*/

FdLayoutOptions fd_layout_options_default(void) {
    FdLayoutOptions o = {80, FD_DEFAULT_INDENT_WIDTH, FD_DEFAULT_LINE_HEIGHT};
    return o;
}

/* UTF-8 code-point count (matches Rust chars().count()). */
static size_t visible_width(const char *s) {
    size_t count = 0;
    for (const unsigned char *p = (const unsigned char *)s; *p; p++)
        if ((*p & 0xC0) != 0x80) count++;
    return count;
}

/* Immutable cons-list of active annotations in a per-run arena (indices, so the
 * backing array may realloc freely). */
#define ANN_NIL ((size_t)-1)
typedef struct {
    const FdAnnotation *ann; /* borrowed from the immutable doc tree */
    size_t tail;
} AnnNode;
typedef struct {
    AnnNode *nodes;
    size_t len, cap;
} AnnArena;

static int ann_arena_push(AnnArena *ar, const FdAnnotation *ann, size_t tail,
                          size_t *out) {
    if (ar->len == ar->cap) {
        size_t nc = ar->cap ? ar->cap : 16;
        if (nc > ((size_t)-1) / 2 / sizeof(AnnNode)) return 0;
        nc *= 2;
        AnnNode *p = (AnnNode *)realloc(ar->nodes, nc * sizeof(AnnNode));
        if (p == NULL) return 0;
        ar->nodes = p;
        ar->cap = nc;
    }
    ar->nodes[ar->len].ann = ann;
    ar->nodes[ar->len].tail = tail;
    *out = ar->len;
    ar->len++;
    return 1;
}

/* Materialise an annotation list into an outer-first owned array. Returns 0 on
 * OOM. *out may be NULL with *n == 0. */
static int ann_materialise(const AnnArena *ar, size_t list, FdAnnotation **out,
                           size_t *n) {
    *out = NULL;
    *n = 0;
    size_t count = 0;
    for (size_t i = list; i != ANN_NIL; i = ar->nodes[i].tail) count++;
    if (count == 0) return 1;
    FdAnnotation *arr = (FdAnnotation *)malloc(count * sizeof(FdAnnotation));
    if (arr == NULL) return 0;
    size_t idx = count; /* list is inner-first; fill so index 0 is outermost */
    for (size_t i = list; i != ANN_NIL; i = ar->nodes[i].tail) {
        idx--;
        if (!ann_copy(ar->nodes[i].ann, &arr[idx])) {
            for (size_t k = idx + 1; k < count; k++) fd_ann_free(&arr[k]);
            free(arr);
            return 0;
        }
    }
    *out = arr;
    *n = count;
    return 1;
}

static int anns_equal(const FdAnnotation *a, size_t na, const FdAnnotation *b,
                      size_t nb) {
    if (na != nb) return 0;
    for (size_t i = 0; i < na; i++)
        if (!fd_ann_equal(&a[i], &b[i])) return 0;
    return 1;
}

typedef struct {
    size_t row;
    size_t indent_columns;
    FdLayoutSpan *spans;
    size_t n_spans, cap_spans;
} MutableLine;

typedef struct {
    size_t indent_levels;
    int mode; /* 0 = flat, 1 = break */
    size_t anns;
    const FdDoc *doc;
} Command;

typedef struct {
    Command *items;
    size_t len, cap;
} CmdStack;

static int cmd_push(CmdStack *s, Command c) {
    if (s->len == s->cap) {
        size_t nc = s->cap ? s->cap : 32;
        if (nc > ((size_t)-1) / 2 / sizeof(Command)) return 0;
        nc *= 2;
        Command *p = (Command *)realloc(s->items, nc * sizeof(Command));
        if (p == NULL) return 0;
        s->items = p;
        s->cap = nc;
    }
    s->items[s->len++] = c;
    return 1;
}

/* Emit `value` at the cursor with the given (materialised) annotations. */
static int push_text(MutableLine *line, size_t *column, size_t *max_column,
                     const char *value, const FdAnnotation *anns,
                     size_t n_anns) {
    if (value[0] == '\0') return 1;
    size_t w = visible_width(value);
    if (line->n_spans > 0) {
        FdLayoutSpan *last = &line->spans[line->n_spans - 1];
        if (anns_equal(last->annotations, last->n_annotations, anns, n_anns) &&
            last->column + visible_width(last->text) == *column) {
            size_t old = strlen(last->text);
            size_t add = strlen(value);
            char *nt = (char *)realloc(last->text, old + add + 1);
            if (nt == NULL) return 0;
            memcpy(nt + old, value, add + 1);
            last->text = nt;
            *column += w;
            if (*column > *max_column) *max_column = *column;
            return 1;
        }
    }
    if (line->n_spans == line->cap_spans) {
        size_t nc = line->cap_spans ? line->cap_spans * 2 : 4;
        FdLayoutSpan *p =
            (FdLayoutSpan *)realloc(line->spans, nc * sizeof(FdLayoutSpan));
        if (p == NULL) return 0;
        line->spans = p;
        line->cap_spans = nc;
    }
    FdLayoutSpan *sp = &line->spans[line->n_spans];
    sp->column = *column;
    sp->text = str_dup(value);
    if (sp->text == NULL) return 0;
    sp->annotations = NULL;
    sp->n_annotations = 0;
    if (n_anns > 0) {
        sp->annotations = (FdAnnotation *)malloc(n_anns * sizeof(FdAnnotation));
        if (sp->annotations == NULL) { free(sp->text); return 0; }
        for (size_t i = 0; i < n_anns; i++) {
            if (!ann_copy(&anns[i], &sp->annotations[i])) {
                for (size_t k = 0; k < i; k++) fd_ann_free(&sp->annotations[k]);
                free(sp->annotations);
                free(sp->text);
                return 0;
            }
        }
        sp->n_annotations = n_anns;
    }
    line->n_spans++;
    *column += w;
    if (*column > *max_column) *max_column = *column;
    return 1;
}

/* Materialise `anns_list` then push_text. */
static int emit_text(MutableLine *line, size_t *column, size_t *max_column,
                     const char *value, const AnnArena *ar, size_t anns_list) {
    FdAnnotation *anns = NULL;
    size_t n = 0;
    if (!ann_materialise(ar, anns_list, &anns, &n)) return 0;
    int ok = push_text(line, column, max_column, value, anns, n);
    for (size_t i = 0; i < n; i++) fd_ann_free(&anns[i]);
    free(anns);
    return ok;
}

/* Start a new line at the given indentation. */
static int push_line_break(MutableLine **lines, size_t *n_lines,
                           size_t *cap_lines, size_t *current, size_t *column,
                           size_t *max_column, size_t indent_levels,
                           size_t indent_width) {
    size_t indent_columns = indent_levels * indent_width;
    if (*n_lines == *cap_lines) {
        size_t nc = *cap_lines ? *cap_lines * 2 : 8;
        MutableLine *p = (MutableLine *)realloc(*lines, nc * sizeof(MutableLine));
        if (p == NULL) return 0;
        *lines = p;
        *cap_lines = nc;
    }
    MutableLine *ml = &(*lines)[*n_lines];
    ml->row = *n_lines;
    ml->indent_columns = indent_columns;
    ml->spans = NULL;
    ml->n_spans = 0;
    ml->cap_spans = 0;
    *current = *n_lines;
    (*n_lines)++;
    *column = indent_columns;
    if (*column > *max_column) *max_column = *column;
    return 1;
}

/* Push each child of a Concat onto `stack` (reversed, so they pop in order). */
static int push_docs(CmdStack *stack, const Command *base, const FdDoc *concat) {
    for (size_t k = concat->as.concat.len; k > 0; k--) {
        Command c = {base->indent_levels, base->mode, base->anns,
                     concat->as.concat.items[k - 1]};
        if (!cmd_push(stack, c)) return 0;
    }
    return 1;
}

/* Look-ahead: can the pending stack stay on the current line in flat mode? */
static int fits(long budget, const CmdStack *stack, Command next,
                CmdStack *pending) {
    pending->len = 0;
    if (!cmd_push(pending, next)) return 0;
    size_t stack_idx = stack->len;

    while (budget >= 0) {
        Command cmd;
        if (pending->len > 0) {
            cmd = pending->items[--pending->len];
        } else {
            if (stack_idx == 0) return 1;
            stack_idx--;
            cmd = stack->items[stack_idx];
        }
        switch (cmd.doc->kind) {
            case FD_NIL:
                break;
            case FD_TEXT:
                budget -= (long)visible_width(cmd.doc->as.text);
                break;
            case FD_CONCAT:
                if (!push_docs(pending, &cmd, cmd.doc)) return 0;
                break;
            case FD_GROUP: {
                Command c = {cmd.indent_levels, 0, cmd.anns, cmd.doc->as.child};
                if (!cmd_push(pending, c)) return 0;
                break;
            }
            case FD_INDENT: {
                Command c = {cmd.indent_levels + cmd.doc->as.indent.levels,
                             cmd.mode, cmd.anns, cmd.doc->as.indent.content};
                if (!cmd_push(pending, c)) return 0;
                break;
            }
            case FD_LINE:
                switch (cmd.doc->as.line_mode) {
                    case FD_LINE_HARD:
                        return 0;
                    case FD_LINE_NORMAL:
                        if (cmd.mode == 0) budget -= 1;
                        else return 1;
                        break;
                    case FD_LINE_SOFT:
                        if (cmd.mode == 1) return 1;
                        break;
                }
                break;
            case FD_IFBREAK: {
                const FdDoc *chosen = cmd.mode == 0 ? cmd.doc->as.ifbreak.flat
                                                    : cmd.doc->as.ifbreak.broken;
                Command c = {cmd.indent_levels, cmd.mode, cmd.anns, chosen};
                if (!cmd_push(pending, c)) return 0;
                break;
            }
            case FD_ANNOTATE: {
                Command c = {cmd.indent_levels, cmd.mode, cmd.anns,
                             cmd.doc->as.annotate.content};
                if (!cmd_push(pending, c)) return 0;
                break;
            }
        }
    }
    return 0;
}

/* Release partially/fully built mutable lines. */
static void free_mutable_lines(MutableLine *lines, size_t n) {
    for (size_t i = 0; i < n; i++) {
        for (size_t j = 0; j < lines[i].n_spans; j++) {
            free(lines[i].spans[j].text);
            for (size_t k = 0; k < lines[i].spans[j].n_annotations; k++)
                fd_ann_free(&lines[i].spans[j].annotations[k]);
            free(lines[i].spans[j].annotations);
        }
        free(lines[i].spans);
    }
    free(lines);
}

FdLayoutTree fd_layout_doc(const FdDoc *doc, const FdLayoutOptions *options) {
    FdLayoutTree tree;
    tree.print_width = options->print_width;
    tree.indent_width = options->indent_width;
    tree.line_height = options->line_height;
    tree.width = 0;
    tree.height = 0;
    tree.lines = NULL;
    tree.n_lines = 0;

    MutableLine *lines = NULL;
    size_t n_lines = 0, cap_lines = 0;
    CmdStack stack = {NULL, 0, 0};
    CmdStack pending = {NULL, 0, 0};
    AnnArena arena = {NULL, 0, 0};
    size_t current = 0, column = 0, max_column = 0;
    int ok = 1;

    if (!push_line_break(&lines, &n_lines, &cap_lines, &current, &column,
                         &max_column, 0, options->indent_width)) {
        ok = 0;
    } else {
        Command seed = {0, 1, ANN_NIL, doc};
        if (!cmd_push(&stack, seed)) ok = 0;
    }

    while (ok && stack.len > 0) {
        Command cmd = stack.items[--stack.len];
        switch (cmd.doc->kind) {
            case FD_NIL:
                break;
            case FD_TEXT:
                ok = emit_text(&lines[current], &column, &max_column,
                               cmd.doc->as.text, &arena, cmd.anns);
                break;
            case FD_CONCAT:
                ok = push_docs(&stack, &cmd, cmd.doc);
                break;
            case FD_GROUP: {
                long remaining = (long)options->print_width - (long)column;
                if (remaining < 0) remaining = 0;
                Command probe = {cmd.indent_levels, 0, cmd.anns,
                                 cmd.doc->as.child};
                int pick_flat = cmd.mode == 0 ||
                                fits(remaining, &stack, probe, &pending);
                Command c = {cmd.indent_levels, pick_flat ? 0 : 1, cmd.anns,
                             cmd.doc->as.child};
                ok = cmd_push(&stack, c);
                break;
            }
            case FD_INDENT: {
                Command c = {cmd.indent_levels + cmd.doc->as.indent.levels,
                             cmd.mode, cmd.anns, cmd.doc->as.indent.content};
                ok = cmd_push(&stack, c);
                break;
            }
            case FD_LINE:
                switch (cmd.doc->as.line_mode) {
                    case FD_LINE_HARD:
                        ok = push_line_break(&lines, &n_lines, &cap_lines,
                                             &current, &column, &max_column,
                                             cmd.indent_levels,
                                             options->indent_width);
                        break;
                    case FD_LINE_NORMAL:
                        if (cmd.mode == 0)
                            ok = emit_text(&lines[current], &column, &max_column,
                                           " ", &arena, cmd.anns);
                        else
                            ok = push_line_break(&lines, &n_lines, &cap_lines,
                                                 &current, &column, &max_column,
                                                 cmd.indent_levels,
                                                 options->indent_width);
                        break;
                    case FD_LINE_SOFT:
                        if (cmd.mode == 1)
                            ok = push_line_break(&lines, &n_lines, &cap_lines,
                                                 &current, &column, &max_column,
                                                 cmd.indent_levels,
                                                 options->indent_width);
                        break;
                }
                break;
            case FD_IFBREAK: {
                const FdDoc *chosen = cmd.mode == 0 ? cmd.doc->as.ifbreak.flat
                                                    : cmd.doc->as.ifbreak.broken;
                Command c = {cmd.indent_levels, cmd.mode, cmd.anns, chosen};
                ok = cmd_push(&stack, c);
                break;
            }
            case FD_ANNOTATE: {
                size_t new_list;
                if (!ann_arena_push(&arena, &cmd.doc->as.annotate.ann, cmd.anns,
                                    &new_list)) {
                    ok = 0;
                    break;
                }
                Command c = {cmd.indent_levels, cmd.mode, new_list,
                             cmd.doc->as.annotate.content};
                ok = cmd_push(&stack, c);
                break;
            }
        }
    }

    free(stack.items);
    free(pending.items);
    free(arena.nodes);

    if (!ok) {
        free_mutable_lines(lines, n_lines);
        return tree;
    }

    FdLayoutLine *out = (FdLayoutLine *)malloc(n_lines * sizeof(FdLayoutLine));
    if (out == NULL) {
        free_mutable_lines(lines, n_lines);
        return tree;
    }
    for (size_t i = 0; i < n_lines; i++) {
        MutableLine *ml = &lines[i];
        size_t width;
        if (ml->n_spans > 0) {
            FdLayoutSpan *last = &ml->spans[ml->n_spans - 1];
            width = last->column + visible_width(last->text);
        } else {
            width = ml->indent_columns;
        }
        out[i].row = ml->row;
        out[i].indent_columns = ml->indent_columns;
        out[i].width = width;
        out[i].spans = ml->spans; /* move span storage */
        out[i].n_spans = ml->n_spans;
    }
    free(lines);

    tree.width = max_column;
    tree.height = n_lines * options->line_height;
    tree.lines = out;
    tree.n_lines = n_lines;
    return tree;
}

void fd_layout_free(FdLayoutTree *tree) {
    if (tree == NULL || tree->lines == NULL) return;
    for (size_t i = 0; i < tree->n_lines; i++) {
        for (size_t j = 0; j < tree->lines[i].n_spans; j++) {
            free(tree->lines[i].spans[j].text);
            for (size_t k = 0; k < tree->lines[i].spans[j].n_annotations; k++)
                fd_ann_free(&tree->lines[i].spans[j].annotations[k]);
            free(tree->lines[i].spans[j].annotations);
        }
        free(tree->lines[i].spans);
    }
    free(tree->lines);
    tree->lines = NULL;
    tree->n_lines = 0;
}

char *fd_render_text(const FdLayoutTree *tree) {
    char *buf = NULL;
    size_t len = 0, cap = 0;
#define ENSURE(extra)                                                        \
    do {                                                                     \
        if (len + (extra) + 1 > cap) {                                       \
            size_t nc = cap ? cap : 64;                                      \
            while (nc < len + (extra) + 1) {                                 \
                if (nc > ((size_t)-1) / 2) { nc = len + (extra) + 1; break; } \
                nc *= 2;                                                     \
            }                                                                \
            char *np = (char *)realloc(buf, nc);                             \
            if (np == NULL) { free(buf); return NULL; }                      \
            buf = np;                                                        \
            cap = nc;                                                        \
        }                                                                    \
    } while (0)

    for (size_t i = 0; i < tree->n_lines; i++) {
        const FdLayoutLine *line = &tree->lines[i];
        if (i > 0) {
            ENSURE(1);
            buf[len++] = '\n';
        }
        ENSURE(line->indent_columns);
        for (size_t k = 0; k < line->indent_columns; k++) buf[len++] = ' ';
        size_t col = line->indent_columns;
        for (size_t j = 0; j < line->n_spans; j++) {
            const FdLayoutSpan *sp = &line->spans[j];
            while (col < sp->column) {
                ENSURE(1);
                buf[len++] = ' ';
                col++;
            }
            size_t tl = strlen(sp->text);
            ENSURE(tl);
            memcpy(buf + len, sp->text, tl);
            len += tl;
            col += visible_width(sp->text);
        }
    }
    ENSURE(0);
    buf[len] = '\0';
    return buf;
#undef ENSURE
}
