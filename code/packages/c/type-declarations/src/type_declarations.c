/*
 * type_declarations.c — implementation of the type-declaration format.
 * ===========================================================================
 * TdKind is a small value that owns a string only for the Named variant.
 * TdNamedType, the two maps inside TypeDeclarations, and TdAnnotatedNode own
 * their nested data; every constructor deep-copies and pairs with a `*_free`.
 */
#include "type_declarations.h"

#include <stdlib.h>
#include <string.h>

static char *dup_cstr(const char *s) {
    size_t n = strlen(s);
    char *out = malloc(n + 1);
    if (out) {
        memcpy(out, s, n + 1);
    }
    return out;
}

/* Grow *data so it holds at least `need` elements of `elem` bytes. 0 / -1. */
static int ensure_cap(void **data, size_t *cap, size_t need, size_t elem) {
    if (need <= *cap) {
        return 0;
    }
    size_t nc = *cap ? *cap : 4;
    while (nc < need) {
        if (nc > ((size_t)-1) / 2 / elem) {
            return -1;
        }
        nc *= 2;
    }
    void *nd = realloc(*data, nc * elem);
    if (!nd) {
        return -1;
    }
    *data = nd;
    *cap = nc;
    return 0;
}

/* ===========================================================================
 *  TdKind
 * =========================================================================== */

static TdKind kind_simple(TdKindTag tag) {
    TdKind k;
    k.tag = tag;
    k.named = NULL;
    k.arity = 0;
    return k;
}

TdKind td_kind_int(void) { return kind_simple(TD_INT); }
TdKind td_kind_bool(void) { return kind_simple(TD_BOOL); }
TdKind td_kind_nil(void) { return kind_simple(TD_NIL); }
TdKind td_kind_symbol(void) { return kind_simple(TD_SYMBOL); }
TdKind td_kind_str(void) { return kind_simple(TD_STR); }
TdKind td_kind_list(void) { return kind_simple(TD_LIST); }
TdKind td_kind_any(void) { return kind_simple(TD_ANY); }

TdKind td_kind_function(size_t arity) {
    TdKind k = kind_simple(TD_FUNCTION);
    k.arity = arity;
    return k;
}

int td_kind_named(const char *name, TdKind *out) {
    char *dup = dup_cstr(name);
    if (!dup) {
        return -1;
    }
    out->tag = TD_NAMED;
    out->named = dup;
    out->arity = 0;
    return 0;
}

int td_kind_copy(const TdKind *src, TdKind *out) {
    out->tag = src->tag;
    out->arity = src->arity;
    out->named = NULL;
    if (src->tag == TD_NAMED) {
        out->named = dup_cstr(src->named);
        if (!out->named) {
            return -1;
        }
    }
    return 0;
}

void td_kind_free(TdKind *k) {
    if (k && k->tag == TD_NAMED) {
        free(k->named);
        k->named = NULL;
    }
}

int td_kind_equals(const TdKind *a, const TdKind *b) {
    if (a->tag != b->tag) {
        return 0;
    }
    if (a->tag == TD_NAMED) {
        return strcmp(a->named, b->named) == 0;
    }
    if (a->tag == TD_FUNCTION) {
        return a->arity == b->arity;
    }
    return 1;
}

const char *td_kind_to_iir_hint(const TdKind *k) {
    switch (k->tag) {
        case TD_INT: return "i64";
        case TD_BOOL: return "bool";
        case TD_STR: return "str";
        case TD_FUNCTION: return "closure";
        default: return "any"; /* Nil, Symbol, List, Named, Any */
    }
}

int td_kind_is_concrete_hint(const TdKind *k) {
    return strcmp(td_kind_to_iir_hint(k), "any") != 0;
}

/* ===========================================================================
 *  FieldDecl / VariantDecl
 * =========================================================================== */

int td_field_init(TdField *out, const char *name, const TdKind *kind) {
    out->name = dup_cstr(name);
    if (!out->name) {
        return -1;
    }
    if (td_kind_copy(kind, &out->kind) != 0) {
        free(out->name);
        out->name = NULL;
        return -1;
    }
    return 0;
}

void td_field_free(TdField *f) {
    if (!f) {
        return;
    }
    free(f->name);
    td_kind_free(&f->kind);
    f->name = NULL;
}

