// Tests for the C++ dom-core tree model, using the header-only iso_test.h
// harness (pure ISO). Vectors mirror the Rust crate's own unit test, extended to
// cover every node kind, namespaced elements, nested children, and accessors.
#include "iso_test.h"

#include <string>
#include <variant>
#include <vector>

#include "dom_core.hpp"

namespace dom = ca::dom;

int main() {
    // ── a document holding element, text, and comment nodes ──────────────
    {
        dom::Document doc;
        doc.push_child(dom::Node::element("p", {{"class", "intro"}}));
        doc.push_child(dom::Node::text("hello"));
        doc.push_child(dom::Node::comment("note"));

        ISO_CHECK_EQ_UINT(doc.children.size(), 3u);

        const dom::Element* e = std::get_if<dom::Element>(&doc.children[0].value);
        ISO_CHECK(e != nullptr);
        if (e) {
            ISO_CHECK_STR_EQ(e->name.c_str(), "p");
            const std::string* cls = e->attribute("class");
            ISO_CHECK(cls != nullptr);
            if (cls) ISO_CHECK_STR_EQ(cls->c_str(), "intro");
            ISO_CHECK(e->attribute("id") == nullptr); // absent
        }
        ISO_CHECK(std::holds_alternative<dom::Text>(doc.children[1].value));
        ISO_CHECK(std::holds_alternative<dom::Comment>(doc.children[2].value));
        ISO_CHECK_STR_EQ(std::get<dom::Text>(doc.children[1].value).data.c_str(),
                         "hello");
        ISO_CHECK_STR_EQ(
            std::get<dom::Comment>(doc.children[2].value).data.c_str(), "note");
    }

    // ── nested elements via children_mut, and children() ─────────────────
    {
        dom::Node div = dom::Node::element("div", {});
        std::vector<dom::Node>* kids = div.children_mut();
        ISO_CHECK(kids != nullptr);
        if (kids) {
            kids->push_back(dom::Node::element("span", {}));
            kids->push_back(dom::Node::text("inner"));
        }
        const std::vector<dom::Node>* view = div.children();
        ISO_CHECK(view != nullptr);
        if (view) {
            ISO_CHECK_EQ_UINT(view->size(), 2u);
            ISO_CHECK_STR_EQ(
                std::get<dom::Element>((*view)[0].value).name.c_str(), "span");
            ISO_CHECK_STR_EQ(
                std::get<dom::Text>((*view)[1].value).data.c_str(), "inner");
        }
    }

    // ── children() on a non-element is nullptr (Rust: Option::None) ──────
    {
        dom::Node t = dom::Node::text("x");
        ISO_CHECK(t.children() == nullptr);
        ISO_CHECK(t.children_mut() == nullptr);
    }

    // ── namespaced element ───────────────────────────────────────────────
    {
        dom::Node svg = dom::Node::namespaced_element(
            "http://www.w3.org/2000/svg", "svg", {{"viewBox", "0 0 10 10"}});
        const dom::Element& e = std::get<dom::Element>(svg.value);
        ISO_CHECK(e.namespace_.has_value());
        if (e.namespace_)
            ISO_CHECK_STR_EQ(e.namespace_->c_str(), "http://www.w3.org/2000/svg");
        ISO_CHECK_STR_EQ(e.name.c_str(), "svg");
        const std::string* vb = e.attribute("viewBox");
        ISO_CHECK(vb != nullptr);
        if (vb) ISO_CHECK_STR_EQ(vb->c_str(), "0 0 10 10");

        dom::Node p = dom::Node::element("p", {});
        ISO_CHECK(!std::get<dom::Element>(p.value).namespace_.has_value());
    }

    // ── doctype, with optional fields present and absent ─────────────────
    {
        dom::DocumentType dt;
        dt.name = "html";
        dom::Node node{dt};
        const dom::DocumentType& d = std::get<dom::DocumentType>(node.value);
        ISO_CHECK(d.name.has_value() && *d.name == "html");
        ISO_CHECK(!d.public_identifier.has_value());
        ISO_CHECK(!d.system_identifier.has_value());
        ISO_CHECK(d.force_quirks == false);

        dom::DocumentType legacy;
        legacy.name = "html";
        legacy.public_identifier = "-//W3C//DTD HTML 4.01//EN";
        legacy.system_identifier = "http://www.w3.org/TR/html4/strict.dtd";
        legacy.force_quirks = true;
        ISO_CHECK(*legacy.public_identifier == "-//W3C//DTD HTML 4.01//EN");
        ISO_CHECK(legacy.force_quirks == true);
    }

    return ISO_TEST_RESULT();
}
