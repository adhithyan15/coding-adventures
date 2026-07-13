/*
 * format_doc_std.h — reusable pretty-printing templates over format-doc.
 * =====================================================================
 *
 * A faithful port of the Rust `format-doc-std` crate — the "80% layer" of the
 * formatter stack. `format-doc` owns the primitive document algebra; this layer
 * owns the common syntax shapes most languages reuse. Every template BUILDS AND
 * RETURNS an `FdDoc *` (see format_doc.h); the width-fitting layout later
 * decides flat-vs-broken.
 *
 *   fds_delimited_list  arrays / tuples / parameter & argument lists / fields
 *   fds_call_like       function & constructor calls (callee + delimited args)
 *   fds_block_like      braces / begin…end / indented block bodies
 *   fds_infix_chain     arithmetic / boolean / pipeline / type-operator chains
 *
 * ## Ownership
 *
 * A template CONSUMES the content documents handed to it directly (the
 * `open`/`close` delimiters, the `items`/`args`/`operands`/`operators`, the
 * `body`, the `callee`) — exactly like the format-doc builders — and returns a
 * freshly-owned `FdDoc *` you release with `fd_free`. A config, by contrast,
 * only BORROWS the delimiter documents it carries: the template clones what it
 * needs, so a config is reusable and you free its documents yourself.
 *
 * Pure ISO C17: compiles under GCC, Clang and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors; no compiler extensions.
 */
#ifndef FORMAT_DOC_STD_H
#define FORMAT_DOC_STD_H

#include <stddef.h> /* size_t */

#include "format_doc.h" /* FdDoc and its builders */

#ifdef __cplusplus
extern "C" {
#endif

extern const char *const fds_version; /* "0.1.0" */

/* Whether a delimited list emits a trailing separator. */
typedef enum {
    FDS_TRAILING_NEVER,   /* [a, b, c] (default) */
    FDS_TRAILING_ALWAYS,  /* [a, b, c,] even when flat */
    FDS_TRAILING_IF_BREAK /* trailing separator only when broken */
} FdsTrailingSeparator;

/* ── delimited_list ────────────────────────────────────────────────────────*/

typedef struct {
    const FdDoc *separator; /* borrowed & cloned by the template (e.g. ",") */
    FdsTrailingSeparator trailing_separator;
    int empty_spacing; /* 1 → an empty list is "[ ]" rather than "[]" */
} FdsDelimitedListConfig;

/* Format `items` surrounded by `open`/`close`, comma-separated, no trailing
 * separator. Consumes `open`, `items[0..n)`, `close`. NULL on OOM. */
FdDoc *fds_delimited_list(FdDoc *open, FdDoc **items, size_t n, FdDoc *close);
/* As above with a caller-supplied config (its `separator` is borrowed and must
 * be non-NULL). */
FdDoc *fds_delimited_list_with(FdDoc *open, FdDoc **items, size_t n,
                               FdDoc *close,
                               const FdsDelimitedListConfig *config);

/* ── call_like ─────────────────────────────────────────────────────────────*/

typedef struct {
    const FdDoc *open;      /* borrowed; e.g. "(" */
    const FdDoc *close;     /* borrowed; e.g. ")" */
    const FdDoc *separator; /* borrowed; e.g. "," */
    FdsTrailingSeparator trailing_separator;
} FdsCallLikeConfig;

/* Format a call: `callee` followed by a delimited argument list. Consumes
 * `callee` and `args[0..n)` (the config's delimiter documents are borrowed).
 * NULL on OOM. */
FdDoc *fds_call_like(FdDoc *callee, FdDoc **args, size_t n,
                     const FdsCallLikeConfig *config);

/* ── block_like ────────────────────────────────────────────────────────────*/

typedef struct {
    int empty_spacing; /* 1 (default) → empty body is "{ }" rather than "{}" */
} FdsBlockLikeConfig;

FdsBlockLikeConfig fds_block_like_config_default(void); /* empty_spacing = 1 */

/* Format `open <body> close`. A nil `body` (see fd_is_nil) collapses to
 * `open close` (with a space between when empty_spacing). Consumes `open`,
 * `body`, `close`. NULL on OOM. */
FdDoc *fds_block_like(FdDoc *open, FdDoc *body, FdDoc *close);
FdDoc *fds_block_like_with(FdDoc *open, FdDoc *body, FdDoc *close,
                           const FdsBlockLikeConfig *config);

/* ── infix_chain ───────────────────────────────────────────────────────────*/

typedef struct {
    /* 1 → broken form leads each new line with the operator (Haskell/SQL);
     * 0 (default) → operators trail the previous line (C/Java/JS). */
    int break_before_operators;
} FdsInfixChainConfig;

FdsInfixChainConfig fds_infix_chain_config_default(void); /* break_before = 0 */

/* Format `operands` joined by `operators` (which must number one fewer than the
 * operands; on a mismatch the arguments are freed and NULL is returned).
 * Consumes `operands[0..n_operands)` and `operators[0..n_operators)`. Empty →
 * the nil document. NULL on OOM. */
FdDoc *fds_infix_chain(FdDoc **operands, size_t n_operands, FdDoc **operators,
                       size_t n_operators, const FdsInfixChainConfig *config);

#ifdef __cplusplus
}
#endif

#endif /* FORMAT_DOC_STD_H */
