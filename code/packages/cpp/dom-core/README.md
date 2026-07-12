# dom-core (C++)

A small DOM tree model in pure ISO C++17, header-only, in namespace `ca::dom`. A
faithful port of the Rust `dom-core` crate.

A lower-level model than a document AST: it preserves HTML element names,
namespaces, attributes, text, comments, and doctypes so browser-facing code can
later layer CSS, layout, and scripting on top.

## API

```cpp
#include "dom_core.hpp"
namespace dom = ca::dom;

dom::Document doc;
doc.push_child(dom::Node::element("p", {{"class", "intro"}}));
doc.push_child(dom::Node::text("hello"));

auto& e = std::get<dom::Element>(doc.children[0].value);
const std::string* cls = e.attribute("class");   // "intro", or nullptr if absent
```

A `Node` is a `std::variant<DocumentType, Element, Text, Comment>` (mirroring the
Rust enum) with static factories `Node::element` / `namespaced_element` / `text`
/ `comment`. `Node::children()` / `children_mut()` return the child vector for an
element or `nullptr` otherwise (Rust's `Option<&[Node]>`). Value semantics
throughout.

## Portability

Pure ISO C++17 — standard library only. Compiles clean under GCC, Clang, and MSVC
with `-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../../c/iso-harness).

## Development

```bash
sh BUILD
```