/* Deep-copy an array of fields into a fresh calloc'd array (NULL for n==0). */
static int copy_fields(const TdField *fields, size_t n, TdField **out) {
    if (n == 0) {
        *out = NULL;
        return 0;
    }
    TdField *arr = calloc(n, sizeof(TdField));
    if (!arr) {
        return -1;
    }
    size_t i;
    for (i = 0; i < n; i++) {
        if (td_field_init(&arr[i], fields[i].name, &fields[i].kind) != 0) {
            size_t j;
            for (j = 0; j < i; j++) {
                td_field_free(&arr[j]);
            }
            free(arr);
            return -1;
        }
    }
    *out = arr;
    return 0;
}

int td_variant_init(TdVariant *out, const char *name, const TdField *fields,
                    size_t n_fields) {
    out->name = dup_cstr(name);
    out->fields = NULL;
    out->n_fields = 0;
    if (!out->name) {
        return -1;
    }
    if (copy_fields(fields, n_fields, &out->fields) != 0) {
        free(out->name);
        out->name = NULL;
        return -1;
    }
    out->n_fields = n_fields;
    return 0;
}

void td_variant_free(TdVariant *v) {
    if (!v) {
        return;
    }
    free(v->name);
    size_t i;
    for (i = 0; i < v->n_fields; i++) {
        td_field_free(&v->fields[i]);
    }
    free(v->fields);
    v->name = NULL;
    v->fields = NULL;
    v->n_fields = 0;
}

/* ===========================================================================
 *  NamedTypeDecl
 * =========================================================================== */

int td_named_record(const TdField *fields, size_t n_fields, TdNamedType *out) {
    memset(out, 0, sizeof *out);
    out->tag = TD_DECL_RECORD;
    if (copy_fields(fields, n_fields, &out->fields) != 0) {
        return -1;
    }
    out->n_fields = n_fields;
    return 0;
}

int td_named_union(const TdVariant *variants, size_t n_variants,
                   TdNamedType *out) {
    memset(out, 0, sizeof *out);
    out->tag = TD_DECL_UNION;
    if (n_variants == 0) {
        return 0;
    }
    out->variants = calloc(n_variants, sizeof(TdVariant));
    if (!out->variants) {
        return -1;
    }
    size_t i;
    for (i = 0; i < n_variants; i++) {
        if (td_variant_init(&out->variants[i], variants[i].name,
                            variants[i].fields, variants[i].n_fields) != 0) {
            size_t j;
            for (j = 0; j < i; j++) {
                td_variant_free(&out->variants[j]);
            }
            free(out->variants);
            out->variants = NULL;
            return -1;
        }
    }
    out->n_variants = n_variants;
    return 0;
}

int td_named_alias(const TdKind *target, TdNamedType *out) {
    memset(out, 0, sizeof *out);
    out->tag = TD_DECL_ALIAS;
    return td_kind_copy(target, &out->alias_target);
}

void td_named_free(TdNamedType *nt) {
    if (!nt) {
        return;
    }
    size_t i;
    for (i = 0; i < nt->n_fields; i++) {
        td_field_free(&nt->fields[i]);
    }
    free(nt->fields);
    for (i = 0; i < nt->n_variants; i++) {
        td_variant_free(&nt->variants[i]);
    }
    free(nt->variants);
    td_kind_free(&nt->alias_target); /* no-op unless it is an alias */
    memset(nt, 0, sizeof *nt);
}

/* ===========================================================================
 *  TypeDeclarations
 * =========================================================================== */

typedef struct {
    char *key;
    TdNamedType val;
} NamedEntry;

typedef struct {
    char *key;
    TdKind val;
} GlobalEntry;

struct TypeDeclarations {
    char *language;
    NamedEntry *named_types;
    size_t n_named, cap_named;
    GlobalEntry *globals;
    size_t n_globals, cap_globals;
    int has_typed_mode;
    TdTypedMode typed_mode;
};

int td_new(TypeDeclarations **out, const char *language) {
    TypeDeclarations *d = calloc(1, sizeof *d);
    if (!d) {
        return -1;
    }
    d->language = dup_cstr(language);
    if (!d->language) {
        free(d);
        return -1;
    }
    *out = d;
    return 0;
}

void td_free(TypeDeclarations *d) {
    if (!d) {
        return;
    }
    free(d->language);
    size_t i;
    for (i = 0; i < d->n_named; i++) {
        free(d->named_types[i].key);
        td_named_free(&d->named_types[i].val);
    }
    free(d->named_types);
    for (i = 0; i < d->n_globals; i++) {
        free(d->globals[i].key);
        td_kind_free(&d->globals[i].val);
    }
    free(d->globals);
    free(d);
}

