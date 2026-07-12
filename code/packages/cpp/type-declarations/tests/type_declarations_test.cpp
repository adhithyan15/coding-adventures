// Tests for the C++ type-declarations library, using the header-only iso_test.h
// harness (pure ISO). Vectors mirror the Rust crate's own tests.
#include "iso_test.h"

#include <string>
#include <vector>

#include "type_declarations.hpp"

namespace td = ca::type_declarations;
using td::KindDecl;

int main() {
    // ── KindDecl::to_iir_hint ─────────────────────────────────────────────
    ISO_CHECK_STR_EQ(KindDecl::Int().to_iir_hint(), "i64");
    ISO_CHECK_STR_EQ(KindDecl::Bool().to_iir_hint(), "bool");
    ISO_CHECK_STR_EQ(KindDecl::Str().to_iir_hint(), "str");
    ISO_CHECK_STR_EQ(KindDecl::Function(1).to_iir_hint(), "closure");
    ISO_CHECK_STR_EQ(KindDecl::Function(0).to_iir_hint(), "closure");
    ISO_CHECK_STR_EQ(KindDecl::Any().to_iir_hint(), "any");
    ISO_CHECK_STR_EQ(KindDecl::Nil().to_iir_hint(), "any");
    ISO_CHECK_STR_EQ(KindDecl::Symbol().to_iir_hint(), "any");
    ISO_CHECK_STR_EQ(KindDecl::List().to_iir_hint(), "any");
    ISO_CHECK_STR_EQ(KindDecl::Named("Foo").to_iir_hint(), "any");

    // ── is_concrete_hint ──────────────────────────────────────────────────
    ISO_CHECK(KindDecl::Int().is_concrete_hint());
    ISO_CHECK(KindDecl::Bool().is_concrete_hint());
    ISO_CHECK(KindDecl::Function(2).is_concrete_hint());
    ISO_CHECK(!KindDecl::Any().is_concrete_hint());
    ISO_CHECK(!KindDecl::Nil().is_concrete_hint());

    // ── AnnotatedNode::iir_hint + position ────────────────────────────────
    {
        td::AnnotatedNode node("atom", KindDecl::Int());
        node.set_position(1, 1, 1, 2);
        ISO_CHECK_STR_EQ(node.iir_hint(), "i64");
        ISO_CHECK(node.position().first == 1u && node.position().second == 1u);
    }

    // ── resolve: alias chain + passthrough ────────────────────────────────
    {
        td::TypeDeclarations d("twig");
        ISO_CHECK_STR_EQ(d.language.c_str(), "twig");
        d.named_types["Nat"] = td::AliasType{KindDecl::Int()};
        ISO_CHECK(d.resolve(KindDecl::Named("Nat")) == KindDecl::Int());
        ISO_CHECK(d.resolve(KindDecl::Bool()) == KindDecl::Bool());
    }

    // ── resolve on a cycle returns Any ────────────────────────────────────
    {
        td::TypeDeclarations d("twig");
        d.named_types["A"] = td::AliasType{KindDecl::Named("A")};
        ISO_CHECK(d.resolve(KindDecl::Named("A")) == KindDecl::Any());
    }

    // ── a Named record stays Named ────────────────────────────────────────
    {
        td::TypeDeclarations d("twig");
        d.named_types["Point"] = td::RecordType{{}};
        ISO_CHECK(d.resolve(KindDecl::Named("Point")) ==
                  KindDecl::Named("Point"));
    }

    // ── union_variants lookup ─────────────────────────────────────────────
    {
        td::TypeDeclarations d("twig");
        d.named_types["Shape"] = td::UnionType{
            {td::VariantDecl{"Circle", {}}, td::VariantDecl{"Rect", {}}}};
        auto vs = d.union_variants("Shape");
        ISO_CHECK(vs.has_value());
        ISO_CHECK_EQ_UINT(vs->size(), 2u);
        ISO_CHECK_STR_EQ((*vs)[0].c_str(), "Circle");
        ISO_CHECK_STR_EQ((*vs)[1].c_str(), "Rect");
        ISO_CHECK(!d.union_variants("Unknown").has_value());
    }

    // ── new is empty; typed mode + globals + a record with fields ─────────
    {
        td::TypeDeclarations d("twig");
        ISO_CHECK(d.named_types.empty());
        ISO_CHECK(d.globals.empty());
        ISO_CHECK(!d.typed_mode.has_value());
        d.typed_mode = td::TypedModeDecl::Strict;
        ISO_CHECK(d.typed_mode == td::TypedModeDecl::Strict);

        d.globals["origin"] = KindDecl::Named("Point");
        d.named_types["Point"] =
            td::RecordType{{td::FieldDecl{"x", KindDecl::Int()},
                            td::FieldDecl{"y", KindDecl::Int()}}};
        ISO_CHECK_EQ_UINT(d.globals.size(), 1u);
        ISO_CHECK_EQ_UINT(d.named_types.size(), 1u);
    }

    // ── AnnotatedNode children: child_node / node_children ────────────────
    {
        td::AnnotatedNode root("program", KindDecl::Any());
        root.add_token("(", 1, 1);
        root.add_child(td::AnnotatedNode("atom", KindDecl::Int()));
        root.add_child(td::AnnotatedNode("cond", KindDecl::Bool()));

        const td::AnnotatedNode* found = root.child_node("atom");
        ISO_CHECK(found != nullptr);
        if (found) ISO_CHECK_STR_EQ(found->iir_hint(), "i64");
        ISO_CHECK(root.child_node("missing") == nullptr);

        ISO_CHECK_EQ_UINT(root.node_children().size(), 2u);  // token excluded
        ISO_CHECK(root.position().first == 0u &&
                  root.position().second == 0u);  // unset -> 0
    }

    return ISO_TEST_RESULT();
}
