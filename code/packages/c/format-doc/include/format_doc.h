/*
 * format_doc.h — Wadler-style document algebra for pretty-printers, pure ISO C17.
 * =============================================================================
 *
 * A faithful port of the Rust `format-doc` crate. Build a backend-neutral
 * pretty-printing document (`FdDoc`) from primitives, then realise it into a
 * `FdLayoutTree` of positioned text spans, or flatten to a plain string.
 *
 * ## Primitives
 *
 *   fd_text        emit literal text (embedded \n auto-split into hardlines)
 *   fd_concat      emit child docs in sequence (flattens, drops nil)
 *   fd_group       print flat if it fits, else broken
 *   fd_indent      add indentation for broken lines inside
 *   fd_line        space when flat, newline when broken
 *   fd_softline    empty when flat, newline when broken
 *   fd_hardline    always newline (forces the enclosing group to break)
 *   fd_if_break    emit `broken` in broken mode, else `flat`
 *   fd_annotate    attach metadata to emitted spans without changing layout
 *   fd_nil         the empty document
 *   fd_join        join docs by a separator
 *
 * ## Ownership
 *
 * An `FdDoc *` is an immutable owned tree: each builder TAKES OWNERSHIP of the
 * docs handed to it (so you compose bottom-up), and you release the whole tree
 * with one `fd_free`. `fd_layout_doc` only borrows the doc; the returned
 * `FdLayoutTree` and the `char *` from `fd_render_text` are separately owned
 * (`fd_layout_free` / `free`). A builder returns NULL on allocation failure,
 * having freed the docs you passed in.
 *
 * Pure ISO C17: compiles under GCC, Clang and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors; no <math.h>, no compiler extensions.
 */
#ifndef CA_FORMAT_DOC_H
#define CA_FORMAT_DOC_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* int64_t */

#ifdef __cplusplus
extern "C" {
#endif

#define FD_DEFAULT_INDENT_WIDTH 2
#define FD_DEFAULT_LINE_HEIGHT 1

/* ── Annotations ──────────────────────────────────────────────────────────*/

typedef enum { FD_ANN_STR, FD_ANN_INT, FD_ANN_BOOL, FD_ANN_NULL } FdAnnKind;

typedef struct {
    FdAnnKind kind;
    char *str; /* owned, for FD_ANN_STR only */
    int64_t i; /* FD_ANN_INT */
    int b;     /* FD_ANN_BOOL */
} FdAnnotation;

FdAnnotation fd_ann_str(const char *s); /* copies `s`; NULL str on OOM */
FdAnnotation fd_ann_int(int64_t v);
FdAnnotation fd_ann_bool(int v);
FdAnnotation fd_ann_null(void);
void fd_ann_free(FdAnnotation *a); /* frees an owned FD_ANN_STR string */
int fd_ann_equal(const FdAnnotation *a, const FdAnnotation *b);

/* ── Documents ────────────────────────────────────────────────────────────*/

typedef struct FdDoc FdDoc;

FdDoc *fd_nil(void);
FdDoc *fd_text(const char *value);
/* Consume `parts[0..n)` (owned) into a flattened concat. `parts` array itself
 * is not freed. */
FdDoc *fd_concat(FdDoc **parts, size_t n);
/* Join `parts[0..n)` with copies of `separator`; consumes both. */
FdDoc *fd_join(FdDoc *separator, FdDoc **parts, size_t n);
FdDoc *fd_group(FdDoc *content);
FdDoc *fd_indent(FdDoc *content, size_t levels);
FdDoc *fd_line(void);
FdDoc *fd_softline(void);
FdDoc *fd_hardline(void);
FdDoc *fd_if_break(FdDoc *broken, FdDoc *flat);
FdDoc *fd_annotate(FdAnnotation annotation, FdDoc *content); /* takes `annotation` */

/* Deep-copy a document tree (a structural clone; the original is untouched).
 * Returns NULL on allocation failure. */
FdDoc *fd_clone(const FdDoc *doc);
/* True iff `doc` is the empty document (`fd_nil`, or empty text that collapsed
 * to nil). Useful for callers that special-case an empty body. */
int fd_is_nil(const FdDoc *doc);

void fd_free(FdDoc *doc);

/* ── Layout ───────────────────────────────────────────────────────────────*/

typedef struct {
    size_t column;
    char *text; /* owned, single-line */
    FdAnnotation *annotations;
    size_t n_annotations;
} FdLayoutSpan;

typedef struct {
    size_t row;
    size_t indent_columns;
    size_t width;
    FdLayoutSpan *spans;
    size_t n_spans;
} FdLayoutLine;

typedef struct {
    size_t print_width;
    size_t indent_width;
    size_t line_height;
    size_t width;  /* max column reached */
    size_t height; /* n_lines * line_height */
    FdLayoutLine *lines;
    size_t n_lines;
} FdLayoutTree;

typedef struct {
    size_t print_width; /* must be > 0 */
    size_t indent_width;
    size_t line_height;
} FdLayoutOptions;

FdLayoutOptions fd_layout_options_default(void); /* 80 / 2 / 1 */

/* Realise `doc` into a layout tree (borrows `doc`; requires print_width > 0).
 * On allocation failure the returned tree is empty (n_lines == 0). Release with
 * fd_layout_free. */
FdLayoutTree fd_layout_doc(const FdDoc *doc, const FdLayoutOptions *options);
void fd_layout_free(FdLayoutTree *tree);

/* Flatten a layout tree to a plain string (indent spaces + spans, '\n'-joined,
 * no trailing newline). Returns a malloc'd string (caller frees) or NULL. */
char *fd_render_text(const FdLayoutTree *tree);

#ifdef __cplusplus
}
#endif

#endif /* CA_FORMAT_DOC_H */