const char *td_language(const TypeDeclarations *d) { return d->language; }
size_t td_named_type_count(const TypeDeclarations *d) { return d->n_named; }
size_t td_global_count(const TypeDeclarations *d) { return d->n_globals; }
int td_has_typed_mode(const TypeDeclarations *d) { return d->has_typed_mode; }
TdTypedMode td_typed_mode(const TypeDeclarations *d) { return d->typed_mode; }
void td_set_typed_mode(TypeDeclarations *d, TdTypedMode mode) {
    d->has_typed_mode = 1;
    d->typed_mode = mode;
}

static const TdNamedType *find_named(const TypeDeclarations *d,
                                     const char *name) {
    size_t i;
    for (i = 0; i < d->n_named; i++) {
        if (strcmp(d->named_types[i].key, name) == 0) {
            return &d->named_types[i].val;
        }
    }
    return NULL;
}

int td_insert_named_type(TypeDeclarations *d, const char *name,
                         TdNamedType decl) {
    size_t i;
    for (i = 0; i < d->n_named; i++) {
        if (strcmp(d->named_types[i].key, name) == 0) {
            td_named_free(&d->named_types[i].val); /* replace in place */
            d->named_types[i].val = decl;
            return 0;
        }
    }
    if (ensure_cap((void **)&d->named_types, &d->cap_named, d->n_named + 1,
                   sizeof(NamedEntry)) != 0) {
        td_named_free(&decl);
        return -1;
    }
    char *key = dup_cstr(name);
    if (!key) {
        td_named_free(&decl);
        return -1;
    }
    d->named_types[d->n_named].key = key;
    d->named_types[d->n_named].val = decl;
    d->n_named++;
    return 0;
}

int td_insert_global(TypeDeclarations *d, const char *name, TdKind kind) {
    size_t i;
    for (i = 0; i < d->n_globals; i++) {
        if (strcmp(d->globals[i].key, name) == 0) {
            td_kind_free(&d->globals[i].val);
            d->globals[i].val = kind;
            return 0;
        }
    }
    if (ensure_cap((void **)&d->globals, &d->cap_globals, d->n_globals + 1,
                   sizeof(GlobalEntry)) != 0) {
        td_kind_free(&kind);
        return -1;
    }
    char *key = dup_cstr(name);
    if (!key) {
        td_kind_free(&kind);
        return -1;
    }
    d->globals[d->n_globals].key = key;
    d->globals[d->n_globals].val = kind;
    d->n_globals++;
    return 0;
}

static int resolve_depth(const TypeDeclarations *d, const TdKind *kind,
                         size_t depth, TdKind *out) {
    if (depth > 32) {
        *out = td_kind_any();
        return 0;
    }
    if (kind->tag == TD_NAMED) {
        const TdNamedType *nt = find_named(d, kind->named);
        if (nt && nt->tag == TD_DECL_ALIAS) {
            return resolve_depth(d, &nt->alias_target, depth + 1, out);
        }
        return td_kind_copy(kind, out);
    }
    return td_kind_copy(kind, out);
}

int td_resolve(const TypeDeclarations *d, const TdKind *kind, TdKind *out) {
    return resolve_depth(d, kind, 0, out);
}

int td_union_variants(const TypeDeclarations *d, const char *name, char ***out,
                      size_t *count) {
    const TdNamedType *nt = find_named(d, name);
    if (!nt || nt->tag != TD_DECL_UNION) {
        *out = NULL;
        *count = 0;
        return 0; /* not a union (or absent) */
    }
    if (nt->n_variants == 0) {
        *out = NULL;
        *count = 0;
        return 1; /* an empty union is still "some" */
    }
    char **arr = calloc(nt->n_variants, sizeof(char *));
    if (!arr) {
        return -1;
    }
    size_t i;
    for (i = 0; i < nt->n_variants; i++) {
        arr[i] = dup_cstr(nt->variants[i].name);
        if (!arr[i]) {
            size_t j;
            for (j = 0; j < i; j++) {
                free(arr[j]);
            }
            free(arr);
            return -1;
        }
    }
    *out = arr;
    *count = nt->n_variants;
    return 1;
}

void td_string_array_free(char **arr, size_t count) {
    if (!arr) {
        return;
    }
    size_t i;
    for (i = 0; i < count; i++) {
        free(arr[i]);
    }
    free(arr);
}

/* ===========================================================================
 *  AnnotatedNode / AnnotatedChild
 * =========================================================================== */

