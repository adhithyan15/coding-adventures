# dom-core (C)

A small DOM tree model in pure ISO C17. A faithful port of the Rust `dom-core`
crate.

A lower-level model than a document AST: it preserves HTML element names,
namespaces, attributes, text, comments, and doctypes so browser-facing code can
later layer CSS, layout, and scripting on top.

## API

```c
#include "dom_core.h"

DomDocument *doc = dom_document_new();
DomAttribute attrs[] = {{"class", "intro"}};
dom_document_push_child(doc, dom_element("p", attrs, 1)); /* ownership transfers */
dom_document_push_child(doc, dom_text("hello"));

size_t n;
DomNode *const *kids = dom_document_children(doc, &n);   /* n == 2 */
dom_element_attribute(kids[0], "class");                 /* "intro" */

dom_document_free(doc);  /* frees the whole tree */
```

Constructors return a malloc'd `DomNode *` (NULL on OOM). Appending a node
(`dom_element_append_child`, `dom_document_push_child`) **transfers ownership** to
the parent — do not free an appended node yourself. Freeing a document (or a
detached node) recursively frees its subtree. Attribute arrays are deep-copied.
`dom_node_children` on a non-element returns NULL, mirroring Rust's
`children() -> Option<&[Node]>`.

## Portability

Pure ISO C17 — no POSIX `strdup`, no extensions. Compiles clean under GCC, Clang,
and MSVC with `-pedantic-errors` / `/permissive-` and warnings-as-errors, via the
shared [`iso-harness`](../iso-harness).

## Development

```bash
sh BUILD
```
