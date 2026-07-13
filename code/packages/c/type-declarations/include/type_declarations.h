/*
 * type_declarations.h — a language-agnostic type-declaration format, in pure
 * ISO C17. A faithful port of the Rust `type-declarations` crate.
 * ===========================================================================
 *
 * The analogue of TypeScript `.d.ts` files: any parser can emit a set of type
 * declarations (named record/union/alias types, global binding kinds, and a
 * typed-mode setting) alongside its own AST; a generic checker consumes them to
 * infer a `KindDecl` for every expression.
 *
 * The core value is `TdKind` — the base kind of an expression:
 *   Int, Bool, Nil, Symbol, Str, List, Named(name), Function(arity), Any.
 * Each maps to an IIR `type_hint` string (`td_kind_to_iir_hint`): Int->"i64",
 * Bool->"bool", Str->"str", Function->"closure", everything else->"any".
 *
 * `TypeDeclarations` holds the named types + globals and can `resolve` a kind
 * through alias chains (depth-limited to 32, returning Any on a cycle) and list
 * a union's `union_variants`. `TdAnnotatedNode` is the checker's output tree —
 * a rule-named node carrying its inferred kind plus annotated children.
 *
 * OWNERSHIP. Values that own strings / arrays / sub-trees (TdKind's Named name,
 * TdNamedType, TypeDeclarations, TdAnnotatedNode) each pair a constructor with a
 * matching `*_free`. Constructors deep-copy their inputs; `insert_*` and
 * `add_child_*` take ownership of the value passed by value.
 *
 * DIVERGENCE FROM RUST. Rust returns owned values / `Option`; this port writes
 * through out-parameters and signals allocation failure with a `0`/`-1` return.
 *
 * PORTABILITY. Pure ISO C17 — no compiler extensions. Builds clean under GCC,
 * Clang, and MSVC with -pedantic-errors / /permissive- and warnings-as-errors.
 */
#ifndef CA_TYPE_DECLARATIONS_H
#define CA_TYPE_DECLARATIONS_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ── KindDecl ─────────────────────────────────────────────────────────────── */

typedef enum {
    TD_INT,
    TD_BOOL,
    TD_NIL,
    TD_SYMBOL,
    TD_STR,
    TD_LIST,
    TD_NAMED,    /* carries an owned `named` string */
    TD_FUNCTION, /* carries an `arity` */
    TD_ANY
} TdKindTag;

/* A base kind. `named` is owned iff tag == TD_NAMED; `arity` is meaningful iff
 * tag == TD_FUNCTION. Copy with td_kind_copy, release with td_kind_free. */
typedef struct {
    TdKindTag tag;
    char *named;
    size_t arity;
} TdKind;

/* Simple (payload-free) kinds returned by value. */
TdKind td_kind_int(void);
TdKind td_kind_bool(void);
TdKind td_kind_nil(void);
TdKind td_kind_symbol(void);
TdKind td_kind_str(void);
TdKind td_kind_list(void);
TdKind td_kind_any(void);
TdKind td_kind_function(size_t arity);
/* Named kind; deep-copies `name`. Returns 0 or -1 on OOM. */
int td_kind_named(const char *name, TdKind *out);

int td_kind_copy(const TdKind *src, TdKind *out); /* deep copy; 0 / -1 */
void td_kind_free(TdKind *k);
int td_kind_equals(const TdKind *a, const TdKind *b);

/* The IIR type_hint string ("i64" / "bool" / "str" / "closure" / "any"). */
const char *td_kind_to_iir_hint(const TdKind *k);
/* True (1) when the hint is concrete (not "any"). */
int td_kind_is_concrete_hint(const TdKind *k);

/* ── FieldDecl / VariantDecl ──────────────────────────────────────────────── */

typedef struct {
    char *name; /* owned */
    TdKind kind;
} TdField;

typedef struct {
    char *name; /* owned */
    TdField *fields;
    size_t n_fields;
} TdVariant;

int td_field_init(TdField *out, const char *name, const TdKind *kind);
void td_field_free(TdField *f);
int td_variant_init(TdVariant *out, const char *name, const TdField *fields,
                    size_t n_fields);
void td_variant_free(TdVariant *v);

/* ── NamedTypeDecl ────────────────────────────────────────────────────────── */

typedef enum { TD_DECL_RECORD, TD_DECL_UNION, TD_DECL_ALIAS } TdNamedTag;

typedef struct {
    TdNamedTag tag;
    TdField *fields; /* RECORD (owned) */
    size_t n_fields;
    TdVariant *variants; /* UNION (owned) */
    size_t n_variants;
    TdKind alias_target; /* ALIAS */
} TdNamedType;