int td_annotated_node_init(TdAnnotatedNode *out, const char *rule_name,
                           const TdKind *kind) {
    memset(out, 0, sizeof *out);
    out->rule_name = dup_cstr(rule_name);
    if (!out->rule_name) {
        return -1;
    }
    if (td_kind_copy(kind, &out->kind) != 0) {
        free(out->rule_name);
        out->rule_name = NULL;
        return -1;
    }
    return 0;
}

static void child_free(TdAnnotatedChild *c) {
    if (c->tag == TD_CHILD_NODE) {
        td_annotated_node_free(c->node);
        free(c->node);
    } else {
        free(c->token_text);
    }
}

void td_annotated_node_free(TdAnnotatedNode *n) {
    if (!n) {
        return;
    }
    free(n->rule_name);
    td_kind_free(&n->kind);
    size_t i;
    for (i = 0; i < n->n_children; i++) {
        child_free(&n->children[i]);
    }
    free(n->children);
    n->rule_name = NULL;
    n->children = NULL;
    n->n_children = 0;
}

/* Append `c`, growing the children array by one (its contents move in). On
 * failure the child is freed and -1 returned. */
static int push_child(TdAnnotatedNode *parent, TdAnnotatedChild c) {
    if (parent->n_children > ((size_t)-1) / sizeof(TdAnnotatedChild) - 1) {
        child_free(&c);
        return -1;
    }
    size_t newn = parent->n_children + 1;
    TdAnnotatedChild *nd =
        realloc(parent->children, newn * sizeof(TdAnnotatedChild));
    if (!nd) {
        child_free(&c);
        return -1;
    }
    parent->children = nd;
    parent->children[parent->n_children] = c;
    parent->n_children = newn;
    return 0;
}

int td_annotated_node_add_child_node(TdAnnotatedNode *parent,
                                     TdAnnotatedNode child) {
    TdAnnotatedNode *node = malloc(sizeof(TdAnnotatedNode));
    if (!node) {
        td_annotated_node_free(&child);
        return -1;
    }
    *node = child; /* move the value's owned pointers into the heap node */
    TdAnnotatedChild c;
    c.tag = TD_CHILD_NODE;
    c.node = node;
    c.token_text = NULL;
    c.token_line = 0;
    c.token_column = 0;
    return push_child(parent, c);
}

int td_annotated_node_add_token(TdAnnotatedNode *parent, const char *text,
                                size_t line, size_t column) {
    char *dup = dup_cstr(text);
    if (!dup) {
        return -1;
    }
    TdAnnotatedChild c;
    c.tag = TD_CHILD_TOKEN;
    c.node = NULL;
    c.token_text = dup;
    c.token_line = line;
    c.token_column = column;
    return push_child(parent, c);
}

void td_annotated_node_set_position(TdAnnotatedNode *n, size_t start_line,
                                    size_t start_column, size_t end_line,
                                    size_t end_column) {
    n->has_start_line = 1;
    n->start_line = start_line;
    n->has_start_column = 1;
    n->start_column = start_column;
    n->has_end_line = 1;
    n->end_line = end_line;
    n->has_end_column = 1;
    n->end_column = end_column;
}

const char *td_annotated_node_iir_hint(const TdAnnotatedNode *n) {
    return td_kind_to_iir_hint(&n->kind);
}

const TdAnnotatedNode *td_annotated_node_child_node(const TdAnnotatedNode *n,
                                                    const char *rule) {
    size_t i;
    for (i = 0; i < n->n_children; i++) {
        if (n->children[i].tag == TD_CHILD_NODE &&
            strcmp(n->children[i].node->rule_name, rule) == 0) {
            return n->children[i].node;
        }
    }
    return NULL;
}

int td_annotated_node_node_children(const TdAnnotatedNode *n,
                                    const TdAnnotatedNode ***out,
                                    size_t *count) {
    size_t i, k = 0;
    for (i = 0; i < n->n_children; i++) {
        if (n->children[i].tag == TD_CHILD_NODE) {
            k++;
        }
    }
    if (k == 0) {
        *out = NULL;
        *count = 0;
        return 0;
    }
    const TdAnnotatedNode **arr = calloc(k, sizeof(TdAnnotatedNode *));
    if (!arr) {
        return -1;
    }
    size_t j = 0;
    for (i = 0; i < n->n_children; i++) {
        if (n->children[i].tag == TD_CHILD_NODE) {
            arr[j++] = n->children[i].node;
        }
    }
    *out = arr;
    *count = k;
    return 0;
}

void td_annotated_node_position(const TdAnnotatedNode *n, size_t *line,
                                size_t *column) {
    *line = n->has_start_line ? n->start_line : 0;
    *column = n->has_start_column ? n->start_column : 0;
}
