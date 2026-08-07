/*
 * dom_core.c — implementation of the small DOM tree model.
 * ===========================================================================
 *
 * Each node is a tagged union. Element and Document own growable arrays of child
 * node POINTERS (each child is a separately-allocated node the parent owns), so
 * freeing a document walks the tree and frees every node exactly once. Every
 * owned string is a deep copy, so callers may build a tree from literals.
 */
#include "dom_core.h"

#include <stdlib.h>
#include <string.h>

/* ===========================================================================
 *  Owned strings & child arrays
 * =========================================================================== */

/* Deep-copy a NUL-terminated string (NULL stays NULL, distinct from OOM which
 * the callers detect by the source being non-NULL yet the result NULL). */
static char *dup_str(const char *s) {
    if (!s) return NULL;
    size_t n = strlen(s);
    char *out = malloc(n + 1);
    if (!out) return NULL;
    memcpy(out, s, n + 1);
    return out;
}

/* A growable array of child node pointers. */
typedef struct {
    DomNode **items;
    size_t len;
    size_t cap;
} NodeList;

/* Append (taking ownership). Returns 0, or -1 on OOM (child NOT freed here — the
 * caller decides, matching each public function's documented contract). */
static int nodelist_push(NodeList *list, DomNode *child) {
    if (list->len == list->cap) {
        size_t ncap = list->cap ? list->cap * 2 : 4;
        if (list->cap > ((size_t)-1) / 2 / sizeof(DomNode *)) return -1;
        DomNode **ni = realloc(list->items, ncap * sizeof(DomNode *));
        if (!ni) return -1;
        list->items = ni;
        list->cap = ncap;
    }
    list->items[list->len++] = child;
    return 0;
}

/* dom_node_free is declared in dom_core.h (public) and used below. */

static void nodelist_free(NodeList *list) {
    for (size_t i = 0; i < list->len; i++) dom_node_free(list->items[i]);
    free(list->items);
    list->items = NULL;
    list->len = 0;
    list->cap = 0;
}

/* ===========================================================================
 *  Node representation
 * =========================================================================== */

typedef struct {
    char *name;  /* owned */
    char *value; /* owned */
} OwnedAttr;

typedef struct {
    char *namespace_; /* NULL if none */
    char *name;
    OwnedAttr *attrs;
    size_t nattrs;
    NodeList children;
} ElementData;

typedef struct {
    char *name;          /* NULL if absent */
    char *public_id;     /* NULL if absent */
    char *system_id;     /* NULL if absent */
    int force_quirks;
} DoctypeData;

struct DomNode {
    DomNodeKind kind;
    union {
        ElementData element;
        DoctypeData doctype;
        char *text;    /* DOM_TEXT */
        char *comment; /* DOM_COMMENT */
    } as;
};

struct DomDocument {
    NodeList children;
};

/* ===========================================================================
 *  Freeing
 * =========================================================================== */

void dom_node_free(DomNode *node) {
    if (!node) return;
    switch (node->kind) {
        case DOM_ELEMENT: {
            ElementData *e = &node->as.element;
            free(e->namespace_);
            free(e->name);
            for (size_t i = 0; i < e->nattrs; i++) {
                free(e->attrs[i].name);
                free(e->attrs[i].value);
            }
            free(e->attrs);
            nodelist_free(&e->children);
            break;
        }
        case DOM_DOCUMENT_TYPE: {
            DoctypeData *d = &node->as.doctype;
            free(d->name);
            free(d->public_id);
            free(d->system_id);
            break;
        }
        case DOM_TEXT:
            free(node->as.text);
            break;
        case DOM_COMMENT:
            free(node->as.comment);
            break;
    }
    free(node);
}

/* ===========================================================================
 *  Constructors
 * =========================================================================== */

/* Deep-copy an attribute array into *out (count into *n_out). 0, or -1 on OOM. */
static int copy_attrs(const DomAttribute *attrs, size_t nattrs, OwnedAttr **out,
                      size_t *n_out) {
    *out = NULL;
    *n_out = 0;
    if (nattrs == 0) return 0;
    OwnedAttr *copy = calloc(nattrs, sizeof *copy); /* checked multiply */
    if (!copy) return -1;
    for (size_t i = 0; i < nattrs; i++) {
        copy[i].name = dup_str(attrs[i].name);
        copy[i].value = dup_str(attrs[i].value);
        if ((attrs[i].name && !copy[i].name) ||
            (attrs[i].value && !copy[i].value)) {
            for (size_t j = 0; j <= i; j++) {
                free(copy[j].name);
                free(copy[j].value);
            }
            free(copy);
            return -1;
        }
    }
    *out = copy;
    *n_out = nattrs;
    return 0;
}

static DomNode *make_element(const char *ns, const char *name,
                             const DomAttribute *attrs, size_t nattrs) {
    DomNode *node = calloc(1, sizeof *node);
    if (!node) return NULL;
    node->kind = DOM_ELEMENT;
    ElementData *e = &node->as.element;
    if (ns) {
        e->namespace_ = dup_str(ns);
        if (!e->namespace_) {
            free(node);
            return NULL;
        }
    }
    e->name = dup_str(name);
    if (!e->name) {
        free(e->namespace_);
        free(node);
        return NULL;
    }
    if (copy_attrs(attrs, nattrs, &e->attrs, &e->nattrs) != 0) {
        free(e->namespace_);
        free(e->name);
        free(node);
        return NULL;
    }
    return node;
}

