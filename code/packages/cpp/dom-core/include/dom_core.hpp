// dom_core.hpp — a small DOM tree model, in pure ISO C++17, header-only, in
// namespace ca::dom. A faithful port of the Rust `dom-core` crate.
// ===========================================================================
//
// A lower-level model than a document AST: it preserves HTML element names,
// namespaces, attributes, text, comments, and doctypes so browser-facing code
// can later layer CSS, layout, and scripting on top.
//
// A `Document` owns a list of top-level `Node`s. A `Node` is a std::variant over
// the four node kinds — a document-type declaration, an element (with a name, an
// optional namespace, attributes, and its own children), text, or a comment —
// mirroring the Rust enum. Value semantics throughout.
//
// DEPTH. Like the Rust original, the recursive destructor of a `Node` descends
// once per nesting level, so destroying a pathologically deep tree (as an
// untrusted-HTML parser layered on top could build) can exhaust the stack. A
// consumer that ingests untrusted markup should bound the nesting depth.
//
// PORTABILITY. Pure ISO C++17 — standard library only. Compiles clean under GCC,
// Clang, and MSVC with -pedantic-errors / /permissive- and warnings-as-errors.
#ifndef CA_DOM_CORE_HPP
#define CA_DOM_CORE_HPP

#include <optional>
#include <string>
#include <utility>
#include <variant>
#include <vector>

namespace ca {
namespace dom {

// An element attribute.
struct Attribute {
    std::string name;
    std::string value;
};

// A document-type declaration.
struct DocumentType {
    std::optional<std::string> name;
    std::optional<std::string> public_identifier;
    std::optional<std::string> system_identifier;
    bool force_quirks = false;
};

// Text and comment nodes.
struct Text {
    std::string data;
};
struct Comment {
    std::string data;
};

struct Node; // forward: Element holds a vector<Node>

// An element node.
struct Element {
    std::optional<std::string> namespace_;
    std::string name;
    std::vector<Attribute> attributes;
    std::vector<Node> children; // Node incomplete here is fine for a vector member

    // The value of attribute `name`, or nullptr when absent (a borrow, matching
    // Rust's `Option<&str>`). Defined after Node is complete.
    const std::string* attribute(const std::string& name) const;
};

// A DOM node: exactly one of the four kinds.
struct Node {
    std::variant<DocumentType, Element, Text, Comment> value;

    static Node element(std::string name, std::vector<Attribute> attributes) {
        Element e;
        e.name = std::move(name);
        e.attributes = std::move(attributes);
        return Node{std::move(e)};
    }
    static Node namespaced_element(std::string ns, std::string name,
                                   std::vector<Attribute> attributes) {
        Element e;
        e.namespace_ = std::move(ns);
        e.name = std::move(name);
        e.attributes = std::move(attributes);
        return Node{std::move(e)};
    }
    static Node text(std::string data) { return Node{Text{std::move(data)}}; }
    static Node comment(std::string data) {
        return Node{Comment{std::move(data)}};
    }

    // The child nodes if this is an element, else nullptr (Rust's
    // `children() -> Option<&[Node]>` / `children_mut()`).
    const std::vector<Node>* children() const {
        if (const Element* e = std::get_if<Element>(&value)) return &e->children;
        return nullptr;
    }
    std::vector<Node>* children_mut() {
        if (Element* e = std::get_if<Element>(&value)) return &e->children;
        return nullptr;
    }
};

inline const std::string* Element::attribute(const std::string& name) const {
    for (const Attribute& a : attributes) {
        if (a.name == name) return &a.value;
    }
    return nullptr;
}

// A parsed DOM document.
struct Document {
    std::vector<Node> children;

    void push_child(Node node) { children.push_back(std::move(node)); }
};

}  // namespace dom
}  // namespace ca

#endif  // CA_DOM_CORE_HPP