int td_named_record(const TdField *fields, size_t n_fields, TdNamedType *out);
int td_named_union(const TdVariant *variants, size_t n_variants,
                   TdNamedType *out);
int td_named_alias(const TdKind *target, TdNamedType *out);
void td_named_free(TdNamedType *nt);

/* ── TypedModeDecl ────────────────────────────────────────────────────────── */

typedef enum { TD_MODE_OFF, TD_MODE_LENIENT, TD_MODE_STRICT } TdTypedMode;

/* ── TypeDeclarations ─────────────────────────────────────────────────────── */

typedef struct TypeDeclarations TypeDeclarations;

int td_new(TypeDeclarations **out, const char *language); /* 0 / -1 */
void td_free(TypeDeclarations *d);

const char *td_language(const TypeDeclarations *d);
size_t td_named_type_count(const TypeDeclarations *d);
size_t td_global_count(const TypeDeclarations *d);
int td_has_typed_mode(const TypeDeclarations *d);
TdTypedMode td_typed_mode(const TypeDeclarations *d);
void td_set_typed_mode(TypeDeclarations *d, TdTypedMode mode);

/* Insert (taking ownership of `decl` / `kind`). Replaces an existing entry with
 * the same name. Returns 0 or -1 on OOM (on failure, ownership of decl/kind is
 * consumed — it is freed). */
int td_insert_named_type(TypeDeclarations *d, const char *name,
                         TdNamedType decl);
int td_insert_global(TypeDeclarations *d, const char *name, TdKind kind);

/* Resolve a kind through alias chains (depth-limited to 32; a cycle yields
 * Any). Writes a freshly-owned kind to *out. Returns 0 or -1 on OOM. */
int td_resolve(const TypeDeclarations *d, const TdKind *kind, TdKind *out);

/* If `name` maps to a Union, write a malloc'd array of owned variant-name
 * strings to *out (count in *count) and return 1; if it is not a union (or is
 * absent) return 0; return -1 on OOM. Release with td_string_array_free. */
int td_union_variants(const TypeDeclarations *d, const char *name, char ***out,
                      size_t *count);
void td_string_array_free(char **arr, size_t count);

/* ── AnnotatedNode / AnnotatedChild ───────────────────────────────────────── */

typedef struct TdAnnotatedNode TdAnnotatedNode;

typedef enum { TD_CHILD_NODE, TD_CHILD_TOKEN } TdChildTag;

typedef struct {
    TdChildTag tag;
    TdAnnotatedNode *node; /* TD_CHILD_NODE (owned) */
    char *token_text;      /* TD_CHILD_TOKEN (owned) */
    size_t token_line;
    size_t token_column;
} TdAnnotatedChild;

struct TdAnnotatedNode {
    char *rule_name; /* owned */
    TdKind kind;
    TdAnnotatedChild *children; /* owned */
    size_t n_children;
    int has_start_line;
    size_t start_line;
    int has_start_column;
    size_t start_column;
    int has_end_line;
    size_t end_line;
    int has_end_column;
    size_t end_column;
};

/* Initialize a node from a rule name and kind (both deep-copied; positions
 * absent). Returns 0 or -1 on OOM. */
int td_annotated_node_init(TdAnnotatedNode *out, const char *rule_name,
                           const TdKind *kind);
void td_annotated_node_free(TdAnnotatedNode *n); /* recursive */

/* Append a child. add_child_node takes ownership of `child`. Returns 0 / -1. */
int td_annotated_node_add_child_node(TdAnnotatedNode *parent,
                                     TdAnnotatedNode child);
int td_annotated_node_add_token(TdAnnotatedNode *parent, const char *text,
                                size_t line, size_t column);
void td_annotated_node_set_position(TdAnnotatedNode *n, size_t start_line,
                                    size_t start_column, size_t end_line,
                                    size_t end_column);

const char *td_annotated_node_iir_hint(const TdAnnotatedNode *n);
/* First child *node* with the given rule name, or NULL (borrowed). */
const TdAnnotatedNode *td_annotated_node_child_node(const TdAnnotatedNode *n,
                                                    const char *rule);
/* Malloc'd array of borrowed pointers to the immediate child nodes (token
 * leaves excluded); count in *count. Returns 0 / -1 (free the array itself). */
int td_annotated_node_node_children(const TdAnnotatedNode *n,
                                    const TdAnnotatedNode ***out, size_t *count);
/* Source position (start_line, start_column), each falling back to 0. */
void td_annotated_node_position(const TdAnnotatedNode *n, size_t *line,
                                size_t *column);

#ifdef __cplusplus
}
#endif

#endif /* CA_TYPE_DECLARATIONS_H */