DomNode *dom_element(const char *name, const DomAttribute *attrs,
                     size_t nattrs) {
    return make_element(NULL, name, attrs, nattrs);
}

DomNode *dom_namespaced_element(const char *ns, const char *name,
                                const DomAttribute *attrs, size_t nattrs) {
    return make_element(ns, name, attrs, nattrs);
}

/* Text and comment share a shape: one owned string. */
static DomNode *make_leaf(DomNodeKind kind, const char *data) {
    DomNode *node = calloc(1, sizeof *node);
    if (!node) return NULL;
    node->kind = kind;
    char *copy = dup_str(data);
    if (!copy) {
        free(node);
        return NULL;
    }
    if (kind == DOM_TEXT) {
        node->as.text = copy;
    } else {
        node->as.comment = copy;
    }
    return node;
}

DomNode *dom_text(const char *data) { return make_leaf(DOM_TEXT, data); }
DomNode *dom_comment(const char *data) { return make_leaf(DOM_COMMENT, data); }

DomNode *dom_doctype(const char *name, const char *public_id,
                     const char *system_id, int force_quirks) {
    DomNode *node = calloc(1, sizeof *node);
    if (!node) return NULL;
    node->kind = DOM_DOCUMENT_TYPE;
    DoctypeData *d = &node->as.doctype;
    d->force_quirks = force_quirks ? 1 : 0;
    /* dup_str keeps NULL as NULL; only a non-NULL source that fails to copy is
     * an error. */
    if ((name && !(d->name = dup_str(name))) ||
        (public_id && !(d->public_id = dup_str(public_id))) ||
        (system_id && !(d->system_id = dup_str(system_id)))) {
        dom_node_free(node);
        return NULL;
    }
    return node;
}

/* ===========================================================================
 *  Accessors
 * =========================================================================== */

DomNodeKind dom_node_kind(const DomNode *node) { return node->kind; }

int dom_element_append_child(DomNode *parent, DomNode *child) {
    if (parent->kind != DOM_ELEMENT) return -1; /* child left to the caller */
    if (nodelist_push(&parent->as.element.children, child) != 0) {
        dom_node_free(child); /* took ownership; free on failure */
        return -1;
    }
    return 0;
}

const char *dom_element_name(const DomNode *node) {
    return node->kind == DOM_ELEMENT ? node->as.element.name : NULL;
}

const char *dom_element_namespace(const DomNode *node) {
    return node->kind == DOM_ELEMENT ? node->as.element.namespace_ : NULL;
}

const char *dom_element_attribute(const DomNode *node, const char *name) {
    if (node->kind != DOM_ELEMENT) return NULL;
    const ElementData *e = &node->as.element;
    for (size_t i = 0; i < e->nattrs; i++) {
        if (e->attrs[i].name && strcmp(e->attrs[i].name, name) == 0) {
            return e->attrs[i].value;
        }
    }
    return NULL;
}

DomNode *const *dom_node_children(const DomNode *node, size_t *count_out) {
    if (node->kind != DOM_ELEMENT) {
        *count_out = 0;
        return NULL;
    }
    *count_out = node->as.element.children.len;
    return node->as.element.children.items;
}

const char *dom_text_data(const DomNode *node) {
    return node->kind == DOM_TEXT ? node->as.text : NULL;
}
const char *dom_comment_data(const DomNode *node) {
    return node->kind == DOM_COMMENT ? node->as.comment : NULL;
}

const char *dom_doctype_name(const DomNode *node) {
    return node->kind == DOM_DOCUMENT_TYPE ? node->as.doctype.name : NULL;
}
const char *dom_doctype_public_id(const DomNode *node) {
    return node->kind == DOM_DOCUMENT_TYPE ? node->as.doctype.public_id : NULL;
}
const char *dom_doctype_system_id(const DomNode *node) {
    return node->kind == DOM_DOCUMENT_TYPE ? node->as.doctype.system_id : NULL;
}
int dom_doctype_force_quirks(const DomNode *node) {
    return node->kind == DOM_DOCUMENT_TYPE ? node->as.doctype.force_quirks : 0;
}

/* ===========================================================================
 *  Document
 * =========================================================================== */

DomDocument *dom_document_new(void) {
    return calloc(1, sizeof(DomDocument)); /* zeroed NodeList = empty */
}

void dom_document_free(DomDocument *doc) {
    if (!doc) return;
    nodelist_free(&doc->children);
    free(doc);
}

int dom_document_push_child(DomDocument *doc, DomNode *child) {
    if (nodelist_push(&doc->children, child) != 0) {
        dom_node_free(child);
        return -1;
    }
    return 0;
}

DomNode *const *dom_document_children(const DomDocument *doc,
                                      size_t *count_out) {
    *count_out = doc->children.len;
    return doc->children.items;
}
