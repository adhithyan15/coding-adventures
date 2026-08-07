/*
 * dom_core.h — a small DOM tree model, in pure ISO C17. A faithful port of the
 * Rust `dom-core` crate.
 * ===========================================================================
 *
 * A lower-level model than a document AST: it preserves HTML element names,
 * namespaces, attributes, text, comments, and doctypes so browser-facing code
 * can later layer CSS, layout, and scripting on top.
 *
 * SHAPE. A `DomDocument` owns a list of top-level `DomNode`s. A node is one of
 * four kinds — a document-type declaration, an element (with a name, an optional
 * namespace, attributes, and its own child nodes), a text run, or a comment.
 *
 * OWNERSHIP. Constructors return a malloc'd `DomNode *` (NULL on allocation
 * failure). Appending a node (`dom_element_append_child`, `dom_document_push_child`)
 * TRANSFERS ownership to the parent — do not free an appended node yourself.
 * Freeing a document (or a detached node) recursively frees the whole subtree.
 * Attribute arrays passed to a constructor are deep-copied, so the caller keeps
 * ownership of its own array.
 *
 * DEPTH. Like the Rust original's recursive `Drop`, `dom_node_free` recurses
 * once per nesting level, so tearing down a pathologically deep tree (as an
 * untrusted-HTML parser layered on top could build) can exhaust the stack. A
 * consumer that ingests untrusted markup should bound the nesting depth it
 * constructs.
 *
 * PORTABILITY. Pure ISO C17 — no POSIX strdup, no extensions. Builds clean under
 * GCC, Clang, and MSVC with -pedantic-errors / /permissive- and
 * warnings-as-errors.
 */
#ifndef CA_DOM_CORE_H
#define CA_DOM_CORE_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Which kind of node this is. */
typedef enum {
    DOM_DOCUMENT_TYPE,
    DOM_ELEMENT,
    DOM_TEXT,
    DOM_COMMENT
} DomNodeKind;

/* An element attribute (name/value); deep-copied on construction. */
typedef struct {
    const char *name;
    const char *value;
} DomAttribute;

/* Opaque node and document handles. */
typedef struct DomNode DomNode;
typedef struct DomDocument DomDocument;

/* ── Constructors (malloc'd; NULL on OOM) ───────────────────────────────── */

/* An element with `name`, no namespace, and a deep copy of `nattrs` attributes
 * (pass NULL/0 for none). Starts with no children. */
DomNode *dom_element(const char *name, const DomAttribute *attrs, size_t nattrs);
/* An element in the given namespace. */
DomNode *dom_namespaced_element(const char *ns, const char *name,
                                const DomAttribute *attrs, size_t nattrs);
DomNode *dom_text(const char *data);
DomNode *dom_comment(const char *data);
/* A document-type declaration; `name`/`public_id`/`system_id` may be NULL. */
DomNode *dom_doctype(const char *name, const char *public_id,
                     const char *system_id, int force_quirks);

/* Recursively free a node and its whole subtree. */
void dom_node_free(DomNode *node);

/* ── Node accessors ─────────────────────────────────────────────────────── */

DomNodeKind dom_node_kind(const DomNode *node);

/* Append `child` to an element, TAKING OWNERSHIP of it. Returns 0 on success,
 * or -1 if `parent` is not an element (child untouched) or on OOM (child freed).
 */
int dom_element_append_child(DomNode *parent, DomNode *child);

/* The element's tag name, or NULL if `node` is not an element. */
const char *dom_element_name(const DomNode *node);
/* The element's namespace, or NULL if absent or `node` is not an element. */
const char *dom_element_namespace(const DomNode *node);
/* The value of attribute `name` on an element, or NULL if absent / not an
 * element. Borrows from the node. */
const char *dom_element_attribute(const DomNode *node, const char *name);

/* The child nodes of an element: returns the array (borrowed) and writes the
 * count to *count_out. Returns NULL (count 0) if `node` is not an element —
 * mirroring Rust's `children() -> Option<&[Node]>`. */
DomNode *const *dom_node_children(const DomNode *node, size_t *count_out);

/* The data of a text / comment node, or NULL for the wrong kind. */
const char *dom_text_data(const DomNode *node);
const char *dom_comment_data(const DomNode *node);

/* Doctype fields (NULL / 0 for the wrong kind or an absent optional). */
const char *dom_doctype_name(const DomNode *node);
const char *dom_doctype_public_id(const DomNode *node);
const char *dom_doctype_system_id(const DomNode *node);
int dom_doctype_force_quirks(const DomNode *node);

/* ── Document ────────────────────────────────────────────────────────────── */

DomDocument *dom_document_new(void); /* NULL on OOM */
void dom_document_free(DomDocument *doc);

/* Append `child` to the document, TAKING OWNERSHIP. Returns 0, or -1 on OOM
 * (child freed). */
int dom_document_push_child(DomDocument *doc, DomNode *child);

/* The document's top-level children (borrowed) and their count. */
DomNode *const *dom_document_children(const DomDocument *doc, size_t *count_out);

#ifdef __cplusplus
}
#endif

#endif /* CA_DOM_CORE_H */
